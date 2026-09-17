//! Background and scheduled jobs.
//!
//! Jobs are independent from the web server: run a [`Scheduler`] inside the
//! server process, or in a separate binary for horizontally scaled
//! deployments (so jobs do not run once per replica).

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinHandle;

use crate::middleware::BoxFuture;

type JobFn = Arc<dyn Fn() -> BoxFuture<()> + Send + Sync>;

enum Schedule {
    Every(Duration),
    DailyAt { hour: u32, minute: u32 },
}

/// Periodic job scheduler.
#[derive(Default)]
pub struct Scheduler {
    jobs: Vec<(String, Schedule, JobFn)>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run `job` every `interval` (the first run happens after one interval).
    pub fn every<F, Fut>(mut self, name: &str, interval: Duration, job: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.jobs.push((name.to_owned(), Schedule::Every(interval), Arc::new(move || Box::pin(job()))));
        self
    }

    /// Run `job` once a day at `hour:minute` UTC.
    pub fn daily_at<F, Fut>(mut self, name: &str, hour: u32, minute: u32, job: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.jobs.push((
            name.to_owned(),
            Schedule::DailyAt { hour: hour % 24, minute: minute % 60 },
            Arc::new(move || Box::pin(job())),
        ));
        self
    }

    /// Spawn all jobs on the current Tokio runtime. A job never overlaps with
    /// itself; panics are contained and logged.
    pub fn start(self) -> Vec<JoinHandle<()>> {
        self.jobs
            .into_iter()
            .map(|(name, schedule, job)| {
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(next_delay(&schedule)).await;
                        let run = tokio::spawn((job)());
                        if let Err(e) = run.await {
                            crate::log::error(&format!("job `{name}` panicked: {e}"));
                        }
                    }
                })
            })
            .collect()
    }
}

fn next_delay(schedule: &Schedule) -> Duration {
    match schedule {
        Schedule::Every(d) => *d,
        Schedule::DailyAt { hour, minute } => {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let target = u64::from(*hour) * 3600 + u64::from(*minute) * 60;
            let today = now % 86_400;
            Duration::from_secs(if target > today { target - today } else { 86_400 - today + target })
        }
    }
}

/// A simple in-process work queue with bounded concurrency.
#[derive(Clone)]
pub struct Queue<T: Send + 'static> {
    tx: mpsc::UnboundedSender<T>,
}

impl<T: Send + 'static> Queue<T> {
    /// Start a queue whose items are processed by `worker`, at most
    /// `concurrency` at a time.
    pub fn start<F, Fut>(concurrency: usize, worker: F) -> Self
    where
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel::<T>();
        let worker = Arc::new(worker);
        let permits = Arc::new(Semaphore::new(concurrency.max(1)));
        tokio::spawn(async move {
            while let Some(item) = rx.recv().await {
                let Ok(permit) = permits.clone().acquire_owned().await else { break };
                let worker = worker.clone();
                tokio::spawn(async move {
                    worker(item).await;
                    drop(permit);
                });
            }
        });
        Queue { tx }
    }

    /// Enqueue an item. Returns `false` if the queue has shut down.
    pub fn push(&self, item: T) -> bool {
        self.tx.send(item).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn queue_processes_items() {
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let q = Queue::start(2, move |n: usize| {
            let c = c.clone();
            async move {
                c.fetch_add(n, Ordering::SeqCst);
            }
        });
        for i in 1..=4 {
            assert!(q.push(i));
        }
        for _ in 0..50 {
            if count.load(Ordering::SeqCst) == 10 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(count.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn scheduler_runs_jobs() {
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let handles = Scheduler::new()
            .every("tick", Duration::from_millis(5), move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            })
            .start();
        // Generous margin: Windows timers have ~15 ms resolution.
        tokio::time::sleep(Duration::from_millis(250)).await;
        for h in handles {
            h.abort();
        }
        assert!(count.load(Ordering::SeqCst) >= 2);
    }
}
