//! The in-process backend: the default, for development, tests and
//! single-instance deployments that can afford to lose queued jobs on restart.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use super::{Backend, BackendResult, DEFAULT_DEAD_LIMIT, Enqueued, QueueStats, Reaped, ready_score};
use crate::error::Error;
use crate::job::{ClaimedJob, DeadJob, JobRecord};
use crate::util::{BoxFuture, duration_ms, new_id, now_ms};

/// Jobs kept in memory. Clones of the surrounding `Arc` share one store;
/// nothing survives the process.
///
/// Memory is the limit: queued jobs are held until a worker takes them, so a
/// producer that outruns its workers grows the process. Set
/// [`max_jobs`](Self::max_jobs) to refuse enqueues beyond a bound instead.
pub struct MemoryBackend {
    state: Mutex<State>,
    dead_limit: usize,
    max_jobs: usize,
}

/// The unique-key map is swept for expired idempotency windows when it
/// reaches this size, and then each time it doubles.
const UNIQUE_SWEEP_MIN: usize = 1024;

#[derive(Default)]
struct State {
    seq: u64,
    jobs: HashMap<String, Stored>,
    queues: HashMap<String, QueueState>,
    /// unique key → (job id, expiry of an idempotency window).
    unique: HashMap<String, (String, Option<u64>)>,
    /// Size of `unique` at which expired entries are next swept.
    unique_sweep_at: usize,
    locks: HashMap<String, u64>,
}

struct Stored {
    record: JobRecord,
    attempt: u32,
    token: String,
    error: String,
    failed_at_ms: u64,
}

#[derive(Default)]
struct QueueState {
    /// (score, sequence, id)
    ready: BTreeSet<(u64, u64, String)>,
    /// (run at, sequence, id)
    scheduled: BTreeSet<(u64, u64, String)>,
    /// id → claim deadline
    active: HashMap<String, u64>,
    /// Oldest first.
    dead: VecDeque<String>,
    counters: [u64; 4],
}

