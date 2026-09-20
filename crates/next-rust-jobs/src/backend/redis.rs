//! The Redis backend: a reliable queue shared by every instance.
//!
//! Layout, for prefix `p` and queue `q`:
//!
//! | Key | Type | Holds |
//! | --- | --- | --- |
//! | `p:job:<id>` | hash | `data` (the immutable [`JobRecord`] as JSON), `attempt`, `max`, `priority`, `token`, `error`, `failed_at`, `uniq` |
//! | `p:q:<q>:ready` | sorted set | claimable ids, scored by priority then time |
//! | `p:q:<q>:scheduled` | sorted set | delayed jobs and retries, scored by run time |
//! | `p:q:<q>:active` | sorted set | claimed ids, scored by claim deadline |
//! | `p:q:<q>:dead` | sorted set | the dead-letter list, scored by failure time |
//! | `p:q:<q>:stats` | hash | lifetime counters |
//! | `p:queues` | set | queue names |
//! | `p:uniq:<key>` | string | id of the job holding a unique key |
//! | `p:lock:<key>` | string | owner of a lock (cron ticks) |
//!
//! Every state change is one Lua script, so it is atomic: a job is always in
//! exactly one of ready / scheduled / active / dead. Scripts read the time
//! from the Redis server (`TIME`), so claim deadlines do not depend on the
//! clocks of the application instances; delays are passed as durations for
//! the same reason.
//!
//! The record is never re-encoded inside Lua (cjson would corrupt large
//! integers), which is why mutable fields sit beside `data` in the hash.

use std::time::Duration;

use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::{FromRedisValue, Script};

use super::{Backend, BackendResult, DEFAULT_DEAD_LIMIT, Enqueued, QueueStats, Reaped};
use crate::error::Error;
use crate::job::{ClaimedJob, DeadJob, JobRecord, MAX_PRIORITY};
use crate::util::{BoxFuture, duration_ms, new_id, now_ms};

/// A unique key that is released when its job finishes still expires after
/// this long, so a key can never be stuck forever.
const UNIQUE_SAFETY_TTL_MS: u64 = 30 * 24 * 3600 * 1000;

/// Redis refuses a `PX` that overflows its clock; ten years is "forever".
const MAX_TTL_MS: u64 = 10 * 365 * 24 * 3600 * 1000;

/// The same for delays and claim deadlines, which become sorted-set scores:
/// beyond 2^53 a score stops being an exact integer.
const MAX_DELAY_MS: u64 = 1 << 52;

/// Shared by every script. `fmt` exists because Redis turns Lua numbers into
/// strings with `%.14g`, which would round our 16-digit scores.
const PRELUDE: &str = r"
local t = redis.call('TIME')
local now = t[1] * 1000 + math.floor(t[2] / 1000)
local SPAN = 8796093022208
local function fmt(n) return string.format('%.0f', n) end
local function ready_score(priority) return fmt((100 - tonumber(priority)) * SPAN + now) end
local function owns(jobkey, activekey, id, token)
  if token == '' or redis.call('HGET', jobkey, 'token') ~= token then return false end
  return redis.call('ZSCORE', activekey, id) ~= false
end
local function release_unique(jobkey, id)
  local uniq = redis.call('HGET', jobkey, 'uniq')
  if uniq and uniq ~= '' and redis.call('GET', uniq) == id then redis.call('DEL', uniq) end
end
local function bury(jobkey, id, err, deadkey, statskey, limit, prefix)
  release_unique(jobkey, id)
  redis.call('HSET', jobkey, 'error', err, 'failed_at', fmt(now), 'token', '')
  redis.call('ZADD', deadkey, fmt(now), id)
  redis.call('HINCRBY', statskey, 'dead', 1)
  local extra = redis.call('ZCARD', deadkey) - limit
  if extra > 0 then
    for _, old in ipairs(redis.call('ZRANGE', deadkey, 0, extra - 1)) do redis.call('DEL', prefix .. old) end
    redis.call('ZREMRANGEBYRANK', deadkey, 0, extra - 1)
  end
end
";

