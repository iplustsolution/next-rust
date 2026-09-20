//! Scenarios shared by the in-memory and Redis test suites: both backends
//! must behave the same.

#![allow(dead_code)]

use std::future::Future;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use next_rust_jobs::*;
use serde::{Deserialize, Serialize};

pub async fn wait_until(what: &str, mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while !check() {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

pub async fn wait_for<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = Instant::now();
    while !check().await {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Test jobs are generic over a marker so that each test gets statics of its own.
pub trait Slot: Send + Sync + 'static {
    fn log() -> &'static Mutex<Vec<String>>;
    fn counter() -> &'static AtomicU32;
}

macro_rules! slot {
    ($name:ident) => {
        pub struct $name;
        impl $crate::common::Slot for $name {
            fn log() -> &'static std::sync::Mutex<Vec<String>> {
                static LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
                &LOG
            }
            fn counter() -> &'static std::sync::atomic::AtomicU32 {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                &N
            }
        }
    };
}

pub fn logged<S: Slot>() -> Vec<String> {
    S::log().lock().unwrap().clone()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub text: String,
    /// Larger than 2^53: must survive the backend byte for byte.
    pub big: u64,
}

pub fn note(text: &str) -> Note {
    Note { text: text.to_owned(), big: u64::MAX - 7 }
}

/// Records its input.
pub struct Record<S>(pub std::marker::PhantomData<S>);
impl<S: Slot> Job for Record<S> {
    type Input = Note;
    const NAME: &'static str = "record";
    fn options() -> JobOptions {
        JobOptions::new().queue("q")
    }
    async fn run(input: Note, _ctx: JobContext) -> Result<(), JobError> {
        assert_eq!(input.big, u64::MAX - 7);
        S::log().lock().unwrap().push(input.text);
        Ok(())
    }
}
pub fn record<S>() -> Record<S> {
    Record(std::marker::PhantomData)
}

/// Fails until the attempt number reaches the input.
pub struct Flaky<S>(pub std::marker::PhantomData<S>);
impl<S: Slot> Job for Flaky<S> {
    type Input = u32;
    const NAME: &'static str = "flaky";
    fn options() -> JobOptions {
        JobOptions::new().queue("q").max_attempts(3).backoff(Backoff::fixed(Duration::from_millis(20)))
    }
    async fn run(succeed_on: u32, ctx: JobContext) -> Result<(), JobError> {
        S::counter().fetch_add(1, Ordering::SeqCst);
        S::log().lock().unwrap().push(format!("attempt {}", ctx.attempt));
        if ctx.attempt >= succeed_on { Ok(()) } else { Err(JobError::msg(format!("boom {}", ctx.attempt))) }
    }
}
pub fn flaky<S>() -> Flaky<S> {
    Flaky(std::marker::PhantomData)
}

