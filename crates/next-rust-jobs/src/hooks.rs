//! Observability: lifecycle hooks, counters and the log sink.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;

/// Whether an event is about a queued job or a scheduled task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkKind {
    Job,
    Cron,
}

/// What a hook is told about the work it observes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInfo {
    pub kind: WorkKind,
    /// The job id; for a cron task, `name@unix-seconds-of-the-tick`.
    pub id: String,
    pub name: String,
    /// Empty for cron tasks.
    pub queue: String,
    pub attempt: u32,
    pub max_attempts: u32,
}

/// Passed to `on_failure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobFailure {
    pub error: String,
    pub duration: Duration,
    /// `Some(delay)` when another attempt is scheduled; `None` when the job
    /// went to the dead-letter list (or, for a cron task, simply failed).
    pub retry_in: Option<Duration>,
    pub timed_out: bool,
    pub panicked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub(crate) type StartHook = Arc<dyn Fn(&JobInfo) + Send + Sync>;
pub(crate) type SuccessHook = Arc<dyn Fn(&JobInfo, Duration) + Send + Sync>;
pub(crate) type FailureHook = Arc<dyn Fn(&JobInfo, &JobFailure) + Send + Sync>;
pub(crate) type LogSink = Arc<dyn Fn(LogLevel, &str) + Send + Sync>;

#[derive(Clone, Default)]
pub(crate) struct Hooks {
    pub start: Vec<StartHook>,
    pub success: Vec<SuccessHook>,
    pub failure: Vec<FailureHook>,
    pub log: Option<LogSink>,
}

/// Hooks and the log sink are application code running inside the runner's
/// own tasks: a panic there must not take a worker or a cron loop down, nor
/// skip the acknowledgement that follows.
fn guarded(what: &str, call: impl FnOnce()) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).is_err() {
        eprintln!("[jobs] {what} panicked; the runner carries on");
    }
}

impl Hooks {
    pub(crate) fn log(&self, level: LogLevel, message: &str) {
        match &self.log {
            Some(sink) => guarded("the log sink", || sink(level, message)),
            None if level >= LogLevel::Warn => eprintln!("[jobs] {message}"),
            None => {}
        }
    }

    pub(crate) fn started(&self, info: &JobInfo) {
        for hook in &self.start {
            guarded("an on_start hook", || hook(info));
        }
    }

    pub(crate) fn succeeded(&self, info: &JobInfo, duration: Duration) {
        for hook in &self.success {
            guarded("an on_success hook", || hook(info, duration));
        }
    }

    pub(crate) fn failed(&self, info: &JobInfo, failure: &JobFailure) {
        for hook in &self.failure {
            guarded("an on_failure hook", || hook(info, failure));
        }
    }
}

impl PartialOrd for LogLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some((*self as u8).cmp(&(*other as u8)))
    }
}

/// Counters of this process since it started. Queue sizes and the totals
/// shared between instances come from the backend ([`crate::QueueStats`]).
#[derive(Default)]
pub struct Metrics {
    pub(crate) enqueued: AtomicU64,
    pub(crate) duplicates: AtomicU64,
    pub(crate) started: AtomicU64,
    pub(crate) succeeded: AtomicU64,
    pub(crate) failed: AtomicU64,
    pub(crate) retried: AtomicU64,
    pub(crate) dead: AtomicU64,
    pub(crate) timed_out: AtomicU64,
    pub(crate) panicked: AtomicU64,
    pub(crate) lost_claims: AtomicU64,
    pub(crate) reaped: AtomicU64,
    pub(crate) cron_runs: AtomicU64,
    pub(crate) cron_failures: AtomicU64,
    pub(crate) cron_skipped: AtomicU64,
}

/// A copy of [`Metrics`] at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct MetricsSnapshot {
    /// Jobs this process enqueued.
    pub enqueued: u64,
    /// Enqueues refused because the unique key was taken.
    pub duplicates: u64,
    /// Attempts started.
    pub started: u64,
    pub succeeded: u64,
    /// Attempts that failed, whatever happened next.
    pub failed: u64,
    pub retried: u64,
    /// Jobs sent to the dead-letter list.
    pub dead: u64,
    pub timed_out: u64,
    pub panicked: u64,
    /// Attempts whose claim had expired by the time they finished.
    pub lost_claims: u64,
    /// Expired claims this process recovered.
    pub reaped: u64,
    /// Cron ticks this process won and ran.
    pub cron_runs: u64,
    pub cron_failures: u64,
    /// Cron ticks another instance ran.
    pub cron_skipped: u64,
}

pub(crate) fn bump(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}

impl Metrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        let get = |c: &AtomicU64| c.load(Ordering::Relaxed);
        MetricsSnapshot {
            enqueued: get(&self.enqueued),
            duplicates: get(&self.duplicates),
            started: get(&self.started),
            succeeded: get(&self.succeeded),
            failed: get(&self.failed),
            retried: get(&self.retried),
            dead: get(&self.dead),
            timed_out: get(&self.timed_out),
            panicked: get(&self.panicked),
            lost_claims: get(&self.lost_claims),
            reaped: get(&self.reaped),
            cron_runs: get(&self.cron_runs),
            cron_failures: get(&self.cron_failures),
            cron_skipped: get(&self.cron_skipped),
        }
    }
}
