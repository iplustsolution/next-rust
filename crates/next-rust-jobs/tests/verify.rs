//! Independent verification of the hardening claims, plus attempts to break
//! them with concurrency. Written from the outside: nothing here reaches into
//! crate internals, so a claim that is only true of a private helper fails.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use next_rust_jobs::*;

// ---------------------------------------------------------------------------
// Claim: a payload over the limit is refused with `Error::Invalid`.
// ---------------------------------------------------------------------------

struct Nothing;
impl Job for Nothing {
    type Input = serde_json::Value;
    const NAME: &'static str = "nothing";
    fn options() -> JobOptions {
        JobOptions::new().queue("v")
    }
    async fn run(_: serde_json::Value, _: JobContext) -> Result<(), JobError> {
        Ok(())
    }
}

#[tokio::test]
async fn an_oversized_payload_is_invalid_and_never_reaches_the_backend() {
    let client = JobClient::new(MemoryBackend::new());
    // One byte over the serialised limit.
    let over = serde_json::json!("x".repeat(DEFAULT_MAX_PAYLOAD_BYTES - 1));
    let err = client.enqueue(Nothing, over).await.unwrap_err();
    assert!(matches!(err, Error::Invalid(_)), "{err:?}");
    assert_eq!(client.stats("v").await.unwrap().enqueued_total, 0, "nothing was stored");
    assert_eq!(client.metrics().enqueued, 0);
    // The typed path is bounded too, not only `raw`.
    let just_under = serde_json::json!("x".repeat(DEFAULT_MAX_PAYLOAD_BYTES - 3));
    assert!(client.enqueue(Nothing, just_under).await.is_ok());
    // The refusal names the limit but not the payload.
    let huge = serde_json::json!("SENSITIVE".repeat(DEFAULT_MAX_PAYLOAD_BYTES));
    let Err(Error::Invalid(message)) = client.enqueue(Nothing, huge).await else { panic!("accepted") };
    assert!(!message.contains("SENSITIVE"), "the refusal echoed the payload: {message}");
}

// ---------------------------------------------------------------------------
// Claim: names are validated at `send()` *and* inside the backend.
// ---------------------------------------------------------------------------

/// Every rejection reason, checked twice: once through the client (which could
/// be doing the work on its own) and once by handing the backend a record that
/// never passed through `send`.
#[tokio::test]
async fn names_are_checked_at_send_and_again_by_the_backend() {
    let backend: Arc<dyn Backend> = Arc::new(MemoryBackend::new());
    let client = JobClient::from_arc(backend.clone());

    let bad_names = [
        String::new(),
        "x".repeat(MAX_NAME_LEN + 1),
        "a\nb".to_owned(),
        "a\0b".to_owned(),
        "\u{7f}".to_owned(),
        "tab\there".to_owned(),
    ];
    for bad in &bad_names {
        // Job name.
        let out = client.raw(bad, serde_json::json!(null), JobOptions::new().queue("v")).send().await;
        assert!(matches!(out, Err(Error::Invalid(_))), "job name {bad:?} was accepted");
        // Queue name.
        let out = client.raw("ok", serde_json::json!(null), JobOptions::new().queue(bad.clone())).send().await;
        assert!(matches!(out, Err(Error::Invalid(_))), "queue {bad:?} was accepted");
        // Unique keys have their own, larger length limit, so only the
        // shape rules apply to every name here.
        if bad.is_empty() || bad.chars().any(char::is_control) {
            let out = client
                .raw("ok", serde_json::json!(null), JobOptions::new().queue("v"))
                .unique(bad.clone())
                .send()
                .await;
            assert!(matches!(out, Err(Error::Invalid(_))), "unique key {bad:?} was accepted");
            let mut record = valid_record();
            record.unique_key = Some(bad.clone());
            assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))), "backend took key {bad:?}");
        }

        // Straight to the backend, bypassing `send` entirely.
        let mut record = valid_record();
        record.name = bad.clone();
        assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))), "backend took job name {bad:?}");
        let mut record = valid_record();
        record.queue = bad.clone();
        assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))), "backend took queue {bad:?}");
        let mut record = valid_record();
        record.id = bad.clone();
        assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))), "backend took id {bad:?}");
    }
    // An over-long unique key, at its own limit.
    let mut record = valid_record();
    record.unique_key = Some("k".repeat(MAX_UNIQUE_KEY_LEN + 1));
    assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))), "backend took an over-long key");
    // A key of exactly the limit is fine; one byte more is not.
    let at = "k".repeat(MAX_UNIQUE_KEY_LEN);
    assert!(client.raw("ok", serde_json::json!(null), JobOptions::new()).unique(at.clone()).send().await.is_ok());
    let over = format!("{at}k");
    assert!(client.raw("ok", serde_json::json!(null), JobOptions::new()).unique(over).send().await.is_err());
    // Nothing invalid was ever stored.
    // Every accepted job went to the default queue; not one refused name
    // created the queue it asked for.
    assert_eq!(backend.queues().await.unwrap(), vec![DEFAULT_QUEUE.to_owned()]);
}