/// Sleeps for the given milliseconds.
pub struct Sleep<S>(pub std::marker::PhantomData<S>);
impl<S: Slot> Job for Sleep<S> {
    type Input = u64;
    const NAME: &'static str = "sleep";
    fn options() -> JobOptions {
        JobOptions::new().queue("q").max_attempts(2).backoff(Backoff::fixed(Duration::from_millis(10)))
    }
    async fn run(ms: u64, ctx: JobContext) -> Result<(), JobError> {
        S::log().lock().unwrap().push(format!("start {}", ctx.attempt));
        tokio::time::sleep(Duration::from_millis(ms)).await;
        S::log().lock().unwrap().push(format!("end {}", ctx.attempt));
        S::counter().fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
pub fn sleep<S>() -> Sleep<S> {
    Sleep(std::marker::PhantomData)
}

pub fn fast(runner: JobRunner) -> JobRunner {
    runner.poll_interval(Duration::from_millis(10)).shutdown_timeout(Duration::from_secs(5))
}

// ── scenarios ───────────────────────────────────────────────────────────

slot!(RunsSlot);
pub async fn runs_a_job_and_cleans_up(client: JobClient) {
    let handle = fast(JobRunner::with_client(client.clone())).register(record::<RunsSlot>()).start().unwrap();
    let out = client.enqueue(record::<RunsSlot>(), note("hello")).await.unwrap();
    assert!(!out.duplicate);
    wait_until("the job to run", || logged::<RunsSlot>() == ["hello"]).await;
    wait_for("the queue to be empty", || async {
        let s = client.stats("q").await.unwrap();
        s.completed_total == 1 && s.ready + s.scheduled + s.active + s.dead == 0
    })
    .await;
    assert_eq!(client.stats("q").await.unwrap().enqueued_total, 1);
    let m = client.metrics();
    assert_eq!((m.enqueued, m.started, m.succeeded, m.failed), (1, 1, 1, 0));
    handle.shutdown().await;
}

slot!(DelaySlot);
pub async fn delayed_jobs_wait(client: JobClient) {
    let handle = fast(JobRunner::with_client(client.clone())).register(record::<DelaySlot>()).start().unwrap();
    let started = Instant::now();
    client.enqueue_in(Duration::from_millis(300), record::<DelaySlot>(), note("later")).await.unwrap();
    client.enqueue(record::<DelaySlot>(), note("now")).await.unwrap();
    wait_until("the immediate job", || logged::<DelaySlot>() == ["now"]).await;
    let s = client.stats("q").await.unwrap();
    assert_eq!(s.scheduled, 1, "{s:?}");
    wait_until("the delayed job", || logged::<DelaySlot>() == ["now", "later"]).await;
    assert!(started.elapsed() >= Duration::from_millis(290), "{:?}", started.elapsed());
    // A time in the past runs at once.
    client.enqueue_at(std::time::SystemTime::UNIX_EPOCH, record::<DelaySlot>(), note("past")).await.unwrap();
    wait_until("the past job", || logged::<DelaySlot>().len() == 3).await;
    handle.shutdown().await;
}

slot!(RetrySlot);
pub async fn retries_then_succeeds(client: JobClient) {
    let failures = Arc::new(Mutex::new(Vec::new()));
    let seen = failures.clone();
    let successes = Arc::new(AtomicUsize::new(0));
    let ok = successes.clone();
    let handle = fast(JobRunner::with_client(client.clone()))
        .register(flaky::<RetrySlot>())
        .on_failure(move |info, f| seen.lock().unwrap().push((info.attempt, f.error.clone(), f.retry_in)))
        .on_success(move |info, _| {
            assert_eq!(info.attempt, 3);
            ok.fetch_add(1, Ordering::SeqCst);
        })
        .logger(|_, _| {})
        .start()
        .unwrap();
    client.enqueue(flaky::<RetrySlot>(), 3).await.unwrap();
    wait_until("the third attempt", || successes.load(Ordering::SeqCst) == 1).await;
    assert_eq!(logged::<RetrySlot>(), ["attempt 1", "attempt 2", "attempt 3"]);
    let failures = failures.lock().unwrap().clone();
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0], (1, "boom 1".to_owned(), Some(Duration::from_millis(20))));
    wait_for("stats", || async { client.stats("q").await.unwrap().completed_total == 1 }).await;
    let s = client.stats("q").await.unwrap();
    assert_eq!((s.retried_total, s.dead_total, s.dead), (2, 0, 0));
    handle.shutdown().await;
}

slot!(DeadSlot);
pub async fn exhausted_jobs_are_dead_lettered_and_can_be_revived(client: JobClient) {
    let handle =
        fast(JobRunner::with_client(client.clone())).register(flaky::<DeadSlot>()).logger(|_, _| {}).start().unwrap();
    // Succeeds on attempt 4, but only 3 are allowed.
    let id = client.enqueue(flaky::<DeadSlot>(), 4).await.unwrap().id;
    wait_for("the dead letter", || async { client.stats("q").await.unwrap().dead == 1 }).await;
    assert_eq!(DeadSlot::counter().load(Ordering::SeqCst), 3);
    let dead = client.dead_jobs("q", 10).await.unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!((dead[0].record.id.as_str(), dead[0].attempts, dead[0].error.as_str()), (id.as_str(), 3, "boom 3"));
    assert_eq!(dead[0].record.payload, serde_json::json!(4));
    assert!(dead[0].failed_at_ms > 0);

    // Reviving gives it a fresh set of attempts: 3 more failures.
    assert!(client.retry_dead("q", &id).await.unwrap());
    assert!(!client.retry_dead("q", "nope").await.unwrap());
    wait_until("three more attempts", || DeadSlot::counter().load(Ordering::SeqCst) == 6).await;
    wait_for("dead again", || async { client.stats("q").await.unwrap().dead == 1 }).await;
    assert_eq!(client.purge_dead("q").await.unwrap(), 1);
    assert_eq!(client.stats("q").await.unwrap().dead, 0);
    assert!(client.dead_jobs("q", 10).await.unwrap().is_empty());
    handle.shutdown().await;
}

