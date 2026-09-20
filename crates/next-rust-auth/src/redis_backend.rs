//! Redis backends (the `redis` feature): [`RedisLimiter`] and [`RedisGrace`].
//!
//! Both work over a [`ConnectionManager`]: one multiplexed connection that
//! reconnects by itself and is cheap to clone. Every command has a deadline
//! (250 ms by default), so a stalled Redis turns into a [`LimitError`] that
//! the [`RateLimiter`](crate::limits::RateLimiter)'s failure policy decides
//! on, not into stalled requests.
//!
//! ```ignore
//! let limiter = RateLimiter::new(RedisLimiter::connect("redis://127.0.0.1/", "myapp:rl:").await?);
//! let sessions = Sessions::new(store, keys)
//!     .shared_grace(RedisGrace::new(limiter_connection.clone(), "myapp:grace:"))
//!     .limit_refresh(limiter.clone(), Quota::new(600, 900));
//! ```
//!
//! Counting is one Lua script per event, so it is atomic however many
//! instances count at once. The sliding window reads Redis's own clock
//! (`TIME`), so instances need not agree on the time; this needs Redis 5 or
//! newer (scripts replicated by effect).

use std::time::Duration;

use redis::aio::ConnectionManager;

use crate::limits::{Decision, LimitError, LimitFuture, Limiter, Quota, Window};
use crate::session::{GraceFuture, GraceStore};

/// Returns `{count, ms until the window ends}`.
const FIXED: &str = r"
local c = redis.call('INCR', KEYS[1])
local t = redis.call('PTTL', KEYS[1])
if c == 1 or t < 0 then
  redis.call('PEXPIRE', KEYS[1], ARGV[1])
  t = tonumber(ARGV[1])
end
return {c, t}
";

/// A sorted set of the allowed events' times. Returns
/// `{allowed, remaining, ms until the next event is allowed}`.
const SLIDING: &str = r"
local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000 + math.floor(tonumber(time[2]) / 1000)
local window = tonumber(ARGV[1])
local max = tonumber(ARGV[2])
redis.call('ZREMRANGEBYSCORE', KEYS[1], '-inf', now - window)
local n = redis.call('ZCARD', KEYS[1])
if n < max then
  redis.call('ZADD', KEYS[1], now, ARGV[3])
  redis.call('PEXPIRE', KEYS[1], window)
  return {1, max - n - 1, 0}
end
local oldest = redis.call('ZRANGE', KEYS[1], 0, 0, 'WITHSCORES')
return {0, 0, tonumber(oldest[2]) + window - now}
";

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(250);

async fn within<T>(
    timeout: Duration,
    command: impl Future<Output = redis::RedisResult<T>> + Send,
) -> Result<T, LimitError> {
    match tokio::time::timeout(timeout, command).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(LimitError(e.to_string())),
        Err(_) => Err(LimitError(format!("no answer within {timeout:?}"))),
    }
}

/// Counters in Redis, shared by every instance.
#[derive(Clone)]
pub struct RedisLimiter {
    conn: ConnectionManager,
    prefix: String,
    timeout: Duration,
    fixed: redis::Script,
    sliding: redis::Script,
}

impl RedisLimiter {
    /// Over an existing connection. Keys are `{prefix}{key}`; give each
    /// application (and each test run) its own prefix.
    pub fn new(conn: ConnectionManager, prefix: &str) -> Self {
        RedisLimiter {
            conn,
            prefix: prefix.to_owned(),
            timeout: DEFAULT_TIMEOUT,
            fixed: redis::Script::new(FIXED),
            sliding: redis::Script::new(SLIDING),
        }
    }

    /// Connects to `url` (`redis://host:port/db`).
    pub async fn connect(url: &str, prefix: &str) -> redis::RedisResult<Self> {
        let conn = ConnectionManager::new(redis::Client::open(url)?).await?;
        Ok(RedisLimiter::new(conn, prefix))
    }

