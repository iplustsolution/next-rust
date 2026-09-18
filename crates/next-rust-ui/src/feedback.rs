//! Spinners, chips and dividers.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::{Color, Extra, Radius, Size, Variant, apply, attr, root, styled};

/// The spinner markup used inside other components.
pub(crate) fn spinner_node(class: &str) -> Node {
    Element::new("span")
        .with(attr("class", format!("nr-spinner {class}")))
        .with(attr("aria-hidden", "true"))
        .with(Element::new("i").with(attr("class", "nr-spinner-ring")))
        .into_node()
}

/// A loading indicator.
#[derive(Default)]
pub struct Spinner {
    color: Color,
    size: Size,
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Spinner {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
    /// Text shown next to the spinner (also its accessible name).
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
}

impl View for Spinner {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let size = format!("nr-spinner-{}", self.size.suffix());
        let mut el = root("span", &["nr-spinner", &size, self.color.class()], self.unstyled, class);
        el.set_attr(attr("role", "status"));
        el.set_attr(attr("aria-label", self.label.clone().unwrap_or_else(|| "Loading".into())));
        apply(&mut el, attrs);
        el.children.push(Element::new("i").with(attr("class", "nr-spinner-ring")).into_node());
        if let Some(label) = self.label {
            el.children.push(Element::new("span").with(attr("class", "nr-spinner-label")).with(label).into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

/// A small label: a tag, a status, a count.
#[derive(Default)]
pub struct Chip {
    color: Color,
    variant: Variant,
    size: Size,
    radius: Option<Radius>,
    dot: bool,
    start_content: Option<Node>,
    end_content: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl Chip {
    pub fn new() -> Self {
        Self { color: Color::Default, ..Self::default() }
    }
    common_methods!();
    with_parts!();

    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
        self
    }
    pub fn variant(mut self, v: Variant) -> Self {
        self.variant = v;
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
    /// A colored dot before the text.
    pub fn dot(mut self, v: bool) -> Self {
        self.dot = v;
        self
    }
    pub fn start_content(mut self, v: impl View) -> Self {
        self.start_content = Some(v.into_node());
        self
    }
    pub fn end_content(mut self, v: impl View) -> Self {
        self.end_content = Some(v.into_node());
        self
    }
}

impl View for Chip {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let variant = format!("nr-chip-{}", self.variant.suffix());
        let size = format!("nr-chip-{}", self.size.suffix());
        let classes = ["nr-chip", &variant, self.color.class(), &size, self.radius.map(Radius::class).unwrap_or("")];
        let mut el = root("span", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        if self.dot {
            el.children.push(Element::new("span").with(attr("class", "nr-chip-dot")).into_node());
        }
        el.children.extend(self.start_content);
        el.children.push(Element::new("span").with(attr("class", "nr-chip-content")).with(children).into_node());
        el.children.extend(self.end_content);
        styled(el)
    }
}

/// A thin line between content.
#[derive(Default)]
pub struct Divider {
    vertical: bool,
    unstyled: bool,
    extra: Extra,
}

impl Divider {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn vertical(mut self, v: bool) -> Self {
        self.vertical = v;
        self
    }
}

impl View for Divider {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("hr", &["nr-divider", if self.vertical { "nr-divider-v" } else { "" }], self.unstyled, class);
        el.void = true;
        el.set_attr(attr("aria-orientation", if self.vertical { "vertical" } else { "horizontal" }));
        apply(&mut el, attrs);
        styled(el)
    }
}
