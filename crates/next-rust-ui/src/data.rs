//! Data display: lists, description lists, code, callouts, banners, meters,
//! figures, video embeds, result blocks and table helpers. Everything is
//! rendered on the server; only the copy button of a code block and the
//! "select all" checkbox of a table use the component script.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::{Color, Extra, Radius, Size, apply, attr, class_list, flag, part, props, root, styled, trim_float};

/// Direction a column is sorted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    /// Not sorted by this column.
    #[default]
    None,
    Asc,
    Desc,
}

/// A sortable column header for [`Table`](struct@crate::Table)'s `header`: a
/// link the server answers with the sorted page, so sorting needs no script.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let h = SortHeader![href = "/students?sort=name&dir=desc", direction = SortDirection::Asc, "Name"];
/// let html = render_static(h);
/// assert!(html.contains(r#"<th class="nr-th-sort" scope="col" aria-sort="ascending">"#), "{html}");
/// ```
#[derive(Default)]
pub struct SortHeader {
    href: Option<String>,
    direction: SortDirection,
    unstyled: bool,
    extra: Extra,
}

impl SortHeader {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// The URL that sorts by this column (usually toggling the direction).
        href: text,
        /// How the table is sorted by this column right now.
        direction: val<SortDirection>,
    }
}

impl View for SortHeader {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut th = root("th", &["nr-th-sort"], self.unstyled, class);
        th.set_attr(attr("scope", "col"));
        let (sort, glyph) = match self.direction {
            SortDirection::None => ("none", icons::chevrons_up_down()),
            SortDirection::Asc => ("ascending", icons::arrow_up()),
            SortDirection::Desc => ("descending", icons::arrow_down()),
        };
        th.set_attr(attr("aria-sort", sort));
        apply(&mut th, attrs);
        let mut a = part("a", "nr-th-sort-link")
            .with(attr("href", self.href.unwrap_or_else(|| "#".to_owned())))
            .with(attr("data-nr-link", ""));
        a.children = children;
        a.children.push(icon(glyph));
        th.children.push(a.into_node());
        styled(th)
    }
}

/// A header cell with a checkbox that selects every [`TableSelect`] row of
/// its table (the component script does; each row's checkbox works alone).
#[derive(Default)]
pub struct TableSelectAll {
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl TableSelectAll {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Accessible name ("Select all rows" by default).
        label: text,
    }
}

impl View for TableSelectAll {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut th = root("th", &["nr-table-check"], self.unstyled, class);
        th.set_attr(attr("scope", "col"));
        apply(&mut th, attrs);
        th.children.push(
            Element::new_void("input")
                .with(attr("class", "nr-table-checkbox"))
                .with(attr("type", "checkbox"))
                .with(attr("data-nr-ui", "checkall"))
                .with(attr("aria-label", self.label.unwrap_or_else(|| "Select all rows".to_owned())))
                .into_node(),
        );
        styled(th)
    }
}

/// A row's selection checkbox: a cell with `<input type="checkbox" name value>`,
/// so a form around the table submits the chosen rows.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let row = tr![TableSelect![name = "ids", value = "42", label = "Select Ada"], td!["Ada"]];
/// assert!(render_static(row).contains(r#"type="checkbox" name="ids" value="42""#));
/// ```
#[derive(Default)]
pub struct TableSelect {
    name: Option<String>,
    value: Option<String>,
    checked: bool,
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl TableSelect {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        name: text,
        value: text,
        checked: val<bool>,
        /// Accessible name ("Select row" by default).
        label: text,
    }
}

impl View for TableSelect {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut td = root("td", &["nr-table-check"], self.unstyled, class);
        apply(&mut td, attrs);
        let mut input =
            Element::new_void("input").with(attr("class", "nr-table-checkbox")).with(attr("type", "checkbox"));
        if let Some(name) = self.name {
            input.set_attr(attr("name", name));
        }
        if let Some(value) = self.value {
            input.set_attr(attr("value", value));
        }
        input.set_attr(flag("checked", self.checked));
        input.set_attr(attr("aria-label", self.label.unwrap_or_else(|| "Select row".to_owned())));
        td.children.push(input.into_node());
        styled(td)
    }
}

