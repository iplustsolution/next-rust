//! HTML rendering: synchronous, async (fully resolved) and streaming.
//!
//! ## Streaming protocol
//!
//! When a [`Suspense`](crate::Suspense) boundary is encountered while
//! streaming, the renderer writes
//!
//! ```html
//! <nr-b id="nr-b1">…fallback…</nr-b>
//! ```
//!
//! and continues with the rest of the document. The *shell* (everything that
//! does not wait) is flushed immediately. Each boundary resolves concurrently;
//! as soon as one finishes, a chunk is sent:
//!
//! ```html
//! <template id="nr-t1">…content…</template><script>$nr(1)</script>
//! ```
//!
//! `$nr` (≈150 bytes, inlined once in `<head>` only when a page actually
//! suspends) moves the template content into place. Nested boundaries inside
//! streamed content work the same way. Without JavaScript the fallback stays
//! visible, so pages that must work without JS should avoid suspense or be
//! rendered non-streaming (bots and static generation always are).

use std::borrow::Cow;
use std::collections::HashSet;
use std::pin::Pin;

use futures_util::future::join_all;
use futures_util::stream::{FuturesUnordered, Stream, StreamExt};

use crate::class_names::{map_class_list, map_raw_html};
use crate::css_split::{self, Pieces};
use crate::escape::{escape_attr, escape_raw_text, escape_text, is_safe_url, is_url_attr, is_valid_attr_name};
use crate::node::{AttrValue, BoxNodeFuture, Element, Node, View};
use crate::style::Stylesheet;

/// Inline script used to swap streamed content into place.
pub const STREAMING_RUNTIME: &str = "function $nr(i){var t=document.getElementById(\"nr-t\"+i),b=document.getElementById(\"nr-b\"+i);if(t&&b){b.replaceWith(t.content);t.remove()}}";

type PendingBoundary = Pin<Box<dyn std::future::Future<Output = (u32, Node)> + Send>>;

/// Features used by a rendered document; the server uses these to decide
/// which (if any) client scripts to include.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RenderFlags {
    /// Something needing the navigation runtime was rendered: a `Link!`, a
    /// same-origin anchor, or a form posting to a server action.
    pub links: bool,
    /// An interactive island was rendered.
    pub islands: bool,
    /// At least one suspense boundary was streamed.
    pub streamed: bool,
    /// An interactive UI component was rendered (`data-nr-ui`): the page
    /// needs the component script.
    pub ui: bool,
}

struct Writer {
    out: String,
    styles_seen: HashSet<&'static str>,
    /// Stylesheet ids the browser already has (partial navigations).
    styles_known: HashSet<String>,
    /// When set, stylesheets are collected here instead of written inline.
    hoisted: Option<Vec<&'static Stylesheet>>,
    /// Documents send per-class stylesheets rule by rule (see `css_split`).
    split: bool,
    /// Per-class stylesheets in the document, with the pieces the browser has.
    split_sheets: Vec<(&'static Stylesheet, Pieces)>,
    /// Classes rendered so far, as written to the HTML.
    classes: HashSet<String>,
    /// Scripts the page loads so far (`client/home.js`, `assets/js/site.js`).
    scripts: HashSet<String>,
    flags: RenderFlags,
    streaming: bool,
    next_id: u32,
    pending: Vec<(u32, BoxNodeFuture)>,
}

impl Writer {
    fn new(streaming: bool, hoist: bool) -> Self {
        Writer {
            out: String::with_capacity(4096),
            styles_seen: HashSet::new(),
            styles_known: HashSet::new(),
            hoisted: hoist.then(Vec::new),
            split: false,
            split_sheets: Vec::new(),
            classes: HashSet::new(),
            scripts: HashSet::new(),
            flags: RenderFlags::default(),
            streaming,
            next_id: 0,
            pending: Vec::new(),
        }
    }

