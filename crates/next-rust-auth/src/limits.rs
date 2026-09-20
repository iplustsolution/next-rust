//! Rate limits: a [`Quota`] counted under a key by a [`Limiter`].
//!
//! The framework's `rate_limit` middleware is a per-process token bucket
//! over whole requests. This module is for the limits an auth flow needs on
//! top of that: *per action* and *per subject* ("ten password attempts per
//! account per 15 minutes, 200 per network"), optionally shared between
//! instances through Redis.
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap().block_on(async {
//! use next_rust_auth::limits::{key, Quota, RateLimiter};
//!
//! const LOGIN_PER_ACCOUNT: Quota = Quota::new(10, 15 * 60);
//! const LOGIN_PER_NETWORK: Quota = Quota::new(200, 15 * 60);
//!
//! let limiter = RateLimiter::memory();            // or RateLimiter::new(RedisLimiter::connect(..).await?)
//! let ip = "203.0.113.77".parse().ok();
//!
//! limiter.check(&key::scoped("login:net", &key::network(ip)), LOGIN_PER_NETWORK).await.unwrap();
//! let account = key::scoped("login:acct", &key::identifier("Ada@Example.com"));
//! limiter.check(&account, LOGIN_PER_ACCOUNT).await.unwrap();
//! // ... after a correct password:
//! limiter.reset(&account).await;
//! # });
//! ```
//!
//! # Windows
//!
//! * **Fixed** (the default): a counter that starts with the first event and
//!   expires `window` later. One `INCR` in Redis. A client can spend a quota
//!   at the end of one window and again at the start of the next, so up to
//!   `2 × max` events can fall in one `window`-long span.
//! * **Sliding** ([`Quota::sliding`]): remembers the time of each allowed
//!   event, so no `window`-long span ever holds more than `max`. Costs
//!   `max` timestamps per key; use it for small quotas where the bound matters
//!   (codes sent, password attempts), not for request floods.
//!
//! Events over the quota are refused and, in both windows, do not extend the
//! wait: `retry_after` says when the next event will be allowed.
//!
//! # Memory
//!
//! The in-memory backend holds at most 100 000 keys; when a flood of made-up
//! keys fills it, keys it is not yet counting share one counter per quota
//! until spent counters free room (limits get coarser, never looser). Keys
//! over [`MAX_KEY_LEN`] bytes are replaced by a hash, and windows are capped
//! at [`MAX_WINDOW`].
//!
//! # When the backend is down
//!
//! [`RateLimiter`] *fails open* by default: a Redis outage lets requests
//! through (and logs it) instead of locking everyone out. Choose
//! [`RateLimiter::fail_closed`] for limits that are the only guard of
//! something expensive or dangerous. Keep a second guard that does not depend
//! on the cache where it matters, such as an account lockout in the database.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use next_rust_server::{BoxFuture, Middleware, Next, Request, Response};
use tokio::time::Instant;

/// How events are counted; see the [module docs](self#windows).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Window {
    #[default]
    Fixed,
    Sliding,
}

/// `max` events per `window`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quota {
    pub max: u32,
    pub window: Duration,
    pub kind: Window,
}

impl Quota {
    /// `max` events per `window_secs` seconds, fixed window.
    pub const fn new(max: u32, window_secs: u64) -> Self {
        Quota { max, window: Duration::from_secs(window_secs), kind: Window::Fixed }
    }

    pub const fn per(max: u32, window: Duration) -> Self {
        Quota { max, window, kind: Window::Fixed }
    }

    pub const fn per_minute(max: u32) -> Self {
        Quota::new(max, 60)
    }

    pub const fn per_hour(max: u32) -> Self {
        Quota::new(max, 3600)
    }

    /// One event per `secs` seconds: a resend cooldown.
    pub const fn cooldown(secs: u64) -> Self {
        Quota::new(1, secs)
    }

    /// Count with a sliding window.
    pub const fn sliding(mut self) -> Self {
        self.kind = Window::Sliding;
        self
    }

