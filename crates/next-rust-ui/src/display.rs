//! Alerts, badges, progress, skeletons, stats, keyboard keys, empty states
//! and tooltips: things that show, and need no script.

use next_rust_view::{Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::{Color, Extra, Radius, Size, Variant, apply, attr, flag, next_id, root, styled};

/// A message that stands out: information, a success, a warning, an error.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let a = Alert![color = Color::Success, title = "Saved", "Your changes are live."];
/// let html = render_static(a);
/// assert!(html.contains(r#"<div class="nr-alert nr-alert-flat nr-c-success" role="status">"#));
/// ```
#[derive(Default)]
pub struct Alert {
    color: Color,
    variant: Variant,
    radius: Option<Radius>,
    title: Option<String>,
    icon: Option<Node>,
    hide_icon: bool,
    closable: bool,
    end_content: Option<Node>,
    unstyled: bool,
    extra: Extra,
}

impl Alert {
    pub fn new() -> Self {
        Self { variant: Variant::Flat, ..Self::default() }
    }
    common_methods!();
    with_parts!();

    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
        self
    }
    /// `Solid`, `Flat` (default), `Bordered` or `Faded`; others render as `Flat`.
    pub fn variant(mut self, v: Variant) -> Self {
        self.variant = v;
        self
    }
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
    /// Bold first line.
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    /// Your own icon instead of the one the color implies.
    pub fn icon(mut self, v: impl View) -> Self {
        self.icon = Some(v.into_node());
        self
    }
    pub fn hide_icon(mut self, v: bool) -> Self {
        self.hide_icon = v;
        self
    }
    /// A close button; the component script removes the alert on click.
    pub fn closable(mut self, v: bool) -> Self {
        self.closable = v;
        self
    }
    /// Something at the end: a button, a link.
    pub fn end_content(mut self, v: impl View) -> Self {
        self.end_content = Some(v.into_node());
        self
    }
}

impl View for Alert {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let variant = match self.variant {
            Variant::Solid => "nr-alert-solid",
            Variant::Bordered => "nr-alert-bordered",
            Variant::Faded => "nr-alert-faded",
            _ => "nr-alert-flat",
        };
        let classes = ["nr-alert", variant, self.color.class(), self.radius.map(Radius::class).unwrap_or("")];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(attr(
            "role",
            if matches!(self.color, Color::Danger | Color::Warning) { "alert" } else { "status" },
        ));
        if self.closable {
            el.set_attr(attr("data-nr-ui", "alert"));
        }
        apply(&mut el, attrs);
        if !self.hide_icon {
            let glyph = self.icon.unwrap_or_else(|| {
                icon(match self.color {
                    Color::Success => icons::circle_check(),
                    Color::Warning => icons::triangle_alert(),
                    Color::Danger => icons::circle_x(),
                    _ => icons::info(),
                })
            });
            el.children.push(Element::new("span").with(attr("class", "nr-alert-icon")).with(glyph).into_node());
        }
        let mut body = Element::new("div").with(attr("class", "nr-alert-body"));
        if let Some(title) = self.title {
            body.children.push(Element::new("strong").with(attr("class", "nr-alert-title")).with(title).into_node());
        }
        if !children.is_empty() {
            body.children.push(Element::new("div").with(attr("class", "nr-alert-text")).with(children).into_node());
        }
        el.children.push(body.into_node());
        if let Some(end) = self.end_content {
            el.children.push(Element::new("div").with(attr("class", "nr-alert-end")).with(end).into_node());
        }
        if self.closable {
            el.children.push(
                Element::new("button")
                    .with(attr("type", "button"))
                    .with(attr("class", "nr-alert-close"))
                    .with(attr("aria-label", "Dismiss"))
                    .with(attr("data-nr-dismiss", ""))
                    .with(icon(icons::close()))
                    .into_node(),
            );
        }
        styled(el)
    }
}

/// Where a [`Badge`] sits on its child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

impl Placement {
    fn class(self) -> &'static str {
        match self {
            Placement::TopRight => "nr-badge-tr",
            Placement::TopLeft => "nr-badge-tl",
            Placement::BottomRight => "nr-badge-br",
            Placement::BottomLeft => "nr-badge-bl",
        }
    }
}

/// A count or a dot pinned to a corner of its child (an icon, an avatar).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let b = Badge![content = "3", color = Color::Danger, Avatar![name = "Ada"]];
/// assert!(render_static(b).contains(r#"<span class="nr-badge-content nr-badge-tr nr-c-danger nr-badge-md">3</span>"#));
/// ```
#[derive(Default)]
pub struct Badge {
    content: Option<String>,
    color: Color,
    variant: Variant,
    size: Size,
    placement: Placement,
    dot: bool,
    hidden: bool,
    unstyled: bool,
    extra: Extra,
}

