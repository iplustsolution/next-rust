//! Framework endpoints under `/_next-rust/`.

use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use crate::app::AppInner;
use crate::request::Request;
use crate::response::Response;
use crate::sse::SseEvent;

#[cfg(debug_assertions)]
use crate::client_runtime::RUNTIME_JS;
use crate::client_runtime::RUNTIME_JS_MIN;

/// The runtime served to browsers: readable in development, minified
/// otherwise. Release builds contain only the minified copy.
fn runtime_js(dev: bool) -> &'static str {
    #[cfg(debug_assertions)]
    if dev {
        return RUNTIME_JS;
    }
    let _ = dev;
    RUNTIME_JS_MIN
}

pub(crate) fn runtime_version(dev: bool) -> &'static str {
    static DEV: OnceLock<String> = OnceLock::new();
    static PROD: OnceLock<String> = OnceLock::new();
    let cell = if dev { &DEV } else { &PROD };
    cell.get_or_init(|| next_rust_assets::content_hash(runtime_js(dev).as_bytes())[..10].to_owned())
}

/// The banner comment at the top of the framework's scripts while
/// `[build] signature = true`: `/*! Next Rust 0.1.10 */`. Minifiers keep
/// `/*!` comments, so it survives any further bundling.
pub(crate) fn script_banner() -> &'static str {
    static BANNER: OnceLock<String> = OnceLock::new();
    BANNER.get_or_init(|| format!("/*! {} */\n", crate::signature()))
}

/// `script` with the banner in front, when the build is signed.
fn signed_script(inner: &AppInner, script: &'static str) -> String {
    if inner.config.build.signature { format!("{}{script}", script_banner()) } else { script.to_owned() }
}

/// A framework script as served: the text, compressed once per process with
/// Brotli (quality 11) and gzip (level 9).
struct Precompressed {
    text: String,
    br: Vec<u8>,
    gz: Vec<u8>,
}

#[cfg(feature = "compression")]
fn precompress(text: String) -> Precompressed {
    use std::io::Write;
    let mut br = Vec::with_capacity(text.len() / 3);
    {
        let mut w = brotli::CompressorWriter::new(&mut br, 4096, 11, 22);
        let _ = w.write_all(text.as_bytes());
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::with_capacity(text.len() / 3), flate2::Compression::best());
    let gz = gz.write_all(text.as_bytes()).and_then(|()| gz.finish()).unwrap_or_default();
    Precompressed { text, br, gz }
}

#[cfg(not(feature = "compression"))]
fn precompress(text: String) -> Precompressed {
    Precompressed { text, br: Vec::new(), gz: Vec::new() }
}

/// Serve a script in the encoding the client accepts; `cache` keeps the
/// text and its compressed copies (one slot per variant).
fn script_response(
    req: &Request,
    text: impl FnOnce() -> String,
    cache: &'static OnceLock<Precompressed>,
    cc: &str,
) -> Response {
    let accept = req.header("accept-encoding");
    let encoded = cache.get_or_init(|| precompress(text()));
    let pick = if crate::server::accepts(accept, "br") && !encoded.br.is_empty() {
        Some(("br", encoded.br.as_slice()))
    } else if crate::server::accepts(accept, "gzip") && !encoded.gz.is_empty() {
        Some(("gzip", encoded.gz.as_slice()))
    } else {
        None
    };
    let mut res = match pick {
        Some((coding, bytes)) => {
            Response::new(http::StatusCode::OK, crate::response::Body::Bytes(bytes::Bytes::copy_from_slice(bytes)))
                .with_header("content-encoding", coding)
        }
        None => Response::text(encoded.text.clone()),
    };
    res.set_header("vary", "Accept-Encoding");
    res.with_content_type("text/javascript; charset=utf-8").with_cache_control(cc)
}

/// One slot per variant: development or not, signed or not.
type ScriptSlots = [OnceLock<Precompressed>; 4];
static UI_JS_ENCODED: ScriptSlots = [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];
static RUNTIME_JS_ENCODED: ScriptSlots = [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];