/// A list of rows: people, files, settings.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let l = List![divided = true, ListItem![title = "Ada Lovelace", description = "Admin", href = "/users/ada"]];
/// let html = render_static(l);
/// assert!(html.contains(r#"<ul class="nr-list nr-list-divided">"#) && html.contains(r#"<a class="nr-list-row" href="/users/ada""#), "{html}");
/// ```
#[derive(Default)]
pub struct List {
    divided: bool,
    bordered: bool,
    compact: bool,
    unstyled: bool,
    extra: Extra,
}

impl List {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Lines between rows.
        divided: val<bool>,
        /// A border around the list.
        bordered: val<bool>,
        compact: val<bool>,
    }
}

impl View for List {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-list",
            if self.divided { "nr-list-divided" } else { "" },
            if self.bordered { "nr-list-bordered" } else { "" },
            if self.compact { "nr-list-compact" } else { "" },
        ];
        let mut el = root("ul", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A row of a [`List`]: content at the start (an avatar, an icon), a title
/// and a description, content at the end. With `href` the row is a link.
#[derive(Default)]
pub struct ListItem {
    title: Option<String>,
    description: Option<String>,
    href: Option<String>,
    start_content: Option<Node>,
    end_content: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl ListItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
        description: text,
        href: text,
        start_content: node,
        end_content: node,
    }
}

impl View for ListItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut li = root("li", &["nr-list-item"], self.unstyled, class);
        apply(&mut li, attrs);
        let mut row = match self.href {
            Some(href) => part("a", "nr-list-row").with(attr("href", href)).with(attr("data-nr-link", "")),
            None => part("div", "nr-list-row"),
        };
        if let Some(start) = self.start_content {
            row.children.push(part("span", "nr-list-start").with(start).into_node());
        }
        let mut main = part("span", "nr-list-main");
        if let Some(title) = self.title {
            main.children.push(part("span", "nr-list-title").with(title).into_node());
        }
        if let Some(description) = self.description {
            main.children.push(part("span", "nr-list-description").with(description).into_node());
        }
        main.children.extend(children);
        row.children.push(main.into_node());
        if let Some(end) = self.end_content {
            row.children.push(part("span", "nr-list-end").with(end).into_node());
        }
        li.children.push(row.into_node());
        styled(li)
    }
}

/// Terms and their values (`<dl>`): the details of an order, a profile.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let d = DescriptionList![horizontal = true, DescriptionItem![term = "Plan", "Pro"], DescriptionItem![term = "Seats", "12"]];
/// let html = render_static(d);
/// assert!(html.contains(r#"<dl class="nr-dl nr-dl-row">"#) && html.contains("<dt class=\"nr-dl-term\">Plan</dt>"), "{html}");
/// ```
#[derive(Default)]
pub struct DescriptionList {
    horizontal: bool,
    cols: Option<u8>,
    unstyled: bool,
    extra: Extra,
}

impl DescriptionList {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Term on the left, value on the right, a line between items.
        horizontal: val<bool>,
        /// Items side by side in 2 to 4 columns (term above value).
        cols: opt<u8>,
    }
}

