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

use std::collections::HashSet;
use std::pin::Pin;

use futures_util::future::join_all;
use futures_util::stream::{FuturesUnordered, Stream, StreamExt};

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
    /// A `Link!` with client navigation was rendered.
    pub links: bool,
    /// An interactive island was rendered.
    pub islands: bool,
    /// At least one suspense boundary was streamed.
    pub streamed: bool,
}

struct Writer {
    out: String,
    styles_seen: HashSet<&'static str>,
    /// When set, stylesheets are collected here instead of written inline.
    hoisted: Option<Vec<&'static Stylesheet>>,
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
            hoisted: hoist.then(Vec::new),
            flags: RenderFlags::default(),
            streaming,
            next_id: 0,
            pending: Vec::new(),
        }
    }

    fn style(&mut self, sheet: &'static Stylesheet) {
        if !self.styles_seen.insert(sheet.id) {
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
            Node::Raw(r) => self.out.push_str(&r),
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
                    self.out.push_str(&a.name);
                }
                AttrValue::Bool(false) => {}
                AttrValue::Text(t) => {
                    let safe = if is_url_attr(&a.name) && !is_safe_url(t) { "#" } else { t.as_ref() };
                    self.out.push(' ');
                    self.out.push_str(&a.name);
                    self.out.push_str("=\"");
                    self.out.push_str(&escape_attr(safe));
                    self.out.push('"');
                }
            }
            if a.name == "data-nr-link" {
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
}

impl DocumentParts {
    pub fn new(body: impl View) -> Self {
        DocumentParts {
            lang: "en".into(),
            head: String::new(),
            head_extra: String::new(),
            body: body.into_node(),
            nonce: None,
            tail: Box::new(|_| String::new()),
        }
    }
}

fn nonce_attr(nonce: &Option<String>) -> String {
    nonce.as_ref().map(|n| format!(" nonce=\"{}\"", escape_attr(n))).unwrap_or_default()
}

fn open_document(parts: &DocumentParts, w: &mut Writer, body: &str) -> String {
    let mut s = String::with_capacity(body.len() + parts.head.len() + 256);
    s.push_str("<!DOCTYPE html><html lang=\"");
    s.push_str(&escape_attr(&parts.lang));
    s.push_str("\"><head>");
    s.push_str(&parts.head);
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