impl Badge {
    pub fn new() -> Self {
        Self { color: Color::Danger, ..Self::default() }
    }
    common_methods!();
    with_parts!();

    /// The text of the badge (a number, "new").
    pub fn content(mut self, v: impl Into<String>) -> Self {
        self.content = Some(v.into());
        self
    }
    pub fn color(mut self, v: Color) -> Self {
        self.color = v;
        self
    }
    /// `Solid` (default), `Flat` or `Faded`.
    pub fn variant(mut self, v: Variant) -> Self {
        self.variant = v;
        self
    }
    pub fn size(mut self, v: Size) -> Self {
        self.size = v;
        self
    }
    pub fn placement(mut self, v: Placement) -> Self {
        self.placement = v;
        self
    }
    /// Just a dot, no text.
    pub fn dot(mut self, v: bool) -> Self {
        self.dot = v;
        self
    }
    /// Keep the child, hide the badge (a count of zero).
    pub fn hidden(mut self, v: bool) -> Self {
        self.hidden = v;
        self
    }
}

impl View for Badge {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("span", &["nr-badge"], self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        if !self.hidden {
            let variant = match self.variant {
                Variant::Flat => "nr-badge-flat",
                Variant::Faded => "nr-badge-faded",
                _ => "",
            };
            let size = format!("nr-badge-{}", self.size.suffix());
            let mut badge = Element::new("span").with(attr(
                "class",
                [
                    "nr-badge-content",
                    self.placement.class(),
                    self.color.class(),
                    &size,
                    variant,
                    if self.dot { "nr-badge-dot" } else { "" },
                ]
                .iter()
                .filter(|c| !c.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" "),
            ));
            if !self.dot {
                badge = badge.with(self.content.unwrap_or_default());
            }
            el.children.push(badge.into_node());
        }
        styled(el)
    }
}

/// A horizontal progress bar.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let p = Progress![value = 40.0, label = "Uploading", show_value = true];
/// let html = render_static(p);
/// assert!(html.contains(r#"role="progressbar""#) && html.contains(r#"aria-valuenow="40""#) && html.contains("40%"));
/// ```
#[derive(Default)]
pub struct Progress {
    value: Option<f64>,
    min: f64,
    max: Option<f64>,
    color: Color,
    size: Size,
    radius: Option<Radius>,
    label: Option<String>,
    show_value: bool,
    value_label: Option<String>,
    striped: bool,
    unstyled: bool,
    extra: Extra,
}