/// A record built by hand, as a custom backend or another binary would send
/// one: it never passes through `Enqueue::send`.
fn valid_record() -> JobRecord {
    JobRecord {
        id: format!("hand-made-{}", now_ms()),
        name: "ok".to_owned(),
        queue: "v".to_owned(),
        payload: serde_json::json!(null),
        priority: 0,
        max_attempts: 1,
        timeout_ms: 1_000,
        backoff: Backoff::fixed(Duration::from_millis(1)),
        unique_key: None,
        unique_ttl_ms: None,
        enqueued_at_ms: now_ms(),
        run_at_ms: now_ms(),
    }
}

#[test]
fn validate_is_public_so_custom_backends_can_call_it() {
    let mut record = valid_record();
    assert!(record.validate().is_ok());
    record.queue = "a\rb".into();
    assert!(matches!(record.validate(), Err(Error::Invalid(_))));
}

// ---------------------------------------------------------------------------
// Claim: error messages are truncated to MAX_ERROR_LEN wherever they are kept.
// ---------------------------------------------------------------------------

struct Loud;
impl Job for Loud {
    type Input = ();
    const NAME: &'static str = "loud";
    fn options() -> JobOptions {
        JobOptions::new().queue("loud").max_attempts(1)
    }
    async fn run(_: (), _: JobContext) -> Result<(), JobError> {
        // Multi-byte, so a naive byte cut would panic.
        Err(JobError::fatal("\u{4e16}".repeat(400_000)))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn long_errors_are_cut_in_the_dead_letter_list_and_in_the_status_report() {
    let client = JobClient::new(MemoryBackend::new());
    let handle = JobRunner::with_client(client.clone())
        .poll_interval(Duration::from_millis(5))
        .register(Loud)
        .schedule("shout", "* * * * * *", || async { Err::<(), _>(JobError::msg("z".repeat(200_000))) })
        .logger(|_, _| {})
        .start()
        .unwrap();
    client.enqueue(Loud, ()).await.unwrap();
    wait(|| async { client.stats("loud").await.unwrap().dead == 1 }).await;
    let dead = client.dead_jobs("loud", 1).await.unwrap();
    assert!(dead[0].error.len() <= MAX_ERROR_LEN, "{} bytes stored", dead[0].error.len());
    assert!(dead[0].error.starts_with('\u{4e16}'), "cut on a character boundary");

    wait(|| async { client.metrics().cron_failures >= 1 }).await;
    let status = client.status().await;
    let cron_error = status.cron[0].last_error.clone().expect("a failure was recorded");
    assert!(cron_error.len() <= MAX_ERROR_LEN, "{} bytes in the status report", cron_error.len());
    handle.shutdown().await;
}

type Fut<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, Error>> + Send + 'a>>;

fn loud_error<T: Send + 'static>(tag: char) -> Fut<'static, T> {
    Box::pin(async move { Err(Error::Backend(tag.to_string().repeat(100_000))) })
}

fn quiet<T: Send + 'static>(value: T) -> Fut<'static, T> {
    Box::pin(std::future::ready(Ok(value)))
}

/// A backend whose every call fails with a runaway message: the status report
/// must not grow with it.
struct Screaming;

impl Backend for Screaming {
    fn kind(&self) -> &'static str {
        "screaming"
    }
    fn enqueue(&self, _: JobRecord) -> Fut<'_, Enqueued> {
        loud_error('e')
    }
    fn claim<'a>(&'a self, _: &'a str, _: Duration) -> Fut<'a, Option<ClaimedJob>> {
        loud_error('c')
    }
    fn heartbeat<'a>(&'a self, _: &'a ClaimedJob, _: Duration) -> Fut<'a, bool> {
        quiet(false)
    }
    fn complete<'a>(&'a self, _: &'a ClaimedJob) -> Fut<'a, bool> {
        quiet(false)
    }
    fn retry<'a>(&'a self, _: &'a ClaimedJob, _: u64, _: &'a str) -> Fut<'a, bool> {
        quiet(false)
    }
    fn dead_letter<'a>(&'a self, _: &'a ClaimedJob, _: &'a str) -> Fut<'a, bool> {
        quiet(false)
    }
    fn reap<'a>(&'a self, _: &'a str) -> Fut<'a, Reaped> {
        quiet(Reaped::default())
    }
    fn cancel<'a>(&'a self, _: &'a str, _: &'a str) -> Fut<'a, bool> {
        quiet(false)
    }
    fn stats<'a>(&'a self, _: &'a str) -> Fut<'a, QueueStats> {
        loud_error('s')
    }
    fn queues(&self) -> Fut<'_, Vec<String>> {
        Box::pin(async { Ok(vec!["one".to_owned(), "two".to_owned()]) })
    }
    fn dead_jobs<'a>(&'a self, _: &'a str, _: usize) -> Fut<'a, Vec<DeadJob>> {
        quiet(Vec::new())
    }
    fn retry_dead<'a>(&'a self, _: &'a str, _: &'a str) -> Fut<'a, bool> {
        quiet(false)
    }
    fn purge_dead<'a>(&'a self, _: &'a str) -> Fut<'a, usize> {
        quiet(0)
    }
    fn try_lock<'a>(&'a self, _: &'a str, _: &'a str, _: Duration) -> Fut<'a, bool> {
        quiet(false)
    }
}

