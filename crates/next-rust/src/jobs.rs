//! Background jobs and scheduled tasks: see [`next_rust_jobs`] for the
//! engine; this module adds the `#[job]` / `#[cron]` macros and the glue to
//! the application builder.
//!
//! ```ignore
//! use next_rust::jobs::*;
//!
//! #[job(queue = "mail", max_attempts = 5, timeout = "30s")]
//! pub async fn send_welcome(input: Welcome) -> Result<(), JobError> { /* ... */ }
//!
//! #[cron("0 0 3 * * *")]
//! pub async fn nightly_cleanup() { /* ... */ }
//!
//! fn main() {
//!     next_rust::run_with_setup(routes(), |app| async move {
//!         let runner = JobRunner::new().queue("mail", 4).register(send_welcome).cron(nightly_cleanup);
//!         let status = status_handler(runner.client());
//!         app.jobs(runner).get("/internal/jobs", status)
//!     });
//! }
//!
//! // anywhere: a server action, a route, another job
//! enqueue(send_welcome, Welcome { /* ... */ }).await?;
//! ```

use std::sync::{Arc, Mutex};

pub use next_rust_jobs::*;
pub use next_rust_macros::{cron, job};
/// The original fire-and-forget helpers: an interval scheduler and an
/// in-process channel queue, without retries or persistence.
pub use next_rust_server::jobs::{Queue, Scheduler};

use next_rust_server::{AppBuilder, BoxFuture, Json, Request};

/// `AppBuilder::jobs`.
pub trait AppJobsExt: Sized {
    /// Run `runner` for as long as the server serves:
    ///
    /// * the workers start when the server starts listening — not for
    ///   `--routes`, `--export` (what `next-rust build` runs) or tests that
    ///   call `App::handle`;
    /// * on SIGINT/SIGTERM, once open connections have drained, the runner
    ///   stops claiming and finishes in-flight jobs within its
    ///   `shutdown_timeout`;
    /// * its [`JobClient`] becomes the global one (so [`enqueue`] works) and
    ///   request state (`Extension<JobClient>`);
    /// * its messages go to the framework log unless it has a logger.
    ///
    /// # Panics
    ///
    /// When the runner is misconfigured (an invalid cron expression, two
    /// jobs with one name): a programming error, caught at startup.
    fn jobs(self, runner: JobRunner) -> Self;
}

impl AppJobsExt for AppBuilder {
    fn jobs(self, runner: JobRunner) -> Self {
        if let Err(e) = runner.validate() {
            panic!("{e}");
        }
        let runner = runner.default_logger(|level, message| {
            let line = format!("jobs: {message}");
            match level {
                LogLevel::Debug => next_rust_server::log::debug(&line),
                LogLevel::Info => next_rust_server::log::info(&line),
                LogLevel::Warn => next_rust_server::log::warn(&line),
                LogLevel::Error => next_rust_server::log::error(&line),
            }
        });
        let client = runner.client();
        client.install_global();
        let pending = Arc::new(Mutex::new(Some(runner)));
        let running: Arc<Mutex<Option<JobHandle>>> = Arc::new(Mutex::new(None));
        let started = running.clone();
        self.state(client)
            .on_serve(move || {
                let runner = pending.lock().unwrap_or_else(|e| e.into_inner()).take();
                let started = started.clone();
                async move {
                    let Some(runner) = runner else { return };
                    match runner.start() {
                        Ok(handle) => *started.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle),
                        Err(e) => next_rust_server::log::error(&format!("jobs: {e}")),
                    }
                }
            })
            .on_shutdown(move || {
                let handle = running.lock().unwrap_or_else(|e| e.into_inner()).take();
                async move {
                    if let Some(handle) = handle {
                        handle.shutdown().await;
                    }
                }
            })
    }
}

/// A read-only JSON endpoint with queue sizes, counters, cron state and the
/// registered jobs ([`StatusReport`]):
///
/// ```ignore
/// app.get("/internal/jobs", status_handler(runner.client()))
/// ```
///
/// It answers anyone who can reach it, and job names and error messages can
/// be revealing: mount it on a path your middleware protects, or behind the
/// private network.
pub fn status_handler(client: JobClient) -> impl Fn(Request) -> BoxFuture<Json<StatusReport>> + Send + Sync + 'static {
    move |_req| {
        let client = client.clone();
        Box::pin(async move { Json(client.status().await) })
    }
}
