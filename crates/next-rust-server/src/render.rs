//! Page rendering: layout composition, boundaries, metadata, documents and
//! incremental static regeneration.
//!
//! For a page with segments `root → dashboard → settings` the rendered tree is
//!
//! ```text
//! RootLayout
//!   DashboardLayout
//!     [error boundary: dashboard/error.rs]
//!       [loading boundary: dashboard/loading.rs]   ← Suspense when streaming
//!         SettingsLayout
//!           SettingsPage
//! ```
//!
//! Boundaries of a segment catch errors thrown *below* its layout (including
//! the segment's own page), never errors of the layout itself, which bubble
//! to the parent segment. `not_found()` is caught by the nearest
//! `not-found.rs`, redirects are never caught.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use futures_util::future::join_all;
use next_rust_cache::{CacheEntry, CacheKey};
use next_rust_router::Params;
use next_rust_view::{
    Children, DocumentParts, Metadata, Node, RenderFlags, Slots, View, escape_attr, escape_text, raw_html,
    stream_document, suspense,
};

use crate::app::{AppInner, ErrorInfo, PageBody, PageDef, Rendering};
use crate::context::{Ctx, RequestContext};
use crate::error::{Error, ErrorKind, Result};
use crate::middleware::BoxFuture;
use crate::request::Request;
use crate::response::{Body, Response};

/// Placeholder nonce written into cached static HTML; replaced per request.
pub(crate) const NONCE_PLACEHOLDER: &str = "nr-nonce-placeholder-5f3a9c";

pub(crate) async fn page_response(inner: &Arc<AppInner>, index: usize, req: Request) -> Response {
    let def = &inner.routes.pages[index];
    let nonce = req.extension::<crate::CspNonce>().map(|n| n.0.clone()).unwrap_or_else(|| crate::random_hex(16));

    if let PageBody::Html(html) = def.body
        && is_full_document(html)
    {
        return full_html_page(inner, html, &nonce);
    }

    if def.rendering == Rendering::Static && !inner.env.is_dev() {
        return serve_static(inner, index, &req, &nonce).await;
    }

    let ctx = RequestContext::from_request(&req, inner.config.clone(), nonce, inner.env.is_dev());
    let streaming = inner.config.rendering.streaming && !is_bot(req.header("user-agent"));
    let mut res = render_page(inner, index, ctx, streaming).await;
    if !res.headers.contains_key("cache-control") {
        res.set_header("cache-control", "private, no-cache, no-store, max-age=0, must-revalidate");
    }
    res
}

fn is_full_document(html: &str) -> bool {
    let head: String = html.chars().take(512).collect::<String>().to_ascii_lowercase();
    head.contains("<!doctype") || head.contains("<html")
}

fn full_html_page(inner: &AppInner, html: &str, nonce: &str) -> Response {
    let mut body = html.to_owned();
    if inner.env.is_dev() {
        let script = crate::internal::dev_script(nonce);
        match body.to_ascii_lowercase().rfind("</body>") {
            Some(pos) => body.insert_str(pos, &script),
            None => body.push_str(&script),
        }
    }
    Response::html(body)
}

/// Heuristic bot detection: crawlers get fully rendered (non-streamed) HTML.
pub fn is_bot(user_agent: Option<&str>) -> bool {
    let Some(ua) = user_agent else { return false };
    let ua = ua.to_ascii_lowercase();
    ["bot", "crawler", "spider", "slurp", "facebookexternalhit", "embedly", "lighthouse", "preview"]
        .iter()
        .any(|needle| ua.contains(needle))
}

/// Render a page for `ctx` into a response.
pub(crate) async fn render_page(inner: &Arc<AppInner>, index: usize, ctx: Ctx, streaming: bool) -> Response {
    let def = &inner.routes.pages[index];

    let metadata = match collect_metadata(def, &ctx).await {
        Ok(m) => m,
        Err(e) if e.is_redirect() => return crate::response::IntoResponse::into_response(e),
        Err(e) if e.is_not_found() => return not_found_page(inner, &ctx).await,
        Err(e) if ctx.is_static() => return static_failure(e),
        Err(e) => {
            crate::log::error(&format!("metadata for {} failed: {}", def.pattern, e.detailed_message()));
            Metadata::default()
        }
    };

    let body = match render_level(inner.clone(), index, 0, ctx.clone(), streaming).await {
        Ok(node) => node,
        Err(e) if e.is_redirect() => return crate::response::IntoResponse::into_response(e),
        Err(e) if e.is_not_found() => return not_found_page(inner, &ctx).await,
        Err(e) if ctx.is_static() => return static_failure(e),
        Err(e) => return global_error_page(inner, &ctx, &e, def.source).await,
    };
    document_response(inner, &ctx, metadata, body, streaming)
}

