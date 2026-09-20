//! Menus and popovers, built on the browser's `popover` attribute: they open,
//! close on a click outside or `Escape`, and return focus with no script.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::display::Side;
use crate::icons::{self, icon};
use crate::{
    Button, Color, Extra, Press, Variant, apply, attr, class_list, first_element, flag, next_id, part, props, root,
    styled,
};

/// Which edge of the trigger a menu lines up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAlign {
    #[default]
    Start,
    End,
}

/// Point `trigger` (its first element) at the popover `id`.
fn wire_trigger(trigger: &mut [Node], id: &str, popup: &'static str) {
    if let Some(el) = first_element(trigger) {
        el.set_attr(attr("popovertarget", id.to_owned()));
        el.set_attr(attr("aria-haspopup", popup));
        el.set_attr(attr("aria-expanded", "false"));
        el.set_attr(attr("aria-controls", id.to_owned()));
    }
}

fn side_class(side: Side) -> &'static str {
    match side {
        Side::Top => "nr-pop-top",
        Side::Bottom => "",
        Side::Left => "nr-pop-left",
        Side::Right => "nr-pop-right",
    }
}

/// A menu of actions that opens from a button.
///
/// The menu is a native popover: it opens, closes on a click outside or
/// `Escape` and returns focus without any script, and sits next to its
/// trigger through CSS anchor positioning. The component script adds arrow
/// keys, `Home`/`End`, type-ahead, and positions the menu in browsers
/// without anchor positioning.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let d = Dropdown![trigger = Button![variant = Variant::Bordered, "Options"],
///     DropdownItem![href = "/settings", "Settings"],
///     DropdownItem![color = Color::Danger, on_press = Press::emit("delete"), "Delete"],
/// ];
/// let html = render_static(d);
/// assert!(html.contains(r#"aria-haspopup="menu""#) && html.contains(r#"role="menu""#));
/// assert!(html.contains(r#"<a class="nr-menu-item nr-c-default" role="menuitem" href="/settings""#), "{html}");
/// ```
#[derive(Default)]
pub struct Dropdown {
    trigger: Option<Node>,
    label: Option<String>,
    align: MenuAlign,
    side: Side,
    id: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Dropdown {
    pub fn new() -> Self {
        Self { side: Side::Bottom, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// What opens the menu, typically a `Button!`. A "more" icon button when not given.
        trigger: node,
        /// Accessible name of the menu (and of the default trigger).
        label: text,
        /// `Start` (default) or `End` edge of the trigger.
        align: val<MenuAlign>,
        /// `Bottom` (default), `Top`, `Left` or `Right` of the trigger; flips when there is no room.
        side: val<Side>,
        /// `id` of the menu (generated when not given).
        id: text,
    }
}

impl View for Dropdown {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let id = self.id.unwrap_or_else(|| next_id("nr-menu"));
        let label = self.label.unwrap_or_else(|| "Menu".to_owned());
        let mut el = root("div", &["nr-dropdown"], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "dropdown"));
        apply(&mut el, attrs);
        let mut trigger = vec![self.trigger.unwrap_or_else(|| {
            Button::new()
                .icon_only(true)
                .variant(Variant::Light)
                .color(Color::Default)
                .with(attr("aria-label", label.clone()))
                .with(icon(icons::ellipsis()))
                .into_node()
        })];
        wire_trigger(&mut trigger, &id, "menu");
        el.children = trigger;
        let align = if self.align == MenuAlign::End { "nr-pop-end" } else { "" };
        let mut menu = Element::new("div")
            .with(class_list(&["nr-pop", "nr-menu", side_class(self.side), align]))
            .with(attr("id", id))
            .with(flag("popover", true))
            .with(attr("role", "menu"))
            .with(attr("aria-label", label));
        menu.set_attr(attr("data-nr-align", if self.align == MenuAlign::End { "end" } else { "start" }));
        menu.children = children;
        el.children.push(menu.into_node());
        styled(el)
    }
}

/// An entry of a [`Dropdown`] or [`ContextMenu`]: a link (`href`), a press
/// (`on_press`) or a submit button inside a form (`submit`).
#[derive(Default)]
pub struct DropdownItem {
    href: Option<String>,
    press: Option<Press>,
    submit: bool,
    color: Color,
    description: Option<String>,
    start_content: Option<Node>,
    end_content: Option<Node>,
    shortcut: Option<String>,
    disabled: bool,
    unstyled: bool,
    extra: Extra,
}

