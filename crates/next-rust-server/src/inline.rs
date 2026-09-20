//! Inline delivery of scripts (`[build] inline_assets = true`).
//!
//! With the switch on, an HTML response leaves the server without any
//! `<script src>` that points at the framework or the app:
//!
//! * `/_next-rust/runtime.js` and `/_next-rust/ui.js` are written into the
//!   document as inline module scripts — **except** the runtime under
//!   `[client] runtime = "wasm"`, where the shim stays a `<script src>` so it
//!   is cached once for the whole site instead of repeated in every page, and
//!   keeps its `data-nr-shim` marking (`Inliner::wasm_runtime`);
//! * a `<script src>` naming a script of `assets/` or `client/` (what
//!   `asset!("js/site.js")` produces) becomes an inline script with the same
//!   attributes;
//! * the modules of the islands on the page (`<nr-island data-module>`), and
//!   what they import, are registered in `window.__nrm` by inline scripts in
//!   `<head>` (classic scripts, so a module is registered before anything
//!   after it runs); the runtime hydrates islands from that registry.
//!
//! Every inline script carries `data-nr-js="<id>"`. The runtime reports the
//! ids it has on client navigations (`x-nr-scripts`), so a script is sent
//! once per document, and it never runs a module script twice.
//!
//! The scripts were prepared at build time (`next_rust_build::js_inline`):
//! modules are factories, so they need no URL to import each other. Modules
//! reached only through `import("./x.js")` are not written into pages; they
//! are served at `/_next-rust/m/<hash>`.
//!
//! The work happens on the finished response, after rendering and caching,
//! so every kind of page (streamed, static, partial, resumed) is covered and
//! each response gets the nonce of its own request.

use std::collections::HashSet;
use std::sync::OnceLock;

use bytes::Bytes;
use futures_util::StreamExt;

use crate::app::AppInner;
use crate::request::Request;
use crate::response::{Body, Response};

/// How a prepared script is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineKind {
    /// An ES module as a factory: `async (import) => exports`.
    Factory,
    /// A script without imports or exports, as written.
    Plain,
    /// An ES module served at `/_next-rust/m/<id>`, never written into a page.
    Lazy,
}

/// A script prepared for inline delivery by the build.
#[derive(Debug, Clone, Copy)]
pub struct InlineScript {
    /// `client/chart.js`, `assets/js/site.js`.
    pub key: &'static str,
    /// Content hash: the `data-nr-js` id, or the URL of a lazy module.
    pub id: &'static str,
    pub kind: InlineKind,
    pub code: &'static str,
    /// Keys of the modules a factory imports.
    pub deps: &'static [&'static str],
}

/// Creates the module registry when the page has none, as `r`. A module is
/// started once, when something first asks for it; a key without a factory
/// is imported from its URL. Both tables have a null prototype, so a module
/// key like `constructor` or `__proto__` cannot resolve to a member of
/// `Object.prototype` (the runtime's lookup matches).
const REGISTRY: &str = "const r=window.__nrm||(window.__nrm={f:Object.create(null),p:Object.create(null),load(k){return this.p[k]||(this.p[k]=this.f[k]?this.f[k](s=>this.load(s)):import(\"/_next-rust/\"+k))}});";

/// What the request says about the page the browser already shows.
pub(crate) struct Known {
    /// A client navigation: the runtime is running.
    nav: bool,
    /// `data-nr-js` ids the document has (`x-nr-scripts`).
    ids: HashSet<String>,
}

/// `Some` when responses to `req` get their scripts inline.
pub(crate) fn known(inner: &AppInner, req: &Request) -> Option<Known> {
    if !inner.config.build.inline_assets {
        return None;
    }
    let nav = req.header("x-nr-nav") == Some("1");
    let ids = req
        .header("x-nr-scripts")
        .filter(|_| nav)
        .map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).take(256).map(str::to_owned).collect())
        .unwrap_or_default();
    Some(Known { nav, ids })
}

/// The framework's scripts, escaped for a `<script>` element: `(id, code)`
/// of the runtime and of the component script.
fn framework_scripts(inner: &AppInner) -> &'static [(String, String); 2] {
    static DEV: OnceLock<[(String, String); 2]> = OnceLock::new();
    static PROD: OnceLock<[(String, String); 2]> = OnceLock::new();
    let dev = inner.env.is_dev();
    let wasm_ui = inner.config.ui.script.is_wasm();
    let signed = inner.config.build.signature;
    (if dev { &DEV } else { &PROD }).get_or_init(|| {
        let prepare = |name: &str, version: &str, code: &str| {
            let banner = if signed { crate::internal::script_banner() } else { "" };
            (format!("{name}-{version}"), next_rust_assets::html::escape_inline_script(&format!("{banner}{code}")))
        };
        [
            prepare("rt", crate::internal::runtime_version(dev), crate::internal::runtime_js(dev)),
            prepare("ui", crate::internal::ui_version(dev, wasm_ui), crate::internal::ui_js(dev, wasm_ui)),
        ]
    })
}