pub struct Fatal;
impl Job for Fatal {
    type Input = ();
    const NAME: &'static str = "fatal";
    fn options() -> JobOptions {
        JobOptions::new().queue("q")
    }
    async fn run(_: (), _: JobContext) -> Result<(), JobError> {
        Err(JobError::fatal("bad input"))
    }
}

pub struct Panics;
impl Job for Panics {
    type Input = ();
    const NAME: &'static str = "panics";
    fn options() -> JobOptions {
        JobOptions::new().queue("q").max_attempts(1)
    }
    async fn run(_: (), _: JobContext) -> Result<(), JobError> {
        panic!("kaboom")
    }
}

pub struct Slow;
impl Job for Slow {
    type Input = ();
    const NAME: &'static str = "slow";
    fn options() -> JobOptions {
        JobOptions::new().queue("q").max_attempts(1).timeout(Duration::from_millis(40))
    }
    async fn run(_: (), _: JobContext) -> Result<(), JobError> {
        tokio::time::sleep(Duration::from_secs(30)).await;
        Ok(())
    }
}

pub async fn fatal_timeout_panic_and_unknown(client: JobClient) {
    let handle = fast(JobRunner::with_client(client.clone()))
        .register(Fatal)
        .register(Panics)
        .register(Slow)
        .logger(|_, _| {})
        .start()
        .unwrap();
    client.enqueue(Fatal, ()).await.unwrap();
    client.enqueue(Panics, ()).await.unwrap();
    client.enqueue(Slow, ()).await.unwrap();
    // No handler anywhere, and input that does not deserialise.
    client.raw("ghost", serde_json::json!(1), JobOptions::new().queue("q").max_attempts(1)).send().await.unwrap();
    client.raw("fatal", serde_json::json!("not unit"), JobOptions::new().queue("q")).send().await.unwrap();
    wait_for("five dead jobs", || async { client.stats("q").await.unwrap().dead == 5 }).await;
    let mut errors: Vec<(String, u32, String)> =
        client.dead_jobs("q", 10).await.unwrap().into_iter().map(|d| (d.record.name, d.attempts, d.error)).collect();
    errors.sort();
    assert_eq!(errors[0].0, "fatal");
    assert!(errors.iter().any(|e| e.2 == "bad input" && e.1 == 1), "fatal errors skip the retries: {errors:?}");
    assert!(errors.iter().any(|e| e.2.contains("invalid input for job `fatal`")), "{errors:?}");
    assert!(errors.iter().any(|e| e.0 == "ghost" && e.2.contains("no handler")), "{errors:?}");
    assert!(errors.iter().any(|e| e.0 == "panics" && e.2 == "panicked: kaboom"), "{errors:?}");
    assert!(errors.iter().any(|e| e.0 == "slow" && e.2.contains("timed out")), "{errors:?}");
    let m = client.metrics();
    assert_eq!((m.timed_out, m.panicked, m.dead), (1, 1, 5));
    handle.shutdown().await;
}

