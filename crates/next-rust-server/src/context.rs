//! Render context and extractors.
//!
//! Page, layout and metadata functions declare what they need as arguments;
//! the generated route glue resolves each argument through [`FromContext`]:
//!
//! ```ignore
//! pub async fn Page(Path(p): Path<PostParams>, cookies: Cookies) -> Result<impl View> { .. }
//! ```
//!
//! Arguments that read request data (cookies, headers, query, ...) make the
//! route dynamic under `rendering = "auto"`.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use http::{HeaderMap, Method, Uri};
use next_rust_core::Config;
use next_rust_router::Params;
use serde::de::DeserializeOwned;

use crate::cookies::Cookies;
use crate::error::{Error, ErrorKind, Result, not_found};

/// Whether a render serves a live request or runs at build/revalidation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Request,
    Static,
}

/// Everything known about the request being rendered.
pub struct RequestContext {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub params: Params,
    pub cookies: Cookies,
    pub extensions: http::Extensions,
    pub remote_addr: Option<SocketAddr>,
    /// Per-request CSP nonce.
    pub nonce: String,
    pub mode: RenderMode,
    /// Development mode: detailed error messages.
    pub dev: bool,
    pub config: Arc<Config>,
    pub(crate) status: AtomicU16,
    pub(crate) response_headers: Mutex<HeaderMap>,
}

/// Shared handle to the context.
pub type Ctx = Arc<RequestContext>;

impl RequestContext {
    pub fn path(&self) -> &str {
        self.uri.path()
    }

    pub fn is_static(&self) -> bool {
        self.mode == RenderMode::Static
    }

    /// Override the response status (e.g. 201, 410).
    pub fn set_status(&self, code: u16) {
        self.status.store(code, Ordering::Relaxed);
    }

    pub fn status(&self) -> u16 {
        self.status.load(Ordering::Relaxed)
    }

    pub(crate) fn raise_status(&self, code: u16) {
        // Error statuses from boundaries win over 200, first one sticks.
        let _ = self.status.compare_exchange(200, code, Ordering::Relaxed, Ordering::Relaxed);
    }

    fn require_request(&self, what: &'static str) -> Result<()> {
        if self.is_static() { Err(Error::new(ErrorKind::DynamicUsage(what))) } else { Ok(()) }
    }

    /// Build a context for tests or static rendering.
    pub fn for_static(path: &str, params: Params, config: Arc<Config>) -> Ctx {
        Self::for_static_with(path, params, config, crate::is_dev())
    }

    pub(crate) fn for_static_with(path: &str, params: Params, config: Arc<Config>, dev: bool) -> Ctx {
        Arc::new(RequestContext {
            method: Method::GET,
            uri: path.parse().unwrap_or_else(|_| Uri::from_static("/")),
            headers: HeaderMap::new(),
            params,
            cookies: Cookies::default(),
            extensions: http::Extensions::new(),
            remote_addr: None,
            nonce: crate::random_hex(16),
            mode: RenderMode::Static,
            dev,
            config,
            status: AtomicU16::new(200),
            response_headers: Mutex::new(HeaderMap::new()),
        })
    }

    pub(crate) fn from_request(req: &crate::Request, config: Arc<Config>, nonce: String, dev: bool) -> Ctx {
        Arc::new(RequestContext {
            method: req.method().clone(),
            uri: req.uri().clone(),
            headers: req.headers().clone(),
            params: req.params().clone(),
            cookies: req.cookies().clone(),
            extensions: req.extensions().clone(),
            remote_addr: req.remote_addr(),
            nonce,
            mode: RenderMode::Request,
            dev,
            config,
            status: AtomicU16::new(200),
            response_headers: Mutex::new(HeaderMap::new()),
        })
    }
}

/// Resolve a function argument from the render context.
pub trait FromContext: Sized {
    fn from_context(ctx: &Ctx) -> Result<Self>;
}

impl FromContext for Ctx {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        Ok(ctx.clone())
    }
}

impl FromContext for Params {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        Ok(ctx.params.clone())
    }
}

/// Typed route parameters: `Path(p): Path<BlogParams>`. A parameter that
/// fails to parse (e.g. `/users/abc` for `id: u64`) renders the 404 page.
#[derive(Debug, Clone)]
pub struct Path<T>(pub T);

impl<T: DeserializeOwned> FromContext for Path<T> {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        crate::params_de::from_params(&ctx.params).map(Path).map_err(|_| not_found())
    }
}

/// Parsed query string: `Query(q): Query` (a map) or `Query<T>` (typed).
#[derive(Debug, Clone)]
pub struct Query<T = QueryMap>(pub T);

/// Query parameters as a multi-map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryMap(pub Vec<(String, String)>);

impl QueryMap {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn get_all(&self, name: &str) -> Vec<&str> {
        self.0.iter().filter(|(k, _)| k == name).map(|(_, v)| v.as_str()).collect()
    }
}

impl<'de> serde::Deserialize<'de> for QueryMap {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Vec::<(String, String)>::deserialize(d).map(QueryMap)
    }
}

impl<T: DeserializeOwned> FromContext for Query<T> {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("Query")?;
        serde_urlencoded::from_str(ctx.uri.query().unwrap_or(""))
            .map(Query)
            .map_err(|e| Error::http(400, format!("invalid query string: {e}")))
    }
}

impl FromContext for Cookies {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("Cookies")?;
        Ok(ctx.cookies.clone())
    }
}

