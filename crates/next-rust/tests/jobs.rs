//! `#[job]`, `#[cron]` and `AppBuilder::jobs` through the facade.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use next_rust::jobs::*;
use next_rust::{App, Environment, Routes, TestClient};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Welcome {
    email: String,
}

static SENT: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Sends the welcome mail.
#[job(queue = "mail", max_attempts = 4, priority = 5, timeout = "2s", backoff = "10ms", max_backoff = "40ms")]
pub async fn send_welcome(input: Welcome, ctx: JobContext) -> Result<(), JobError> {
    if ctx.attempt == 1 {
        // Any std error converts with `?`.
        "not a number".parse::<u32>()?;
    }
    SENT.lock().unwrap().push(format!("{} (attempt {})", input.email, ctx.attempt));
    Ok(())
}

static PINGS: AtomicU32 = AtomicU32::new(0);

#[job(name = "ops.ping")]
async fn ping() {
    PINGS.fetch_add(1, Ordering::SeqCst);
}

static TICKS: AtomicU32 = AtomicU32::new(0);

#[cron("* * * * * *", name = "every-second")]
async fn tick() {
    TICKS.fetch_add(1, Ordering::SeqCst);
}

async fn wait_until(what: &str, mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while !check() {
        assert!(start.elapsed() < Duration::from_secs(10), "timed out waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[test]
fn macros_describe_the_job() {
    assert_eq!(<send_welcome as Job>::NAME, "send_welcome");
    assert_eq!(<ping as Job>::NAME, "ops.ping");
    let o = <send_welcome as Job>::options();
    assert_eq!((o.queue.as_str(), o.max_attempts, o.priority, o.timeout), ("mail", 4, 5, Duration::from_secs(2)));
    assert_eq!((o.backoff.base_ms, o.backoff.max_ms), (10, 40));
    assert_eq!(<ping as Job>::options(), JobOptions::default());
    assert_eq!((<tick as CronTask>::NAME, <tick as CronTask>::SCHEDULE), ("every-second", "* * * * * *"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_body_stays_callable() {
    let before = PINGS.load(Ordering::SeqCst);
    ping::perform().await;
    assert!(PINGS.load(Ordering::SeqCst) > before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn app_runs_jobs_while_it_serves() {
    let started = Arc::new(AtomicU32::new(0));
    let seen = started.clone();
    let runner = JobRunner::new()
        .poll_interval(Duration::from_millis(10))
        .register(send_welcome)
        .register(ping)
        .cron(tick)
        .on_start(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
        })
        .logger(|_, _| {});
    let status = status_handler(runner.client());
    let builder =
        App::new(Routes::default()).config(next_rust::Config::default()).jobs(runner).get("/internal/jobs", status);
    let t = TestClient::with(builder, Environment::Test);

    // Built but not serving: the global client works, nothing runs yet.
    let queued = enqueue(send_welcome, Welcome { email: "ada@example.com".into() }).await.unwrap();
    assert!(!queued.duplicate);
    enqueue(ping, ()).await.unwrap();
    let report: serde_json::Value = t.get("/internal/jobs").await.json();
    assert_eq!(report["workers"], "none");
    assert_eq!(report["backend"], "memory");
    let mail = report["queues"].as_array().unwrap().iter().find(|q| q["name"] == "mail").expect("mail queue");
    assert_eq!(mail["ready"], 1);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(started.load(Ordering::SeqCst), 0);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app = t.app().clone();
    let server = tokio::spawn(async move { app.serve_listener(listener).await });
    wait_until("the retried job", || SENT.lock().unwrap().len() == 1).await;
    assert_eq!(SENT.lock().unwrap()[0], "ada@example.com (attempt 2)");
    wait_until("a cron tick", || TICKS.load(Ordering::SeqCst) >= 1).await;

    let report: serde_json::Value = t.get("/internal/jobs").await.json();
    assert_eq!(report["workers"], "running");
    assert_eq!(report["jobs"], serde_json::json!(["ops.ping", "send_welcome"]));
    assert_eq!(report["cron"][0]["name"], "every-second");
    assert!(report["process"]["succeeded"].as_u64().unwrap() >= 2, "{report}");
    assert_eq!(report["process"]["retried"], 1);
    server.abort();
}

#[test]
#[should_panic(expected = "invalid cron expression")]
fn a_bad_schedule_fails_at_startup() {
    let runner = JobRunner::new().schedule("broken", "* * * * * 9", || async {});
    let _ = App::new(Routes::default()).jobs(runner);
}