slot!(UniqueSlot);
pub async fn unique_jobs_and_idempotency_keys(client: JobClient) {
    // No workers yet: jobs stay queued.
    let first = client.job(record::<UniqueSlot>(), note("a")).unique("report:7").send().await.unwrap();
    let second = client.job(record::<UniqueSlot>(), note("b")).unique("report:7").send().await.unwrap();
    assert!(!first.duplicate);
    assert!(second.duplicate);
    assert_eq!(second.id, first.id);
    let other = client.job(record::<UniqueSlot>(), note("c")).unique("report:8").send().await.unwrap();
    assert!(!other.duplicate);
    assert_eq!(client.stats("q").await.unwrap().ready, 2);
    assert_eq!(client.metrics().duplicates, 1);

    // Cancelling releases the key.
    assert!(client.cancel("q", &other.id).await.unwrap());
    assert!(!client.cancel("q", &other.id).await.unwrap());
    assert!(!client.job(record::<UniqueSlot>(), note("c2")).unique("report:8").send().await.unwrap().duplicate);

    let handle = fast(JobRunner::with_client(client.clone())).register(record::<UniqueSlot>()).start().unwrap();
    wait_until("both jobs", || logged::<UniqueSlot>().len() == 2).await;
    wait_for("completion", || async { client.stats("q").await.unwrap().completed_total == 2 }).await;
    // Completed: the key is free again.
    assert!(!client.job(record::<UniqueSlot>(), note("d")).unique("report:7").send().await.unwrap().duplicate);
    wait_until("the third job", || logged::<UniqueSlot>().len() == 3).await;

    // An idempotency window outlives the job.
    let window = Duration::from_millis(400);
    let once = client.job(record::<UniqueSlot>(), note("pay")).unique_for("pay:1", window).send().await.unwrap();
    assert!(!once.duplicate);
    wait_until("the payment", || logged::<UniqueSlot>().len() == 4).await;
    wait_for("completion", || async { client.stats("q").await.unwrap().completed_total == 4 }).await;
    let again = client.job(record::<UniqueSlot>(), note("pay")).unique_for("pay:1", window).send().await.unwrap();
    assert!(again.duplicate);
    assert_eq!(again.id, once.id);
    tokio::time::sleep(window + Duration::from_millis(50)).await;
    assert!(
        !client.job(record::<UniqueSlot>(), note("pay")).unique_for("pay:1", window).send().await.unwrap().duplicate
    );
    wait_until("the second payment", || logged::<UniqueSlot>().len() == 5).await;
    handle.shutdown().await;
}

slot!(PrioritySlot);
pub async fn priorities_then_fifo(client: JobClient) {
    for (text, priority) in [("low", -5), ("normal-1", 0), ("high", 50), ("normal-2", 0), ("urgent", 100)] {
        client.job(record::<PrioritySlot>(), note(text)).priority(priority).send().await.unwrap();
        // Same-millisecond ties are broken by id; keep the order unambiguous.
        tokio::time::sleep(Duration::from_millis(3)).await;
    }
    let handle =
        fast(JobRunner::with_client(client.clone())).queue("q", 1).register(record::<PrioritySlot>()).start().unwrap();
    wait_until("all five", || logged::<PrioritySlot>().len() == 5).await;
    assert_eq!(logged::<PrioritySlot>(), ["urgent", "high", "normal-1", "normal-2", "low"]);
    handle.shutdown().await;
}