/// Request headers.
#[derive(Debug, Clone)]
pub struct Headers(pub HeaderMap);

impl Headers {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(|v| v.to_str().ok())
    }
}

impl FromContext for Headers {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("Headers")?;
        Ok(Headers(ctx.headers.clone()))
    }
}

/// Basic request information.
#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub method: Method,
    pub uri: Uri,
    pub remote_addr: Option<SocketAddr>,
}

impl FromContext for RequestInfo {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("RequestInfo")?;
        Ok(RequestInfo { method: ctx.method.clone(), uri: ctx.uri.clone(), remote_addr: ctx.remote_addr })
    }
}

/// A value inserted by middleware (`req.insert_extension(user)`), e.g. the
/// authenticated user: `Extension(user): Extension<User>`. Missing values
/// produce a 500 in development with a clear message; use
/// `Option<Extension<T>>` for optional values.
#[derive(Debug, Clone)]
pub struct Extension<T>(pub T);

impl<T: Clone + Send + Sync + 'static> FromContext for Extension<T> {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("Extension")?;
        ctx.extensions.get::<T>().cloned().map(Extension).ok_or_else(|| {
            Error::msg(format!(
                "Extension<{}> is not set; insert it in middleware with `req.insert_extension(..)`",
                std::any::type_name::<T>()
            ))
        })
    }
}

impl<T: Clone + Send + Sync + 'static> FromContext for Option<Extension<T>> {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("Extension")?;
        Ok(ctx.extensions.get::<T>().cloned().map(Extension))
    }
}

/// Authentication context primitive: set by your auth middleware, read by
/// pages. `Auth<T>` is `Option`-like: `auth.user()` is `None` for guests.
#[derive(Debug, Clone)]
pub struct Auth<T>(pub Option<T>);

impl<T> Auth<T> {
    pub fn user(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub fn is_authenticated(&self) -> bool {
        self.0.is_some()
    }

    /// The user, or a redirect to `login`.
    pub fn require(self, login: &str) -> Result<T> {
        self.0.ok_or_else(|| crate::error::redirect(login))
    }
}

/// Wrapper middleware inserts: `req.insert_extension(AuthUser(user))`.
#[derive(Debug, Clone)]
pub struct AuthUser<T>(pub T);

impl<T: Clone + Send + Sync + 'static> FromContext for Auth<T> {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("Auth")?;
        Ok(Auth(ctx.extensions.get::<AuthUser<T>>().map(|u| u.0.clone())))
    }
}

/// The per-request CSP nonce (for your own inline scripts).
#[derive(Debug, Clone)]
pub struct Nonce(pub String);

impl FromContext for Nonce {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        Ok(Nonce(ctx.nonce.clone()))
    }
}

/// Add headers to the page response.
#[derive(Clone)]
pub struct ResponseHeaders(Ctx);

impl ResponseHeaders {
    pub fn set(&self, name: &str, value: &str) {
        if let (Ok(n), Ok(v)) = (http::HeaderName::from_bytes(name.as_bytes()), http::HeaderValue::from_str(value)) {
            self.0.response_headers.lock().unwrap_or_else(|e| e.into_inner()).insert(n, v);
        }
    }
}

impl FromContext for ResponseHeaders {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("ResponseHeaders")?;
        Ok(ResponseHeaders(ctx.clone()))
    }
}

/// Result of the route's `load` function: `Data(posts): Data<Vec<Post>>`.
#[derive(Debug, Clone)]
pub struct Data<T>(pub T);

/// CSRF token for forms when `csrf()` middleware or `[security] csrf = "token"` is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfToken(pub String);

impl CsrfToken {
    /// Hidden input carrying the token.
    pub fn field(&self) -> next_rust_view::Element {
        next_rust_view::Element::new_void("input")
            .with(next_rust_view::attrs::type_("hidden"))
            .with(next_rust_view::attrs::name("_csrf"))
            .with(next_rust_view::attrs::value(self.0.clone()))
    }
}

impl FromContext for CsrfToken {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("CsrfToken")?;
        Ok(ctx
            .extensions
            .get::<CsrfToken>()
            .cloned()
            .or_else(|| ctx.cookies.get(crate::CSRF_COOKIE).map(CsrfToken))
            .unwrap_or_else(|| {
                let t = crate::random_hex(32);
                ctx.cookies.set(crate::Cookie::new(crate::CSRF_COOKIE, t.clone()).http_only(false));
                CsrfToken(t)
            }))
    }
}

/// Result of the last progressive-enhancement form submission (validation
/// errors / error message), read once from a short-lived flash cookie.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormState {
    pub errors: BTreeMap<String, String>,
    pub message: Option<String>,
    pub values: BTreeMap<String, String>,
}

impl FormState {
    pub fn error(&self, field: &str) -> Option<&str> {
        self.errors.get(field).map(String::as_str)
    }

    pub fn value(&self, field: &str) -> &str {
        self.values.get(field).map(String::as_str).unwrap_or("")
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.message.is_some()
    }
}

impl FromContext for FormState {
    fn from_context(ctx: &Ctx) -> Result<Self> {
        ctx.require_request("FormState")?;
        let Some(raw) = ctx.cookies.get_decoded(crate::actions::FLASH_COOKIE) else { return Ok(FormState::default()) };
        ctx.cookies.delete(crate::actions::FLASH_COOKIE);
        Ok(crate::actions::decode_flash(&raw))
    }
}
