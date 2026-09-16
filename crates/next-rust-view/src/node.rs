use std::borrow::Cow;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use crate::style::Stylesheet;

/// A rendered view tree.
#[derive(Default)]
pub enum Node {
    #[default]
    Empty,
    /// Escaped text.
    Text(Cow<'static, str>),
    /// Unescaped HTML created with [`crate::raw_html`].
    Raw(Cow<'static, str>),
    Element(Box<Element>),
    Fragment(Vec<Node>),
    /// Async boundary (see [`crate::suspense`]).
    Suspense(Box<Suspense>),
    /// A stylesheet used by this subtree; emitted once per document.
    Style(&'static Stylesheet),
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Empty => f.write_str("Empty"),
            Node::Text(t) => f.debug_tuple("Text").field(t).finish(),
            Node::Raw(t) => f.debug_tuple("Raw").field(t).finish(),
            Node::Element(e) => e.fmt(f),
            Node::Fragment(c) => f.debug_tuple("Fragment").field(c).finish(),
            Node::Suspense(_) => f.write_str("Suspense(..)"),
            Node::Style(s) => f.debug_tuple("Style").field(&s.id).finish(),
        }
    }
}

pub type BoxNodeFuture = Pin<Box<dyn Future<Output = Node> + Send + 'static>>;

/// An async boundary: `fallback` is rendered immediately, `content` replaces
/// it once resolved (in place when not streaming, out-of-order when streaming).
pub struct Suspense {
    pub fallback: Node,
    pub content: BoxNodeFuture,
}

/// Attribute value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    Text(Cow<'static, str>),
    /// Boolean attribute: rendered as `name` when true, omitted when false.
    Bool(bool),
}

/// An attribute. Create with the helpers in [`crate::attrs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub name: Cow<'static, str>,
    pub value: Option<AttrValue>,
    /// Allows event-handler attributes (`on*`). Only set by `raw_attr`.
    pub(crate) trusted: bool,
    pub(crate) sheets: Vec<&'static Stylesheet>,
}

impl Attr {
    pub fn new(name: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>) -> Self {
        Attr { name: name.into(), value: Some(AttrValue::Text(value.into())), trusted: false, sheets: Vec::new() }
    }

    pub fn boolean(name: impl Into<Cow<'static, str>>, on: bool) -> Self {
        Attr { name: name.into(), value: Some(AttrValue::Bool(on)), trusted: false, sheets: Vec::new() }
    }

    /// An attribute that renders nothing (useful for conditionals).
    pub fn none() -> Self {
        Attr { name: Cow::Borrowed(""), value: None, trusted: false, sheets: Vec::new() }
    }
}

/// An HTML element.
#[derive(Debug)]
pub struct Element {
    pub tag: Cow<'static, str>,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
    pub void: bool,
}

impl Element {
    /// A normal element with a trusted, static tag name.
    pub fn new(tag: &'static str) -> Self {
        Element { tag: Cow::Borrowed(tag), attrs: Vec::new(), children: Vec::new(), void: false }
    }

    /// A void element (`<img>`, `<input>`, ...).
    pub fn new_void(tag: &'static str) -> Self {
        Element { tag: Cow::Borrowed(tag), attrs: Vec::new(), children: Vec::new(), void: true }
    }

    /// An element with a runtime tag name. Invalid names render as `<div>`
    /// with a `data-invalid-tag` attribute rather than producing broken HTML.
    pub fn custom(tag: impl Into<String>) -> Self {
        let tag: String = tag.into();
        if crate::escape::is_valid_tag_name(&tag) {
            Element { tag: Cow::Owned(tag), attrs: Vec::new(), children: Vec::new(), void: false }
        } else {
            Element::new("div").with(Attr::new("data-invalid-tag", tag))
        }
    }

    /// Add an attribute or child.
    pub fn with<P: Part>(mut self, part: P) -> Self {
        part.apply(&mut self);
        self
    }

    /// Add a child.
    pub fn child(mut self, child: impl View) -> Self {
        self.children.push(child.into_node());
        self
    }

