//! Drawers, the confirm dialog preset and the loading overlay.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::feedback::spinner_node;
use crate::icons::{self, icon};
use crate::layout::Width;
use crate::{Button, Color, Extra, Press, Variant, apply, attr, class_list, flag, next_id, part, props, root, styled};

/// The edge a [`Drawer`] slides in from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerSide {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}

/// The header of a dialog: a title and a `method="dialog"` close button.
fn dialog_header(id: &str, title: Option<String>, closable: bool) -> Option<Node> {
    if title.is_none() && !closable {
        return None;
    }
    let mut header = part("div", "nr-modal-header");
    if let Some(title) = title {
        header
            .children
            .push(part("h2", "nr-modal-title").with(attr("id", format!("{id}-title"))).with(title).into_node());
    }
    if closable {
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
    Some(header.into_node())
}

/// A panel that slides in from an edge of the screen: a `<dialog>`, like
/// [`Modal`](struct@crate::Modal). Open it with
/// [`Press::open_modal`](crate::Press::open_modal); use
/// [`ModalBody`](struct@crate::ModalBody) and
/// [`ModalFooter`](struct@crate::ModalFooter) inside.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let d = Drawer![id = "filters", title = "Filters", side = DrawerSide::Left, ModalBody![p!["…"]]];
/// let html = render_static(d);
/// assert!(html.contains(r#"<dialog class="nr-modal nr-drawer nr-drawer-left nr-drawer-md" id="filters""#), "{html}");
/// ```
#[derive(Default)]
pub struct Drawer {
    id: Option<String>,
    title: Option<String>,
    side: DrawerSide,
    size: Width,
    open: bool,
    closable: bool,
    dismissable: bool,
    unstyled: bool,
    extra: Extra,
}

impl Drawer {
    pub fn new() -> Self {
        Self { size: Width::Md, closable: true, dismissable: true, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// The id [`Press::open_modal`](crate::Press::open_modal) refers to.
        id: text,
        /// A heading at the top (also the accessible name).
        title: text,
        /// `Right` (default), `Left`, `Top` or `Bottom`.
        side: val<DrawerSide>,
        /// Width (or height for `Top`/`Bottom`): `Sm` 18rem, `Md` 24rem, `Lg` 32rem, `Xl` 40rem, `Xxl` 48rem, `Full`.
        size: val<Width>,
        /// Shown when the page loads.
        open: val<bool>,
        /// A close button in the corner (on by default).
        closable: val<bool>,
        /// A click on the backdrop closes it (on by default).
        dismissable: val<bool>,
    }
}

impl View for Drawer {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let side = match self.side {
            DrawerSide::Left => "nr-drawer-left",
            DrawerSide::Right => "nr-drawer-right",
            DrawerSide::Top => "nr-drawer-top",
            DrawerSide::Bottom => "nr-drawer-bottom",
        };
        let size = match self.size {
            Width::Sm => "nr-drawer-sm",
            Width::Md => "nr-drawer-md",
            Width::Lg => "nr-drawer-lg",
            Width::Xl => "nr-drawer-xl",
            Width::Xxl => "nr-drawer-2xl",
            Width::Full => "nr-drawer-full",
        };
        let mut el = root("dialog", &["nr-modal", "nr-drawer", side, size], self.unstyled, class);
        let id = self.id.unwrap_or_else(|| next_id("nr-drawer"));
        el.set_attr(attr("id", id.clone()));
        el.set_attr(attr("data-nr-ui", "modal"));
        el.set_attr(flag("open", self.open));
        el.set_attr(flag("data-dismissable", self.dismissable));
        if self.title.is_some() {
            el.set_attr(attr("aria-labelledby", format!("{id}-title")));
        }
        apply(&mut el, attrs);
        let mut boxed = part("div", "nr-modal-box");
        boxed.children.extend(dialog_header(&id, self.title, self.closable));
        boxed.children.extend(children);
        el.children.push(boxed.into_node());
        styled(el)
    }
}

/// A small dialog asking to confirm an action: a title, a message (the
/// children), a cancel button and a confirm button.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = ConfirmDialog![id = "delete", title = "Delete the class?", confirm_label = "Delete",
///     on_confirm = Press::action("/_next-rust/action/delete"), "This cannot be undone."];
/// let html = render_static(c);
/// assert!(html.contains(r#"role="alertdialog""#) && html.contains(r#"data-nr-press="action""#), "{html}");
/// ```
#[derive(Default)]
pub struct ConfirmDialog {
    id: Option<String>,
    title: Option<String>,
    confirm_label: Option<String>,
    cancel_label: Option<String>,
    color: Color,
    on_confirm: Option<Press>,
    confirm: Option<Node>,
    open: bool,
    unstyled: bool,
    extra: Extra,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self { color: Color::Danger, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// The id [`Press::open_modal`](crate::Press::open_modal) refers to.
        id: text,
        title: text,
        /// "Confirm" by default.
        confirm_label: text,
        /// "Cancel" by default.
        cancel_label: text,
        /// Color of the confirm button (`Danger` by default).
        color: val<Color>,
        /// Your own confirm control instead of the button: a `form!` posting to an action with a submit `Button!`.
        confirm: node,
        open: val<bool>,
    }
    /// What confirming does: `on_confirm = action!(delete)`, or a [`Press`].
    pub fn on_confirm(mut self, v: impl Into<Press>) -> Self {
        self.on_confirm = Some(v.into());
        self
    }
}

impl View for ConfirmDialog {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("dialog", &["nr-modal", "nr-modal-sm", "nr-confirm"], self.unstyled, class);
        let id = self.id.unwrap_or_else(|| next_id("nr-confirm"));
        el.set_attr(attr("id", id.clone()));
        el.set_attr(attr("data-nr-ui", "modal"));
        el.set_attr(attr("role", "alertdialog"));
        el.set_attr(flag("open", self.open));
        if self.title.is_some() {
            el.set_attr(attr("aria-labelledby", format!("{id}-title")));
        }
        el.set_attr(attr("aria-describedby", format!("{id}-text")));
        apply(&mut el, attrs);
        let mut boxed = part("div", "nr-modal-box");
        boxed.children.extend(dialog_header(&id, self.title, false));
        let mut body = part("div", "nr-modal-body").with(attr("id", format!("{id}-text")));
        body.children = children;
        boxed.children.push(body.into_node());
        let cancel = Button::new()
            .variant(Variant::Light)
            .color(Color::Default)
            .with(self.cancel_label.unwrap_or_else(|| "Cancel".to_owned()));
        // Without the script the cancel button still closes: a `method="dialog"` form.
        let cancel = Element::new("form")
            .with(attr("method", "dialog"))
            .with(attr("class", "nr-modal-closer"))
            .with(cancel.submit(true));
        let confirm = self.confirm.unwrap_or_else(|| {
            let mut b =
                Button::new().color(self.color).with(self.confirm_label.unwrap_or_else(|| "Confirm".to_owned()));
            if let Some(press) = self.on_confirm {
                b = b.on_press(press);
            }
            b.into_node()
        });
        boxed.children.push(part("div", "nr-modal-footer").with(cancel).with(confirm).into_node());
        el.children.push(boxed.into_node());
        styled(el)
    }
}

