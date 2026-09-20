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
//!
//! ## Shells and holes (partial prerendering)
//!
//! A [`Hole`](crate::Hole) is a suspense boundary whose content is a function
//! of the request. [`render_shell`] renders a document once, with every hole
//! showing its fallback inside `<nr-b id="nr-bN">`, and stops before the
//! document tail; the returned [`Shell`] remembers the holes and what the
//! document has sent so far (styles, classes, ids). [`resume_document`] then
//! serves any number of requests from it: the shell bytes go out first, every
//! hole runs concurrently with that request's context, and each result is
//! streamed with the same `<template>` + `$nr(id)` chunk as above. With
//! `streaming = false` the holes are awaited and written in place instead, so
//! clients without JavaScript (bots) get complete HTML.

use std::borrow::Cow;
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::future::join_all;
use futures_util::stream::{FuturesUnordered, Stream, StreamExt};

use crate::class_names::{map_class_list, map_raw_html};
use crate::css_split::{self, Pieces};
use crate::escape::{escape_attr, escape_raw_text, escape_text, is_safe_url, is_url_attr, is_valid_attr_name};
use crate::node::{AttrValue, BoxNodeFuture, Element, HoleContext, HoleFn, Node, View};
use crate::style::Stylesheet;

/// Inline script used to swap streamed content into place.
/// Swaps a streamed boundary in; inside a view transition when the motion
/// stylesheet sets `--nr-page-fade`, so content fades in instead of popping.
pub const STREAMING_RUNTIME: &str = "function $nr(i){var t=document.getElementById(\"nr-t\"+i),b=document.getElementById(\"nr-b\"+i);if(t&&b){var s=function(){b.replaceWith(t.content);t.remove()};document.startViewTransition&&parseFloat(getComputedStyle(document.documentElement).getPropertyValue(\"--nr-page-fade\"))?document.startViewTransition(s):s()}}";

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
    /// A `<form>` was rendered: the navigation runtime sends it without a
    /// full page load (`[client] enhance_forms`).
    pub forms: bool,
}

impl RenderFlags {
    fn merge(&mut self, other: RenderFlags) {
        self.links |= other.links;
        self.islands |= other.islands;
        self.streamed |= other.streamed;
        self.ui |= other.ui;
        self.forms |= other.forms;
    }
}