impl View for DescriptionList {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let cols = self.cols.map(|c| format!("nr-dl-{}", c.clamp(2, 4))).unwrap_or_default();
        let mut el =
            root("dl", &["nr-dl", if self.horizontal { "nr-dl-row" } else { "" }, &cols], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A term and its value in a [`DescriptionList`].
#[derive(Default)]
pub struct DescriptionItem {
    term: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl DescriptionItem {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        term: text,
    }
}

impl View for DescriptionItem {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-dl-item"], self.unstyled, class);
        apply(&mut el, attrs);
        el.children.push(part("dt", "nr-dl-term").with(self.term.unwrap_or_default()).into_node());
        el.children.push(part("dd", "nr-dl-value").with(children).into_node());
        styled(el)
    }
}

/// Chips that wrap onto several lines with even spacing: tags, filters.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let g = ChipGroup![label = "Topics", Chip!["rust"], Chip!["web"]];
/// assert!(render_static(g).contains(r#"<div class="nr-chip-group" role="group" aria-label="Topics">"#));
/// ```
#[derive(Default)]
pub struct ChipGroup {
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl ChipGroup {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Accessible name of the group.
        label: text,
    }
}

impl View for ChipGroup {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-chip-group"], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        if let Some(label) = self.label {
            el.set_attr(attr("aria-label", label));
        }
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// Inline code.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// assert!(render_static(Code!["cargo run"]).contains(r#"<code class="nr-code">cargo run</code>"#));
/// ```
#[derive(Default)]
pub struct Code {
    unstyled: bool,
    extra: Extra,
}

impl Code {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
}

impl View for Code {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("code", &["nr-code"], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}

/// A block of code with an optional title and a copy button. The button
/// needs the component script and the clipboard; it is hidden without them.
/// Children are the code: plain text, or markup from your highlighter.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = CodeBlock![language = "rust", title = "src/main.rs", "fn main() {}"];
/// let html = render_static(c);
/// assert!(html.contains(r#"<code class="language-rust">fn main() {}</code>"#) && html.contains("data-nr-copy"), "{html}");
/// ```
#[derive(Default)]
pub struct CodeBlock {
    language: Option<String>,
    title: Option<String>,
    copy: Option<bool>,
    wrap: bool,
    unstyled: bool,
    extra: Extra,
}

impl CodeBlock {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Sets `class="language-…"` on the `<code>` (what highlighters look for) and shows in the header.
        language: text,
        /// A file name or caption in the header.
        title: text,
        /// The copy button (on by default).
        copy: opt<bool>,
        /// Wrap long lines instead of scrolling sideways.
        wrap: val<bool>,
    }
}

impl View for CodeBlock {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let copy = self.copy.unwrap_or(true);
        let mut el =
            root("figure", &["nr-codeblock", if self.wrap { "nr-codeblock-wrap" } else { "" }], self.unstyled, class);
        if copy {
            el.set_attr(attr("data-nr-ui", "code"));
        }
        apply(&mut el, attrs);
        let has_head = self.title.is_some() || self.language.is_some();
        let button = copy.then(|| {
            Element::new("button")
                .with(attr("type", "button"))
                .with(attr("class", "nr-codeblock-copy"))
                .with(attr("data-nr-copy", ""))
                .with(attr("aria-label", "Copy code"))
                .with(part("span", "nr-codeblock-copy-idle").with(icon(icons::copy())))
                .with(part("span", "nr-codeblock-copy-done").with(icon(icons::check())))
                .into_node()
        });
        if has_head {
            let mut head = part("figcaption", "nr-codeblock-head");
            if let Some(title) = self.title {
                head.children.push(part("span", "nr-codeblock-title").with(title).into_node());
            }
            if let Some(language) = &self.language {
                head.children.push(part("span", "nr-codeblock-lang").with(language.clone()).into_node());
            }
            el.children.push(head.into_node());
        }
        let mut code = Element::new("code");
        if let Some(language) = self.language {
            let safe: String =
                language.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '#')).collect();
            code.set_attr(attr("class", format!("language-{safe}")));
        }
        code.children = children;
        let pre = part("pre", "nr-codeblock-pre").with(attr("tabindex", "0")).with(code);
        let mut body = part("div", "nr-codeblock-body").with(pre);
        body.children.extend(button);
        el.children.push(body.into_node());
        styled(el)
    }
}

/// What a [`Callout`] is about; it picks the color, the icon and the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalloutKind {
    #[default]
    Note,
    Tip,
    Important,
    Warning,
    Danger,
}