    fn style(&mut self, sheet: &'static Stylesheet) {
        let sheet = crate::style::resolve(sheet);
        if self.split && sheet.per_class.is_some() {
            if !self.split_sheets.iter().any(|(s, _)| std::ptr::eq(*s, sheet)) {
                let known = css_split::split(sheet).known(sheet, &self.styles_known);
                self.split_sheets.push((sheet, known));
            }
            return;
        }
        if !self.styles_seen.insert(sheet.id) || self.styles_known.contains(sheet.id) {
            return;
        }
        match &mut self.hoisted {
            Some(list) => list.push(sheet),
            None => write_style(&mut self.out, sheet),
        }
    }

    fn node(&mut self, node: Node, raw_text: bool) {
        match node {
            Node::Empty => {}
            Node::Text(t) => {
                if raw_text {
                    self.out.push_str(&escape_raw_text(&t));
                } else {
                    self.out.push_str(&escape_text(&t));
                }
            }
            Node::Raw(r) => {
                let html = map_raw_html(&r);
                if self.split {
                    collect_raw_classes(&html, &mut self.classes);
                }
                self.out.push_str(&html);
            }
            Node::Fragment(children) => {
                for c in children {
                    self.node(c, raw_text);
                }
            }
            Node::Style(sheet) => self.style(sheet),
            Node::Element(el) => self.element(*el),
            Node::Suspense(s) => {
                if self.streaming {
                    self.next_id += 1;
                    let id = self.next_id;
                    self.flags.streamed = true;
                    self.out.push_str(&format!("<nr-b id=\"nr-b{id}\">"));
                    self.node(s.fallback, false);
                    self.out.push_str("</nr-b>");
                    self.pending.push((id, s.content));
                } else {
                    // Unresolved suspense in a synchronous render: show the fallback.
                    self.node(s.fallback, raw_text);
                }
            }
        }
    }

