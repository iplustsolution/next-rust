//! Modals.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::layout::Width;
use crate::{Extra, Radius, apply, attr, flag, next_id, root, styled};

/// Where a [`Modal`] sits on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalPlacement {
    #[default]
    Center,
    Top,
    /// A sheet from the bottom: the usual choice on phones.
    Bottom,
}

/// A dialog over the page: a `<dialog>` element.
///
/// Open it with [`Press::open_modal`](crate::Press::open_modal) on any
/// pressable component, or with the `open` property to show it right away.
/// Its close button and `Escape` work without any script; the component
/// script adds opening as a true modal (focus kept inside, the page inert)
/// and closing on a click outside.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let m = Modal![id = "confirm", title = "Delete the class?",
///     ModalBody![p!["This cannot be undone."]],
///     ModalFooter![Button![on_press = Press::close_modal(), "Cancel"]],
/// ];
/// let html = render_static(m);
/// assert!(html.contains(r#"<dialog class="nr-modal nr-modal-md nr-modal-center" id="confirm""#), "{html}");
/// assert!(html.contains(r#"<form method="dialog""#), "closes without a script");
/// ```
#[derive(Default)]
pub struct Modal {
    id: Option<String>,
    title: Option<String>,
    size: Width,
    placement: ModalPlacement,
    radius: Option<Radius>,
    open: bool,
    closable: bool,
    dismissable: bool,
    scroll_inside: bool,
    unstyled: bool,
    extra: Extra,
}

impl Modal {
    pub fn new() -> Self {
        Self { size: Width::Md, closable: true, dismissable: true, ..Self::default() }
    }
    common_methods!();
    with_parts!();

    /// The id [`Press::open_modal`](crate::Press::open_modal) refers to.
    pub fn id(mut self, v: impl Into<String>) -> Self {
        self.id = Some(v.into());
        self
    }
    /// A heading at the top (also the dialog's accessible name).
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    /// `Sm` 24rem, `Md` 32rem, `Lg` 40rem, `Xl` 48rem, `Xxl` 64rem, `Full`.
    pub fn size(mut self, v: Width) -> Self {
        self.size = v;
        self
    }
    pub fn placement(mut self, v: ModalPlacement) -> Self {
        self.placement = v;
        self
    }
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
    /// Shown when the page loads (as a modal once the script runs).
    pub fn open(mut self, v: bool) -> Self {
        self.open = v;
        self
    }
    /// A close button in the corner (on by default).
    pub fn closable(mut self, v: bool) -> Self {
        self.closable = v;
        self
    }
    /// A click on the backdrop closes it (on by default). `Escape` always does.
    pub fn dismissable(mut self, v: bool) -> Self {
        self.dismissable = v;
        self
    }
    /// Long content scrolls inside the body, the header and footer stay put.
    pub fn scroll_inside(mut self, v: bool) -> Self {
        self.scroll_inside = v;
        self
    }
}

impl View for Modal {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let size = match self.size {
            Width::Sm => "nr-modal-sm",
            Width::Md => "nr-modal-md",
            Width::Lg => "nr-modal-lg",
            Width::Xl => "nr-modal-xl",
            Width::Xxl => "nr-modal-2xl",
            Width::Full => "nr-modal-full",
        };
        let placement = match self.placement {
            ModalPlacement::Center => "nr-modal-center",
            ModalPlacement::Top => "nr-modal-top",
            ModalPlacement::Bottom => "nr-modal-bottom",
        };
        let classes = [
            "nr-modal",
            size,
            placement,
            self.radius.map(Radius::class).unwrap_or(""),
            if self.scroll_inside { "nr-modal-scroll" } else { "" },
        ];
        let mut el = root("dialog", &classes, self.unstyled, class);
        let id = self.id.unwrap_or_else(|| next_id("nr-modal"));
        el.set_attr(attr("id", id.clone()));
        el.set_attr(attr("data-nr-ui", "modal"));
        el.set_attr(flag("open", self.open));
        el.set_attr(flag("data-dismissable", self.dismissable));
        if self.title.is_some() {
            el.set_attr(attr("aria-labelledby", format!("{id}-title")));
        }
        apply(&mut el, attrs);

        let mut boxed = Element::new("div").with(attr("class", "nr-modal-box"));
        if self.title.is_some() || self.closable {
            let mut header = Element::new("div").with(attr("class", "nr-modal-header"));
            if let Some(title) = self.title {
                header.children.push(
                    Element::new("h2")
                        .with(attr("class", "nr-modal-title"))
                        .with(attr("id", format!("{id}-title")))
                        .with(title)
                        .into_node(),
                );
            }
            if self.closable {
                // Its own `method="dialog"` form: closes natively, and never
                // nests inside a form the body may hold.
                header.children.push(
                    Element::new("form")
                        .with(attr("method", "dialog"))
                        .with(attr("class", "nr-modal-closer"))
                        .with(
                            Element::new("button")
                                .with(attr("type", "submit"))
                                .with(attr("class", "nr-modal-close"))
                                .with(attr("aria-label", "Close"))
                                .with(icon(icons::close())),
                        )
                        .into_node(),
                );
            }
            boxed.children.push(header.into_node());
        }
        boxed.children.extend(children);
        el.children.push(boxed.into_node());
        styled(el)
    }
}

macro_rules! modal_part {
    ($name:ident, $class:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Default)]
        pub struct $name {
            unstyled: bool,
            extra: Extra,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
            common_methods!();
            with_parts!();
        }

        impl View for $name {
            fn into_node(self) -> Node {
                let (class, attrs, children) = self.extra.split();
                let mut el = root("div", &[$class], self.unstyled, class);
                apply(&mut el, attrs);
                el.children = children;
                styled(el)
            }
        }
    };
}

modal_part!(ModalBody, "nr-modal-body", "The content of a [`Modal`].");
modal_part!(ModalFooter, "nr-modal-footer", "The bottom of a [`Modal`], typically its actions.");