fn static_failure(e: Error) -> Response {
    let mut r = Response::text(e.detailed_message()).with_status(500);
    r.set_header("x-nr-static-error", "1");
    r
}

async fn collect_metadata(def: &PageDef, ctx: &Ctx) -> Result<Metadata> {
    let fns: Vec<_> = def.segments.iter().filter_map(|s| s.metadata).chain(def.metadata).collect();
    let results = join_all(fns.into_iter().map(|f| f(ctx.clone()))).await;
    let mut merged = Metadata::default();
    for r in results {
        merged = merged.merge(r?);
    }
    Ok(merged)
}

fn page_body(def: &PageDef, ctx: Ctx) -> BoxFuture<Result<Node>> {
    match def.body {
        PageBody::Rust(f) => f(ctx),
        PageBody::Html(html) => Box::pin(async move { Ok(raw_html(html)) }),
    }
}

fn render_level(
    inner: Arc<AppInner>,
    index: usize,
    level: usize,
    ctx: Ctx,
    streaming: bool,
) -> BoxFuture<Result<Node>> {
    Box::pin(async move {
        let def = &inner.routes.pages[index];
        let seg = def.segments[level].clone();
        let last = level + 1 == def.segments.len();
        let below_fut = if last {
            page_body(def, ctx.clone())
        } else {
            render_level(inner.clone(), index, level + 1, ctx.clone(), streaming)
        };

        let below = match seg.loading {
            Some(loading) if streaming => {
                let fallback = loading(ctx.clone());
                let inner2 = inner.clone();
                let ctx2 = ctx.clone();
                let content = async move {
                    match below_fut.await {
                        Ok(node) => node,
                        Err(e) => streamed_error_node(&inner2, index, level, &ctx2, e).await,
                    }
                };
                Ok(suspense(fallback, content))
            }
            _ => below_fut.await,
        };

        let below = apply_boundaries(&seg, &ctx, below).await;
        let mut children = below?;
        if let Some(template) = seg.template {
            children = template(ctx.clone(), Children(children), Slots::new()).await?;
        }
        match seg.layout {
            Some(layout) => {
                let slots = render_slots(&seg, &ctx).await?;
                layout(ctx.clone(), Children(children), slots).await
            }
            None => Ok(children),
        }
    })
}

async fn apply_boundaries(seg: &crate::app::SegmentDef, ctx: &Ctx, below: Result<Node>) -> Result<Node> {
    match below {
        Err(e) if e.is_not_found() => match seg.not_found {
            Some(nf) => {
                ctx.raise_status(404);
                nf(ctx.clone()).await
            }
            None => Err(e),
        },
        // Static renders surface errors to the build instead of rendering an error UI.
        Err(e) if !e.is_redirect() && !ctx.is_static() => match seg.error {
            Some(boundary) => {
                ctx.raise_status(e.status().as_u16());
                Ok(boundary(ctx.clone(), ErrorInfo::from_error(&e, ctx.dev)))
            }
            None => Err(e),
        },
        other => other,
    }
}

async fn render_slots(seg: &crate::app::SegmentDef, ctx: &Ctx) -> Result<Slots> {
    let mut slots = Slots::new();
    for slot in &seg.slots {
        if let Some(render) = slot.render {
            match render(ctx.clone()).await {
                Ok(node) => slots.insert(slot.name, node),
                Err(e) if e.is_redirect() || e.is_not_found() => return Err(e),
                Err(e) => {
                    let info = ErrorInfo::from_error(&e, ctx.dev);
                    crate::log::error(&format!("slot @{} failed [{}]", slot.name, info.digest));
                }
            }
        }
    }
    Ok(slots)
}

