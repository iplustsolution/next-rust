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

// `deny` rather than `forbid`: `wasm::host` declares the one imported
// function the WebAssembly build calls the browser through, which no safe
// Rust can express. It is the only exception in the crate, it is annotated
// there, and nothing else may add another.
#![deny(unsafe_code)]

use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

use next_rust_view::{Attr, Element, Node, Part, Stylesheet, View};

mod avatar;
mod button;
mod card;
mod charts;
mod command;
mod data;
mod date_picker;
mod display;
mod drawer;
mod feedback;
mod forms;
mod icons;
mod input;
mod layout;
mod menu;
mod navigation;
mod overlay;
mod press;
mod script;
mod sections;
mod select;
#[doc(hidden)]
pub mod style;
mod table;
mod toast;
mod toggle;

pub use avatar::{Avatar, AvatarGroup, AvatarStatus};
pub use button::{Button, ButtonGroup};
pub use card::{Card, CardBody, CardFooter, CardHeader, Shadow};
pub use charts::{BarChart, Donut, LineChart, Sparkline};
pub use command::{CommandGroup, CommandItem, CommandPalette};
pub use data::{
    Banner, Callout, CalloutKind, ChipGroup, Code, CodeBlock, DescriptionItem, DescriptionList, Figure, List, ListItem,
    Meter, ResultBlock, ResultStatus, SortDirection, SortHeader, TableSelect, TableSelectAll, VideoEmbed,
};
pub use date_picker::DatePicker;
pub use display::{
    Alert, Badge, CircularProgress, EmptyState, Kbd, Placement, Progress, Side, Skeleton, Stat, Tooltip, Trend,
};
pub use drawer::{ConfirmDialog, Drawer, DrawerSide, LoadingOverlay};
pub use feedback::{Chip, Divider, Spinner};
pub use forms::{
    ColorPicker, Combobox, DateRangePicker, FileUpload, FormActions, FormField, FormSection, NumberInput, PinInput,
    Rating, SearchInput, Segment, SegmentPart, SegmentedControl, Slider, TagsInput, TimePicker,
};
pub use input::{Input, LabelPlacement, PasswordInput, Textarea};
pub use layout::{Align, AppShell, Container, Grid, Justify, Navbar, NavbarItem, Sidebar, SidebarItem, Stack, Width};
pub use menu::{ContextMenu, Dropdown, DropdownItem, DropdownSection, MenuAlign, Popover};
pub use navigation::{
    Accordion, AccordionItem, AccordionVariant, BreadcrumbItem, Breadcrumbs, Step, StepStatus, Steps, Tab, Tabs,
    TabsVariant,
};
pub use overlay::{Modal, ModalBody, ModalFooter, ModalPlacement};
pub use press::Press;
pub use sections::{
    AspectRatio, Bento, BentoItem, Breakpoint, Carousel, CarouselItem, Cta, Faq, FaqPart, Feature, FeatureGrid, Footer,
    FooterColumn, FooterLink, Hero, Hide, LogoCloud, Marquee, Pagination, PricingCard, PricingFeature, PricingTable,
    ScrollArea, Section, Show, Spacer, Sticky, Testimonial, Timeline, TimelineItem,
};
pub use select::{Select, SelectItem};
pub use table::Table;
pub use toast::{Toast, ToastPlacement, Toaster};
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

/// Every value the components write into `data-nr-ui`.
///
/// The browser layer keys off these and nothing else, whether it is the
/// script or the wasm module, so this list is the contract between the
/// renderer and both of them. `tests/components.rs` checks that the markup
/// writes no other value.
#[doc(hidden)]
pub const UI_WASM_KINDS: &[&str] = &[
    "alert",
    "avatar",
    "banner",
    "carousel",
    "checkall",
    "code",
    "color",
    "combobox",
    "command",
    "context",
    "datepicker",
    "dropdown",
    "field",
    "modal",
    "popover",
    "press",
    "select",
    "shell",
    "slider",
    "tabs",
    "tags",
    "toaster",
    "upload",
];

/// Every class, attribute and custom property the wasm module names.
///
/// The module itself is the `next-rust-ui-wasm` crate; this list is the part
/// of it the *build* has to know about.
///
/// A release build shortens `nr-*` classes and `data-*` attributes in the
/// markup, the stylesheet and the scripts it can rewrite. It cannot rewrite
/// a wasm module — the names live in a data segment whose offsets a shorter
/// string would move — so with `[ui] script = "wasm"` the build keeps these
/// names instead, the same way it keeps the names a file under `public/`
/// uses. Passing the list rather than parsing the module keeps that decision
/// reviewable.
#[doc(hidden)]
pub const UI_WASM_NAMES: &str = include_str!("wasm_names.txt");