pub struct Gauge;
static GAUGE_NOW: AtomicUsize = AtomicUsize::new(0);
static GAUGE_MAX: AtomicUsize = AtomicUsize::new(0);
static GAUGE_DONE: AtomicUsize = AtomicUsize::new(0);
impl Job for Gauge {
    type Input = ();
    const NAME: &'static str = "gauge";
    fn options() -> JobOptions {
        JobOptions::new().queue("limited")
    }
    async fn run(_: (), _: JobContext) -> Result<(), JobError> {
        let now = GAUGE_NOW.fetch_add(1, Ordering::SeqCst) + 1;
        GAUGE_MAX.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
        GAUGE_NOW.fetch_sub(1, Ordering::SeqCst);
        GAUGE_DONE.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub async fn concurrency_limit_per_queue(client: JobClient) {
    for _ in 0..9 {
        client.enqueue(Gauge, ()).await.unwrap();
    }
    let handle = fast(JobRunner::with_client(client.clone())).queue("limited", 3).register(Gauge).start().unwrap();
    wait_until("nine jobs", || GAUGE_DONE.load(Ordering::SeqCst) == 9).await;
    assert_eq!(GAUGE_MAX.load(Ordering::SeqCst), 3);
    handle.shutdown().await;
}

slot!(ShutdownSlot);
pub async fn shutdown_finishes_in_flight_jobs(client: JobClient) {
    let handle =
        fast(JobRunner::with_client(client.clone())).queue("q", 1).register(sleep::<ShutdownSlot>()).start().unwrap();
    client.enqueue(sleep::<ShutdownSlot>(), 200).await.unwrap();
    client.enqueue(sleep::<ShutdownSlot>(), 200).await.unwrap();
    wait_until("the first job to start", || logged::<ShutdownSlot>().len() == 1).await;
    handle.shutdown().await;
    // The running job finished; the queued one was never claimed.
    assert_eq!(logged::<ShutdownSlot>(), ["start 1", "end 1"]);
    let s = client.stats("q").await.unwrap();
    assert_eq!((s.ready, s.active, s.completed_total), (1, 0, 1), "{s:?}");
    assert_eq!(client.status().await.workers, "none");
}

slot!(DeadlineSlot);
pub async fn shutdown_deadline_returns_jobs_to_the_queue(client: JobClient) {
    let handle = fast(JobRunner::with_client(client.clone()))
        .shutdown_timeout(Duration::from_millis(50))
        .register(sleep::<DeadlineSlot>())
        .logger(|_, _| {})
        .start()
        .unwrap();
    client.enqueue(sleep::<DeadlineSlot>(), 60_000).await.unwrap();
    wait_until("the job to start", || logged::<DeadlineSlot>().len() == 1).await;
    let started = Instant::now();
    handle.shutdown().await;
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(logged::<DeadlineSlot>(), ["start 1"]);
    let s = client.stats("q").await.unwrap();
    assert_eq!((s.ready + s.scheduled, s.active), (1, 0), "{s:?}");
}

slot!(ReapSlot);
/// A worker that dies keeps its claim until the visibility timeout; the
/// reaper of a healthy runner then hands the job to someone else.
pub async fn lost_workers_are_reaped(client: JobClient) {
    let backend = client.backend().clone();
    client.enqueue(sleep::<ReapSlot>(), 1).await.unwrap();
    // "Crash": claim and never acknowledge.
    let stolen = backend.claim("q", Duration::from_millis(150)).await.unwrap().expect("a job");
    assert_eq!(stolen.attempt, 1);
    assert!(backend.claim("q", Duration::from_millis(150)).await.unwrap().is_none());
    assert_eq!(client.stats("q").await.unwrap().active, 1);
    // Heartbeats keep it alive.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(backend.heartbeat(&stolen, Duration::from_millis(150)).await.unwrap());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(backend.reap("q").await.unwrap(), Reaped::default());

    let handle = fast(JobRunner::with_client(client.clone()))
        .reap_interval(Duration::from_millis(30))
        .register(sleep::<ReapSlot>())
        .logger(|_, _| {})
        .start()
        .unwrap();
    wait_until("the second attempt", || logged::<ReapSlot>() == ["start 2", "end 2"]).await;
    wait_for("completion", || async { client.stats("q").await.unwrap().completed_total == 1 }).await;
    // The dead worker's token is stale: its acknowledgements change nothing.
    assert!(!backend.heartbeat(&stolen, Duration::from_secs(1)).await.unwrap());
    assert!(!backend.complete(&stolen).await.unwrap());
    assert!(!backend.retry(&stolen, 0, "x").await.unwrap());
    assert!(!backend.dead_letter(&stolen, "x").await.unwrap());
    assert_eq!(client.metrics().reaped, 1);

    // Out of attempts (max 2): a second lost claim goes to the dead-letter list.
    handle.shutdown().await;
    client.job(sleep::<ReapSlot>(), 1).max_attempts(1).send().await.unwrap();
    let lost = backend.claim("q", Duration::from_millis(30)).await.unwrap().expect("a job");
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert_eq!(backend.reap("q").await.unwrap(), Reaped { requeued: 0, dead: 1 });
    let dead = client.dead_jobs("q", 5).await.unwrap();
    assert_eq!(dead[0].record.id, lost.record.id);
    assert!(dead[0].error.contains("worker lost"));
}

/// Two runners on one backend stand in for two instances of a deployment.
pub async fn cron_ticks_run_on_one_instance(a: JobClient, b: JobClient) {
    let runs = Arc::new(Mutex::new(Vec::new()));
    let make = |client: JobClient, who: &'static str| {
        let runs = runs.clone();
        let seen = runs.clone();
        fast(JobRunner::with_client(client))
            .schedule("tick", "* * * * * *", move || {
                let runs = runs.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    runs.lock().unwrap().push(("run", who.to_owned()));
                }
            })
            .on_start(move |info| {
                assert_eq!(info.kind, WorkKind::Cron);
                seen.lock().unwrap().push(("start", info.id.clone()));
            })
    };
    let (ha, hb) = (make(a.clone(), "a").start().unwrap(), make(b.clone(), "b").start().unwrap());
    wait_until("three ticks", || runs.lock().unwrap().iter().filter(|r| r.0 == "run").count() >= 3).await;
    ha.shutdown().await;
    let status = b.status().await;
    hb.shutdown().await;