    /// At least one event per at least one millisecond, so backends never
    /// divide by or wait for zero; at most [`MAX_WINDOW`], so adding a window
    /// to a clock reading can never overflow (and panic) and the millisecond
    /// count always fits what Redis takes.
    pub(crate) fn normalized(self) -> Self {
        Quota { max: self.max.max(1), window: self.window.clamp(Duration::from_millis(1), MAX_WINDOW), kind: self.kind }
    }
}

/// The longest window a backend counts over (ten years); longer ones are
/// treated as this.
pub const MAX_WINDOW: Duration = Duration::from_secs(10 * 365 * 24 * 60 * 60);

/// Keys longer than this are replaced by a hash before they reach a backend.
pub const MAX_KEY_LEN: usize = 256;

/// `key`, or a fixed-size stand-in when it is longer than [`MAX_KEY_LEN`]:
/// a key built from request data (an API key header, a user name) must not
/// let a client choose how much memory each counter costs.
pub(crate) fn bounded_key(key: &str) -> std::borrow::Cow<'_, str> {
    if key.len() <= MAX_KEY_LEN {
        return std::borrow::Cow::Borrowed(key);
    }
    // Keep the head (the scope, usually) readable for whoever inspects Redis.
    let mut head = 64;
    while !key.is_char_boundary(head) {
        head -= 1;
    }
    std::borrow::Cow::Owned(format!("{}~{}", &key[..head], &crate::crypto::sha256_hex(key)[..32]))
}

/// The answer to one counted event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub allowed: bool,
    /// Events left in the window after this one.
    pub remaining: u32,
    /// How long until an event will be allowed again; zero when `allowed`.
    pub retry_after: Duration,
}

/// The backend could not answer (Redis unreachable, timed out).
#[derive(Debug, Clone)]
pub struct LimitError(pub String);

impl std::fmt::Display for LimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rate limit backend: {}", self.0)
    }
}

impl std::error::Error for LimitError {}

pub type LimitFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, LimitError>> + Send + 'a>>;

/// A store of counters. Implemented by [`MemoryLimiter`] and, with the
/// `redis` feature, `RedisLimiter`.
pub trait Limiter: Send + Sync + 'static {
    /// Counts one event under `key`. The event is recorded only when allowed
    /// (sliding) or always (fixed; the counter keeps climbing but its expiry
    /// does not move).
    fn hit<'a>(&'a self, key: &'a str, quota: Quota) -> LimitFuture<'a, Decision>;

    /// Forgets `key` (after a correct password, a correct code).
    fn reset<'a>(&'a self, key: &'a str) -> LimitFuture<'a, ()>;
}

// ---------------------------------------------------------------------------
// In-memory backend
// ---------------------------------------------------------------------------

enum Counter {
    Fixed { count: u32, until: Instant },
    Sliding { events: VecDeque<Instant>, window: Duration },
}

impl Counter {
    fn spent(&self, now: Instant) -> bool {
        match self {
            Counter::Fixed { until, .. } => *until <= now,
            Counter::Sliding { events, window } => events.back().is_none_or(|last| *last + *window <= now),
        }
    }
}

/// Keys kept per limiter. Above it, spent counters are dropped (at most once
/// a second); while it is still exceeded, new keys share one counter *per
/// quota*, so a flood of keys can neither grow memory nor make every event
/// scan the table. While the table is full, new keys are therefore limited
/// together (strict, never loose): a flood cannot switch limits off, it can
/// only make them coarse for keys that were not being counted yet.
const MAX_KEYS: usize = 100_000;

/// One shared counter per quota shape. Sharing a single counter between
/// quotas would let a fixed and a sliding quota restart each other's count
/// on every event, which is no limit at all.
fn overflow_key(quota: Quota) -> String {
    let kind = match quota.kind {
        Window::Fixed => 'f',
        Window::Sliding => 's',
    };
    format!("\0overflow:{kind}:{}:{}", quota.max, quota.window.as_millis())
}

/// Counters in this process: for one instance, development and tests.
#[derive(Default)]
pub struct MemoryLimiter {
    state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    counters: HashMap<String, Counter>,
    pruned: Option<Instant>,
}

impl MemoryLimiter {
    pub fn new() -> Self {
        MemoryLimiter::default()
    }

    /// Keys currently held (for tests and metrics).
    pub fn len(&self) -> usize {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).counters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn hit_now(&self, key: &str, quota: Quota) -> Decision {
        self.hit_capped(key, quota, MAX_KEYS)
    }

