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
pub(crate) fn runtime_js(dev: bool) -> &'static str {
    #[cfg(debug_assertions)]
    if dev {
        return RUNTIME_JS;
    }
    let _ = dev;
    RUNTIME_JS_MIN
}

/// The shim as served, with the module's version stamped into the URL it
/// fetches.
///
/// The shim spells its module as a bare literal (`const U =
/// "/_next-rust/runtime.wasm"`) because `[build] inline_assets` can write it
/// into the page as a classic script, where `import.meta.url` is a syntax
/// error. A bare URL cannot be cached — the same URL means something else
/// after the next build — so the server answered it `no-cache`, and the
/// module was re-downloaded on every page load. Stamping the version here
/// makes the URL name one exact build, which is what
/// [`versioned_cache_control`]'s `?v=` branch has always been waiting for.
///
/// The version is the hash of the *pristine* shim plus the module, so this
/// substitution cannot feed back into it.
fn wasm_shim_source(dev: bool) -> String {
    let bare = format!("\"{RUNTIME_MODULE}\"");
    let versioned = format!("\"{}\"", wasm_module_path(dev));
    let shim = crate::client_runtime::shim_js(dev);
    debug_assert!(shim.contains(&bare), "the shim no longer spells {RUNTIME_MODULE} as a bare literal");
    shim.replace(&bare, &versioned)
}

/// `/_next-rust/runtime.wasm?v=<hash>` — the one URL the shim fetches and the
/// preload announces. Both must be byte-identical or the module downloads
/// twice, so they are both this function.
fn wasm_module_path(dev: bool) -> String {
    format!("{RUNTIME_MODULE}?v={}", wasm_runtime_version(dev))
}

/// Cache-busting version of `/_next-rust/runtime.js`. It covers the module as
/// well as the shim: a shim that did not change still has to be re-fetched
/// when the module behind it did.
pub(crate) fn runtime_version(dev: bool) -> &'static str {
    static DEV: OnceLock<String> = OnceLock::new();
    static PROD: OnceLock<String> = OnceLock::new();
    let cell = if dev { &DEV } else { &PROD };
    cell.get_or_init(|| next_rust_assets::content_hash(runtime_js(dev).as_bytes())[..10].to_owned())
}

/// Cache-busting version of the WebAssembly runtime and its shim together.
pub(crate) fn wasm_runtime_version(dev: bool) -> &'static str {
    static DEV: OnceLock<String> = OnceLock::new();
    static PROD: OnceLock<String> = OnceLock::new();
    let cell = if dev { &DEV } else { &PROD };
    cell.get_or_init(|| {
        let mut bytes = crate::client_runtime::shim_js(dev).as_bytes().to_vec();
        bytes.extend_from_slice(crate::client_runtime::RUNTIME_WASM);
        next_rust_assets::content_hash(&bytes)[..10].to_owned()
    })
}

/// The version a page stamps on its `<script src>`, whichever runtime it is.
pub(crate) fn client_version(inner: &AppInner, dev: bool) -> &'static str {
    if inner.config.client.runtime.is_wasm() { wasm_runtime_version(dev) } else { runtime_version(dev) }
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

/// The WebAssembly runtime, compressed once per process like the scripts.
/// It is already `-Oz`'d, so Brotli has less to find than in JavaScript, but
/// a wasm module still compresses to roughly a third of its size.
///
/// `version` becomes the `ETag`. A client that asks for the bare path — a
/// shim cached from before the URL carried `?v=`, a preload written by an
/// older build, a hand-typed request — then revalidates in one empty 304
/// instead of re-downloading the body. Without it `no-cache` meant exactly
/// what it says: ~68 kB brotli on every single page load.
fn wasm_response(req: &Request, bytes: &'static [u8], cc: &str, version: &str) -> Response {
    static ENCODED: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    let (br, gz) = ENCODED.get_or_init(|| compress_bytes(bytes));
    let accept = req.header("accept-encoding");
    let pick = if crate::server::accepts(accept, "br") && !br.is_empty() {
        Some(("br", br.as_slice()))
    } else if crate::server::accepts(accept, "gzip") && !gz.is_empty() {
        Some(("gzip", gz.as_slice()))
    } else {
        None
    };
    let (coding, body) = match pick {
        Some((coding, bytes)) => (Some(coding), bytes),
        None => (None, bytes),
    };
    // The encoding is part of the identity: the same URL answers with three
    // different byte strings under `Vary: Accept-Encoding`, so the tag has to
    // say which one it names. This is the shape `static_files` already uses
    // for the island module (`"<hash>-br"`).
    let etag = match coding {
        Some(coding) => format!("\"{version}-{coding}\""),
        None => format!("\"{version}\""),
    };
    if req.header("if-none-match") == Some(etag.as_str()) {
        return Response::new(http::StatusCode::NOT_MODIFIED, crate::response::Body::Empty)
            .with_header("etag", &etag)
            .with_cache_control(cc);
    }
    let mut res =
        Response::new(http::StatusCode::OK, crate::response::Body::Bytes(bytes::Bytes::copy_from_slice(body)));
    if let Some(coding) = coding {
        res.set_header("content-encoding", coding);
    }
    res.set_header("vary", "Accept-Encoding");
    res.set_header("etag", &etag);
    res.with_content_type("application/wasm").with_cache_control(cc)
}

