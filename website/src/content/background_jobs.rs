//! The "background jobs" documentation page.

use next_rust::prelude::*;

fn h(slug: &'static str, title: &'static str) -> Node {
    h2![id(slug), a![class("anchor"), href(format!("#{slug}")), title]].into_node()
}

pub fn content() -> Node {
    fragment![
        p![
            "Work that should not make a request wait — sending mail, resizing an upload, calling a slow API, \
             nightly clean-up — belongs in a background job. Next.js leaves this to an external service; in Next \
             Rust it is part of the framework: ",
            code!["next_rust::jobs"],
            " gives you durable queues with retries and a dead-letter list, cron-style scheduled tasks that run \
             once per deployment rather than once per instance, and a graceful shutdown that lets running jobs finish.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "A restaurant does not make you stand at the counter while your meal is cooked: the waiter writes a ticket, hands it to the kitchen and you sit down. A background job is that ticket. The visitor clicks \"Sign up\", the page answers at once, and the welcome email is sent a moment later by a worker. If sending fails, the ticket goes back on the rail and is tried again. A scheduled task is the kitchen's opening checklist: work that runs by the clock, not because someone asked.",
            ],
        ],
        p!["The words this page uses:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["term"], th!["meaning"]]],
                tbody![
                    tr![td![strong!["Job"]], td!["one piece of work to do later: a function plus its input"]],
                    tr![
                        td![strong!["Queue"]],
                        td!["a named waiting line of jobs (", code!["\"mail\""], ", ", code!["\"default\""], ")"]
                    ],
                    tr![td![strong!["Enqueue"]], td!["put a job in the line; returns immediately"]],
                    tr![
                        td![strong!["Worker"]],
                        td!["the part of the process that takes jobs from a queue and runs them"]
                    ],
                    tr![td![strong!["Retry, backoff"]], td!["a failed job is tried again, waiting longer each time"]],
                    tr![
                        td![strong!["Dead-letter list"]],
                        td!["where a job goes after its last failed attempt, kept for you to inspect or retry by hand"]
                    ],
                    tr![
                        td![strong!["Cron"]],
                        td![
                            "the classic Unix notation for schedules: ",
                            code!["0 3 * * *"],
                            " means \"at 03:00 every day\""
                        ],
                    ],
                    tr![
                        td![strong!["Backend"]],
                        td![
                            "where queued jobs are stored: the process's memory, or Redis (a fast shared data store) so they survive restarts"
                        ]
                    ],
                    tr![
                        td![strong!["Idempotent"]],
                        td![
                            "safe to run twice: sending the same invoice email twice is not, marking an invoice as sent is"
                        ]
                    ],
                    tr![
                        td![strong!["Instance"]],
                        td!["one running copy of your app; a busy site runs several behind a load balancer"]
                    ],
                ],
            ],
        ],
        p![
            "A complete example. The job and the scheduled task can live in any module (for example ",
            code!["src/jobs.rs"],
            "); the runner is set up once in ",
            code!["src/main.rs"],
            ", which replaces the one-line ",
            code!["next_rust::app!();"],
            "; the last line is what a server action or route handler calls:",
        ],
        pre![code![
            class("language-rust"),
            r#"use next_rust::jobs::*;

#[derive(Serialize, Deserialize)]
pub struct Welcome { pub user_id: u64, pub email: String }

#[job(queue = "mail", max_attempts = 5, timeout = "30s")]
pub async fn send_welcome(input: Welcome) -> Result<(), JobError> {
    mailer::send(&input.email, "Welcome!").await?;      // any std error converts with `?`
    Ok(())
}

#[cron("0 0 3 * * *")]                                   // 03:00:00 UTC, every day
pub async fn nightly_cleanup() {
    db::delete_expired_sessions().await;
}

// src/main.rs
next_rust::routes!();
fn main() {
    next_rust::run_with_setup(routes(), |app| async move {
        let runner = JobRunner::new()                    // in-memory backend
            .queue("mail", 4)                            // at most 4 mail jobs at a time
            .register(send_welcome)
            .cron(nightly_cleanup);
        app.jobs(runner)
    });
}

// anywhere — a server action, a route handler, another job:
enqueue(send_welcome, Welcome { user_id: 7, email: "ada@example.com".into() }).await?;"#,
        ],],
        h("defining-jobs", "Defining jobs"),
        p![
            code!["#[job]"],
            " goes on an ",
            code!["async fn"],
            " that takes nothing, one input, a ",
            code!["JobContext"],
            ", or both (input first). The input is stored as JSON, so it must implement ",
            code!["Serialize"],
            " and ",
            code!["Deserialize"],
            ". The function returns ",
            code!["()"],
            " or ",
            code!["Result<(), E>"],
            " for any ",
            code!["E"],
            " that converts into ",
            code!["JobError"],
            ", which every ",
            code!["std::error::Error"],
            " does.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["argument"], th!["default"], th!["meaning"]]],
                tbody![
                    tr![
                        td![code!["name = \"…\""]],
                        td!["the function name"],
                        td!["The name stored with each queued job. Keep it stable: renaming it orphans jobs that are \
                             already queued."],
                    ],
                    tr![td![code!["queue = \"…\""]], td![code!["\"default\""]], td!["Which queue it goes to."]],
                    tr![
                        td![code!["max_attempts = n"]],
                        td!["5"],
                        td!["Total attempts, the first included. After the last failure the job is dead-lettered."],
                    ],
                    tr![
                        td![code!["timeout = \"30s\""]],
                        td!["5 minutes"],
                        td!["One attempt is cancelled and counted as failed after this long."],
                    ],
                    tr![
                        td![code!["backoff = \"5s\""], ", ", code!["max_backoff = \"1h\""]],
                        td!["5 s, 1 h"],
                        td!["Retry n waits backoff × 2ⁿ⁻¹, capped, ±20 % jitter so retries do not arrive in waves."],
                    ],
                    tr![
                        td![code!["priority = n"]],
                        td!["0"],
                        td!["-100 to 100. Higher runs first; equal priorities run first-in first-out."],
                    ],
                ],
            ],
        ],
        p![
            "The macro turns the function into a unit struct of the same name that implements ",
            code!["next_rust::jobs::Job"],
            " — that is why ",
            code!["enqueue(send_welcome, …)"],
            " type-checks the input. The body stays callable as ",
            code!["send_welcome::perform(input).await"],
            ", which is handy in tests. Jobs are registered explicitly with ",
            code!["JobRunner::register"],
            ": a process only runs the jobs it registers, so a web binary and a worker binary can share one queue.",
        ],
        h3![id("failing"), a![class("anchor"), href("#failing"), "Failing on purpose"]],
        p![
            "Returning an error is how a job says \"try me again\". This job shows the three kinds of failure: an ordinary error (retried with growing waits), a fatal one (no point retrying) and a retry at a time you choose:",
        ],
        pre![code![
            class("language-rust"),
            r#"#[job(max_attempts = 8)]
pub async fn sync_invoice(id: u64, ctx: JobContext) -> Result<(), JobError> {
    let invoice = db::invoice(id).await?                                  // retried with backoff
        .ok_or_else(|| JobError::fatal(format!("invoice {id} is gone")))?; // dead-lettered at once
    match billing::push(&invoice).await {
        Err(e) if e.is_rate_limited() => Err(JobError::retry_in(e.retry_after(), e)),  // explicit delay
        Err(e) if ctx.is_last_attempt() => { alert::page(&e).await; Err(e.into()) }
        other => Ok(other?),
    }
}"#,
        ],],
        p!["A panic is caught and counts as a failed attempt, like a timeout. Input that no longer deserialises — \
             the struct changed while jobs were queued — is dead-lettered immediately, since retrying cannot help.",],
        h("enqueueing", "Enqueueing"),
        p![
            "Call these wherever the work is triggered: in a server action, an API route, a page's ",
            code!["load"],
            ", or another job. Each call returns as soon as the job is stored, not when it has run:",
        ],
        pre![code![
            class("language-rust"),
            r#"enqueue(send_welcome, input).await?;                                   // as soon as a worker is free
enqueue_in(Duration::from_secs(600), send_reminder, input).await?;       // in ten minutes
enqueue_at(deadline_as_system_time, close_auction, auction_id).await?;   // at a point in time

// Per-call options go through the client (`Extension<JobClient>` in pages and actions,
// `JobClient::global()` elsewhere):
let queued = jobs.job(charge_card, order)
    .unique(format!("charge:{order_id}"))     // unique job, see below
    .priority(50)
    .max_attempts(3)
    .queue("payments")
    .send()
    .await?;
println!("{} (duplicate: {})", queued.id, queued.duplicate);"#,
        ],],
        h3![id("unique-jobs"), a![class("anchor"), href("#unique-jobs"), "Unique jobs and idempotency keys"]],
        ul![
            li![
                code![".unique(key)"],
                " — while a job holding the key is queued, scheduled, running or waiting for a retry, enqueueing the \
                 same key again does nothing and returns the first job's id with ",
                code!["duplicate: true"],
                ". The key is released when the job completes or is dead-lettered. Use it for \"rebuild the search \
                 index\": a hundred requests, one job.",
            ],
            li![
                code![".unique_for(key, window)"],
                " — an idempotency key: the key stays taken for the whole window, even after the job has finished. \
                 Use it where the caller may repeat itself: a webhook delivered twice, a double-clicked button.",
            ],
        ],
        h("scheduled-tasks", "Scheduled tasks"),
        p![
            "Work that runs by the clock: nightly clean-up, a report every weekday morning, a heartbeat every 30 seconds. Mark a function with ",
            code!["#[cron]"],
            " and hand it to the runner in ",
            code!["src/main.rs"],
            ", or schedule a closure directly:",
        ],
        pre![code![
            class("language-rust"),
            r#"#[cron("0 */5 * * * *")]                       // six fields: seconds first
pub async fn refresh_rates() -> Result<(), JobError> { … }

JobRunner::new()
    .cron(refresh_rates)
    .schedule("digest", "0 7 * * MON-FRI", || async { digest::send().await })   // five fields: no seconds
    .schedule("heartbeat", "@every 30s", || async { metrics::beat() })"#,
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["expression"], th!["runs"]]],
                tbody![
                    tr![td![code!["0 3 * * *"]], td!["every day at 03:00 UTC"]],
                    tr![td![code!["*/15 * * * *"]], td!["every 15 minutes"]],
                    tr![td![code!["0 7 * * MON-FRI"]], td!["at 07:00 UTC, Monday to Friday"]],
                    tr![td![code!["0 0 1 * *"]], td!["at midnight UTC on the first day of each month"]],
                    tr![td![code!["30 0 3 * * *"]], td!["six fields, seconds first: every day at 03:00:30 UTC"]],
                    tr![td![code!["@every 90s"]], td!["every 90 seconds"]],
                ],
            ],
        ],
        p![
            "Expressions have five fields (",
            code!["minute hour day-of-month month day-of-week"],
            ") or six with seconds first. Each field takes ",
            code!["*"],
            ", values, ranges (",
            code!["9-17"],
            "), steps (",
            code!["*/5"],
            ", ",
            code!["10-50/10"],
            ") and lists; months and weekdays accept names (",
            code!["JAN"],
            ", ",
            code!["MON-FRI"],
            "). Also accepted: ",
            code!["@hourly"],
            ", ",
            code!["@daily"],
            ", ",
            code!["@weekly"],
            ", ",
            code!["@monthly"],
            ", ",
            code!["@yearly"],
            " and ",
            code!["@every 90s"],
            ". When both day-of-month and day-of-week are restricted a day matches if either does, as in cron. ",
            strong!["All times are UTC."],
        ],
        p![
            strong!["One instance runs each tick."],
            " Before running, a runner takes a lock named after the task and the tick's time in the backend; whoever \
             gets it runs the task, the others skip. With the Redis backend that holds across every instance of a \
             deployment; with the in-memory backend it holds within the process. The lock is per tick and expires \
             on its own (",
            code!["cron_lock_ttl"],
            ", 5 minutes), so there is no leader to fail over. A task never overlaps with itself on one instance, \
             and ticks missed while it was still running are skipped, not queued up. A failed tick is reported and \
             not retried — when the work must not be lost, have the task ",
            code!["enqueue"],
            " a job and let the queue's retries take over.",
        ],
        h("backends", "Backends"),
        p![
            "The backend is where waiting jobs are kept. The choice decides what happens when the app restarts: memory forgets, Redis remembers.",
        ],
        p![
            "The default ",
            code!["MemoryBackend"],
            " keeps everything in the process: right for development, tests and a single instance that can afford \
             to lose queued jobs on restart. For anything else enable the ",
            code!["jobs-redis"],
            " feature (Redis 5 or newer). The first line goes in ",
            code!["Cargo.toml"],
            ", the rest replaces ",
            code!["JobRunner::new()"],
            " in ",
            code!["src/main.rs"],
            "; ",
            code!["REDIS_URL"],
            " is an environment variable you set on the server:",
        ],
        pre![code![
            class("language-rust"),
            r#"// Cargo.toml:  next-rust = { version = "…", features = ["jobs-redis"] }
let backend = RedisBackend::connect(&std::env::var("REDIS_URL")?, "myapp:jobs").await?;
let runner = JobRunner::with_backend(backend).register(send_welcome);"#,
        ],],
        p![
            "Delivery is ",
            strong!["at least once"],
            ". A worker claims a job atomically for a visibility timeout (60 s), renews the claim with a heartbeat \
             while the job runs, and acknowledges it when done. If the process dies the heartbeat stops, the claim \
             expires, and the reaper of any live instance puts the job back (or dead-letters it when it was on its \
             last attempt). So a job can run twice — when a worker dies after doing the work but before \
             acknowledging it — and must be idempotent. Every transition is a single Lua script, so a job is always \
             in exactly one of ready, scheduled, active or dead; claim deadlines use the Redis server's clock, not \
             the instances'.",
        ],
        p![
            "A process that only enqueues needs no runner: ",
            code!["JobClient::new(backend).install_global()"],
            ". Your own storage implements the ",
            code!["Backend"],
            " trait; the test suite in ",
            code!["crates/next-rust-jobs/tests/common"],
            " runs unchanged against any backend.",
        ],
        h("dead-letters", "The dead-letter list"),
        p![
            "A job that failed its last attempt is not thrown away; it is parked here with its error message. Look at the list from an admin page or a maintenance script (",
            code!["jobs"],
            " is a ",
            code!["JobClient"],
            "):",
        ],
        pre![code![
            class("language-rust"),
            r#"for dead in jobs.dead_jobs("mail", 50).await? {            // newest first
    println!("{} {} after {} attempts: {}", dead.record.id, dead.record.name, dead.attempts, dead.error);
}
jobs.retry_dead("mail", &id).await?;                        // back in the queue, attempts reset
jobs.purge_dead("mail").await?;
jobs.cancel("mail", &id).await?;                            // a job that has not started yet"#,
        ],],
        p!["Each queue keeps its newest 1000 dead jobs (", code!["dead_limit"], " on the backend)."],
        h("shutdown", "Running and shutting down"),
        p![
            code!["app.jobs(runner)"],
            " ties the runner to the server: workers start when the server starts listening (not during ",
            code!["next-rust build"],
            ", ",
            code!["--routes"],
            " or tests that call ",
            code!["App::handle"],
            "), the runner's client becomes the global one and request state, and its messages go to the framework \
             log. On SIGINT/SIGTERM the server drains its connections, then the runner stops claiming jobs and \
             firing ticks and waits for running jobs up to ",
            code!["shutdown_timeout"],
            " (30 s). Whatever is still running then is cancelled and handed back to the queue.",
        ],
        p!["A dedicated worker binary does the same by hand:"],
        pre![code![
            class("language-rust"),
            r#"#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = RedisBackend::connect(&std::env::var("REDIS_URL")?, "myapp:jobs").await?;
    let handle = JobRunner::with_backend(backend)
        .queue("mail", 8)
        .register(send_welcome)
        .shutdown_timeout(Duration::from_secs(60))
        .start()?;
    tokio::signal::ctrl_c().await?;
    handle.shutdown().await;
    Ok(())
}"#,
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["runner setting"], th!["default"], th!["meaning"]]],
                tbody![
                    tr![
                        td![code!["queue(name, n)"]],
                        td!["10 per queue"],
                        td!["Concurrency limit of a queue, per process. Queues that registered jobs default to are \
                             served automatically."],
                    ],
                    tr![
                        td![code!["poll_interval"]],
                        td!["500 ms"],
                        td![
                            "How often an idle worker asks the backend. Enqueues from the same process wake it at once."
                        ],
                    ],
                    tr![
                        td![code!["visibility_timeout"]],
                        td!["60 s"],
                        td!["How long a dead worker's jobs stay stuck. Not a limit on job duration."],
                    ],
                    tr![td![code!["shutdown_timeout"]], td!["30 s"], td!["How long shutdown waits for running jobs."],],
                    tr![
                        td![code!["cron_lock_ttl"]],
                        td!["5 min"],
                        td!["How long a tick stays locked; must exceed the clock difference between instances."],
                    ],
                ],
            ],
        ],
        h("observability", "Observability"),
        p![
            "Jobs run out of sight, so make them report. Hooks are called when a job starts, succeeds or fails; attach them where the runner is built in ",
            code!["src/main.rs"],
            " and send the events to your logs or metrics:",
        ],
        pre![code![
            class("language-rust"),
            r#"let runner = JobRunner::new()
    .on_start(|job| tracing::debug!(id = %job.id, name = %job.name, attempt = job.attempt, "job started"))
    .on_success(|job, took| metrics::histogram("job_seconds", took.as_secs_f64(), &job.name))
    .on_failure(|job, failure| match failure.retry_in {
        Some(delay) => tracing::warn!(name = %job.name, error = %failure.error, ?delay, "job will retry"),
        None => tracing::error!(name = %job.name, error = %failure.error, "job dead-lettered"),
    });

// A read-only JSON endpoint: queue sizes, counters, cron state, registered jobs.
let status = status_handler(runner.client());
app.jobs(runner).get("/internal/jobs", status)"#,
        ],],
        p![
            "Hooks fire for cron runs too (",
            code!["job.kind"],
            " tells them apart). ",
            code!["jobs.metrics()"],
            " returns this process's counters and ",
            code!["jobs.stats(queue)"],
            " the queue sizes and lifetime totals shared by all instances; ",
            code!["jobs.status()"],
            " combines them into the document the endpoint serves. The endpoint answers anyone who can reach it, \
             and job names and error messages can be revealing: mount it on a path your middleware protects.",
        ],
        h("common-mistakes", "Common mistakes"),
        ul![
            li![
                strong!["Defining a job but not registering it. "],
                "A process only runs jobs passed to ",
                code!["JobRunner::register"],
                ". A job nobody registered waits in a queue no worker serves, or fails with \"no handler is registered\" and ends on the dead-letter list.",
            ],
            li![
                strong!["The in-memory backend in production. "],
                "Every deploy or crash loses what was queued. Use Redis unless losing jobs is acceptable.",
            ],
            li![
                strong!["A job that is not safe to run twice. "],
                "Delivery is at least once. Make the job check whether its work is already done, or guard it with ",
                code![".unique_for(key, window)"],
                ".",
            ],
            li![
                strong!["Renaming a job function while jobs are queued. "],
                "The stored name no longer matches. Pin it with ",
                code!["name = \"…\""],
                " before renaming.",
            ],
            li![
                strong!["Changing the input struct while jobs are queued. "],
                "Old inputs no longer deserialise and are dead-lettered. Add new fields as ",
                code!["Option"],
                " or with a serde default.",
            ],
            li![
                strong!["Expecting local time. "],
                "All schedules are UTC: ",
                code!["0 9 * * *"],
                " is 09:00 UTC, whatever the server's time zone.",
            ],
            li![
                strong!["Five fields or six. "],
                "With six, the first is seconds: ",
                code!["0 */5 * * * *"],
                " is every five minutes, ",
                code!["*/5 * * * *"],
                " too, but ",
                code!["*/5 * * * * *"],
                " is every five seconds.",
            ],
            li![
                strong!["Putting the data in the job. "],
                "Inputs are limited to 1 MiB. Queue the id of the upload, not the upload.",
            ],
            li![
                strong!["Must-not-lose work directly in a cron task. "],
                "A failed tick is not retried. Have the task enqueue a job.",
            ],
        ],
        h("check-it-worked", "How to check it worked"),
        ul![
            li![
                "In a test, call the body directly: ",
                code!["send_welcome::perform(input).await"],
                " runs the job without a queue.",
            ],
            li![
                "In development, add an ",
                code!["on_start"],
                " / ",
                code!["on_failure"],
                " hook that logs, trigger the action and watch the terminal of ",
                code!["next-rust dev"],
                ".",
            ],
            li![
                "Mount ",
                code!["status_handler"],
                " on a protected path and open it: queue sizes, counters, registered jobs and the state of each scheduled task, as JSON.",
            ],
            li!["After a failure, ", code!["jobs.dead_jobs(queue, 50)"], " shows the job with its last error.",],
        ],
        h("limits", "Limits"),
        ul![
            li!["Jobs must be idempotent: delivery is at least once."],
            li![
                "An input is at most 1 MiB of JSON (",
                code!["JobClient::max_payload_bytes"],
                "); larger ones are refused with ",
                code!["Error::Invalid"],
                ". Queue a reference to the data, not the data. Job, queue and id names are 1 to 128 bytes, unique keys up \
                 to 512, without control characters; stored error messages are cut to 4 KB. Queue depth itself is not \
                 bounded (",
                code!["MemoryBackend::max_jobs"],
                " bounds the in-memory store): rate-limit whatever lets visitors enqueue.",
            ],
            li!["Workers poll (every 500 ms by default), so a job enqueued by another instance starts up to one poll \
                 interval late."],
            li![
                "Cron is UTC-only and relies on clocks that agree to within ",
                code!["cron_lock_ttl"],
                "; ticks that fall while no instance is running are not caught up.",
            ],
            li![
                "On Redis Cluster every key must share a slot: give the prefix a hash tag, for example ",
                code!["\"{myapp}:jobs\""],
                ".",
            ],
            li![
                "The older ",
                code!["Scheduler"],
                " and ",
                code!["Queue"],
                " (fire-and-forget, in-process, no retries) are still exported from the same module.",
            ],
        ],
    ]
}