    let runs = runs.lock().unwrap().clone();
    let mut ticks: Vec<&String> = runs.iter().filter(|r| r.0 == "start").map(|r| &r.1).collect();
    let total = ticks.len();
    ticks.sort();
    ticks.dedup();
    assert_eq!(ticks.len(), total, "a tick ran on both instances: {runs:?}");
    let (ma, mb) = (a.metrics(), b.metrics());
    assert_eq!(ma.cron_runs + mb.cron_runs, total as u64);
    assert!(ma.cron_skipped + mb.cron_skipped >= 2, "{ma:?} {mb:?}");
    assert_eq!(status.cron.len(), 1);
    assert_eq!(status.cron[0].schedule, "* * * * * *");
    assert!(status.cron[0].next_run.is_some());
}

pub async fn status_report_is_json(client: JobClient) {
    let handle = fast(JobRunner::with_client(client.clone()))
        .queue("mail", 2)
        .register(Fatal)
        .schedule("nightly", "0 3 * * *", || async {})
        .start()
        .unwrap();
    client.job(Fatal, ()).queue("mail").delay(Duration::from_secs(3600)).send().await.unwrap();
    let json = serde_json::to_value(client.status().await).unwrap();
    assert_eq!(json["workers"], "running");
    assert_eq!(json["jobs"], serde_json::json!(["fatal"]));
    let mail = json["queues"].as_array().unwrap().iter().find(|q| q["name"] == "mail").unwrap();
    assert_eq!((mail["concurrency"].as_u64(), mail["scheduled"].as_u64()), (Some(2), Some(1)));
    // `q` is served because `Fatal` defaults to it.
    assert!(json["queues"].as_array().unwrap().iter().any(|q| q["name"] == "q" && q["concurrency"] == 10));
    assert_eq!(json["cron"][0]["name"], "nightly");
    assert!(json["cron"][0]["next_run"].as_str().unwrap().ends_with("T03:00:00Z"));
    assert!(json.get("error").is_none());
    handle.shutdown().await;
}

// ── hostile input ───────────────────────────────────────────────────────

/// xorshift64*: deterministic, no dependency.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Pieces that would matter if names were ever spliced into keys, patterns or scripts.
const HOSTILE: [&str; 24] = [
    ":", "{", "}", "{}", "*", "?", "[", "]", "\\", "\n", "\0", "\u{7f}", " ", "'", "\"", "q", "ready", "stats", "job",
    "uniq", "lock", "queues", "\u{e9}", "..",
];

fn hostile_name(rng: &mut Rng, max: usize) -> String {
    match rng.below(12) {
        0 => String::new(),
        1 => "x".repeat(max + 1 + rng.below(3)),
        2 => "\u{e9}".repeat(max / 2), // exactly at the limit, in bytes
        _ => (0..=rng.below(4)).map(|_| HOSTILE[rng.below(HOSTILE.len())]).collect(),
    }
}

