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
//!
//! ## Partial navigation
//!
//! The children of every reusable layout are wrapped in comment markers,
//! `<!--nr-l:KEY-->…<!--/nr-l:KEY-->`. On a client-side navigation the
//! browser sends the keys it shows in `x-nr-layouts`. When the new page shares
//! layouts with it, only what is inside the deepest shared layout is rendered
//! and sent (`x-nr-partial: KEY`); the shared layouts are not run again and
//! stay on screen in the browser. Static pages are cut out of their cached
//! HTML instead of being rendered.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::future::join_all;
use futures_util::{FutureExt, StreamExt};
use next_rust_cache::{CacheEntry, CacheKey};
use next_rust_router::Params;
use next_rust_view::{
    Children, DocumentParts, HoleContext, Metadata, Node, RenderFlags, Resume, Shell, Slots, View, escape_attr,
    escape_text, raw_html, render_shell, resume_document, stream_document, suspense,
};

use crate::app::{AppInner, ErrorInfo, PageBody, PageDef, SegmentDef};
use crate::context::{Ctx, RequestContext};
use crate::error::{Error, ErrorKind, Result};
use crate::middleware::BoxFuture;
use crate::partial::{FragmentUse, HoleCx, SHELL_META, track_fragments};
use crate::request::Request;
use crate::response::{Body, Response};

/// Placeholder nonce written into cached static HTML; replaced per request
/// wherever it appears. It is random per process, so page content cannot
/// contain it: a fixed string would be substituted inside ordinary text too,
/// handing the request's live CSP nonce to whatever wrote that text (a CMS
/// field, a route parameter, a file name).
pub(crate) fn nonce_placeholder() -> &'static str {
    static PLACEHOLDER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PLACEHOLDER.get_or_init(|| format!("nr-nonce-placeholder-{}", crate::random_hex(16)))
}

pub(crate) async fn page_response(inner: &Arc<AppInner>, index: usize, req: Request) -> Response {
    let def = &inner.routes.pages[index];
    let nonce = req.extension::<crate::CspNonce>().map(|n| n.0.clone()).unwrap_or_else(|| crate::random_hex(16));

    // Analytics start and stop with a document: see `crate::tracking`.
    if inner.tracking.crosses_boundary(&req) {
        return crate::tracking::full_load();
    }

    if let PageBody::Html(html) = def.body
        && is_full_document(html)
    {
        return full_html_page(inner, html, &req, &nonce);
    }

    let partial = partial_target(inner, def, &req);

    // An intercepting route shares its URL with the page it stands in for,
    // and the page cache is keyed by URL: cached, either one would be served
    // for the other, and any visitor could choose which (`x-nr-from`).
    if def.rendering.is_prerendered() && def.intercept_from.is_none() && !inner.env.is_dev() {
        return serve_static(inner, index, &req, &nonce, partial).await;
    }

    let ctx = RequestContext::from_request(&req, inner.config.clone(), nonce, inner.env.is_dev());
    let streaming = inner.config.rendering.streaming && !is_bot(req.header("user-agent"));
    let mut res = match partial {
        Some(target) => render_partial(inner, index, ctx, streaming, target).await,
        None => render_page(inner, index, ctx, streaming).await,
    };
    if !res.headers.contains_key("cache-control") {
        res.set_header("cache-control", "private, no-cache, no-store, max-age=0, must-revalidate");
    }
    res
}

fn is_full_document(html: &str) -> bool {
    let head: String = html.chars().take(512).collect::<String>().to_ascii_lowercase();
    head.contains("<!doctype") || head.contains("<html")
}