#[tokio::test]
async fn a_backend_error_in_the_status_report_is_truncated() {
    let client = JobClient::new(Screaming);
    let report = client.status().await;
    let error = report.error.clone().expect("the backend failed");
    assert!(error.len() <= MAX_ERROR_LEN, "{} bytes of backend error in the report", error.len());
    assert!(serde_json::to_string(&report).unwrap().len() < 3 * MAX_ERROR_LEN, "the whole report grew with it");
}

// ---------------------------------------------------------------------------
// Claim: hook and log-sink panics are contained.
// ---------------------------------------------------------------------------

struct Counted;
static RAN: AtomicU32 = AtomicU32::new(0);
impl Job for Counted {
    type Input = u32;
    const NAME: &'static str = "counted";
    fn options() -> JobOptions {
        JobOptions::new().queue("hooks").max_attempts(1)
    }
    async fn run(n: u32, _: JobContext) -> Result<(), JobError> {
        RAN.fetch_add(1, Ordering::SeqCst);
        if n.is_multiple_of(2) { Ok(()) } else { Err(JobError::fatal("odd")) }
    }
}

/// Every hook and the log sink panic on every call. A worker must keep
/// claiming and a cron loop must keep ticking, and every job must still be
/// acknowledged (nothing left `active`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panicking_hooks_never_stop_a_worker_or_a_cron_loop() {
    let client = JobClient::new(MemoryBackend::new());
    let ticks = Arc::new(AtomicU32::new(0));
    let counted = ticks.clone();
    let handle = JobRunner::with_client(client.clone())
        .poll_interval(Duration::from_millis(5))
        .queue("hooks", 2)
        .register(Counted)
        .schedule("tick", "* * * * * *", move || {
            let ticks = counted.clone();
            async move {
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        })
        .on_start(|_| panic!("start hook"))
        .on_success(|_, _| panic!("success hook"))
        .on_failure(|_, _| panic!("failure hook"))
        .logger(|_, _| panic!("log sink"))
        .start()
        .unwrap();
    for n in 0..20 {
        client.enqueue(Counted, n).await.unwrap();
    }
    wait(|| async { RAN.load(Ordering::SeqCst) >= 20 }).await;
    wait(|| async {
        let s = client.stats("hooks").await.unwrap();
        s.completed_total == 10 && s.dead == 10 && s.active == 0 && s.ready == 0
    })
    .await;
    // The cron loop, whose `on_start`/`on_success` panic on every tick, keeps
    // firing: it did not die on the first one.
    wait(|| async { ticks.load(Ordering::SeqCst) >= 2 }).await;
    handle.shutdown().await;
}

