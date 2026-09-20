//! Workers, the cron scheduler and graceful shutdown.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio::task::{JoinHandle, JoinSet};

use crate::backend::{Backend, MemoryBackend};
use crate::client::{CronState, JobClient, QueueInfo, RunnerInfo};
use crate::cron::Cron;
use crate::error::{Error, IntoJobResult, JobError};
use crate::hooks::{Hooks, JobFailure, JobInfo, LogLevel, WorkKind, bump};
use crate::job::{ClaimedJob, CronTask, Handler, Job, JobContext, MAX_ERROR_LEN, MAX_NAME_LEN, check_name, handler_of};
use crate::util::{BoxFuture, duration_ms, new_id, now_ms, truncate};

type CronFn = Arc<dyn Fn() -> BoxFuture<'static, Result<(), JobError>> + Send + Sync>;

/// Concurrency of a queue that was not configured with [`JobRunner::queue`].
pub const DEFAULT_CONCURRENCY: usize = 10;

/// Runs jobs and scheduled tasks.
///
/// ```no_run
/// # use next_rust_jobs::*;
/// # use std::time::Duration;
/// # struct SendWelcome;
/// # impl Job for SendWelcome { type Input = String; const NAME: &'static str = "w";
/// #   async fn run(_: String, _: JobContext) -> Result<(), JobError> { Ok(()) } }
/// # async fn demo() -> Result<(), Error> {
/// let runner = JobRunner::new()                       // in-memory backend
///     .queue("default", 10)
///     .queue("mail", 2)
///     .register(SendWelcome)
///     .schedule("cleanup", "0 */5 * * * *", || async { /* ... */ });
/// let jobs = runner.client();
/// let handle = runner.start()?;
/// jobs.enqueue(SendWelcome, "ada@example.com".to_owned()).await?;
/// handle.shutdown().await;                            // finish in-flight work
/// # Ok(()) }
/// ```
pub struct JobRunner {
    client: JobClient,
    queues: Vec<(String, usize)>,
    handlers: HashMap<String, (Handler, String)>,
    crons: Vec<(String, String, CronFn)>,
    hooks: Hooks,
    config: RunnerConfig,
    errors: Vec<String>,
}

#[derive(Clone)]
struct RunnerConfig {
    poll_interval: Duration,
    visibility_timeout: Duration,
    shutdown_timeout: Duration,
    cron_lock_ttl: Duration,
    reap_interval: Option<Duration>,
}

impl Default for JobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl JobRunner {
    /// A runner on the in-memory backend.
    pub fn new() -> Self {
        Self::with_backend(MemoryBackend::new())
    }

    pub fn with_backend(backend: impl Backend) -> Self {
        Self::with_client(JobClient::new(backend))
    }

    /// Share the backend (and counters) of an existing client.
    pub fn with_client(client: JobClient) -> Self {
        JobRunner {
            client,
            queues: Vec::new(),
            handlers: HashMap::new(),
            crons: Vec::new(),
            hooks: Hooks::default(),
            config: RunnerConfig {
                poll_interval: Duration::from_millis(500),
                visibility_timeout: Duration::from_secs(60),
                shutdown_timeout: Duration::from_secs(30),
                cron_lock_ttl: Duration::from_secs(300),
                reap_interval: None,
            },
            errors: Vec::new(),
        }
    }

    /// Work `name` with at most `concurrency` jobs at a time **in this
    /// process**. Queues of registered jobs that are not configured get
    /// [`DEFAULT_CONCURRENCY`].
    pub fn queue(mut self, name: &str, concurrency: usize) -> Self {
        if let Err(e) = check_name("queue name", name, MAX_NAME_LEN) {
            self.errors.push(format!("queue `{}`: {e}", truncate(&name.escape_debug().to_string(), 64)));
        }
        match self.queues.iter_mut().find(|(n, _)| n == name) {
            Some(q) => q.1 = concurrency.max(1),
            None => self.queues.push((name.to_owned(), concurrency.max(1))),
        }
        self
    }

    /// Make this runner able to run `job`.
    pub fn register<J: Job>(mut self, _job: J) -> Self {
        let checked = check_name("job name", J::NAME, MAX_NAME_LEN)
            .and_then(|()| check_name("queue name", &J::options().queue, MAX_NAME_LEN));
        if let Err(e) = checked {
            self.errors.push(format!("job `{}`: {e}", truncate(&J::NAME.escape_debug().to_string(), 64)));
        }
        if self.handlers.insert(J::NAME.to_owned(), (handler_of::<J>(), J::options().queue)).is_some() {
            self.errors.push(format!("two jobs are named `{}`", J::NAME));
        }
        self
    }