    fn hit_capped(&self, key: &str, quota: Quota, max_keys: usize) -> Decision {
        let quota = quota.normalized();
        let now = Instant::now();
        let bounded = bounded_key(key);
        let overflow;
        let mut key: &str = &bounded;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.counters.len() >= max_keys && !state.counters.contains_key(key) {
            if state.pruned.is_none_or(|at| now.duration_since(at) >= Duration::from_secs(1)) {
                state.pruned = Some(now);
                state.counters.retain(|_, c| !c.spent(now));
            }
            if state.counters.len() >= max_keys {
                overflow = overflow_key(quota);
                key = &overflow;
            }
        }
        // A counter of the other kind, or a spent one, starts over.
        let fresh = match state.counters.get(key) {
            None => true,
            Some(c) if c.spent(now) => true,
            Some(Counter::Fixed { .. }) => quota.kind != Window::Fixed,
            Some(Counter::Sliding { .. }) => quota.kind != Window::Sliding,
        };
        if fresh {
            let counter = match quota.kind {
                Window::Fixed => Counter::Fixed { count: 0, until: now + quota.window },
                Window::Sliding => Counter::Sliding { events: VecDeque::new(), window: quota.window },
            };
            state.counters.insert(key.to_owned(), counter);
        }
        match state.counters.get_mut(key).expect("inserted above") {
            Counter::Fixed { count, until } => {
                *count = count.saturating_add(1);
                if *count <= quota.max {
                    Decision { allowed: true, remaining: quota.max - *count, retry_after: Duration::ZERO }
                } else {
                    Decision { allowed: false, remaining: 0, retry_after: until.duration_since(now) }
                }
            }
            Counter::Sliding { events, window } => {
                *window = quota.window;
                while events.front().is_some_and(|at| *at + quota.window <= now) {
                    events.pop_front();
                }
                if (events.len() as u64) < u64::from(quota.max) {
                    events.push_back(now);
                    let remaining = quota.max - events.len() as u32;
                    Decision { allowed: true, remaining, retry_after: Duration::ZERO }
                } else {
                    let oldest = *events.front().expect("max >= 1, so a full window has an event");
                    Decision { allowed: false, remaining: 0, retry_after: (oldest + quota.window).duration_since(now) }
                }
            }
        }
    }
}

impl Limiter for MemoryLimiter {
    fn hit<'a>(&'a self, key: &'a str, quota: Quota) -> LimitFuture<'a, Decision> {
        Box::pin(async move { Ok(self.hit_now(key, quota)) })
    }

    fn reset<'a>(&'a self, key: &'a str) -> LimitFuture<'a, ()> {
        Box::pin(async move {
            self.state.lock().unwrap_or_else(|e| e.into_inner()).counters.remove(&*bounded_key(key));
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// The front: failure policy and errors
// ---------------------------------------------------------------------------

/// The quota is spent. Converts into a 429 [`next_rust_server::Error`], so
/// `limiter.check(..).await?` works in loaders, actions and API routes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Throttled {
    pub retry_after: Duration,
}

impl Throttled {
    /// Whole seconds to wait, at least one: the `Retry-After` value.
    pub fn retry_after_secs(&self) -> u64 {
        (self.retry_after.as_millis().div_ceil(1000) as u64).max(1)
    }

    /// `429 Too Many Requests` with `Retry-After`.
    pub fn response(&self) -> Response {
        Response::text("Too Many Requests")
            .with_status(429)
            .with_header("retry-after", &self.retry_after_secs().to_string())
    }
}

impl std::fmt::Display for Throttled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "too many requests; retry in {}s", self.retry_after_secs())
    }
}

impl From<Throttled> for next_rust_server::Error {
    fn from(t: Throttled) -> Self {
        next_rust_server::Error::http(429, format!("Too many requests. Try again in {} seconds.", t.retry_after_secs()))
    }
}

/// What a backend failure means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OnBackendError {
    /// Let the event through (and log): an outage must not lock everyone out.
    #[default]
    Allow,
    /// Refuse the event.
    Deny,
}

