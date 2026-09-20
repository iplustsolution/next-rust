//! Enqueueing and inspection.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tokio::sync::Notify;

use crate::backend::{Backend, Enqueued, MemoryBackend, QueueStats};
use crate::backoff::Backoff;
use crate::cron::format_utc;
use crate::error::Error;
use crate::hooks::{Metrics, MetricsSnapshot, bump};
use crate::job::{DEFAULT_MAX_PAYLOAD_BYTES, DeadJob, Job, JobOptions, JobRecord, MAX_ERROR_LEN};
use crate::util::{duration_ms, now_ms, system_time_ms, truncate};

pub(crate) struct Shared {
    pub backend: Arc<dyn Backend>,
    /// Wakes idle workers of this process when it enqueues something.
    pub wake: Notify,
    pub metrics: Metrics,
    pub runner: RwLock<Option<Arc<RunnerInfo>>>,
    /// Size limit of one serialised input.
    pub max_payload_bytes: AtomicUsize,
}

/// What a started runner publishes for the status report.
pub(crate) struct RunnerInfo {
    pub queues: Vec<QueueInfo>,
    pub jobs: Vec<String>,
    pub cron: Vec<Arc<CronState>>,
    pub stopping: std::sync::atomic::AtomicBool,
}

pub(crate) struct QueueInfo {
    pub name: String,
    pub concurrency: usize,
    pub in_flight: AtomicUsize,
}

pub(crate) struct CronState {
    pub name: String,
    pub schedule: String,
    pub next_run: AtomicU64,
    pub last_run: AtomicU64,
    pub last_error: std::sync::Mutex<Option<String>>,
    pub runs: AtomicU64,
    pub failures: AtomicU64,
    pub skipped: AtomicU64,
}

/// Enqueues jobs and inspects queues. Cheap to clone.
///
/// A [`JobRunner`](crate::JobRunner) hands one out with
/// [`client()`](crate::JobRunner::client); a process that only enqueues (no
/// workers) builds one directly with [`JobClient::new`].
#[derive(Clone)]
pub struct JobClient {
    pub(crate) shared: Arc<Shared>,
}

static GLOBAL: RwLock<Option<JobClient>> = RwLock::new(None);

impl Default for JobClient {
    fn default() -> Self {
        JobClient::new(MemoryBackend::new())
    }
}

impl JobClient {
    pub fn new(backend: impl Backend) -> Self {
        JobClient::from_arc(Arc::new(backend))
    }

    pub fn from_arc(backend: Arc<dyn Backend>) -> Self {
        JobClient {
            shared: Arc::new(Shared {
                backend,
                wake: Notify::new(),
                metrics: Metrics::default(),
                runner: RwLock::new(None),
                max_payload_bytes: AtomicUsize::new(DEFAULT_MAX_PAYLOAD_BYTES),
            }),
        }
    }

    /// The largest serialised job input [`send`](Enqueue::send) accepts
    /// (default [`DEFAULT_MAX_PAYLOAD_BYTES`], 1 MiB); a larger one is
    /// refused with [`Error::Invalid`]. Queue a reference (an id, an object
    /// key) rather than the data itself. The limit is shared by every clone
    /// of this client; `usize::MAX` turns it off.
    pub fn max_payload_bytes(self, bytes: usize) -> Self {
        self.shared.max_payload_bytes.store(bytes.max(1), Ordering::Relaxed);
        self
    }

    /// Make this the client behind the free functions [`enqueue`],
    /// [`enqueue_in`] and [`enqueue_at`]. A later call replaces it.
    pub fn install_global(&self) {
        *GLOBAL.write().unwrap_or_else(|e| e.into_inner()) = Some(self.clone());
    }

    /// The client installed with [`install_global`](Self::install_global).
    pub fn global() -> Option<JobClient> {
        GLOBAL.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn backend(&self) -> &Arc<dyn Backend> {
        &self.shared.backend
    }

    /// Queue `input` for `job` to run as soon as a worker is free.
    pub async fn enqueue<J: Job>(&self, job: J, input: J::Input) -> Result<Enqueued, Error> {
        self.job(job, input).send().await
    }

    /// Run after `delay`.
    pub async fn enqueue_in<J: Job>(&self, delay: Duration, job: J, input: J::Input) -> Result<Enqueued, Error> {
        self.job(job, input).delay(delay).send().await
    }

    /// Run at `time` (at once if it has passed).
    pub async fn enqueue_at<J: Job>(&self, time: SystemTime, job: J, input: J::Input) -> Result<Enqueued, Error> {
        self.job(job, input).at(time).send().await
    }

    /// Start an enqueue with per-call options:
    ///
    /// ```ignore
    /// jobs.job(charge_card, order).unique(format!("charge:{}", order_id)).priority(10).send().await?;
    /// ```
    pub fn job<J: Job>(&self, _job: J, input: J::Input) -> Enqueue<'_> {
        Enqueue {
            client: self,
            name: J::NAME.to_owned(),
            payload: serde_json::to_value(&input).map_err(|e| Error::Serialize(e.to_string())),
            options: J::options(),
            run_at_ms: None,
            unique: None,
        }
    }