/// An error surfaced inside streamed content, after the response head was
/// sent: render the nearest boundary in place.
async fn streamed_error_node(inner: &Arc<AppInner>, index: usize, level: usize, ctx: &Ctx, e: Error) -> Node {
    if let ErrorKind::Redirect { location, .. } = e.kind() {
        let url = escape_attr(location);
        let js = serde_json::to_string(location).unwrap_or_default().replace("</", "<\\/");
        return raw_html(format!(
            "<script nonce=\"{}\">location.replace({js})</script><noscript><meta http-equiv=\"refresh\" content=\"0;url={url}\"></noscript>",
            escape_attr(&ctx.nonce)
        ));
    }
    let def = &inner.routes.pages[index];
    for seg in def.segments[..=level].iter().rev() {
        if e.is_not_found() {
            if let Some(nf) = seg.not_found
                && let Ok(node) = nf(ctx.clone()).await
            {
                return node;
            }
        } else if let Some(boundary) = seg.error {
            return boundary(ctx.clone(), ErrorInfo::from_error(&e, ctx.dev));
        }
    }
    let info = ErrorInfo::from_error(&e, ctx.dev);
    default_error_node(&info, ctx.dev)
}

fn default_error_node(info: &ErrorInfo, dev: bool) -> Node {
    next_rust_view::Element::new("div")
        .with(next_rust_view::attrs::role("alert"))
        .with(next_rust_view::Element::new("p").child(format!("Something went wrong ({}).", info.digest)))
        .with(dev.then(|| next_rust_view::Element::new("pre").child(info.message.clone())))
        .into_node()
}

pub(crate) fn document_response(
    inner: &Arc<AppInner>,
    ctx: &Ctx,
    metadata: Metadata,
    body: Node,
    streaming: bool,
) -> Response {
    let nonce = ctx.nonce.clone();
    let mut head_extra = String::new();
    if inner.env.is_dev() && !ctx.is_static() {
        head_extra.push_str(&crate::internal::dev_script(&nonce));
    }
    for p in &inner.plugins {
        if let Some(h) = p.head() {
            head_extra.push_str(&h);
        }
    }
    let client_nav = inner.config.rendering.client_navigation;
    let env_json = inner.public_env_json.clone();
    let tail_nonce = nonce.clone();
    let parts = DocumentParts {
        lang: inner.config.app.lang.clone(),
        head: metadata.render_head(),
        head_extra,
        body,
        nonce: Some(nonce),
        tail: Box::new(move |flags: RenderFlags| {
            let mut t = String::new();
            if flags.islands
                && let Some(json) = env_json
            {
                t.push_str(&format!(
                    "<script id=\"__nr_env\" type=\"application/json\">{}</script>",
                    json.replace("</", "<\\/")
                ));
            }
            if flags.islands || (flags.links && client_nav) {
                t.push_str(&format!(
                    "<script type=\"module\" src=\"/_nr/runtime.js?v={}\" nonce=\"{}\"></script>",
                    crate::internal::runtime_version(),
                    escape_attr(&tail_nonce)
                ));
            }
            t
        }),
    };
    let stream =
        stream_document(parts, streaming).map(|chunk| Ok::<Bytes, crate::request::BoxError>(Bytes::from(chunk)));
    let mut res = Response::new(http::StatusCode::OK, Body::Stream(Box::pin(stream)));
    res.status = http::StatusCode::from_u16(ctx.status()).unwrap_or(http::StatusCode::OK);
    res.set_header("content-type", "text/html; charset=utf-8");
    let extra = ctx.response_headers.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for (k, v) in extra.iter() {
        res.headers.insert(k, v.clone());
    }
    res
}

async fn root_wrapped(inner: &Arc<AppInner>, ctx: &Ctx, content: Node) -> Node {
    match inner.routes.root.layout {
        Some(layout) => match layout(ctx.clone(), Children(content), Slots::new()).await {
            Ok(node) => node,
            Err(e) => {
                let info = ErrorInfo::from_error(&e, ctx.dev);
                default_error_node(&info, ctx.dev)
            }
        },
        None => content,
    }
}

async fn root_metadata(inner: &Arc<AppInner>, ctx: &Ctx) -> Metadata {
    match inner.routes.root.metadata {
        Some(f) => f(ctx.clone()).await.unwrap_or_default(),
        None => Metadata::default(),
    }
}