/// An aside in running text: a note, a tip, a warning.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let c = Callout![kind = CalloutKind::Warning, "Back up first."];
/// let html = render_static(c);
/// assert!(html.contains(r#"<aside class="nr-callout nr-c-warning" role="note">"#) && html.contains("Warning"), "{html}");
/// ```
#[derive(Default)]
pub struct Callout {
    kind: CalloutKind,
    title: Option<String>,
    icon: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl Callout {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        kind: val<CalloutKind>,
        /// Instead of the kind's name.
        title: text,
        /// Instead of the kind's icon.
        icon: node,
    }
}

impl View for Callout {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let (color, name, glyph) = match self.kind {
            CalloutKind::Note => (Color::Primary, "Note", icons::info()),
            CalloutKind::Tip => (Color::Success, "Tip", icons::lightbulb()),
            CalloutKind::Important => (Color::Secondary, "Important", icons::circle_alert()),
            CalloutKind::Warning => (Color::Warning, "Warning", icons::triangle_alert()),
            CalloutKind::Danger => (Color::Danger, "Danger", icons::flame()),
        };
        let mut el = root("aside", &["nr-callout", color.class()], self.unstyled, class);
        el.set_attr(attr("role", "note"));
        apply(&mut el, attrs);
        el.children.push(
            part("p", "nr-callout-title")
                .with(self.icon.unwrap_or_else(|| icon(glyph)))
                .with(self.title.unwrap_or_else(|| name.to_owned()))
                .into_node(),
        );
        el.children.push(part("div", "nr-callout-text").with(children).into_node());
        styled(el)
    }
}

/// An announcement across the top of a page. Its dismiss button works with
/// no script (a checkbox and `:has()`); with `persist` and the component
/// script, a dismissed banner stays away on the next visits.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let b = Banner![persist = "launch-2026", href = "/blog/launch", link_label = "Read more", "Version 2 is out."];
/// let html = render_static(b);
/// assert!(html.contains(r#"data-nr-persist="launch-2026""#) && html.contains(r#"type="checkbox""#), "{html}");
/// ```
#[derive(Default)]
pub struct Banner {
    color: Color,
    soft: bool,
    href: Option<String>,
    link_label: Option<String>,
    icon: Option<Node>,
    dismissable: Option<bool>,
    persist: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Banner {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        color: val<Color>,
        /// A soft tint instead of the solid color.
        soft: val<bool>,
        /// A link after the text.
        href: text,
        /// Text of the link ("Learn more" by default).
        link_label: text,
        icon: node,
        /// A dismiss button (on by default).
        dismissable: opt<bool>,
        /// A key to remember the dismissal under (in `localStorage`); change it for a new announcement.
        persist: text,
    }
}

impl View for Banner {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = ["nr-banner", if self.soft { "nr-banner-soft" } else { "nr-banner-solid" }, self.color.class()];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(attr("role", "region"));
        el.set_attr(attr("aria-label", "Announcement"));
        let dismissable = self.dismissable.unwrap_or(true);
        if let Some(key) = self.persist.filter(|_| dismissable) {
            el.set_attr(attr("data-nr-ui", "banner"));
            el.set_attr(attr("data-nr-persist", key));
        }
        apply(&mut el, attrs);
        let mut text = part("p", "nr-banner-text");
        text.children.extend(self.icon);
        text.children.push(Element::new("span").with(children).into_node());
        if let Some(href) = self.href {
            text.children.push(
                part("a", "nr-banner-link")
                    .with(attr("href", href))
                    .with(attr("data-nr-link", ""))
                    .with(self.link_label.unwrap_or_else(|| "Learn more".to_owned()))
                    .with(icon(icons::arrow_right()))
                    .into_node(),
            );
        }
        el.children.push(text.into_node());
        if dismissable {
            el.children.push(
                part("label", "nr-banner-close")
                    .with(
                        Element::new_void("input")
                            .with(attr("class", "nr-banner-x"))
                            .with(attr("type", "checkbox"))
                            .with(attr("aria-label", "Dismiss announcement")),
                    )
                    .with(icon(icons::close()))
                    .into_node(),
            );
        }
        styled(el)
    }
}