impl Progress {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// The current value; none means indeterminate (an animated bar).
    pub fn value(mut self, v: f64) -> Self {
        self.value = Some(v);
        self
    }
    pub fn min(mut self, v: f64) -> Self {
        self.min = v;
        self
    }
    /// Defaults to 100.
    pub fn max(mut self, v: f64) -> Self {
        self.max = Some(v);
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
    /// Text above the bar (also its accessible name).
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    /// Show the percentage at the end of the label line.
    pub fn show_value(mut self, v: bool) -> Self {
        self.show_value = v;
        self
    }
    /// Text instead of the percentage ("3 of 8").
    pub fn value_label(mut self, v: impl Into<String>) -> Self {
        self.value_label = Some(v.into());
        self
    }
    pub fn striped(mut self, v: bool) -> Self {
        self.striped = v;
        self
    }
}

/// `(percent 0..=100, aria-valuenow text)` for a value in `min..=max`.
fn percent(value: f64, min: f64, max: f64) -> (f64, String) {
    let span = (max - min).max(f64::EPSILON);
    let ratio = ((value - min) / span).clamp(0.0, 1.0);
    ((ratio * 1000.0).round() / 10.0, trim_float(value))
}

fn trim_float(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{v}") }
}

impl View for Progress {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let max = self.max.unwrap_or(100.0);
        let size = format!("nr-progress-{}", self.size.suffix());
        let classes = [
            "nr-progress",
            &size,
            self.color.class(),
            self.radius.map(Radius::class).unwrap_or(""),
            if self.striped { "nr-progress-striped" } else { "" },
            if self.value.is_none() { "nr-progress-indeterminate" } else { "" },
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(attr("role", "progressbar"));
        el.set_attr(attr("aria-valuemin", trim_float(self.min)));
        el.set_attr(attr("aria-valuemax", trim_float(max)));
        let mut pct = None;
        if let Some(value) = self.value {
            let (p, now) = percent(value, self.min, max);
            el.set_attr(attr("aria-valuenow", now));
            pct = Some(p);
        }
        let label_id = self.label.as_ref().map(|_| next_id("nr-progress"));
        match (&self.label, &label_id) {
            (Some(_), Some(id)) => el.set_attr(attr("aria-labelledby", id.clone())),
            _ => el.set_attr(attr("aria-label", "Progress")),
        }
        apply(&mut el, attrs);
        if self.label.is_some() || self.show_value || self.value_label.is_some() {
            let mut line = Element::new("div").with(attr("class", "nr-progress-label"));
            if let (Some(label), Some(id)) = (self.label, label_id) {
                line.children.push(Element::new("span").with(attr("id", id)).with(label).into_node());
            }
            let shown = self.value_label.or_else(|| match (self.show_value, pct) {
                (true, Some(p)) => Some(format!("{}%", trim_float(p))),
                _ => None,
            });
            if let Some(text) = shown {
                line.children
                    .push(Element::new("span").with(attr("class", "nr-progress-value")).with(text).into_node());
            }
            el.children.push(line.into_node());
        }
        let mut bar = Element::new("div").with(attr("class", "nr-progress-bar"));
        if let Some(p) = pct {
            bar.set_attr(attr("style", format!("width:{}%", trim_float(p))));
        }
        el.children.push(Element::new("div").with(attr("class", "nr-progress-track")).with(bar).into_node());
        el.children.extend(children);
        styled(el)
    }
}

/// A ring that fills up.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let r = CircularProgress![value = 75.0, show_value = true, color = Color::Success];
/// let html = render_static(r);
/// assert!(html.contains("nr-ring") && html.contains("75%"));
/// ```
#[derive(Default)]
pub struct CircularProgress {
    value: Option<f64>,
    max: Option<f64>,
    color: Color,
    size: Size,
    label: Option<String>,
    show_value: bool,
    value_label: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl CircularProgress {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// The current value; none means indeterminate (a spinning arc).
    pub fn value(mut self, v: f64) -> Self {
        self.value = Some(v);
        self
    }
    /// Defaults to 100.
    pub fn max(mut self, v: f64) -> Self {
        self.max = Some(v);
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
    /// Text under the ring (also its accessible name).
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    /// The percentage inside the ring.
    pub fn show_value(mut self, v: bool) -> Self {
        self.show_value = v;
        self
    }
    /// Text inside the ring instead of the percentage.
    pub fn value_label(mut self, v: impl Into<String>) -> Self {
        self.value_label = Some(v.into());
        self
    }
}

impl View for CircularProgress {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let max = self.max.unwrap_or(100.0);
        let size = format!("nr-ring-{}", self.size.suffix());
        let classes =
            ["nr-ring", &size, self.color.class(), if self.value.is_none() { "nr-ring-indeterminate" } else { "" }];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(attr("role", "progressbar"));
        el.set_attr(attr("aria-valuemin", "0"));
        el.set_attr(attr("aria-valuemax", trim_float(max)));
        let mut pct = None;
        if let Some(value) = self.value {
            let (p, now) = percent(value, 0.0, max);
            el.set_attr(attr("aria-valuenow", now));
            el.set_attr(attr("style", format!("--p:{}", trim_float(p))));
            pct = Some(p);
        }
        match &self.label {
            Some(label) => el.set_attr(attr("aria-label", label.clone())),
            None => el.set_attr(attr("aria-label", "Progress")),
        }
        apply(&mut el, attrs);
        // Two concentric circles; the second is dashed to `--p` percent by CSS.
        let circle = |class: &'static str| {
            Element::new("circle")
                .with(attr("class", class))
                .with(attr("cx", "18"))
                .with(attr("cy", "18"))
                .with(attr("r", "16"))
                .with(attr("fill", "none"))
                .with(attr("stroke-width", "3"))
                .into_node()
        };
        let svg = Element::new("svg")
            .with(attr("class", "nr-ring-svg"))
            .with(attr("viewBox", "0 0 36 36"))
            .with(attr("aria-hidden", "true"))
            .with(circle("nr-ring-track"))
            .with(circle("nr-ring-fill"));
        let mut face = Element::new("div").with(attr("class", "nr-ring-face")).with(svg);
        let shown = self.value_label.or_else(|| match (self.show_value, pct) {
            (true, Some(p)) => Some(format!("{}%", trim_float(p))),
            _ => None,
        });
        if let Some(text) = shown {
            face.children.push(Element::new("span").with(attr("class", "nr-ring-value")).with(text).into_node());
        }
        el.children.push(face.into_node());
        if let Some(label) = self.label {
            el.children.push(Element::new("span").with(attr("class", "nr-ring-label")).with(label).into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

/// A placeholder that shimmers while the real content loads.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Skeleton![lines = 3];
/// assert_eq!(render_static(s).matches(r#"<span class="nr-skeleton-line">"#).count(), 3);
/// ```
#[derive(Default)]
pub struct Skeleton {
    lines: u8,
    radius: Option<Radius>,
    unstyled: bool,
    extra: Extra,
}

impl Skeleton {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// Render as that many text lines (the last one shorter) instead of one box.
    pub fn lines(mut self, v: u8) -> Self {
        self.lines = v;
        self
    }
    pub fn radius(mut self, v: Radius) -> Self {
        self.radius = Some(v);
        self
    }
}

impl View for Skeleton {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let lines = self.lines > 0;
        let classes =
            ["nr-skeleton", if lines { "nr-skeleton-lines" } else { "" }, self.radius.map(Radius::class).unwrap_or("")];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(attr("aria-hidden", "true"));
        apply(&mut el, attrs);
        for _ in 0..self.lines {
            el.children.push(Element::new("span").with(attr("class", "nr-skeleton-line")).into_node());
        }
        el.children.extend(children);
        styled(el)
    }
}

/// Direction of a [`Stat`]'s change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

/// A number with its label: "Students · 1,204 · ↑ 12%".
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Stat![label = "Students", value = "1,204", delta = "12%", trend = Trend::Up];
/// let html = render_static(s);
/// assert!(html.contains(r#"<span class="nr-stat-delta nr-stat-up">"#) && html.contains("12%"));
/// ```
#[derive(Default)]
pub struct Stat {
    label: Option<String>,
    value: Option<String>,
    delta: Option<String>,
    trend: Option<Trend>,
    description: Option<String>,
    icon: Option<Node>,
    color: Option<Color>,
    unstyled: bool,
    extra: Extra,
}

impl Stat {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }
    /// The change ("12%", "+3"), colored by `trend`.
    pub fn delta(mut self, v: impl Into<String>) -> Self {
        self.delta = Some(v.into());
        self
    }
    pub fn trend(mut self, v: Trend) -> Self {
        self.trend = Some(v);
        self
    }
    /// Small text under the value ("since last week").
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = Some(v.into());
        self
    }
    /// An icon in a tinted square at the end.
    pub fn icon(mut self, v: impl View) -> Self {
        self.icon = Some(v.into_node());
        self
    }
    /// Tint of the icon square.
    pub fn color(mut self, v: Color) -> Self {
        self.color = Some(v);
        self
    }
}

impl View for Stat {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("div", &["nr-stat", self.color.map(Color::class).unwrap_or("")], self.unstyled, class);
        apply(&mut el, attrs);
        let mut body = Element::new("div").with(attr("class", "nr-stat-body"));
        if let Some(label) = self.label {
            body.children.push(Element::new("span").with(attr("class", "nr-stat-label")).with(label).into_node());
        }
        let mut line = Element::new("div").with(attr("class", "nr-stat-line"));
        if let Some(value) = self.value {
            line.children.push(Element::new("span").with(attr("class", "nr-stat-value")).with(value).into_node());
        }
        if let Some(delta) = self.delta {
            let (trend_class, glyph) = match self.trend {
                Some(Trend::Up) => ("nr-stat-up", Some(icons::trending_up())),
                Some(Trend::Down) => ("nr-stat-down", Some(icons::trending_down())),
                Some(Trend::Flat) => ("nr-stat-flat", Some(icons::minus())),
                None => ("", None),
            };
            let mut d =
                Element::new("span").with(attr("class", format!("nr-stat-delta {trend_class}").trim_end().to_owned()));
            if let Some(g) = glyph {
                d.children.push(icon(g));
            }
            line.children.push(d.with(delta).into_node());
        }
        body.children.push(line.into_node());
        if let Some(description) = self.description {
            body.children
                .push(Element::new("span").with(attr("class", "nr-stat-description")).with(description).into_node());
        }
        body.children.extend(children);
        el.children.push(body.into_node());
        if let Some(glyph) = self.icon {
            el.children.push(Element::new("span").with(attr("class", "nr-stat-icon")).with(glyph).into_node());
        }
        styled(el)
    }
}

/// A keyboard key: `Kbd!["⌘", "K"]` renders two keys.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// assert!(render_static(Kbd!["Ctrl", "K"]).contains(r#"<kbd class="nr-kbd"><kbd>Ctrl</kbd><kbd>K</kbd></kbd>"#));
/// ```
#[derive(Default)]
pub struct Kbd {
    unstyled: bool,
    extra: Extra,
}

impl Kbd {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
}

impl View for Kbd {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("kbd", &["nr-kbd"], self.unstyled, class);
        apply(&mut el, attrs);
        for child in children {
            el.children.push(Element::new("kbd").with(child).into_node());
        }
        styled(el)
    }
}