const ENQUEUED: usize = 0;
const COMPLETED: usize = 1;
const RETRIED: usize = 2;
const DEAD: usize = 3;

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBackend {
    pub fn new() -> Self {
        MemoryBackend { state: Mutex::new(State::default()), dead_limit: DEFAULT_DEAD_LIMIT, max_jobs: usize::MAX }
    }

    /// Refuse an enqueue with [`Error::Backend`] while `limit` jobs are held
    /// (queued, scheduled, running and dead together). Unlimited by default.
    pub fn max_jobs(mut self, limit: usize) -> Self {
        self.max_jobs = limit.max(1);
        self
    }

    /// Keep at most `limit` dead jobs per queue (default 1000).
    pub fn dead_limit(mut self, limit: usize) -> Self {
        self.dead_limit = limit.max(1);
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl State {
    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    fn push_ready(&mut self, queue: &str, id: &str, priority: i8, now: u64) {
        let seq = self.next_seq();
        self.queues.entry(queue.to_owned()).or_default().ready.insert((ready_score(priority, now), seq, id.to_owned()));
    }

    /// Whether `job` currently holds its claim.
    fn owns(&self, job: &ClaimedJob) -> bool {
        self.jobs.get(&job.record.id).is_some_and(|s| s.token == job.token)
            && self.queues.get(&job.record.queue).is_some_and(|q| q.active.contains_key(&job.record.id))
    }

    fn release_unique(&mut self, record: &JobRecord) {
        if let (Some(key), None) = (&record.unique_key, record.unique_ttl_ms)
            && self.unique.get(key).is_some_and(|(id, _)| *id == record.id)
        {
            self.unique.remove(key);
        }
    }

    fn bury(&mut self, id: &str, error: &str, now: u64, limit: usize) {
        let Some(stored) = self.jobs.get_mut(id) else { return };
        stored.error = error.to_owned();
        stored.failed_at_ms = now;
        stored.token.clear();
        let record = stored.record.clone();
        self.release_unique(&record);
        let q = self.queues.entry(record.queue).or_default();
        q.dead.push_back(id.to_owned());
        q.counters[DEAD] += 1;
        while q.dead.len() > limit {
            if let Some(old) = q.dead.pop_front() {
                self.jobs.remove(&old);
            }
        }
    }
}

fn ready<T: Send + 'static>(value: T) -> BoxFuture<'static, BackendResult<T>> {
    Box::pin(std::future::ready(Ok(value)))
}

impl Backend for MemoryBackend {
    fn kind(&self) -> &'static str {
        "memory"
    }

    fn enqueue(&self, job: JobRecord) -> BoxFuture<'_, BackendResult<Enqueued>> {
        if let Err(e) = job.validate() {
            return Box::pin(std::future::ready(Err(e)));
        }
        let now = now_ms();
        let mut s = self.lock();
        if let Some(key) = &job.unique_key
            && let Some((holder, expiry)) = s.unique.get(key)
            && expiry.is_none_or(|e| e > now)
        {
            return ready(Enqueued { id: holder.clone(), duplicate: true });
        }
        if s.jobs.len() >= self.max_jobs {
            let full = Error::Backend(format!("the in-memory backend is full ({} jobs)", self.max_jobs));
            return Box::pin(std::future::ready(Err(full)));
        }
        if let Some(key) = &job.unique_key {
            // Sweeping on every enqueue would make each one linear in the
            // number of open idempotency windows; sweep when the map doubles.
            if s.unique.len() >= s.unique_sweep_at.max(UNIQUE_SWEEP_MIN) {
                s.unique.retain(|_, (_, expiry)| expiry.is_none_or(|e| e > now));
                s.unique_sweep_at = s.unique.len().saturating_mul(2);
            }
            s.unique.insert(key.clone(), (job.id.clone(), job.unique_ttl_ms.map(|ttl| now.saturating_add(ttl))));
        }
        let id = job.id.clone();
        if job.run_at_ms > now {
            let seq = s.next_seq();
            s.queues.entry(job.queue.clone()).or_default().scheduled.insert((job.run_at_ms, seq, id.clone()));
        } else {
            s.push_ready(&job.queue, &id, job.priority, now);
        }
        s.queues.entry(job.queue.clone()).or_default().counters[ENQUEUED] += 1;
        s.jobs.insert(
            id.clone(),
            Stored { record: job, attempt: 0, token: String::new(), error: String::new(), failed_at_ms: 0 },
        );
        ready(Enqueued { id, duplicate: false })
    }

    fn claim<'a>(&'a self, queue: &'a str, visibility: Duration) -> BoxFuture<'a, BackendResult<Option<ClaimedJob>>> {
        let now = now_ms();
        let mut s = self.lock();
        // Promote due scheduled jobs.
        let due: Vec<(u64, u64, String)> = match s.queues.get(queue) {
            Some(q) => q.scheduled.iter().take_while(|(at, _, _)| *at <= now).cloned().collect(),
            None => return ready(None),
        };
        for entry in due {
            if let Some(q) = s.queues.get_mut(queue) {
                q.scheduled.remove(&entry);
            }
            if let Some(priority) = s.jobs.get(&entry.2).map(|j| j.record.priority) {
                s.push_ready(queue, &entry.2, priority, now);
            }
        }
        loop {
            let Some(entry) = s.queues.get_mut(queue).and_then(|q| q.ready.pop_first()) else { return ready(None) };
            let id = entry.2;
            let Some(stored) = s.jobs.get_mut(&id) else { continue };
            stored.attempt += 1;
            stored.token = new_id();
            let claimed =
                ClaimedJob { record: stored.record.clone(), attempt: stored.attempt, token: stored.token.clone() };
            if let Some(q) = s.queues.get_mut(queue) {
                q.active.insert(id, now.saturating_add(duration_ms(visibility)));
            }
            return ready(Some(claimed));
        }
    }

    fn heartbeat<'a>(&'a self, job: &'a ClaimedJob, visibility: Duration) -> BoxFuture<'a, BackendResult<bool>> {
        let mut s = self.lock();
        if !s.owns(job) {
            return ready(false);
        }
        let deadline = now_ms().saturating_add(duration_ms(visibility));
        if let Some(q) = s.queues.get_mut(&job.record.queue) {
            q.active.insert(job.record.id.clone(), deadline);
        }
        ready(true)
    }

    fn complete<'a>(&'a self, job: &'a ClaimedJob) -> BoxFuture<'a, BackendResult<bool>> {
        let mut s = self.lock();
        if !s.owns(job) {
            return ready(false);
        }
        if let Some(q) = s.queues.get_mut(&job.record.queue) {
            q.active.remove(&job.record.id);
            q.counters[COMPLETED] += 1;
        }
        s.jobs.remove(&job.record.id);
        s.release_unique(&job.record);
        ready(true)
    }

    fn retry<'a>(&'a self, job: &'a ClaimedJob, run_at_ms: u64, error: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        let mut s = self.lock();
        if !s.owns(job) {
            return ready(false);
        }
        let seq = s.next_seq();
        if let Some(stored) = s.jobs.get_mut(&job.record.id) {
            stored.error = error.to_owned();
            stored.token.clear();
        }
        if let Some(q) = s.queues.get_mut(&job.record.queue) {
            q.active.remove(&job.record.id);
            q.scheduled.insert((run_at_ms, seq, job.record.id.clone()));
            q.counters[RETRIED] += 1;
        }
        ready(true)
    }

    fn dead_letter<'a>(&'a self, job: &'a ClaimedJob, error: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        let mut s = self.lock();
        if !s.owns(job) {
            return ready(false);
        }
        if let Some(q) = s.queues.get_mut(&job.record.queue) {
            q.active.remove(&job.record.id);
        }
        s.bury(&job.record.id, error, now_ms(), self.dead_limit);
        ready(true)
    }

    fn reap<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<Reaped>> {
        let now = now_ms();
        let mut s = self.lock();
        let expired: Vec<String> = match s.queues.get(queue) {
            Some(q) => q.active.iter().filter(|(_, deadline)| **deadline <= now).map(|(id, _)| id.clone()).collect(),
            None => return ready(Reaped::default()),
        };
        let mut out = Reaped::default();
        for id in expired {
            if let Some(q) = s.queues.get_mut(queue) {
                q.active.remove(&id);
            }
            let Some((attempt, max, priority)) =
                s.jobs.get(&id).map(|j| (j.attempt, j.record.max_attempts, j.record.priority))
            else {
                continue;
            };
            if attempt >= max {
                s.bury(&id, "worker lost: the claim expired on the last attempt", now, self.dead_limit);
                out.dead += 1;
            } else {
                if let Some(j) = s.jobs.get_mut(&id) {
                    j.token.clear();
                    j.error = "worker lost: the claim expired".to_owned();
                }
                s.push_ready(queue, &id, priority, now);
                if let Some(q) = s.queues.get_mut(queue) {
                    q.counters[RETRIED] += 1;
                }
                out.requeued += 1;
            }
        }
        ready(out)
    }

    fn cancel<'a>(&'a self, queue: &'a str, id: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        let mut s = self.lock();
        let Some(q) = s.queues.get_mut(queue) else { return ready(false) };
        let before = q.ready.len() + q.scheduled.len();
        q.ready.retain(|(_, _, i)| i != id);
        q.scheduled.retain(|(_, _, i)| i != id);
        if q.ready.len() + q.scheduled.len() == before {
            return ready(false);
        }
        if let Some(stored) = s.jobs.remove(id) {
            s.release_unique(&stored.record);
        }
        ready(true)
    }

    fn stats<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<QueueStats>> {
        let now = now_ms();
        let s = self.lock();
        let Some(q) = s.queues.get(queue) else { return ready(QueueStats::default()) };
        let due = q.scheduled.iter().take_while(|(at, _, _)| *at <= now).count() as u64;
        ready(QueueStats {
            ready: q.ready.len() as u64 + due,
            scheduled: q.scheduled.len() as u64 - due,
            active: q.active.len() as u64,
            dead: q.dead.len() as u64,
            enqueued_total: q.counters[ENQUEUED],
            completed_total: q.counters[COMPLETED],
            retried_total: q.counters[RETRIED],
            dead_total: q.counters[DEAD],
        })
    }

    fn queues(&self) -> BoxFuture<'_, BackendResult<Vec<String>>> {
        let mut names: Vec<String> = self.lock().queues.keys().cloned().collect();
        names.sort();
        ready(names)
    }

    fn dead_jobs<'a>(&'a self, queue: &'a str, limit: usize) -> BoxFuture<'a, BackendResult<Vec<DeadJob>>> {
        let s = self.lock();
        let Some(q) = s.queues.get(queue) else { return ready(Vec::new()) };
        let jobs = q
            .dead
            .iter()
            .rev()
            .filter_map(|id| s.jobs.get(id))
            .take(limit)
            .map(|j| DeadJob {
                record: j.record.clone(),
                attempts: j.attempt,
                error: j.error.clone(),
                failed_at_ms: j.failed_at_ms,
            })
            .collect();
        ready(jobs)
    }

    fn retry_dead<'a>(&'a self, queue: &'a str, id: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        let now = now_ms();
        let mut s = self.lock();
        let Some(q) = s.queues.get_mut(queue) else { return ready(false) };
        let Some(pos) = q.dead.iter().position(|i| i == id) else { return ready(false) };
        q.dead.remove(pos);
        let Some(stored) = s.jobs.get_mut(id) else { return ready(false) };
        stored.attempt = 0;
        let priority = stored.record.priority;
        s.push_ready(queue, id, priority, now);
        ready(true)
    }

    fn purge_dead<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<usize>> {
        let mut s = self.lock();
        let ids: Vec<String> = s.queues.get_mut(queue).map(|q| q.dead.drain(..).collect()).unwrap_or_default();
        for id in &ids {
            s.jobs.remove(id);
        }
        ready(ids.len())
    }

    fn try_lock<'a>(&'a self, key: &'a str, _owner: &'a str, ttl: Duration) -> BoxFuture<'a, BackendResult<bool>> {
        let now = now_ms();
        let mut s = self.lock();
        s.locks.retain(|_, expiry| *expiry > now);
        if s.locks.contains_key(key) {
            return ready(false);
        }
        s.locks.insert(key.to_owned(), now.saturating_add(duration_ms(ttl)));
        ready(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(key: &str, ttl_ms: u64) -> JobRecord {
        let mut r = JobRecord::from_options("n", serde_json::Value::Null, crate::JobOptions::new(), now_ms());
        r.unique_key = Some(key.to_owned());
        r.unique_ttl_ms = Some(ttl_ms);
        r
    }

    fn block_on<T>(fut: BoxFuture<'_, BackendResult<T>>) -> T {
        let mut fut = fut;
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(out) => out.unwrap(),
            std::task::Poll::Pending => unreachable!("the memory backend never waits"),
        }
    }

    /// Idempotency windows are swept when the map doubles: not on every
    /// enqueue (each one was linear in the number of open windows) and not
    /// never (one leaked entry per key).
    #[test]
    fn expired_idempotency_keys_are_swept_in_bulk() {
        let backend = MemoryBackend::new();
        for i in 0..UNIQUE_SWEEP_MIN * 5 {
            assert!(!block_on(backend.enqueue(record(&format!("k{i}"), 0))).duplicate);
            if let Some(job) = block_on(backend.claim(crate::DEFAULT_QUEUE, Duration::from_secs(5))) {
                assert!(block_on(backend.complete(&job)));
            }
        }
        let s = backend.lock();
        assert!(s.unique.len() <= UNIQUE_SWEEP_MIN, "{} expired keys were kept", s.unique.len());
        assert!(s.jobs.is_empty());
        drop(s);
        // An expired key is free at once, swept or not; a live one is not.
        assert!(!block_on(backend.enqueue(record("k0", 60_000))).duplicate);
        assert!(block_on(backend.enqueue(record("k0", 60_000))).duplicate);
    }

    #[test]
    fn a_poisoned_lock_does_not_wedge_the_backend() {
        let backend = std::sync::Arc::new(MemoryBackend::new());
        let held = backend.clone();
        let _ = std::thread::spawn(move || {
            let _guard = held.lock();
            panic!("poison the state lock");
        })
        .join();
        assert!(backend.state.is_poisoned());
        assert!(!block_on(backend.enqueue(record("after", 1000))).duplicate);
        assert_eq!(block_on(backend.stats(crate::DEFAULT_QUEUE)).ready, 1);
    }

    #[test]
    fn records_are_validated_by_the_backend_too() {
        let backend = MemoryBackend::new();
        let mut bad = record("k", 1000);
        bad.queue = "a\nb".to_owned();
        let mut fut = backend.enqueue(bad);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(out) = fut.as_mut().poll(&mut cx) else { unreachable!() };
        assert!(matches!(out, Err(Error::Invalid(_))));
        assert!(backend.lock().unique.is_empty(), "a refused job must not hold its key");
    }
}
