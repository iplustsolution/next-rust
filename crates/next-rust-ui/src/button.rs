//! Buttons.

use next_rust_view::{Node, View};

use crate::feedback::spinner_node;
use crate::{Color, Extra, Press, Radius, Size, Variant, apply, attr, flag, root, styled};

/// Add the `class`, `unstyled` and `with` methods every component has.
macro_rules! common_methods {
    () => {
        /// Extra classes for the outer element. They win over the defaults.
        pub fn class(mut self, v: impl Into<String>) -> Self {
            self.extra.push(next_rust_view::attrs::class(v.into()));
            self
        }
        /// Leave out the default classes of the outer element.
        pub fn unstyled(mut self, v: bool) -> Self {
            self.unstyled = v;
            self
        }
    };
}
pub(crate) use common_methods;

/// Add the `with` method: attributes and children, like in an element macro.
macro_rules! with_parts {
    () => {
        /// Add an attribute (`class(..)`, `id(..)`, `aria(..)`, ...) or a child.
        pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
            self.extra.push(part);
            self
        }
    };
}
pub(crate) use with_parts;

/// A button, or a link styled as one (`href`).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let b = Button![variant = Variant::Bordered, color = Color::Danger, size = Size::Sm, "Delete"];
/// assert!(render_static(b).contains(r#"<button class="nr-btn nr-btn-bordered nr-c-danger nr-btn-sm" type="button">Delete</button>"#));
/// ```
#[derive(Default)]
pub struct Button {
    color: Color,
    variant: Variant,
    size: Size,
    radius: Option<Radius>,
    full_width: bool,
    icon_only: bool,
    loading: bool,
    disabled: bool,
    submit: bool,
    href: Option<String>,
    start_content: Option<Node>,
    end_content: Option<Node>,
    press: Option<Press>,
    unstyled: bool,
    extra: Extra,
}

impl Button {
    pub fn new() -> Self {
        Self::default()
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
    /// Take the full width of the container.
    pub fn full_width(mut self, v: bool) -> Self {
        self.full_width = v;
        self
    }
    /// A square button holding only an icon (give it an `aria("label", ..)`).
    pub fn icon_only(mut self, v: bool) -> Self {
        self.icon_only = v;
        self
    }
    /// Show a spinner and ignore presses.
    pub fn loading(mut self, v: bool) -> Self {
        self.loading = v;
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    /// A submit button (`type="submit"`). Buttons are `type="button"` otherwise.
    /// Inside a form posting to a server action, it shows a spinner while the form submits.
    pub fn submit(mut self, v: bool) -> Self {
        self.submit = v;
        self
    }
    /// Render a link that looks like a button (client-side navigation).
    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    /// Content before the label, typically an icon.
    pub fn start_content(mut self, v: impl View) -> Self {
        self.start_content = Some(v.into_node());
        self
    }
    /// Content after the label.
    pub fn end_content(mut self, v: impl View) -> Self {
        self.end_content = Some(v.into_node());
        self
    }
    /// What pressing the button does: `on_press = action!(save)`, or a [`Press`].
    pub fn on_press(mut self, v: impl Into<Press>) -> Self {
        self.press = Some(v.into());
        self
    }
    /// Same as [`Button::on_press`].
    pub fn on_click(self, v: impl Into<Press>) -> Self {
        self.on_press(v)
    }
}

impl View for Button {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let variant = format!("nr-btn-{}", self.variant.suffix());
        let size = format!("nr-btn-{}", self.size.suffix());
        let classes = [
            "nr-btn",
            &variant,
            self.color.class(),
            &size,
            self.radius.map(Radius::class).unwrap_or(""),
            if self.full_width { "nr-btn-full" } else { "" },
            if self.icon_only { "nr-btn-icon" } else { "" },
        ];
        let tag = if self.href.is_some() { "a" } else { "button" };
        let mut el = root(tag, &classes, self.unstyled, class);
        let inactive = self.disabled || self.loading;
        match &self.href {
            Some(href) => {
                el.set_attr(attr("href", href.clone()));
                el.set_attr(attr("data-nr-link", ""));
                if inactive {
                    el.set_attr(attr("aria-disabled", "true"));
                    el.set_attr(attr("tabindex", "-1"));
                }
            }
            None => {
                el.set_attr(attr("type", if self.submit { "submit" } else { "button" }));
                el.set_attr(flag("disabled", inactive));
            }
        }
        if self.loading {
            el.set_attr(attr("data-loading", "true"));
            el.set_attr(attr("aria-busy", "true"));
        }
        if let Some(press) = &self.press {
            apply(&mut el, press.attrs());
        }
        apply(&mut el, attrs);
        let start = if self.loading { Some(spinner_node("nr-btn-spinner")) } else { self.start_content };
        if let Some(start) = start {
            el.children.push(start);
        } else if self.submit || self.press.as_ref().is_some_and(Press::is_action) {
            // Shown while the form submits or the action runs (see the stylesheet).
            el.children.push(spinner_node("nr-btn-spinner nr-btn-pending"));
        }
        el.children.extend(children);
        if let Some(end) = self.end_content {
            el.children.push(end);
        }
        styled(el)
    }
}

/// Buttons joined into one control.
#[derive(Default)]
pub struct ButtonGroup {
    full_width: bool,
    unstyled: bool,
    extra: Extra,
}

impl ButtonGroup {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn full_width(mut self, v: bool) -> Self {
        self.full_width = v;
        self
    }
}

impl View for ButtonGroup {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el =
            root("div", &["nr-btn-group", if self.full_width { "nr-btn-full" } else { "" }], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}
