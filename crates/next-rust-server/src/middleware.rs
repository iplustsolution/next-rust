//! Middleware.
//!
//! A middleware is any `async fn(Request, Next) -> Response`:
//!
//! ```ignore
//! pub async fn middleware(req: Request, next: Next) -> Response {
//!     let start = std::time::Instant::now();
//!     let res = next.run(req).await;
//!     res.with_header("x-elapsed", &format!("{:?}", start.elapsed()))
//! }
//! ```
//!
//! Execution order for a request to `/dashboard/settings`:
//!
//! 1. global middleware registered with `App::middleware` (in order)
//! 2. `app/middleware.rs` (runs before routing, so it may rewrite the path)
//! 3. nested `middleware.rs` files from the outermost to the innermost
//!    matched segment (e.g. `app/dashboard/middleware.rs`)
//! 4. the page, API handler or server action

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::request::Request;
use crate::response::Response;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Middleware trait; implemented for all matching async functions/closures.
pub trait Middleware: Send + Sync + 'static {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response>;
}

impl<F, Fut> Middleware for F
where
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        Box::pin(self(req, next))
    }
}

/// Terminal handler at the end of a middleware chain.
pub trait Endpoint: Send + Sync + 'static {
    fn call(&self, req: Request) -> BoxFuture<Response>;
}

impl<F, Fut> Endpoint for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn call(&self, req: Request) -> BoxFuture<Response> {
        Box::pin(self(req))
    }
}

/// The rest of the chain.
#[derive(Clone)]
pub struct Next {
    stack: Arc<[Arc<dyn Middleware>]>,
    index: usize,
    endpoint: Arc<dyn Endpoint>,
}

impl Next {
    pub fn new(stack: Arc<[Arc<dyn Middleware>]>, endpoint: Arc<dyn Endpoint>) -> Self {
        Next { stack, index: 0, endpoint }
    }

    /// Run the remaining middleware and the endpoint.
    pub async fn run(self, req: Request) -> Response {
        match self.stack.get(self.index) {
            Some(mw) => {
                let mw = mw.clone();
                let next = Next { stack: self.stack, index: self.index + 1, endpoint: self.endpoint };
                mw.handle(req, next).await
            }
            None => self.endpoint.call(req).await,
        }
    }
}

/// Wrap a middleware function pointer generated from `middleware.rs`.
pub struct FnMiddleware(pub fn(Request, Next) -> BoxFuture<Response>);

impl Middleware for FnMiddleware {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        (self.0)(req, next)
    }
}

// ---------------------------------------------------------------------------
// Built-in middleware
// ---------------------------------------------------------------------------

/// Request identifier inserted by [`request_id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub String);

/// Ensure every request has an `x-request-id` (reused from the client when
/// it is a short, safe token) and echo it on the response.
pub fn request_id() -> impl Middleware {
    |mut req: Request, next: Next| async move {
        let id = req
            .header("x-request-id")
            .filter(|v| v.len() <= 128 && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
            .map(str::to_owned)
            .unwrap_or_else(|| crate::random_hex(16));
        req.insert_extension(RequestId(id.clone()));
        let mut res = next.run(req).await;
        res.set_header("x-request-id", &id);
        res
    }
}

/// CORS configuration.
#[derive(Debug, Clone)]
pub struct Cors {
    origins: Vec<String>,
    methods: String,
    headers: String,
    expose: Option<String>,
    credentials: bool,
    max_age: Option<Duration>,
}

impl Default for Cors {
    fn default() -> Self {
        Cors {
            origins: Vec::new(),
            methods: "GET, POST, PUT, PATCH, DELETE, OPTIONS".into(),
            headers: "content-type, authorization".into(),
            expose: None,
            credentials: false,
            max_age: Some(Duration::from_secs(600)),
        }
    }
}

impl Cors {
    /// Allow any origin (without credentials).
    pub fn permissive() -> Self {
        Cors { origins: vec!["*".into()], ..Default::default() }
    }

    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.origins.push(origin.into());
        self
    }

    pub fn allow_methods(mut self, methods: &[&str]) -> Self {
        self.methods = methods.join(", ");
        self
    }

    pub fn allow_headers(mut self, headers: &[&str]) -> Self {
        self.headers = headers.join(", ");
        self
    }

    pub fn expose_headers(mut self, headers: &[&str]) -> Self {
        self.expose = Some(headers.join(", "));
        self
    }

    pub fn allow_credentials(mut self, v: bool) -> Self {
        self.credentials = v;
        self
    }

    pub fn max_age(mut self, d: Duration) -> Self {
        self.max_age = Some(d);
        self
    }

    fn allowed_origin(&self, origin: &str) -> Option<String> {
        if self.origins.iter().any(|o| o == origin) {
            Some(origin.to_owned())
        } else if self.origins.iter().any(|o| o == "*") {
            // Credentials cannot be combined with a wildcard; echo the origin instead.
            Some(if self.credentials { origin.to_owned() } else { "*".into() })
        } else {
            None
        }
    }
}