#[cfg(feature = "compression")]
fn compress_bytes(bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
    use std::io::Write;
    let mut br = Vec::with_capacity(bytes.len() / 3);
    {
        let mut w = brotli::CompressorWriter::new(&mut br, 4096, 11, 22);
        let _ = w.write_all(bytes);
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::with_capacity(bytes.len() / 3), flate2::Compression::best());
    let gz = gz.write_all(bytes).and_then(|()| gz.finish()).unwrap_or_default();
    (br, gz)
}

#[cfg(not(feature = "compression"))]
fn compress_bytes(_bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
    (Vec::new(), Vec::new())
}

/// One slot per variant: development or not, signed or not.
type ScriptSlots = [OnceLock<Precompressed>; 8];
/// `/_next-rust/ui.js` answers with the component script or with the boot
/// shim, so it needs the same third dimension the runtime does.
static UI_JS_ENCODED: ScriptSlots = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

/// `/_next-rust/runtime.js` answers with the JavaScript runtime or with the
/// WebAssembly shim, and the two are nothing alike. They must not share a
/// slot — these are process-wide, and a second application in the same
/// process would otherwise be served whichever runtime the first one warmed.
static RUNTIME_JS_ENCODED: ScriptSlots = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

fn slot(inner: &AppInner) -> usize {
    usize::from(inner.env.is_dev()) * 2 + usize::from(inner.config.build.signature)
}

fn runtime_slot(inner: &AppInner) -> usize {
    slot(inner) + usize::from(inner.config.client.runtime.is_wasm()) * 4
}

fn ui_slot(inner: &AppInner) -> usize {
    slot(inner) + usize::from(inner.config.ui.script.is_wasm()) * 4
}

/// `(readable, minified)` component script with short class names, when
/// the build gave the component classes short names.
static SHORT_UI_JS: OnceLock<(String, String)> = OnceLock::new();

/// What a page loads at `/_next-rust/ui.js` when `[ui] script = "wasm"`.
///
/// It carries no behaviour: the components' logic is in the shared wasm
/// module, and the module's `start()` export wires it up. This asks the
/// island loader to instantiate the module, which is the only thing a page
/// with components but no islands would otherwise never do.
///
/// It is served at the same URL as the script it replaces so the versioning,
/// caching, CSP nonce and inlining all keep working unchanged, and the tag is
/// marked `data-nr-shim` so an audit counts it as shim rather than as
/// application JavaScript.
///
/// It asks `window.__nrm` — the page's own module registry — rather than
/// importing the loader by URL, and that is load-bearing. With
/// `[build] inline_assets` the loader is *inlined into the page*, registered
/// under the registry key `client/__nrw.js` and never fetched; a bare
/// `import "/_next-rust/client/__nrw.js"` is keyed by URL instead, so the
/// browser builds a **second, unrelated module instance**, instantiates the
/// wasm a second time, and calls `start()` again. Everything `start()` puts on
/// `document` then exists twice, and every toggle written as "read the state,
/// invert it" runs twice and lands back where it began: a colour-mode button
/// that does nothing, a password reveal that does nothing. Nothing throws, so
/// there is no error anywhere to find it by. It also cannot happen without
/// inlining — both halves resolve to the same URL and the browser dedupes
/// them — which is why it appears only in a release build. The registry falls
/// back to the URL import itself for a key that was not inlined, and `?.`
/// covers a page that has no registry at all.
pub(crate) const UI_BOOT_JS: &str =
    "const{start}=await(window.__nrm?.load(\"client/__nrw.js\")??import(\"/_next-rust/client/__nrw.js\"));start();\n";

