//! Tabs, breadcrumbs, steps and accordions.

use next_rust_view::{Attr, Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::{Color, Extra, Radius, Size, apply, attr, flag, next_id, root, styled};

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/// Look of a [`Tabs`] strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsVariant {
    /// A segmented control: a soft track with the selected tab raised.
    #[default]
    Solid,
    /// Text with a line under the selected tab.
    Underlined,
    /// An outlined track.
    Bordered,
    /// Text only, the selected tab tinted.
    Light,
}

impl TabsVariant {
    fn class(self) -> &'static str {
        match self {
            TabsVariant::Solid => "nr-tabs-solid",
            TabsVariant::Underlined => "nr-tabs-underlined",
            TabsVariant::Bordered => "nr-tabs-bordered",
            TabsVariant::Light => "nr-tabs-light",
        }
    }
}

/// One tab. With `href` it is a link (the page decides which is selected,
/// by `selected` or by matching the current path with the client runtime);
/// with children it is a panel the component script switches to.
///
/// `Tab![key = "grades", title = "Grades", p!["…"]]`
#[derive(Default)]
pub struct Tab {
    key: Option<String>,
    title: Option<String>,
    href: Option<String>,
    selected: bool,
    disabled: bool,
    start_content: Option<Node>,
    end_content: Option<Node>,
    extra: Extra,
}

impl Tab {
    pub fn new() -> Self {
        Self::default()
    }
    with_parts!();