fn slot(inner: &AppInner) -> usize {
    usize::from(inner.env.is_dev()) * 2 + usize::from(inner.config.build.signature)
}

/// `(readable, minified)` component script with short class names, when
/// the build gave the component classes short names.
static SHORT_UI_JS: OnceLock<(String, String)> = OnceLock::new();

fn ui_js(dev: bool) -> &'static str {
    match (SHORT_UI_JS.get(), dev) {
        (Some((js, _)), true) => js,
        (Some((_, min)), false) => min,
        (None, true) => next_rust_ui::UI_JS,
        (None, false) => next_rust_ui::UI_JS_MIN,
    }
}

/// Release builds shorten the UI component classes (`nr-card` → `k2`) along
/// with Tailwind's: rename them in the component stylesheet and script too.
/// The renderer already renames them in the HTML.
pub(crate) fn install_short_component_names(names: next_rust_view::class_names::ClassNames) {
    if SHORT_UI_JS.get().is_some() || !names.iter().any(|(class, _)| class.starts_with("nr-")) {
        return;
    }
    let short = |class: &str| {
        names.binary_search_by(|(original, _)| (*original).cmp(class)).ok().map(|i| names[i].1.to_owned())
    };
    let css = next_rust_assets::css::rename_classes(next_rust_ui::UI_CSS.css, &short);
    let id: &'static str =
        Box::leak(format!("nr-ui-{}", &next_rust_assets::content_hash(css.as_bytes())[..8]).into_boxed_str());
    let sheet: &'static next_rust_view::Stylesheet = Box::leak(Box::new(next_rust_view::Stylesheet {
        id,
        css: Box::leak(css.into_boxed_str()),
        per_class: Some(&[]),
        scripts: &[],
    }));
    next_rust_view::style::set_overrides(vec![(&next_rust_ui::UI_CSS, sheet)]);
    let _ = SHORT_UI_JS.set((
        next_rust_ui::rename_script_classes(next_rust_ui::UI_JS, &short),
        next_rust_ui::rename_script_classes(next_rust_ui::UI_JS_MIN, &short),
    ));
}

/// Cache-busting version of `/_next-rust/ui.js`.
pub(crate) fn ui_version(dev: bool) -> &'static str {
    static DEV: OnceLock<String> = OnceLock::new();
    static PROD: OnceLock<String> = OnceLock::new();
    let cell = if dev { &DEV } else { &PROD };
    cell.get_or_init(|| next_rust_assets::content_hash(ui_js(dev).as_bytes())[..10].to_owned())
}