/// A measurement within a known range (disk used, password strength):
/// `role="meter"`. With `low`/`high` the color follows the value.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let m = Meter![label = "Storage", value = 92.0, low = 60.0, high = 85.0, high_is_bad = true, value_label = "92 GB of 100 GB"];
/// let html = render_static(m);
/// assert!(html.contains(r#"role="meter""#) && html.contains("nr-c-danger") && html.contains("width:92%"), "{html}");
/// ```
#[derive(Default)]
pub struct Meter {
    value: f64,
    min: f64,
    max: Option<f64>,
    low: Option<f64>,
    high: Option<f64>,
    high_is_bad: bool,
    segments: Option<u8>,
    label: Option<String>,
    value_label: Option<String>,
    color: Option<Color>,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl Meter {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        value: val<f64>,
        min: val<f64>,
        /// Defaults to 100.
        max: opt<f64>,
        /// Below this the value counts as low.
        low: opt<f64>,
        /// Above this the value counts as high.
        high: opt<f64>,
        /// High values are the bad ones (disk usage); low ones otherwise (battery).
        high_is_bad: val<bool>,
        /// Draw the bar as this many blocks (2 to 12).
        segments: opt<u8>,
        /// Text above the bar (also the accessible name).
        label: text,
        /// Text for the value ("92 GB of 100 GB"); the number when not given.
        value_label: text,
        /// A fixed color instead of the one `low`/`high` give.
        color: opt<Color>,
        size: val<Size>,
    }
}

impl View for Meter {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let max = self.max.unwrap_or(100.0);
        let value = self.value.clamp(self.min, max.max(self.min));
        let ratio = if max > self.min { (value - self.min) / (max - self.min) } else { 0.0 };
        let color = self.color.unwrap_or_else(|| {
            let zone = match (self.low, self.high) {
                (Some(low), _) if value < low => 0,
                (_, Some(high)) if value > high => 2,
                (None, None) => return Color::Primary,
                _ => 1,
            };
            match (zone, self.high_is_bad) {
                (1, _) => {
                    if self.low.is_some() && self.high.is_some() {
                        Color::Warning
                    } else {
                        Color::Success
                    }
                }
                (0, false) | (2, true) => Color::Danger,
                _ => Color::Success,
            }
        });
        let size = format!("nr-meter-{}", self.size.suffix());
        let mut el = root("div", &["nr-meter", color.class(), &size], self.unstyled, class);
        let text = self.value_label.unwrap_or_else(|| trim_float(value));
        if self.label.is_some() {
            let mut head = part("div", "nr-meter-head");
            head.children.push(part("span", "nr-meter-label").with(self.label.clone()).into_node());
            head.children.push(part("span", "nr-meter-value").with(text.clone()).into_node());
            el.children.push(head.into_node());
        }
        let mut track = Element::new("div")
            .with(class_list(&["nr-meter-track", if self.segments.is_some() { "nr-meter-segmented" } else { "" }]))
            .with(attr("role", "meter"))
            .with(attr("aria-valuemin", trim_float(self.min)))
            .with(attr("aria-valuemax", trim_float(max)))
            .with(attr("aria-valuenow", trim_float(value)))
            .with(attr("aria-valuetext", text));
        if let Some(label) = self.label {
            track.set_attr(attr("aria-label", label));
        }
        if let Some(n) = self.segments {
            track.set_attr(attr("style", format!("--nr-meter-n:{}", n.clamp(2, 12))));
        }
        apply(&mut track, attrs);
        track.children.push(
            part("div", "nr-meter-bar")
                .with(attr("style", format!("width:{}%", trim_float((ratio * 1000.0).round() / 10.0))))
                .into_node(),
        );
        el.children.push(track.into_node());
        styled(el)
    }
}