/// Classes the components render that no stylesheet rule starts with (hooks
/// for the script and for your own CSS). Release builds shorten them with the
/// stylesheet's classes; `tests/components.rs` keeps the list complete.
#[doc(hidden)]
pub const EXTRA_CLASSES: &[&str] = &[
    "nr-barchart",
    "nr-bento-2",
    "nr-bento-c1",
    "nr-bento-head",
    "nr-bento-r1",
    "nr-carousel-1",
    "nr-checkbox-label",
    "nr-cols-1",
    "nr-combobox",
    "nr-datepicker-toggle",
    "nr-donut",
    "nr-footer-column",
    "nr-footer-legal",
    "nr-formfield-control",
    "nr-formsection-head",
    "nr-label-none",
    "nr-label-outside",
    "nr-linechart",
    "nr-modal-center",
    "nr-password",
    "nr-price-cta",
    "nr-radio-label",
    "nr-rating-static",
    "nr-section-inner",
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

/// `class="a b c"` from the non-empty names.
pub(crate) fn class_list(classes: &[&str]) -> Attr {
    Attr::new("class", classes.iter().filter(|c| !c.is_empty()).copied().collect::<Vec<_>>().join(" "))
}

/// `<tag class="…">` for an inner part.
pub(crate) fn part(tag: &'static str, class: &'static str) -> Element {
    Element::new(tag).with(Attr::new("class", class))
}

/// The first element of `nodes`, looking inside fragments (components are a
/// stylesheet plus an element).
pub(crate) fn first_element(nodes: &mut [Node]) -> Option<&mut Element> {
    for node in nodes {
        match node {
            Node::Element(el) => return Some(el),
            Node::Fragment(inner) => {
                if let Some(el) = first_element(inner) {
                    return Some(el);
                }
            }
            _ => {}
        }
    }
    None
}

/// A copy of static markup (text, elements, stylesheets). Async boundaries
/// and holes cannot be copied and are left out.
pub(crate) fn clone_node(node: &Node) -> Node {
    match node {
        Node::Text(t) => Node::Text(t.clone()),
        Node::Raw(t) => Node::Raw(t.clone()),
        Node::Element(el) => Node::Element(Box::new(Element {
            tag: el.tag.clone(),
            attrs: el.attrs.clone(),
            children: el.children.iter().map(clone_node).collect(),
            void: el.void,
        })),
        Node::Fragment(nodes) => Node::Fragment(nodes.iter().map(clone_node).collect()),
        Node::Style(sheet) => Node::Style(sheet),
        _ => Node::Empty,
    }
}

/// A number without a trailing `.0`.
pub(crate) fn trim_float(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{v}") }
}

/// Builder setters: `name: text` (`Option<String>`), `name: node`
/// (`Option<Node>` from a view), `name: maybe` (`Option<String>` from
/// [`MaybeText`]), `name: val<T>` (`T`), `name: opt<T>` (`Option<T>`).
macro_rules! props {
    ($( $(#[$m:meta])* $name:ident : $kind:ident $(<$ty:ty>)? ),* $(,)?) => {
        $( $crate::props!(@one [$(#[$m])*] $name $kind $($ty)?); )*
    };
    (@one [$(#[$m:meta])*] $name:ident text) => {
        $(#[$m])*
        pub fn $name(mut self, v: impl Into<String>) -> Self {
            self.$name = Some(v.into());
            self
        }
    };
    (@one [$(#[$m:meta])*] $name:ident maybe) => {
        $(#[$m])*
        pub fn $name(mut self, v: impl $crate::MaybeText) -> Self {
            self.$name = v.into_text();
            self
        }
    };
    (@one [$(#[$m:meta])*] $name:ident node) => {
        $(#[$m])*
        pub fn $name(mut self, v: impl next_rust_view::View) -> Self {
            self.$name = Some(v.into_node());
            self
        }
    };
    (@one [$(#[$m:meta])*] $name:ident val $ty:ty) => {
        $(#[$m])*
        pub fn $name(mut self, v: $ty) -> Self {
            self.$name = v;
            self
        }
    };
    (@one [$(#[$m:meta])*] $name:ident opt $ty:ty) => {
        $(#[$m])*
        pub fn $name(mut self, v: $ty) -> Self {
            self.$name = Some(v);
            self
        }
    };
}
pub(crate) use props;

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

/// Build a [`BarChart`](struct@crate::BarChart): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! BarChart {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::BarChart::new()] $($body)*) };
}

/// Build a [`Donut`](struct@crate::Donut): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Donut {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Donut::new()] $($body)*) };
}

/// Build a [`LineChart`](struct@crate::LineChart): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! LineChart {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::LineChart::new()] $($body)*) };
}

/// Build a [`Sparkline`](struct@crate::Sparkline): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Sparkline {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Sparkline::new()] $($body)*) };
}

/// Build a [`CommandGroup`](struct@crate::CommandGroup): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CommandGroup {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CommandGroup::new()] $($body)*) };
}

/// Build a [`CommandItem`](struct@crate::CommandItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CommandItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CommandItem::new()] $($body)*) };
}

/// Build a [`CommandPalette`](struct@crate::CommandPalette): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CommandPalette {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CommandPalette::new()] $($body)*) };
}

/// Build a [`Banner`](struct@crate::Banner): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Banner {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Banner::new()] $($body)*) };
}

/// Build a [`Callout`](struct@crate::Callout): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Callout {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Callout::new()] $($body)*) };
}

/// Build a [`ChipGroup`](struct@crate::ChipGroup): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ChipGroup {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ChipGroup::new()] $($body)*) };
}

/// Build a [`Code`](struct@crate::Code): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Code {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Code::new()] $($body)*) };
}

/// Build a [`CodeBlock`](struct@crate::CodeBlock): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CodeBlock {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CodeBlock::new()] $($body)*) };
}

/// Build a [`DescriptionItem`](struct@crate::DescriptionItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! DescriptionItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::DescriptionItem::new()] $($body)*) };
}

/// Build a [`DescriptionList`](struct@crate::DescriptionList): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! DescriptionList {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::DescriptionList::new()] $($body)*) };
}

/// Build a [`Figure`](struct@crate::Figure): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Figure {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Figure::new()] $($body)*) };
}

/// Build a [`List`](struct@crate::List): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! List {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::List::new()] $($body)*) };
}

/// Build a [`ListItem`](struct@crate::ListItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ListItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ListItem::new()] $($body)*) };
}

/// Build a [`Meter`](struct@crate::Meter): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Meter {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Meter::new()] $($body)*) };
}

/// Build a [`ResultBlock`](struct@crate::ResultBlock): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ResultBlock {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ResultBlock::new()] $($body)*) };
}

/// Build a [`SortHeader`](struct@crate::SortHeader): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! SortHeader {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::SortHeader::new()] $($body)*) };
}

/// Build a [`TableSelect`](struct@crate::TableSelect): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! TableSelect {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::TableSelect::new()] $($body)*) };
}

/// Build a [`TableSelectAll`](struct@crate::TableSelectAll): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! TableSelectAll {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::TableSelectAll::new()] $($body)*) };
}

