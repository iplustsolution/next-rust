//! The command palette.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::{Extra, Press, apply, attr, flag, next_id, part, props, root, styled};

/// A searchable list of links and actions in a dialog, opened with a
/// keyboard shortcut (`⌘K` / `Ctrl K` by default) or
/// [`Press::open_modal`](crate::Press::open_modal).
///
/// The component script filters as you type and moves through results with
/// the arrow keys. Without it the dialog still lists every command as a
/// link you can tab to.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = CommandPalette![id = "commands", placeholder = "Search…",
///     CommandGroup![title = "Pages", CommandItem![href = "/docs", keywords = "guide manual", "Documentation"]],
/// ];
/// let html = render_static(c);
/// assert!(html.contains(r#"data-nr-ui="command""#) && html.contains(r#"data-hotkey="k""#));
/// assert!(html.contains(r#"role="option""#) && html.contains(r#"data-keywords="guide manual""#), "{html}");
/// ```
#[derive(Default)]
pub struct CommandPalette {
    id: Option<String>,
    placeholder: Option<String>,
    empty: Option<String>,
    hotkey: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self { hotkey: Some("k".to_owned()), ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        /// The id [`Press::open_modal`](crate::Press::open_modal) refers to.
        id: text,
        /// Placeholder of the search box.
        placeholder: text,
        /// Shown when nothing matches ("No results" by default).
        empty: text,
    }
    /// The key that opens it together with `⌘` or `Ctrl` (`"k"` by default); `""` for none.
    pub fn hotkey(mut self, v: impl Into<String>) -> Self {
        self.hotkey = Some(v.into()).filter(|k| !k.is_empty());
        self
    }
}

impl View for CommandPalette {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let id = self.id.unwrap_or_else(|| next_id("nr-command"));
        let placeholder = self.placeholder.unwrap_or_else(|| "Type a command or search…".to_owned());
        let mut el = root("dialog", &["nr-modal", "nr-command"], self.unstyled, class);
        el.set_attr(attr("id", id.clone()));
        el.set_attr(attr("data-nr-ui", "command"));
        el.set_attr(flag("data-dismissable", true));
        el.set_attr(attr("aria-label", "Command palette"));
        if let Some(hotkey) = self.hotkey {
            el.set_attr(attr("data-hotkey", hotkey));
        }
        apply(&mut el, attrs);
        let search = part("div", "nr-command-search")
            .with(icon(icons::search()))
            .with(
                Element::new_void("input")
                    .with(attr("class", "nr-command-input"))
                    .with(attr("type", "search"))
                    .with(attr("role", "combobox"))
                    .with(attr("aria-expanded", "true"))
                    .with(attr("aria-controls", format!("{id}-list")))
                    .with(attr("aria-autocomplete", "list"))
                    .with(attr("aria-label", placeholder.clone()))
                    .with(attr("placeholder", placeholder))
                    .with(attr("autocomplete", "off"))
                    .with(attr("spellcheck", "false"))
                    .with(flag("autofocus", true)),
            )
            .with(
                Element::new("form").with(attr("method", "dialog")).with(attr("class", "nr-modal-closer")).with(
                    Element::new("button")
                        .with(attr("type", "submit"))
                        .with(attr("class", "nr-command-esc"))
                        .with(attr("aria-label", "Close"))
                        .with("Esc"),
                ),
            );
        let mut list = part("div", "nr-command-list")
            .with(attr("id", format!("{id}-list")))
            .with(attr("role", "listbox"))
            .with(attr("aria-label", "Commands"));
        list.children = children;
        let empty = part("p", "nr-command-empty")
            .with(flag("hidden", true))
            .with(self.empty.unwrap_or_else(|| "No results".to_owned()));
        el.children.push(part("div", "nr-command-box").with(search).with(list).with(empty).into_node());
        styled(el)
    }
}

/// A titled group of [`CommandItem`]s.
#[derive(Default)]
pub struct CommandGroup {
    title: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl CommandGroup {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
    }
}

impl View for CommandGroup {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-command-group"], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        apply(&mut el, attrs);
        if let Some(title) = self.title {
            el.set_attr(attr("aria-label", title.clone()));
            el.children
                .push(part("div", "nr-command-heading").with(attr("aria-hidden", "true")).with(title).into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

/// A command: a link (`href`) or a press (`on_press`).
#[derive(Default)]
pub struct CommandItem {
    href: Option<String>,
    press: Option<Press>,
    keywords: Option<String>,
    start_content: Option<Node>,
    shortcut: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl CommandItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        href: text,
        /// Extra words the search matches, besides the label.
        keywords: text,
        /// Content before the label, typically an icon.
        start_content: node,
        /// A keyboard hint at the end; display only.
        shortcut: text,
    }
    /// What choosing the command does: `on_press = action!(sign_out)`, or a [`Press`].
    pub fn on_press(mut self, v: impl Into<Press>) -> Self {
        self.press = Some(v.into());
        self
    }
}

impl View for CommandItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let tag = if self.href.is_some() { "a" } else { "button" };
        let mut el = root(tag, &["nr-command-item"], self.unstyled, class);
        el.set_attr(attr("role", "option"));
        el.set_attr(attr("id", next_id("nr-command-item")));
        el.set_attr(attr("aria-selected", "false"));
        match self.href {
            Some(href) => {
                el.set_attr(attr("href", href));
                el.set_attr(attr("data-nr-link", ""));
            }
            None => el.set_attr(attr("type", "button")),
        }
        if let Some(press) = &self.press {
            apply(&mut el, press.attrs());
        }
        if let Some(keywords) = self.keywords {
            el.set_attr(attr("data-keywords", keywords));
        }
        apply(&mut el, attrs);
        if let Some(start) = self.start_content {
            el.children.push(part("span", "nr-command-start").with(start).into_node());
        }
        el.children.push(part("span", "nr-command-label").with(children).into_node());
        if let Some(shortcut) = self.shortcut {
            el.children.push(part("kbd", "nr-command-shortcut").with(shortcut).into_node());
        }
        styled(el)
    }
}