pub(crate) async fn handle(inner: &AppInner, req: &Request) -> Option<Response> {
    let path = req.path();
    match path {
        "/_next-rust/ui.js" => {
            let cc = if req.query_string().starts_with("v=") && !inner.env.is_dev() {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            let dev = inner.env.is_dev();
            Some(script_response(req, || signed_script(inner, ui_js(dev)), &UI_JS_ENCODED[slot(inner)], cc))
        }
        "/_next-rust/runtime.js" => {
            let cc = if req.query_string().starts_with("v=") && !inner.env.is_dev() {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            let dev = inner.env.is_dev();
            Some(script_response(req, || signed_script(inner, runtime_js(dev)), &RUNTIME_JS_ENCODED[slot(inner)], cc))
        }
        "/_next-rust/dev/events" if inner.env.is_dev() => Some(dev_events(inner)),
        "/_next-rust/dev/ping" if inner.env.is_dev() => Some(Response::text("ok").with_cache_control("no-store")),
        "/_next-rust/image" => Some(image(inner, req).await),
        _ if path.starts_with("/_next-rust/assets/") => Some(asset(inner, req).await),
        _ if path.starts_with("/_next-rust/client/") => {
            let rest = &path["/_next-rust/client".len()..];
            if let Some(embedded) = inner.embedded {
                let file = crate::embed::find(embedded.client, rest)?;
                return Some(crate::static_files::serve_embedded(req, file, embedded.built_at, "public, max-age=3600"));
            }
            let (file, meta) = crate::static_files::resolve_safe(&inner.client_dir, rest).await?;
            let cc = if inner.env.is_dev() { "no-cache" } else { "public, max-age=3600" };
            Some(crate::static_files::serve_file(req, &file, &meta, cc).await)
        }
        _ => None,
    }
}

/// `/_next-rust/assets/<dir>/<name>.<hash>.<ext>` → `assets/<dir>/<name>.<ext>`.
/// URLs are produced at compile time by `asset!`; when the hash matches the
/// current file the response is cached immutably.
async fn asset(inner: &AppInner, req: &Request) -> Response {
    let rest = &req.path()["/_next-rust/assets".len()..];
    let (dir, file) = rest.rsplit_once('/').unwrap_or(("", rest));
    let mut parts: Vec<&str> = file.split('.').collect();
    let hash_pos = parts.iter().rposition(|p| p.len() == 16 && p.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(pos) = hash_pos.filter(|p| *p > 0) else { return Response::not_found() };
    let hash = parts.remove(pos).to_owned();
    let original = format!("{dir}/{}", parts.join("."));
    if let Some(embedded) = inner.embedded {
        let Some(file) = crate::embed::find(embedded.assets, &original) else { return Response::not_found() };
        let cc = if file.hash == hash { "public, max-age=31536000, immutable" } else { "no-cache" };
        return crate::static_files::serve_embedded(req, file, embedded.built_at, cc);
    }
    let Some((path, meta)) = crate::static_files::resolve_safe(&inner.config.root.join("assets"), &original).await
    else {
        return Response::not_found();
    };
    let current = asset_hash(&path, &meta).await;
    let cc = if current.as_deref() == Some(hash.as_str()) { "public, max-age=31536000, immutable" } else { "no-cache" };
    crate::static_files::serve_file(req, &path, &meta, cc).await
}

async fn asset_hash(path: &Path, meta: &std::fs::Metadata) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type HashCache = Mutex<HashMap<std::path::PathBuf, (u64, Option<SystemTime>, String)>>;
    static CACHE: OnceLock<HashCache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (meta.len(), meta.modified().ok());
    if let Some((len, mtime, hash)) = cache.lock().ok()?.get(path)
        && (*len, *mtime) == key
    {
        return Some(hash.clone());
    }
    let bytes = tokio::fs::read(path).await.ok()?;
    let hash = next_rust_assets::content_hash(&bytes);
    cache.lock().ok()?.insert(path.to_path_buf(), (key.0, key.1, hash.clone()));
    Some(hash)
}

/// `/_next-rust/image?url=/photo.jpg&w=640&q=75`
///
/// Only local files from `public/` are served (never remote URLs, so the
/// endpoint cannot be abused for SSRF). The current implementation serves
/// the original file with long-lived caching; resizing and format
/// conversion are not implemented yet (see the "Styling & assets" docs page).
async fn image(inner: &AppInner, req: &Request) -> Response {
    #[derive(serde::Deserialize)]
    struct Q {
        url: String,
        w: Option<u32>,
    }
    let Ok(q) = req.query::<Q>() else { return Response::text("invalid image request").with_status(400) };
    if !q.url.starts_with('/') || q.url.starts_with("//") {
        return Response::text("only local images are supported").with_status(400);
    }
    if q.w.is_some_and(|w| w == 0 || w > 8192) {
        return Response::text("invalid width").with_status(400);
    }
    let url = q.url.split('?').next().unwrap_or("");
    let not_image = || Response::text("not an image").with_status(400);
    let cc = format!("public, max-age={}", inner.config.images.max_age);
    let mut res = if let Some(embedded) = inner.embedded {
        let Some(file) = crate::embed::find(embedded.public, url) else { return Response::not_found() };
        if !next_rust_assets::mime::from_path(file.path).starts_with("image/") {
            return not_image();
        }
        crate::static_files::serve_embedded(req, file, embedded.built_at, &cc)
    } else {
        let Some((file, meta)) = crate::static_files::resolve_safe(&inner.public_dir, url).await else {
            return Response::not_found();
        };
        if !next_rust_assets::mime::from_path(&file.to_string_lossy()).starts_with("image/") {
            return not_image();
        }
        crate::static_files::serve_file(req, &file, &meta, &cc).await
    };
    res.set_header("x-nr-image", "original");
    res
}

/// Inline development client: live reload and build error overlay.
pub(crate) fn dev_script(nonce: &str) -> String {
    format!(
        "<script nonce=\"{}\">{}</script>",
        next_rust_view::escape_attr(nonce),
        r#"(()=>{let lost=false,box;const show=(m)=>{if(!box){box=document.createElement("div");box.id="__next_rust_overlay";box.style.cssText="position:fixed;inset:0;z-index:2147483647;background:rgba(15,15,20,.92);color:#fca5a5;font:13px/1.5 ui-monospace,monospace;padding:32px;overflow:auto;white-space:pre-wrap";document.body.appendChild(box)}box.textContent="Next Rust — build failed\n\n"+m},hide=()=>{box&&box.remove();box=null};const connect=()=>{const es=new EventSource("/_next-rust/dev/events");es.onopen=()=>{if(lost)location.reload()};es.addEventListener("reload",()=>location.reload());es.addEventListener("error-overlay",(e)=>show(JSON.parse(e.data).message));es.addEventListener("clear",hide);es.onerror=()=>{lost=true;es.close();setTimeout(connect,300)}};connect()})();"#
    )
}

#[derive(serde::Deserialize, Default, PartialEq, Clone)]
struct DevStatus {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    message: String,
}

fn fingerprint(dir: &Path) -> u64 {
    let mut h = next_rust_assets::Fnv64::new();
    let mut stack = vec![dir.to_path_buf()];
    let mut seen = 0usize;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            seen += 1;
            if seen > 20_000 {
                return h.finish();
            }
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(e.path());
            } else {
                h.write(e.path().to_string_lossy().as_bytes());
                h.write(&meta.len().to_le_bytes());
                let m = meta
                    .modified()
                    .ok()
                    .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                h.write(&m.to_le_bytes());
            }
        }
    }
    h.finish()
}

