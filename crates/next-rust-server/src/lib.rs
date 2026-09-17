//! The Next Rust runtime.
//!
//! Most applications use this crate through the `next-rust` facade. It
//! contains the request pipeline (middleware → routing → pages/API routes/
//! server actions), server-side rendering with streaming, incremental static
//! regeneration, static files, the development endpoints and the hyper-based
//! production server.

#![forbid(unsafe_code)]

pub mod actions;
pub mod app;
mod banner;
mod client_runtime;
pub mod context;
pub mod cookies;
pub mod embed;
pub mod error;
pub mod export;
pub mod http_date;
mod internal;
pub mod jobs;
pub mod log;
pub mod middleware;
mod params_de;
pub mod plugin;
mod redirects;
mod render;
pub mod request;
pub mod response;
pub mod runtime;
pub mod seo;
pub mod server;
pub mod session;
pub mod sse;
pub mod static_files;
pub mod testing;
#[cfg(feature = "websocket")]
pub mod ws;

use std::sync::atomic::{AtomicBool, Ordering};

pub use actions::{ActionContext, ActionRef, action_url};
pub use app::{
    ActionDef, ApiDef, App, AppBuilder, ErrorInfo, PageBody, PageDef, Rendering, Routes, SegmentDef, SlotDef,
    parse_pattern,
};
pub use context::{
    Auth, AuthUser, CsrfToken, Ctx, Data, Extension, FormState, FromContext, Headers, Nonce, Path, Query, QueryMap,
    RenderMode, RequestContext, RequestInfo, ResponseHeaders,
};
pub use cookies::{Cookie, Cookies, SameSite};
pub use embed::{Embedded, EmbeddedFile};
pub use error::{Error, ErrorKind, OrNotFound, Result, not_found, permanent_redirect, redirect};
pub use middleware::{
    BoxFuture, Cors, Middleware, Next, RateLimit, RequestId, cors, csrf, protected, rate_limit, request_id,
    security_headers,
};
pub use request::Request;
pub use response::{Body, Html, IntoResponse, Json, Response};
pub use runtime::{run, run_with};
pub use seo::{Robots, RobotsRule, Sitemap, SitemapEntry};
pub use session::{MemorySessionStore, Session, SessionStore, sessions};
pub use sse::SseEvent;
pub use testing::{TestClient, TestResponse};

pub use bytes;
pub use futures_util as futures;
pub use http;
pub use tokio;

static PAGE_STORE: std::sync::OnceLock<std::sync::Arc<dyn next_rust_cache::CacheStore>> = std::sync::OnceLock::new();

/// Re-render `path` on its next request (pages rendered statically) and
/// drop the matching data-cache entry. Usable from API routes and actions.
pub async fn revalidate_path(path: &str) {
    let path = if path.len() > 1 { path.trim_end_matches('/') } else { path };
    if let Some(store) = PAGE_STORE.get() {
        let _ = store.delete(&next_rust_cache::CacheKey::page(path)).await;
    }
    next_rust_cache::revalidate_path(path).await;
}

/// Invalidate every cached page and data entry tagged with `tag`.
pub async fn revalidate_tag(tag: &str) {
    if let Some(store) = PAGE_STORE.get() {
        let _ = store.delete_tag(tag).await;
    }
    next_rust_cache::revalidate_tag(tag).await;
}

/// Name of the CSRF double-submit cookie.
pub const CSRF_COOKIE: &str = "nr_csrf";

/// Per-request CSP nonce stored in request extensions.
#[derive(Debug, Clone)]
pub struct CspNonce(pub String);

static DEV: AtomicBool = AtomicBool::new(cfg!(debug_assertions));
static TRUST_PROXY: AtomicBool = AtomicBool::new(false);

/// Whether the process runs in development mode (detailed errors).
pub fn is_dev() -> bool {
    DEV.load(Ordering::Relaxed)
}

pub(crate) fn set_dev(v: bool) {
    DEV.store(v, Ordering::Relaxed);
}

pub(crate) fn trust_proxy() -> bool {
    TRUST_PROXY.load(Ordering::Relaxed)
}

pub(crate) fn set_trust_proxy(v: bool) {
    TRUST_PROXY.store(v, Ordering::Relaxed);
}

/// `n` random bytes from the operating system CSPRNG, hex encoded.
pub fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    if getrandom::fill(&mut buf).is_err() {
        // The OS RNG failing is unrecoverable for security-sensitive tokens.
        panic!("operating system random number generator unavailable");
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time byte comparison (for tokens).
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Items used by generated code and macros. Not a stable API.
#[doc(hidden)]
pub mod __private {
    pub use http::Method;
    pub use next_rust_router::Params;
    pub use next_rust_view::{Children, IntoViewResult, Metadata, Node, Slots, Stylesheet, View};

    pub use crate::actions::{ActionContext, ActionRef, run_action, run_action_ctx, run_action0};
    pub use crate::app::{
        ActionDef, ApiDef, ErrorInfo, HandlerFn, MiddlewareFn, PageBody, PageDef, Rendering, Routes, SegmentDef,
        SlotDef,
    };
    pub use crate::context::{Ctx, Data, FromContext};
    pub use crate::embed::{Embedded, EmbeddedFile};

    /// The TOML parser handed to development builds (see `Routes::toml`).
    pub const TOML: Option<next_rust_core::config::TomlParser> = Some(next_rust_core::Config::from_toml_str);
    pub use crate::error::{Error, IntoResult, Result};
    pub use crate::middleware::{BoxFuture, Next};
    pub use crate::render::is_bot;
    pub use crate::request::Request;
    pub use crate::response::{IntoResponse, Response};
    pub use crate::seo::{Robots, Sitemap};
    pub use crate::server::into_hyper;

    pub fn log_error(msg: &str) {
        crate::log::error(msg);
    }
}
