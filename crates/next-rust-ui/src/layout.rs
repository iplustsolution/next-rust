//! Layout: container, stack, grid, navbar, sidebar and a whole app shell.

use next_rust_view::{Element, Node, View, attrs::active_class, attrs::active_class_prefix};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::{Extra, apply, attr, root, styled};

/// Maximum width of a [`Container`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Width {
    /// 40rem
    Sm,
    /// 48rem
    Md,
    /// 64rem
    Lg,
    /// 80rem
    #[default]
    Xl,
    /// 96rem
    Xxl,
    /// No limit.
    Full,
}

impl Width {
    fn class(self) -> &'static str {
        match self {
            Width::Sm => "nr-container-sm",
            Width::Md => "nr-container-md",
            Width::Lg => "nr-container-lg",
            Width::Xl => "nr-container-xl",
            Width::Xxl => "nr-container-2xl",
            Width::Full => "nr-container-full",
        }
    }
}

/// Cross-axis alignment of a [`Stack`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}

/// Main-axis distribution of a [`Stack`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

/// Gap classes exist for these steps (in quarters of a rem); others round down.
const GAPS: [u8; 11] = [0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16];

fn gap_class(n: u8) -> String {
    let step = GAPS.iter().rev().find(|g| **g <= n).copied().unwrap_or(0);
    format!("nr-gap-{step}")
}

/// Centered content with a maximum width and side padding.
#[derive(Default)]
pub struct Container {
    width: Width,
    unstyled: bool,
    extra: Extra,
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn width(mut self, v: Width) -> Self {
        self.width = v;
        self
    }
}

