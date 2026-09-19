//! Ready-made UI components for Next Rust.
//!
//! ```
//! use next_rust_ui::*;
//! use next_rust_view::*;
//!
//! let form = form![
//!     Input![label = "Email", name = "email", kind = "email", placeholder = "you@example.com"],
//!     PasswordInput![label = "Password", name = "password"],
//!     Button![color = Color::Primary, submit = true, class("w-full"), "Sign in"],
//! ];
//! let html = render_static(form);
//! assert!(html.contains(r#"class="nr-btn nr-btn-solid nr-c-primary nr-btn-md w-full""#));
//! ```
//!
//! Every component is a builder with a macro of the same name. Inside the
//! macro, `key = value` sets a property and anything else is added like in an
//! element macro: attributes (`class(..)`, `id(..)`, `aria(..)`, `data(..)`)
//! and children. Every property is optional.
//!
//! Styling:
//!
//! * Each component has default classes (`nr-btn`, `nr-input`, ...) whose
//!   rules live in the CSS `components` layer. Classes you add, Tailwind
//!   utilities or your own CSS, always win over them, without `!important`.
//! * `unstyled = true` drops the default classes of the outer element.
//! * Colors, radii and shadows are CSS variables (`--nr-primary`,
//!   `--nr-radius-md`, ...): set them on `:root` to theme every component.
//!   Dark mode follows the system, or `class="dark"` / `data-theme="dark"`
//!   on an ancestor.
//! * Pages receive only the CSS of the components they render.
//!
//! Interactive components (select, date picker, password toggle, tabs with
//! panels, modals, closable alerts, `on_press`) load a small script,
//! `/_next-rust/ui.js`, on the pages that use them. Without it they fall
//! back to the native control (a `<select>`, a `<dialog>`, `<details>`), so
//! pages keep working.

#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

use next_rust_view::{Attr, Element, Node, Part, Stylesheet, View};

mod avatar;
mod button;
mod card;
mod date_picker;
mod display;
mod feedback;
mod icons;
mod input;
mod layout;
mod navigation;
mod overlay;
mod press;
mod script;
mod select;
#[doc(hidden)]
pub mod style;
mod table;
mod toggle;

pub use avatar::{Avatar, AvatarGroup};
pub use button::{Button, ButtonGroup};
pub use card::{Card, CardBody, CardFooter, CardHeader, Shadow};
pub use date_picker::DatePicker;
pub use display::{
    Alert, Badge, CircularProgress, EmptyState, Kbd, Placement, Progress, Side, Skeleton, Stat, Tooltip, Trend,
};
pub use feedback::{Chip, Divider, Spinner};
pub use input::{Input, LabelPlacement, PasswordInput, Textarea};
pub use layout::{Align, AppShell, Container, Grid, Justify, Navbar, NavbarItem, Sidebar, SidebarItem, Stack, Width};
pub use navigation::{
    Accordion, AccordionItem, AccordionVariant, BreadcrumbItem, Breadcrumbs, Step, StepStatus, Steps, Tab, Tabs,
    TabsVariant,
};
pub use overlay::{Modal, ModalBody, ModalFooter, ModalPlacement};
pub use press::Press;
pub use select::{Select, SelectItem};
pub use table::Table;
pub use toggle::{Checkbox, Radio, RadioGroup, Switch};

/// The component stylesheet. Components add it to the page themselves; only
/// the rules for the components a page renders are sent.
pub static UI_CSS: Stylesheet = Stylesheet {
    id: "nr-ui",
    // `style::UI_CSS_SOURCE`, minified by `build.rs`.
    css: include_str!(concat!(env!("OUT_DIR"), "/ui.min.css")),
    per_class: Some(&[]),
    scripts: &[],
};

pub use script::{UI_JS, UI_JS_MIN, rename_classes as rename_script_classes};

/// Classes the components render that no stylesheet rule starts with (hooks
/// for the script and for your own CSS). Release builds shorten them with the
/// stylesheet's classes; `tests/components.rs` keeps the list complete.
#[doc(hidden)]
pub const EXTRA_CLASSES: &[&str] = &[
    "nr-checkbox-label",
    "nr-cols-1",
    "nr-datepicker-toggle",
    "nr-label-outside",
    "nr-modal-center",
    "nr-password",
    "nr-radio-label",
    "nr-shell-with-sidebar",
    "nr-switch-label",
    "nr-tab-title",
];