pub(crate) fn ui_js(dev: bool, wasm: bool) -> &'static str {
    if wasm {
        return UI_BOOT_JS;
    }
    match (SHORT_UI_JS.get(), dev) {
        (Some((js, _)), true) => js,
        (Some((_, min)), false) => min,
        (None, true) => next_rust_ui::UI_JS,
        (None, false) => next_rust_ui::UI_JS_MIN,
    }
}

/// Release builds shorten the UI component classes (`nr-card` → `k2`) along
/// with Tailwind's: rename them in the component stylesheet and script too.
/// The renderer already renames them in the HTML. The component script also
/// gets the build's other short names (`data-open`, see [`crate::markup`]);
/// the stylesheet gets those with every other `<style>`, as a page leaves.
pub(crate) fn install_short_component_names(
    names: next_rust_view::class_names::ClassNames,
    other: crate::markup::NameTable,
) {
    let classes = names.iter().any(|(class, _)| class.starts_with("nr-"));
    if SHORT_UI_JS.get().is_some() || !(classes || !other.is_empty()) {
        return;
    }
    let short = |class: &str| {
        names.binary_search_by(|(original, _)| (*original).cmp(class)).ok().map(|i| names[i].1.to_owned())
    };
    if classes {
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
    }
    let script = |js: &str| {
        let js = next_rust_ui::rename_script_classes(js, &short);
        if other.is_empty() {
            js
        } else {
            next_rust_assets::names::rename_js_text(&js, &|key| next_rust_assets::names::find(other, key))
        }
    };
    let _ = SHORT_UI_JS.set((script(next_rust_ui::UI_JS), script(next_rust_ui::UI_JS_MIN)));
}

/// Per-page stylesheets keep the rules for what the framework's own scripts
/// set (`data-open`, `hidden`, `--nr-load`, elements they create): tell the
/// renderer which names those are. The minified copies carry no comments.
pub(crate) fn register_framework_scripts(stylesheets: &[&'static next_rust_view::Stylesheet]) {
    next_rust_view::set_framework_scripts(crate::client_runtime::RUNTIME_JS_MIN, next_rust_ui::UI_JS_MIN);
    // The names the project's scripts use, for `global_css!` and UI sheets.
    next_rust_view::set_script_names(stylesheets);
}

/// Cache-busting version of `/_next-rust/ui.js`.
pub(crate) fn ui_version(dev: bool, wasm: bool) -> &'static str {
    static DEV: OnceLock<String> = OnceLock::new();
    static PROD: OnceLock<String> = OnceLock::new();
    let cell = if dev { &DEV } else { &PROD };
    cell.get_or_init(|| next_rust_assets::content_hash(ui_js(dev, wasm).as_bytes())[..10].to_owned())
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
            let wasm = inner.config.ui.script.is_wasm();
            Some(script_response(req, || signed_script(inner, ui_js(dev, wasm)), &UI_JS_ENCODED[ui_slot(inner)], cc))
        }
        "/_next-rust/runtime.js" => {
            let cc = if req.query_string().starts_with("v=") && !inner.env.is_dev() {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            let dev = inner.env.is_dev();
            let wasm = inner.config.client.runtime.is_wasm();
            let text = move || {
                if wasm {
                    // The shim, with `?v=<hash>` stamped into the module URL.
                    let shim = wasm_shim_source(dev);
                    if inner.config.build.signature { format!("{}{shim}", script_banner()) } else { shim }
                } else {
                    signed_script(inner, runtime_js(dev))
                }
            };
            Some(script_response(req, text, &RUNTIME_JS_ENCODED[runtime_slot(inner)], cc))
        }
        // The module behind the shim. Immutable like any other versioned
        // asset: its URL carries the hash of the shim and the module together.
        "/_next-rust/runtime.wasm" if inner.config.client.runtime.is_wasm() => {
            let dev = inner.env.is_dev();
            let version = wasm_runtime_version(dev);
            // Immutable only when the version asked for is the one being
            // served. `?v=` from an older build names bytes we no longer
            // have, and answering that with `immutable` would pin today's
            // module — which an older shim cannot drive — under yesterday's
            // key, forever.
            let cc = if !dev && req.query_string().strip_prefix("v=") == Some(version) {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            Some(wasm_response(req, crate::client_runtime::RUNTIME_WASM, cc, version))
        }
        "/_next-rust/dev/events" if inner.env.is_dev() => Some(dev_events(inner)),
        "/_next-rust/dev/ping" if inner.env.is_dev() => Some(Response::text("ok").with_cache_control("no-store")),
        "/_next-rust/image" => Some(image(inner, req).await),
        _ if path.starts_with("/_next-rust/m/") => crate::inline::lazy_module(inner, req),
        _ if path.starts_with("/_next-rust/assets/") => Some(asset(inner, req).await),
        _ if path.starts_with("/_next-rust/client/") => {
            let rest = &path["/_next-rust/client".len()..];
            // A `?v=<hash>` request names one exact build of the file, so it
            // can be cached forever: the shim asks for the module under the
            // hash the build wrote into it, and a new build changes the URL.
            // Without the hash the file keeps revalidating, as before. Same
            // rule as `/_next-rust/runtime.js` above.
            let cc = versioned_cache_control(inner, req);
            if let Some(embedded) = inner.embedded {
                let file = crate::embed::find(embedded.client, rest)?;
                return Some(crate::static_files::serve_embedded(req, file, embedded.built_at, cc));
            }
            let Some((file, meta)) = crate::static_files::resolve_safe(&inner.client_dir, rest).await else {
                // Generated, but not a script: the wasm module of
                // `[build] wasm_islands`, which keeps its own media type.
                if let Some(res) = declared_binary(inner, req, "client", rest, cc) {
                    return Some(res);
                }
                // A module declared in Rust (`client_script!`).
                return declared_script(inner, req, "client", rest, cc);
            };
            Some(crate::static_files::serve_file(req, &file, &meta, cc).await)
        }
        _ => None,
    }
}