    /// Add a `#[cron]` task.
    pub fn cron<C: CronTask>(self, _task: C) -> Self {
        self.schedule(C::NAME, C::SCHEDULE, C::run)
    }

    /// Run `task` whenever `expr` fires (UTC). With a shared backend only
    /// one instance of the deployment runs each tick. The closure may return
    /// `()` or a `Result`.
    pub fn schedule<F, Fut, R>(mut self, name: &str, expr: &str, task: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoJobResult,
    {
        // The name is part of the tick lock's key.
        if let Err(e) = check_name("task name", name, MAX_NAME_LEN) {
            self.errors.push(format!("scheduled task `{}`: {e}", truncate(&name.escape_debug().to_string(), 64)));
        }
        if let Err(e) = Cron::parse(expr) {
            self.errors.push(format!("scheduled task `{name}`: {e}"));
        }
        if self.crons.iter().any(|(n, _, _)| n == name) {
            self.errors.push(format!("two scheduled tasks are named `{name}`"));
        }
        let task: CronFn = Arc::new(move || {
            let fut = task();
            Box::pin(async move { fut.await.into_job_result() })
        });
        self.crons.push((name.to_owned(), expr.to_owned(), task));
        self
    }

    /// Called when an attempt (or a cron run) starts.
    pub fn on_start(mut self, hook: impl Fn(&JobInfo) + Send + Sync + 'static) -> Self {
        self.hooks.start.push(Arc::new(hook));
        self
    }

    /// Called with the run time when an attempt succeeds.
    pub fn on_success(mut self, hook: impl Fn(&JobInfo, Duration) + Send + Sync + 'static) -> Self {
        self.hooks.success.push(Arc::new(hook));
        self
    }

    /// Called when an attempt fails, times out or panics.
    pub fn on_failure(mut self, hook: impl Fn(&JobInfo, &JobFailure) + Send + Sync + 'static) -> Self {
        self.hooks.failure.push(Arc::new(hook));
        self
    }

    /// Where the runner's own messages go (default: warnings and errors to stderr).
    pub fn logger(mut self, sink: impl Fn(LogLevel, &str) + Send + Sync + 'static) -> Self {
        self.hooks.log = Some(Arc::new(sink));
        self
    }

    /// Like [`logger`](Self::logger), unless one is already set. For integrations.
    #[doc(hidden)]
    pub fn default_logger(self, sink: impl Fn(LogLevel, &str) + Send + Sync + 'static) -> Self {
        if self.hooks.log.is_some() { self } else { self.logger(sink) }
    }

    /// How long an idle worker waits before asking the backend again
    /// (default 500 ms). Enqueues from this process wake it at once.
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.config.poll_interval = interval.max(Duration::from_millis(1));
        self
    }

    /// How long a claim lasts without a heartbeat (default 60 s). Running
    /// jobs renew theirs every third of it, so this is *not* a limit on how
    /// long a job may run; it is how long a crashed worker's jobs stay stuck.
    pub fn visibility_timeout(mut self, timeout: Duration) -> Self {
        self.config.visibility_timeout = timeout.max(Duration::from_millis(30));
        self
    }

    /// How often expired claims are recovered (default: half the visibility
    /// timeout, at most 30 s). Clamped to 1 ms ..= 1 day.
    pub fn reap_interval(mut self, interval: Duration) -> Self {
        self.config.reap_interval = Some(interval.clamp(Duration::from_millis(1), Duration::from_secs(86_400)));
        self
    }