/// Color of a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    Default,
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

impl Color {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Color::Default => "nr-c-default",
            Color::Primary => "nr-c-primary",
            Color::Secondary => "nr-c-secondary",
            Color::Success => "nr-c-success",
            Color::Warning => "nr-c-warning",
            Color::Danger => "nr-c-danger",
        }
    }
}

/// Size of a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Size::Sm => "sm",
            Size::Md => "md",
            Size::Lg => "lg",
        }
    }
}

/// Corner rounding. `None` in a builder keeps the component's own default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radius {
    None,
    Sm,
    Md,
    Lg,
    Full,
}

impl Radius {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Radius::None => "nr-r-none",
            Radius::Sm => "nr-r-sm",
            Radius::Md => "nr-r-md",
            Radius::Lg => "nr-r-lg",
            Radius::Full => "nr-r-full",
        }
    }
}

/// Visual style of buttons and chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// Filled with the color.
    #[default]
    Solid,
    /// Outlined with the color.
    Bordered,
    /// Transparent until hovered.
    Light,
    /// A soft tint of the color.
    Flat,
    /// Neutral fill and border, colored text.
    Faded,
    /// Solid with a colored glow.
    Shadow,
    /// Outlined, filled on hover.
    Ghost,
}

impl Variant {
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Variant::Solid => "solid",
            Variant::Bordered => "bordered",
            Variant::Light => "light",
            Variant::Flat => "flat",
            Variant::Faded => "faded",
            Variant::Shadow => "shadow",
            Variant::Ghost => "ghost",
        }
    }
}

/// Visual style of text fields, selects and date pickers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldVariant {
    /// A soft filled box.
    #[default]
    Flat,
    /// An outlined box.
    Bordered,
    /// Filled and outlined.
    Faded,
    /// Only a bottom line.
    Underlined,
}

impl FieldVariant {
    pub(crate) fn class(self) -> &'static str {
        match self {
            FieldVariant::Flat => "nr-field-flat",
            FieldVariant::Bordered => "nr-field-bordered",
            FieldVariant::Faded => "nr-field-faded",
            FieldVariant::Underlined => "nr-field-underlined",
        }
    }
}

/// Text that may be absent: `&str`, `String` or an `Option` of either, so
/// `error_message = form.error("email")` works as is.
pub trait MaybeText {
    fn into_text(self) -> Option<String>;
}

impl MaybeText for &str {
    fn into_text(self) -> Option<String> {
        Some(self.to_owned())
    }
}
impl MaybeText for String {
    fn into_text(self) -> Option<String> {
        Some(self)
    }
}
impl MaybeText for &String {
    fn into_text(self) -> Option<String> {
        Some(self.clone())
    }
}
impl<T: MaybeText> MaybeText for Option<T> {
    fn into_text(self) -> Option<String> {
        self.and_then(MaybeText::into_text).filter(|s| !s.is_empty())
    }
}

/// Attributes and children given to a component inside its macro.
#[derive(Default)]
pub(crate) struct Extra {
    el: Option<Element>,
}

impl Extra {
    pub(crate) fn push(&mut self, part: impl Part) {
        part.apply(self.el.get_or_insert_with(|| Element::new("div")));
    }

    pub(crate) fn has_children(&self) -> bool {
        self.el.as_ref().is_some_and(|el| !el.children.is_empty())
    }

    /// `(class attribute of the outer element, other attributes, children)`.
    pub(crate) fn split(self) -> (Option<Attr>, Vec<Attr>, Vec<Node>) {
        let Some(el) = self.el else { return (None, Vec::new(), Vec::new()) };
        let mut class = None;
        let mut attrs = Vec::new();
        for a in el.attrs {
            if a.name == "class" {
                class = Some(a);
            } else {
                attrs.push(a);
            }
        }
        (class, attrs, el.children)
    }
}