/// What to show when there is nothing to show yet.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let e = EmptyState![title = "No classes yet", description = "Create one to get started.", Button!["New class"]];
/// let html = render_static(e);
/// assert!(html.contains(r#"<h3 class="nr-empty-title">No classes yet</h3>"#) && html.contains("nr-empty-actions"));
/// ```
#[derive(Default)]
pub struct EmptyState {
    icon: Option<Node>,
    title: Option<String>,
    description: Option<String>,
    compact: bool,
    unstyled: bool,
    extra: Extra,
}

impl EmptyState {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// Your own icon (an inbox by default).
    pub fn icon(mut self, v: impl View) -> Self {
        self.icon = Some(v.into_node());
        self
    }
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = Some(v.into());
        self
    }
    /// Less padding, for inside a card or a table.
    pub fn compact(mut self, v: bool) -> Self {
        self.compact = v;
        self
    }
}

impl View for EmptyState {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el =
            root("div", &["nr-empty", if self.compact { "nr-empty-compact" } else { "" }], self.unstyled, class);
        apply(&mut el, attrs);
        let glyph = self.icon.unwrap_or_else(|| icon(icons::inbox()));
        el.children.push(Element::new("span").with(attr("class", "nr-empty-icon")).with(glyph).into_node());
        if let Some(title) = self.title {
            el.children.push(Element::new("h3").with(attr("class", "nr-empty-title")).with(title).into_node());
        }
        if let Some(description) = self.description {
            el.children
                .push(Element::new("p").with(attr("class", "nr-empty-description")).with(description).into_node());
        }
        if !children.is_empty() {
            el.children.push(Element::new("div").with(attr("class", "nr-empty-actions")).with(children).into_node());
        }
        styled(el)
    }
}