    fn element(&mut self, el: Element) {
        for a in &el.attrs {
            for sheet in &a.sheets {
                self.style(sheet);
            }
        }
        let tag = el.tag;
        self.out.push('<');
        self.out.push_str(&tag);
        for a in &el.attrs {
            let Some(value) = &a.value else { continue };
            if !is_valid_attr_name(&a.name) {
                continue;
            }
            if !a.trusted && a.name.len() > 2 && a.name[..2].eq_ignore_ascii_case("on") {
                // Event handler attributes from `attr()` are dropped: use `raw_attr`.
                continue;
            }
            match value {
                AttrValue::Bool(true) => {
                    self.out.push(' ');
                    self.out.push_str(&attr_name(&a.name));
                }
                AttrValue::Bool(false) => {}
                AttrValue::Text(t) => {
                    let t = match a.name.as_ref() {
                        "class" | "data-nr-active" | "data-nr-active-prefix" => {
                            let mapped = map_class_list(t);
                            if self.split {
                                self.classes.extend(mapped.split_ascii_whitespace().map(str::to_owned));
                            }
                            mapped
                        }
                        _ => Cow::Borrowed(t.as_ref()),
                    };
                    if a.name == "class" && t.is_empty() {
                        // Every class was dropped: no attribute at all.
                        continue;
                    }
                    let safe = if is_url_attr(&a.name) && !is_safe_url(&t) { "#" } else { t.as_ref() };
                    let name = attr_name(&a.name);
                    if self.split {
                        if let Some(class) = name.strip_prefix("data-nr-class-") {
                            self.classes.insert(class.to_owned());
                        }
                        if (tag == "nr-island" && a.name == "data-module") || (tag == "script" && a.name == "src") {
                            self.scripts.insert(script_key(&t));
                        }
                    }
                    self.out.push(' ');
                    self.out.push_str(&name);
                    self.out.push_str("=\"");
                    self.out.push_str(&escape_attr(safe));
                    self.out.push('"');
                }
            }
            // `Link!`, plain same-origin anchors and action forms all need the
            // client runtime: the first two for navigation, the last so the
            // form submits without a full page load.
            if a.name == "data-nr-ui" {
                self.flags.ui = true;
            }
            if a.name == "data-nr-link"
                || a.name == "data-nr-action"
                || (tag == "a" && a.name == "href" && is_internal_href(value))
            {
                self.flags.links = true;
            }
        }
        self.out.push('>');
        if tag == "nr-island" {
            self.flags.islands = true;
        }
        if el.void {
            return;
        }
        let raw = tag.eq_ignore_ascii_case("script") || tag.eq_ignore_ascii_case("style");
        for c in el.children {
            self.node(c, raw);
        }
        self.out.push_str("</");
        self.out.push_str(&tag);
        self.out.push('>');
    }
}

impl Writer {
    /// `<style>` elements with the per-class rules needed by what was
    /// rendered since the last call and not yet sent.
    fn split_styles(&mut self) -> String {
        let mut out = String::new();
        for (sheet, sent) in &mut self.split_sheets {
            let split = css_split::split(sheet);
            let scripts = &self.scripts;
            let scoped = sheet.scripts.iter().filter(|(k, _)| scripts.contains(*k)).flat_map(|(_, c)| c.iter());
            let keep: Vec<&str> = sheet
                .per_class
                .unwrap_or_default()
                .iter()
                .chain(scoped)
                .map(|c| crate::class_names::short_class_name(c).unwrap_or(c))
                .collect();
            let classes = &self.classes;
            let used = |c: &str| classes.contains(c) || keep.contains(&c);
            let pieces = split.delta(&used, sent);
            out.push_str(&split.style(sheet, &pieces));
            split.record(sent, &pieces);
        }
        out
    }
}

/// The key of a script URL in [`Stylesheet::scripts`]: the path under
/// `/_next-rust/` or `public/`, without query string or fingerprint
/// (`/_next-rust/assets/js/site.1a2b3c4d5e6f7a8b.js` → `assets/js/site.js`).
pub fn script_key(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let path = path.strip_prefix("/_next-rust/").or_else(|| path.strip_prefix('/')).unwrap_or(path);
    let (dir, file) = path.rsplit_once('/').map_or(("", path), |(d, f)| (d, f));
    let parts: Vec<&str> = file.split('.').collect();
    let mut name: Vec<&str> = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let fingerprint =
            i > 0 && i + 1 < parts.len() && part.len() == 16 && part.bytes().all(|b| b.is_ascii_hexdigit());
        if !fingerprint {
            name.push(part);
        }
    }
    let file = name.join(".");
    if dir.is_empty() { file } else { format!("{dir}/{file}") }
}

/// Class names in `class="…"` attributes of raw HTML.
fn collect_raw_classes(html: &str, into: &mut HashSet<String>) {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("class=") {
        let start = from + at + "class=".len();
        from = start;
        let Some(q @ (b'"' | b'\'')) = html.as_bytes().get(start).copied() else { continue };
        let Some(len) = html[start + 1..].find(q as char) else { break };
        into.extend(html[start + 1..start + 1 + len].split_ascii_whitespace().map(str::to_owned));
        from = start + 1 + len;
    }
}

/// An attribute name as written: `data-nr-class-<class>` toggles a class on
/// the client, so the class part gets its short name.
fn attr_name(name: &str) -> Cow<'_, str> {
    match name.strip_prefix("data-nr-class-").and_then(crate::class_names::short_class_name) {
        Some(short) => Cow::Owned(format!("data-nr-class-{short}")),
        None => Cow::Borrowed(name),
    }
}

fn write_style(out: &mut String, sheet: &Stylesheet) {
    out.push_str("<style data-nr-css=\"");
    out.push_str(sheet.id);
    out.push_str("\">");
    out.push_str(&escape_raw_text(sheet.css));
    out.push_str("</style>");
}