impl View for Container {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-container", self.width.class()], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// Children in a column (or a row) with even spacing.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Stack![row = true, gap = 4, align = Align::Center, "a", "b"]);
/// assert!(html.ends_with(r#"<div class="nr-stack nr-stack-row nr-gap-4 nr-items-center">ab</div>"#));
/// ```
#[derive(Default)]
pub struct Stack {
    row: bool,
    wrap: bool,
    gap: Option<u8>,
    align: Option<Align>,
    justify: Option<Justify>,
    unstyled: bool,
    extra: Extra,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// Lay children out horizontally.
    pub fn row(mut self, v: bool) -> Self {
        self.row = v;
        self
    }
    /// Let children wrap onto new lines.
    pub fn wrap(mut self, v: bool) -> Self {
        self.wrap = v;
        self
    }
    /// Space between children, in quarters of a rem (`4` = 1rem). Default 4.
    pub fn gap(mut self, v: u8) -> Self {
        self.gap = Some(v);
        self
    }
    pub fn align(mut self, v: Align) -> Self {
        self.align = Some(v);
        self
    }
    pub fn justify(mut self, v: Justify) -> Self {
        self.justify = Some(v);
        self
    }
}

impl View for Stack {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let gap = gap_class(self.gap.unwrap_or(4));
        let align = match self.align {
            None => "",
            Some(Align::Start) => "nr-items-start",
            Some(Align::Center) => "nr-items-center",
            Some(Align::End) => "nr-items-end",
            Some(Align::Stretch) => "nr-items-stretch",
            Some(Align::Baseline) => "nr-items-baseline",
        };
        let justify = match self.justify {
            None => "",
            Some(Justify::Start) => "nr-justify-start",
            Some(Justify::Center) => "nr-justify-center",
            Some(Justify::End) => "nr-justify-end",
            Some(Justify::Between) => "nr-justify-between",
            Some(Justify::Around) => "nr-justify-around",
            Some(Justify::Evenly) => "nr-justify-evenly",
        };
        let classes = [
            "nr-stack",
            if self.row { "nr-stack-row" } else { "" },
            if self.wrap { "nr-stack-wrap" } else { "" },
            &gap,
            align,
            justify,
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A responsive grid: `cols` columns on wide screens, fewer on narrow ones.
/// With `min_width`, as many columns as fit.
#[derive(Default)]
pub struct Grid {
    cols: Option<u8>,
    gap: Option<u8>,
    min_width: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Grid {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// Columns on wide screens (1–12, default 3). Two below 1024px, one below 640px.
    pub fn cols(mut self, v: u8) -> Self {
        self.cols = Some(v.clamp(1, 12));
        self
    }
    /// Space between cells, in quarters of a rem. Default 4.
    pub fn gap(mut self, v: u8) -> Self {
        self.gap = Some(v);
        self
    }
    /// Fit as many columns of at least this width as there is room for (`"16rem"`).
    pub fn min_width(mut self, v: impl Into<String>) -> Self {
        self.min_width = Some(v.into());
        self
    }
}

impl View for Grid {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let cols = format!("nr-cols-{}", self.cols.unwrap_or(3));
        let gap = gap_class(self.gap.unwrap_or(4));
        let mut el = if let Some(min) = &self.min_width {
            let mut el = root("div", &["nr-grid", "nr-grid-fit", &gap], self.unstyled, class);
            let min: String = min.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '%')).collect();
            el.set_attr(attr("style", format!("--nr-grid-min:{min}")));
            el
        } else {
            root("div", &["nr-grid", &cols, &gap], self.unstyled, class)
        };
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A top bar: brand on the left, links in the middle, actions on the right.
#[derive(Default)]
pub struct Navbar {
    brand: Option<Node>,
    end_content: Option<Node>,
    sticky: Option<bool>,
    blurred: Option<bool>,
    bordered: bool,
    width: Width,
    menu_toggle: bool,
    unstyled: bool,
    extra: Extra,
}

impl Navbar {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// Logo or site name.
    pub fn brand(mut self, v: impl View) -> Self {
        self.brand = Some(v.into_node());
        self
    }
    /// Actions on the right (buttons, an avatar).
    pub fn end_content(mut self, v: impl View) -> Self {
        self.end_content = Some(v.into_node());
        self
    }
    /// Stay at the top while scrolling (default true).
    pub fn sticky(mut self, v: bool) -> Self {
        self.sticky = Some(v);
        self
    }
    /// Translucent with a background blur (default true).
    pub fn blurred(mut self, v: bool) -> Self {
        self.blurred = Some(v);
        self
    }
    /// A line under the bar.
    pub fn bordered(mut self, v: bool) -> Self {
        self.bordered = v;
        self
    }
    /// Width of the bar's content.
    pub fn width(mut self, v: Width) -> Self {
        self.width = v;
        self
    }
    /// A menu button (small screens) that opens the [`AppShell`] sidebar.
    pub fn menu_toggle(mut self, v: bool) -> Self {
        self.menu_toggle = v;
        self
    }
}

impl View for Navbar {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-navbar",
            if self.sticky.unwrap_or(true) { "nr-navbar-sticky" } else { "" },
            if self.blurred.unwrap_or(true) { "nr-navbar-blurred" } else { "" },
            if self.bordered { "nr-navbar-bordered" } else { "" },
        ];
        let mut el = root("header", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let mut inner =
            Element::new("div").with(attr("class", format!("nr-navbar-inner nr-container {}", self.width.class())));
        if self.menu_toggle {
            el.set_attr(attr("data-nr-ui", "shell"));
            inner.children.push(
                Element::new("button")
                    .with(attr("type", "button"))
                    .with(attr("class", "nr-navbar-menu"))
                    .with(attr("data-nr-shell-toggle", ""))
                    .with(attr("aria-label", "Menu"))
                    .with(attr("aria-expanded", "false"))
                    .with(icon(icons::menu()))
                    .into_node(),
            );
        }
        if let Some(brand) = self.brand {
            inner.children.push(Element::new("div").with(attr("class", "nr-navbar-brand")).with(brand).into_node());
        }
        if !children.is_empty() {
            inner
                .children
                .push(Element::new("nav").with(attr("class", "nr-navbar-content")).with(children).into_node());
        }
        if let Some(end) = self.end_content {
            inner.children.push(Element::new("div").with(attr("class", "nr-navbar-end")).with(end).into_node());
        }
        el.children.push(inner.into_node());
        styled(el)
    }
}

/// A navbar link, highlighted on its page.
#[derive(Default)]
pub struct NavbarItem {
    href: Option<String>,
    prefix: bool,
    unstyled: bool,
    extra: Extra,
}

impl NavbarItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    /// Also highlighted on pages below `href` (`/docs` on `/docs/intro`).
    pub fn prefix(mut self, v: bool) -> Self {
        self.prefix = v;
        self
    }
}

impl View for NavbarItem {
    fn into_node(self) -> Node {
        nav_link("nr-navbar-link", self.href, self.prefix, None, self.unstyled, self.extra)
    }
}

fn nav_link(
    kind: &'static str,
    href: Option<String>,
    prefix: bool,
    icon: Option<Node>,
    unstyled: bool,
    extra: Extra,
) -> Node {
    let (class, attrs, children) = extra.split();
    let mut el = root("a", &[kind], unstyled, class);
    el.set_attr(attr("href", href.unwrap_or_else(|| "#".into())));
    let active = format!("{kind}-active");
    el.set_attr(if prefix { active_class_prefix(active) } else { active_class(active) });
    apply(&mut el, attrs);
    if let Some(icon) = icon {
        el.children.push(Element::new("span").with(attr("class", "nr-sidebar-icon")).with(icon).into_node());
    }
    el.children.push(Element::new("span").with(attr("class", "nr-nav-text")).with(children).into_node());
    styled(el)
}

/// A vertical navigation list.
#[derive(Default)]
pub struct Sidebar {
    title: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Sidebar {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// A small heading above the items.
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
}

impl View for Sidebar {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("nav", &["nr-sidebar"], self.unstyled, class);
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.children.push(Element::new("p").with(attr("class", "nr-sidebar-title")).with(title).into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

/// A sidebar link with an optional icon, highlighted on its page.
#[derive(Default)]
pub struct SidebarItem {
    href: Option<String>,
    icon: Option<Node>,
    prefix: bool,
    unstyled: bool,
    extra: Extra,
}

impl SidebarItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    pub fn icon(mut self, v: impl View) -> Self {
        self.icon = Some(v.into_node());
        self
    }
    /// Also highlighted on pages below `href`.
    pub fn prefix(mut self, v: bool) -> Self {
        self.prefix = v;
        self
    }
}

impl View for SidebarItem {
    fn into_node(self) -> Node {
        nav_link("nr-sidebar-link", self.href, self.prefix, self.icon, self.unstyled, self.extra)
    }
}

/// A full page layout: navbar on top, sidebar on the left (a drawer on small
/// screens, opened by the navbar's `menu_toggle`), content in the middle.
///
/// ```ignore
/// AppShell![
///     navbar = Navbar![brand = strong!["Acme"], menu_toggle = true],
///     sidebar = Sidebar![SidebarItem![href = "/", "Home"]],
///     children,
/// ]
/// ```
#[derive(Default)]
pub struct AppShell {
    navbar: Option<Node>,
    sidebar: Option<Node>,
    footer: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl AppShell {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn navbar(mut self, v: impl View) -> Self {
        self.navbar = Some(v.into_node());
        self
    }
    pub fn sidebar(mut self, v: impl View) -> Self {
        self.sidebar = Some(v.into_node());
        self
    }
    pub fn footer(mut self, v: impl View) -> Self {
        self.footer = Some(v.into_node());
        self
    }
}

impl View for AppShell {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root(
            "div",
            &["nr-shell", if self.sidebar.is_some() { "nr-shell-with-sidebar" } else { "" }],
            self.unstyled,
            class,
        );
        apply(&mut el, attrs);
        el.children.extend(self.navbar);
        let mut body = Element::new("div").with(attr("class", "nr-shell-body"));
        if let Some(sidebar) = self.sidebar {
            body.children.push(Element::new("aside").with(attr("class", "nr-shell-sidebar")).with(sidebar).into_node());
            body.children.push(
                Element::new("div")
                    .with(attr("class", "nr-shell-scrim"))
                    .with(attr("data-nr-shell-toggle", ""))
                    .into_node(),
            );
        }
        body.children.push(Element::new("main").with(attr("class", "nr-shell-main")).with(children).into_node());
        el.children.push(body.into_node());
        if let Some(footer) = self.footer {
            el.children.push(Element::new("footer").with(attr("class", "nr-shell-footer")).with(footer).into_node());
        }
        styled(el)
    }
}