    /// How long [`JobHandle::shutdown`] lets in-flight jobs finish (default
    /// 30 s). Jobs still running then are cancelled and retried later.
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.config.shutdown_timeout = timeout;
        self
    }

    /// How long a cron tick stays locked (default 5 min). It has to exceed
    /// the clock difference between instances.
    pub fn cron_lock_ttl(mut self, ttl: Duration) -> Self {
        self.config.cron_lock_ttl = ttl.max(Duration::from_secs(1));
        self
    }

    /// A client on the same backend. Available before and after `start`.
    pub fn client(&self) -> JobClient {
        self.client.clone()
    }

    /// Configuration mistakes collected so far: bad cron expressions,
    /// duplicate names. [`start`](Self::start) fails on the same condition.
    pub fn validate(&self) -> Result<(), Error> {
        if self.errors.is_empty() { Ok(()) } else { Err(Error::Config(self.errors.join("; "))) }
    }

    /// Spawn the workers on the current Tokio runtime and install the
    /// global client (so [`enqueue`](crate::enqueue) works).
    pub fn start(self) -> Result<JobHandle, Error> {
        self.validate()?;
        let JobRunner { client, mut queues, handlers, crons, hooks, config, .. } = self;
        for (_, default_queue) in handlers.values() {
            if !queues.iter().any(|(n, _)| n == default_queue) {
                queues.push((default_queue.clone(), DEFAULT_CONCURRENCY));
            }
        }
        queues.sort();
        let mut job_names: Vec<String> = handlers.keys().cloned().collect();
        job_names.sort();
        let cron_states: Vec<Arc<CronState>> = crons
            .iter()
            .map(|(name, expr, _)| {
                Arc::new(CronState {
                    name: name.clone(),
                    schedule: expr.clone(),
                    // Known before the task's loop first runs, so an early status report has it.
                    next_run: AtomicU64::new(
                        Cron::parse(expr).ok().and_then(|c| c.next_after(now_ms() / 1000)).unwrap_or(0),
                    ),
                    last_run: AtomicU64::new(0),
                    last_error: Mutex::new(None),
                    runs: AtomicU64::new(0),
                    failures: AtomicU64::new(0),
                    skipped: AtomicU64::new(0),
                })
            })
            .collect();
        let info = Arc::new(RunnerInfo {
            queues: queues
                .iter()
                .map(|(name, concurrency)| QueueInfo {
                    name: name.clone(),
                    concurrency: *concurrency,
                    in_flight: AtomicUsize::new(0),
                })
                .collect(),
            jobs: job_names,
            cron: cron_states.clone(),
            stopping: AtomicBool::new(false),
        });
        *client.shared.runner.write().unwrap_or_else(|e| e.into_inner()) = Some(info.clone());
        client.install_global();

        let (stop, _) = watch::channel(false);
        let inner = Arc::new(Inner {
            client,
            handlers: handlers.into_iter().map(|(name, (handler, _))| (name, handler)).collect(),
            hooks,
            config,
            info,
            stop,
            instance: new_id(),
            in_flight: Mutex::new(HashMap::new()),
        });

        let mut tasks = Vec::new();
        for (index, _) in queues.iter().enumerate() {
            tasks.push(tokio::spawn(queue_loop(inner.clone(), index)));
        }
        if !queues.is_empty() {
            tasks.push(tokio::spawn(reap_loop(inner.clone())));
        }
        for ((_, expr, task), state) in crons.into_iter().zip(cron_states) {
            // Validated above.
            if let Ok(cron) = Cron::parse(&expr) {
                tasks.push(tokio::spawn(cron_loop(inner.clone(), cron, task, state)));
            }
        }
        Ok(JobHandle { inner, tasks })
    }
}

struct Inner {
    client: JobClient,
    handlers: HashMap<String, Handler>,
    hooks: Hooks,
    config: RunnerConfig,
    info: Arc<RunnerInfo>,
    /// Lives here (not in the handle) so dropping a `JobHandle` does not stop the workers.
    stop: watch::Sender<bool>,
    instance: String,
    in_flight: Mutex<HashMap<String, ClaimedJob>>,
}

impl Inner {
    fn backend(&self) -> &Arc<dyn Backend> {
        &self.client.shared.backend
    }

    fn log(&self, level: LogLevel, message: &str) {
        self.hooks.log(level, message);
    }

    fn stopping(&self) -> bool {
        *self.stop.borrow()
    }
}

/// Resolves once shutdown has been requested.
async fn stopped(rx: &mut watch::Receiver<bool>) {
    while !*rx.borrow_and_update() {
        if rx.changed().await.is_err() {
            return;
        }
    }
}

/// A running [`JobRunner`].
pub struct JobHandle {
    inner: Arc<Inner>,
    tasks: Vec<JoinHandle<()>>,
}

impl JobHandle {
    pub fn client(&self) -> JobClient {
        self.inner.client.clone()
    }