    /// Set or merge an attribute. `class` values are appended; other
    /// attributes are replaced.
    pub fn set_attr(&mut self, attr: Attr) {
        if attr.value.is_none() && attr.sheets.is_empty() {
            return;
        }
        if let Some(existing) = self.attrs.iter_mut().find(|a| a.name == attr.name) {
            if attr.name == "class"
                && let (Some(AttrValue::Text(old)), Some(AttrValue::Text(new))) = (&existing.value, &attr.value)
            {
                let merged = if old.is_empty() {
                    new.clone()
                } else if new.is_empty() {
                    old.clone()
                } else {
                    Cow::Owned(format!("{old} {new}"))
                };
                existing.value = Some(AttrValue::Text(merged));
                existing.sheets.extend(attr.sheets);
                return;
            }
            *existing = attr;
        } else {
            self.attrs.push(attr);
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.name == name).and_then(|a| match &a.value {
            Some(AttrValue::Text(t)) => Some(t.as_ref()),
            _ => None,
        })
    }
}

/// Something that can be placed inside an element macro.
pub trait Part {
    fn apply(self, el: &mut Element);
}

impl Part for Attr {
    fn apply(self, el: &mut Element) {
        el.set_attr(self);
    }
}

impl Part for Vec<Attr> {
    fn apply(self, el: &mut Element) {
        for a in self {
            el.set_attr(a);
        }
    }
}

impl<V: View> Part for V {
    fn apply(self, el: &mut Element) {
        el.children.push(self.into_node());
    }
}

/// Anything renderable.
pub trait View {
    fn into_node(self) -> Node;
}

impl View for Node {
    fn into_node(self) -> Node {
        self
    }
}

impl View for Element {
    fn into_node(self) -> Node {
        Node::Element(Box::new(self))
    }
}

impl View for &str {
    fn into_node(self) -> Node {
        Node::Text(Cow::Owned(self.to_owned()))
    }
}

impl View for &String {
    fn into_node(self) -> Node {
        Node::Text(Cow::Owned(self.clone()))
    }
}

impl View for String {
    fn into_node(self) -> Node {
        Node::Text(Cow::Owned(self))
    }
}

impl View for Cow<'static, str> {
    fn into_node(self) -> Node {
        Node::Text(self)
    }
}

impl View for char {
    fn into_node(self) -> Node {
        Node::Text(Cow::Owned(self.to_string()))
    }
}

macro_rules! display_views {
    ($($t:ty),*) => {$(
        impl View for $t {
            fn into_node(self) -> Node {
                Node::Text(Cow::Owned(self.to_string()))
            }
        }
    )*};
}
display_views!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl<V: View> View for Option<V> {
    fn into_node(self) -> Node {
        match self {
            Some(v) => v.into_node(),
            None => Node::Empty,
        }
    }
}

impl<V: View> View for Vec<V> {
    fn into_node(self) -> Node {
        Node::Fragment(self.into_iter().map(View::into_node).collect())
    }
}

impl<V: View, const N: usize> View for [V; N] {
    fn into_node(self) -> Node {
        Node::Fragment(self.into_iter().map(View::into_node).collect())
    }
}

impl<V: View> View for Box<V> {
    fn into_node(self) -> Node {
        (*self).into_node()
    }
}

impl View for () {
    fn into_node(self) -> Node {
        Node::Empty
    }
}

macro_rules! tuple_views {
    ($($name:ident),+) => {
        impl<$($name: View),+> View for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_node(self) -> Node {
                let ($($name,)+) = self;
                Node::Fragment(vec![$($name.into_node()),+])
            }
        }
    };
}
tuple_views!(A);
tuple_views!(A, B);
tuple_views!(A, B, C);
tuple_views!(A, B, C, D);
tuple_views!(A, B, C, D, E);
tuple_views!(A, B, C, D, E, F);
tuple_views!(A, B, C, D, E, F, G);
tuple_views!(A, B, C, D, E, F, G, H);

/// Conversion of a component's return value into a render result: accepts
/// `impl View` as well as `Result<impl View, E>`. Used by generated code.
pub trait IntoViewResult<E> {
    fn into_view_result(self) -> Result<Node, E>;
}

impl<V: View, E> IntoViewResult<E> for V {
    fn into_view_result(self) -> Result<Node, E> {
        Ok(self.into_node())
    }
}

impl<V: View, E, E2: Into<E>> IntoViewResult<E> for Result<V, E2> {
    fn into_view_result(self) -> Result<Node, E> {
        self.map(View::into_node).map_err(Into::into)
    }
}