/// The 404 page for `ctx` (root `not-found.rs` inside the root layout).
pub(crate) async fn not_found_page(inner: &Arc<AppInner>, ctx: &Ctx) -> Response {
    ctx.set_status(404);
    let content = match inner.routes.root.not_found {
        Some(f) => f(ctx.clone()).await.unwrap_or_else(|_| default_not_found()),
        None => default_not_found(),
    };
    let body = root_wrapped(inner, ctx, content).await;
    let metadata =
        root_metadata(inner, ctx).await.merge(Metadata::new().absolute_title("404: Not Found").robots("noindex"));
    let mut res = document_response(inner, ctx, metadata, body, false);
    res.set_header("cache-control", "private, no-cache, no-store, max-age=0, must-revalidate");
    res
}

fn default_not_found() -> Node {
    next_rust_view::Element::new("main")
        .with(next_rust_view::Element::new("h1").child("404"))
        .with(next_rust_view::Element::new("p").child("This page could not be found."))
        .into_node()
}

/// Response for URLs matching no route.
pub(crate) async fn not_found_response(inner: &Arc<AppInner>, req: &Request) -> Response {
    let accepts_html = req.header("accept").is_none_or(|a| a.contains("text/html") || a.contains("*/*"));
    if !accepts_html || req.path().starts_with(&inner.config.api.prefix) {
        return Response::not_found();
    }
    let nonce = req.extension::<crate::CspNonce>().map(|n| n.0.clone()).unwrap_or_default();
    let ctx = RequestContext::from_request(req, inner.config.clone(), nonce, inner.env.is_dev());
    not_found_page(inner, &ctx).await
}

async fn global_error_page(inner: &Arc<AppInner>, ctx: &Ctx, e: &Error, source: &str) -> Response {
    let info = ErrorInfo::from_error(e, ctx.dev);
    ctx.set_status(info.status.max(500));
    let body = match inner.routes.global_error {
        Some(f) => f(ctx.clone(), info),
        None => {
            let mut html = String::from(
                "<main style=\"font-family:system-ui,sans-serif;max-width:48rem;margin:4rem auto;padding:0 1rem\">",
            );
            if ctx.dev {
                html.push_str(&format!(
                    "<h1 style=\"color:#b91c1c\">Unhandled error</h1><p>in <code>{}</code></p><pre style=\"white-space:pre-wrap;background:#fef2f2;padding:1rem;border-radius:.5rem\">{}</pre><p style=\"color:#6b7280\">Add an <code>error.rs</code> next to the page to handle this error. Digest: {}</p>",
                    escape_text(source),
                    escape_text(&info.message),
                    info.digest
                ));
            } else {
                html.push_str(&format!("<h1>Something went wrong</h1><p>Error reference: {}</p>", info.digest));
            }
            html.push_str("</main>");
            raw_html(html)
        }
    };
    let metadata = Metadata::new().title("Error").robots("noindex");
    document_response(inner, ctx, metadata, body, false)
}

// ---------------------------------------------------------------------------
// Static rendering and ISR
// ---------------------------------------------------------------------------

/// Render a static page to HTML. `Err` carries a response to send instead.
#[allow(clippy::result_large_err)]
pub(crate) async fn render_static_html(
    inner: &Arc<AppInner>,
    index: usize,
    path: &str,
    params: Params,
) -> std::result::Result<(String, u16), Response> {
    let mut ctx_inner = RequestContext::for_static_with(path, params, inner.config.clone(), inner.env.is_dev());
    if let Some(ctx) = Arc::get_mut(&mut ctx_inner) {
        ctx.nonce = NONCE_PLACEHOLDER.to_owned();
    }
    let res = render_page(inner, index, ctx_inner, false).await;
    if res.headers.contains_key("x-nr-static-error") || res.status.is_redirection() {
        return Err(res);
    }
    let status = res.status.as_u16();
    Ok((res.into_text().await, status))
}

async fn static_params(inner: &Arc<AppInner>, index: usize) -> Option<Arc<Vec<Params>>> {
    let f = inner.routes.pages[index].generate_params?;
    if let Some(p) = inner.static_params.lock().unwrap_or_else(|e| e.into_inner()).get(&index) {
        return Some(p.clone());
    }
    let list = match f().await {
        Ok(list) => Arc::new(list),
        Err(e) => {
            crate::log::error(&format!("generate_params failed: {}", e.detailed_message()));
            return None;
        }
    };
    inner.static_params.lock().unwrap_or_else(|e| e.into_inner()).insert(index, list.clone());
    Some(list)
}