fn acceptable(name: &str, max: usize) -> bool {
    !name.is_empty() && name.len() <= max && !name.chars().any(char::is_control)
}

/// Whatever the names are, a job is either refused or lands in exactly the
/// queue it names, under exactly the unique key it names.
pub async fn fuzzed_names_never_cross_queues(client: JobClient) {
    use std::collections::{BTreeMap, HashMap};
    let mut rng = Rng(0xC0FF_EE00_D15E_A5E5);
    let mut expected: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut holders: HashMap<String, String> = HashMap::new();
    let (mut refused, mut duplicates) = (0, 0);
    for _ in 0..160 {
        let queue = hostile_name(&mut rng, MAX_NAME_LEN);
        let name = hostile_name(&mut rng, MAX_NAME_LEN);
        let unique = (rng.below(2) == 0).then(|| hostile_name(&mut rng, MAX_UNIQUE_KEY_LEN));
        let valid = acceptable(&queue, MAX_NAME_LEN)
            && acceptable(&name, MAX_NAME_LEN)
            && unique.as_deref().is_none_or(|k| acceptable(k, MAX_UNIQUE_KEY_LEN));
        let mut call = client.raw(&name, serde_json::json!(null), JobOptions::new().queue(queue.clone()));
        if let Some(key) = &unique {
            call = call.unique(key.clone());
        }
        match call.send().await {
            Err(Error::Invalid(_)) => {
                assert!(!valid, "refused a valid job: {queue:?} {name:?} {unique:?}");
                refused += 1;
            }
            Err(e) => panic!("{queue:?} {name:?} {unique:?}: {e}"),
            Ok(out) => {
                assert!(valid, "accepted an invalid job: {queue:?} {name:?} {unique:?}");
                match unique.as_ref().and_then(|k| holders.get(k)) {
                    Some(holder) => {
                        assert!(out.duplicate, "{unique:?} should be taken");
                        assert_eq!(&out.id, holder);
                        duplicates += 1;
                    }
                    None => {
                        assert!(!out.duplicate, "{unique:?} collided with another key");
                        if let Some(key) = unique {
                            holders.insert(key, out.id.clone());
                        }
                        expected.entry(queue).or_default().push((out.id, name));
                    }
                }
            }
        }
    }
    assert!(refused > 20 && duplicates > 3 && expected.len() > 20, "{refused} {duplicates} {}", expected.len());
    assert_eq!(client.backend().queues().await.unwrap(), expected.keys().cloned().collect::<Vec<_>>());
    for (queue, jobs) in &expected {
        assert_eq!(client.stats(queue).await.unwrap().ready, jobs.len() as u64, "queue {queue:?}");
        let mut claimed = Vec::new();
        while let Some(job) = client.backend().claim(queue, Duration::from_secs(30)).await.unwrap() {
            assert_eq!(&job.record.queue, queue);
            assert!(client.backend().complete(&job).await.unwrap());
            claimed.push((job.record.id, job.record.name));
        }
        assert_eq!(&claimed, jobs, "queue {queue:?}: first in, first out, nothing foreign");
    }
    // Every key was released with its job.
    for key in holders.keys() {
        let again = client.raw("again", serde_json::json!(null), JobOptions::new()).unique(key.clone()).send().await;
        assert!(!again.unwrap().duplicate, "{key:?} is still held");
    }
}

