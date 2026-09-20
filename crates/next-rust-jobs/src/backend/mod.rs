//! Where jobs live between `enqueue` and completion.
//!
//! The contract every backend implements:
//!
//! * **At-least-once delivery.** [`claim`](Backend::claim) hands a job to one
//!   worker for a *visibility timeout*. The worker extends it with
//!   [`heartbeat`](Backend::heartbeat) while it runs and finishes with exactly
//!   one of [`complete`](Backend::complete), [`retry`](Backend::retry) or
//!   [`dead_letter`](Backend::dead_letter). A claim that expires — the worker
//!   crashed — is returned to the queue by [`reap`](Backend::reap), or
//!   dead-lettered when it has no attempts left.
//! * **Claim tokens.** The four acknowledgement calls return `Ok(false)` and
//!   change nothing when the job's token is no longer current.
//! * **Ordering.** Higher priority first, then first-in first-out. A job with
//!   a future `run_at_ms` becomes claimable once that time has passed.
//! * **Uniqueness.** An `enqueue` whose `unique_key` is held returns
//!   [`Enqueued::duplicate`] with the id of the job holding it.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::job::{ClaimedJob, DeadJob, JobRecord};
use crate::util::BoxFuture;

mod memory;
#[cfg(feature = "redis")]
mod redis;

#[cfg(feature = "redis")]
pub use self::redis::RedisBackend;
pub use memory::MemoryBackend;

/// How many entries a dead-letter list keeps per queue before the oldest are dropped.
pub const DEFAULT_DEAD_LIMIT: usize = 1000;

/// The outcome of an enqueue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enqueued {
    /// The new job's id or, for a duplicate, the id of the job that holds the unique key.
    pub id: String,
    /// Nothing was queued because the unique key is taken.
    pub duplicate: bool,
}

/// Sizes and lifetime counters of one queue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueStats {
    /// Claimable now.
    pub ready: u64,
    /// Waiting for their run time (delayed jobs and retries).
    pub scheduled: u64,
    /// Held by a worker.
    pub active: u64,
    /// In the dead-letter list.
    pub dead: u64,
    /// Lifetime counters, shared by every instance using the backend.
    pub enqueued_total: u64,
    pub completed_total: u64,
    pub retried_total: u64,
    pub dead_total: u64,
}

/// What [`Backend::reap`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reaped {
    pub requeued: usize,
    pub dead: usize,
}

pub type BackendResult<T> = Result<T, Error>;

/// Storage for queues, the dead-letter list and locks. See the [module documentation](self).
pub trait Backend: Send + Sync + 'static {
    /// `"memory"`, `"redis"`: shown in the status report.
    fn kind(&self) -> &'static str;

    fn enqueue(&self, job: JobRecord) -> BoxFuture<'_, BackendResult<Enqueued>>;

    /// Promote due scheduled jobs, then claim the next ready one.
    fn claim<'a>(&'a self, queue: &'a str, visibility: Duration) -> BoxFuture<'a, BackendResult<Option<ClaimedJob>>>;

    /// Extend the claim to `visibility` from now.
    fn heartbeat<'a>(&'a self, job: &'a ClaimedJob, visibility: Duration) -> BoxFuture<'a, BackendResult<bool>>;

    fn complete<'a>(&'a self, job: &'a ClaimedJob) -> BoxFuture<'a, BackendResult<bool>>;

    /// Schedule another attempt at `run_at_ms`, remembering `error`.
    fn retry<'a>(&'a self, job: &'a ClaimedJob, run_at_ms: u64, error: &'a str) -> BoxFuture<'a, BackendResult<bool>>;

    fn dead_letter<'a>(&'a self, job: &'a ClaimedJob, error: &'a str) -> BoxFuture<'a, BackendResult<bool>>;

    /// Recover jobs whose claim expired. Safe to call from every instance.
    fn reap<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<Reaped>>;

    /// Remove a job that is not running yet. `false` when it is unknown,
    /// running or dead.
    fn cancel<'a>(&'a self, queue: &'a str, id: &'a str) -> BoxFuture<'a, BackendResult<bool>>;

    fn stats<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<QueueStats>>;

    /// Every queue that has ever received a job.
    fn queues(&self) -> BoxFuture<'_, BackendResult<Vec<String>>>;

    /// Newest first.
    fn dead_jobs<'a>(&'a self, queue: &'a str, limit: usize) -> BoxFuture<'a, BackendResult<Vec<DeadJob>>>;

    /// Put a dead job back in the queue with a fresh attempt counter.
    fn retry_dead<'a>(&'a self, queue: &'a str, id: &'a str) -> BoxFuture<'a, BackendResult<bool>>;

    fn purge_dead<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<usize>>;

    /// Take the lock `key` for `ttl` unless someone holds it. There is no
    /// unlock: cron ticks use one key per tick and let it expire, so a tick
    /// stays taken even if its runner dies.
    fn try_lock<'a>(&'a self, key: &'a str, owner: &'a str, ttl: Duration) -> BoxFuture<'a, BackendResult<bool>>;
}

/// Ready-set ordering shared by the backends: priority first, then time.
/// `2^43` ms is the year 2248, and `201 × 2^43` still fits the 53-bit
/// mantissa of a Redis sorted-set score.
pub(crate) fn ready_score(priority: i8, now_ms: u64) -> u64 {
    const SPAN: u64 = 1 << 43;
    let rank = (i64::from(crate::job::MAX_PRIORITY) - i64::from(priority)).clamp(0, 200) as u64;
    rank * SPAN + now_ms.min(SPAN - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_orders_by_priority_then_time() {
        assert!(ready_score(10, 2_000) < ready_score(0, 1_000));
        assert!(ready_score(0, 1_000) < ready_score(0, 1_001));
        assert!(ready_score(0, u64::MAX) < ready_score(-1, 0));
        assert!(ready_score(-100, u64::MAX) < (1 << 53));
    }
}
