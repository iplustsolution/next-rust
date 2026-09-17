//! Framework endpoints under `/_nr/`.

use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use crate::app::AppInner;
use crate::request::Request;
use crate::response::Response;
use crate::sse::SseEvent;

use crate::client_runtime::{RUNTIME_JS, RUNTIME_JS_MIN};

/// The runtime served to browsers: readable in development, minified otherwise.
fn runtime_js(dev: bool) -> &'static str {
    if dev { RUNTIME_JS } else { RUNTIME_JS_MIN }
}

pub(crate) fn runtime_version(dev: bool) -> &'static str {
    static DEV: OnceLock<String> = OnceLock::new();
    static PROD: OnceLock<String> = OnceLock::new();
    let cell = if dev { &DEV } else { &PROD };
    cell.get_or_init(|| next_rust_assets::content_hash(runtime_js(dev).as_bytes())[..10].to_owned())
}

pub(crate) async fn handle(inner: &AppInner, req: &Request) -> Option<Response> {
    let path = req.path();
    match path {
        "/_nr/runtime.js" => {
            let cc = if req.query_string().starts_with("v=") && !inner.env.is_dev() {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            Some(
                Response::text(runtime_js(inner.env.is_dev()))
                    .with_content_type("text/javascript; charset=utf-8")
                    .with_cache_control(cc),
            )
        }
        "/_nr/dev/events" if inner.env.is_dev() => Some(dev_events(inner)),
        "/_nr/dev/ping" if inner.env.is_dev() => Some(Response::text("ok").with_cache_control("no-store")),
        "/_nr/image" => Some(image(inner, req).await),
        _ if path.starts_with("/_nr/assets/") => Some(asset(inner, req).await),
        _ if path.starts_with("/_nr/client/") => {
            let rest = &path["/_nr/client".len()..];
            let (file, meta) = crate::static_files::resolve_safe(&inner.client_dir, rest).await?;
            let cc = if inner.env.is_dev() { "no-cache" } else { "public, max-age=3600" };
            Some(crate::static_files::serve_file(req, &file, &meta, cc).await)
        }
        _ => None,
    }
}

/// `/_nr/assets/<dir>/<name>.<hash>.<ext>` → `assets/<dir>/<name>.<ext>`.
/// URLs are produced at compile time by `asset!`; when the hash matches the
/// current file the response is cached immutably.
async fn asset(inner: &AppInner, req: &Request) -> Response {
    let rest = &req.path()["/_nr/assets".len()..];
    let (dir, file) = rest.rsplit_once('/').unwrap_or(("", rest));
    let mut parts: Vec<&str> = file.split('.').collect();
    let hash_pos = parts.iter().rposition(|p| p.len() == 16 && p.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(pos) = hash_pos.filter(|p| *p > 0) else { return Response::not_found() };
    let hash = parts.remove(pos).to_owned();
    let original = format!("{dir}/{}", parts.join("."));
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

/// `/_nr/image?url=/photo.jpg&w=640&q=75`
///
/// Only local files from `public/` are served (never remote URLs, so the
/// endpoint cannot be abused for SSRF). The current implementation serves
/// the original file with long-lived caching; resizing and format
/// conversion are not implemented yet (see docs/assets.md).
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
    let Some((file, meta)) =
        crate::static_files::resolve_safe(&inner.public_dir, q.url.split('?').next().unwrap_or("")).await
    else {
        return Response::not_found();
    };
    let mime = next_rust_assets::mime::from_path(&file.to_string_lossy());
    if !mime.starts_with("image/") {
        return Response::text("not an image").with_status(400);
    }
    let cc = format!("public, max-age={}", inner.config.images.max_age);
    let mut res = crate::static_files::serve_file(req, &file, &meta, &cc).await;
    res.set_header("x-nr-image", "original");
    res
}

/// Inline development client: live reload and build error overlay.
pub(crate) fn dev_script(nonce: &str) -> String {
    format!(
        "<script nonce=\"{}\">{}</script>",
        next_rust_view::escape_attr(nonce),
        r#"(()=>{let lost=false,box;const show=(m)=>{if(!box){box=document.createElement("div");box.id="__nr_overlay";box.style.cssText="position:fixed;inset:0;z-index:2147483647;background:rgba(15,15,20,.92);color:#fca5a5;font:13px/1.5 ui-monospace,monospace;padding:32px;overflow:auto;white-space:pre-wrap";document.body.appendChild(box)}box.textContent="Next Rust — build failed\n\n"+m},hide=()=>{box&&box.remove();box=null};const connect=()=>{const es=new EventSource("/_nr/dev/events");es.onopen=()=>{if(lost)location.reload()};es.addEventListener("reload",()=>location.reload());es.addEventListener("error-overlay",(e)=>show(JSON.parse(e.data).message));es.addEventListener("clear",hide);es.onerror=()=>{lost=true;es.close();setTimeout(connect,300)}};connect()})();"#
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
                    } else if !first {
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