/// Rewrites the HTML of one response.
struct Inliner {
    table: &'static [InlineScript],
    framework: &'static [(String, String); 2],
    nonce: String,
    known: Known,
    /// `[client] runtime = "wasm"`: the shim is left as a `<script src>`
    /// rather than written into the page. Two reasons, and the first is a
    /// correctness one:
    ///
    /// * `framework_scripts` prepares the *JavaScript* runtime, which is not
    ///   what this page's `<script>` asks for. Inlining it here would put the
    ///   whole JavaScript runtime into a page that also loads the WebAssembly
    ///   one, and the `data-nr-shim` marking would be lost with the tag.
    /// * Inlining trades one cached request for bytes on every page view. That
    ///   is a good trade for small per-page scripts and a bad one for the
    ///   shim: it is byte-identical on every page, does not change between
    ///   deploys, and is ~3 KB brotli each time. Served externally it is
    ///   fetched once under an immutable URL and shared by the whole site.
    ///   Nothing is lost by leaving it out, because a page using the wasm
    ///   runtime has to fetch the `.wasm` before it can do anything anyway —
    ///   the "no requests before interactive" property inlining exists to
    ///   protect is already gone.
    wasm_runtime: bool,
    /// `[ui] script = "wasm"`: the same, for `/_next-rust/ui.js`.
    ///
    /// On that setting the file is not the components' behaviour but
    /// [`crate::internal::UI_BOOT_JS`] — two statements that import the island
    /// loader and call `start()` — and its tag carries `data-nr-shim` for the
    /// same reason the runtime's does. Both halves of the argument above apply
    /// unchanged: the prepared copy in `framework[1]` is the *JavaScript*
    /// component script, which is not what this page asked for, and inlining
    /// turns the tag into an anonymous inline script, which loses the marking
    /// and files the boot module under application JavaScript in any audit.
    ///
    /// It is only ~80 bytes, so this is about attribution rather than weight;
    /// but a page that reports carrying application logic it does not carry is
    /// exactly what the marking exists to prevent.
    wasm_ui: bool,
    /// Ids written into this response so far.
    sent: HashSet<&'static str>,
}

/// One parsed start tag: `(name, value as written between the quotes)`.
type Attrs<'a> = Vec<(&'a str, Option<&'a str>)>;

/// Parse the start tag at the beginning of `html` (`<script …>`): its
/// attributes and its length, or `None` when the tag is not complete.
fn start_tag(html: &str) -> Option<(Attrs<'_>, usize)> {
    let bytes = html.as_bytes();
    let mut i = 1;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
        i += 1;
    }
    let mut attrs = Vec::new();
    loop {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        match bytes.get(i) {
            None => return None,
            Some(b'>') => return Some((attrs, i + 1)),
            _ => {}
        }
        let name_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && !matches!(bytes[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let name = &html[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) != Some(&b'=') {
            attrs.push((name, None));
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = match bytes.get(i) {
            None => return None,
            Some(q @ (b'"' | b'\'')) => {
                let start = i + 1;
                let len = html[start..].find(*q as char)?;
                i = start + len + 1;
                &html[start..start + len]
            }
            Some(_) => {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                &html[start..i]
            }
        };
        attrs.push((name, Some(value)));
    }
}

fn attr<'a>(attrs: &Attrs<'a>, name: &str) -> Option<&'a str> {
    attrs.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).and_then(|(_, v)| *v)
}

/// A script key as a JavaScript string literal, quotes included: keys are
/// file names, so JSON takes care of quotes, backslashes and control
/// characters, and the raw-text escaper of anything that would end the
/// `<script>` element.
fn js_string(value: &str) -> String {
    let quoted = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned());
    next_rust_assets::html::escape_inline_script(&quoted)
}

/// Where `</head>` is in a piece of markup copied to `out`, once. The
/// contents of a script element are not markup and are not searched.
fn note_head(at: &mut Option<usize>, out: &str, markup: &str) {
    if at.is_none()
        && let Some(i) = markup.find("</head>")
    {
        *at = Some(out.len() + i);
    }
}

