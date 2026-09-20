//! The Redis backend against a real server. Runs only when `REDIS_URL` is
//! set (`REDIS_URL=redis://localhost:6379 cargo test -p next-rust-jobs --features redis`).
//! Every test works under a key prefix of its own and deletes it afterwards.

#![cfg(feature = "redis")]

mod common;

use std::sync::Arc;
use std::time::Duration;

use next_rust_jobs::*;

fn url() -> Option<String> {
    match std::env::var("REDIS_URL") {
        Ok(url) if !url.is_empty() => Some(url),
        _ => {
            eprintln!("REDIS_URL is not set: skipping");
            None
        }
    }
}

async fn backend(url: &str, test: &str) -> Arc<RedisBackend> {
    let prefix = format!("nr-jobs-test:{test}:{}:{}", std::process::id(), now_ms());
    Arc::new(RedisBackend::connect(url, &prefix).await.expect("connect to REDIS_URL"))
}

macro_rules! scenarios {
    ($($name:ident),* $(,)?) => {$(
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn $name() {
            let Some(url) = url() else { return };
            let backend = backend(&url, stringify!($name)).await;
            // Clean up even when the scenario panics.
            let run = tokio::spawn(common::$name(JobClient::from_arc(backend.clone())));
            let result = run.await;
            backend.clear().await.unwrap();
            if let Err(e) = result {
                std::panic::resume_unwind(e.into_panic());
            }
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
    let Some(url) = url() else { return };
    let backend = backend(&url, "cron").await;
    // Two connections, as two instances would have.
    let other = Arc::new(RedisBackend::connect(&url, backend.prefix()).await.unwrap());
    let run = tokio::spawn(common::cron_ticks_run_on_one_instance(
        JobClient::from_arc(backend.clone()),
        JobClient::from_arc(other),
    ));
    let result = run.await;
    backend.clear().await.unwrap();
    if let Err(e) = result {
        std::panic::resume_unwind(e.into_panic());
    }
}

/// Many workers on several connections: every job runs exactly once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn claims_are_atomic_across_instances() {
    let Some(url) = url() else { return };
    let first = backend(&url, "atomic").await;
    let producer = JobClient::from_arc(first.clone());
    for i in 0..60 {
        producer.raw("n", serde_json::json!(i), JobOptions::new().queue("atomic")).send().await.unwrap();
    }
    let mut workers = Vec::new();
    for _ in 0..4 {
        let backend = RedisBackend::connect(&url, first.prefix()).await.unwrap();
        workers.push(tokio::spawn(async move {
            let mut seen = Vec::new();
            while let Some(job) = backend.claim("atomic", Duration::from_secs(30)).await.unwrap() {
                seen.push(job.record.payload.as_u64().unwrap());
                assert!(backend.complete(&job).await.unwrap());
            }
            seen
        }));
    }
    let mut all = Vec::new();
    for w in workers {
        all.extend(w.await.unwrap());
    }
    all.sort_unstable();
    let stats = producer.stats("atomic").await.unwrap();
    let left = first.clear().await.unwrap();
    assert_eq!(all, (0..60).collect::<Vec<u64>>());
    assert_eq!((stats.completed_total, stats.ready, stats.active), (60, 0, 0));
    // Only the counters and the queue index remain: no job hashes leak.
    assert_eq!(left, 2, "leftover keys");
}

#[tokio::test]
async fn dead_letter_list_is_bounded_and_clear_is_scoped() {
    let Some(url) = url() else { return };
    let prefix = format!("nr-jobs-test:bounded[*]:{}", now_ms());
    let backend = Arc::new(RedisBackend::connect(&url, &prefix).await.unwrap().dead_limit(2));
    let neighbour = Arc::new(RedisBackend::connect(&url, &format!("{prefix}x")).await.unwrap());
    JobClient::from_arc(neighbour.clone()).enqueue(common::Fatal, ()).await.unwrap();
    let client = JobClient::from_arc(backend.clone());
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(client.enqueue(common::Fatal, ()).await.unwrap().id);
        let job = backend.claim("q", Duration::from_secs(5)).await.unwrap().unwrap();
        assert!(backend.dead_letter(&job, "no").await.unwrap());
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let dead: Vec<String> = client.dead_jobs("q", 10).await.unwrap().into_iter().map(|d| d.record.id).collect();
    // 2 dead job hashes + dead set + stats + queues: the third hash was trimmed.
    let cleared = backend.clear().await.unwrap();
    let neighbour_ready = JobClient::from_arc(neighbour.clone()).stats("q").await.unwrap().ready;
    neighbour.clear().await.unwrap();
    assert_eq!(dead, [ids[2].clone(), ids[1].clone()]);
    assert_eq!(cleared, 5);
    assert_eq!(neighbour_ready, 1, "clear() must not touch other prefixes");
}

/// A record that no longer parses (written by something else, or by a future
/// version) is buried on the first claim: it does not fail the claim once per
/// attempt, block the jobs behind it, or make the dead-letter list unreadable.
#[tokio::test]
async fn corrupt_records_are_dead_lettered_at_once() {
    let Some(url) = url() else { return };
    let backend = backend(&url, "corrupt").await;
    let client = JobClient::from_arc(backend.clone());
    let bad = client.raw("n", serde_json::json!(1), JobOptions::new().queue("c")).unique("c:1").send().await.unwrap();
    let good = client.raw("n", serde_json::json!(2), JobOptions::new().queue("c")).send().await.unwrap();
    let mut conn = redis::Client::open(url.as_str()).unwrap().get_multiplexed_async_connection().await.unwrap();
    let _: () = redis::cmd("HSET")
        .arg(format!("{}:job:{}", backend.prefix(), bad.id))
        .arg("data")
        .arg("{\"id\": [[[[ not a record")
        .query_async(&mut conn)
        .await
        .unwrap();
    // A record that lies about where it lives is put right.
    let lying = client.raw("n", serde_json::json!(3), JobOptions::new().queue("c")).send().await.unwrap();
    let forged = serde_json::json!({
        "id": good.id, "name": "n", "queue": "elsewhere", "payload": 3, "priority": 0, "max_attempts": 1,
        "timeout_ms": 0, "backoff": Backoff::default(), "enqueued_at_ms": 0, "run_at_ms": 0,
    });
    let _: () = redis::cmd("HSET")
        .arg(format!("{}:job:{}", backend.prefix(), lying.id))
        .arg("data")
        .arg(forged.to_string())
        .query_async(&mut conn)
        .await
        .unwrap();

    let run = tokio::spawn({
        let (backend, client) = (backend.clone(), client.clone());
        async move {
            let first = backend.claim("c", Duration::from_secs(30)).await.unwrap().unwrap();
            assert_eq!(first.record.id, good.id, "the corrupt job was skipped, not returned as an error");
            assert!(backend.complete(&first).await.unwrap());
            let second = backend.claim("c", Duration::from_secs(30)).await.unwrap().unwrap();
            assert_eq!((second.record.id.as_str(), second.record.queue.as_str()), (lying.id.as_str(), "c"));
            assert!(backend.complete(&second).await.unwrap());
            assert!(backend.claim("c", Duration::from_secs(30)).await.unwrap().is_none());
            let stats = client.stats("c").await.unwrap();
            assert_eq!((stats.dead, stats.active, stats.ready, stats.completed_total), (1, 0, 0, 2));
            let dead = client.dead_jobs("c", 10).await.unwrap();
            assert_eq!(
                (dead[0].record.id.as_str(), dead[0].record.name.as_str()),
                (bad.id.as_str(), "(corrupt record)")
            );
            assert!(dead[0].error.contains("corrupt job record"), "{}", dead[0].error);
            // Its unique key went with it, and the entry can be purged.
            assert!(
                !client
                    .raw("n", serde_json::json!(4), JobOptions::new().queue("c"))
                    .unique("c:1")
                    .send()
                    .await
                    .unwrap()
                    .duplicate
            );
            assert_eq!(client.purge_dead("c").await.unwrap(), 1);
        }
    });
    let result = run.await;
    backend.clear().await.unwrap();
    if let Err(e) = result {
        std::panic::resume_unwind(e.into_panic());
    }
}