/// An image with a caption.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let f = Figure![src = "/team.jpg", alt = "The team", caption = "Berlin, 2026", ratio = "3 / 2"];
/// let html = render_static(f);
/// assert!(html.contains(r#"loading="lazy""#) && html.contains("<figcaption") && html.contains("aspect-ratio:3 / 2"), "{html}");
/// ```
#[derive(Default)]
pub struct Figure {
    src: Option<String>,
    alt: Option<String>,
    caption: Option<String>,
    ratio: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    eager: bool,
    radius: Option<Radius>,
    bordered: bool,
    unstyled: bool,
    extra: Extra,
}

impl Figure {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        src: text,
        /// Describe the image; empty for decoration.
        alt: text,
        caption: text,
        /// Crop to a ratio (`"16 / 9"`); the image covers the box.
        ratio: text,
        /// Intrinsic width, to reserve space before the image loads.
        width: opt<u32>,
        height: opt<u32>,
        /// Load right away (for images at the top of a page); lazy otherwise.
        eager: val<bool>,
        radius: opt<Radius>,
        bordered: val<bool>,
    }
}

impl View for Figure {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = [
            "nr-figure",
            self.radius.map(Radius::class).unwrap_or(""),
            if self.bordered { "nr-figure-bordered" } else { "" },
        ];
        let mut el = root("figure", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let mut media = part("div", "nr-figure-media");
        if let Some(ratio) = self.ratio {
            let r: String = ratio.chars().filter(|c| c.is_ascii_digit() || matches!(c, ' ' | '.' | '/')).collect();
            media.set_attr(attr("style", format!("aspect-ratio:{}", r.trim())));
        }
        if let Some(src) = self.src {
            let mut img = Element::new_void("img")
                .with(attr("class", "nr-figure-img"))
                .with(attr("src", src))
                .with(attr("alt", self.alt.unwrap_or_default()))
                .with(attr("loading", if self.eager { "eager" } else { "lazy" }))
                .with(attr("decoding", "async"));
            if let Some(w) = self.width {
                img.set_attr(attr("width", w.to_string()));
            }
            if let Some(h) = self.height {
                img.set_attr(attr("height", h.to_string()));
            }
            media.children.push(img.into_node());
        }
        // Children: your own media (a `picture!`, a `video!`, an `Image!`).
        media.children.extend(children);
        el.children.push(media.into_node());
        if let Some(caption) = self.caption {
            el.children.push(part("figcaption", "nr-figure-caption").with(caption).into_node());
        }
        styled(el)
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A responsive video embed (YouTube, Vimeo, ...). The iframe loads lazily;
/// with a `poster` (always with `youtube`) only the poster image loads until
/// the visitor presses play: no third-party script, no cookies before that.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let v = VideoEmbed![youtube = "dQw4w9WgXcQ", title = "Launch video"];
/// let html = render_static(v);
/// assert!(html.contains(r#"loading="lazy""#) && html.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"), "{html}");
/// assert!(html.contains("srcdoc="), "the poster facade");
/// ```
#[derive(Default)]
pub struct VideoEmbed {
    src: Option<String>,
    poster: Option<String>,
    title: Option<String>,
    ratio: Option<String>,
    radius: Option<Radius>,
    unstyled: bool,
    extra: Extra,
}

impl VideoEmbed {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// The embed URL (`https://player.vimeo.com/video/…`).
        src: text,
        /// An image shown until play is pressed.
        poster: text,
        /// Accessible name of the frame (required for a good experience).
        title: text,
        /// `"16 / 9"` by default.
        ratio: text,
        radius: opt<Radius>,
    }
    /// A YouTube video by id: the privacy-friendly embed URL and its poster.
    pub fn youtube(mut self, id: impl Into<String>) -> Self {
        let id: String = id.into().chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).collect();
        self.src = Some(format!("https://www.youtube-nocookie.com/embed/{id}"));
        self.poster.get_or_insert(format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"));
        self
    }
}

impl View for VideoEmbed {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("div", &["nr-video", self.radius.map(Radius::class).unwrap_or("")], self.unstyled, class);
        let ratio: String = self
            .ratio
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_ascii_digit() || matches!(c, ' ' | '.' | '/'))
            .collect();
        el.set_attr(attr(
            "style",
            format!("aspect-ratio:{}", if ratio.trim().is_empty() { "16 / 9" } else { ratio.trim() }),
        ));
        let src = self.src.unwrap_or_default();
        let title = self.title.unwrap_or_else(|| "Video".to_owned());
        let mut frame = Element::new("iframe")
            .with(attr("class", "nr-video-frame"))
            .with(attr("src", src.clone()))
            .with(attr("title", title.clone()))
            .with(attr("loading", "lazy"))
            .with(attr(
                "allow",
                "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share",
            ))
            .with(attr("referrerpolicy", "strict-origin-when-cross-origin"))
            .with(flag("allowfullscreen", true));
        if let Some(poster) = self.poster {
            let play = format!("{src}{}autoplay=1", if src.contains('?') { '&' } else { '?' });
            frame.set_attr(attr(
                "srcdoc",
                format!(
                    "<style>*{{margin:0;padding:0;overflow:hidden}}html,body,a{{display:block;height:100%}}img{{width:100%;height:100%;object-fit:cover}}span{{position:absolute;inset:0;margin:auto;width:68px;height:48px;border-radius:14px;background:rgba(0,0,0,.72);color:#fff;font:26px/48px sans-serif;text-align:center}}a:hover span,a:focus span{{background:#f00}}</style><a href=\"{}\" aria-label=\"Play: {}\"><img src=\"{}\" alt=\"\"><span>&#9654;</span></a>",
                    escape_html(&play),
                    escape_html(&title),
                    escape_html(&poster),
                ),
            ));
        }
        apply(&mut frame, attrs);
        el.children.push(frame.into_node());
        styled(el)
    }
}