/// CORS middleware, including preflight handling.
pub fn cors(config: Cors) -> impl Middleware {
    let config = Arc::new(config);
    move |req: Request, next: Next| {
        let config = config.clone();
        async move {
            let origin = req.header("origin").map(str::to_owned);
            let Some(origin) = origin else { return next.run(req).await };
            let allowed = config.allowed_origin(&origin);
            let preflight =
                req.method() == http::Method::OPTIONS && req.header("access-control-request-method").is_some();
            let mut res = if preflight { Response::status(204) } else { next.run(req).await };
            if let Some(allowed) = allowed {
                res.set_header("access-control-allow-origin", &allowed);
                if allowed != "*" {
                    res.append_header("vary", "Origin");
                }
                if config.credentials {
                    res.set_header("access-control-allow-credentials", "true");
                }
                if let Some(e) = &config.expose {
                    res.set_header("access-control-expose-headers", e);
                }
                if preflight {
                    res.set_header("access-control-allow-methods", &config.methods);
                    res.set_header("access-control-allow-headers", &config.headers);
                    if let Some(m) = config.max_age {
                        res.set_header("access-control-max-age", &m.as_secs().to_string());
                    }
                }
            }
            res
        }
    }
}

/// Key extractor for rate limiting.
pub type KeyFn = Arc<dyn Fn(&Request) -> String + Send + Sync>;

/// Fixed-window-with-token-bucket rate limiter configuration.
#[derive(Clone)]
pub struct RateLimit {
    capacity: u32,
    refill_every: Duration,
    key: KeyFn,
}

impl RateLimit {
    /// `n` requests per minute per client IP (burst up to `n`).
    pub fn per_minute(n: u32) -> Self {
        RateLimit::new(n, Duration::from_secs(60))
    }

    pub fn per_second(n: u32) -> Self {
        RateLimit::new(n, Duration::from_secs(1))
    }

    /// `capacity` tokens refilled evenly over `period`.
    pub fn new(capacity: u32, period: Duration) -> Self {
        let capacity = capacity.max(1);
        RateLimit {
            capacity,
            refill_every: period / capacity,
            key: Arc::new(|req: &Request| req.client_ip().map_or_else(|| "unknown".into(), |ip| ip.to_string())),
        }
    }

    /// Custom key, e.g. an API key or user id.
    pub fn key(mut self, f: impl Fn(&Request) -> String + Send + Sync + 'static) -> Self {
        self.key = Arc::new(f);
        self
    }
}

/// In-memory token bucket rate limiting (per process). Responds 429 with
/// `Retry-After` when exhausted. For multi-instance deployments, implement
/// your own middleware backed by a shared store.
pub fn rate_limit(config: RateLimit) -> impl Middleware {
    let buckets: Arc<Mutex<HashMap<String, (f64, Instant)>>> = Arc::new(Mutex::new(HashMap::new()));
    let config = Arc::new(config);
    move |req: Request, next: Next| {
        let buckets = buckets.clone();
        let config = config.clone();
        async move {
            let key = (config.key)(&req);
            let now = Instant::now();
            let refill = config.refill_every.as_secs_f64().max(f64::EPSILON);
            let (allowed, retry) = {
                let mut map = buckets.lock().unwrap_or_else(|e| e.into_inner());
                if map.len() > 100_000 {
                    // Drop full buckets to bound memory.
                    map.retain(|_, (tokens, at)| {
                        *tokens + now.duration_since(*at).as_secs_f64() / refill < config.capacity as f64
                    });
                }
                let entry = map.entry(key).or_insert((config.capacity as f64, now));
                let elapsed = now.duration_since(entry.1).as_secs_f64();
                entry.0 = (entry.0 + elapsed / refill).min(config.capacity as f64);
                entry.1 = now;
                if entry.0 >= 1.0 {
                    entry.0 -= 1.0;
                    (true, 0.0)
                } else {
                    (false, (1.0 - entry.0) * refill)
                }
            };
            if allowed {
                next.run(req).await
            } else {
                Response::text("Too Many Requests")
                    .with_status(429)
                    .with_header("retry-after", &(retry.ceil() as u64).max(1).to_string())
            }
        }
    }
}