fn cache_path(path: &str) -> String {
    if path.len() > 1 { path.trim_end_matches('/').to_owned() } else { path.to_owned() }
}

async fn serve_static(inner: &Arc<AppInner>, index: usize, req: &Request, nonce: &str) -> Response {
    let def = &inner.routes.pages[index];
    let path = cache_path(req.path());
    let key = CacheKey::page(&path);
    let revalidate = def.revalidate.or(inner.config.rendering.revalidate);

    if let Ok(Some(entry)) = inner.page_store.get(&key).await {
        let stale = entry.is_stale();
        if stale {
            spawn_regeneration(inner.clone(), index, path.clone(), req.params().clone());
        }
        return cached_html(&entry.value, nonce, if stale { "STALE" } else { "HIT" }, revalidate);
    }

    if !def.dynamic_params
        && next_rust_router::RoutePattern::is_dynamic(&crate::app::parse_pattern(def.pattern).unwrap_or_default())
    {
        let allowed = static_params(inner, index).await;
        let ok = allowed.is_some_and(|list| list.iter().any(|p| params_equal(p, req.params())));
        if !ok {
            let ctx = RequestContext::from_request(req, inner.config.clone(), nonce.to_owned(), inner.env.is_dev());
            return not_found_page(inner, &ctx).await;
        }
    }

    match render_static_html(inner, index, &path, req.params().clone()).await {
        Ok((html, 200)) => {
            let entry = CacheEntry::new(html.clone().into_bytes())
                .revalidate(revalidate.map(Duration::from_secs))
                .tags(def.tags.iter().copied());
            if let Err(e) = inner.page_store.set(key, entry).await {
                crate::log::warn(&format!("could not cache {path}: {e}"));
            }
            cached_html(html.as_bytes(), nonce, "MISS", revalidate)
        }
        Ok((html, status)) => cached_html(html.as_bytes(), nonce, "BYPASS", None)
            .with_status(status)
            .with_cache_control("private, no-cache, no-store, max-age=0, must-revalidate"),
        Err(res) => {
            if res.headers.contains_key("x-nr-static-error") {
                crate::log::error(&format!("static render of {path} failed"));
                return Response::text("Internal Server Error").with_status(500);
            }
            res
        }
    }
}

fn params_equal(a: &Params, b: &Params) -> bool {
    a.len() == b.len() && a.iter().all(|(k, v)| b.value(k) == Some(v))
}

fn cached_html(bytes: &[u8], nonce: &str, cache: &str, revalidate: Option<u64>) -> Response {
    let text = String::from_utf8_lossy(bytes);
    let html =
        if text.contains(NONCE_PLACEHOLDER) { text.replace(NONCE_PLACEHOLDER, nonce) } else { text.into_owned() };
    let cc = match revalidate {
        Some(secs) => format!("public, max-age=0, s-maxage={secs}, stale-while-revalidate"),
        None => "public, max-age=0, must-revalidate".to_owned(),
    };
    Response::html(html).with_header("x-nr-cache", cache).with_cache_control(&cc)
}

fn spawn_regeneration(inner: Arc<AppInner>, index: usize, path: String, params: Params) {
    {
        let mut set = inner.revalidating.lock().unwrap_or_else(|e| e.into_inner());
        if !set.insert(path.clone()) {
            return;
        }
    }
    tokio::spawn(async move {
        let def = &inner.routes.pages[index];
        let revalidate = def.revalidate.or(inner.config.rendering.revalidate);
        match render_static_html(&inner, index, &path, params).await {
            Ok((html, 200)) => {
                let entry = CacheEntry::new(html.into_bytes())
                    .revalidate(revalidate.map(Duration::from_secs))
                    .tags(def.tags.iter().copied());
                let _ = inner.page_store.set(CacheKey::page(&path), entry).await;
                crate::log::debug(&format!("revalidated {path}"));
            }
            _ => crate::log::warn(&format!("revalidation of {path} failed; serving stale content")),
        }
        inner.revalidating.lock().unwrap_or_else(|e| e.into_inner()).remove(&path);
    });
}