impl Inliner {
    fn find(&self, key: &str) -> Option<&'static InlineScript> {
        self.table.iter().find(|s| s.key == key && s.kind != InlineKind::Lazy)
    }

    fn open(&self, out: &mut String, id: &str, module: bool) {
        out.push_str(if module { "<script type=\"module\" data-nr-js=\"" } else { "<script data-nr-js=\"" });
        // Ids are content hashes, so this escapes nothing today; it keeps the
        // attribute sound if that ever stops being true.
        out.push_str(&next_rust_view::escape_attr(id));
        out.push_str("\" nonce=\"");
        out.push_str(&next_rust_view::escape_attr(&self.nonce));
        out.push_str("\">");
    }

    /// Scripts registering `script` and what it imports, imports first.
    fn define(&mut self, script: &'static InlineScript, out: &mut String) {
        if script.kind != InlineKind::Factory || self.sent.contains(script.id) {
            return;
        }
        // Before the imports: an import cycle ends here.
        self.sent.insert(script.id);
        for dep in script.deps {
            if let Some(dep) = self.find(dep) {
                self.define(dep, out);
            }
        }
        if self.known.ids.contains(script.id) {
            return;
        }
        // A classic script: it has run by the time the next element is
        // parsed or inserted, so the module is there when an island asks.
        self.open(out, script.id, false);
        let key = js_string(script.key);
        out.push_str(&format!("{{{REGISTRY}r.f[{0}]||(r.f[{0}]={1})}}</script>", key, script.code));
    }

    /// A framework script (`which`: 0 runtime, 1 components) in place of its `<script src>`.
    fn framework(&mut self, which: usize, out: &mut String) {
        let (id, code) = &self.framework[which];
        // The runtime asked for this page, so it is running.
        if (which == 0 && self.known.nav) || self.known.ids.contains(id) {
            return;
        }
        self.open(out, id, true);
        out.push_str(code);
        out.push_str("</script>");
    }

    /// `<script src="…">` of the app, inline: the element with its other
    /// attributes and the code inside.
    fn entry(&mut self, script: &'static InlineScript, attrs: &Attrs<'_>, out: &mut String) {
        let module = attr(attrs, "type").is_some_and(|t| t.eq_ignore_ascii_case("module"));
        let id = if script.kind == InlineKind::Factory { format!("{}-run", script.id) } else { script.id.to_owned() };
        // A module runs once per document, like a module loaded from a URL.
        if module && self.known.ids.contains(&id) {
            return;
        }
        if script.kind == InlineKind::Factory {
            for dep in script.deps {
                if let Some(dep) = self.find(dep) {
                    self.define(dep, out);
                }
            }
        }
        out.push_str("<script");
        for (name, value) in attrs {
            if ["src", "integrity", "crossorigin", "data-nr-js"].iter().any(|n| name.eq_ignore_ascii_case(n)) {
                continue;
            }
            out.push(' ');
            out.push_str(name);
            if let Some(value) = value {
                out.push_str("=\"");
                out.push_str(&value.replace('"', "&quot;"));
                out.push('"');
            }
        }
        // The code comes from the build, so it may run under a nonce policy
        // even when the element was written without one.
        if attr(attrs, "nonce").is_none() {
            out.push_str(" nonce=\"");
            out.push_str(&next_rust_view::escape_attr(&self.nonce));
            out.push('"');
        }
        out.push_str(" data-nr-js=\"");
        out.push_str(&next_rust_view::escape_attr(&id));
        // A classic script runs again on every client navigation, as it
        // would when loaded from its URL.
        out.push_str(if module { "\">" } else { "\" data-nr-rerun>" });
        if script.kind == InlineKind::Factory {
            out.push_str(REGISTRY);
            let key = js_string(script.key);
            out.push_str(&format!("r.f[{0}]||(r.f[{0}]={1});r.load({0});", key, script.code));
        } else {
            out.push_str(script.code);
        }
        out.push_str("</script>");
    }

    /// Rewrite `html`, a run of complete tags.
    fn process(&mut self, html: &str) -> String {
        let mut out = String::with_capacity(html.len() + 1024);
        // Registrations for the islands of this piece; they go into <head>.
        let mut head = String::new();
        // Where the first island that needed one starts: a piece of a streamed
        // body may begin anywhere (inside an attribute value, an entity, a
        // `<style>`), so without a `</head>` that is the one place known to
        // be between tags.
        let mut before_island = 0;
        // The `</head>` of the document, when this piece holds it. Only text
        // between elements is searched: a `</head>` in the code of a script
        // (the page's own, or one written in here) is not the end of a head.
        let mut head_at: Option<usize> = None;
        // Set when the piece ends inside a tag: what is left of it is not
        // known to be markup either.
        let mut truncated = false;
        let mut rest = html;
        loop {
            let next = ["<script", "<nr-island"].iter().filter_map(|t| rest.find(t)).min();
            let Some(at) = next else { break };
            note_head(&mut head_at, &out, &rest[..at]);
            out.push_str(&rest[..at]);
            rest = &rest[at..];
            let Some((attrs, len)) = start_tag(rest) else {
                truncated = true;
                break;
            };
            if rest.starts_with("<nr-island") {
                if let Some(script) =
                    attr(&attrs, "data-module").and_then(|m| self.find(&next_rust_view::script_key(m)))
                {
                    if head.is_empty() {
                        before_island = out.len();
                    }
                    self.define(script, &mut head);
                }
                out.push_str(&rest[..len]);
                rest = &rest[len..];
                continue;
            }
            // A script element: `src` decides. Whatever is inside is skipped
            // either way, so code is never read as markup.
            let end = rest[len..].find("</script").and_then(|e| rest[len + e..].find('>').map(|g| len + e + g + 1));
            let Some(end) = end else {
                truncated = true;
                break;
            };
            let src = attr(&attrs, "src").map(|s| s.replace("&amp;", "&")).unwrap_or_default();
            // Classic scripts that wait (`defer`, `async`) cannot do so inline.
            let waits = attrs.iter().any(|(n, _)| n.eq_ignore_ascii_case("defer") || n.eq_ignore_ascii_case("async"));
            let module = attr(&attrs, "type").is_some_and(|t| t.eq_ignore_ascii_case("module"));
            if src.starts_with("/_next-rust/runtime.js") && !self.wasm_runtime {
                self.framework(0, &mut out);
            } else if src.starts_with("/_next-rust/ui.js") && !self.wasm_ui {
                self.framework(1, &mut out);
            } else if let Some(script) = Some(&src)
                .filter(|s| s.starts_with("/_next-rust/") && (module || !waits))
                .and_then(|s| self.find(&next_rust_view::script_key(s)))
                .filter(|s| module || s.kind == InlineKind::Plain)
            {
                self.entry(script, &attrs, &mut out);
            } else {
                out.push_str(&rest[..end]);
            }
            rest = &rest[end..];
        }
        if !truncated {
            note_head(&mut head_at, &out, rest);
        }
        out.push_str(rest);
        if !head.is_empty() {
            match head_at {
                Some(at) => out.insert_str(at, &head),
                None => out.insert_str(before_island, &head),
            }
        }
        out
    }
}