/// Build a [`VideoEmbed`](struct@crate::VideoEmbed): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! VideoEmbed {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::VideoEmbed::new()] $($body)*) };
}

/// Build a [`ConfirmDialog`](struct@crate::ConfirmDialog): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ConfirmDialog {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ConfirmDialog::new()] $($body)*) };
}

/// Build a [`Drawer`](struct@crate::Drawer): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Drawer {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Drawer::new()] $($body)*) };
}

/// Build a [`LoadingOverlay`](struct@crate::LoadingOverlay): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! LoadingOverlay {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::LoadingOverlay::new()] $($body)*) };
}

/// Build a [`ColorPicker`](struct@crate::ColorPicker): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ColorPicker {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ColorPicker::new()] $($body)*) };
}

/// Build a [`Combobox`](struct@crate::Combobox): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Combobox {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Combobox::new()] $($body)*) };
}

/// Build a [`DateRangePicker`](struct@crate::DateRangePicker): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! DateRangePicker {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::DateRangePicker::new()] $($body)*) };
}

/// Build a [`FileUpload`](struct@crate::FileUpload): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FileUpload {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FileUpload::new()] $($body)*) };
}

/// Build a [`FormActions`](struct@crate::FormActions): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FormActions {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FormActions::new()] $($body)*) };
}

/// Build a [`FormField`](struct@crate::FormField): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FormField {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FormField::new()] $($body)*) };
}

/// Build a [`FormSection`](struct@crate::FormSection): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FormSection {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FormSection::new()] $($body)*) };
}

/// Build a [`NumberInput`](struct@crate::NumberInput): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! NumberInput {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::NumberInput::new()] $($body)*) };
}

/// Build a [`PinInput`](struct@crate::PinInput): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! PinInput {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::PinInput::new()] $($body)*) };
}

/// Build a [`Rating`](struct@crate::Rating): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Rating {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Rating::new()] $($body)*) };
}

/// Build a [`SearchInput`](struct@crate::SearchInput): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! SearchInput {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::SearchInput::new()] $($body)*) };
}

/// Build a [`Segment`](struct@crate::Segment): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Segment {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Segment::new()] $($body)*) };
}

/// Build a [`SegmentedControl`](struct@crate::SegmentedControl): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! SegmentedControl {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::SegmentedControl::new()] $($body)*) };
}

/// Build a [`Slider`](struct@crate::Slider): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Slider {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Slider::new()] $($body)*) };
}

/// Build a [`TagsInput`](struct@crate::TagsInput): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! TagsInput {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::TagsInput::new()] $($body)*) };
}