/// Render synchronously. Suspense boundaries render their fallback.
pub fn render_static(view: impl View) -> String {
    let mut w = Writer::new(false, false);
    w.node(view.into_node(), false);
    w.out
}

/// Resolve every suspense boundary in a tree (siblings concurrently).
pub fn resolve(node: Node) -> Pin<Box<dyn std::future::Future<Output = Node> + Send>> {
    Box::pin(async move {
        match node {
            Node::Suspense(s) => resolve(s.content.await).await,
            Node::Fragment(children) => Node::Fragment(join_all(children.into_iter().map(resolve)).await),
            Node::Element(mut el) => {
                let children = std::mem::take(&mut el.children);
                el.children = join_all(children.into_iter().map(resolve)).await;
                Node::Element(el)
            }
            other => other,
        }
    })
}

/// Render to a string, awaiting all async content.
pub async fn render_to_string(view: impl View) -> String {
    let node = resolve(view.into_node()).await;
    render_static(node)
}

/// Everything needed to render a full HTML document.
pub struct DocumentParts {
    pub lang: String,
    /// Other attributes of `<html>` (see `Metadata::html_attribute`). A
    /// `lang` entry replaces [`DocumentParts::lang`].
    pub html_attributes: Vec<(String, String)>,
    /// Pre-rendered `<head>` contents (metadata).
    pub head: String,
    /// Extra head markup (e.g. dev scripts). Trusted.
    pub head_extra: String,
    pub body: Node,
    /// CSP nonce applied to framework inline scripts.
    pub nonce: Option<String>,
    /// Called once the body is complete; returns markup appended before
    /// `</body>` (client runtime scripts, depending on the flags).
    pub tail: Box<dyn FnOnce(RenderFlags) -> String + Send>,
    /// Ids of stylesheets the browser already has; they are not written again.
    pub known_styles: Vec<String>,
    /// Written as `<meta name="generator">`, the first element of `<head>`:
    /// what built the page (`Next Rust 0.1.10`). `None` leaves it out.
    pub generator: Option<String>,
}

impl DocumentParts {
    pub fn new(body: impl View) -> Self {
        DocumentParts {
            lang: "en".into(),
            html_attributes: Vec::new(),
            head: String::new(),
            head_extra: String::new(),
            body: body.into_node(),
            nonce: None,
            tail: Box::new(|_| String::new()),
            known_styles: Vec::new(),
            generator: None,
        }
    }
}

fn nonce_attr(nonce: &Option<String>) -> String {
    nonce.as_ref().map(|n| format!(" nonce=\"{}\"", escape_attr(n))).unwrap_or_default()
}

fn open_document(parts: &DocumentParts, w: &mut Writer, body: &str) -> String {
    let mut s = String::with_capacity(body.len() + parts.head.len() + 256);
    let lang = parts.html_attributes.iter().find(|(name, _)| name == "lang").map_or(&parts.lang, |(_, v)| v);
    s.push_str("<!DOCTYPE html><html lang=\"");
    s.push_str(&escape_attr(lang));
    s.push('"');
    for (name, value) in &parts.html_attributes {
        let name = name.as_str();
        if name == "lang" || !is_valid_attr_name(name) || (name.len() > 2 && name[..2].eq_ignore_ascii_case("on")) {
            continue;
        }
        let value = if name == "class" {
            // Before `split_styles` below: the classes' rules are sent too.
            let mapped = map_class_list(value);
            w.classes.extend(mapped.split_ascii_whitespace().map(str::to_owned));
            mapped
        } else {
            Cow::Borrowed(value.as_str())
        };
        let value = if is_url_attr(name) && !is_safe_url(&value) { "#" } else { value.as_ref() };
        s.push(' ');
        s.push_str(&attr_name(name));
        s.push_str("=\"");
        s.push_str(&escape_attr(value));
        s.push('"');
    }
    s.push_str("><head>");
    if let Some(generator) = &parts.generator {
        s.push_str("<meta name=\"generator\" content=\"");
        s.push_str(&escape_attr(generator));
        s.push_str("\">");
    }
    s.push_str(&parts.head);
    // Utility sheets first, so page and module styles can override them.
    s.push_str(&w.split_styles());
    if let Some(sheets) = w.hoisted.take() {
        for sheet in sheets {
            write_style(&mut s, sheet);
        }
    }
    if !w.pending.is_empty() {
        s.push_str("<style>nr-b,nr-island{display:contents}</style>");
        s.push_str(&format!("<script{}>{STREAMING_RUNTIME}</script>", nonce_attr(&parts.nonce)));
    } else if w.flags.islands {
        s.push_str("<style>nr-island{display:contents}</style>");
    }
    s.push_str(&parts.head_extra);
    s.push_str("</head><body>");
    s.push_str(body);
    s
}