impl Inliner {
    /// Add `chunk` of a streamed body to `carry` and rewrite what is
    /// complete: only whole characters and whole tags are rewritten, the
    /// rest waits in `carry`. `None` when nothing is complete yet.
    fn feed(&mut self, carry: &mut Vec<u8>, chunk: &[u8]) -> Option<String> {
        carry.extend_from_slice(chunk);
        let valid = match std::str::from_utf8(carry) {
            Ok(text) => text.len(),
            Err(e) => e.valid_up_to(),
        };
        let text = std::str::from_utf8(&carry[..valid]).unwrap_or_default();
        let cut = hold_back(text);
        if cut == 0 {
            return None;
        }
        let out = self.process(&text[..cut]);
        carry.drain(..cut);
        Some(out)
    }

    /// The end of a streamed body: what is left in `carry`.
    fn finish(&mut self, carry: Vec<u8>) -> Vec<u8> {
        match String::from_utf8(carry) {
            Ok(text) => self.process(&text).into_bytes(),
            Err(e) => e.into_bytes(),
        }
    }
}

/// Where the last tag of `buf` starts when it is not complete yet: a
/// `<script>` without its end tag, or a start tag without its `>`.
fn hold_back(buf: &str) -> usize {
    let lower_script = buf.rfind("<script");
    if let Some(at) = lower_script
        && !buf[at..].contains("</script>")
    {
        return at;
    }
    match buf.rfind('<') {
        Some(at) if !buf[at..].contains('>') => at,
        _ => buf.len(),
    }
}