/// Build a [`TimePicker`](struct@crate::TimePicker): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! TimePicker {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::TimePicker::new()] $($body)*) };
}

/// Build a [`ContextMenu`](struct@crate::ContextMenu): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ContextMenu {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ContextMenu::new()] $($body)*) };
}

/// Build a [`Dropdown`](struct@crate::Dropdown): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Dropdown {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Dropdown::new()] $($body)*) };
}

/// Build a [`DropdownItem`](struct@crate::DropdownItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! DropdownItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::DropdownItem::new()] $($body)*) };
}

/// Build a [`DropdownSection`](struct@crate::DropdownSection): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! DropdownSection {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::DropdownSection::new()] $($body)*) };
}

/// Build a [`Popover`](struct@crate::Popover): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Popover {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Popover::new()] $($body)*) };
}

/// Build an [`AspectRatio`](struct@crate::AspectRatio): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! AspectRatio {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::AspectRatio::new()] $($body)*) };
}

/// Build a [`Bento`](struct@crate::Bento): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Bento {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Bento::new()] $($body)*) };
}

/// Build a [`BentoItem`](struct@crate::BentoItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! BentoItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::BentoItem::new()] $($body)*) };
}

/// Build a [`Carousel`](struct@crate::Carousel): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Carousel {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Carousel::new()] $($body)*) };
}

/// Build a [`CarouselItem`](struct@crate::CarouselItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! CarouselItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::CarouselItem::new()] $($body)*) };
}

/// Build a [`Cta`](struct@crate::Cta): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Cta {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Cta::new()] $($body)*) };
}

/// Build a [`Faq`](struct@crate::Faq): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Faq {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Faq::new()] $($body)*) };
}

/// Build a [`Feature`](struct@crate::Feature): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Feature {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Feature::new()] $($body)*) };
}

/// Build a [`FeatureGrid`](struct@crate::FeatureGrid): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FeatureGrid {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FeatureGrid::new()] $($body)*) };
}

/// Build a [`Footer`](struct@crate::Footer): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Footer {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Footer::new()] $($body)*) };
}

/// Build a [`FooterColumn`](struct@crate::FooterColumn): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FooterColumn {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FooterColumn::new()] $($body)*) };
}

/// Build a [`FooterLink`](struct@crate::FooterLink): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! FooterLink {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::FooterLink::new()] $($body)*) };
}

/// Build a [`Hero`](struct@crate::Hero): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Hero {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Hero::new()] $($body)*) };
}

/// Build a [`Hide`](struct@crate::Hide): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Hide {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Hide::new()] $($body)*) };
}

/// Build a [`LogoCloud`](struct@crate::LogoCloud): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! LogoCloud {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::LogoCloud::new()] $($body)*) };
}

/// Build a [`Marquee`](struct@crate::Marquee): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Marquee {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Marquee::new()] $($body)*) };
}

/// Build a [`Pagination`](struct@crate::Pagination): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Pagination {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Pagination::new()] $($body)*) };
}

/// Build a [`PricingCard`](struct@crate::PricingCard): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! PricingCard {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::PricingCard::new()] $($body)*) };
}

/// Build a [`PricingFeature`](struct@crate::PricingFeature): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! PricingFeature {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::PricingFeature::new()] $($body)*) };
}

/// Build a [`PricingTable`](struct@crate::PricingTable): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! PricingTable {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::PricingTable::new()] $($body)*) };
}

/// Build a [`ScrollArea`](struct@crate::ScrollArea): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! ScrollArea {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::ScrollArea::new()] $($body)*) };
}

/// Build a [`Section`](struct@crate::Section): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Section {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Section::new()] $($body)*) };
}

/// Build a [`Show`](struct@crate::Show): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Show {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Show::new()] $($body)*) };
}

/// Build a [`Spacer`](struct@crate::Spacer): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Spacer {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Spacer::new()] $($body)*) };
}

/// Build a [`Sticky`](struct@crate::Sticky): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Sticky {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Sticky::new()] $($body)*) };
}

/// Build a [`Testimonial`](struct@crate::Testimonial): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Testimonial {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Testimonial::new()] $($body)*) };
}

/// Build a [`Timeline`](struct@crate::Timeline): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Timeline {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Timeline::new()] $($body)*) };
}

/// Build a [`TimelineItem`](struct@crate::TimelineItem): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! TimelineItem {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::TimelineItem::new()] $($body)*) };
}

/// Build a [`Toast`](struct@crate::Toast): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Toast {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Toast::new()] $($body)*) };
}

/// Build a [`Toaster`](struct@crate::Toaster): `key = value` sets a property, anything else is
/// added as an attribute or child.
#[macro_export]
macro_rules! Toaster {
    ($($body:tt)*) => { $crate::__ui_munch!([$crate::Toaster::new()] $($body)*) };
}