/// The source of the script `<field>/<rel>` when it was declared in Rust and
/// is not embedded (development builds; release builds embed declared
/// scripts with the files of `client/` and `assets/`).
fn find_declared(inner: &AppInner, field: &str, rel: &str) -> Option<&'static str> {
    let rel = rel.trim_start_matches('/');
    if rel.split('/').any(|s| s.is_empty() || s.starts_with('.')) {
        return None;
    }
    inner
        .routes
        .declared_scripts
        .iter()
        .find(|(key, _)| key.strip_prefix(field).and_then(|k| k.strip_prefix('/')) == Some(rel))
        .map(|(_, source)| *source)
}

/// Path of the WebAssembly browser runtime, as `runtime/shim.js` spells it.
/// It is a literal there and a literal here, because `[build] inline_assets`
/// can write a shim into the page as a classic script, where
/// `import.meta.url` is a syntax error.
pub(crate) const RUNTIME_MODULE: &str = "/_next-rust/runtime.wasm";

/// The URL to preload, or `None` when this page will not fetch a module.
///
/// **This must be byte-identical to the URL the shim's own `fetch` uses.** A
/// preload is matched to the later request by exact URL and CORS mode, and a
/// mismatch does not degrade gracefully — it downloads the module a second
/// time, which is worse than not preloading at all. So this is written as
/// "what will the shim ask for", not "what is the nicest URL we could serve".
///
/// That URL is [`wasm_module_path`] — `?v=<hash>` and all — and the shim is
/// rewritten through the same function by [`wasm_shim_source`], so the two
/// cannot drift apart. Before that, both spelled the bare path: the server
/// answered it `no-cache` with no `ETag`, so every page load re-downloaded
/// the whole module. (The comment that used to sit here said it "revalidates
/// against its `ETag`" — there was no `ETag`. There is one now, for the bare
/// path, but the stamped URL means nothing asks for it.)
///
/// Note there are two modules today, not one: `[client] runtime = "wasm"`
/// fetches [`RUNTIME_MODULE`] on every page, while the island shim fetches
/// `/_next-rust/client/__nrw/islands.wasm` only once an island actually
/// hydrates. We preload the runtime's, because that is the one a page on the
/// wasm runtime is certain to need; preloading a module the page may never
/// ask for would cost every visitor a download to save some of them a wait.
///
/// That the two are separate modules is a known duplication, not a design:
/// both carry the shared runtime machinery, so a page with islands fetches
/// roughly 68 kB brotli of it twice. Once they are genuinely one module this
/// function has one answer instead of a choice.
pub(crate) fn wasm_module_url(inner: &AppInner) -> Option<String> {
    inner.config.client.runtime.is_wasm().then(|| wasm_module_path(inner.env.is_dev()))
}

