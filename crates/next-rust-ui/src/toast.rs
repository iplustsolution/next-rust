//! Toasts: short messages in a corner of the screen.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::{Color, Extra, apply, attr, part, props, root, styled};

/// The corner a [`Toaster`] shows its toasts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastPlacement {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

/// Where toasts appear. Render one per page, usually in the root layout.
///
/// Toasts given as children show right away, which is how a server flash
/// message becomes a toast; they fade out on their own with CSS alone. With
/// the component script, `nextRust.toast("Saved")` (or an object: `{ title,
/// message, color, duration }`), [`Press::toast`](crate::Press::toast) and an
/// `nr:toast` event on `window` add more.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let flash = Some("Profile saved");
/// let t = Toaster![placement = ToastPlacement::TopRight, flash.map(|m| Toast![color = Color::Success, m])];
/// let html = render_static(t);
/// assert!(html.contains(r#"class="nr-toaster nr-toaster-tr""#) && html.contains(r#"role="status""#), "{html}");
/// ```
#[derive(Default)]
pub struct Toaster {
    placement: ToastPlacement,
    unstyled: bool,
    extra: Extra,
}

impl Toaster {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        placement: val<ToastPlacement>,
    }
}

impl View for Toaster {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let placement = match self.placement {
            ToastPlacement::TopLeft => "nr-toaster-tl",
            ToastPlacement::TopCenter => "nr-toaster-tc",
            ToastPlacement::TopRight => "nr-toaster-tr",
            ToastPlacement::BottomLeft => "nr-toaster-bl",
            ToastPlacement::BottomCenter => "nr-toaster-bc",
            ToastPlacement::BottomRight => "nr-toaster-br",
        };
        let mut el = root("div", &["nr-toaster", placement], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "toaster"));
        el.set_attr(attr("role", "region"));
        el.set_attr(attr("aria-label", "Notifications"));
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// One message inside a [`Toaster`].
#[derive(Default)]
pub struct Toast {
    title: Option<String>,
    color: Color,
    duration: Option<u32>,
    hide_icon: bool,
    unstyled: bool,
    extra: Extra,
}

impl Toast {
    pub fn new() -> Self {
        Self { color: Color::Default, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// Bold first line.
        title: text,
        /// `Success`, `Warning`, `Danger`, ... pick the icon and the accent.
        color: val<Color>,
        /// Milliseconds on screen (5000 by default); `0` keeps it until dismissed.
        duration: opt<u32>,
        hide_icon: val<bool>,
    }
}

impl View for Toast {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let sticky = self.duration == Some(0);
        let mut el = root("div", &["nr-toast", if sticky { "nr-toast-sticky" } else { "" }], self.unstyled, class);
        // The same markup the script's `nextRust.toast` builds.
        el.set_attr(attr(
            "data-color",
            match self.color {
                Color::Default => "default",
                Color::Primary => "primary",
                Color::Secondary => "secondary",
                Color::Success => "success",
                Color::Warning => "warning",
                Color::Danger => "danger",
            },
        ));
        el.set_attr(attr(
            "role",
            if matches!(self.color, Color::Danger | Color::Warning) { "alert" } else { "status" },
        ));
        if let Some(ms) = self.duration.filter(|ms| *ms > 0) {
            el.set_attr(attr("style", format!("--nr-toast-d:{ms}ms")));
        }
        apply(&mut el, attrs);
        if !self.hide_icon {
            let glyph = icon(match self.color {
                Color::Success => icons::circle_check(),
                Color::Warning => icons::triangle_alert(),
                Color::Danger => icons::circle_x(),
                _ => icons::info(),
            });
            el.children.push(part("span", "nr-toast-icon").with(glyph).into_node());
        }
        let mut body = part("div", "nr-toast-body");
        if let Some(title) = self.title {
            body.children.push(part("strong", "nr-toast-title").with(title).into_node());
        }
        if !children.is_empty() {
            body.children.push(part("div", "nr-toast-text").with(children).into_node());
        }
        el.children.push(body.into_node());
        el.children.push(
            Element::new("button")
                .with(attr("type", "button"))
                .with(attr("class", "nr-toast-close"))
                .with(attr("aria-label", "Dismiss"))
                .with(attr("data-nr-dismiss", ""))
                .with(icon(icons::close()))
                .into_node(),
        );
        styled(el)
    }
}