/// Write the scripts of an HTML response into it.
pub(crate) fn apply(inner: &AppInner, res: &mut Response, nonce: &str, known: Option<Known>) {
    let Some(known) = known else { return };
    // Only HTML as text: a body that is already compressed is sent as it is.
    if !res.header("content-type").is_some_and(|c| c.starts_with("text/html"))
        || res.header("content-encoding").is_some()
    {
        return;
    }
    let mut inliner = Inliner {
        table: inner.routes.inline_scripts,
        framework: framework_scripts(inner),
        nonce: nonce.to_owned(),
        known,
        wasm_runtime: inner.config.client.runtime.is_wasm(),
        wasm_ui: inner.config.ui.script.is_wasm(),
        sent: HashSet::new(),
    };
    // What a response holds depends on what the browser said it has.
    if inliner.known.nav {
        let vary = match res.header("vary") {
            Some(v) if !v.is_empty() => format!("{v}, x-nr-nav, x-nr-scripts"),
            _ => "x-nr-nav, x-nr-scripts".to_owned(),
        };
        res.set_header("vary", &vary);
        // `Vary` is not enough: several CDNs ignore it, and this document
        // lacks scripts a full page load needs. Keep it out of shared caches.
        res.set_header("cache-control", "private, no-cache, no-store, max-age=0, must-revalidate");
    }
    match std::mem::replace(&mut res.body, Body::Empty) {
        Body::Bytes(bytes) => {
            res.body = match std::str::from_utf8(&bytes) {
                Ok(html) => Body::Bytes(Bytes::from(inliner.process(html))),
                Err(_) => Body::Bytes(bytes),
            };
            res.headers.remove("content-length");
        }
        Body::Stream(stream) => {
            res.headers.remove("content-length");
            struct State {
                inner: crate::response::ByteStream,
                inliner: Inliner,
                carry: Vec<u8>,
                done: bool,
            }
            let state = State { inner: stream, inliner, carry: Vec::new(), done: false };
            res.body = Body::Stream(Box::pin(futures_util::stream::unfold(state, |mut s| async move {
                if s.done {
                    return None;
                }
                loop {
                    match s.inner.next().await {
                        Some(Ok(chunk)) => match s.inliner.feed(&mut s.carry, &chunk) {
                            Some(out) => return Some((Ok(Bytes::from(out)), s)),
                            None => continue,
                        },
                        Some(Err(e)) => {
                            s.done = true;
                            return Some((Err(e), s));
                        }
                        None => {
                            s.done = true;
                            if s.carry.is_empty() {
                                return None;
                            }
                            let out = Bytes::from(s.inliner.finish(std::mem::take(&mut s.carry)));
                            return Some((Ok(out), s));
                        }
                    }
                }
            })));
        }
        Body::Empty => {}
    }
}