/// Protect routes: when `check` returns false the user is redirected to
/// `login` (with `?next=` set) — or receives 401 for non-GET requests.
pub fn protected(check: impl Fn(&Request) -> bool + Send + Sync + 'static, login: &str) -> impl Middleware {
    let check = Arc::new(check);
    let login = login.to_owned();
    move |req: Request, next: Next| {
        let check = check.clone();
        let login = login.clone();
        async move {
            if check(&req) {
                return next.run(req).await;
            }
            if req.method() == http::Method::GET {
                let target = format!("{login}?next={}", crate::cookies::percent_encode(&req.uri().to_string()));
                Response::redirect(&target)
            } else {
                Response::text("Unauthorized").with_status(401)
            }
        }
    }
}

/// Double-submit CSRF token protection for unsafe methods.
///
/// A random token is kept in the `next_rust_csrf` cookie (readable by the page via
/// [`crate::CsrfToken`]); unsafe requests must echo it in the
/// `x-csrf-token` header or a `_csrf` form field.
pub fn csrf() -> impl Middleware {
    |req: Request, next: Next| csrf_check(req, next)
}

async fn csrf_check(mut req: Request, next: Next) -> Response {
    let cookie = req.cookies().get(crate::CSRF_COOKIE);
    let token = cookie.clone().unwrap_or_else(|| crate::random_hex(32));
    req.insert_extension(crate::CsrfToken(token.clone()));
    let safe = matches!(*req.method(), http::Method::GET | http::Method::HEAD | http::Method::OPTIONS);
    if !safe {
        let mut provided = req.header("x-csrf-token").map(str::to_owned);
        let is_form = req.header("content-type").is_some_and(|c| c.starts_with("application/x-www-form-urlencoded"));
        if provided.is_none() && is_form {
            provided = form_field(&mut req, "_csrf").await;
        }
        let valid = match (&cookie, &provided) {
            (Some(c), Some(p)) => crate::constant_time_eq(c.as_bytes(), p.as_bytes()),
            _ => false,
        };
        if !valid {
            return Response::text("Invalid CSRF token").with_status(403);
        }
    }
    if cookie.is_none() {
        // Readable by scripts so JS clients can send the header.
        req.cookies().set(crate::Cookie::new(crate::CSRF_COOKIE, token).http_only(false));
    }
    next.run(req).await
}

pub(crate) async fn form_field(req: &mut Request, name: &str) -> Option<String> {
    let bytes = req.bytes().await.ok()?;
    let fields: Vec<(String, String)> = serde_urlencoded::from_bytes(&bytes).ok()?;
    fields.into_iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// The policy `[security] csp = "strict"` stands for: scripts only with the
/// per-request nonce (`'strict-dynamic'` lets those scripts load modules and
/// islands), everything else from this origin, no plugins, no framing by
/// other sites, forms only to this origin. Inline `style` attributes stay
/// allowed: components set widths and custom properties with them.
pub const STRICT_CSP: &str = "default-src 'self'; \
script-src 'self' 'nonce-{nonce}' 'strict-dynamic'; \
style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob:; \
font-src 'self' data:; \
connect-src 'self'; \
media-src 'self' blob:; \
worker-src 'self' blob:; \
object-src 'none'; \
base-uri 'self'; \
form-action 'self'; \
frame-ancestors 'self'";

/// Apply a conservative set of security headers (enabled by default through
/// `[security] headers = true`).
pub(crate) fn apply_security_headers(res: &mut Response, csp: Option<&str>, nonce: &str, hsts_max_age: u64) {
    let csp = csp.map(|c| if c.trim() == "strict" { STRICT_CSP } else { c });
    let defaults = [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "SAMEORIGIN"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        ("cross-origin-opener-policy", "same-origin"),
        ("permissions-policy", "camera=(), microphone=(), geolocation=()"),
    ];
    for (k, v) in defaults {
        if !res.headers.contains_key(k) {
            res.set_header(k, v);
        }
    }
    if let Some(csp) = csp
        && !res.headers.contains_key("content-security-policy")
    {
        res.set_header("content-security-policy", &csp.replace("{nonce}", nonce));
    }
    if hsts_max_age > 0 && !res.headers.contains_key("strict-transport-security") {
        res.set_header("strict-transport-security", &format!("max-age={hsts_max_age}; includeSubDomains"));
    }
}

/// Middleware form of the security headers (for custom setups).
pub fn security_headers() -> impl Middleware {
    |req: Request, next: Next| async move {
        let mut res = next.run(req).await;
        apply_security_headers(&mut res, None, "", 0);
        res
    }
}