// ---------------------------------------------------------------------------
// Claim: `MemoryBackend::max_jobs` bounds the store.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn max_jobs_refuses_instead_of_growing_without_bound() {
    let client = JobClient::new(MemoryBackend::new().max_jobs(50));
    for _ in 0..50 {
        client.raw("n", serde_json::json!(1), JobOptions::new().queue("cap")).send().await.unwrap();
    }
    for _ in 0..20 {
        let err = client.raw("n", serde_json::json!(1), JobOptions::new().queue("cap")).send().await.unwrap_err();
        assert!(matches!(&err, Error::Backend(m) if m.contains("full")), "{err}");
    }
    assert_eq!(client.stats("cap").await.unwrap().ready, 50, "the bound held");
    // `max_jobs(0)` cannot lock the backend out entirely.
    let one = JobClient::new(MemoryBackend::new().max_jobs(0));
    assert!(one.raw("n", serde_json::json!(1), JobOptions::new()).send().await.is_ok());
}

// ---------------------------------------------------------------------------
// Claim: hostile cron expressions and extreme times do not overflow or hang.
// ---------------------------------------------------------------------------

#[test]
fn hostile_cron_expressions_terminate_and_never_go_backwards() {
    let started = std::time::Instant::now();
    let expressions = [
        "*/4294967295 * * * *",
        "5/4294967295 * * * *",
        "0-59/4294967295 0-23/4294967295 1-31/4294967295 1-12/4294967295 0-7/4294967295",
        "@every 213503982334601d",
        "@every 1s",
        "* * * * * *",
        "0 0 29 2 *",
    ];
    for expr in expressions {
        let cron: Cron = expr.parse().expect(expr);
        for now in [0u64, 1, i64::MAX as u64, u64::MAX - 1, u64::MAX, 1 << 62, 253_402_300_798, 253_402_300_799] {
            match cron.next_after(now) {
                Some(next) => {
                    assert!(next > now, "{expr}: {next} is not after {now}");
                    assert!(cron.matches(next), "{expr}: does not match its own tick");
                }
                None => assert!(cron.upcoming(now, 3).is_empty()),
            }
        }
    }
    // Expressions that cannot parse are errors, never panics or hangs.
    for bad in ["*/4294967296 * * * *", "*/0 * * * *", "@every 0s", "@every 18446744073709551615d", ""] {
        assert!(bad.parse::<Cron>().is_err(), "{bad}");
    }
    assert!(started.elapsed() < Duration::from_secs(10), "took {:?}", started.elapsed());
}

// ---------------------------------------------------------------------------
// Breaking attempts.
// ---------------------------------------------------------------------------