/// `/_next-rust/m/<id>`: a lazy module, cached for good under its hash.
pub(crate) fn lazy_module(inner: &AppInner, req: &Request) -> Option<Response> {
    let id = req.path().strip_prefix("/_next-rust/m/")?;
    let script = inner.routes.inline_scripts.iter().find(|s| s.kind == InlineKind::Lazy && s.id == id)?;
    Some(crate::internal::script_bytes_response(req, script.id, script.code, "public, max-age=31536000, immutable"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &[InlineScript] = &[
        InlineScript { key: "assets/js/site.js", id: "aaaa", kind: InlineKind::Plain, code: "site()", deps: &[] },
        InlineScript {
            key: "client/answers.js",
            id: "bbbb",
            kind: InlineKind::Factory,
            code: "async()=>({})",
            deps: &[],
        },
        InlineScript {
            key: "client/practice.js",
            id: "cccc",
            kind: InlineKind::Factory,
            code: "async(i)=>{await i(\"client/answers.js\");return{}}",
            deps: &["client/answers.js"],
        },
        InlineScript { key: "client/vendor/big.js", id: "dddd", kind: InlineKind::Lazy, code: "export{}", deps: &[] },
    ];

    fn framework() -> &'static [(String, String); 2] {
        static F: OnceLock<[(String, String); 2]> = OnceLock::new();
        F.get_or_init(|| [("rt-1".into(), "runtime()".into()), ("ui-1".into(), "ui()".into())])
    }

    fn inliner(nav: bool, ids: &[&str]) -> Inliner {
        Inliner {
            table: TABLE,
            framework: framework(),
            nonce: "n0".into(),
            known: Known { nav, ids: ids.iter().map(|s| (*s).to_owned()).collect() },
            wasm_runtime: false,
            wasm_ui: false,
            sent: HashSet::new(),
        }
    }

    const PAGE: &str = "<!DOCTYPE html><html><head><title>t</title></head><body><nr-island data-component=\"P\" data-props=\"{&quot;a&quot;:1}\" data-module=\"/_next-rust/client/practice.js\"><p>x</p></nr-island><nr-island data-module=\"/_next-rust/client/practice.js\"></nr-island><script type=\"module\" src=\"/_next-rust/assets/js/site.0123456789abcdef.js\" nonce=\"n0\"></script><script type=\"module\" src=\"/_next-rust/runtime.js?v=1\" nonce=\"n0\"></script><script type=\"module\" src=\"/_next-rust/ui.js?v=1\" nonce=\"n0\"></script></body></html>";

    #[test]
    fn a_document_carries_its_scripts() {
        let html = inliner(false, &[]).process(PAGE);
        assert!(!html.contains(" src="), "{html}");
        // Imports are registered before what imports them, once, in <head>.
        let answers = html.find("r.f[\"client/answers.js\"]||").unwrap();
        let practice = html.find("r.f[\"client/practice.js\"]||").unwrap();
        assert!(answers < practice && practice < html.find("</head>").unwrap(), "{html}");
        assert_eq!(html.matches("data-nr-js=\"cccc\"").count(), 1);
        assert!(html.contains("<script data-nr-js=\"bbbb\" nonce=\"n0\">{const r=window.__nrm||"));
        assert!(html.contains("<script type=\"module\" nonce=\"n0\" data-nr-js=\"aaaa\">site()</script>"), "{html}");
        assert!(html.contains("<script type=\"module\" data-nr-js=\"rt-1\" nonce=\"n0\">runtime()</script>"));
        assert!(html.contains("<script type=\"module\" data-nr-js=\"ui-1\" nonce=\"n0\">ui()</script>"));
        // Islands keep their module: the runtime looks it up in the registry.
        assert!(html.contains("data-module=\"/_next-rust/client/practice.js\""));
    }

    /// `[client] runtime = "wasm"` is the one script inlining leaves alone.
    ///
    /// Inlining it would be wrong twice over: the prepared copy is the
    /// JavaScript runtime, not the shim this page asked for, so the page would
    /// end up running both; and the `<script>` element carries
    /// `data-nr-shim`, which audits use to tell the interpreter apart from
    /// application code and which cannot survive being turned into an inline
    /// script. Everything else on the page is still inlined.
    #[test]
    fn the_wasm_shim_is_never_written_into_the_page() {
        let mut inliner = inliner(false, &[]);
        inliner.wasm_runtime = true;
        let html = inliner.process(PAGE);
        assert!(
            html.contains("<script type=\"module\" src=\"/_next-rust/runtime.js?v=1\" nonce=\"n0\"></script>"),
            "the shim keeps its tag, its URL and its attributes: {html}"
        );
        assert!(!html.contains("runtime()"), "the JavaScript runtime must not be inlined beside it: {html}");
        assert!(!html.contains("data-nr-js=\"rt-1\""), "{html}");
        // The rest of the page is inlined exactly as before.
        assert!(html.contains("<script type=\"module\" nonce=\"n0\" data-nr-js=\"aaaa\">site()</script>"), "{html}");
        assert!(html.contains("<script type=\"module\" data-nr-js=\"ui-1\" nonce=\"n0\">ui()</script>"), "{html}");
        assert_eq!(html.matches("/_next-rust/runtime.js").count(), 1, "exactly one runtime reference: {html}");
    }

    /// `[ui] script = "wasm"` leaves `/_next-rust/ui.js` alone for the same
    /// two reasons.
    ///
    /// On that setting the file is the boot module, not the components'
    /// behaviour: inlining it would write the JavaScript component script
    /// into a page whose components live in the wasm module, and would strip
    /// the `data-nr-shim` marking off the tag on the way. The page would then
    /// report ~16 kB of shim as application JavaScript that it does not have.
    #[test]
    fn the_wasm_ui_boot_module_is_never_written_into_the_page() {
        let mut inliner = inliner(false, &[]);
        inliner.wasm_ui = true;
        let html = inliner.process(PAGE);
        assert!(
            html.contains("<script type=\"module\" src=\"/_next-rust/ui.js?v=1\" nonce=\"n0\"></script>"),
            "the boot module keeps its tag and its attributes: {html}"
        );
        assert!(!html.contains("ui()"), "the JavaScript component script must not be inlined beside it: {html}");
        assert!(!html.contains("data-nr-js=\"ui-1\""), "{html}");
        // The runtime and the rest of the page are inlined exactly as before.
        assert!(html.contains("<script type=\"module\" data-nr-js=\"rt-1\" nonce=\"n0\">runtime()</script>"), "{html}");
        assert!(html.contains("<script type=\"module\" nonce=\"n0\" data-nr-js=\"aaaa\">site()</script>"), "{html}");
    }

    /// A module key comes from a `data-module` attribute, so the registry
    /// tables must not inherit from `Object.prototype`: `constructor` or
    /// `__proto__` would otherwise be found instead of missing.
    #[test]
    fn the_registry_tables_have_no_prototype() {
        assert!(REGISTRY.contains("f:Object.create(null),p:Object.create(null)"), "{REGISTRY}");
        assert!(!REGISTRY.contains("f:{}"), "{REGISTRY}");
        let html = inliner(false, &[]).process(PAGE);
        assert!(html.contains("window.__nrm={f:Object.create(null),p:Object.create(null),"), "{html}");
    }

    #[test]
    fn navigations_get_only_what_the_browser_lacks() {
        let html = inliner(true, &["bbbb", "aaaa", "ui-1"]).process(PAGE);
        assert!(!html.contains("runtime()") && !html.contains("ui()") && !html.contains("site()"), "{html}");
        assert!(!html.contains("data-nr-js=\"bbbb\"") && html.contains("data-nr-js=\"cccc\""), "{html}");
    }

    #[test]
    fn other_scripts_are_left_alone() {
        let html = "<script src=\"/app.js\"></script><script>if (a < b) { x = \"<nr-island data-module='/_next-rust/client/practice.js'>\" }</script><script defer src=\"/_next-rust/assets/js/site.js\"></script><script src=\"/_next-rust/client/vendor/big.js\"></script><script src=\"/_next-rust/client/practice.js\"></script>";
        assert_eq!(inliner(false, &[]).process(html), html);
        // A classic script without imports is written as it is, every time.
        let classic = "<script src=\"/_next-rust/assets/js/site.js\" nonce=\"n\"></script>";
        assert_eq!(
            inliner(true, &["aaaa"]).process(classic),
            "<script nonce=\"n\" data-nr-js=\"aaaa\" data-nr-rerun>site()</script>"
        );
        // A module entry with imports registers them, then starts itself.
        let entry = "<p></p><script type=module src=\"/_next-rust/client/practice.js\"></script>";
        let out = inliner(false, &[]).process(entry);
        assert!(out.starts_with("<p></p><script data-nr-js=\"bbbb\""), "{out}");
        assert!(
            out.ends_with("r.load(\"client/practice.js\");</script>") && out.contains("data-nr-js=\"cccc-run\""),
            "{out}"
        );
        // Written without a nonce: it gets the one of this response.
        assert!(out.contains("<script type=\"module\" nonce=\"n0\" data-nr-js=\"cccc-run\">"), "{out}");
    }

    /// A file name is not code: a script key holding a quote, a bracket or a
    /// closing tag stays inside the JavaScript string it is written into,
    /// whether it is reached as an import (`define`) or as the entry itself.
    #[test]
    fn script_keys_cannot_break_out_of_their_string() {
        const EXEC: &str = "client/x\"];alert(1);r.f[\"y.js";
        const TAG: &str = "client/z</script><img src=x onerror=alert(2)>.js";
        const HOSTILE: &[InlineScript] = &[
            InlineScript {
                key: "client/app.js",
                id: "a1",
                kind: InlineKind::Factory,
                code: "async()=>({})",
                deps: &[EXEC, TAG],
            },
            InlineScript { key: EXEC, id: "a2", kind: InlineKind::Factory, code: "async()=>({})", deps: &[] },
            InlineScript { key: TAG, id: "a3", kind: InlineKind::Factory, code: "async()=>({})", deps: &[] },
        ];
        let mut inliner = inliner(false, &[]);
        inliner.table = HOSTILE;
        let out = inliner.process(
            "<html><head></head><body><nr-island data-module=\"/_next-rust/client/app.js\"></nr-island>\
             <script type=\"module\" src='/_next-rust/client/x\"];alert(1);r.f[\"y.js'></script></body></html>",
        );
        // Three registrations and the entry: four elements, four end tags.
        assert_eq!(out.matches("<script").count(), 4, "{out}");
        assert_eq!(out.matches("</script>").count(), 4, "{out}");
        assert!(!out.contains("</script><img src=x"), "the key ended its element: {out}");
        assert!(out.contains(r#"r.f["client/x\"];alert(1);r.f[\"y.js"]"#), "{out}");
        assert!(out.contains(r#"r.f["client/z<\/script><img src=x onerror=alert(2)>.js"]"#), "{out}");
        // The entry starts itself under the same escaped key.
        assert!(out.contains(r#"r.load("client/x\"];alert(1);r.f[\"y.js")"#), "{out}");
    }

    /// `</head>` inside the code of a script is not the `</head>` of the
    /// document: registrations must not be inserted into JavaScript.
    #[test]
    fn head_registrations_never_land_in_script_code() {
        const TALKS: &[InlineScript] = &[
            InlineScript {
                key: "assets/js/site.js",
                id: "aaaa",
                kind: InlineKind::Plain,
                code: "t=\"</head>\"",
                deps: &[],
            },
            InlineScript {
                key: "client/answers.js",
                id: "bbbb",
                kind: InlineKind::Factory,
                code: "async()=>({})",
                deps: &[],
            },
        ];
        let mut chunked = inliner(false, &[]);
        chunked.table = TALKS;
        // A piece of a streamed body: the real `</head>` went out long ago.
        let out = chunked.process(
            "<script src=\"/_next-rust/assets/js/site.js\"></script>\
             <nr-island data-module=\"/_next-rust/client/answers.js\"></nr-island>",
        );
        assert!(out.contains("data-nr-rerun>t=\"</head>\"</script>"), "the code was cut open: {out}");
        assert_eq!(out.matches("<script").count(), 2, "{out}");
        assert!(out.find("data-nr-js=\"aaaa\"") < out.find("data-nr-js=\"bbbb\""), "{out}");
        // A `</head>` that is markup still wins, wherever the island is.
        let mut with_head = inliner(false, &[]);
        with_head.table = TALKS;
        let out = with_head.process(
            "<head><script src=\"/_next-rust/assets/js/site.js\"></script></head>\
             <body><nr-island data-module=\"/_next-rust/client/answers.js\"></nr-island></body>",
        );
        assert!(out.contains("data-nr-js=\"bbbb\" nonce=\"n0\">"), "{out}");
        assert!(out.find("data-nr-js=\"bbbb\"") < out.rfind("</head>"), "{out}");
    }

    /// The nonce reaches markup only through `escape_attr`, on every path.
    #[test]
    fn a_hostile_nonce_cannot_leave_its_attribute() {
        let mut inliner = inliner(false, &[]);
        inliner.nonce = "\"><script>alert(1)</script>".into();
        let out = inliner.process(
            "<nr-island data-module=\"/_next-rust/client/answers.js\"></nr-island>\
             <script type=\"module\" src=\"/_next-rust/assets/js/site.js\"></script>",
        );
        assert_eq!(out.matches("<script").count(), 2, "{out}");
        assert!(!out.contains("\"><script>alert(1)"), "{out}");
        assert_eq!(out.matches("nonce=\"&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;\"").count(), 2, "{out}");
    }

    #[test]
    fn incomplete_tags_wait_for_the_next_chunk() {
        assert_eq!(hold_back("<p>a</p><script src=\"x\">"), 8);
        assert_eq!(hold_back("<p>a</p><nr-island data-mod"), 8);
        assert_eq!(hold_back("<p>a</p><script>1</script>"), 26);
        assert_eq!(start_tag("<script a b=\"1>2\" c='3' d=4>").unwrap().0.len(), 4);
        assert!(start_tag("<script a=\"").is_none());
    }

    /// Visitor-controlled text and attribute values (escaped, as the renderer
    /// writes them) around real scripts and islands, streamed with a chunk
    /// boundary at every byte: the rewriter never panics, never loses or
    /// alters that text, and never makes a script out of it.
    #[test]
    fn hostile_text_and_every_chunk_boundary() {
        let hostile = [
            "<script src=\"/_next-rust/runtime.js\">",
            "</script><script>alert(1)</script>",
            "<script src=/_next-rust/assets/js/site.js a=onmouseover=alert(1)//>",
            "<nr-island data-module=\"/_next-rust/client/practice.js\">",
            "\"><script",
            "<scr",
            "<",
            "'\"=/>",
            "𝒳é\u{202e}<script\u{0}",
            "</head>",
            "<!--",
            "--><script>",
        ];
        for (n, evil) in hostile.iter().enumerate() {
            let text = next_rust_view::escape_text(evil);
            let value = next_rust_view::escape_attr(evil);
            let page = format!(
                "<!DOCTYPE html><html><head><title>{text}</title></head><body><p title=\"{value}\">{text}</p>\
                 <nr-island data-props=\"{value}\" data-module=\"/_next-rust/client/answers.js\">{text}</nr-island>\
                 <script type=\"application/json\">{{\"q\":\"\\u003cscript\"}}</script><textarea>{text}</textarea>\
                 <script type=\"module\" src=\"/_next-rust/runtime.js?v=1\" nonce=\"n0\"></script><i>{text}</i></body></html>"
            );
            let whole = inliner(false, &[]).process(&page);
            // The island's module, the JSON block and the runtime: nothing else.
            assert_eq!(whole.matches("<script").count(), 3, "case {n}: {whole}");
            assert_eq!(whole.matches(&*text).count(), page.matches(&*text).count(), "case {n}: text survives");
            assert_eq!(whole.matches(&*value).count(), page.matches(&*value).count(), "case {n}: values survive");

            let bytes = page.as_bytes();
            for cut in 0..=bytes.len() {
                let mut inliner = inliner(false, &[]);
                let mut carry = Vec::new();
                let mut out = Vec::new();
                for chunk in [&bytes[..cut], &bytes[cut..]] {
                    out.extend(inliner.feed(&mut carry, chunk).unwrap_or_default().into_bytes());
                }
                out.extend(inliner.finish(carry));
                let out = String::from_utf8(out).expect("chunks are cut at character boundaries");
                assert_eq!(out.matches("<script").count(), 3, "case {n}, cut {cut}: {out}");
                assert_eq!(out.matches("data-nr-js=\"rt-1\"").count(), 1, "case {n}, cut {cut}: {out}");
                assert_eq!(out.matches("data-nr-js=\"bbbb\"").count(), 1, "case {n}, cut {cut}: {out}");
                assert!(!out.contains("<script type=\"module\" src="), "case {n}, cut {cut}: {out}");
                assert_eq!(out.matches(&*text).count(), page.matches(&*text).count(), "case {n}, cut {cut}: {out}");
            }
        }
    }
}