    /// The deadline of one command (default 250 ms).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The connection, to share with [`RedisGrace`] or your own code.
    pub fn connection(&self) -> ConnectionManager {
        self.conn.clone()
    }
}

impl Limiter for RedisLimiter {
    fn hit<'a>(&'a self, key: &'a str, quota: Quota) -> LimitFuture<'a, Decision> {
        Box::pin(async move {
            // `normalized` caps the window, so the millisecond count fits.
            let quota = quota.normalized();
            let max = quota.max;
            let window_ms = (quota.window.as_millis() as u64).max(1);
            let full_key = format!("{}{}", self.prefix, crate::limits::bounded_key(key));
            let mut conn = self.conn.clone();
            match quota.kind {
                Window::Fixed => {
                    let mut call = self.fixed.key(full_key);
                    call.arg(window_ms);
                    let (count, ttl_ms): (u64, i64) = within(self.timeout, call.invoke_async(&mut conn)).await?;
                    let allowed = count <= u64::from(max);
                    Ok(Decision {
                        allowed,
                        remaining: u64::from(max).saturating_sub(count) as u32,
                        retry_after: if allowed { Duration::ZERO } else { Duration::from_millis(ttl_ms.max(1) as u64) },
                    })
                }
                Window::Sliding => {
                    let mut call = self.sliding.key(full_key);
                    call.arg(window_ms).arg(max).arg(crate::crypto::short_token());
                    let (allowed, remaining, wait_ms): (u8, u32, i64) =
                        within(self.timeout, call.invoke_async(&mut conn)).await?;
                    Ok(Decision {
                        allowed: allowed == 1,
                        remaining,
                        retry_after: if allowed == 1 {
                            Duration::ZERO
                        } else {
                            Duration::from_millis(wait_ms.max(1) as u64)
                        },
                    })
                }
            }
        })
    }

    fn reset<'a>(&'a self, key: &'a str) -> LimitFuture<'a, ()> {
        Box::pin(async move {
            let mut conn = self.conn.clone();
            let mut cmd = redis::cmd("DEL");
            cmd.arg(format!("{}{}", self.prefix, crate::limits::bounded_key(key)));
            within(self.timeout, cmd.query_async::<()>(&mut conn)).await
        })
    }
}

/// Grace entries in Redis, so a rotation on one instance is known to the
/// others (see [`crate::session`]). Values arrive sealed; Redis never holds
/// a readable refresh token.
#[derive(Clone)]
pub struct RedisGrace {
    conn: ConnectionManager,
    prefix: String,
    timeout: Duration,
}

impl RedisGrace {
    pub fn new(conn: ConnectionManager, prefix: &str) -> Self {
        RedisGrace { conn, prefix: prefix.to_owned(), timeout: DEFAULT_TIMEOUT }
    }

    pub async fn connect(url: &str, prefix: &str) -> redis::RedisResult<Self> {
        Ok(RedisGrace::new(ConnectionManager::new(redis::Client::open(url)?).await?, prefix))
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl GraceStore for RedisGrace {
    fn park<'a>(&'a self, key: &'a str, sealed: &'a str, ttl: Duration) -> GraceFuture<'a, ()> {
        Box::pin(async move {
            let mut conn = self.conn.clone();
            let mut cmd = redis::cmd("SET");
            cmd.arg(format!("{}{key}", self.prefix)).arg(sealed).arg("PX").arg((ttl.as_millis() as u64).max(1));
            if let Err(e) = within(self.timeout, cmd.query_async::<()>(&mut conn)).await {
                next_rust_server::log::warn(&format!("auth: grace entry not shared: {e}"));
            }
        })
    }

    fn find<'a>(&'a self, key: &'a str) -> GraceFuture<'a, Option<String>> {
        Box::pin(async move {
            let mut conn = self.conn.clone();
            let mut cmd = redis::cmd("GET");
            cmd.arg(format!("{}{key}", self.prefix));
            within(self.timeout, cmd.query_async::<Option<String>>(&mut conn)).await.ok().flatten()
        })
    }
}