impl DropdownItem {
    pub fn new() -> Self {
        Self { color: Color::Default, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// Render a link (client-side navigation).
        href: text,
        /// `type="submit"`, for an item inside a `form!` (sign out).
        submit: val<bool>,
        /// Color of the item; `Danger` for destructive actions.
        color: val<Color>,
        /// A second, muted line.
        description: text,
        /// Content before the label, typically an icon.
        start_content: node,
        /// Content at the end.
        end_content: node,
        /// A keyboard hint at the end ("⌘K"); display only.
        shortcut: text,
        disabled: val<bool>,
    }
    /// What choosing the item does: `on_press = action!(archive)`, or a [`Press`].
    pub fn on_press(mut self, v: impl Into<Press>) -> Self {
        self.press = Some(v.into());
        self
    }
}

impl View for DropdownItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let tag = if self.href.is_some() { "a" } else { "button" };
        let mut el = root(tag, &["nr-menu-item", self.color.class()], self.unstyled, class);
        el.set_attr(attr("role", "menuitem"));
        match self.href {
            Some(href) => {
                el.set_attr(attr("href", href));
                el.set_attr(attr("data-nr-link", ""));
                if self.disabled {
                    el.set_attr(attr("aria-disabled", "true"));
                    el.set_attr(attr("tabindex", "-1"));
                }
            }
            None => {
                el.set_attr(attr("type", if self.submit { "submit" } else { "button" }));
                el.set_attr(flag("disabled", self.disabled));
            }
        }
        if let Some(press) = &self.press {
            apply(&mut el, press.attrs());
        }
        apply(&mut el, attrs);
        if let Some(start) = self.start_content {
            el.children.push(part("span", "nr-menu-start").with(start).into_node());
        }
        let mut label = part("span", "nr-menu-label");
        label.children = children;
        if let Some(description) = self.description {
            label.children.push(part("span", "nr-menu-description").with(description).into_node());
        }
        el.children.push(label.into_node());
        if let Some(shortcut) = self.shortcut {
            el.children.push(part("kbd", "nr-menu-shortcut").with(shortcut).into_node());
        }
        if let Some(end) = self.end_content {
            el.children.push(part("span", "nr-menu-end").with(end).into_node());
        }
        styled(el)
    }
}

/// A titled group of [`DropdownItem`]s, separated from its neighbors by a line.
#[derive(Default)]
pub struct DropdownSection {
    title: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl DropdownSection {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// A small heading above the items.
        title: text,
    }
}

impl View for DropdownSection {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-menu-section"], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.set_attr(attr("aria-label", title.clone()));
            el.children.push(part("div", "nr-menu-title").with(attr("aria-hidden", "true")).with(title).into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

/// Any content in a small panel that opens from a trigger: a native popover,
/// placed next to the trigger with CSS anchor positioning (the component
/// script positions it where that is missing).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let p = Popover![trigger = Button!["Share"], title = "Share this page", side = Side::Top, p!["Anyone with the link can view."]];
/// let html = render_static(p);
/// assert!(html.contains("popovertarget=") && html.contains(r#"class="nr-pop nr-popover nr-pop-top""#), "{html}");
/// ```
#[derive(Default)]
pub struct Popover {
    trigger: Option<Node>,
    title: Option<String>,
    side: Side,
    align: MenuAlign,
    id: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Popover {
    pub fn new() -> Self {
        Self { side: Side::Bottom, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// What opens the popover, typically a `Button!`.
        trigger: node,
        /// A heading (also the accessible name).
        title: text,
        /// `Bottom` (default), `Top`, `Left` or `Right`; flips when there is no room.
        side: val<Side>,
        align: val<MenuAlign>,
        /// `id` of the panel (generated when not given).
        id: text,
    }
}

impl View for Popover {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let id = self.id.unwrap_or_else(|| next_id("nr-popover"));
        let mut el = root("span", &["nr-popover-anchor"], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "popover"));
        apply(&mut el, attrs);
        let mut trigger = vec![self.trigger.unwrap_or_else(|| Button::new().with("Open").into_node())];
        wire_trigger(&mut trigger, &id, "dialog");
        el.children = trigger;
        let align = if self.align == MenuAlign::End { "nr-pop-end" } else { "" };
        let mut panel = Element::new("div")
            .with(class_list(&["nr-pop", "nr-popover", side_class(self.side), align]))
            .with(attr("id", id.clone()))
            .with(flag("popover", true))
            .with(attr("role", "dialog"));
        panel.set_attr(attr("data-nr-align", if self.align == MenuAlign::End { "end" } else { "start" }));
        if let Some(title) = self.title {
            panel.set_attr(attr("aria-labelledby", format!("{id}-title")));
            panel.children.push(
                part("strong", "nr-popover-title").with(attr("id", format!("{id}-title"))).with(title).into_node(),
            );
        }
        panel.children.extend(children);
        el.children.push(panel.into_node());
        styled(el)
    }
}

/// A menu on right click (or the context-menu key) over its `target`. It
/// needs the component script; without it the browser's own menu shows.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = ContextMenu![target = Card![CardBody!["Right click me"]], DropdownItem![href = "/edit", "Edit"]];
/// assert!(render_static(c).contains(r#"data-nr-ui="context""#));
/// ```
#[derive(Default)]
pub struct ContextMenu {
    target: Option<Node>,
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// The area that answers to a right click.
        target: node,
        /// Accessible name of the menu.
        label: text,
    }
}

impl View for ContextMenu {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-context"], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "context"));
        apply(&mut el, attrs);
        el.children.extend(self.target);
        let mut menu = Element::new("div")
            .with(attr("class", "nr-pop nr-menu nr-pop-free"))
            .with(attr("id", next_id("nr-context")))
            .with(attr("popover", "manual"))
            .with(attr("role", "menu"))
            .with(attr("aria-label", self.label.unwrap_or_else(|| "Context menu".to_owned())));
        menu.children = children;
        el.children.push(menu.into_node());
        styled(el)
    }
}