fn full_html_page(inner: &AppInner, html: &str, req: &Request, nonce: &str) -> Response {
    let mut body = html.to_owned();
    // A document of its own still gets `[verification]` and `[analytics]`.
    let tracking = &inner.tracking;
    let decision = if crate::tracking::is_navigation(req) {
        crate::tracking::Decision::Nothing
    } else {
        tracking.decide(req.path(), None)
    };
    let head = format!("{}{}", tracking.verification_head(), tracking.head(decision, nonce));
    if !head.is_empty()
        && let Some(pos) = body.to_ascii_lowercase().find("</head>")
    {
        body.insert_str(pos, &head);
        let after_body = tracking.body_start(decision);
        let lower = body.to_ascii_lowercase();
        if !after_body.is_empty()
            && let Some(open) = lower.find("<body")
            && let Some(end) = lower[open..].find('>')
        {
            body.insert_str(open + end + 1, after_body);
        }
    }
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

// ---------------------------------------------------------------------------
// Partial navigation
// ---------------------------------------------------------------------------

const MARKER_OPEN: &str = "<!--nr-l:";
const MARKER_CLOSE: &str = "<!--/nr-l:";

/// The key of a reusable layout for these parameters, or `None` when the
/// layout must always be rendered.
pub(crate) fn layout_key(inner: &AppInner, seg: &SegmentDef, params: &Params) -> Option<String> {
    if seg.layout.is_none() || !seg.layout_reusable || seg.layout_id.is_empty() {
        return None;
    }
    let mut h = next_rust_assets::Fnv64::new();
    h.write(inner.routes.build_id.as_bytes());
    h.write(b"\0");
    h.write(seg.layout_id.as_bytes());
    if seg.layout_uses_params {
        let mut values: Vec<(&str, String)> = params.iter().map(|(k, v)| (k, format!("{v:?}"))).collect();
        values.sort();
        for (k, v) in values {
            h.write(b"\0");
            h.write(k.as_bytes());
            h.write(b"=");
            h.write(v.as_bytes());
        }
    }
    Some(format!("{:016x}", h.finish()))
}

/// The layout level to render below, and its key, when the browser already
/// shows every layout above it.
fn partial_target(inner: &AppInner, def: &PageDef, req: &Request) -> Option<(usize, String)> {
    if req.header("x-nr-nav") != Some("1") || !inner.config.rendering.client_navigation {
        return None;
    }
    if let PageBody::Html(html) = def.body
        && is_full_document(html)
    {
        return None;
    }
    let shown: std::collections::HashSet<&str> =
        req.header("x-nr-layouts")?.split(',').map(str::trim).filter(|k| !k.is_empty()).take(64).collect();
    let mut target = None;
    for (level, seg) in def.segments.iter().enumerate() {
        // Parallel slots are rendered by the layout itself and change per URL.
        if !seg.slots.is_empty() {
            break;
        }
        if seg.layout.is_some() {
            // Stop at the first layout that must be rendered again.
            let Some(key) = layout_key(inner, seg, req.params()).filter(|k| shown.contains(k.as_str())) else {
                break;
            };
            target = Some((level, key));
        }
        // A template renders again on every navigation, so nothing below it
        // can be reused.
        if seg.template.is_some() {
            break;
        }
    }
    target
}

/// Render only what is inside the layout at `level`. Anything unusual (an
/// error, `not_found()`) falls back to the full page.
async fn render_partial(
    inner: &Arc<AppInner>,
    index: usize,
    ctx: Ctx,
    streaming: bool,
    (level, key): (usize, String),
) -> Response {
    let def = &inner.routes.pages[index];
    let metadata = match collect_metadata(def, &ctx).await {
        Ok(m) => m,
        Err(_) => return render_page(inner, index, ctx, streaming).await,
    };
    match render_level(inner.clone(), index, level, ctx.clone(), streaming, false).await {
        Ok(region) if ctx.status() == 200 => {
            let known = known_styles(&ctx);
            let mut res =
                document_response_with(inner, &ctx, Some(index), partial_metadata(metadata), region, streaming, known);
            mark_partial(&mut res, &key);
            res
        }
        Err(e) if e.is_redirect() => crate::response::IntoResponse::into_response(e),
        _ => {
            ctx.set_status(200);
            render_page(inner, index, ctx, streaming).await
        }
    }
}

/// Metadata for a partial page. Icons are left out: the browser loaded them
/// with the first page, and a client navigation never changes them.
fn partial_metadata(mut metadata: Metadata) -> Metadata {
    metadata.icons = None;
    metadata
}

/// Remove `<link rel="icon">` and `<link rel="apple-touch-icon">` tags from
/// cached head markup (the static-page equivalent of [`partial_metadata`]).
fn strip_icon_links(head: &mut String) {
    for rel in ["<link rel=\"icon\"", "<link rel=\"apple-touch-icon\""] {
        while let Some(at) = head.find(rel) {
            let Some(len) = head[at..].find('>') else { break };
            head.replace_range(at..at + len + 1, "");
        }
    }
}

fn mark_partial(res: &mut Response, key: &str) {
    res.set_header("x-nr-partial", key);
    // Depends on what this browser already shows: never store it in a shared cache.
    res.set_header("cache-control", "private, no-cache, no-store, max-age=0, must-revalidate");
    res.set_header("vary", "x-nr-layouts");
}

/// Cut the region inside layout `key` out of a complete HTML document,
/// leaving out icons and the stylesheets the browser already has.
///
/// `complete`: `html` is a whole document and so is the result. Otherwise it
/// is a shell, and the result ends where the holes are streamed.
fn slice_partial(html: &str, key: &str, known_styles: &[&str], complete: bool) -> Option<String> {
    let open = format!("{MARKER_OPEN}{key}-->");
    let close = format!("{MARKER_CLOSE}{key}-->");
    let start = html.find(&open)? + open.len();
    let end = start + html[start..].find(&close)?;
    let head_start = html.find("<head>")? + "<head>".len();
    let head_end = head_start + html[head_start..].find("</head>")?;
    let env = html
        .find("<script id=\"__next_rust_env\"")
        .and_then(|at| html[at..].find("</script>").map(|len| &html[at..at + len + "</script>".len()]))
        .unwrap_or("");
    let mut head = html[head_start..head_end].to_owned();
    strip_icon_links(&mut head);
    for id in known_styles {
        let open = format!("<style data-nr-css=\"{id}\">");
        if let Some(at) = head.find(&open)
            && let Some(len) = head[at..].find("</style>")
        {
            head.replace_range(at..at + len + "</style>".len(), "");
        }
    }
    let region = &html[start..end];
    let scripts = scripts_outside(html, start..end);
    Some(if complete {
        format!("<!DOCTYPE html><html><head>{head}</head><body>{region}{scripts}{env}</body></html>")
    } else {
        format!("<!DOCTYPE html><html><head>{head}</head><body>{region}{scripts}")
    })
}

/// The `<script>` tags of `<body>` that are not inside `region`.
///
/// A page's scripts sit at the end of `<body>`, outside every layout, so
/// slicing a layout's region out of a cached page leaves them behind. That is
/// only visible in production: a dynamically rendered partial is built from
/// the page's own document and carries them, a pre-rendered one is cut out of
/// stored HTML and did not. Navigating from a page that needs no scripts to
/// one that does then delivered the new page complete, correct and inert —
/// every component rendered and nothing responding to a click.
///
/// The browser drops what it has already run: both runtimes skip the runtime
/// shim, and an inline script is matched by its `data-nr-js` id. The
/// environment is carried separately, so it is left out here rather than sent
/// twice.
fn scripts_outside(html: &str, region: std::ops::Range<usize>) -> String {
    const CLOSE: &str = "</script>";
    let Some(body) = html.find("<body") else { return String::new() };
    let mut out = String::new();
    let mut at = body;
    while let Some(rel) = html[at..].find("<script") {
        let open = at + rel;
        let Some(len) = html[open..].find(CLOSE) else { break };
        let close = open + len + CLOSE.len();
        at = close;
        if region.contains(&open) || html[open..close].contains("__next_rust_env") {
            continue;
        }
        out.push_str(&html[open..close]);
    }
    out
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

    let body = match render_level(inner.clone(), index, 0, ctx.clone(), streaming, true).await {
        Ok(node) => node,
        Err(e) if e.is_redirect() => return crate::response::IntoResponse::into_response(e),
        Err(e) if e.is_not_found() => return not_found_page(inner, &ctx).await,
        Err(e) if ctx.is_static() => return static_failure(e),
        Err(e) => return global_error_page(inner, &ctx, &e, def.source).await,
    };
    document_response(inner, &ctx, Some(index), metadata, body, streaming)
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

/// Render the segment at `level` and everything below it. With
/// `with_layout = false` the segment's own layout is skipped, which yields the
/// region a partial navigation replaces.
fn render_level(
    inner: Arc<AppInner>,
    index: usize,
    level: usize,
    ctx: Ctx,
    streaming: bool,
    with_layout: bool,
) -> BoxFuture<Result<Node>> {
    Box::pin(async move {
        let def = &inner.routes.pages[index];
        let seg = def.segments[level].clone();
        let last = level + 1 == def.segments.len();
        let below_fut = if last {
            page_body(def, ctx.clone())
        } else {
            render_level(inner.clone(), index, level + 1, ctx.clone(), streaming, true)
        };

        let below = match seg.loading {
            Some(loading) if streaming => {
                let fallback = loading(ctx.clone());
                let inner2 = inner.clone();
                let ctx2 = ctx.clone();
                let content = async move {
                    match below_fut.await {
                        Ok(node) => node,
                        Err(e) => streamed_error_node(&inner2, Some((index, level)), &ctx2, e).await,
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
            Some(layout) if with_layout => {
                if let Some(key) = layout_key(&inner, &seg, &ctx.params) {
                    children = next_rust_view::fragment![
                        raw_html(format!("{MARKER_OPEN}{key}-->")),
                        children,
                        raw_html(format!("{MARKER_CLOSE}{key}-->"))
                    ];
                }
                let slots = render_slots(&seg, &ctx).await?;
                layout(ctx.clone(), Children(children), slots).await
            }
            _ => Ok(children),
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
/// sent: render the nearest boundary in place, looking up from segment
/// `level` of page `index`.
async fn streamed_error_node(inner: &Arc<AppInner>, at: Option<(usize, usize)>, ctx: &Ctx, e: Error) -> Node {
    if let ErrorKind::Redirect { location, .. } = e.kind() {
        // Run by the browser as script and as a meta refresh: never a script URL.
        let location = crate::response::safe_location(location);
        let url = escape_attr(location);
        let js = serde_json::to_string(location).unwrap_or_default().replace('<', "\\u003c");
        return raw_html(format!(
            "<script nonce=\"{}\">location.replace({js})</script><noscript><meta http-equiv=\"refresh\" content=\"0;url={url}\"></noscript>",
            escape_attr(&ctx.nonce)
        ));
    }
    let segments = at.map_or(&[][..], |(index, level)| &inner.routes.pages[index].segments[..=level]);
    for seg in segments.iter().rev() {
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

/// `index`: the page being rendered, whose boundaries catch errors of holes.
pub(crate) fn document_response(
    inner: &Arc<AppInner>,
    ctx: &Ctx,
    index: Option<usize>,
    metadata: Metadata,
    body: Node,
    streaming: bool,
) -> Response {
    // Client navigations keep the styles the page already has.
    let known = if ctx.headers.get("x-nr-nav").is_some_and(|v| v == "1") { known_styles(ctx) } else { Vec::new() };
    document_response_with(inner, ctx, index, metadata, body, streaming, known)
}

/// What `dynamic` holes of page `index` are rendered with for this request.
fn hole_context(inner: &Arc<AppInner>, ctx: &Ctx, index: Option<usize>) -> HoleContext {
    let (inner, error_ctx) = (inner.clone(), ctx.clone());
    Arc::new(HoleCx {
        ctx: ctx.clone(),
        on_error: Box::new(move |e| {
            let (inner, ctx) = (inner.clone(), error_ctx.clone());
            Box::pin(async move {
                let at = index.map(|i| (i, inner.routes.pages[i].segments.len().saturating_sub(1)));
                streamed_error_node(&inner, at, &ctx, e).await
            })
        }),
    })
}

/// Markup before `</body>`: the public environment and the client scripts
/// the rendered features need.
fn document_tail(inner: &AppInner, nonce: &str) -> Box<dyn FnOnce(RenderFlags) -> String + Send> {
    let client_nav = inner.config.rendering.client_navigation;
    let enhance_forms = inner.config.client.enhance_forms;
    let dev = inner.env.is_dev();
    let wasm_runtime = inner.config.client.runtime.is_wasm();
    let wasm_ui = inner.config.ui.script.is_wasm();
    // Run the island module's `start()` on every page, not only on pages that
    // happen to render an island or a UI component.
    //
    // The loader instantiates lazily, on first hydration, so an app whose
    // browser logic lives in `start()` — restoring a colour mode, a resend
    // countdown, filling `input[data-timezone]` before sign-up posts it —
    // simply never ran it on an island-less page, and had to keep a
    // hand-written `<script>` in the root layout to cover those pages. That
    // script is exactly what `[ui] script = "wasm"` exists to remove.
    //
    // It costs no extra fetch: it boots the module this page's own islands
    // would have booted anyway, and on a page with none it is the same module
    // every other page already cached. `boot()` memoises its promise, so a
    // page that also hydrates an island still instantiates once, and calling
    // `start()` before any hydration is the order the loader already uses.
    //
    // Gated on `wasm_ui` because `/_next-rust/ui.js` only answers with the
    // boot module ([`crate::internal::UI_BOOT_JS`]) when the UI script is
    // wasm. With the JavaScript component script it would put the components'
    // behaviour on every page that renders no components.
    let wasm_start = wasm_ui && inner.config.build.wasm_islands;
    let client_version = crate::internal::client_version(inner, dev);
    let wasm_preload = inner.config.uses_wasm().then(|| crate::internal::wasm_module_url(inner)).flatten();
    let env_json = inner.public_env_json.clone();
    let nonce = nonce.to_owned();
    Box::new(move |flags: RenderFlags| {
        let mut t = String::new();
        if flags.islands
            && let Some(json) = env_json
        {
            t.push_str(&format!(
                "<script id=\"__next_rust_env\" type=\"application/json\">{}</script>",
                // `<` never appears raw: no `</script>`, no `<!--` state change.
                json.replace('<', "\\u003c")
            ));
        }
        // UI components call actions and navigate through the runtime.
        if flags.islands || flags.ui || ((flags.links || (flags.forms && enhance_forms)) && client_nav) {
            // `[client] enhance_forms = false`: the runtime leaves plain forms to the browser.
            if !enhance_forms {
                t.push_str("<script id=\"__next_rust_forms\" type=\"application/json\">false</script>");
            }
            // Start the module download now rather than when the shim runs.
            // The shim is `type="module"`, so it is deferred: without this the
            // fetch cannot begin until the document has finished parsing and
            // the shim itself has loaded and executed. The link is parsed
            // where it stands, so the two overlap instead of queueing.
            //
            // `crossorigin` is required even though this is same-origin:
            // `as="fetch"` preloads are keyed by CORS mode, and the shim's own
            // `fetch` is a CORS-mode request. Without the attribute the
            // preload lands in a different cache entry and the module is
            // downloaded twice — which is worse than not preloading at all.
            if let Some(url) = &wasm_preload {
                t.push_str(&format!("<link rel=\"preload\" as=\"fetch\" crossorigin href=\"{}\">", escape_attr(url)));
            }
            // `data-nr-shim` marks the one script a page is allowed to carry
            // when `[client] runtime = "wasm"`: it loads the module and
            // forwards its imports, and decides nothing. Tools that audit a
            // page for remaining JavaScript key off it.
            t.push_str(&format!(
                "<script type=\"module\"{} src=\"/_next-rust/runtime.js?v={}\" nonce=\"{}\"></script>",
                if wasm_runtime { " data-nr-shim" } else { "" },
                client_version,
                escape_attr(&nonce)
            ));
        }
        if flags.ui || wasm_start {
            // With `[ui] script = "wasm"` this is the boot module, not the
            // components' behaviour, so it carries the same `data-nr-shim`
            // mark the runtime's shim does.
            t.push_str(&format!(
                "<script type=\"module\"{} src=\"/_next-rust/ui.js?v={}\" nonce=\"{}\"></script>",
                if wasm_ui { " data-nr-shim" } else { "" },
                crate::internal::ui_version(dev, wasm_ui),
                escape_attr(&nonce)
            ));
        }
        t
    })
}

/// Point icons from `public/` at a versioned URL (`/logo.svg?v=<hash>`) that
/// browsers cache for good. Browsers re-check the favicon on their own (for
/// example when the URL changes); with a versioned URL that check is answered
/// from the browser cache instead of the server.
fn version_icons(inner: &AppInner, mut metadata: Metadata) -> Metadata {
    if let Some(icons) = &mut metadata.icons {
        for icon in icons {
            if let Some(v) = crate::static_files::public_version(inner, &icon.href) {
                icon.href = format!("{}?v={v}", icon.href);
            }
        }
    }
    metadata
}

/// Stylesheet ids the browser reports having (`x-nr-styles`).
fn known_styles(ctx: &Ctx) -> Vec<String> {
    ctx.headers
        .get("x-nr-styles")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).take(256).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn document_response_with(
    inner: &Arc<AppInner>,
    ctx: &Ctx,
    index: Option<usize>,
    metadata: Metadata,
    mut body: Node,
    streaming: bool,
    known_styles: Vec<String>,
) -> Response {
    // `[analytics]`: written into full documents only. A client navigation
    // keeps the scripts of the document it started in, and one that would
    // end on a page that opted out is loaded in full instead.
    let navigation = ctx.headers.get("x-nr-nav").is_some_and(|v| v == "1");
    let analytics = inner.tracking.decide(ctx.path(), Some(&metadata));
    if navigation && analytics == crate::tracking::Decision::OptOut {
        return crate::tracking::full_load();
    }
    let analytics = if navigation { crate::tracking::Decision::Nothing } else { analytics };
    next_rust_view::attrs::mark_active_links(&mut body, ctx.path());
    // App-wide stylesheets (Tailwind) come first, so page and module styles
    // can override them. Like any stylesheet they are hoisted into <head>
    // once, and not sent again on partial navigations.
    if !inner.routes.stylesheets.is_empty() {
        let mut nodes: Vec<Node> = inner.routes.stylesheets.iter().map(|s| Node::Style(s)).collect();
        nodes.push(body);
        body = Node::Fragment(nodes);
    }
    let metadata = version_icons(inner, metadata);
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
    head_extra.push_str(&inner.tracking.head(analytics, &nonce));
    // The motion stylesheet is the first stylesheet of the document, right
    // after the metadata (charset, viewport, title): its cascade layer must
    // be declared before Tailwind's so that everything the app styles wins.
    let mut head = metadata.render_head();
    // `[verification]`: real <meta> tags, in every document.
    head.push_str(inner.tracking.verification_head());
    for (id, css) in [&inner.motion, &inner.loader].into_iter().flatten() {
        if !known_styles.iter().any(|k| k == id) {
            head.push_str(&format!("<style data-nr-css=\"{id}\">{css}</style>"));
        }
    }
    let parts = DocumentParts {
        lang: inner.config.app.lang.clone(),
        html_attributes: metadata.html_attributes.clone(),
        head,
        head_extra,
        body_start: inner.tracking.body_start(analytics).to_owned(),
        body,
        nonce: Some(nonce.clone()),
        generator: inner.config.build.signature.then(crate::signature),
        tail: document_tail(inner, &nonce),
        // A static render has no request: holes stay open (see `render_shell`).
        hole_context: (!ctx.is_static()).then(|| hole_context(inner, ctx, index)),
        known_styles,
    };
    let body = if ctx.is_static() {
        // Rendered once for many requests: the holes are handed to the caller.
        let sink = ctx.clone();
        Body::Stream(Box::pin(futures_util::stream::once(async move {
            let (html, shell) = render_shell(parts).await;
            *sink.shell.lock().unwrap_or_else(|e| e.into_inner()) = shell;
            Ok::<Bytes, crate::request::BoxError>(Bytes::from(html))
        })))
    } else {
        Body::Stream(Box::pin(
            stream_document(parts, streaming).map(|chunk| Ok::<Bytes, crate::request::BoxError>(Bytes::from(chunk))),
        ))
    };
    let mut res = Response::new(http::StatusCode::OK, body);
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
    let metadata =
        root_metadata(inner, ctx).await.merge(Metadata::new().absolute_title("404: Not Found").robots("noindex"));
    // Like any page, the 404 page keeps the root layout on screen when the
    // browser already shows it.
    let root_key = layout_key(inner, &inner.routes.root, &ctx.params);
    let shown = ctx.headers.get("x-nr-nav").is_some_and(|v| v == "1")
        && ctx
            .headers
            .get("x-nr-layouts")
            .and_then(|v| v.to_str().ok())
            .zip(root_key.as_deref())
            .is_some_and(|(keys, key)| keys.split(',').any(|k| k.trim() == key));
    let mut res = match root_key {
        Some(key) if shown => {
            let mut res =
                document_response_with(inner, ctx, None, partial_metadata(metadata), content, false, known_styles(ctx));
            mark_partial(&mut res, &key);
            res
        }
        Some(key) => {
            let marked = next_rust_view::fragment![
                raw_html(format!("{MARKER_OPEN}{key}-->")),
                content,
                raw_html(format!("{MARKER_CLOSE}{key}-->"))
            ];
            let body = root_wrapped(inner, ctx, marked).await;
            document_response(inner, ctx, None, metadata, body, false)
        }
        None => {
            let body = root_wrapped(inner, ctx, content).await;
            document_response(inner, ctx, None, metadata, body, false)
        }
    };
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
    if inner.tracking.crosses_boundary(req) {
        return crate::tracking::full_load();
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
    document_response(inner, ctx, None, metadata, body, false)
}

// ---------------------------------------------------------------------------
// Static rendering and ISR
// ---------------------------------------------------------------------------

/// A page rendered for the page cache.
pub(crate) struct StaticRender {
    /// The whole document, or with `shell` the document up to its tail.
    pub(crate) html: String,
    pub(crate) status: u16,
    /// The page has `dynamic` holes: what finishes `html` for a request.
    pub(crate) shell: Option<Arc<Shell>>,
    fragments: FragmentUse,
}

impl StaticRender {
    /// Seconds until the page is stale: the page's `REVALIDATE`, or sooner
    /// when a cached fragment in it lives shorter.
    pub(crate) fn revalidate(&self, inner: &AppInner, index: usize) -> Option<u64> {
        let page = inner.routes.pages[index].revalidate.or(inner.config.rendering.revalidate);
        let fragments = self.fragments.revalidate.map(|d| d.as_secs().max(1));
        match (page, fragments) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// The cache entry of this render. A shell is registered with the app,
    /// and the entry carries its token.
    pub(crate) fn entry(&self, inner: &AppInner, index: usize, path: &str) -> CacheEntry {
        let def = &inner.routes.pages[index];
        let mut tags: Vec<String> = def.tags.iter().map(|t| (*t).to_owned()).collect();
        tags.extend(self.fragments.tags.iter().filter(|t| !def.tags.contains(&t.as_str())).cloned());
        let mut entry = CacheEntry::new(self.html.clone().into_bytes())
            .revalidate(self.revalidate(inner, index).map(Duration::from_secs))
            .tags(tags);
        if let Some(shell) = &self.shell {
            let token = crate::random_hex(8);
            inner.shells.insert(path, &token, shell.clone(), None);
            entry = entry.meta(SHELL_META, token);
        }
        entry
    }
}

/// Render a static page to HTML. `Err` carries a response to send instead.
#[allow(clippy::result_large_err)]
pub(crate) async fn render_static_html(
    inner: &Arc<AppInner>,
    index: usize,
    path: &str,
    params: Params,
) -> std::result::Result<StaticRender, Response> {
    let mut ctx = RequestContext::for_static_with(path, params, inner.config.clone(), inner.env.is_dev());
    if let Some(ctx) = Arc::get_mut(&mut ctx) {
        ctx.nonce = nonce_placeholder().to_owned();
    }
    // The body is rendered while it is read, so that is tracked too.
    let (rendered, fragments) = track_fragments(async {
        let res = render_page(inner, index, ctx.clone(), false).await;
        if res.headers.contains_key("x-nr-static-error") || res.status.is_redirection() {
            return Err(res);
        }
        let status = res.status.as_u16();
        Ok((res.into_text().await, status))
    })
    .await;
    let (html, status) = rendered?;
    let shell = ctx.shell.lock().unwrap_or_else(|e| e.into_inner()).take().map(Arc::new);
    Ok(StaticRender { html, status, shell, fragments })
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

/// A static page ready to send.
struct StaticHit {
    /// With the request's nonce filled in.
    html: String,
    status: u16,
    shell: Option<Arc<Shell>>,
    /// `x-nr-cache`
    cache: &'static str,
    /// `cache-control` of a complete page sent as it is cached.
    cache_control: String,
}

/// What every response that must not be kept anywhere says. `no-store`
/// alone would do for a current browser; the rest is for intermediaries and
/// older clients that were known to need it.
pub(crate) const NO_STORE: &str = "private, no-cache, no-store, max-age=0, must-revalidate";

async fn serve_static(
    inner: &Arc<AppInner>,
    index: usize,
    req: &Request,
    nonce: &str,
    partial: Option<(usize, String)>,
) -> Response {
    let hit = match serve_static_full(inner, index, req, nonce).await {
        Ok(hit) => hit,
        Err(res) => return res,
    };
    // Client navigations: the browser reports the styles it has, so cached
    // pages send only the CSS it is missing.
    let known: Vec<&str> =
        req.header("x-nr-styles").map(|v| v.split(',').map(str::trim).take(256).collect()).unwrap_or_default();
    let StaticHit { mut html, status, shell, cache, mut cache_control } = hit;
    // The cached document has the `[analytics]` tags; a client navigation
    // gets it without them, or is sent to load a page that opted out in full.
    if crate::tracking::is_navigation(req) {
        match inner.tracking.for_navigation(html) {
            Ok((prepared, changed)) => {
                html = prepared;
                if changed {
                    cache_control = NO_STORE.to_owned();
                }
            }
            Err(()) => return crate::tracking::full_load(),
        }
    }
    let mut region = None;
    if status == 200 && (partial.is_some() || !known.is_empty()) {
        html = next_rust_view::restyle_document(&html, inner.routes.stylesheets, &known);
        cache_control = NO_STORE.to_owned();
        if let Some((_, key)) = partial
            && let Some(cut) = slice_partial(&html, &key, &known, shell.is_none())
        {
            html = cut;
            region = Some(key);
        }
    }
    let mut res = match shell {
        None => Response::html(html).with_cache_control(&cache_control),
        // The holes are this visitor's: the response is never stored. What is
        // shared, the shell, is cached on the server only.
        Some(shell) => resume_response(inner, index, req, nonce, html, shell, &known)
            .with_header("x-nr-prerender", "partial")
            .with_cache_control(NO_STORE),
    };
    res.status = http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::OK);
    res.set_header("x-nr-cache", cache);
    if let Some(key) = region {
        mark_partial(&mut res, &key);
    }
    res
}

/// Send a shell and stream this request's holes after it.
fn resume_response(
    inner: &Arc<AppInner>,
    index: usize,
    req: &Request,
    nonce: &str,
    html: String,
    shell: Arc<Shell>,
    known_styles: &[&str],
) -> Response {
    let ctx = RequestContext::from_request(req, inner.config.clone(), nonce.to_owned(), inner.env.is_dev());
    let resume = Resume {
        html,
        shell,
        context: hole_context(inner, &ctx, Some(index)),
        nonce: Some(nonce.to_owned()),
        tail: document_tail(inner, nonce),
        // Crawlers, and everyone when streaming is off, get the holes in place.
        streaming: inner.config.rendering.streaming && !is_bot(req.header("user-agent")),
        known_styles: known_styles.iter().map(|s| (*s).to_owned()).collect(),
    };
    let stream = resume_document(resume).map(|chunk| Ok::<Bytes, crate::request::BoxError>(Bytes::from(chunk)));
    let mut res = Response::new(http::StatusCode::OK, Body::Stream(Box::pin(stream)));
    res.set_header("content-type", "text/html; charset=utf-8");
    res
}

#[allow(clippy::result_large_err)]
async fn serve_static_full(
    inner: &Arc<AppInner>,
    index: usize,
    req: &Request,
    nonce: &str,
) -> std::result::Result<StaticHit, Response> {
    let def = &inner.routes.pages[index];
    let path = cache_path(req.path());
    let key = CacheKey::page(&path);

    // The token of a cached shell whose holes this process does not have.
    let mut foreign = None;
    if let Ok(Some(entry)) = inner.page_store.get(&key).await {
        // A shell is only usable with its holes, which live in this process.
        let local = entry.meta.get(SHELL_META).map(|token| inner.shells.get(&path, token).ok_or(token));
        match local.transpose() {
            Ok(local) => {
                let stale = entry.is_stale();
                if stale {
                    spawn_regeneration(inner.clone(), index, path.clone(), req.params().clone());
                }
                let html = local.as_ref().and_then(|l| l.html.as_deref()).map(str::as_bytes);
                return Ok(StaticHit {
                    html: fill_nonce(html.unwrap_or(&entry.value), nonce),
                    status: 200,
                    shell: local.map(|l| l.shell),
                    cache: if stale { "STALE" } else { "HIT" },
                    cache_control: static_cache_control(entry.revalidate.map(|d| d.as_secs())),
                });
            }
            Err(token) => foreign = Some(token.clone()),
        }
    }

    if !def.dynamic_params
        && next_rust_router::RoutePattern::is_dynamic(&crate::app::parse_pattern(def.pattern).unwrap_or_default())
    {
        let allowed = static_params(inner, index).await;
        let ok = allowed.is_some_and(|list| list.iter().any(|p| params_equal(p, req.params())));
        if !ok {
            let ctx = RequestContext::from_request(req, inner.config.clone(), nonce.to_owned(), inner.env.is_dev());
            return Err(not_found_page(inner, &ctx).await);
        }
    }

    match render_static_html(inner, index, &path, req.params().clone()).await {
        Ok(render) if render.status == 200 => {
            let revalidate = render.revalidate(inner, index);
            match (&foreign, &render.shell) {
                // Someone else's entry stays: what was rendered here serves
                // this process for as long as that entry is there.
                (Some(token), Some(shell)) => {
                    inner.shells.insert(&path, token, shell.clone(), Some(render.html.as_str().into()));
                }
                _ => {
                    if let Err(e) = inner.page_store.set(key, render.entry(inner, index, &path)).await {
                        crate::log::warn(&format!("could not cache {path}: {e}"));
                    }
                }
            }
            Ok(StaticHit {
                html: fill_nonce(render.html.as_bytes(), nonce),
                status: 200,
                shell: render.shell,
                cache: "MISS",
                cache_control: static_cache_control(revalidate),
            })
        }
        Ok(render) => Ok(StaticHit {
            html: fill_nonce(render.html.as_bytes(), nonce),
            status: render.status,
            shell: render.shell,
            cache: "BYPASS",
            cache_control: NO_STORE.to_owned(),
        }),
        Err(res) => {
            if res.headers.contains_key("x-nr-static-error") {
                crate::log::error(&format!("static render of {path} failed"));
                return Err(Response::text("Internal Server Error").with_status(500));
            }
            Err(res)
        }
    }
}

fn params_equal(a: &Params, b: &Params) -> bool {
    a.len() == b.len() && a.iter().all(|(k, v)| b.value(k) == Some(v))
}

fn fill_nonce(bytes: &[u8], nonce: &str) -> String {
    let text = String::from_utf8_lossy(bytes);
    let placeholder = nonce_placeholder();
    if text.contains(placeholder) { text.replace(placeholder, nonce) } else { text.into_owned() }
}

fn static_cache_control(revalidate: Option<u64>) -> String {
    match revalidate {
        Some(secs) => format!("public, max-age=0, s-maxage={secs}, stale-while-revalidate"),
        None => "public, max-age=0, must-revalidate".to_owned(),
    }
}

/// Marks a path as being revalidated for as long as it lives. A guard rather
/// than a plain `remove` at the end: a page that panics while it is rendered
/// would otherwise leave its path marked for the life of the process, and
/// never be revalidated again.
struct Revalidating {
    inner: Arc<AppInner>,
    path: String,
}

impl Drop for Revalidating {
    fn drop(&mut self) {
        self.inner.revalidating.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.path);
    }
}

fn spawn_regeneration(inner: Arc<AppInner>, index: usize, path: String, params: Params) {
    {
        let mut set = inner.revalidating.lock().unwrap_or_else(|e| e.into_inner());
        if !set.insert(path.clone()) {
            return;
        }
    }
    let guard = Revalidating { inner: inner.clone(), path: path.clone() };
    tokio::spawn(async move {
        let _guard = guard;
        let render =
            std::panic::AssertUnwindSafe(render_static_html(&inner, index, &path, params)).catch_unwind().await;
        match render {
            Ok(Ok(render)) if render.status == 200 => {
                let entry = render.entry(&inner, index, &path);
                let _ = inner.page_store.set(CacheKey::page(&path), entry).await;
                crate::log::debug(&format!("revalidated {path}"));
            }
            Err(_) => crate::log::error(&format!("revalidation of {path} panicked; serving stale content")),
            _ => crate::log::warn(&format!("revalidation of {path} failed; serving stale content")),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page stores its scripts at the end of `<body>`, outside every
    /// layout. Slicing a layout's region out of a pre-rendered page used to
    /// leave them behind, so navigating to a page whose scripts the browser
    /// did not have yet delivered it complete and inert — the components
    /// rendered and nothing answered a click. Only production saw it: a
    /// dynamically rendered partial is built from the page's own document and
    /// always carried them.
    #[test]
    fn a_sliced_page_keeps_the_scripts_that_live_outside_the_layout() {
        let html = concat!(
            "<!DOCTYPE html><html><head><title>t</title></head><body>",
            "<header>shared</header>",
            "<!--nr-l:abc--><main><button>in the region</button></main><!--/nr-l:abc-->",
            r#"<script type="module" data-nr-shim src="/_next-rust/ui.js"></script>"#,
            r#"<script id="__next_rust_env" type="application/json">{}</script>"#,
            "</body></html>",
        );
        let cut = slice_partial(html, "abc", &[], true).expect("the region is there");
        assert!(cut.contains("in the region"), "{cut}");
        assert!(cut.contains(r#"src="/_next-rust/ui.js""#), "the page's script has to travel with it: {cut}");
        // The shared layout is not sent again — the browser keeps it.
        assert!(!cut.contains("shared"), "{cut}");
        // The environment is appended on its own, so it must not be doubled.
        assert_eq!(cut.matches("__next_rust_env").count(), 1, "{cut}");
    }

    /// A script that belongs to the region is part of the region, and must
    /// not be sent a second time after it.
    #[test]
    fn a_script_inside_the_region_is_not_repeated() {
        let html = concat!(
            "<!DOCTYPE html><html><head></head><body>",
            r#"<!--nr-l:k--><div><script src="/inside.js"></script></div><!--/nr-l:k-->"#,
            r#"<script src="/outside.js"></script>"#,
            "</body></html>",
        );
        let cut = slice_partial(html, "k", &[], true).expect("the region is there");
        assert_eq!(cut.matches("/inside.js").count(), 1, "{cut}");
        assert_eq!(cut.matches("/outside.js").count(), 1, "{cut}");
    }
}
