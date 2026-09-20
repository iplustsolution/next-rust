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
            key: Arc::new(|req: &Request| req.client_ip().map_or_else(|| "unknown".into(), client_key)),
        }
    }

    /// Custom key, e.g. an API key or user id.
    pub fn key(mut self, f: impl Fn(&Request) -> String + Send + Sync + 'static) -> Self {
        self.key = Arc::new(f);
        self
    }
}

/// The rate-limit key of an address: the address itself for IPv4, the /64
/// network for IPv6. One IPv6 customer usually owns a whole /64 (2^64
/// addresses), so per-address buckets would never fill.
fn client_key(ip: std::net::IpAddr) -> String {
    match ip {
        std::net::IpAddr::V4(v4) => v4.to_string(),
        std::net::IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => v4.to_string(),
            None => {
                let s = v6.segments();
                format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
            }
        },
    }
}

/// Buckets kept per limiter. Above it, full buckets are dropped (at most once
/// a second); while it is still exceeded, new clients share one bucket, so a
/// flood of addresses can neither grow memory nor make every request scan
/// the table.
const MAX_BUCKETS: usize = 100_000;
const OVERFLOW_KEY: &str = "\0overflow";

/// In-memory token bucket rate limiting (per process). Responds 429 with
/// `Retry-After` when exhausted. For multi-instance deployments, implement
/// your own middleware backed by a shared store.
pub fn rate_limit(config: RateLimit) -> impl Middleware {
    let buckets: Arc<Mutex<HashMap<String, (f64, Instant)>>> = Arc::new(Mutex::new(HashMap::new()));
    let pruned: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let config = Arc::new(config);
    move |req: Request, next: Next| {
        let buckets = buckets.clone();
        let pruned = pruned.clone();
        let config = config.clone();
        async move {
            let mut key = (config.key)(&req);
            let now = Instant::now();
            let refill = config.refill_every.as_secs_f64().max(f64::EPSILON);
            let (allowed, retry) = {
                let mut map = buckets.lock().unwrap_or_else(|e| e.into_inner());
                if map.len() >= MAX_BUCKETS && !map.contains_key(&key) {
                    let mut last = pruned.lock().unwrap_or_else(|e| e.into_inner());
                    if last.is_none_or(|at| now.duration_since(at) >= Duration::from_secs(1)) {
                        *last = Some(now);
                        // Drop full buckets to bound memory.
                        map.retain(|_, (tokens, at)| {
                            *tokens + now.duration_since(*at).as_secs_f64() / refill < config.capacity as f64
                        });
                    }
                    if map.len() >= MAX_BUCKETS {
                        key = OVERFLOW_KEY.to_owned();
                    }
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
                // Path and query only, and only when it is a path on this
                // site: the login page redirects to it afterwards.
                let here = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
                let here = if crate::response::is_local_path(here) { here } else { "/" };
                let target = format!("{login}?next={}", encode_query_value(here));
                Response::redirect(&target)
            } else {
                Response::text("Unauthorized").with_status(401)
            }
        }
    }
}

/// Percent-encode a query value. `&`, `+`, `=` and `#` are encoded too: left
/// as they are, a request for `/a?x=1&next=//evil.example` would add a second
/// `next` parameter to the login URL.
fn encode_query_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
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

/// Add `'wasm-unsafe-eval'` to a policy's `script-src`, which is what a
/// browser requires before it will compile a WebAssembly module (a page with
/// `#[client(wasm = "…")]` islands fails without it, with a `CompileError`).
/// The keyword permits `WebAssembly.instantiate` **only** — it does not bring
/// back `eval` or inline scripts. Policies without a `script-src` are left
/// alone: they fall back to `default-src`, which the caller controls.
fn allow_wasm(csp: &str) -> String {
    if csp.contains("'wasm-unsafe-eval'") {
        return csp.to_owned();
    }
    let mut out = String::with_capacity(csp.len() + 19);
    let mut added = false;
    for (i, directive) in csp.split(';').enumerate() {
        if i > 0 {
            out.push(';');
        }
        if !added && directive.trim_start().starts_with("script-src ") {
            out.push_str(directive.trim_end());
            out.push_str(" 'wasm-unsafe-eval'");
            added = true;
        } else {
            out.push_str(directive);
        }
    }
    out
}

/// Apply a conservative set of security headers (enabled by default through
/// `[security] headers = true`). `wasm` mirrors `Config::uses_wasm()` — any of
/// `[build] wasm_islands`, `[client] runtime = "wasm"` and `[ui] script =
/// "wasm"`: it relaxes `script-src` by exactly one keyword so the browser may
/// compile the module. It composes with `[analytics]`, which has already
/// widened the same directive with the tag origins by the time this runs.
pub(crate) fn apply_security_headers(
    res: &mut Response,
    csp: Option<&str>,
    nonce: &str,
    hsts_max_age: u64,
    wasm: bool,
) {
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
        let csp = csp.replace("{nonce}", nonce);
        let csp = if wasm { allow_wasm(&csp) } else { csp };
        res.set_header("content-security-policy", &csp);
    }
    if hsts_max_age > 0 && !res.headers.contains_key("strict-transport-security") {
        res.set_header("strict-transport-security", &format!("max-age={hsts_max_age}; includeSubDomains"));
    }
}

