//! The in-memory backend, end to end.

mod common;

use std::sync::Arc;
use std::time::Duration;

use next_rust_jobs::*;

fn client() -> JobClient {
    JobClient::new(MemoryBackend::new())
}

macro_rules! scenarios {
    ($($name:ident),* $(,)?) => {$(
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn $name() {
            common::$name(client()).await;
        }
    )*};
}

scenarios!(
    runs_a_job_and_cleans_up,
    delayed_jobs_wait,
    retries_then_succeeds,
    exhausted_jobs_are_dead_lettered_and_can_be_revived,
    fatal_timeout_panic_and_unknown,
    unique_jobs_and_idempotency_keys,
    priorities_then_fifo,
    concurrency_limit_per_queue,
    shutdown_finishes_in_flight_jobs,
    shutdown_deadline_returns_jobs_to_the_queue,
    lost_workers_are_reaped,
    status_report_is_json,
    fuzzed_names_never_cross_queues,
    oversized_input_and_extreme_durations,
    long_errors_are_cut_and_hook_panics_are_contained,
);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cron_ticks_run_on_one_instance() {
    let backend: Arc<dyn Backend> = Arc::new(MemoryBackend::new());
    common::cron_ticks_run_on_one_instance(JobClient::from_arc(backend.clone()), JobClient::from_arc(backend)).await;
}

#[tokio::test]
async fn dead_letter_list_is_bounded() {
    let client = JobClient::new(MemoryBackend::new().dead_limit(2));
    let backend = client.backend().clone();
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(client.enqueue(common::Fatal, ()).await.unwrap().id);
        let job = backend.claim("q", Duration::from_secs(5)).await.unwrap().unwrap();
        assert!(backend.dead_letter(&job, "no").await.unwrap());
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let dead: Vec<String> = client.dead_jobs("q", 10).await.unwrap().into_iter().map(|d| d.record.id).collect();
    assert_eq!(dead, [ids[2].clone(), ids[1].clone()], "newest first, oldest dropped");
    assert_eq!(client.stats("q").await.unwrap().dead_total, 3);
}

#[tokio::test]
async fn configuration_errors_surface_at_start() {
    let runner = JobRunner::new().schedule("bad", "61 * * * *", || async {});
    let err = runner.validate().unwrap_err().to_string();
    assert!(err.contains("bad") && err.contains("minute"), "{err}");
    assert!(runner.start().is_err());
    let twice = JobRunner::new().register(common::Fatal).register(common::Fatal);
    assert!(twice.validate().unwrap_err().to_string().contains("two jobs are named `fatal`"));
    let twice = JobRunner::new().schedule("a", "@daily", || async {}).schedule("a", "@hourly", || async {});
    assert!(twice.validate().is_err());
    // Names that could not be deliberate.
    assert!(JobRunner::new().queue("", 1).validate().unwrap_err().to_string().contains("queue name is empty"));
    assert!(JobRunner::new().queue("a\nb", 1).validate().unwrap_err().to_string().contains("control character"));
    assert!(JobRunner::new().queue(&"q".repeat(MAX_NAME_LEN + 1), 1).validate().is_err());
    assert!(JobRunner::new().schedule("tick\0", "@daily", || async {}).validate().is_err());
    // A hostile expression is an error, not a panic or a busy loop.
    for expr in ["@every 0s", "*/0 * * * *", "*/4294967296 * * * *", &"1,".repeat(200)] {
        assert!(JobRunner::new().schedule("t", expr, || async {}).validate().is_err(), "{expr}");
    }
    // The widest intervals are accepted and clamped.
    let wide = JobRunner::new()
        .reap_interval(Duration::MAX)
        .visibility_timeout(Duration::MAX)
        .poll_interval(Duration::MAX)
        .cron_lock_ttl(Duration::MAX)
        .shutdown_timeout(Duration::MAX);
    assert!(wide.validate().is_ok());
}

#[tokio::test]
async fn global_enqueue_uses_the_installed_client() {
    // The global client is one per process and every `start()` in this binary
    // installs its own, so a test running in parallel can replace ours between
    // `install_global` and `enqueue_in`. Try again until the pair ran undisturbed:
    // a job that `client` can cancel was enqueued through the client installed here.
    let client = client();
    for _ in 0..200 {
        client.install_global();
        let out = enqueue_in(Duration::from_secs(60), common::Fatal, ()).await.unwrap();
        if client.cancel("q", &out.id).await.unwrap() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("enqueue_in never reached the installed client");
}

#[tokio::test]
async fn max_jobs_bounds_the_store() {
    let client = JobClient::new(MemoryBackend::new().max_jobs(2));
    let first = client.job(common::Fatal, ()).unique("k").send().await.unwrap();
    client.enqueue(common::Fatal, ()).await.unwrap();
    let err = client.enqueue(common::Fatal, ()).await.unwrap_err();
    assert!(matches!(&err, Error::Backend(m) if m.contains("full")), "{err}");
    // A duplicate stores nothing, so it is still answered.
    assert!(client.job(common::Fatal, ()).unique("k").send().await.unwrap().duplicate);
    // A refused job must not keep its unique key.
    assert!(client.job(common::Fatal, ()).unique("other").send().await.is_err());
    assert!(client.cancel("q", &first.id).await.unwrap());
    assert!(!client.job(common::Fatal, ()).unique("other").send().await.unwrap().duplicate);
}