// KEYS: job, ready, scheduled, stats, queues, uniq (or the job key again)
// ARGV: id, data, max, priority, delay_ms, queue, unique ('1'/'0'), unique_ttl_ms, release_on_finish ('1'/'0')
const ENQUEUE: &str = r"
if ARGV[7] == '1' then
  if not redis.call('SET', KEYS[6], ARGV[1], 'NX', 'PX', ARGV[8]) then
    return {0, redis.call('GET', KEYS[6]) or ''}
  end
end
redis.call('HSET', KEYS[1], 'data', ARGV[2], 'attempt', 0, 'max', ARGV[3], 'priority', ARGV[4], 'token', '')
if ARGV[9] == '1' then redis.call('HSET', KEYS[1], 'uniq', KEYS[6]) end
local delay = tonumber(ARGV[5])
if delay > 0 then
  redis.call('ZADD', KEYS[3], fmt(now + delay), ARGV[1])
else
  redis.call('ZADD', KEYS[2], ready_score(ARGV[4]), ARGV[1])
end
redis.call('HINCRBY', KEYS[4], 'enqueued', 1)
redis.call('SADD', KEYS[5], ARGV[6])
return {1, ARGV[1]}
";

// KEYS: scheduled, ready, active.  ARGV: job key prefix, visibility_ms, token
const CLAIM: &str = r"
for _, id in ipairs(redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', fmt(now), 'LIMIT', 0, 200)) do
  redis.call('ZREM', KEYS[1], id)
  local priority = redis.call('HGET', ARGV[1] .. id, 'priority')
  if priority then redis.call('ZADD', KEYS[2], ready_score(priority), id) end
end
while true do
  local popped = redis.call('ZPOPMIN', KEYS[2])
  if #popped == 0 then return false end
  local id = popped[1]
  local key = ARGV[1] .. id
  local data = redis.call('HGET', key, 'data')
  if data then
    local attempt = redis.call('HINCRBY', key, 'attempt', 1)
    redis.call('HSET', key, 'token', ARGV[3])
    redis.call('ZADD', KEYS[3], fmt(now + tonumber(ARGV[2])), id)
    return {data, attempt, id}
  end
end
";

// KEYS: job, active.  ARGV: id, token, visibility_ms
const HEARTBEAT: &str = r"
if not owns(KEYS[1], KEYS[2], ARGV[1], ARGV[2]) then return 0 end
redis.call('ZADD', KEYS[2], fmt(now + tonumber(ARGV[3])), ARGV[1])
return 1
";

// KEYS: job, active, stats.  ARGV: id, token
const COMPLETE: &str = r"
if not owns(KEYS[1], KEYS[2], ARGV[1], ARGV[2]) then return 0 end
redis.call('ZREM', KEYS[2], ARGV[1])
release_unique(KEYS[1], ARGV[1])
redis.call('DEL', KEYS[1])
redis.call('HINCRBY', KEYS[3], 'completed', 1)
return 1
";

// KEYS: job, active, scheduled, stats.  ARGV: id, token, delay_ms, error
const RETRY: &str = r"
if not owns(KEYS[1], KEYS[2], ARGV[1], ARGV[2]) then return 0 end
redis.call('ZREM', KEYS[2], ARGV[1])
redis.call('HSET', KEYS[1], 'error', ARGV[4], 'token', '')
redis.call('ZADD', KEYS[3], fmt(now + math.max(0, tonumber(ARGV[3]))), ARGV[1])
redis.call('HINCRBY', KEYS[4], 'retried', 1)
return 1
";

// KEYS: job, active, dead, stats.  ARGV: id, token, error, dead_limit, job key prefix
const DEAD_LETTER: &str = r"
if not owns(KEYS[1], KEYS[2], ARGV[1], ARGV[2]) then return 0 end
redis.call('ZREM', KEYS[2], ARGV[1])
bury(KEYS[1], ARGV[1], ARGV[3], KEYS[3], KEYS[4], tonumber(ARGV[4]), ARGV[5])
return 1
";

// KEYS: active, ready, dead, stats.  ARGV: job key prefix, dead_limit
const REAP: &str = r"
local requeued, dead = 0, 0
for _, id in ipairs(redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', fmt(now), 'LIMIT', 0, 200)) do
  redis.call('ZREM', KEYS[1], id)
  local key = ARGV[1] .. id
  local f = redis.call('HMGET', key, 'attempt', 'max', 'priority')
  if f[1] and f[2] and f[3] then
    if tonumber(f[1]) >= tonumber(f[2]) then
      bury(key, id, 'worker lost: the claim expired on the last attempt', KEYS[3], KEYS[4], tonumber(ARGV[2]), ARGV[1])
      dead = dead + 1
    else
      redis.call('HSET', key, 'token', '', 'error', 'worker lost: the claim expired')
      redis.call('ZADD', KEYS[2], ready_score(f[3]), id)
      redis.call('HINCRBY', KEYS[4], 'retried', 1)
      requeued = requeued + 1
    end
  end
end
return {requeued, dead}
";

// KEYS: job, ready, scheduled.  ARGV: id
const CANCEL: &str = r"
if redis.call('ZREM', KEYS[2], ARGV[1]) + redis.call('ZREM', KEYS[3], ARGV[1]) == 0 then return 0 end
release_unique(KEYS[1], ARGV[1])
redis.call('DEL', KEYS[1])
return 1
";

// KEYS: job, dead, ready.  ARGV: id
const RETRY_DEAD: &str = r"
if redis.call('ZREM', KEYS[2], ARGV[1]) == 0 then return 0 end
local priority = redis.call('HGET', KEYS[1], 'priority')
if not priority then return 0 end
redis.call('HSET', KEYS[1], 'attempt', 0, 'token', '')
redis.call('ZADD', KEYS[3], ready_score(priority), ARGV[1])
return 1
";

// KEYS: dead.  ARGV: job key prefix
const PURGE_DEAD: &str = r"
local ids = redis.call('ZRANGE', KEYS[1], 0, -1)
for _, id in ipairs(ids) do redis.call('DEL', ARGV[1] .. id) end
redis.call('DEL', KEYS[1])
return #ids
";

struct Scripts {
    enqueue: Script,
    claim: Script,
    heartbeat: Script,
    complete: Script,
    retry: Script,
    dead_letter: Script,
    reap: Script,
    cancel: Script,
    retry_dead: Script,
    purge_dead: Script,
}

/// Jobs in Redis (5.0 or newer). Needs the `redis` feature.
///
/// ```ignore
/// let backend = RedisBackend::connect("redis://localhost:6379", "myapp:jobs").await?;
/// let runner = JobRunner::with_backend(backend);
/// ```
///
/// All keys start with the prefix. On Redis Cluster the prefix must contain
/// a hash tag (`"{myapp}:jobs"`) so that every key lands in one slot.
pub struct RedisBackend {
    conn: ConnectionManager,
    prefix: String,
    dead_limit: usize,
    scripts: Scripts,
}

fn backend_err(e: redis::RedisError) -> Error {
    Error::Backend(e.to_string())
}

impl RedisBackend {
    /// Connect with a 5 s connection and 10 s response timeout; the
    /// connection re-establishes itself after a failure.
    pub async fn connect(url: &str, prefix: &str) -> Result<Self, Error> {
        let client = redis::Client::open(url).map_err(backend_err)?;
        let config = ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_secs(5))
            .set_response_timeout(Duration::from_secs(10));
        let conn = ConnectionManager::new_with_config(client, config).await.map_err(backend_err)?;
        Ok(Self::new(conn, prefix))
    }

    /// Use a connection manager the application already has.
    pub fn new(conn: ConnectionManager, prefix: &str) -> Self {
        let script = |body: &str| Script::new(&format!("{PRELUDE}{body}"));
        RedisBackend {
            conn,
            prefix: prefix.trim_end_matches(':').to_owned(),
            dead_limit: DEFAULT_DEAD_LIMIT,
            scripts: Scripts {
                enqueue: script(ENQUEUE),
                claim: script(CLAIM),
                heartbeat: script(HEARTBEAT),
                complete: script(COMPLETE),
                retry: script(RETRY),
                dead_letter: script(DEAD_LETTER),
                reap: script(REAP),
                cancel: script(CANCEL),
                retry_dead: script(RETRY_DEAD),
                purge_dead: script(PURGE_DEAD),
            },
        }
    }

    /// Keep at most `limit` dead jobs per queue (default 1000).
    pub fn dead_limit(mut self, limit: usize) -> Self {
        self.dead_limit = limit.max(1);
        self
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Delete every key under the prefix: queues, jobs, locks. For tests and
    /// for wiping a development database.
    pub async fn clear(&self) -> Result<usize, Error> {
        let mut conn = self.conn.clone();
        let pattern = format!("{}:*", glob_escape(&self.prefix));
        let mut cursor = 0u64;
        let mut deleted = 0;
        loop {
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(500)
                .query_async(&mut conn)
                .await
                .map_err(backend_err)?;
            if !keys.is_empty() {
                deleted += keys.len();
                let _: () = redis::cmd("DEL").arg(&keys).query_async(&mut conn).await.map_err(backend_err)?;
            }
            if next == 0 {
                return Ok(deleted);
            }
            cursor = next;
        }
    }

    fn job_prefix(&self) -> String {
        format!("{}:job:", self.prefix)
    }

    fn job_key(&self, id: &str) -> String {
        format!("{}:job:{id}", self.prefix)
    }

    fn queue_key(&self, queue: &str, part: &str) -> String {
        format!("{}:q:{queue}:{part}", self.prefix)
    }

    async fn run<T: FromRedisValue>(&self, invocation: &redis::ScriptInvocation<'_>) -> Result<T, Error> {
        let mut conn = self.conn.clone();
        invocation.invoke_async(&mut conn).await.map_err(backend_err)
    }
}

fn glob_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn parse_record(data: &str) -> Result<JobRecord, Error> {
    serde_json::from_str(data).map_err(|e| Error::Backend(format!("corrupt job record: {e}")))
}

/// Stands in for a record that no longer parses, so the dead-letter list
/// stays readable (and purgeable) with such an entry in it.
fn placeholder_record(id: &str, queue: &str) -> JobRecord {
    JobRecord {
        id: id.to_owned(),
        name: "(corrupt record)".to_owned(),
        queue: queue.to_owned(),
        payload: serde_json::Value::Null,
        priority: 0,
        max_attempts: 1,
        timeout_ms: 0,
        backoff: crate::backoff::Backoff::default(),
        unique_key: None,
        unique_ttl_ms: None,
        enqueued_at_ms: 0,
        run_at_ms: 0,
    }
}

impl Backend for RedisBackend {
    fn kind(&self) -> &'static str {
        "redis"
    }

    fn enqueue(&self, job: JobRecord) -> BoxFuture<'_, BackendResult<Enqueued>> {
        Box::pin(async move {
            job.validate()?;
            let data = serde_json::to_string(&job).map_err(|e| Error::Serialize(e.to_string()))?;
            let job_key = self.job_key(&job.id);
            let uniq_key = job.unique_key.as_ref().map(|k| format!("{}:uniq:{k}", self.prefix));
            let mut call = self.scripts.enqueue.prepare_invoke();
            call.key(&job_key)
                .key(self.queue_key(&job.queue, "ready"))
                .key(self.queue_key(&job.queue, "scheduled"))
                .key(self.queue_key(&job.queue, "stats"))
                .key(format!("{}:queues", self.prefix))
                .key(uniq_key.as_deref().unwrap_or(&job_key))
                .arg(&job.id)
                .arg(data)
                .arg(job.max_attempts)
                .arg(i64::from(job.priority.clamp(-MAX_PRIORITY, MAX_PRIORITY)))
                .arg(job.run_at_ms.saturating_sub(now_ms()).min(MAX_DELAY_MS))
                .arg(&job.queue)
                .arg(if uniq_key.is_some() { "1" } else { "0" })
                .arg(job.unique_ttl_ms.unwrap_or(UNIQUE_SAFETY_TTL_MS).clamp(1, MAX_TTL_MS))
                .arg(if uniq_key.is_some() && job.unique_ttl_ms.is_none() { "1" } else { "0" });
            let (created, id): (i64, String) = self.run(&call).await?;
            Ok(Enqueued { id, duplicate: created == 0 })
        })
    }

    fn claim<'a>(&'a self, queue: &'a str, visibility: Duration) -> BoxFuture<'a, BackendResult<Option<ClaimedJob>>> {
        Box::pin(async move {
            loop {
                let token = new_id();
                let mut call = self.scripts.claim.prepare_invoke();
                call.key(self.queue_key(queue, "scheduled"))
                    .key(self.queue_key(queue, "ready"))
                    .key(self.queue_key(queue, "active"))
                    .arg(self.job_prefix())
                    .arg(duration_ms(visibility).clamp(1, MAX_DELAY_MS))
                    .arg(&token);
                let Some((data, attempt, id)): Option<(String, u32, String)> = self.run(&call).await? else {
                    return Ok(None);
                };
                match parse_record(&data) {
                    Ok(mut record) => {
                        // The keys are the truth: a record that disagrees with
                        // them could never be acknowledged.
                        record.id = id;
                        record.queue = queue.to_owned();
                        return Ok(Some(ClaimedJob { record, attempt, token }));
                    }
                    // It will not parse next time either: bury it now rather
                    // than fail this claim once per attempt, and move on.
                    Err(e) => {
                        let corrupt = ClaimedJob { record: placeholder_record(&id, queue), attempt, token };
                        self.dead_letter(&corrupt, &e.to_string()).await?;
                    }
                }
            }
        })
    }

    fn heartbeat<'a>(&'a self, job: &'a ClaimedJob, visibility: Duration) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut call = self.scripts.heartbeat.prepare_invoke();
            call.key(self.job_key(&job.record.id))
                .key(self.queue_key(&job.record.queue, "active"))
                .arg(&job.record.id)
                .arg(&job.token)
                .arg(duration_ms(visibility).clamp(1, MAX_DELAY_MS));
            Ok(self.run::<i64>(&call).await? == 1)
        })
    }

    fn complete<'a>(&'a self, job: &'a ClaimedJob) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut call = self.scripts.complete.prepare_invoke();
            call.key(self.job_key(&job.record.id))
                .key(self.queue_key(&job.record.queue, "active"))
                .key(self.queue_key(&job.record.queue, "stats"))
                .arg(&job.record.id)
                .arg(&job.token);
            Ok(self.run::<i64>(&call).await? == 1)
        })
    }

    fn retry<'a>(&'a self, job: &'a ClaimedJob, run_at_ms: u64, error: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut call = self.scripts.retry.prepare_invoke();
            call.key(self.job_key(&job.record.id))
                .key(self.queue_key(&job.record.queue, "active"))
                .key(self.queue_key(&job.record.queue, "scheduled"))
                .key(self.queue_key(&job.record.queue, "stats"))
                .arg(&job.record.id)
                .arg(&job.token)
                .arg(run_at_ms.saturating_sub(now_ms()).min(MAX_DELAY_MS))
                .arg(error);
            Ok(self.run::<i64>(&call).await? == 1)
        })
    }

    fn dead_letter<'a>(&'a self, job: &'a ClaimedJob, error: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut call = self.scripts.dead_letter.prepare_invoke();
            call.key(self.job_key(&job.record.id))
                .key(self.queue_key(&job.record.queue, "active"))
                .key(self.queue_key(&job.record.queue, "dead"))
                .key(self.queue_key(&job.record.queue, "stats"))
                .arg(&job.record.id)
                .arg(&job.token)
                .arg(error)
                .arg(self.dead_limit)
                .arg(self.job_prefix());
            Ok(self.run::<i64>(&call).await? == 1)
        })
    }

    fn reap<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<Reaped>> {
        Box::pin(async move {
            let mut call = self.scripts.reap.prepare_invoke();
            call.key(self.queue_key(queue, "active"))
                .key(self.queue_key(queue, "ready"))
                .key(self.queue_key(queue, "dead"))
                .key(self.queue_key(queue, "stats"))
                .arg(self.job_prefix())
                .arg(self.dead_limit);
            let (requeued, dead): (usize, usize) = self.run(&call).await?;
            Ok(Reaped { requeued, dead })
        })
    }

    fn cancel<'a>(&'a self, queue: &'a str, id: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut call = self.scripts.cancel.prepare_invoke();
            call.key(self.job_key(id))
                .key(self.queue_key(queue, "ready"))
                .key(self.queue_key(queue, "scheduled"))
                .arg(id);
            Ok(self.run::<i64>(&call).await? == 1)
        })
    }

    fn stats<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<QueueStats>> {
        Box::pin(async move {
            let mut conn = self.conn.clone();
            let scheduled_key = self.queue_key(queue, "scheduled");
            let (ready, scheduled, due, active, dead, counters): (
                u64,
                u64,
                u64,
                u64,
                u64,
                std::collections::HashMap<String, u64>,
            ) = redis::pipe()
                .cmd("ZCARD")
                .arg(self.queue_key(queue, "ready"))
                .cmd("ZCARD")
                .arg(&scheduled_key)
                .cmd("ZCOUNT")
                .arg(&scheduled_key)
                .arg("-inf")
                .arg(now_ms())
                .cmd("ZCARD")
                .arg(self.queue_key(queue, "active"))
                .cmd("ZCARD")
                .arg(self.queue_key(queue, "dead"))
                .cmd("HGETALL")
                .arg(self.queue_key(queue, "stats"))
                .query_async(&mut conn)
                .await
                .map_err(backend_err)?;
            let counter = |name: &str| counters.get(name).copied().unwrap_or(0);
            Ok(QueueStats {
                ready: ready + due,
                scheduled: scheduled.saturating_sub(due),
                active,
                dead,
                enqueued_total: counter("enqueued"),
                completed_total: counter("completed"),
                retried_total: counter("retried"),
                dead_total: counter("dead"),
            })
        })
    }

    fn queues(&self) -> BoxFuture<'_, BackendResult<Vec<String>>> {
        Box::pin(async move {
            let mut conn = self.conn.clone();
            let mut names: Vec<String> = redis::cmd("SMEMBERS")
                .arg(format!("{}:queues", self.prefix))
                .query_async(&mut conn)
                .await
                .map_err(backend_err)?;
            names.sort();
            Ok(names)
        })
    }

    fn dead_jobs<'a>(&'a self, queue: &'a str, limit: usize) -> BoxFuture<'a, BackendResult<Vec<DeadJob>>> {
        Box::pin(async move {
            if limit == 0 {
                return Ok(Vec::new());
            }
            let mut conn = self.conn.clone();
            let ids: Vec<String> = redis::cmd("ZREVRANGE")
                .arg(self.queue_key(queue, "dead"))
                .arg(0)
                .arg(limit - 1)
                .query_async(&mut conn)
                .await
                .map_err(backend_err)?;
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            let mut pipe = redis::pipe();
            for id in &ids {
                pipe.cmd("HMGET").arg(self.job_key(id)).arg("data").arg("attempt").arg("error").arg("failed_at");
            }
            type Row = (Option<String>, Option<u32>, Option<String>, Option<u64>);
            let rows: Vec<Row> = pipe.query_async(&mut conn).await.map_err(backend_err)?;
            let mut out = Vec::with_capacity(rows.len());
            for (id, (data, attempt, error, failed_at)) in ids.iter().zip(rows) {
                let Some(data) = data else { continue };
                out.push(DeadJob {
                    // One bad entry must not hide the rest of the list.
                    record: parse_record(&data).unwrap_or_else(|_| placeholder_record(id, queue)),
                    attempts: attempt.unwrap_or(0),
                    error: error.unwrap_or_default(),
                    failed_at_ms: failed_at.unwrap_or(0),
                });
            }
            Ok(out)
        })
    }

    fn retry_dead<'a>(&'a self, queue: &'a str, id: &'a str) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut call = self.scripts.retry_dead.prepare_invoke();
            call.key(self.job_key(id)).key(self.queue_key(queue, "dead")).key(self.queue_key(queue, "ready")).arg(id);
            Ok(self.run::<i64>(&call).await? == 1)
        })
    }

    fn purge_dead<'a>(&'a self, queue: &'a str) -> BoxFuture<'a, BackendResult<usize>> {
        Box::pin(async move {
            let mut call = self.scripts.purge_dead.prepare_invoke();
            call.key(self.queue_key(queue, "dead")).arg(self.job_prefix());
            self.run(&call).await
        })
    }

    fn try_lock<'a>(&'a self, key: &'a str, owner: &'a str, ttl: Duration) -> BoxFuture<'a, BackendResult<bool>> {
        Box::pin(async move {
            let mut conn = self.conn.clone();
            let set: Option<String> = redis::cmd("SET")
                .arg(format!("{}:lock:{key}", self.prefix))
                .arg(owner)
                .arg("NX")
                .arg("PX")
                .arg(duration_ms(ttl).clamp(1, MAX_TTL_MS))
                .query_async(&mut conn)
                .await
                .map_err(backend_err)?;
            Ok(set.is_some())
        })
    }
}