/// Middleware form of the security headers (for custom setups).
pub fn security_headers() -> impl Middleware {
    |req: Request, next: Next| async move {
        let mut res = next.run(req).await;
        apply_security_headers(&mut res, None, "", 0, false);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_islands_relax_script_src_by_one_keyword() {
        let csp = allow_wasm(STRICT_CSP);
        assert!(csp.contains("script-src 'self' 'nonce-{nonce}' 'strict-dynamic' 'wasm-unsafe-eval';"), "{csp}");
        // Nothing else moves, and no other directive gains the keyword.
        assert_eq!(csp.matches("'wasm-unsafe-eval'").count(), 1, "{csp}");
        assert!(csp.contains("default-src 'self';") && csp.contains("object-src 'none'"), "{csp}");
        assert_eq!(csp.split(';').count(), STRICT_CSP.split(';').count());
        // Idempotent: a policy that already allows it is returned unchanged.
        assert_eq!(allow_wasm(&csp), csp);
        // A policy with no `script-src` falls back to `default-src`; leave it.
        assert_eq!(allow_wasm("default-src 'self'"), "default-src 'self'");
        // Only the first `script-src` is touched, and `script-src-elem` is not it.
        let elem = allow_wasm("script-src-elem 'self'; script-src 'self'");
        assert_eq!(elem, "script-src-elem 'self'; script-src 'self' 'wasm-unsafe-eval'");
    }

    /// The two things that widen `script-src` must compose: `[analytics]`
    /// adds the tag origins while the policy is built (`tracking::strict_csp`),
    /// and `Config::uses_wasm()` appends `'wasm-unsafe-eval'` while the
    /// response is written. Neither may undo the other, and the keyword must
    /// land in `script-src` — never in `connect-src`, `img-src` or `frame-src`,
    /// which analytics widens too.
    #[test]
    fn analytics_widening_and_wasm_compose_in_one_script_src() {
        use crate::tracking::Tracking;
        use next_rust_core::{Config, Environment};

        let policy = |toml: &str, wasm: bool| {
            let config = Config::from_toml_str(toml).expect("valid config");
            let tracking = Tracking::new(&config, Environment::Production);
            let mut res = Response::text("ok");
            apply_security_headers(&mut res, tracking.csp(), "N0", 0, wasm);
            res.headers
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok())
                .expect("a policy was written")
                .to_owned()
        };

        let toml = "[analytics]\ngoogle_tag_manager = \"GTM-ABC123\"\n[security]\ncsp = \"strict\"\n";
        let with_wasm = policy(toml, true);
        let without = policy(toml, false);

        // The analytics origins survived, and so did every other directive:
        // the only difference between the two policies is the keyword.
        assert_eq!(with_wasm.replace(" 'wasm-unsafe-eval'", ""), without, "{with_wasm}");
        assert_eq!(with_wasm.matches("'wasm-unsafe-eval'").count(), 1, "{with_wasm}");
        assert!(
            with_wasm.contains(
                "script-src 'self' 'nonce-N0' 'strict-dynamic' https://www.googletagmanager.com 'wasm-unsafe-eval';"
            ),
            "the keyword belongs at the end of the widened script-src: {with_wasm}"
        );
        // Analytics widens these three as well; none of them may gain it.
        for directive in with_wasm.split(';').map(str::trim) {
            let widened_elsewhere = directive.starts_with("connect-src")
                || directive.starts_with("img-src")
                || directive.starts_with("frame-src");
            assert!(
                !(widened_elsewhere && directive.contains("'wasm-unsafe-eval'")),
                "`{directive}` must not allow wasm compilation: {with_wasm}"
            );
        }
        assert!(without.contains("googletagmanager"), "the analytics origins are unconditional: {without}");
        assert!(!without.contains("wasm"), "a project with no wasm gets the unrelaxed policy: {without}");

        // A hand-written policy is the app's to maintain: analytics leaves it
        // alone, and so does the wasm relaxation when it has no `script-src`.
        let custom =
            policy("[analytics]\ngoogle_analytics = \"G-ABCD1234\"\n[security]\ncsp = \"default-src 'self'\"\n", true);
        assert_eq!(custom, "default-src 'self'");
    }

    /// `Config::uses_wasm()` is what the CSP keys off, so each of the three
    /// switches on its own has to answer yes — a page hydrating the wasm
    /// runtime with no islands of its own still compiles a module.
    #[test]
    fn every_wasm_switch_relaxes_the_policy() {
        use next_rust_core::Config;

        for toml in [
            "[build]\nwasm_islands = true\n",
            "[client]\nruntime = \"wasm\"\n",
            "[ui]\nscript = \"wasm\"\n",
            "[build]\nwasm_islands = true\n[client]\nruntime = \"wasm\"\n[ui]\nscript = \"wasm\"\n",
        ] {
            let config = Config::from_toml_str(toml).expect("valid config");
            assert!(config.uses_wasm(), "{toml}");
            let mut res = Response::text("ok");
            apply_security_headers(&mut res, Some("strict"), "N0", 0, config.uses_wasm());
            let csp = res.headers.get("content-security-policy").and_then(|v| v.to_str().ok()).expect("a policy");
            assert_eq!(csp.matches("'wasm-unsafe-eval'").count(), 1, "{toml}: {csp}");
        }
        let plain = Config::from_toml_str("[build]\nminify_js = true\n").expect("valid config");
        assert!(!plain.uses_wasm(), "the defaults keep an existing app exactly as it was");
    }

    #[test]
    fn ipv6_clients_are_limited_per_network() {
        let key = |s: &str| client_key(s.parse().unwrap());
        assert_eq!(key("203.0.113.7"), "203.0.113.7");
        assert_eq!(key("2001:db8:1:2:aaaa::1"), key("2001:db8:1:2:bbbb::2"), "same /64");
        assert_ne!(key("2001:db8:1:2::1"), key("2001:db8:1:3::1"));
        assert_eq!(key("::ffff:203.0.113.7"), "203.0.113.7", "IPv4-mapped");
    }

    #[tokio::test]
    async fn protected_only_passes_on_a_local_next() {
        let mw = protected(|_| false, "/login");
        let endpoint: Arc<dyn Endpoint> = Arc::new(|_req: Request| async { Response::text("secret") });
        let next = || Next::new(Arc::from(Vec::new()), endpoint.clone());
        let res = mw.handle(Request::get("/account?tab=1"), next()).await;
        assert_eq!(res.header("location"), Some("/login?next=%2Faccount%3Ftab%3D1"));
        let res = mw.handle(Request::get("/a?x=1&next=//evil.example"), next()).await;
        assert_eq!(res.header("location"), Some("/login?next=%2Fa%3Fx%3D1%26next%3D%2F%2Fevil.example"));
        let res = mw.handle(Request::get("//evil.example/x"), next()).await;
        assert_eq!(res.header("location"), Some("/login?next=%2F"));
    }

    /// xorshift64*: seeded, so a failure is reproducible.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
            from[self.below(from.len())]
        }
    }

    /// Generated addresses: a key is always derived, never panics, and two
    /// addresses share a key only when they are meant to.
    #[test]
    fn every_address_shape_gets_a_key() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
        let mut rng = Rng(0x1_C0DE_0FFE_5EED);
        for _ in 0..20_000 {
            let v4 = Ipv4Addr::from(rng.next() as u32);
            let v6 = Ipv6Addr::from(rng.next() as u128 ^ ((rng.next() as u128) << 64));
            let mapped = v4.to_ipv6_mapped();
            assert_eq!(client_key(IpAddr::V4(v4)), client_key(IpAddr::V6(mapped)), "{v4} vs {mapped}");
            let key = client_key(IpAddr::V6(v6));
            assert!(!key.is_empty());
            // Two addresses in the same /64 share a bucket; a different /64
            // does not, whatever the low half is.
            let mut same = v6.segments();
            same[4] ^= 0xffff;
            same[7] = same[7].wrapping_add(1);
            assert_eq!(client_key(IpAddr::V6(Ipv6Addr::from(same))), key);
            let mut other = v6.segments();
            other[3] = other[3].wrapping_add(1);
            if Ipv6Addr::from(other).to_ipv4_mapped().is_none() && v6.to_ipv4_mapped().is_none() {
                assert_ne!(client_key(IpAddr::V6(Ipv6Addr::from(other))), key);
            }
        }
        // The addresses that are easy to get wrong.
        for (addr, want) in [
            ("::", "0:0:0:0::/64"),
            ("::1", "0:0:0:0::/64"),
            ("::ffff:0:0", "0.0.0.0"),
            ("::ffff:127.0.0.1", "127.0.0.1"),
            ("::ffff:255.255.255.255", "255.255.255.255"),
            ("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff", "ffff:ffff:ffff:ffff::/64"),
            ("2001:db8::", "2001:db8:0:0::/64"),
        ] {
            assert_eq!(client_key(addr.parse().unwrap()), want, "{addr}");
        }
        assert_eq!(client_key("0.0.0.0".parse().unwrap()), "0.0.0.0");
        // A /64 key can never be mistaken for an IPv4 key, and neither can be
        // mistaken for the internal overflow bucket.
        assert!(!client_key("::".parse().unwrap()).contains('.'));
        assert_ne!(client_key("::".parse().unwrap()), OVERFLOW_KEY);
    }

    const NEXT_PIECES: &[&str] = &[
        "/",
        "//",
        "/a",
        "..",
        "%2e%2e",
        "?",
        "&",
        "=",
        "#",
        "%23",
        "next=",
        "//evil.example",
        "%2f%2fevil.example",
        "https:",
        "javascript:",
        "%00",
        "%",
        "~",
        ".",
        "-",
        "_",
        "héllo",
        "%c3%a9",
        ";",
        ":",
        "@",
        "+",
        "'",
        "\"",
        "<",
    ];

    /// Whatever a visitor asks for, the login redirect stays on this site and
    /// carries exactly one `next` parameter.
    #[tokio::test]
    async fn a_generated_request_target_never_leaves_the_login_redirect() {
        let mw = protected(|_| false, "/login");
        let endpoint: Arc<dyn Endpoint> = Arc::new(|_req: Request| async { Response::text("secret") });
        let mut rng = Rng(0x10C4_1B0D_0001);
        let mut reached = 0usize;
        for _ in 0..20_000 {
            let mut target = String::new();
            for _ in 0..1 + rng.below(6) {
                target.push_str(rng.pick(NEXT_PIECES));
            }
            let Ok(req) = std::panic::catch_unwind(|| http::Request::get(&target).body(bytes::Bytes::new())) else {
                continue;
            };
            let Ok(req) = req else { continue };
            reached += 1;
            let next = Next::new(Arc::from(Vec::new()), endpoint.clone());
            let res = mw.handle(Request::from_http(req), next).await;
            let location = res.header("location").unwrap_or_else(|| panic!("no Location for {target:?}"));
            assert!(crate::response::is_local_path(location), "{target:?} -> {location:?}");
            assert!(location.starts_with("/login?next=%2F"), "{target:?} -> {location:?}");
            assert_eq!(location.matches('?').count(), 1, "{target:?} -> {location:?}");
            assert_eq!(location.matches("next=").count(), 1, "{target:?} -> {location:?}");
            assert!(!location.contains('#') && !location.contains('&'), "{target:?} -> {location:?}");
            // Everything after `next=` is a percent-encoded local path.
            let encoded = location.trim_start_matches("/login?next=");
            let decoded: String = decode(encoded);
            assert!(crate::response::is_local_path(&decoded), "{target:?} -> {decoded:?}");
        }
        assert!(reached > 5_000, "the generator produces valid request targets ({reached})");
    }

    fn decode(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' && i + 2 < b.len() {
                out.push(u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap(), 16).unwrap());
                i += 3;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn a_query_value_is_encoded_down_to_the_unreserved_characters() {
        for (input, want) in [
            ("/a", "%2Fa"),
            ("", ""),
            ("-._~", "-._~"),
            ("a&b=c#d+e", "a%26b%3Dc%23d%2Be"),
            ("\r\n", "%0D%0A"),
            ("\u{0}", "%00"),
            ("é", "%C3%A9"),
        ] {
            assert_eq!(encode_query_value(input), want, "{input:?}");
        }
        // The output is always ASCII and always decodes back to the input.
        let mut rng = Rng(0x9A17_C0DE_0002);
        for _ in 0..20_000 {
            let text: String = (0..rng.below(10)).map(|_| rng.pick(NEXT_PIECES)).collect();
            let encoded = encode_query_value(&text);
            assert!(encoded.is_ascii() && !encoded.contains(['&', '=', '#', '?', '/']));
            assert_eq!(decode(&encoded), text, "{text:?}");
        }
    }
}