/// Markup rendered ahead of the document it appears in, together with what
/// the document must know about it: the stylesheets it uses, its classes and
/// scripts (for per-class CSS) and the client features it needs. Made by
/// [`render_fragment`]; place it in a tree as [`Node::Rendered`].
#[derive(Debug, Clone, Default)]
pub struct Rendered {
    /// Final HTML: escaped, class names already shortened.
    pub html: String,
    pub sheets: Vec<&'static Stylesheet>,
    pub classes: Vec<String>,
    pub scripts: Vec<String>,
    pub flags: RenderFlags,
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
    /// What holes are rendered with; without it they show their fallback.
    hole_cx: Option<HoleContext>,
    /// Rendering a shell: holes are recorded in `holes`, not run.
    shell: bool,
    holes: Vec<(u32, HoleFn)>,
    /// Inside the fallback of a recorded hole (holes in there are not recorded).
    in_hole_fallback: bool,
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
            hole_cx: None,
            shell: false,
            holes: Vec::new(),
            in_hole_fallback: false,
        }
    }

    fn boundary(&mut self, fallback: Node) -> u32 {
        self.next_id += 1;
        let id = self.next_id;
        self.flags.streamed = true;
        self.out.push_str(&format!("<nr-b id=\"nr-b{id}\">"));
        self.node(fallback, false);
        self.out.push_str("</nr-b>");
        id
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
                    if self.split {
                        // An inline script or style: the names it uses.
                        css_split::note_text(&t, &mut self.classes);
                    }
                    self.out.push_str(&escape_raw_text(&t));
                } else {
                    self.out.push_str(&escape_text(&t));
                }
            }
            Node::Raw(r) => {
                let html = map_raw_html(&r);
                if self.split && raw_text {
                    // `script![raw_html(…)]`: an inline script or style.
                    css_split::note_text(&html, &mut self.classes);
                } else if self.split {
                    collect_raw_classes(&html, &mut self.classes);
                    css_split::note_html(&html, &mut self.classes);
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
                    let id = self.boundary(s.fallback);
                    self.pending.push((id, s.content));
                } else {
                    // Unresolved suspense in a synchronous render: show the fallback.
                    self.node(s.fallback, raw_text);
                }
            }
            Node::Hole(h) => {
                if self.shell && !self.in_hole_fallback {
                    self.in_hole_fallback = true;
                    let id = self.boundary(h.fallback);
                    self.in_hole_fallback = false;
                    self.holes.push((id, h.content));
                } else if let (true, Some(cx)) = (self.streaming, self.hole_cx.clone()) {
                    let id = self.boundary(h.fallback);
                    self.pending.push((id, (h.content)(cx)));
                } else {
                    self.node(h.fallback, raw_text);
                }
            }
            Node::Rendered(r) => {
                for sheet in &r.sheets {
                    self.style(sheet);
                }
                if self.split {
                    self.classes.extend(r.classes.iter().cloned());
                    self.scripts.extend(r.scripts.iter().cloned());
                    // Read from the markup, so a fragment cached by an older
                    // build says what it contains too.
                    css_split::note_html(&r.html, &mut self.classes);
                }
                self.flags.merge(r.flags);
                self.out.push_str(&r.html);
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
        if self.split {
            // What selectors can match: the element and its attributes.
            css_split::note_tag(&tag, &mut self.classes);
            for a in &el.attrs {
                match &a.value {
                    Some(AttrValue::Text(t)) => css_split::note_attr(&a.name, Some(t), &mut self.classes),
                    Some(AttrValue::Bool(true)) => css_split::note_attr(&a.name, None, &mut self.classes),
                    _ => {}
                }
            }
        }
        self.out.push('<');
        self.out.push_str(&tag);
        for a in &el.attrs {
            let Some(value) = &a.value else { continue };
            if !is_valid_attr_name(&a.name) {
                continue;
            }
            if !a.trusted && a.name.len() > 2 && a.name.as_bytes()[..2].eq_ignore_ascii_case(b"on") {
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
        } else if tag == "form" {
            self.flags.forms = true;
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
        // One sheet may define what another reads (`--nr-fg` in the app's
        // CSS, read by the UI components' sheet): those names are kept.
        let all: Vec<&'static Stylesheet> = self.split_sheets.iter().map(|(s, _)| *s).collect();
        for (sheet, sent) in &mut self.split_sheets {
            let split = css_split::split(sheet);
            let elsewhere = all
                .iter()
                .filter(|other| !std::ptr::eq(**other, *sheet))
                .flat_map(|other| css_split::split(other).reads());
            let scripts = &self.scripts;
            // A sheet without a list of its own goes by the app's.
            let listed = css_split::script_names(sheet);
            let scoped =
                listed.unwrap_or_default().iter().filter(|(k, _)| scripts.contains(*k)).flat_map(|(_, c)| c.iter());
            let scoped_names = scoped.clone();
            let keep: Vec<&str> = sheet
                .per_class
                .unwrap_or_default()
                .iter()
                .chain(scoped)
                .map(|c| crate::class_names::short_class_name(c).unwrap_or(c))
                .collect();
            // Names the scripts list count as written too (an attribute or a
            // custom property is never renamed).
            let names = sheet.per_class.unwrap_or_default().iter().chain(scoped_names.clone()).copied();
            let page = css_split::Page::new(&self.classes, keep.into_iter().chain(names).chain(elsewhere))
                .with_unknown_scripts(listed.is_none() && !scripts.is_empty());
            let pieces = split.delta(&page, sent);
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

/// Resolve every suspense boundary in a tree (siblings concurrently). Holes
/// are left in place: they need a context (see [`resolve_with`]).
pub fn resolve(node: Node) -> Pin<Box<dyn std::future::Future<Output = Node> + Send>> {
    resolve_with(node, None)
}

/// Like [`resolve`]; with a context, holes are rendered with it and resolved too.
pub fn resolve_with(node: Node, cx: Option<HoleContext>) -> Pin<Box<dyn std::future::Future<Output = Node> + Send>> {
    Box::pin(async move {
        match node {
            Node::Suspense(s) => resolve_with(s.content.await, cx).await,
            Node::Hole(h) => match cx {
                Some(cx) => resolve_with((h.content)(cx.clone()).await, Some(cx)).await,
                None => Node::Hole(h),
            },
            Node::Fragment(children) => {
                Node::Fragment(join_all(children.into_iter().map(|c| resolve_with(c, cx.clone()))).await)
            }
            Node::Element(mut el) => {
                let children = std::mem::take(&mut el.children);
                el.children = join_all(children.into_iter().map(|c| resolve_with(c, cx.clone()))).await;
                Node::Element(el)
            }
            other => other,
        }
    })
}

/// Render a view ahead of time, for [`Node::Rendered`]: suspense boundaries
/// are awaited, holes show their fallback (there is no request to run them
/// for; put the hole around the fragment instead).
pub async fn render_fragment(view: impl View) -> Rendered {
    let node = resolve(view.into_node()).await;
    let mut w = Writer::new(false, true);
    w.split = true;
    w.node(node, false);
    let mut sheets = w.hoisted.take().unwrap_or_default();
    sheets.extend(w.split_sheets.iter().map(|(s, _)| *s));
    // Without the facts recorded next to them (they start with a space).
    let mut classes: Vec<String> = w.classes.into_iter().filter(|c| !c.starts_with(' ')).collect();
    classes.sort();
    let mut scripts: Vec<String> = w.scripts.into_iter().collect();
    scripts.sort();
    Rendered { html: w.out, sheets, classes, scripts, flags: w.flags }
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
    /// Markup written right after `<body>` (e.g. a tag manager's
    /// `<noscript>` fallback). Trusted.
    pub body_start: String,
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
    /// What holes are rendered with. `None`: holes show their fallback
    /// ([`stream_document`]) or are recorded ([`render_shell`]).
    pub hole_context: Option<HoleContext>,
}

impl DocumentParts {
    pub fn new(body: impl View) -> Self {
        DocumentParts {
            lang: "en".into(),
            html_attributes: Vec::new(),
            head: String::new(),
            head_extra: String::new(),
            body_start: String::new(),
            body: body.into_node(),
            nonce: None,
            tail: Box::new(|_| String::new()),
            known_styles: Vec::new(),
            generator: None,
            hole_context: None,
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
        if name == "lang"
            || !is_valid_attr_name(name)
            || (name.len() > 2 && name.as_bytes()[..2].eq_ignore_ascii_case(b"on"))
        {
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
        css_split::note_attr(name, Some(&value), &mut w.classes);
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
    // Inline scripts and styles of the head name attributes and custom properties.
    css_split::note_html(&parts.head, &mut w.classes);
    css_split::note_html(&parts.head_extra, &mut w.classes);
    // Utility sheets first, so page and module styles can override them.
    s.push_str(&w.split_styles());
    if let Some(sheets) = w.hoisted.take() {
        for sheet in sheets {
            write_style(&mut s, sheet);
        }
    }
    if !w.pending.is_empty() || !w.holes.is_empty() {
        s.push_str("<style>nr-b,nr-island{display:contents}</style>");
        s.push_str(&format!("<script{}>{STREAMING_RUNTIME}</script>", nonce_attr(&parts.nonce)));
    } else if w.flags.islands {
        s.push_str("<style>nr-island{display:contents}</style>");
    }
    s.push_str(&parts.head_extra);
    s.push_str("</head><body>");
    s.push_str(&parts.body_start);
    s.push_str(body);
    s
}

type Tail = Box<dyn FnOnce(RenderFlags) -> String + Send>;

enum State {
    Start(DocumentParts, bool),
    Resume(Box<Resume>),
    Streaming { pending: FuturesUnordered<PendingBoundary>, w: Writer, nonce: Option<String>, tail: Tail },
    Done,
}

fn boundary_future(id: u32, fut: BoxNodeFuture) -> PendingBoundary {
    Box::pin(async move { (id, fut.await) })
}

/// Render a complete document as a stream of chunks.
///
/// With `streaming = false`, all async content is resolved first and the
/// stream yields exactly one chunk (used for bots, static generation and
/// when streaming is disabled).
pub fn stream_document(parts: DocumentParts, streaming: bool) -> impl Stream<Item = String> + Send {
    drive(State::Start(parts, streaming))
}

/// Serve one request from a [`Shell`]: the shell first, then its holes.
pub fn resume_document(resume: Resume) -> impl Stream<Item = String> + Send {
    drive(State::Resume(Box::new(resume)))
}

fn drive(state: State) -> impl Stream<Item = String> + Send {
    futures_util::stream::unfold(state, |state| async move {
        match state {
            State::Done => None,
            State::Start(mut parts, streaming) => {
                let body = std::mem::take(&mut parts.body);
                let body = if streaming { body } else { resolve_with(body, parts.hole_context.clone()).await };
                let mut w = Writer::new(streaming, true);
                w.split = true;
                w.hole_cx = parts.hole_context.take();
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
                    pending.push(boundary_future(id, fut));
                }
                // Styles in streamed chunks are written inline, deduplicated
                // against those already hoisted into <head>.
                w.hoisted = None;
                Some((chunk, State::Streaming { pending, w, nonce: parts.nonce, tail: parts.tail }))
            }
            State::Resume(resume) => {
                let Resume { html, shell, context, nonce, tail, streaming, known_styles } = *resume;
                let mut w = shell.writer(streaming, context.clone(), known_styles);
                let holes = shell.holes.iter().filter(|(id, _)| html.contains(&boundary_open(*id)));
                if !streaming {
                    // In order: every hole is awaited and written where its fallback was.
                    let nodes = join_all(holes.map(|(id, f)| {
                        let (id, cx) = (*id, context.clone());
                        async move { (id, resolve_with(f(cx.clone()).await, Some(cx)).await) }
                    }))
                    .await;
                    let mut html = html;
                    for (id, node) in nodes {
                        w.out.clear();
                        w.node(node, false);
                        let mut content = w.split_styles();
                        content.push_str(&w.out);
                        replace_boundary(&mut html, id, &content);
                    }
                    html.push_str(&tail(w.flags));
                    html.push_str("</body></html>");
                    return Some((html, State::Done));
                }
                let pending = FuturesUnordered::new();
                for (id, f) in holes {
                    pending.push(boundary_future(*id, f(context.clone())));
                }
                Some((html, State::Streaming { pending, w, nonce, tail }))
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
                        pending.push(boundary_future(nid, fut));
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

fn boundary_open(id: u32) -> String {
    format!("<nr-b id=\"nr-b{id}\">")
}

/// Replace `<nr-b id="nr-b{id}">…</nr-b>` with `content`. The fallback of a
/// recorded hole never contains another boundary, so the first close tag ends it.
fn replace_boundary(html: &mut String, id: u32, content: &str) {
    let open = boundary_open(id);
    let Some(start) = html.find(&open) else { return };
    let Some(len) = html[start..].find("</nr-b>") else { return };
    html.replace_range(start..start + len + "</nr-b>".len(), content);
}

/// A document rendered up to its holes, and what is needed to finish it for
/// a request: see [`render_shell`] and [`resume_document`].
pub struct Shell {
    holes: Vec<(u32, HoleFn)>,
    styles_seen: HashSet<&'static str>,
    split_sheets: Vec<(&'static Stylesheet, Pieces)>,
    classes: HashSet<String>,
    scripts: HashSet<String>,
    flags: RenderFlags,
    next_id: u32,
}

impl std::fmt::Debug for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shell").field("holes", &self.holes.len()).finish()
    }
}

impl Shell {
    /// Number of holes in the shell.
    pub fn holes(&self) -> usize {
        self.holes.len()
    }

    /// A writer that continues where the shell stopped.
    fn writer(&self, streaming: bool, cx: HoleContext, known_styles: Vec<String>) -> Writer {
        let mut w = Writer::new(streaming, false);
        w.split = true;
        w.hole_cx = Some(cx);
        w.styles_seen = self.styles_seen.clone();
        w.styles_known = known_styles.into_iter().collect();
        w.split_sheets = self
            .split_sheets
            .iter()
            .map(|(sheet, sent)| {
                let mut sent = sent.clone();
                if !w.styles_known.is_empty() {
                    sent.union(&css_split::split(sheet).known(sheet, &w.styles_known));
                }
                (*sheet, sent)
            })
            .collect();
        w.classes = self.classes.clone();
        w.scripts = self.scripts.clone();
        w.flags = self.flags;
        w.next_id = self.next_id;
        w
    }
}

/// One request served from a [`Shell`].
pub struct Resume {
    /// The shell's HTML, as it goes out for this request (nonce filled in,
    /// possibly cut down to a region). Holes whose `<nr-b>` is not in it are
    /// not run.
    pub html: String,
    pub shell: Arc<Shell>,
    pub context: HoleContext,
    /// CSP nonce for the swap scripts.
    pub nonce: Option<String>,
    /// Like [`DocumentParts::tail`]: a shell ends before it.
    pub tail: Box<dyn FnOnce(RenderFlags) -> String + Send>,
    /// `false`: holes are awaited and written in place (one chunk, no script).
    pub streaming: bool,
    /// Ids of stylesheets the browser already has.
    pub known_styles: Vec<String>,
}

/// Render a document once for many requests. Suspense boundaries are awaited;
/// holes are left open.
///
/// Without holes the result is the complete document and `None`. With holes
/// it is the document up to the end of the body content (no tail, no
/// `</body></html>`) and the [`Shell`] to [resume](resume_document) it with.
pub async fn render_shell(mut parts: DocumentParts) -> (String, Option<Shell>) {
    let body = resolve(std::mem::take(&mut parts.body)).await;
    let mut w = Writer::new(false, true);
    w.split = true;
    w.shell = true;
    w.styles_known = std::mem::take(&mut parts.known_styles).into_iter().collect();
    w.node(body, false);
    let body_html = std::mem::take(&mut w.out);
    let mut html = open_document(&parts, &mut w, &body_html);
    if w.holes.is_empty() {
        html.push_str(&(parts.tail)(w.flags));
        html.push_str("</body></html>");
        return (html, None);
    }
    let shell = Shell {
        holes: std::mem::take(&mut w.holes),
        styles_seen: w.styles_seen,
        split_sheets: w.split_sheets,
        classes: w.classes,
        scripts: w.scripts,
        flags: w.flags,
        next_id: w.next_id,
    };
    (html, Some(shell))
}

/// A link the client runtime can navigate to without a full reload:
/// root-relative (`/about`), not protocol-relative (`//cdn`).
fn is_internal_href(value: &AttrValue) -> bool {
    matches!(value, AttrValue::Text(href) if href.starts_with('/') && !href.starts_with("//"))
}