    /// Enqueue by name with an already-serialised input: for jobs whose
    /// handler lives in another binary.
    pub fn raw(&self, name: &str, payload: serde_json::Value, options: JobOptions) -> Enqueue<'_> {
        Enqueue { client: self, name: name.to_owned(), payload: Ok(payload), options, run_at_ms: None, unique: None }
    }

    /// Remove a job that has not started. `false` if it is running, done or unknown.
    pub async fn cancel(&self, queue: &str, id: &str) -> Result<bool, Error> {
        self.shared.backend.cancel(queue, id).await
    }

    pub async fn stats(&self, queue: &str) -> Result<QueueStats, Error> {
        self.shared.backend.stats(queue).await
    }

    /// The dead-letter list of `queue`, newest first.
    pub async fn dead_jobs(&self, queue: &str, limit: usize) -> Result<Vec<DeadJob>, Error> {
        self.shared.backend.dead_jobs(queue, limit).await
    }

    /// Give a dead job a fresh set of attempts.
    pub async fn retry_dead(&self, queue: &str, id: &str) -> Result<bool, Error> {
        let moved = self.shared.backend.retry_dead(queue, id).await?;
        if moved {
            self.shared.wake.notify_waiters();
        }
        Ok(moved)
    }

    pub async fn purge_dead(&self, queue: &str) -> Result<usize, Error> {
        self.shared.backend.purge_dead(queue).await
    }

    /// This process's counters.
    pub fn metrics(&self) -> MetricsSnapshot {
        self.shared.metrics.snapshot()
    }

    /// Everything worth showing on a status page. Read-only. A backend error
    /// is reported in [`StatusReport::error`] rather than failing the call.
    pub async fn status(&self) -> StatusReport {
        let runner = self.shared.runner.read().unwrap_or_else(|e| e.into_inner()).clone();
        let mut error = None;
        let mut names = match self.shared.backend.queues().await {
            Ok(names) => names,
            Err(e) => {
                error = Some(e.to_string());
                Vec::new()
            }
        };
        if let Some(r) = &runner {
            for q in &r.queues {
                if !names.contains(&q.name) {
                    names.push(q.name.clone());
                }
            }
        }
        names.sort();
        let mut queues = Vec::with_capacity(names.len());
        for name in names {
            let stats = match self.shared.backend.stats(&name).await {
                Ok(s) => s,
                Err(e) => {
                    error.get_or_insert_with(|| e.to_string());
                    QueueStats::default()
                }
            };
            let local = runner.as_ref().and_then(|r| r.queues.iter().find(|q| q.name == name));
            queues.push(QueueStatus {
                name,
                concurrency: local.map(|q| q.concurrency),
                in_flight: local.map_or(0, |q| q.in_flight.load(Ordering::Relaxed)),
                stats,
            });
        }
        let cron = runner
            .as_ref()
            .map(|r| {
                r.cron
                    .iter()
                    .map(|c| {
                        let time = |a: &AtomicU64| match a.load(Ordering::Relaxed) {
                            0 => None,
                            t => Some(format_utc(t)),
                        };
                        CronStatus {
                            name: c.name.clone(),
                            schedule: c.schedule.clone(),
                            next_run: time(&c.next_run),
                            last_run: time(&c.last_run),
                            last_error: c.last_error.lock().unwrap_or_else(|e| e.into_inner()).clone(),
                            runs: c.runs.load(Ordering::Relaxed),
                            failures: c.failures.load(Ordering::Relaxed),
                            skipped: c.skipped.load(Ordering::Relaxed),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        StatusReport {
            backend: self.shared.backend.kind(),
            workers: match &runner {
                None => "none",
                Some(r) if r.stopping.load(Ordering::Relaxed) => "stopping",
                Some(_) => "running",
            },
            generated_at: format_utc(now_ms() / 1000),
            queues,
            jobs: runner.as_ref().map(|r| r.jobs.clone()).unwrap_or_default(),
            cron,
            process: self.metrics(),
            error: error.map(|e| truncate(&e, MAX_ERROR_LEN).into_owned()),
        }
    }
}

/// The JSON document behind a status endpoint. See [`JobClient::status`].
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub backend: &'static str,
    /// `"running"`, `"stopping"`, or `"none"` when this process has no workers.
    pub workers: &'static str,
    pub generated_at: String,
    pub queues: Vec<QueueStatus>,
    /// Names of the jobs this process can run.
    pub jobs: Vec<String>,
    pub cron: Vec<CronStatus>,
    /// Counters of this process only.
    pub process: MetricsSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueStatus {
    pub name: String,
    /// `None` when this process runs no workers for the queue.
    pub concurrency: Option<usize>,
    /// Jobs running in this process.
    pub in_flight: usize,
    #[serde(flatten)]
    pub stats: QueueStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct CronStatus {
    pub name: String,
    pub schedule: String,
    pub next_run: Option<String>,
    /// The last tick **this process** ran.
    pub last_run: Option<String>,
    pub last_error: Option<String>,
    pub runs: u64,
    pub failures: u64,
    /// Ticks another instance ran.
    pub skipped: u64,
}

/// A pending enqueue. Nothing happens until [`send`](Enqueue::send).
#[must_use = "call .send().await to enqueue the job"]
pub struct Enqueue<'a> {
    client: &'a JobClient,
    name: String,
    payload: Result<serde_json::Value, Error>,
    options: JobOptions,
    run_at_ms: Option<u64>,
    unique: Option<(String, Option<u64>)>,
}

impl Enqueue<'_> {
    pub fn delay(mut self, delay: Duration) -> Self {
        self.run_at_ms = Some(now_ms().saturating_add(duration_ms(delay)));
        self
    }

    pub fn at(mut self, time: SystemTime) -> Self {
        self.run_at_ms = Some(system_time_ms(time));
        self
    }

    pub fn queue(mut self, queue: impl Into<String>) -> Self {
        self.options = self.options.queue(queue);
        self
    }

    /// `-100..=100`; higher runs first.
    pub fn priority(mut self, priority: i8) -> Self {
        self.options = self.options.priority(priority);
        self
    }

    pub fn max_attempts(mut self, n: u32) -> Self {
        self.options = self.options.max_attempts(n);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options = self.options.timeout(timeout);
        self
    }

    pub fn backoff(mut self, backoff: Backoff) -> Self {
        self.options = self.options.backoff(backoff);
        self
    }

    /// A **unique job**: while a job holding `key` is queued, scheduled,
    /// running or waiting for a retry, enqueueing the same key again is a
    /// no-op that returns the first job's id with `duplicate: true`. The key
    /// is released when the job completes or is dead-lettered.
    pub fn unique(mut self, key: impl Into<String>) -> Self {
        self.unique = Some((key.into(), None));
        self
    }

    /// An **idempotency key**: like [`unique`](Self::unique), but the key
    /// stays taken for `window` from now, even after the job has finished.
    /// Use it where a caller may repeat a request (a webhook delivered twice,
    /// a double-clicked button).
    pub fn unique_for(mut self, key: impl Into<String>, window: Duration) -> Self {
        self.unique = Some((key.into(), Some(duration_ms(window).max(1))));
        self
    }

    pub async fn send(self) -> Result<Enqueued, Error> {
        let now = now_ms();
        let payload = self.payload?;
        let limit = self.client.shared.max_payload_bytes.load(Ordering::Relaxed);
        if serialized_len_exceeds(&payload, limit) {
            return Err(Error::Invalid(format!(
                "the input of job `{}` is larger than {limit} bytes; queue a reference to the data instead",
                truncate(&self.name, 64)
            )));
        }
        let mut record = JobRecord::from_options(&self.name, payload, self.options, now);
        if let Some(at) = self.run_at_ms {
            record.run_at_ms = at.max(now);
        }
        if let Some((key, ttl)) = self.unique {
            record.unique_key = Some(key);
            record.unique_ttl_ms = ttl;
        }
        record.validate()?;
        let shared = &self.client.shared;
        let immediate = record.run_at_ms <= now;
        let out = shared.backend.enqueue(record).await?;
        if out.duplicate {
            bump(&shared.metrics.duplicates);
        } else {
            bump(&shared.metrics.enqueued);
            if immediate {
                shared.wake.notify_waiters();
            }
        }
        Ok(out)
    }
}

/// Whether `value` serialises to more than `limit` bytes. Stops writing as
/// soon as the answer is known and allocates nothing.
fn serialized_len_exceeds(value: &serde_json::Value, limit: usize) -> bool {
    struct Budget(usize);
    impl std::io::Write for Budget {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match self.0.checked_sub(buf.len()) {
                Some(left) => {
                    self.0 = left;
                    Ok(buf.len())
                }
                None => Err(std::io::Error::other("over the limit")),
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    limit != usize::MAX && serde_json::to_writer(Budget(limit), value).is_err()
}

/// [`JobClient::enqueue`] on the global client.
pub async fn enqueue<J: Job>(job: J, input: J::Input) -> Result<Enqueued, Error> {
    JobClient::global().ok_or(Error::NoGlobalClient)?.enqueue(job, input).await
}

/// [`JobClient::enqueue_in`] on the global client.
pub async fn enqueue_in<J: Job>(delay: Duration, job: J, input: J::Input) -> Result<Enqueued, Error> {
    JobClient::global().ok_or(Error::NoGlobalClient)?.enqueue_in(delay, job, input).await
}

/// [`JobClient::enqueue_at`] on the global client.
pub async fn enqueue_at<J: Job>(time: SystemTime, job: J, input: J::Input) -> Result<Enqueued, Error> {
    JobClient::global().ok_or(Error::NoGlobalClient)?.enqueue_at(time, job, input).await
}