    /// Identifies the tab (`Tabs![selected = key]`); defaults to its index.
    pub fn key(mut self, v: impl Into<String>) -> Self {
        self.key = Some(v.into());
        self
    }
    /// The text of the tab.
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    /// Make the tab a link.
    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    pub fn selected(mut self, v: bool) -> Self {
        self.selected = v;
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    /// An icon before the title.
    pub fn start_content(mut self, v: impl View) -> Self {
        self.start_content = Some(v.into_node());
        self
    }
    /// A chip or count after the title.
    pub fn end_content(mut self, v: impl View) -> Self {
        self.end_content = Some(v.into_node());
        self
    }
}

/// Values accepted inside `Tabs![..]`.
pub trait TabsPart {
    fn add_to(self, tabs: &mut Tabs);
}
impl TabsPart for Tab {
    fn add_to(self, tabs: &mut Tabs) {
        tabs.tabs.push(self);
    }
}
impl TabsPart for Vec<Tab> {
    fn add_to(self, tabs: &mut Tabs) {
        tabs.tabs.extend(self);
    }
}
impl TabsPart for Attr {
    fn add_to(self, tabs: &mut Tabs) {
        tabs.extra.push(self);
    }
}

/// A strip of tabs: links between pages, or panels switched in place.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let t = Tabs![selected = "b", Tab![key = "a", title = "A", p!["one"]], Tab![key = "b", title = "B", p!["two"]]];
/// let html = render_static(t);
/// assert!(html.contains(r#"role="tablist""#) && html.contains(r#"aria-selected="true""#));
/// assert!(html.contains(r#"tabindex="0" hidden>"#), "the unselected panel is hidden");
/// ```
#[derive(Default)]
pub struct Tabs {
    tabs: Vec<Tab>,
    variant: TabsVariant,
    color: Color,
    size: Size,
    radius: Option<Radius>,
    selected: Option<String>,
    full_width: bool,
    label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Tabs {
    pub fn new() -> Self {
        Self { color: Color::Default, ..Self::default() }
    }
    common_methods!();

    /// Add tabs or attributes for the strip.
    pub fn with(mut self, part: impl TabsPart) -> Self {
        part.add_to(&mut self);
        self
    }
    pub fn tabs(mut self, v: impl IntoIterator<Item = Tab>) -> Self {
        self.tabs.extend(v);
        self
    }
    pub fn variant(mut self, v: TabsVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
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
    /// The key of the selected tab (otherwise the first, or the one marked).
    pub fn selected(mut self, v: impl Into<String>) -> Self {
        self.selected = Some(v.into());
        self
    }
    /// Tabs share the width of the strip.
    pub fn full_width(mut self, v: bool) -> Self {
        self.full_width = v;
        self
    }
    /// Accessible name of the strip.
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
}

impl View for Tabs {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let id = next_id("nr-tabs");
        let keys: Vec<String> =
            self.tabs.iter().enumerate().map(|(i, t)| t.key.clone().unwrap_or_else(|| i.to_string())).collect();
        let selected = self
            .selected
            .as_ref()
            .and_then(|s| keys.iter().position(|k| k == s))
            .or_else(|| self.tabs.iter().position(|t| t.selected))
            .or_else(|| self.tabs.iter().position(|t| !t.disabled));
        let has_panels = self.tabs.iter().any(|t| t.extra.has_children());

        let size = format!("nr-tabs-{}", self.size.suffix());
        let classes = [
            "nr-tabs",
            self.variant.class(),
            self.color.class(),
            &size,
            self.radius.map(Radius::class).unwrap_or(""),
            if self.full_width { "nr-tabs-full" } else { "" },
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        if has_panels {
            el.set_attr(attr("data-nr-ui", "tabs"));
        }
        apply(&mut el, attrs);

        let mut list = Element::new("div").with(attr("class", "nr-tabs-list")).with(attr("role", "tablist"));
        if let Some(label) = self.label {
            list.set_attr(attr("aria-label", label));
        }
        let mut panels = Vec::new();
        for (i, tab) in self.tabs.into_iter().enumerate() {
            let key = &keys[i];
            let is_selected = selected == Some(i);
            let (tab_class, tab_attrs, content) = tab.extra.split();
            let tag = if tab.href.is_some() { "a" } else { "button" };
            let mut item = Element::new(tag);
            item.set_attr(attr("class", if is_selected { "nr-tab nr-tab-selected" } else { "nr-tab" }));
            if let Some(c) = tab_class {
                item.set_attr(c);
            }
            item.set_attr(attr("data-key", key.clone()));
            match &tab.href {
                Some(href) => {
                    item.set_attr(attr("href", href.clone()));
                    item.set_attr(attr("data-nr-link", ""));
                    if is_selected {
                        item.set_attr(attr("aria-current", "page"));
                    }
                    if tab.disabled {
                        item.set_attr(attr("aria-disabled", "true"));
                        item.set_attr(attr("tabindex", "-1"));
                    }
                }
                None => {
                    item.set_attr(attr("type", "button"));
                    item.set_attr(attr("role", "tab"));
                    item.set_attr(attr("id", format!("{id}-tab-{key}")));
                    item.set_attr(attr("aria-selected", if is_selected { "true" } else { "false" }));
                    item.set_attr(attr("tabindex", if is_selected { "0" } else { "-1" }));
                    item.set_attr(flag("disabled", tab.disabled));
                    if has_panels {
                        item.set_attr(attr("aria-controls", format!("{id}-panel-{key}")));
                    }
                }
            }
            apply(&mut item, tab_attrs);
            item.children.extend(tab.start_content);
            if let Some(title) = tab.title {
                item.children.push(Element::new("span").with(attr("class", "nr-tab-title")).with(title).into_node());
            }
            item.children.extend(tab.end_content);
            list.children.push(item.into_node());

            if has_panels {
                let mut panel = Element::new("div")
                    .with(attr("class", "nr-tab-panel"))
                    .with(attr("role", "tabpanel"))
                    .with(attr("id", format!("{id}-panel-{key}")))
                    .with(attr("aria-labelledby", format!("{id}-tab-{key}")))
                    .with(attr("tabindex", "0"));
                panel.set_attr(flag("hidden", !is_selected));
                panel.children = content;
                panels.push(panel.into_node());
            }
        }
        el.children.push(list.into_node());
        el.children.extend(panels);
        el.children.extend(children);
        styled(el)
    }
}

// ---------------------------------------------------------------------------
// Breadcrumbs
// ---------------------------------------------------------------------------

/// One crumb; the last one is the current page.
///
/// `BreadcrumbItem![href = "/courses", "Courses"]`
#[derive(Default)]
pub struct BreadcrumbItem {
    href: Option<String>,
    start_content: Option<Node>,
    disabled: bool,
    extra: Extra,
}

impl BreadcrumbItem {
    pub fn new() -> Self {
        Self::default()
    }
    with_parts!();

    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    pub fn start_content(mut self, v: impl View) -> Self {
        self.start_content = Some(v.into_node());
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
}

/// Values accepted inside `Breadcrumbs![..]`.
pub trait BreadcrumbsPart {
    fn add_to(self, crumbs: &mut Breadcrumbs);
}
impl BreadcrumbsPart for BreadcrumbItem {
    fn add_to(self, crumbs: &mut Breadcrumbs) {
        crumbs.items.push(self);
    }
}
impl BreadcrumbsPart for Vec<BreadcrumbItem> {
    fn add_to(self, crumbs: &mut Breadcrumbs) {
        crumbs.items.extend(self);
    }
}
impl BreadcrumbsPart for Attr {
    fn add_to(self, crumbs: &mut Breadcrumbs) {
        crumbs.extra.push(self);
    }
}

/// The path to the current page.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let b = Breadcrumbs![BreadcrumbItem![href = "/", "Home"], BreadcrumbItem!["Settings"]];
/// let html = render_static(b);
/// assert!(html.contains(r#"<nav class="nr-breadcrumbs nr-breadcrumbs-md" aria-label="Breadcrumb">"#));
/// assert!(html.contains(r#"aria-current="page""#));
/// ```
#[derive(Default)]
pub struct Breadcrumbs {
    items: Vec<BreadcrumbItem>,
    size: Size,
    separator: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl Breadcrumbs {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();

    /// Add crumbs or attributes for the `<nav>`.
    pub fn with(mut self, part: impl BreadcrumbsPart) -> Self {
        part.add_to(&mut self);
        self
    }
    pub fn items(mut self, v: impl IntoIterator<Item = BreadcrumbItem>) -> Self {
        self.items.extend(v);
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
    /// Text between crumbs instead of the chevron ("/").
    pub fn separator(mut self, v: impl Into<String>) -> Self {
        self.separator = Some(v.into());
        self
    }
}

impl View for Breadcrumbs {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let size = format!("nr-breadcrumbs-{}", self.size.suffix());
        let mut el = root("nav", &["nr-breadcrumbs", &size], self.unstyled, class);
        el.set_attr(attr("aria-label", "Breadcrumb"));
        apply(&mut el, attrs);
        let mut list = Element::new("ol").with(attr("class", "nr-breadcrumbs-list"));
        let last = self.items.len().saturating_sub(1);
        for (i, item) in self.items.into_iter().enumerate() {
            let (item_class, item_attrs, content) = item.extra.split();
            let current = i == last;
            let mut li = Element::new("li").with(attr("class", "nr-breadcrumb"));
            if i > 0 {
                let sep =
                    Element::new("span").with(attr("class", "nr-breadcrumb-sep")).with(attr("aria-hidden", "true"));
                li.children.push(
                    match &self.separator {
                        Some(text) => sep.with(text.clone()),
                        None => sep.with(icon(icons::chevron_right())),
                    }
                    .into_node(),
                );
            }
            let mut link = match (&item.href, current || item.disabled) {
                (Some(href), false) => {
                    Element::new("a").with(attr("href", href.clone())).with(attr("data-nr-link", ""))
                }
                _ => Element::new("span"),
            };
            link.set_attr(attr(
                "class",
                if current { "nr-breadcrumb-link nr-breadcrumb-current" } else { "nr-breadcrumb-link" },
            ));
            if let Some(c) = item_class {
                link.set_attr(c);
            }
            if current {
                link.set_attr(attr("aria-current", "page"));
            }
            if item.disabled {
                link.set_attr(attr("aria-disabled", "true"));
            }
            apply(&mut link, item_attrs);
            link.children.extend(item.start_content);
            link.children.extend(content);
            li.children.push(link.into_node());
            list.children.push(li.into_node());
        }
        el.children.push(list.into_node());
        el.children.extend(children);
        styled(el)
    }
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

/// State of a [`Step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Done,
    Current,
    Upcoming,
    /// Something went wrong at this step.
    Error,
}

/// One step of a process. Children are extra content under the description.
///
/// `Step![title = "Payment", description = "Card or bank transfer"]`
#[derive(Default)]
pub struct Step {
    title: Option<String>,
    description: Option<String>,
    href: Option<String>,
    status: Option<StepStatus>,
    icon: Option<Node>,
    extra: Extra,
}

impl Step {
    pub fn new() -> Self {
        Self::default()
    }
    with_parts!();

    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = Some(v.into());
        self
    }
    /// Make the step a link (done steps usually are).
    pub fn href(mut self, v: impl Into<String>) -> Self {
        self.href = Some(v.into());
        self
    }
    /// Override the status `Steps![current = ..]` implies.
    pub fn status(mut self, v: StepStatus) -> Self {
        self.status = Some(v);
        self
    }
    /// Your own marker instead of the number or check mark.
    pub fn icon(mut self, v: impl View) -> Self {
        self.icon = Some(v.into_node());
        self
    }
}

/// Values accepted inside `Steps![..]`.
pub trait StepsPart {
    fn add_to(self, steps: &mut Steps);
}
impl StepsPart for Step {
    fn add_to(self, steps: &mut Steps) {
        steps.steps.push(self);
    }
}
impl StepsPart for Vec<Step> {
    fn add_to(self, steps: &mut Steps) {
        steps.steps.extend(self);
    }
}
impl StepsPart for Attr {
    fn add_to(self, steps: &mut Steps) {
        steps.extra.push(self);
    }
}

/// Where someone is in a process.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Steps![current = 1, Step![title = "Account"], Step![title = "Plan"], Step![title = "Done"]];
/// let html = render_static(s);
/// assert!(html.contains("nr-step-done") && html.contains(r#"aria-current="step""#) && html.contains("nr-step-upcoming"));
/// ```
#[derive(Default)]
pub struct Steps {
    steps: Vec<Step>,
    current: usize,
    color: Color,
    size: Size,
    vertical: bool,
    unstyled: bool,
    extra: Extra,
}

impl Steps {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();

    /// Add steps or attributes for the list.
    pub fn with(mut self, part: impl StepsPart) -> Self {
        part.add_to(&mut self);
        self
    }
    pub fn steps(mut self, v: impl IntoIterator<Item = Step>) -> Self {
        self.steps.extend(v);
        self
    }
    /// Index of the current step (0-based); earlier ones are done.
    pub fn current(mut self, v: usize) -> Self {
        self.current = v;
        self
    }
    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
    pub fn vertical(mut self, v: bool) -> Self {
        self.vertical = v;
        self
    }
}

impl View for Steps {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let size = format!("nr-steps-{}", self.size.suffix());
        let classes = ["nr-steps", &size, self.color.class(), if self.vertical { "nr-steps-vertical" } else { "" }];
        let mut el = root("ol", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        for (i, step) in self.steps.into_iter().enumerate() {
            let status = step.status.unwrap_or(if i < self.current {
                StepStatus::Done
            } else if i == self.current {
                StepStatus::Current
            } else {
                StepStatus::Upcoming
            });
            let status_class = match status {
                StepStatus::Done => "nr-step-done",
                StepStatus::Current => "nr-step-current",
                StepStatus::Upcoming => "nr-step-upcoming",
                StepStatus::Error => "nr-step-error",
            };
            let (step_class, step_attrs, content) = step.extra.split();
            let mut li = Element::new("li").with(attr("class", format!("nr-step {status_class}")));
            if let Some(c) = step_class {
                li.set_attr(c);
            }
            if status == StepStatus::Current {
                li.set_attr(attr("aria-current", "step"));
            }
            apply(&mut li, step_attrs);
            let marker = Element::new("span").with(attr("class", "nr-step-marker")).with(match (step.icon, status) {
                (Some(glyph), _) => glyph,
                (None, StepStatus::Done) => icon(icons::check()),
                (None, StepStatus::Error) => icon(icons::close()),
                (None, _) => Node::Text((i + 1).to_string().into()),
            });
            let mut text = Element::new("span").with(attr("class", "nr-step-text"));
            if let Some(title) = step.title {
                text.children.push(Element::new("span").with(attr("class", "nr-step-title")).with(title).into_node());
            }
            if let Some(description) = step.description {
                text.children.push(
                    Element::new("span").with(attr("class", "nr-step-description")).with(description).into_node(),
                );
            }
            text.children.extend(content);
            let mut inner = match &step.href {
                Some(href) => Element::new("a")
                    .with(attr("class", "nr-step-inner"))
                    .with(attr("href", href.clone()))
                    .with(attr("data-nr-link", "")),
                None => Element::new("span").with(attr("class", "nr-step-inner")),
            };
            inner.children.push(marker.into_node());
            inner.children.push(text.into_node());
            li.children.push(inner.into_node());
            el.children.push(li.into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

// ---------------------------------------------------------------------------
// Accordion
// ---------------------------------------------------------------------------

/// Look of an [`Accordion`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccordionVariant {
    /// Items separated by lines.
    #[default]
    Light,
    /// One outlined box.
    Bordered,
    /// Each item its own card.
    Splitted,
    /// One raised box.
    Shadow,
}

impl AccordionVariant {
    fn class(self) -> &'static str {
        match self {
            AccordionVariant::Light => "nr-accordion-light",
            AccordionVariant::Bordered => "nr-accordion-bordered",
            AccordionVariant::Splitted => "nr-accordion-splitted",
            AccordionVariant::Shadow => "nr-accordion-shadow",
        }
    }
}

/// One section of an [`Accordion`]: a `<details>` element, so it opens
/// and closes without any script.
///
/// `AccordionItem![title = "Shipping", subtitle = "2–4 days", p!["…"]]`
#[derive(Default)]
pub struct AccordionItem {
    title: Option<String>,
    subtitle: Option<String>,
    start_content: Option<Node>,
    expanded: bool,
    disabled: bool,
    extra: Extra,
}

impl AccordionItem {
    pub fn new() -> Self {
        Self::default()
    }
    with_parts!();

    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    pub fn subtitle(mut self, v: impl Into<String>) -> Self {
        self.subtitle = Some(v.into());
        self
    }
    /// An icon before the title.
    pub fn start_content(mut self, v: impl View) -> Self {
        self.start_content = Some(v.into_node());
        self
    }
    /// Open at first.
    pub fn expanded(mut self, v: bool) -> Self {
        self.expanded = v;
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
}

/// Values accepted inside `Accordion![..]`.
pub trait AccordionPart {
    fn add_to(self, accordion: &mut Accordion);
}
impl AccordionPart for AccordionItem {
    fn add_to(self, accordion: &mut Accordion) {
        accordion.items.push(self);
    }
}
impl AccordionPart for Vec<AccordionItem> {
    fn add_to(self, accordion: &mut Accordion) {
        accordion.items.extend(self);
    }
}
impl AccordionPart for Attr {
    fn add_to(self, accordion: &mut Accordion) {
        accordion.extra.push(self);
    }
}

/// Sections that expand one at a time (or several, with `multiple`).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let a = Accordion![AccordionItem![title = "One", expanded = true, p!["1"]], AccordionItem![title = "Two", p!["2"]]];
/// let html = render_static(a);
/// assert!(html.contains(r#"<details class="nr-accordion-item" name="nr-accordion-"#) && html.contains(" open>"));
/// ```
#[derive(Default)]
pub struct Accordion {
    items: Vec<AccordionItem>,
    variant: AccordionVariant,
    multiple: bool,
    compact: bool,
    hide_indicator: bool,
    unstyled: bool,
    extra: Extra,
}

impl Accordion {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();

    /// Add items or attributes for the outer element.
    pub fn with(mut self, part: impl AccordionPart) -> Self {
        part.add_to(&mut self);
        self
    }
    pub fn items(mut self, v: impl IntoIterator<Item = AccordionItem>) -> Self {
        self.items.extend(v);
        self
    }
    pub fn variant(mut self, v: AccordionVariant) -> Self {
        self.variant = v;
        self
    }
    /// Let several items be open at once.
    pub fn multiple(mut self, v: bool) -> Self {
        self.multiple = v;
        self
    }
    pub fn compact(mut self, v: bool) -> Self {
        self.compact = v;
        self
    }
    /// No chevron.
    pub fn hide_indicator(mut self, v: bool) -> Self {
        self.hide_indicator = v;
        self
    }
}

impl View for Accordion {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = ["nr-accordion", self.variant.class(), if self.compact { "nr-accordion-compact" } else { "" }];
        let mut el = root("div", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        let group = next_id("nr-accordion");
        for item in self.items {
            let (item_class, item_attrs, content) = item.extra.split();
            let mut details = Element::new("details").with(attr("class", "nr-accordion-item"));
            if let Some(c) = item_class {
                details.set_attr(c);
            }
            if !self.multiple {
                details.set_attr(attr("name", group.clone()));
            }
            details.set_attr(flag("open", item.expanded));
            details.set_attr(flag("data-disabled", item.disabled));
            apply(&mut details, item_attrs);
            let mut summary = Element::new("summary").with(attr("class", "nr-accordion-trigger"));
            if item.disabled {
                summary.set_attr(attr("tabindex", "-1"));
                summary.set_attr(attr("aria-disabled", "true"));
            }
            summary.children.extend(item.start_content);
            let mut heading = Element::new("span").with(attr("class", "nr-accordion-heading"));
            if let Some(title) = item.title {
                heading
                    .children
                    .push(Element::new("span").with(attr("class", "nr-accordion-title")).with(title).into_node());
            }
            if let Some(subtitle) = item.subtitle {
                heading
                    .children
                    .push(Element::new("span").with(attr("class", "nr-accordion-subtitle")).with(subtitle).into_node());
            }
            summary.children.push(heading.into_node());
            if !self.hide_indicator {
                summary.children.push(
                    Element::new("span")
                        .with(attr("class", "nr-accordion-indicator"))
                        .with(icon(icons::chevron_down()))
                        .into_node(),
                );
            }
            details.children.push(summary.into_node());
            details
                .children
                .push(Element::new("div").with(attr("class", "nr-accordion-content")).with(content).into_node());
            el.children.push(details.into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}