/// A [`Limiter`] with a failure policy. Cheap to clone; share one per backend.
#[derive(Clone)]
pub struct RateLimiter {
    backend: Arc<dyn Limiter>,
    on_error: OnBackendError,
}

impl RateLimiter {
    /// Over `backend`, failing open.
    pub fn new(backend: impl Limiter) -> Self {
        RateLimiter { backend: Arc::new(backend), on_error: OnBackendError::Allow }
    }

    /// Over a backend that is already shared.
    pub fn shared(backend: Arc<dyn Limiter>) -> Self {
        RateLimiter { backend, on_error: OnBackendError::Allow }
    }

    /// Over a fresh [`MemoryLimiter`].
    pub fn memory() -> Self {
        RateLimiter::new(MemoryLimiter::new())
    }

    /// Refuse events while the backend cannot answer.
    pub fn fail_closed(mut self) -> Self {
        self.on_error = OnBackendError::Deny;
        self
    }

    pub fn on_backend_error(mut self, policy: OnBackendError) -> Self {
        self.on_error = policy;
        self
    }

    /// Counts one event; `Err` when the quota is spent.
    ///
    /// Keys longer than [`MAX_KEY_LEN`] are hashed first, whatever the backend.
    pub async fn check(&self, key: &str, quota: Quota) -> Result<Decision, Throttled> {
        let key = &*bounded_key(key);
        match self.backend.hit(key, quota).await {
            Ok(decision) if decision.allowed => Ok(decision),
            Ok(decision) => Err(Throttled { retry_after: decision.retry_after }),
            Err(e) => match self.on_error {
                OnBackendError::Allow => {
                    next_rust_server::log::warn(&format!("rate limit check failed open for {key:?}: {e}"));
                    Ok(Decision { allowed: true, remaining: quota.max, retry_after: Duration::ZERO })
                }
                OnBackendError::Deny => {
                    next_rust_server::log::warn(&format!("rate limit check failed closed for {key:?}: {e}"));
                    Err(Throttled { retry_after: Duration::from_secs(5) })
                }
            },
        }
    }

    /// Forgets `key`. A backend failure is logged, never returned: the
    /// counter then simply runs out on its own.
    pub async fn reset(&self, key: &str) {
        let key = &*bounded_key(key);
        if let Err(e) = self.backend.reset(key).await {
            next_rust_server::log::warn(&format!("rate limit reset failed for {key:?}: {e}"));
        }
    }

    /// Middleware that counts every request it sees against `quota`, keyed by
    /// the client's network (see [`key::network`]).
    ///
    /// ```ignore
    /// // app/api/middleware.rs
    /// static LIMIT: LazyLock<RequestLimit> =
    ///     LazyLock::new(|| limiter().requests(Quota::per_minute(120)).scope("api"));
    ///
    /// pub async fn middleware(req: Request, next: Next) -> Response {
    ///     LIMIT.handle(req, next).await
    /// }
    /// ```
    pub fn requests(&self, quota: Quota) -> RequestLimit {
        RequestLimit {
            limiter: self.clone(),
            quota,
            scope: "req".into(),
            key: Arc::new(|req: &Request| key::network(req.client_ip())),
        }
    }
}

/// The middleware adapter made by [`RateLimiter::requests`]. It complements
/// the framework's `rate_limit`: that one is an in-process token bucket;
/// this one counts in whatever backend the limiter has, so a quota holds
/// across instances, and it answers with the same `429` + `Retry-After`.
#[derive(Clone)]
pub struct RequestLimit {
    limiter: RateLimiter,
    quota: Quota,
    scope: String,
    key: Arc<dyn Fn(&Request) -> String + Send + Sync>,
}

impl RequestLimit {
    /// Names this limit's counters (default `"req"`); two limits with the
    /// same scope share counters.
    pub fn scope(mut self, scope: &str) -> Self {
        self.scope = scope.to_owned();
        self
    }

    /// Custom key, e.g. an API key or a user id.
    pub fn key(mut self, f: impl Fn(&Request) -> String + Send + Sync + 'static) -> Self {
        self.key = Arc::new(f);
        self
    }
}