fn dev_events(inner: &AppInner) -> Response {
    let status_file = inner.dev_status_file.clone();
    let watch = vec![inner.public_dir.clone(), inner.client_dir.clone()];
    let interval = Duration::from_millis(inner.config.dev.poll_interval.max(50));
    let overlay = inner.config.dev.overlay;

    struct State {
        status: Option<DevStatus>,
        files: Option<u64>,
    }

    let stream = futures_util::stream::unfold(State { status: None, files: None }, move |mut st| {
        let status_file = status_file.clone();
        let watch = watch.clone();
        async move {
            loop {
                let (status, files) = {
                    let status_file = status_file.clone();
                    let watch = watch.clone();
                    tokio::task::spawn_blocking(move || {
                        let status = std::fs::read(&status_file)
                            .ok()
                            .and_then(|b| serde_json::from_slice::<DevStatus>(&b).ok())
                            .unwrap_or(DevStatus { ok: true, message: String::new() });
                        let files =
                            watch.iter().map(|d| fingerprint(d)).fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b));
                        (status, files)
                    })
                    .await
                    .unwrap_or_default()
                };
                let first = st.status.is_none();
                let status_changed = st.status.as_ref() != Some(&status);
                let files_changed = st.files.is_some_and(|f| f != files);
                st.files = Some(files);
                st.status = Some(status.clone());
                if status_changed && overlay {
                    if !status.ok {
                        let ev =
                            SseEvent::json(&serde_json::json!({ "message": status.message })).event("error-overlay");
                        return Some((ev, st));
                    }
                    if !first {
                        return Some((SseEvent::data("{}").event("clear"), st));
                    }
                }
                if files_changed {
                    return Some((SseEvent::data("{}").event("reload"), st));
                }
                tokio::time::sleep(interval).await;
            }
        }
    });
    Response::sse_with_keep_alive(stream, Duration::from_secs(10))
}
