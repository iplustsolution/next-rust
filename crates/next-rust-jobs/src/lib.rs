//! Background jobs for Next Rust.
//!
//! * **Jobs** — an async function with one serialisable input, queued with
//!   [`enqueue`], [`enqueue_in`] or [`enqueue_at`] and run by a
//!   [`JobRunner`]: retries with exponential backoff and jitter, a maximum
//!   number of attempts, a per-attempt timeout, a dead-letter list, unique
//!   jobs and idempotency keys, priorities, and a concurrency limit per queue.
//! * **Scheduled tasks** — cron expressions ([`cron`]) with a lock per tick,
//!   so one instance of a multi-instance deployment runs each tick.
//! * **Backends** — [`MemoryBackend`] (the default) and, with the `redis`
//!   feature, `RedisBackend`: atomic claims with a visibility timeout,
//!   heartbeats and recovery of jobs whose worker died.
//! * **Shutdown** — [`JobHandle::shutdown`] stops claiming and lets
//!   in-flight jobs finish within a deadline.
//! * **Observability** — `on_start` / `on_success` / `on_failure` hooks,
//!   counters, and a serialisable [`StatusReport`].
//!
//! Delivery is **at least once**: a job can run twice when a worker dies
//! after doing the work but before acknowledging it, so make jobs idempotent.
//!
//! With the `next-rust` facade, jobs are declared with `#[job]` and
//! `#[cron]`; this crate is usable on its own by implementing [`Job`].

#![forbid(unsafe_code)]

pub mod backend;
mod backoff;
mod client;
pub mod cron;
mod error;
mod hooks;
mod job;
mod runner;
mod util;

#[cfg(feature = "redis")]
pub use backend::RedisBackend;
pub use backend::{Backend, Enqueued, MemoryBackend, QueueStats, Reaped};
pub use backoff::Backoff;
pub use client::{CronStatus, Enqueue, JobClient, QueueStatus, StatusReport, enqueue, enqueue_at, enqueue_in};
pub use cron::{Cron, CronError};
pub use error::{Error, IntoJobResult, JobError};
pub use hooks::{JobFailure, JobInfo, LogLevel, Metrics, MetricsSnapshot, WorkKind};
pub use job::{
    ClaimedJob, CronTask, DEFAULT_MAX_PAYLOAD_BYTES, DEFAULT_QUEUE, DeadJob, Job, JobContext, JobOptions, JobRecord,
    MAX_ERROR_LEN, MAX_NAME_LEN, MAX_PRIORITY, MAX_UNIQUE_KEY_LEN, MIN_PRIORITY,
};
pub use runner::{DEFAULT_CONCURRENCY, JobHandle, JobRunner};
pub use util::now_ms;