impl Middleware for RequestLimit {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        let this = self.clone();
        Box::pin(async move {
            let key = key::scoped(&this.scope, &(this.key)(&req));
            match this.limiter.check(&key, this.quota).await {
                Ok(decision) => {
                    let mut res = next.run(req).await;
                    res.set_header("ratelimit-remaining", &decision.remaining.to_string());
                    res
                }
                Err(throttled) => throttled.response(),
            }
        })
    }
}

/// Building rate-limit keys.
pub mod key {
    use super::IpAddr;

    /// The network an address belongs to: the /24 for IPv4 (and IPv4-mapped
    /// IPv6), the /64 for IPv6, `"unknown"` without an address. One customer
    /// usually owns a whole /64, and attackers rotate through neighbouring
    /// IPv4 addresses, so per-address counters would never fill.
    pub fn network(ip: Option<IpAddr>) -> String {
        match ip {
            Some(IpAddr::V4(v4)) => v4_network(v4),
            Some(IpAddr::V6(v6)) => match v6.to_ipv4_mapped() {
                Some(v4) => v4_network(v4),
                None => {
                    let s = v6.segments();
                    format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
                }
            },
            None => "unknown".to_owned(),
        }
    }

    fn v4_network(v4: std::net::Ipv4Addr) -> String {
        let o = v4.octets();
        format!("{}.{}.{}.0/24", o[0], o[1], o[2])
    }

    /// A key for an email address, phone number or user name: trimmed,
    /// lowercased and hashed, so the limiter's store never holds the
    /// identifier itself and its length and characters are no concern.
    /// Nothing is Unicode-normalised: two spellings your account lookup
    /// treats as different are counted separately here too.
    pub fn identifier(value: &str) -> String {
        crate::crypto::sha256_hex(&value.trim().to_lowercase())[..32].to_owned()
    }