pub async fn oversized_input_and_extreme_durations(client: JobClient) {
    // The default limit is 1 MiB of JSON.
    let big = serde_json::json!("x".repeat(DEFAULT_MAX_PAYLOAD_BYTES));
    let err = client.raw("big", big.clone(), JobOptions::new()).send().await.unwrap_err();
    assert!(matches!(&err, Error::Invalid(m) if m.contains("larger than 1048576 bytes")), "{err}");
    let fits = serde_json::json!("x".repeat(DEFAULT_MAX_PAYLOAD_BYTES - 2));
    assert!(client.raw("big", fits, JobOptions::new()).send().await.is_ok());
    let small = client.clone().max_payload_bytes(16);
    assert!(small.raw("n", serde_json::json!([1, 2, 3]), JobOptions::new()).send().await.is_ok());
    assert!(small.raw("n", serde_json::json!("seventeen bytes.."), JobOptions::new()).send().await.is_err());
    let unlimited = client.clone().max_payload_bytes(usize::MAX);
    assert!(unlimited.raw("big", big, JobOptions::new()).send().await.is_ok());
    assert_eq!(client.stats(DEFAULT_QUEUE).await.unwrap().enqueued_total, 3);

    // Durations nobody means, but which must not overflow anything.
    let forever = Duration::MAX;
    let options = || JobOptions::new().queue("far").timeout(forever).backoff(Backoff::exponential(forever, forever));
    let a = client.raw("n", serde_json::json!(1), options()).delay(forever).send().await.unwrap();
    let b = client.raw("n", serde_json::json!(2), options()).unique_for("far:1", forever).send().await.unwrap();
    assert!(
        client.raw("n", serde_json::json!(3), options()).unique_for("far:1", forever).send().await.unwrap().duplicate
    );
    let stats = client.stats("far").await.unwrap();
    assert_eq!((stats.scheduled, stats.ready), (1, 1));
    let job = client.backend().claim("far", forever).await.unwrap().unwrap();
    assert_eq!(job.record.id, b.id);
    assert!(client.backend().heartbeat(&job, forever).await.unwrap());
    assert!(client.backend().retry(&job, u64::MAX, "later").await.unwrap());
    assert_eq!(client.backend().reap("far").await.unwrap(), Reaped::default());
    assert_eq!(client.stats("far").await.unwrap().scheduled, 2);
    assert!(client.backend().try_lock("far", "me", forever).await.unwrap());
    assert!(!client.backend().try_lock("far", "you", forever).await.unwrap());
    assert!(client.cancel("far", &a.id).await.unwrap());
}

pub struct Verbose;
impl Job for Verbose {
    type Input = ();
    const NAME: &'static str = "verbose";
    fn options() -> JobOptions {
        JobOptions::new().queue("q")
    }
    async fn run(_: (), _: JobContext) -> Result<(), JobError> {
        Err(JobError::fatal("\u{e9}".repeat(600_000)))
    }
}

slot!(HooksSlot);
/// Hooks and the log sink are application code: when they panic the job is
/// still acknowledged and the loops keep running. Error messages are cut.
pub async fn long_errors_are_cut_and_hook_panics_are_contained(client: JobClient) {
    let handle = fast(JobRunner::with_client(client.clone()))
        .register(Verbose)
        .register(record::<HooksSlot>())
        .schedule("noisy", "* * * * * *", || async { Err::<(), _>(JobError::msg("y".repeat(100_000))) })
        .on_start(|_| panic!("start hook"))
        .on_success(|_, _| panic!("success hook"))
        .on_failure(|_, _| panic!("failure hook"))
        .logger(|_, _| panic!("log sink"))
        .start()
        .unwrap();
    client.enqueue(Verbose, ()).await.unwrap();
    client.enqueue(record::<HooksSlot>(), note("still here")).await.unwrap();
    wait_for("both outcomes", || async {
        let s = client.stats("q").await.unwrap();
        s.dead == 1 && s.completed_total == 1 && s.active == 0
    })
    .await;
    let dead = client.dead_jobs("q", 10).await.unwrap();
    assert!(dead[0].error.len() <= MAX_ERROR_LEN && dead[0].error.ends_with("(truncated)"), "{}", dead[0].error.len());
    assert!(dead[0].error.starts_with('\u{e9}'));
    // The cron loop survives its hooks, tick after tick.
    wait_until("two cron runs", || client.metrics().cron_failures >= 2).await;
    let status = client.status().await;
    assert!(status.cron[0].last_error.as_ref().unwrap().len() <= MAX_ERROR_LEN);
    assert_eq!(status.queues.iter().find(|q| q.name == "q").unwrap().in_flight, 0);
    handle.shutdown().await;
}