/// A job must never be handed to two workers at once, however many claim in
/// parallel, and only the worker holding the current token may acknowledge it.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_job_is_never_claimed_by_two_workers() {
    let backend: Arc<dyn Backend> = Arc::new(MemoryBackend::new());
    let client = JobClient::from_arc(backend.clone());
    for _ in 0..200 {
        client.raw("n", serde_json::json!(1), JobOptions::new().queue("race")).send().await.unwrap();
    }
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let (backend, seen) = (backend.clone(), seen.clone());
        tasks.push(tokio::spawn(async move {
            let mut mine = Vec::new();
            while let Ok(Some(job)) = backend.claim("race", Duration::from_secs(30)).await {
                mine.push(job.record.id.clone());
                assert!(backend.complete(&job).await.unwrap(), "the claimer could not acknowledge");
                assert!(!backend.complete(&job).await.unwrap(), "a second acknowledgement was accepted");
            }
            seen.lock().unwrap().extend(mine);
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    let mut ids = seen.lock().unwrap().clone();
    let total = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!((ids.len(), total), (200, 200), "a job was claimed twice");
    assert_eq!(client.stats("race").await.unwrap().completed_total, 200);
}

/// Many tasks enqueue the same unique key at once: exactly one may win, and
/// every loser must be told the winner's id.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_unique_key_has_exactly_one_winner_under_contention() {
    let client = JobClient::new(MemoryBackend::new());
    for round in 0..20 {
        let key = format!("pay:{round}");
        let mut tasks = Vec::new();
        for _ in 0..24 {
            let (client, key) = (client.clone(), key.clone());
            tasks.push(tokio::spawn(async move {
                client.raw("n", serde_json::json!(1), JobOptions::new().queue("uniq")).unique(key).send().await
            }));
        }
        let mut winners = Vec::new();
        let mut ids = Vec::new();
        for task in tasks {
            let out = task.await.unwrap().unwrap();
            if !out.duplicate {
                winners.push(out.id.clone());
            }
            ids.push(out.id);
        }
        assert_eq!(winners.len(), 1, "round {round}: {} jobs hold one key", winners.len());
        assert!(ids.iter().all(|id| *id == winners[0]), "a duplicate was told the wrong id");
    }
    assert_eq!(client.stats("uniq").await.unwrap().ready, 20);
}

/// Releasing a unique key twice (complete, then cancel; complete twice; a
/// stale worker's acknowledgement) must not free a *later* job's key.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_unique_window_is_never_released_twice() {
    let backend: Arc<dyn Backend> = Arc::new(MemoryBackend::new());
    let client = JobClient::from_arc(backend.clone());
    let enqueue = |key: &'static str| {
        let client = client.clone();
        async move { client.raw("n", serde_json::json!(1), JobOptions::new().queue("rel")).unique(key).send().await }
    };

    let first = enqueue("k").await.unwrap();
    let job = backend.claim("rel", Duration::from_secs(30)).await.unwrap().unwrap();
    assert!(backend.complete(&job).await.unwrap());
    // The key is free: a second job takes it.
    let second = enqueue("k").await.unwrap();
    assert!(!second.duplicate);
    assert_ne!(second.id, first.id);

    // Now replay the first job's acknowledgements. None of them may release
    // the *second* job's hold on the same key.
    assert!(!backend.complete(&job).await.unwrap(), "a spent claim completed again");
    assert!(!backend.dead_letter(&job, "x").await.unwrap());
    assert!(!backend.retry(&job, 0, "x").await.unwrap());
    assert!(!client.cancel("rel", &first.id).await.unwrap(), "a finished job was cancelled");
    let third = enqueue("k").await.unwrap();
    assert!(third.duplicate, "the second job's key was released by the first job's replay");
    assert_eq!(third.id, second.id);

    // Cancelling the holder does release it, exactly once.
    assert!(client.cancel("rel", &second.id).await.unwrap());
    assert!(!client.cancel("rel", &second.id).await.unwrap());
    assert!(!enqueue("k").await.unwrap().duplicate);
}

/// Two runners over one backend, both claiming, while the client enqueues:
/// every job runs exactly once and the queue drains.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn two_runners_on_one_backend_run_each_job_once() {
    static RUNS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    struct Once;
    impl Job for Once {
        type Input = u32;
        const NAME: &'static str = "once";
        fn options() -> JobOptions {
            JobOptions::new().queue("shared").max_attempts(1)
        }
        async fn run(n: u32, _: JobContext) -> Result<(), JobError> {
            RUNS.lock().unwrap().push(n);
            Ok(())
        }
    }
    let backend: Arc<dyn Backend> = Arc::new(MemoryBackend::new());
    let (a, b) = (JobClient::from_arc(backend.clone()), JobClient::from_arc(backend.clone()));
    let runner = |client: JobClient| {
        JobRunner::with_client(client)
            .poll_interval(Duration::from_millis(5))
            .queue("shared", 4)
            .register(Once)
            .logger(|_, _| {})
            .start()
            .unwrap()
    };
    let (ha, hb) = (runner(a.clone()), runner(b.clone()));
    let mut enqueued = Vec::new();
    for n in 0..120u32 {
        enqueued.push(tokio::spawn({
            let a = a.clone();
            async move { a.enqueue(Once, n).await.unwrap() }
        }));
    }
    for task in enqueued {
        task.await.unwrap();
    }
    wait(|| async { a.stats("shared").await.unwrap().completed_total == 120 }).await;
    ha.shutdown().await;
    hb.shutdown().await;
    let mut runs = RUNS.lock().unwrap().clone();
    let total = runs.len();
    runs.sort_unstable();
    runs.dedup();
    assert_eq!((runs.len(), total), (120, 120), "a job ran twice");
    let s = a.stats("shared").await.unwrap();
    assert_eq!((s.ready, s.scheduled, s.active, s.dead), (0, 0, 0, 0), "{s:?}");
}