/// Render a complete document as a stream of chunks.
///
/// With `streaming = false`, all async content is resolved first and the
/// stream yields exactly one chunk (used for bots, static generation and
/// when streaming is disabled).
pub fn stream_document(parts: DocumentParts, streaming: bool) -> impl Stream<Item = String> + Send {
    enum State {
        Start(DocumentParts, bool),
        Streaming {
            pending: FuturesUnordered<PendingBoundary>,
            w: Writer,
            nonce: Option<String>,
            tail: Box<dyn FnOnce(RenderFlags) -> String + Send>,
        },
        Done,
    }

    futures_util::stream::unfold(State::Start(parts, streaming), |state| async move {
        match state {
            State::Done => None,
            State::Start(mut parts, streaming) => {
                let body = std::mem::take(&mut parts.body);
                let body = if streaming { body } else { resolve(body).await };
                let mut w = Writer::new(streaming, true);
                w.split = true;
                w.styles_known = std::mem::take(&mut parts.known_styles).into_iter().collect();
                w.node(body, false);
                let body_html = std::mem::take(&mut w.out);
                let mut chunk = open_document(&parts, &mut w, &body_html);
                if w.pending.is_empty() {
                    chunk.push_str(&(parts.tail)(w.flags));
                    chunk.push_str("</body></html>");
                    return Some((chunk, State::Done));
                }
                let pending = FuturesUnordered::new();
                for (id, fut) in w.pending.drain(..) {
                    pending.push(Box::pin(async move { (id, fut.await) })
                        as Pin<Box<dyn std::future::Future<Output = _> + Send>>);
                }
                // Styles in streamed chunks are written inline, deduplicated
                // against those already hoisted into <head>.
                w.hoisted = None;
                Some((chunk, State::Streaming { pending, w, nonce: parts.nonce, tail: parts.tail }))
            }
            State::Streaming { mut pending, mut w, nonce, tail } => match pending.next().await {
                Some((id, node)) => {
                    w.out.clear();
                    w.out.push_str(&format!("<template id=\"nr-t{id}\">"));
                    w.node(node, false);
                    w.out.push_str(&format!("</template><script{}>$nr({id})</script>", nonce_attr(&nonce)));
                    // Rules for classes first used in this chunk, ahead of it.
                    let styles = w.split_styles();
                    w.out.insert_str(0, &styles);
                    for (nid, fut) in w.pending.drain(..) {
                        pending.push(Box::pin(async move { (nid, fut.await) }));
                    }
                    let chunk = std::mem::take(&mut w.out);
                    Some((chunk, State::Streaming { pending, w, nonce, tail }))
                }
                None => {
                    let mut chunk = tail(w.flags);
                    chunk.push_str("</body></html>");
                    Some((chunk, State::Done))
                }
            },
        }
    })
}

/// A link the client runtime can navigate to without a full reload:
/// root-relative (`/about`), not protocol-relative (`//cdn`).
fn is_internal_href(value: &AttrValue) -> bool {
    matches!(value, AttrValue::Text(href) if href.starts_with('/') && !href.starts_with("//"))
}