    /// `"{scope}:{part}"`. The scope is yours (a constant); when `part` comes
    /// from the request, pass it through [`identifier`] or [`network`] (or
    /// make sure it cannot contain `:`), so one client's part can never
    /// spell another scope's key.
    pub fn scoped(scope: &str, part: &str) -> String {
        format!("{scope}:{part}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[tokio::test(start_paused = true)]
    async fn a_fixed_window_counts_and_starts_over() {
        let limiter = MemoryLimiter::new();
        let quota = Quota::new(3, 60);
        for left in [2, 1, 0] {
            let d = limiter.hit("k", quota).await.unwrap();
            assert_eq!((d.allowed, d.remaining), (true, left));
        }
        tokio::time::advance(secs(20)).await;
        let d = limiter.hit("k", quota).await.unwrap();
        assert!(!d.allowed);
        assert_eq!(d.retry_after, secs(40), "the window started with the first event");
        tokio::time::advance(secs(30)).await;
        assert_eq!(limiter.hit("k", quota).await.unwrap().retry_after, secs(10), "refusals do not extend it");
        assert!(limiter.hit("other", quota).await.unwrap().allowed, "keys are independent");
        tokio::time::advance(secs(10)).await;
        assert_eq!(limiter.hit("k", quota).await.unwrap().remaining, 2, "a new window");
    }

    #[tokio::test(start_paused = true)]
    async fn a_fixed_window_allows_a_burst_across_the_boundary_and_a_sliding_one_does_not() {
        let limiter = MemoryLimiter::new();
        let fixed = Quota::new(2, 60);
        let sliding = fixed.sliding();
        // One event now, one at 59s, then as many as possible right after 60s.
        for q in [("f", fixed), ("s", sliding)] {
            assert!(limiter.hit(q.0, q.1).await.unwrap().allowed);
        }
        tokio::time::advance(secs(59)).await;
        for q in [("f", fixed), ("s", sliding)] {
            assert!(limiter.hit(q.0, q.1).await.unwrap().allowed);
        }
        tokio::time::advance(secs(2)).await;
        assert!(limiter.hit("f", fixed).await.unwrap().allowed);
        assert!(limiter.hit("f", fixed).await.unwrap().allowed, "fixed: three events within three seconds");
        assert!(limiter.hit("s", sliding).await.unwrap().allowed, "the first event left the window");
        let d = limiter.hit("s", sliding).await.unwrap();
        assert!(!d.allowed, "sliding: never more than two in any minute");
        assert_eq!(d.retry_after, secs(58), "until the event at 59s leaves");
    }

    #[tokio::test(start_paused = true)]
    async fn a_cooldown_and_a_reset() {
        let limiter = RateLimiter::memory();
        let resend = Quota::cooldown(45);
        assert!(limiter.check("resend:u1", resend).await.is_ok());
        let wait = limiter.check("resend:u1", resend).await.unwrap_err();
        assert_eq!(wait.retry_after_secs(), 45);
        tokio::time::advance(Duration::from_millis(44_500)).await;
        assert_eq!(limiter.check("resend:u1", resend).await.unwrap_err().retry_after_secs(), 1, "rounded up");
        limiter.reset("resend:u1").await;
        assert!(limiter.check("resend:u1", resend).await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn spent_counters_are_pruned_and_a_zero_quota_still_allows_one() {
        let limiter = MemoryLimiter::new();
        assert!(limiter.hit("z", Quota::new(0, 0)).await.unwrap().allowed);
        assert_eq!(limiter.len(), 1);
        assert!(!limiter.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_full_table_shares_one_counter_per_quota_and_never_stops_limiting() {
        let limiter = MemoryLimiter::new();
        let fixed = Quota::new(3, 60);
        let sliding = Quota::new(3, 60).sliding();
        for i in 0..8 {
            limiter.hit_capped(&format!("flood-{i}"), Quota::new(100, 600), 8);
        }
        assert_eq!(limiter.len(), 8);
        // New keys, alternating quota kinds: with one shared counter the two
        // kinds restarted each other and nothing was ever refused.
        let mut allowed = (0, 0);
        for i in 0..20 {
            if limiter.hit_capped(&format!("victim-f-{i}"), fixed, 8).allowed {
                allowed.0 += 1;
            }
            if limiter.hit_capped(&format!("victim-s-{i}"), sliding, 8).allowed {
                allowed.1 += 1;
            }
        }
        assert_eq!(allowed, (3, 3), "each quota holds for the keys that share its overflow counter");
        assert_eq!(limiter.len(), 10, "the flood grew the table by the two overflow counters only");
        // Keys already counted keep their own counters.
        assert!(limiter.hit_capped("flood-0", Quota::new(100, 600), 8).allowed);
        // Once the flood's windows are over, the table is pruned and keys are separate again.
        tokio::time::advance(secs(601)).await;
        assert!(limiter.hit_capped("victim-f-new", fixed, 8).allowed);
        assert!(limiter.hit_capped("victim-f-other", fixed, 8).allowed);
        assert!(limiter.len() <= 3);
    }

    #[tokio::test(start_paused = true)]
    async fn long_keys_are_hashed_and_huge_windows_do_not_overflow() {
        let limiter = MemoryLimiter::new();
        let long_a = format!("api:{}", "a".repeat(100_000));
        let long_b = format!("api:{}b", "a".repeat(100_000));
        let quota = Quota::new(1, 60);
        assert!(limiter.hit(&long_a, quota).await.unwrap().allowed);
        assert!(!limiter.hit(&long_a, quota).await.unwrap().allowed, "the same long key is the same counter");
        assert!(limiter.hit(&long_b, quota).await.unwrap().allowed, "another long key is another counter");
        {
            let state = limiter.state.lock().unwrap();
            assert!(state.counters.keys().all(|k| k.len() <= MAX_KEY_LEN), "no counter is stored under a huge key");
            assert!(state.counters.keys().all(|k| k.starts_with("api:")), "the scope stays readable");
        }
        limiter.reset(&long_a).await.unwrap();
        assert!(limiter.hit(&long_a, quota).await.unwrap().allowed, "reset finds the hashed key");
        assert_eq!(bounded_key("short"), "short");
        assert!(bounded_key(&"é".repeat(400)).len() <= MAX_KEY_LEN, "cut on a character boundary");

        // Duration::MAX added to a clock reading used to panic.
        for quota in [Quota::per(2, Duration::MAX), Quota::per(2, Duration::MAX).sliding()] {
            let key = format!("forever-{:?}", quota.kind);
            assert!(limiter.hit(&key, quota).await.unwrap().allowed);
            assert!(limiter.hit(&key, quota).await.unwrap().allowed);
            let refused = limiter.hit(&key, quota).await.unwrap();
            assert!(!refused.allowed);
            assert!(refused.retry_after <= MAX_WINDOW);
        }
        assert_eq!(Quota::new(u32::MAX, u64::MAX).normalized().window, MAX_WINDOW);
    }

    struct Broken;
    impl Limiter for Broken {
        fn hit<'a>(&'a self, _: &'a str, _: Quota) -> LimitFuture<'a, Decision> {
            Box::pin(async { Err(LimitError("connection refused".into())) })
        }
        fn reset<'a>(&'a self, _: &'a str) -> LimitFuture<'a, ()> {
            Box::pin(async { Err(LimitError("connection refused".into())) })
        }
    }

    #[tokio::test]
    async fn a_broken_backend_fails_open_or_closed_as_chosen() {
        let open = RateLimiter::new(Broken);
        assert!(open.check("k", Quota::new(1, 60)).await.is_ok());
        assert!(open.check("k", Quota::new(1, 60)).await.is_ok());
        open.reset("k").await;
        let closed = RateLimiter::new(Broken).fail_closed();
        assert!(closed.check("k", Quota::new(1, 60)).await.is_err());
    }

    #[test]
    fn keys_group_networks_and_hide_identifiers() {
        let net = |s: &str| key::network(s.parse().ok());
        assert_eq!(net("203.0.113.77"), "203.0.113.0/24");
        assert_eq!(net("203.0.113.9"), net("203.0.113.200"));
        assert_ne!(net("203.0.113.9"), net("203.0.114.9"));
        assert_eq!(net("2001:db8:1:2:3:4:5:6"), "2001:db8:1:2::/64");
        assert_eq!(net("2001:db8:1:2:aaaa::1"), net("2001:db8:1:2:bbbb::2"));
        assert_ne!(net("2001:db8:1:2::1"), net("2001:db8:1:3::1"));
        assert_eq!(net("::ffff:203.0.113.9"), "203.0.113.0/24", "IPv4-mapped");
        assert_eq!(net("::ffff:203.0.113.200"), net("203.0.113.9"), "mapped and plain share a counter");
        assert_eq!(net("::ffff:cb00:7109"), "203.0.113.0/24", "IPv4-mapped, hex spelling");
        assert_eq!(net("::1"), "0:0:0:0::/64");
        assert_eq!(net("0.0.0.0"), "0.0.0.0/24");
        assert_eq!(key::network(None), "unknown");

        let id = key::identifier("  Ada@Example.com ");
        assert_eq!(id, key::identifier("ada@example.com"));
        assert_eq!(id.len(), 32);
        assert!(!id.contains("ada"));
        assert_eq!(key::scoped("login:acct", "x"), "login:acct:x");
    }

    #[tokio::test]
    async fn the_middleware_answers_429_with_retry_after() {
        use next_rust_server::middleware::Endpoint;
        let limit = RateLimiter::memory()
            .requests(Quota::new(2, 60))
            .scope("test")
            .key(|req| req.header("x-api-key").unwrap_or("anonymous").to_owned());
        let endpoint: Arc<dyn Endpoint> = Arc::new(|_req: Request| async { Response::text("ok") });
        let send = |api_key: &'static str| {
            let mut req = Request::get("/api/things");
            req.headers_mut().insert("x-api-key", api_key.parse().unwrap());
            limit.handle(req, Next::new(Arc::from(Vec::new()), endpoint.clone()))
        };
        let first = send("a").await;
        assert_eq!((first.status.as_u16(), first.header("ratelimit-remaining")), (200, Some("1")));
        assert_eq!(send("a").await.status.as_u16(), 200);
        let third = send("a").await;
        assert_eq!(third.status.as_u16(), 429);
        assert_eq!(third.header("retry-after"), Some("60"));
        assert_eq!(send("b").await.status.as_u16(), 200, "another key has its own quota");
    }

    #[test]
    fn throttled_becomes_a_429_error() {
        let error: next_rust_server::Error = Throttled { retry_after: secs(30) }.into();
        assert_eq!(error.status().as_u16(), 429);
    }
}