/// The first element among the nodes, looking inside fragments (a
/// component renders as a fragment of its stylesheet and its element).
fn first_element(nodes: &mut [Node]) -> Option<&mut Element> {
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

/// Side of the anchor a [`Tooltip`] shows on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    fn class(self) -> &'static str {
        match self {
            Side::Top => "nr-tooltip-top",
            Side::Bottom => "nr-tooltip-bottom",
            Side::Left => "nr-tooltip-left",
            Side::Right => "nr-tooltip-right",
        }
    }
}

/// A short text shown on hover or focus. CSS only: the anchor is the
/// child, which should be focusable (a button, a link) so keyboard users
/// see it too.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let t = Tooltip![content = "Copy", Button![icon_only = true, "⧉"]];
/// let html = render_static(t);
/// assert!(html.contains(r#"role="tooltip""#) && html.contains("aria-describedby"));
/// ```
#[derive(Default)]
pub struct Tooltip {
    content: Option<String>,
    side: Side,
    color: Option<Color>,
    unstyled: bool,
    extra: Extra,
}

impl Tooltip {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();

    /// The text of the tooltip.
    pub fn content(mut self, v: impl Into<String>) -> Self {
        self.content = Some(v.into());
        self
    }
    pub fn side(mut self, v: Side) -> Self {
        self.side = v;
        self
    }
    /// A colored bubble instead of the neutral one.
    pub fn color(mut self, v: Color) -> Self {
        self.color = Some(v);
        self
    }
}

impl View for Tooltip {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = ["nr-tooltip", self.side.class(), self.color.map(Color::class).unwrap_or("")];
        let mut el = root("span", &["nr-tooltip-anchor"], self.unstyled, class);
        apply(&mut el, attrs);
        let id = next_id("nr-tip");
        // The first element child is the anchor; it gets the description.
        let mut children = children;
        if let Some(anchor) = first_element(&mut children) {
            anchor.set_attr(attr("aria-describedby", id.clone()));
            anchor.set_attr(flag("data-nr-tooltip-anchor", true));
        }
        el.children = children;
        el.children.push(
            Element::new("span")
                .with(attr("class", classes.iter().filter(|c| !c.is_empty()).copied().collect::<Vec<_>>().join(" ")))
                .with(attr("role", "tooltip"))
                .with(attr("id", id))
                .with(self.content.unwrap_or_default())
                .into_node(),
        );
        styled(el)
    }
}