/// How long a file under `/_next-rust/client/` may be cached.
///
/// A request carrying `?v=<hash>` names one exact build — the shim and the
/// preload link both spell the module's content hash into the URL — so the
/// answer can be cached forever and never revalidated. A bare path stays
/// revalidating, because the same URL will mean something else tomorrow.
/// Development never caches: the next keystroke changes the bytes.
fn versioned_cache_control(inner: &AppInner, req: &Request) -> &'static str {
    if inner.env.is_dev() {
        "no-cache"
    } else if req.query_string().starts_with("v=") {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    }
}

/// A generated non-script file (the WebAssembly modules of `[build]
/// wasm_islands`, `[client] runtime = "wasm"` and `[ui] script = "wasm"`),
/// served with the media type its extension implies — a browser refuses to
/// stream-compile a module that does not arrive as `application/wasm`.
///
/// This is the development path; release builds embed the module with the
/// files of `client/` and answer from [`crate::static_files::serve_embedded`],
/// which has copies compressed at build time. Here the copies are made once
/// per process on the first request, because a `.wasm` compresses well
/// (roughly 2.5–3×) and a module is the one thing on the page that hydration
/// waits for.
fn declared_binary(inner: &AppInner, req: &Request, field: &str, rel: &str, cc: &str) -> Option<Response> {
    let name = rel.trim_start_matches('/');
    if name.split('/').any(|s| s.is_empty() || s.starts_with('.')) {
        return None;
    }
    let (_, bytes) = inner
        .routes
        .declared_binaries
        .iter()
        .find(|(key, _)| key.strip_prefix(field).and_then(|k| k.strip_prefix('/')) == Some(name))?;
    let etag = format!("\"{}\"", next_rust_assets::content_hash(bytes));
    if req.header("if-none-match") == Some(etag.as_str()) {
        return Some(
            Response::new(http::StatusCode::NOT_MODIFIED, crate::response::Body::Empty).with_header("etag", &etag),
        );
    }
    let content_type = next_rust_assets::mime::from_path(name);
    let mut res = match compressed_binary(bytes, req.header("accept-encoding")) {
        Some((coding, body)) => Response::bytes(body).with_header("content-encoding", coding),
        None => Response::bytes(bytes::Bytes::from_static(bytes)),
    };
    res.set_header("vary", "Accept-Encoding");
    Some(res.with_content_type(content_type).with_cache_control(cc).with_header("etag", &etag))
}

/// Compressed copies of the generated binaries, keyed by content hash so a
/// rebuilt module is compressed again rather than served stale.
type BinaryCache = std::sync::Mutex<std::collections::HashMap<String, (Vec<u8>, Vec<u8>)>>;
static BINARY_ENCODED: OnceLock<BinaryCache> = OnceLock::new();

/// The best encoding the client accepts, compressing on first use. Returns
/// `None` when nothing is accepted or compression would not pay for itself.
#[cfg(feature = "compression")]
fn compressed_binary(bytes: &'static [u8], accept: Option<&str>) -> Option<(&'static str, bytes::Bytes)> {
    let wants_br = crate::server::accepts(accept, "br");
    let wants_gz = crate::server::accepts(accept, "gzip");
    if !wants_br && !wants_gz {
        return None;
    }
    let cache = BINARY_ENCODED.get_or_init(Default::default);
    let key = next_rust_assets::content_hash(bytes);
    let mut cache = cache.lock().ok()?;
    let (br, gz) = cache.entry(key).or_insert_with(|| {
        use std::io::Write;
        let mut br = Vec::with_capacity(bytes.len() / 3);
        {
            let mut w = brotli::CompressorWriter::new(&mut br, 4096, 11, 22);
            let _ = w.write_all(bytes);
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::with_capacity(bytes.len() / 3), flate2::Compression::best());
        let gz = gz.write_all(bytes).and_then(|()| gz.finish()).unwrap_or_default();
        (br, gz)
    });
    // A copy that did not get smaller is not worth the client's CPU.
    let pick = |coding, encoded: &Vec<u8>| {
        (!encoded.is_empty() && encoded.len() < bytes.len()).then(|| (coding, bytes::Bytes::copy_from_slice(encoded)))
    };
    if wants_br && let Some(picked) = pick("br", br) {
        return Some(picked);
    }
    if wants_gz { pick("gzip", gz) } else { None }
}

#[cfg(not(feature = "compression"))]
fn compressed_binary(_bytes: &'static [u8], _accept: Option<&str>) -> Option<(&'static str, bytes::Bytes)> {
    None
}

