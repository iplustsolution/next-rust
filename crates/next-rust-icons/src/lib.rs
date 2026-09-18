//! Every [Lucide](https://lucide.dev) icon as a Next Rust component.
//!
//! ```
//! use next_rust_icons as icons;
//! use next_rust_view::*;
//!
//! let html = render_static(div![
//!     icons::House(),
//!     icons::ArrowRight().size(16).color("#e11d48").stroke_width(1.5),
//!     icons::Search().class("text-zinc-500").title("Search"),
//! ]);
//! assert!(html.contains(r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#e11d48" stroke-width="1.5""##));
//! ```
//!
//! Each icon is a function named like on lucide.dev in PascalCase
//! (`arrow-right` → [`ArrowRight`]) returning an [`Icon`](struct@Icon). Everything about
//! it can be changed:
//!
//! | method | default | |
//! |---|---|---|
//! | [`size`](Icon::size) | `24` | width and height: a number of pixels or any CSS length (`"1.25em"`) |
//! | [`color`](Icon::color) | `currentColor` | the stroke color: follows the text color unless set |
//! | [`stroke_width`](Icon::stroke_width) | `2` | line thickness |
//! | [`absolute_stroke_width`](Icon::absolute_stroke_width) | `false` | keep the thickness in pixels when resizing |
//! | [`fill`](Icon::fill) | `none` | fill color |
//! | [`title`](Icon::title) | none | accessible name; without one the icon is hidden from screen readers |
//! | [`class`](Icon::class) | `lucide lucide-<name>` | extra classes (Tailwind works) |
//! | [`with`](Icon::with) | | any attribute: `aria(..)`, `data(..)`, `style(..)`, ... |
//!
//! Icons render as inline `<svg>` and add nothing to pages that don't use
//! them; unused icons are left out of the binary. [`by_name`] finds an icon
//! from a string (that one keeps every icon in the binary).
//!
//! The artwork is Lucide's, under the ISC license (`LICENSE-LUCIDE`).

#![forbid(unsafe_code)]

use next_rust_view::{Attr, Element, Node, Part, View, raw_html};

#[allow(non_snake_case)]
mod generated;

pub use generated::*;

/// The Lucide release the icons come from.
pub const LUCIDE_VERSION: &str = generated::LUCIDE_VERSION;

/// The drawing of one icon.
#[derive(Debug, PartialEq, Eq)]
pub struct IconData {
    /// Lucide name: `arrow-right`.
    pub name: &'static str,
    /// SVG elements inside the 24×24 view box.
    pub body: &'static str,
}

/// An icon to render. Created by the icon functions ([`House`], ...) or
/// [`by_name`]; every method is optional.
#[derive(Debug)]
pub struct Icon {
    data: &'static IconData,
    size: Option<String>,
    color: Option<String>,
    stroke_width: Option<f32>,
    absolute_stroke_width: bool,
    fill: Option<String>,
    title: Option<String>,
    unstyled: bool,
    extra: Element,
}

impl Icon {
    pub fn new(data: &'static IconData) -> Self {
        Icon {
            data,
            size: None,
            color: None,
            stroke_width: None,
            absolute_stroke_width: false,
            fill: None,
            title: None,
            unstyled: false,
            extra: Element::new("svg"),
        }
    }

    /// Width and height: pixels (`20`, `1.5`) or a CSS length (`"1.25em"`, `"100%"`).
    pub fn size(mut self, v: impl ToString) -> Self {
        self.size = Some(v.to_string());
        self
    }

    /// Stroke color (`"#e11d48"`, `"var(--brand)"`). Defaults to `currentColor`,
    /// so the icon takes the text color and CSS classes like `text-red-500` work.
    pub fn color(mut self, v: impl Into<String>) -> Self {
        self.color = Some(v.into());
        self
    }

    /// Line thickness, in units of the 24×24 drawing (default 2).
    pub fn stroke_width(mut self, v: f32) -> Self {
        self.stroke_width = Some(v);
        self
    }

    /// Keep the line thickness constant in pixels when the icon is resized.
    pub fn absolute_stroke_width(mut self, v: bool) -> Self {
        self.absolute_stroke_width = v;
        self
    }

    /// Fill color (default `none`).
    pub fn fill(mut self, v: impl Into<String>) -> Self {
        self.fill = Some(v.into());
        self
    }