/// Covers its children with a veil and a spinner while `visible`.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let l = LoadingOverlay![visible = true, label = "Saving…", Card![CardBody!["content"]]];
/// let html = render_static(l);
/// assert!(html.contains(r#"aria-busy="true""#) && html.contains("nr-loading-veil"));
/// ```
#[derive(Default)]
pub struct LoadingOverlay {
    visible: bool,
    label: Option<String>,
    blur: bool,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl LoadingOverlay {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        visible: val<bool>,
        /// Text next to the spinner (also announced).
        label: text,
        /// Blur what is behind.
        blur: val<bool>,
        color: val<Color>,
    }
}

impl View for LoadingOverlay {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-loading"], self.unstyled, class);
        if self.visible {
            el.set_attr(attr("aria-busy", "true"));
        }
        apply(&mut el, attrs);
        el.children = children;
        if self.visible {
            let (label_class, label) = match self.label {
                Some(label) => ("nr-loading-label", label),
                None => ("nr-sr", "Loading".to_owned()),
            };
            let mut veil = Element::new("div")
                .with(class_list(&[
                    "nr-loading-veil",
                    self.color.class(),
                    if self.blur { "nr-loading-blur" } else { "" },
                ]))
                .with(attr("role", "status"))
                .with(spinner_node("nr-loading-spinner"));
            veil.children.push(part("span", label_class).with(label).into_node());
            el.children.push(veil.into_node());
        }
        styled(el)
    }
}