fn declared_script(inner: &AppInner, req: &Request, field: &str, rel: &str, cc: &str) -> Option<Response> {
    let source = find_declared(inner, field, rel)?;
    let etag = format!("\"{}\"", next_rust_assets::content_hash(source.as_bytes()));
    if req.header("if-none-match") == Some(etag.as_str()) {
        return Some(
            Response::new(http::StatusCode::NOT_MODIFIED, crate::response::Body::Empty).with_header("etag", &etag),
        );
    }
    let res = Response::text(source).with_content_type("text/javascript; charset=utf-8").with_cache_control(cc);
    Some(res.with_header("etag", &etag))
}

/// A script kept in the binary, served under a content hash (`id`).
pub(crate) fn script_bytes_response(req: &Request, id: &str, code: &'static str, cc: &str) -> Response {
    let etag = format!("\"{id}\"");
    if req.header("if-none-match") == Some(etag.as_str()) {
        return Response::new(http::StatusCode::NOT_MODIFIED, crate::response::Body::Empty).with_header("etag", &etag);
    }
    Response::new(http::StatusCode::OK, crate::response::Body::Bytes(bytes::Bytes::from_static(code.as_bytes())))
        .with_content_type("text/javascript; charset=utf-8")
        .with_cache_control(cc)
        .with_header("etag", &etag)
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
        // A script declared in Rust (`asset_script!`).
        let declared = declared_script(inner, req, "assets", &original, "no-cache");
        return match (declared, find_declared(inner, "assets", &original)) {
            (Some(res), Some(source)) if next_rust_assets::content_hash(source.as_bytes()) == hash => {
                res.with_cache_control("public, max-age=31536000, immutable")
            }
            (Some(res), _) => res,
            _ => Response::not_found(),
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `[ui] script = "wasm"` replaces the component script with a module
    /// that loads the shared WebAssembly module and nothing else. If any
    /// behaviour ever creeps back into it, the page has JavaScript logic in
    /// it again and the whole point is lost — so this is spelled out rather
    /// than left to review.
    #[test]
    fn the_wasm_boot_module_holds_no_behaviour() {
        let boot = ui_js(false, true);
        assert_eq!(boot, UI_BOOT_JS);
        assert_eq!(ui_js(true, true), UI_BOOT_JS, "development serves the same module: there is nothing to read");
        assert!(boot.len() < 128, "the boot module grew to {} bytes; it should be one import and one call", boot.len());
        assert!(boot.contains("/_next-rust/client/__nrw.js"), "it loads the island loader");
        assert!(boot.contains("start()"), "and asks it to instantiate the module, which runs the components' `start`");
        // Through the page's module registry, never by URL alone: an inlined
        // loader is registered under its key and not under its URL, so a bare
        // URL import would build a SECOND module instance, instantiate the
        // wasm twice and run `start()` twice — every delegated listener
        // duplicated, every toggle a no-op, and nothing thrown to find it by.
        // `__nrm.load` falls back to the URL import for a key nothing inlined.
        assert!(
            boot.contains("__nrm?.load(\"client/__nrw.js\")"),
            "the boot module must go through the page's module registry, or an inlined loader is instantiated twice"
        );
        assert!(!boot.contains("import{"), "a static import cannot consult the registry first");
        // A page with components but no islands would otherwise never
        // instantiate the module, because hydration is what loads it.
        for behaviour in ["data-nr-ui", "addEventListener", "querySelector", "=>", "function", "if(", "nr-"] {
            assert!(!boot.contains(behaviour), "the boot module contains `{behaviour}`: that is behaviour, not a shim");
        }
    }

    /// The JavaScript path is untouched by the switch.
    #[test]
    fn the_script_path_still_serves_the_component_script() {
        assert_eq!(ui_js(true, false), next_rust_ui::UI_JS);
        assert_eq!(ui_js(false, false), next_rust_ui::UI_JS_MIN);
        assert!(ui_js(false, false).contains("data-nr-press"), "the components' behaviour is still in it");
    }

    /// The two paths must not share a cache-busting version, or a site that
    /// switches would serve one from the browser's cache of the other.
    #[test]
    fn each_path_has_its_own_version() {
        let script = next_rust_assets::content_hash(ui_js(false, false).as_bytes());
        let boot = next_rust_assets::content_hash(ui_js(false, true).as_bytes());
        assert_ne!(script, boot);
    }
}