// ---------------------------------------------------------------------------
// Redis, only when REDIS_URL is set.
// ---------------------------------------------------------------------------

#[cfg(feature = "redis")]
mod redis_checks {
    use super::*;

    fn url() -> Option<String> {
        std::env::var("REDIS_URL").ok().filter(|u| !u.is_empty())
    }

    /// A one-command RESP client over `std::net`, so this test needs no
    /// dependency the crate does not already have. Only used to plant the
    /// corrupt rows a real outage (or a careless migration) would leave.
    fn raw_command(parts: &[&str]) {
        use std::io::{Read, Write};
        let url = url().expect("REDIS_URL");
        let authority = url.trim_start_matches("redis://").trim_end_matches('/');
        let authority = authority.rsplit('@').next().unwrap_or(authority);
        let address = if authority.contains(':') { authority.to_owned() } else { format!("{authority}:6379") };
        let mut stream = std::net::TcpStream::connect(&address).expect("connect to REDIS_URL");
        let mut out = format!("*{}\r\n", parts.len());
        for part in parts {
            out.push_str(&format!("${}\r\n{part}\r\n", part.len()));
        }
        stream.write_all(out.as_bytes()).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reply = [0u8; 64];
        let read = stream.read(&mut reply).unwrap();
        let reply = String::from_utf8_lossy(&reply[..read]).into_owned();
        assert!(!reply.starts_with('-'), "{parts:?}: {reply}");
    }

    async fn backend(suffix: &str) -> Option<Arc<RedisBackend>> {
        let url = url()?;
        let prefix = format!("nr-verify:{}:{suffix}", std::process::id());
        let backend = RedisBackend::connect(&url, &prefix).await.expect("connect");
        backend.clear().await.expect("clear");
        Some(Arc::new(backend))
    }

    /// A record that no longer parses must be dead-lettered once, not fail
    /// every claim, and the `queue`/`id` of the claimed job must come from the
    /// keys rather than from the (possibly lying) stored record.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_corrupt_record_is_dead_lettered_and_the_keys_win() {
        let Some(backend) = backend("corrupt").await else { return };
        let client = JobClient::from_arc(backend.clone() as Arc<dyn Backend>);
        // The honest job outranks the corrupt one, so the claim order is fixed.
        let good =
            client.raw("real", serde_json::json!(1), JobOptions::new().queue("cq")).priority(10).send().await.unwrap();
        let bad = client.raw("bad", serde_json::json!(1), JobOptions::new().queue("cq")).send().await.unwrap();

        // Corrupt one record, and make another one lie about its queue and id.
        raw_command(&["HSET", &format!("{}:job:{}", backend.prefix(), bad.id), "data", "{not json"]);
        let lying = serde_json::json!({
            "id": "somebody-elses-id",
            "name": "real",
            "queue": "another-queue",
            "payload": 1,
            "priority": 0,
            "max_attempts": 5,
            "timeout_ms": 1000,
            "backoff": { "base_ms": 1, "max_ms": 1, "factor": 1.0, "jitter": 0.0 },
            "enqueued_at_ms": 0,
            "run_at_ms": 0
        });
        raw_command(&["HSET", &format!("{}:job:{}", backend.prefix(), good.id), "data", &lying.to_string()]);