/// The outcome a [`ResultBlock`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResultStatus {
    #[default]
    Success,
    Error,
    Warning,
    Info,
}

/// The outcome of something, as a page or a panel: "Payment received",
/// "Page not found". Children are the actions.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let r = ResultBlock![status = ResultStatus::Error, title = "Payment failed", description = "The card was declined.",
///     Button![href = "/billing", "Try again"]];
/// let html = render_static(r);
/// assert!(html.contains(r#"class="nr-result nr-c-danger""#) && html.contains(r#"<h2 class="nr-result-title">"#), "{html}");
/// ```
#[derive(Default)]
pub struct ResultBlock {
    status: ResultStatus,
    code: Option<String>,
    title: Option<String>,
    description: Option<String>,
    icon: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl ResultBlock {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        status: val<ResultStatus>,
        /// Big text instead of the icon ("404").
        code: text,
        title: text,
        description: text,
        /// Your own icon.
        icon: node,
    }
}

impl View for ResultBlock {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let (color, glyph) = match self.status {
            ResultStatus::Success => (Color::Success, icons::circle_check()),
            ResultStatus::Error => (Color::Danger, icons::circle_x()),
            ResultStatus::Warning => (Color::Warning, icons::triangle_alert()),
            ResultStatus::Info => (Color::Primary, icons::info()),
        };
        let mut el = root("div", &["nr-result", color.class()], self.unstyled, class);
        apply(&mut el, attrs);
        match self.code {
            Some(code) => el.children.push(part("p", "nr-result-code").with(code).into_node()),
            None => el.children.push(
                part("span", "nr-result-icon")
                    .with(attr("aria-hidden", "true"))
                    .with(self.icon.unwrap_or_else(|| icon(glyph)))
                    .into_node(),
            ),
        }
        if let Some(title) = self.title {
            el.children.push(part("h2", "nr-result-title").with(title).into_node());
        }
        if let Some(description) = self.description {
            el.children.push(part("p", "nr-result-description").with(description).into_node());
        }
        if !children.is_empty() {
            el.children.push(part("div", "nr-result-actions").with(children).into_node());
        }
        styled(el)
    }
}