    /// Stop claiming jobs and firing cron ticks, then wait for in-flight
    /// work up to the runner's shutdown timeout. Whatever is still running
    /// after that is cancelled and handed back to the queue for another attempt.
    pub async fn shutdown(self) {
        self.inner.info.stopping.store(true, Ordering::Relaxed);
        self.inner.stop.send_replace(true);
        for task in self.tasks {
            let _ = task.await;
        }
        *self.inner.client.shared.runner.write().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// Aborts the task when dropped, so cancelling a supervisor cancels its work.
struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn queue_loop(inner: Arc<Inner>, index: usize) {
    let queue = inner.info.queues[index].name.clone();
    let permits = Arc::new(Semaphore::new(inner.info.queues[index].concurrency));
    let mut stop = inner.stop.subscribe();
    let mut running: JoinSet<()> = JoinSet::new();
    let mut errors = 0u32;
    loop {
        while running.try_join_next().is_some() {}
        if inner.stopping() {
            break;
        }
        let permit = tokio::select! {
            p = permits.clone().acquire_owned() => match p { Ok(p) => p, Err(_) => break },
            _ = stopped(&mut stop) => break,
        };
        match inner.backend().claim(&queue, inner.config.visibility_timeout).await {
            Ok(Some(job)) => {
                errors = 0;
                // Claimed during shutdown: run it anyway, it counts as in-flight.
                running.spawn(run_job(inner.clone(), index, job, permit));
            }
            Ok(None) => {
                errors = 0;
                drop(permit);
                tokio::select! {
                    _ = tokio::time::sleep(inner.config.poll_interval) => {}
                    _ = inner.client.shared.wake.notified() => {}
                    _ = stopped(&mut stop) => break,
                }
            }
            Err(e) => {
                drop(permit);
                errors = errors.saturating_add(1);
                if errors == 1 || errors.is_multiple_of(30) {
                    inner.log(LogLevel::Error, &format!("queue `{queue}`: could not claim a job: {e}"));
                }
                let pause = Duration::from_millis(250 * u64::from(errors.min(20)));
                tokio::select! {
                    _ = tokio::time::sleep(pause) => {}
                    _ = stopped(&mut stop) => break,
                }
            }
        }
    }
    // Drain.
    let drained =
        tokio::time::timeout(inner.config.shutdown_timeout, async { while running.join_next().await.is_some() {} })
            .await;
    if drained.is_err() {
        running.abort_all();
        while running.join_next().await.is_some() {}
        let stuck: Vec<ClaimedJob> = {
            let mut map = inner.in_flight.lock().unwrap_or_else(|e| e.into_inner());
            let ids: Vec<String> =
                map.values().filter(|j| j.record.queue == queue).map(|j| j.record.id.clone()).collect();
            ids.iter().filter_map(|id| map.remove(id)).collect()
        };
        inner.log(
            LogLevel::Warn,
            &format!("queue `{queue}`: shutdown timeout, {} job(s) cancelled and returned to the queue", stuck.len()),
        );
        for job in stuck {
            inner.info.queues[index].in_flight.fetch_sub(1, Ordering::Relaxed);
            let _ = inner.backend().retry(&job, now_ms(), "interrupted by shutdown").await;
        }
    }
}

async fn run_job(inner: Arc<Inner>, queue_index: usize, job: ClaimedJob, _permit: OwnedSemaphorePermit) {
    let metrics = &inner.client.shared.metrics;
    let record = &job.record;
    let info = JobInfo {
        kind: WorkKind::Job,
        id: record.id.clone(),
        name: record.name.clone(),
        queue: record.queue.clone(),
        attempt: job.attempt,
        max_attempts: record.max_attempts,
    };
    inner.in_flight.lock().unwrap_or_else(|e| e.into_inner()).insert(record.id.clone(), job.clone());
    inner.info.queues[queue_index].in_flight.fetch_add(1, Ordering::Relaxed);
    bump(&metrics.started);
    inner.hooks.started(&info);

    let started = Instant::now();
    let lost = Arc::new(AtomicBool::new(false));
    let heartbeat = {
        let (inner, job, lost) = (inner.clone(), job.clone(), lost.clone());
        AbortOnDrop(tokio::spawn(async move {
            let visibility = inner.config.visibility_timeout;
            loop {
                tokio::time::sleep(visibility / 3).await;
                match inner.backend().heartbeat(&job, visibility).await {
                    Ok(true) => {}
                    Ok(false) => {
                        lost.store(true, Ordering::Relaxed);
                        return;
                    }
                    Err(e) => inner.log(LogLevel::Warn, &format!("job {}: heartbeat failed: {e}", job.record.id)),
                }
            }
        }))
    };

    let mut timed_out = false;
    let mut panicked = false;
    let result: Result<(), JobError> = match inner.handlers.get(&record.name) {
        None => Err(JobError::msg(format!("no handler is registered for job `{}` in this process", record.name))),
        Some(handler) => {
            let ctx = JobContext {
                id: record.id.clone(),
                name: record.name.clone(),
                queue: record.queue.clone(),
                attempt: job.attempt,
                max_attempts: record.max_attempts,
                enqueued_at_ms: record.enqueued_at_ms,
            };
            // A task of its own, so a panic is contained and reported.
            let mut task = AbortOnDrop(tokio::spawn(handler(record.payload.clone(), ctx)));
            let limit = if record.timeout_ms == 0 { Duration::MAX } else { Duration::from_millis(record.timeout_ms) };
            match tokio::time::timeout(limit, &mut task.0).await {
                Ok(Ok(result)) => result,
                Ok(Err(join)) => {
                    panicked = join.is_panic();
                    Err(JobError::msg(if panicked { panic_message(join) } else { "cancelled".to_owned() }))
                }
                Err(_) => {
                    timed_out = true;
                    Err(JobError::msg(format!("timed out after {limit:?}")))
                }
            }
        }
    };
    drop(heartbeat);
    let duration = started.elapsed();
    // The message is stored with the job: keep a hostile or runaway one small.
    let result = result.map_err(|mut e| {
        if e.message.len() > MAX_ERROR_LEN {
            e.message = truncate(&e.message, MAX_ERROR_LEN).into_owned();
        }
        e
    });

    let acknowledged = match &result {
        Ok(()) => {
            let ok = acknowledge(&inner, "complete", || inner.backend().complete(&job)).await;
            bump(&metrics.succeeded);
            inner.hooks.succeeded(&info, duration);
            ok
        }
        Err(error) => {
            bump(&metrics.failed);
            if timed_out {
                bump(&metrics.timed_out);
            }
            if panicked {
                bump(&metrics.panicked);
            }
            let retry_in = (error.retryable && job.attempt < record.max_attempts)
                .then(|| error.retry_after.unwrap_or_else(|| record.backoff.delay(job.attempt)));
            let ok = match retry_in {
                Some(delay) => {
                    bump(&metrics.retried);
                    let at = now_ms().saturating_add(duration_ms(delay));
                    acknowledge(&inner, "retry", || inner.backend().retry(&job, at, &error.message)).await
                }
                None => {
                    bump(&metrics.dead);
                    acknowledge(&inner, "dead-letter", || inner.backend().dead_letter(&job, &error.message)).await
                }
            };
            let failure = JobFailure { error: error.message.clone(), duration, retry_in, timed_out, panicked };
            inner.hooks.failed(&info, &failure);
            ok
        }
    };
    if !acknowledged || lost.load(Ordering::Relaxed) {
        bump(&metrics.lost_claims);
        inner.log(
            LogLevel::Warn,
            &format!(
                "job {} (`{}`): the claim expired before the attempt finished; another worker may run it again",
                record.id, record.name
            ),
        );
    }
    if inner.in_flight.lock().unwrap_or_else(|e| e.into_inner()).remove(&record.id).is_some() {
        inner.info.queues[queue_index].in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Acknowledge with a few retries: a transient backend error here would
/// otherwise turn into a needless second run once the claim expires.
async fn acknowledge<'a, F>(inner: &Inner, what: &str, call: F) -> bool
where
    F: Fn() -> BoxFuture<'a, Result<bool, Error>>,
{
    for attempt in 1..=3u32 {
        match call().await {
            Ok(owned) => return owned,
            Err(e) if attempt == 3 => inner.log(LogLevel::Error, &format!("could not {what} a job: {e}")),
            Err(_) => tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt))).await,
        }
    }
    false
}

fn panic_message(join: tokio::task::JoinError) -> String {
    let payload = join.into_panic();
    let text =
        payload.downcast_ref::<&str>().map(|s| (*s).to_owned()).or_else(|| payload.downcast_ref::<String>().cloned());
    format!("panicked: {}", text.unwrap_or_else(|| "(no message)".to_owned()))
}

async fn reap_loop(inner: Arc<Inner>) {
    let interval = inner
        .config
        .reap_interval
        .unwrap_or_else(|| (inner.config.visibility_timeout / 2).min(Duration::from_secs(30)));
    let mut stop = inner.stop.subscribe();
    loop {
        // Spread instances out so they do not all reap at the same moment.
        let pause = interval.mul_f64(0.75 + crate::util::random_unit() / 2.0);
        tokio::select! {
            _ = tokio::time::sleep(pause) => {}
            _ = stopped(&mut stop) => return,
        }
        for q in &inner.info.queues {
            match inner.backend().reap(&q.name).await {
                Ok(r) if r.requeued + r.dead > 0 => {
                    inner.client.shared.metrics.reaped.fetch_add((r.requeued + r.dead) as u64, Ordering::Relaxed);
                    inner.log(
                        LogLevel::Warn,
                        &format!(
                            "queue `{}`: recovered {} job(s) from lost workers ({} dead-lettered)",
                            q.name,
                            r.requeued + r.dead,
                            r.dead
                        ),
                    );
                    inner.client.shared.wake.notify_waiters();
                }
                Ok(_) => {}
                Err(e) => inner.log(LogLevel::Warn, &format!("queue `{}`: reaping failed: {e}", q.name)),
            }
        }
    }
}

async fn cron_loop(inner: Arc<Inner>, cron: Cron, task: CronFn, state: Arc<CronState>) {
    let metrics = &inner.client.shared.metrics;
    let mut stop = inner.stop.subscribe();
    let mut after = now_ms() / 1000;
    loop {
        let Some(tick) = cron.next_after(after) else {
            inner.log(LogLevel::Warn, &format!("scheduled task `{}`: `{cron}` never fires again", state.name));
            return;
        };
        state.next_run.store(tick, Ordering::Relaxed);
        // Sleep in short stretches: the wall clock may be adjusted meanwhile.
        loop {
            let now = now_ms();
            let due = tick.saturating_mul(1000);
            if now >= due {
                break;
            }
            let nap = Duration::from_millis((due - now).min(30_000));
            tokio::select! {
                _ = tokio::time::sleep(nap) => {}
                _ = stopped(&mut stop) => return,
            }
        }
        if inner.stopping() {
            return;
        }
        let key = format!("cron:{}:{tick}", state.name);
        match inner.backend().try_lock(&key, &inner.instance, inner.config.cron_lock_ttl).await {
            Ok(true) => {
                let info = JobInfo {
                    kind: WorkKind::Cron,
                    id: format!("{}@{tick}", state.name),
                    name: state.name.clone(),
                    queue: String::new(),
                    attempt: 1,
                    max_attempts: 1,
                };
                inner.hooks.started(&info);
                let started = Instant::now();
                let mut run = AbortOnDrop(tokio::spawn(task()));
                let mut panicked = false;
                let joined = tokio::select! {
                    joined = &mut run.0 => joined,
                    _ = async {
                        stopped(&mut stop).await;
                        tokio::time::sleep(inner.config.shutdown_timeout).await;
                    } => {
                        inner.log(LogLevel::Warn, &format!("scheduled task `{}`: cancelled by shutdown", state.name));
                        return;
                    }
                };
                let result = match joined {
                    Ok(r) => r,
                    Err(join) => {
                        panicked = join.is_panic();
                        Err(JobError::msg(if panicked { panic_message(join) } else { "cancelled".to_owned() }))
                    }
                };
                let duration = started.elapsed();
                state.last_run.store(tick, Ordering::Relaxed);
                state.runs.fetch_add(1, Ordering::Relaxed);
                bump(&metrics.cron_runs);
                let mut last_error = state.last_error.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => {
                        *last_error = None;
                        drop(last_error);
                        inner.hooks.succeeded(&info, duration);
                    }
                    Err(mut e) => {
                        if e.message.len() > MAX_ERROR_LEN {
                            e.message = truncate(&e.message, MAX_ERROR_LEN).into_owned();
                        }
                        *last_error = Some(e.message.clone());
                        drop(last_error);
                        state.failures.fetch_add(1, Ordering::Relaxed);
                        bump(&metrics.cron_failures);
                        inner.log(LogLevel::Error, &format!("scheduled task `{}` failed: {}", state.name, e.message));
                        let failure =
                            JobFailure { error: e.message, duration, retry_in: None, timed_out: false, panicked };
                        inner.hooks.failed(&info, &failure);
                    }
                }
            }
            Ok(false) => {
                state.skipped.fetch_add(1, Ordering::Relaxed);
                bump(&metrics.cron_skipped);
            }
            Err(e) => inner.log(
                LogLevel::Error,
                &format!("scheduled task `{}`: could not take the tick lock, skipping this tick: {e}", state.name),
            ),
        }
        // A run that overran its next ticks skips them instead of piling up.
        after = tick.max(now_ms() / 1000);
    }
}
