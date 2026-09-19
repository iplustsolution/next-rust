//! Built-in components and composition helpers.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::future::Future;

use crate::attrs::{self, AttrText};
use crate::node::{Element, Node, Suspense, View};

/// Content passed to a layout or template.
#[derive(Debug, Default)]
pub struct Children(pub Node);

impl Children {
    pub fn new(node: impl View) -> Self {
        Children(node.into_node())
    }
}

impl View for Children {
    fn into_node(self) -> Node {
        self.0
    }
}

/// Parallel-route slots passed to a layout (`@analytics` → `slots.take("analytics")`).
#[derive(Debug, Default)]
pub struct Slots(BTreeMap<String, Node>);

impl Slots {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, node: impl View) {
        self.0.insert(name.into(), node.into_node());
    }

    /// Take a slot's content (empty if the slot has nothing to render).
    pub fn take(&mut self, name: &str) -> Node {
        self.0.remove(name).unwrap_or(Node::Empty)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// Unescaped HTML. **Never pass untrusted input.**
pub fn raw_html(html: impl Into<Cow<'static, str>>) -> Node {
    Node::Raw(html.into())
}

/// Render `f()` only when `condition` is true.
pub fn when<V: View>(condition: bool, f: impl FnOnce() -> V) -> Node {
    if condition { f().into_node() } else { Node::Empty }
}

/// Render every item of an iterator.
///
/// ```
/// use next_rust_view::*;
/// let list = ul![each(["a", "b"], |x| li![key(x), x])];
/// assert_eq!(render_static(list), r#"<ul><li data-nr-key="a">a</li><li data-nr-key="b">b</li></ul>"#);
/// ```
pub fn each<I, V>(items: I, f: impl FnMut(I::Item) -> V) -> Node
where
    I: IntoIterator,
    V: View,
{
    Node::Fragment(items.into_iter().map(f).map(View::into_node).collect())
}

/// Collect any iterator of views into a fragment.
pub fn fragment_of<V: View>(items: impl IntoIterator<Item = V>) -> Node {
    Node::Fragment(items.into_iter().map(View::into_node).collect())
}

/// An async boundary. `fallback` renders immediately; `content` is awaited.
///
/// With streaming enabled the fallback is sent first and the content is
/// streamed later in the same HTTP response; otherwise the renderer waits.
pub fn suspense<V, F>(fallback: impl View, content: F) -> Node
where
    F: Future<Output = V> + Send + 'static,
    V: View + 'static,
{
    Node::Suspense(Box::new(Suspense {
        fallback: fallback.into_node(),
        content: Box::pin(async move { content.await.into_node() }),
    }))
}

/// Wrap server-rendered markup in an island marker so the client runtime can
/// hydrate it. Used by the `#[client]` macro; `props_json` must be JSON.
pub fn island(component: &str, props_json: String, ssr: impl View) -> Node {
    Element::new("nr-island")
        .with(attrs::data("component", component))
        .with(attrs::data("props", props_json))
        .with(ssr)
        .into_node()
}

/// Props for `Link!`; the macro uses attribute helpers directly, this type
/// exists for programmatic construction.
#[derive(Debug, Default, Clone)]
pub struct LinkProps {
    pub href: String,
    pub class: Option<String>,
    pub prefetch: bool,
    pub replace: bool,
}

/// Default responsive widths for [`ImageProps`] `srcset` generation.
pub const DEFAULT_IMAGE_WIDTHS: &[u32] = &[640, 750, 828, 1080, 1200, 1920, 2048, 3840];

/// Builder behind `Image!`.
#[derive(Debug, Clone)]
pub struct ImageProps {
    src: String,
    alt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    sizes: Option<String>,
    quality: u8,
    priority: bool,
    class: Option<String>,
    unoptimized: bool,
}

impl Default for ImageProps {
    fn default() -> Self {
        Self {
            src: String::new(),
            alt: None,
            width: None,
            height: None,
            sizes: None,
            quality: 75,
            priority: false,
            class: None,
            unoptimized: false,
        }
    }
}

impl ImageProps {
    pub fn src(mut self, v: impl Into<String>) -> Self {
        self.src = v.into();
        self
    }
    pub fn alt(mut self, v: impl Into<String>) -> Self {
        self.alt = Some(v.into());
        self
    }
    pub fn width(mut self, v: u32) -> Self {
        self.width = Some(v);
        self
    }
    pub fn height(mut self, v: u32) -> Self {
        self.height = Some(v);
        self
    }
    /// `sizes` attribute, e.g. `"(max-width: 768px) 100vw, 50vw"`.
    pub fn sizes(mut self, v: impl Into<String>) -> Self {
        self.sizes = Some(v.into());
        self
    }
    pub fn quality(mut self, v: u8) -> Self {
        self.quality = v.clamp(1, 100);
        self
    }
    /// Above-the-fold image: eager loading and high fetch priority.
    pub fn priority(mut self, v: bool) -> Self {
        self.priority = v;
        self
    }
    pub fn class(mut self, v: impl Into<String>) -> Self {
        self.class = Some(v.into());
        self
    }
    /// Serve `src` as-is (no resizing endpoint).
    pub fn unoptimized(mut self, v: bool) -> Self {
        self.unoptimized = v;
        self
    }

    fn url(&self, w: u32) -> String {
        format!("/_next-rust/image?url={}&w={w}&q={}", encode_query(&self.src), self.quality)
    }
}

fn encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl View for ImageProps {
    fn into_node(self) -> Node {
        // Only local images go through the optimizer; remote URLs are served
        // unchanged (the optimizer never fetches remote content: no SSRF).
        let local = self.src.starts_with('/') && !self.src.starts_with("//");
        let mut el = Element::new_void("img");
        if self.unoptimized || !local {
            el = el.with(attrs::src(self.src.clone()));
        } else {
            let base_w = self.width.unwrap_or(1080);
            el = el.with(attrs::src(self.url(base_w)));
            let mut widths: Vec<u32> = DEFAULT_IMAGE_WIDTHS.iter().copied().filter(|w| *w < base_w).collect();
            widths.push(base_w);
            let set: Vec<String> = widths.iter().map(|w| format!("{} {w}w", self.url(*w))).collect();
            el = el.with(attrs::srcset(set.join(", ")));
            el = el.with(attrs::sizes(
                self.sizes.clone().unwrap_or_else(|| format!("(max-width: {base_w}px) 100vw, {base_w}px")),
            ));
        }
        // `alt` is required for accessibility; an empty alt marks decorative images.
        el = el.with(attrs::alt(self.alt.clone().unwrap_or_default()));
        if let Some(w) = self.width {
            el = el.with(attrs::width(w));
        }
        if let Some(h) = self.height {
            el = el.with(attrs::height(h));
        }
        if self.priority {
            el = el.with(attrs::fetchpriority("high"));
        } else {
            el = el.with(attrs::loading("lazy"));
        }
        el = el.with(attrs::decoding("async"));
        if let Some(c) = self.class {
            el = el.with(attrs::class(c));
        }
        if self.alt.is_none() && cfg!(debug_assertions) {
            el = el.with(attrs::data("nr-warning", "missing alt text".into_attr_text()));
        }
        el.into_node()
    }
}