/// Unique element id for labels and descriptions.
pub(crate) fn next_id(prefix: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("{prefix}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// An element with the component classes (unless unstyled), the extra class
/// and the component stylesheet.
pub(crate) fn root(tag: &'static str, classes: &[&str], unstyled: bool, extra: Option<Attr>) -> Element {
    let mut el = Element::new(tag);
    if !unstyled {
        el.set_attr(Attr::new(
            "class",
            classes.iter().filter(|c| !c.is_empty()).copied().collect::<Vec<_>>().join(" "),
        ));
    }
    if let Some(class) = extra {
        el.set_attr(class);
    }
    el
}

/// `class="…"` for an inner part: its default class plus the one given for it.
pub(crate) fn part_class(default: &str, extra: &Option<String>) -> Attr {
    match extra {
        Some(e) => Attr::new("class", format!("{default} {e}")),
        None => Attr::new("class", default.to_owned()),
    }
}

pub(crate) fn attr(name: &'static str, value: impl Into<Cow<'static, str>>) -> Attr {
    Attr::new(name, value)
}

pub(crate) fn flag(name: &'static str, on: bool) -> Attr {
    Attr::boolean(name, on)
}

/// Apply attributes to an element (`class` merges, others replace).
pub(crate) fn apply(el: &mut Element, attrs: Vec<Attr>) {
    for a in attrs {
        el.set_attr(a);
    }
}

/// The element plus the stylesheet, as a node.
pub(crate) fn styled(el: Element) -> Node {
    Node::Fragment(vec![Node::Style(&UI_CSS), el.into_node()])
}

/// Set properties and add parts: `ui!(Button::new(), color = Color::Primary, class("x"), "Save")`.
///
/// The component macros (`Button!`, `Input!`, ...) are built on this.
#[doc(hidden)]
#[macro_export]
macro_rules! __ui_munch {
    ([$($acc:tt)*]) => { $($acc)* };
    ([$($acc:tt)*] $key:ident = $val:expr $(, $($rest:tt)*)?) => {
        $crate::__ui_munch!([$($acc)*.$key($val)] $($($rest)*)?)
    };
    ([$($acc:tt)*] $part:expr $(, $($rest:tt)*)?) => {
        $crate::__ui_munch!([$($acc)*.with($part)] $($($rest)*)?)
    };
}

/// Build a [`AppShell`](struct@crate::AppShell): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! AppShell {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::AppShell::new()] $($body)*) };
}

/// Build a [`Avatar`](struct@crate::Avatar): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Avatar {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Avatar::new()] $($body)*) };
}

/// Build a [`AvatarGroup`](struct@crate::AvatarGroup): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! AvatarGroup {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::AvatarGroup::new()] $($body)*) };
}

/// Build a [`Button`](struct@crate::Button): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Button {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Button::new()] $($body)*) };
}

/// Build a [`ButtonGroup`](struct@crate::ButtonGroup): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ButtonGroup {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ButtonGroup::new()] $($body)*) };
}

/// Build a [`Card`](struct@crate::Card): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Card {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Card::new()] $($body)*) };
}

/// Build a [`CardBody`](struct@crate::CardBody): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CardBody {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CardBody::new()] $($body)*) };
}

/// Build a [`CardFooter`](struct@crate::CardFooter): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CardFooter {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CardFooter::new()] $($body)*) };
}

/// Build a [`CardHeader`](struct@crate::CardHeader): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CardHeader {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CardHeader::new()] $($body)*) };
}

/// Build a [`Checkbox`](struct@crate::Checkbox): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Checkbox {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Checkbox::new()] $($body)*) };
}

/// Build a [`Chip`](struct@crate::Chip): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Chip {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Chip::new()] $($body)*) };
}

/// Build a [`Container`](struct@crate::Container): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Container {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Container::new()] $($body)*) };
}

/// Build a [`DatePicker`](struct@crate::DatePicker): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! DatePicker {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::DatePicker::new()] $($body)*) };
}

/// Build a [`Divider`](struct@crate::Divider): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Divider {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Divider::new()] $($body)*) };
}

/// Build a [`Grid`](struct@crate::Grid): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Grid {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Grid::new()] $($body)*) };
}

/// Build a [`Input`](struct@crate::Input): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Input {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Input::new()] $($body)*) };
}

/// Build a [`Navbar`](struct@crate::Navbar): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Navbar {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Navbar::new()] $($body)*) };
}