        // One claim: the corrupt one is buried on the way, the honest keys win.
        let claimed = backend.claim("cq", Duration::from_secs(30)).await.unwrap().expect("a job");
        assert_eq!(claimed.record.queue, "cq", "the record's queue overrode the key");
        assert_eq!(claimed.record.id, good.id, "the record's id overrode the key");
        // Because the keys won, the claim can be acknowledged.
        assert!(backend.complete(&claimed).await.unwrap());
        // The next claim meets the corrupt record: it answers `Ok(None)` (the
        // record is buried on the way) instead of failing, and repeating the
        // call keeps answering `Ok(None)` instead of failing once per attempt.
        for _ in 0..3 {
            assert!(backend.claim("cq", Duration::from_secs(30)).await.unwrap().is_none());
        }
        let dead = client.dead_jobs("cq", 10).await.unwrap();
        assert_eq!(dead.len(), 1, "the corrupt record was not dead-lettered exactly once");
        assert_eq!(dead[0].record.id, bad.id);
        assert!(dead[0].error.contains("corrupt"), "{}", dead[0].error);
        assert_eq!(client.purge_dead("cq").await.unwrap(), 1);
        backend.clear().await.unwrap();
    }

    /// Delays, visibility timeouts and idempotency windows near `u64::MAX`
    /// must be clamped, not rejected by Redis and not rounded into the past.
    #[tokio::test]
    async fn extreme_ttls_and_scores_are_clamped() {
        let Some(backend) = backend("clamp").await else { return };
        let client = JobClient::from_arc(backend.clone() as Arc<dyn Backend>);
        let forever = Duration::MAX;
        let options = || JobOptions::new().queue("far").timeout(forever);
        let far = client.raw("n", serde_json::json!(1), options()).delay(forever).send().await.unwrap();
        let held = client.raw("n", serde_json::json!(2), options()).unique_for("k", forever).send().await.unwrap();
        assert!(
            client.raw("n", serde_json::json!(3), options()).unique_for("k", forever).send().await.unwrap().duplicate
        );
        let stats = client.stats("far").await.unwrap();
        assert_eq!((stats.scheduled, stats.ready), (1, 1), "{stats:?}");
        let job = backend.claim("far", forever).await.unwrap().expect("the ready one");
        assert_eq!(job.record.id, held.id);
        assert!(backend.heartbeat(&job, forever).await.unwrap());
        assert!(backend.retry(&job, u64::MAX, "later").await.unwrap());
        assert_eq!(backend.reap("far").await.unwrap(), Reaped::default());
        assert!(backend.try_lock("lk", "me", forever).await.unwrap());
        assert!(!backend.try_lock("lk", "you", forever).await.unwrap());
        assert!(client.cancel("far", &far.id).await.unwrap());
        // The clamped delay is still far in the future, never in the past.
        assert_eq!(client.stats("far").await.unwrap().ready, 0);
        backend.clear().await.unwrap();
    }

    /// The same names check as the memory backend, against real Redis.
    #[tokio::test]
    async fn redis_validates_names_too() {
        let Some(backend) = backend("names").await else { return };
        for bad in ["", "a\nb", "\0"] {
            let mut record = valid_record();
            record.queue = bad.to_owned();
            assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))), "queue {bad:?}");
            let mut record = valid_record();
            record.unique_key = Some("x".repeat(MAX_UNIQUE_KEY_LEN + 1));
            assert!(matches!(backend.enqueue(record).await, Err(Error::Invalid(_))));
        }
        assert!(backend.queues().await.unwrap().is_empty(), "a refused job created a queue");
        backend.clear().await.unwrap();
    }

    /// Sixteen claimers against real Redis: no job twice.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn redis_claims_are_exclusive() {
        let Some(backend) = backend("race").await else { return };
        let client = JobClient::from_arc(backend.clone() as Arc<dyn Backend>);
        for _ in 0..100 {
            client.raw("n", serde_json::json!(1), JobOptions::new().queue("rr")).send().await.unwrap();
        }
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let mut tasks = Vec::new();
        for _ in 0..16 {
            let (backend, seen) = (backend.clone(), seen.clone());
            tasks.push(tokio::spawn(async move {
                while let Ok(Some(job)) = backend.claim("rr", Duration::from_secs(30)).await {
                    assert!(backend.complete(&job).await.unwrap());
                    assert!(!backend.complete(&job).await.unwrap());
                    seen.lock().unwrap().push(job.record.id);
                }
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        let mut ids = seen.lock().unwrap().clone();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!((ids.len(), total), (100, 100));
        backend.clear().await.unwrap();
    }
}

// ---------------------------------------------------------------------------

async fn wait<F, Fut>(mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    while !check().await {
        assert!(start.elapsed() < Duration::from_secs(20), "timed out");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