    /// Accessible name, also shown as a tooltip. Icons without one are
    /// decorative and hidden from screen readers.
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }

    /// Extra classes, merged with `lucide lucide-<name>`.
    pub fn class(self, v: impl Into<String>) -> Self {
        self.with(next_rust_view::attrs::class(v.into()))
    }

    /// Leave out the default `lucide lucide-<name>` classes.
    pub fn unstyled(mut self, v: bool) -> Self {
        self.unstyled = v;
        self
    }

    /// Any attribute (`aria(..)`, `data(..)`, `style(..)`, `id(..)`, ...);
    /// it replaces the default of the same name. Children are added after
    /// the drawing (`<animate>`, ...).
    pub fn with(mut self, part: impl Part) -> Self {
        self.extra = self.extra.with(part);
        self
    }

    /// The Lucide name of the icon.
    pub fn name(&self) -> &'static str {
        self.data.name
    }

    fn stroke(&self) -> String {
        let width = self.stroke_width.unwrap_or(2.0);
        let px = self.size.as_deref().map(|s| s.trim_end_matches("px")).and_then(|s| s.parse::<f32>().ok());
        match (self.absolute_stroke_width, px) {
            (true, Some(px)) if px > 0.0 => number(width * 24.0 / px),
            _ => number(width),
        }
    }
}

/// `2` rather than `2.0`, at most three decimals.
fn number(v: f32) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl View for Icon {
    fn into_node(self) -> Node {
        let size = self.size.clone().unwrap_or_else(|| "24".into());
        let mut svg = Element::new("svg");
        for (name, value) in [
            ("width", size.clone()),
            ("height", size),
            ("viewBox", "0 0 24 24".into()),
            ("fill", self.fill.clone().unwrap_or_else(|| "none".into())),
            ("stroke", self.color.clone().unwrap_or_else(|| "currentColor".into())),
            ("stroke-width", self.stroke()),
            ("stroke-linecap", "round".into()),
            ("stroke-linejoin", "round".into()),
        ] {
            svg.set_attr(Attr::new(name, value));
        }
        if !self.unstyled {
            svg.set_attr(Attr::new("class", format!("lucide lucide-{}", self.data.name)));
        }
        match &self.title {
            Some(title) => {
                svg.set_attr(Attr::new("role", "img"));
                svg.set_attr(Attr::new("aria-label", title.clone()));
                svg.children.push(Element::new("title").with(title.clone()).into_node());
            }
            None => svg.set_attr(Attr::new("aria-hidden", "true")),
        }
        for a in self.extra.attrs {
            svg.set_attr(a);
        }
        // Trusted: generated from Lucide's drawings.
        svg.children.push(raw_html(self.data.body));
        svg.children.extend(self.extra.children);
        svg.into_node()
    }
}

/// Every icon, sorted by name.
pub fn all() -> &'static [&'static IconData] {
    generated::ALL
}

/// The icon with a Lucide name (`"arrow-right"`), for names that come from
/// data. Using it keeps every icon in the binary; call the icon functions
/// directly when the name is known.
/// Former Lucide names (`"home"`, `"trash-2"`) work too.
pub fn by_name(name: &str) -> Option<Icon> {
    if let Ok(i) = generated::ALL.binary_search_by(|d| d.name.cmp(name)) {
        return Some(Icon::new(generated::ALL[i]));
    }
    let i = generated::ALIASES.binary_search_by(|(alias, _)| (*alias).cmp(name)).ok()?;
    Some(Icon::new(generated::ALIASES[i].1))
}

/// An icon with properties set, in the style of the element macros:
/// `Icon![House, size = 20, class("text-zinc-500")]`.
#[macro_export]
macro_rules! Icon {
    ($icon:ident $(, $($body:tt)*)?) => { $crate::__icon_munch!([$crate::$icon()] $($($body)*)?) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __icon_munch {
    ([$($acc:tt)*]) => { $($acc)* };
    ([$($acc:tt)*] $key:ident = $val:expr $(, $($rest:tt)*)?) => {
        $crate::__icon_munch!([$($acc)*.$key($val)] $($($rest)*)?)
    };
    ([$($acc:tt)*] $part:expr $(, $($rest:tt)*)?) => {
        $crate::__icon_munch!([$($acc)*.with($part)] $($($rest)*)?)
    };
}