/// Build a [`NavbarItem`](struct@crate::NavbarItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! NavbarItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::NavbarItem::new()] $($body)*) };
}

/// Build a [`PasswordInput`](struct@crate::PasswordInput): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! PasswordInput {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::PasswordInput::new()] $($body)*) };
}

/// Build a [`Radio`](struct@crate::Radio): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Radio {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Radio::new()] $($body)*) };
}

/// Build a [`RadioGroup`](struct@crate::RadioGroup): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! RadioGroup {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::RadioGroup::new()] $($body)*) };
}

/// Build a [`Select`](struct@crate::Select): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Select {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Select::new()] $($body)*) };
}

/// Build a [`SelectItem`](struct@crate::SelectItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! SelectItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::SelectItem::new()] $($body)*) };
}

/// Build a [`Sidebar`](struct@crate::Sidebar): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Sidebar {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Sidebar::new()] $($body)*) };
}

/// Build a [`SidebarItem`](struct@crate::SidebarItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! SidebarItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::SidebarItem::new()] $($body)*) };
}

/// Build a [`Spinner`](struct@crate::Spinner): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Spinner {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Spinner::new()] $($body)*) };
}

/// Build a [`Stack`](struct@crate::Stack): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Stack {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Stack::new()] $($body)*) };
}

/// Build a [`Switch`](struct@crate::Switch): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Switch {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Switch::new()] $($body)*) };
}

/// Build a [`Textarea`](struct@crate::Textarea): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Textarea {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Textarea::new()] $($body)*) };
}

/// Build a [`Accordion`](struct@crate::Accordion): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Accordion {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Accordion::new()] $($body)*) };
}

/// Build a [`AccordionItem`](struct@crate::AccordionItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! AccordionItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::AccordionItem::new()] $($body)*) };
}

/// Build a [`Alert`](struct@crate::Alert): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Alert {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Alert::new()] $($body)*) };
}

/// Build a [`Badge`](struct@crate::Badge): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Badge {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Badge::new()] $($body)*) };
}

/// Build a [`BreadcrumbItem`](struct@crate::BreadcrumbItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! BreadcrumbItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::BreadcrumbItem::new()] $($body)*) };
}

/// Build a [`Breadcrumbs`](struct@crate::Breadcrumbs): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Breadcrumbs {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Breadcrumbs::new()] $($body)*) };
}

/// Build a [`CircularProgress`](struct@crate::CircularProgress): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CircularProgress {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CircularProgress::new()] $($body)*) };
}

/// Build a [`EmptyState`](struct@crate::EmptyState): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! EmptyState {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::EmptyState::new()] $($body)*) };
}

/// Build a [`Kbd`](struct@crate::Kbd): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Kbd {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Kbd::new()] $($body)*) };
}

/// Build a [`Modal`](struct@crate::Modal): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Modal {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Modal::new()] $($body)*) };
}

/// Build a [`ModalBody`](struct@crate::ModalBody): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ModalBody {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ModalBody::new()] $($body)*) };
}

/// Build a [`ModalFooter`](struct@crate::ModalFooter): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ModalFooter {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ModalFooter::new()] $($body)*) };
}

/// Build a [`Progress`](struct@crate::Progress): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Progress {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Progress::new()] $($body)*) };
}

/// Build a [`Skeleton`](struct@crate::Skeleton): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Skeleton {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Skeleton::new()] $($body)*) };
}

/// Build a [`Stat`](struct@crate::Stat): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Stat {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Stat::new()] $($body)*) };
}

/// Build a [`Step`](struct@crate::Step): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Step {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Step::new()] $($body)*) };
}

/// Build a [`Steps`](struct@crate::Steps): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Steps {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Steps::new()] $($body)*) };
}

/// Build a [`Tab`](struct@crate::Tab): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Tab {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Tab::new()] $($body)*) };
}

/// Build a [`Table`](struct@crate::Table): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Table {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Table::new()] $($body)*) };
}

/// Build a [`Tabs`](struct@crate::Tabs): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Tabs {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Tabs::new()] $($body)*) };
}

/// Build a [`Tooltip`](struct@crate::Tooltip): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Tooltip {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Tooltip::new()] $($body)*) };
}
