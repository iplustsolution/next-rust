//! The Next Rust view layer.
//!
//! ```
//! use next_rust_view::*;
//!
//! fn card(title: &str) -> impl View {
//!     div![class("card"), h2![title], p!["Escaped by default: <b>"]]
//! }
//!
//! let html = render_static(card("Hello"));
//! assert_eq!(html, r#"<div class="card"><h2>Hello</h2><p>Escaped by default: &lt;b&gt;</p></div>"#);
//! ```
//!
//! Design:
//!
//! * Element macros (`div!`, `p!`, ...) build an [`Element`] from a list of
//!   *parts*. A part is either an attribute ([`Attr`]) or a child view.
//! * Everything that can be rendered implements [`View`], which converts it
//!   into a [`Node`] tree. Components are plain functions returning `impl View`.
//! * All text and attribute values are escaped. Raw HTML requires
//!   [`raw_html`], which is intentionally conspicuous.
//! * Async content is expressed with [`suspense`] and can be streamed.

#![forbid(unsafe_code)]

pub mod attrs;
pub mod class_names;
pub mod components;
mod escape;
pub mod metadata;
mod node;
mod render;
pub mod style;
mod tags;

pub use attrs::*;
pub use components::{Children, ImageProps, LinkProps, Slots, each, fragment_of, island, raw_html, suspense, when};
pub use escape::{escape_attr, escape_text, is_safe_url};
pub use metadata::Metadata;
pub use node::{Attr, AttrValue, Element, IntoViewResult, Node, Part, Suspense, View};
pub use render::{
    DocumentParts, RenderFlags, STREAMING_RUNTIME, render_static, render_to_string, resolve, stream_document,
};
pub use style::{CssClass, Stylesheet};
pub use tags::{TAGS, VOID_TAGS};

/// Group children without a wrapper element: `fragment![a, b, c]`.
#[macro_export]
macro_rules! fragment {
    ($($child:expr),* $(,)?) => {
        $crate::Node::Fragment(vec![$($crate::View::into_node($child)),*])
    };
}

/// Client-side-navigation aware link.
///
/// ```
/// use next_rust_view::*;
/// let html = render_static(Link!(href = "/about", class = "nav", "About"));
/// assert_eq!(html, r#"<a href="/about" class="nav" data-nr-link="">About</a>"#);
/// ```
#[macro_export]
macro_rules! Link {
    ($($body:tt)*) => { $crate::__link_munch!([$crate::Element::new("a")] $($body)*) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __link_munch {
    ([$($acc:tt)*]) => { $($acc)*.with($crate::attrs::data("nr-link", "")) };
    ([$($acc:tt)*] $key:ident = $val:expr $(, $($rest:tt)*)?) => {
        $crate::__link_munch!([$($acc)*.with($crate::attrs::$key($val))] $($($rest)*)?)
    };
    ([$($acc:tt)*] $child:expr $(, $($rest:tt)*)?) => {
        $crate::__link_munch!([$($acc)*.with($child)] $($($rest)*)?)
    };
}

/// Responsive, lazily loaded image.
///
/// ```
/// use next_rust_view::*;
/// let html = render_static(Image!(src = "/cat.jpg", width = 640, height = 480, alt = "A cat"));
/// assert!(html.starts_with(r#"<img src="/_nr/image?url=%2Fcat.jpg&amp;w=640&amp;q=75""#));
/// assert!(html.contains(r#"loading="lazy""#));
/// ```
#[macro_export]
macro_rules! Image {
    ($($key:ident = $val:expr),* $(,)?) => {
        $crate::ImageProps::default()$(.$key($val))*
    };
}
