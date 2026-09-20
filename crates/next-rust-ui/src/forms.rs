//! More form controls: search, numbers, sliders, one-time codes, ratings,
//! segmented controls, comboboxes, tag inputs, uploads, time, color and
//! date ranges, plus helpers to lay a form out.

use next_rust_view::{Attr, Element, Node, View};

use crate::button::{common_methods, with_parts};
use crate::icons::{self, icon};
use crate::input::{Field, Frame, field_methods, icon_button};
use crate::layout::Justify;
use crate::{
    Color, DatePicker, Extra, FieldVariant, MaybeText, Size, apply, attr, flag, next_id, part, part_class, props, root,
    styled, trim_float,
};

/// An `<input>` for a field: its class, type, placeholder, control
/// attributes, value, and the attributes given in the macro.
fn field_input(field: &mut Field, kind: &'static str) -> (Element, bool) {
    let (class, attrs, _) = std::mem::take(&mut field.extra).split();
    if let Some(class) = class {
        field.extra.push(class);
    }
    let floats = field.placeholder.is_none();
    let mut input = Element::new_void("input")
        .with(part_class("nr-field-input", &field.input_class))
        .with(attr("type", kind))
        .with(attr("placeholder", field.placeholder.clone().unwrap_or_else(|| " ".into())));
    apply(&mut input, field.control_attrs());
    if let Some(value) = &field.value {
        input.set_attr(attr("value", value.clone()));
    }
    input.set_attr(flag("readonly", field.readonly));
    apply(&mut input, attrs);
    (input, floats)
}

/// Add the `with` method of a field: attributes go to the control.
macro_rules! field_with {
    () => {
        /// Add an attribute to the control; `class(..)` goes to the outer element.
        pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
            self.field.extra.push(part);
            self
        }
    };
}

/// The label, description and error slot shared by the controls that have
/// no field box (slider, pin, rating, upload).
#[derive(Default)]
struct Texts {
    label: Option<String>,
    description: Option<String>,
    error_message: Option<String>,
    name: Option<String>,
}

impl Texts {
    fn label(&self, id: &str, required: bool) -> Option<Node> {
        let label = self.label.as_ref()?;
        let mut el = part("label", "nr-field-label").with(attr("for", id.to_owned())).with(label.clone());
        if required {
            el = el.with(part("span", "nr-field-required").with(attr("aria-hidden", "true")).with("*"));
        }
        Some(el.into_node())
    }

    fn describedby(&self, id: &str) -> Option<Attr> {
        let mut ids = Vec::new();
        if self.description.is_some() {
            ids.push(format!("{id}-description"));
        }
        if self.error_message.is_some() || self.name.is_some() {
            ids.push(format!("{id}-error"));
        }
        (!ids.is_empty()).then(|| attr("aria-describedby", ids.join(" ")))
    }

    /// The description and the error slot (`data-nr-error`).
    fn below(&self, id: &str) -> Vec<Node> {
        let mut out = Vec::new();
        if let Some(description) = &self.description {
            out.push(
                part("p", "nr-field-description")
                    .with(attr("id", format!("{id}-description")))
                    .with(description.clone())
                    .into_node(),
            );
        }
        if self.error_message.is_some() || self.name.is_some() {
            let mut err =
                part("p", "nr-field-error").with(attr("id", format!("{id}-error"))).with(attr("aria-live", "polite"));
            if let Some(name) = &self.name {
                err = err.with(attr("data-nr-error", name.clone()));
            }
            out.push(err.with(self.error_message.clone()).into_node());
        }
        out
    }
}

/// The builder methods for [`Texts`] and the control id.
macro_rules! texts_methods {
    () => {
        /// Text of the label.
        pub fn label(mut self, v: impl Into<String>) -> Self {
            self.texts.label = Some(v.into());
            self
        }
        /// Help text under the control.
        pub fn description(mut self, v: impl MaybeText) -> Self {
            self.texts.description = v.into_text();
            self
        }
        /// Error text under the control: `error_message = form.error("code")`.
        pub fn error_message(mut self, v: impl MaybeText) -> Self {
            self.texts.error_message = v.into_text();
            self
        }
        /// Form field name. Server-action validation errors for it appear under the control.
        pub fn name(mut self, v: impl Into<String>) -> Self {
            self.texts.name = Some(v.into());
            self
        }
        /// `id` of the control (generated when not given).
        pub fn id(mut self, v: impl Into<String>) -> Self {
            self.id = Some(v.into());
            self
        }
    };
}

/// A search box: a search icon, a clear button and `name="q"` by default.
/// With `action` it is a whole `GET` form, so it works with no script:
/// `SearchInput![action = "/search", value = query]`.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(SearchInput![action = "/search", placeholder = "Search docs"]);
/// assert!(html.contains(r#"<form class="nr-search-form" role="search" method="get" action="/search">"#), "{html}");
/// assert!(html.contains(r#"type="search""#) && html.contains(r#"name="q""#));
/// ```
#[derive(Default)]
pub struct SearchInput {
    field: Field,
    action: Option<String>,
    shortcut: Option<String>,
}

impl SearchInput {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();
    field_with!();
    props! {
        /// Wrap the field in a `GET` form to this URL.
        action: text,
        /// A keyboard hint at the end ("⌘K"); display only.
        shortcut: text,
    }
}

impl View for SearchInput {
    fn into_node(mut self) -> Node {
        if self.field.name.is_none() {
            self.field.name = Some("q".to_owned());
        }
        if self.field.placeholder.is_none() && self.field.label.is_none() {
            self.field.placeholder = Some("Search…".to_owned());
        }
        if self.field.start_content.is_none() {
            self.field.start_content = Some(icon(icons::search()));
        }
        let (mut input, floats) = field_input(&mut self.field, "search");
        if self.field.label.is_none() && !input.attrs.iter().any(|a| a.name == "aria-label") {
            input.set_attr(attr("aria-label", "Search"));
        }
        let mut end = vec![icon_button("nr-field-clear", "Clear", "clear", icon(icons::close()))];
        if let Some(shortcut) = self.shortcut {
            end.push(part("kbd", "nr-search-shortcut").with(shortcut).into_node());
        }
        let node = self.field.render(
            input.into_node(),
            Frame {
                kind: "nr-input nr-search",
                floats,
                after_box: Vec::new(),
                root_attrs: vec![attr("data-nr-ui", "field")],
                end,
            },
        );
        match self.action {
            Some(action) => Element::new("form")
                .with(attr("class", "nr-search-form"))
                .with(attr("role", "search"))
                .with(attr("method", "get"))
                .with(attr("action", action))
                .with(node)
                .into_node(),
            None => node,
        }
    }
}

/// A number field with step buttons (the script's; the browser's own
/// spinner without it).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(NumberInput![label = "Seats", name = "seats", value = "3", min = 1.0, max = 10.0]);
/// assert!(html.contains(r#"type="number""#) && html.contains(r#"min="1""#) && html.contains(r#"max="10""#));
/// assert!(html.contains(r#"data-nr-field-action="step-up""#));
/// ```
#[derive(Default)]
pub struct NumberInput {
    field: Field,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
}

impl NumberInput {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();
    field_with!();
    props! {
        min: opt<f64>,
        max: opt<f64>,
        /// The amount one step changes the value by (1 by default).
        step: opt<f64>,
    }
}

impl View for NumberInput {
    fn into_node(mut self) -> Node {
        let (mut input, floats) = field_input(&mut self.field, "number");
        input.set_attr(attr(
            "inputmode",
            if self.step.is_some_and(|s| s.fract() != 0.0) { "decimal" } else { "numeric" },
        ));
        for (name, v) in [("min", self.min), ("max", self.max), ("step", self.step)] {
            if let Some(v) = v {
                input.set_attr(attr(name, trim_float(v)));
            }
        }
        let steps = part("span", "nr-number-steps")
            .with(icon_button("nr-number-step", "Increase", "step-up", icon(icons::chevron_up())))
            .with(icon_button("nr-number-step", "Decrease", "step-down", icon(icons::chevron_down())))
            .into_node();
        self.field.render(
            input.into_node(),
            Frame {
                kind: "nr-input nr-number",
                floats,
                after_box: Vec::new(),
                root_attrs: vec![attr("data-nr-ui", "field")],
                end: vec![steps],
            },
        )
    }
}

/// A time field: the browser's time input in the kit's field box.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(TimePicker![label = "Starts at", name = "start", value = "09:30", step = 900]);
/// assert!(html.contains(r#"type="time""#) && html.contains(r#"value="09:30""#) && html.contains(r#"step="900""#));
/// ```
#[derive(Default)]
pub struct TimePicker {
    field: Field,
    min: Option<String>,
    max: Option<String>,
    step: Option<u32>,
}

impl TimePicker {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();
    field_with!();
    props! {
        /// Earliest time (`HH:MM`).
        min: text,
        /// Latest time (`HH:MM`).
        max: text,
        /// Granularity in seconds (`900` = quarter hours).
        step: opt<u32>,
    }
}

impl View for TimePicker {
    fn into_node(mut self) -> Node {
        let (mut input, _) = field_input(&mut self.field, "time");
        for (name, v) in [("min", self.min), ("max", self.max), ("step", self.step.map(|s| s.to_string()))] {
            if let Some(v) = v {
                input.set_attr(attr(name, v));
            }
        }
        // A time input always shows its own format, so the label never floats over it.
        self.field.render(
            input.into_node(),
            Frame {
                kind: "nr-input nr-time",
                floats: false,
                after_box: Vec::new(),
                root_attrs: Vec::new(),
                end: Vec::new(),
            },
        )
    }
}

/// A text field with suggestions. It is an `<input list>` with a
/// `<datalist>`, so suggestions work with no script; the component script
/// replaces the browser's list with a styled one (arrow keys, `Enter`,
/// `Escape`). Any text can be submitted, like a datalist.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Combobox![label = "Country", name = "country", options = ["India", "Germany"]]);
/// assert!(html.contains("<datalist") && html.contains(r#"<option value="India">"#), "{html}");
/// assert!(html.contains(r#"data-nr-ui="combobox""#));
/// ```
#[derive(Default)]
pub struct Combobox {
    field: Field,
    options: Vec<String>,
}

impl Combobox {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();
    field_with!();

    /// The suggestions.
    pub fn options<I, S>(mut self, v: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options = v.into_iter().map(Into::into).collect();
        self
    }
}

fn datalist(id: &str, options: Vec<String>) -> Node {
    let mut list = Element::new("datalist").with(attr("id", id.to_owned()));
    for option in options {
        list.children.push(Element::new("option").with(attr("value", option)).into_node());
    }
    list.into_node()
}

impl View for Combobox {
    fn into_node(mut self) -> Node {
        let (mut input, floats) = field_input(&mut self.field, "text");
        let list_id = format!("{}-list", self.field.control_id());
        input.set_attr(attr("list", list_id.clone()));
        input.set_attr(attr("autocomplete", "off"));
        let chevron = part("span", "nr-combobox-chevron").with(icon(icons::chevrons_up_down())).into_node();
        self.field.render(
            input.into_node(),
            Frame {
                kind: "nr-input nr-combobox",
                floats,
                after_box: vec![
                    datalist(&list_id, self.options),
                    // Filled by the script, which then drops the `list` attribute.
                    part("ul", "nr-combobox-list")
                        .with(attr("id", format!("{list_id}box")))
                        .with(attr("role", "listbox"))
                        .with(flag("hidden", true))
                        .into_node(),
                ],
                root_attrs: vec![attr("data-nr-ui", "combobox")],
                end: vec![chevron],
            },
        )
    }
}

/// Several values in one field, shown as removable tags. Without the script
/// it is a text field taking comma separated values; either way the form
/// receives one string: `"rust, web, ssr"`.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(TagsInput![label = "Topics", name = "topics", values = ["rust", "web"], max = 5]);
/// assert!(html.contains(r#"value="rust, web""#) && html.contains(r#"data-max="5""#), "{html}");
/// ```
#[derive(Default)]
pub struct TagsInput {
    field: Field,
    values: Vec<String>,
    options: Vec<String>,
    max: Option<u32>,
}

impl TagsInput {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();
    field_with!();
    props! {
        /// The most tags that can be added.
        max: opt<u32>,
    }
    /// The initial tags.
    pub fn values<I, S>(mut self, v: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.values = v.into_iter().map(Into::into).collect();
        self
    }
    /// Suggestions while typing (a `<datalist>`).
    pub fn options<I, S>(mut self, v: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options = v.into_iter().map(Into::into).collect();
        self
    }
}

impl View for TagsInput {
    fn into_node(mut self) -> Node {
        if !self.values.is_empty() {
            self.field.value = Some(self.values.join(", "));
        }
        let (mut input, floats) = field_input(&mut self.field, "text");
        input.set_attr(attr("autocomplete", "off"));
        input.set_attr(attr("enterkeyhint", "done"));
        let mut after_box = Vec::new();
        if !self.options.is_empty() {
            let list_id = format!("{}-list", self.field.control_id());
            input.set_attr(attr("list", list_id.clone()));
            after_box.push(datalist(&list_id, self.options));
        }
        let mut root_attrs = vec![attr("data-nr-ui", "tags")];
        if let Some(max) = self.max {
            root_attrs.push(attr("data-max", max.to_string()));
        }
        self.field.render(
            input.into_node(),
            Frame { kind: "nr-input nr-tags", floats, after_box, root_attrs, end: Vec::new() },
        )
    }
}

/// A slider: the browser's range input, styled.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Slider![label = "Volume", name = "volume", value = 40.0, unit = "%"]);
/// assert!(html.contains(r#"type="range""#) && html.contains(r#"value="40""#) && html.contains("--nr-v:40%"), "{html}");
/// ```
#[derive(Default)]
pub struct Slider {
    texts: Texts,
    id: Option<String>,
    value: Option<f64>,
    min: f64,
    max: Option<f64>,
    step: Option<f64>,
    unit: Option<String>,
    hide_value: bool,
    disabled: bool,
    color: Color,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl Slider {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    texts_methods!();
    /// Add an attribute to the range input; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.extra.push(part);
        self
    }
    props! {
        /// The initial value (the middle when not given).
        value: opt<f64>,
        min: val<f64>,
        /// Defaults to 100.
        max: opt<f64>,
        step: opt<f64>,
        /// Text after the shown value ("%", " km").
        unit: text,
        /// Do not show the value next to the label.
        hide_value: val<bool>,
        disabled: val<bool>,
        color: val<Color>,
        size: val<Size>,
    }
}

impl View for Slider {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let id = self.id.unwrap_or_else(|| next_id("nr-slider"));
        let max = self.max.unwrap_or(100.0);
        let value = self.value.unwrap_or((self.min + max) / 2.0).clamp(self.min, max.max(self.min));
        let percent = if max > self.min { (value - self.min) / (max - self.min) * 100.0 } else { 0.0 };
        let size = format!("nr-slider-{}", self.size.suffix());
        let mut el = root("div", &["nr-slider", self.color.class(), &size], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "slider"));
        el.set_attr(attr("style", format!("--nr-v:{}%", trim_float((percent * 100.0).round() / 100.0))));
        el.set_attr(flag("data-disabled", self.disabled));
        let mut head = part("div", "nr-slider-head");
        head.children.extend(self.texts.label(&id, false));
        if !self.hide_value {
            let mut out =
                part("output", "nr-slider-value").with(attr("for", id.clone())).with(attr("data-nr-echo", ""));
            if let Some(unit) = &self.unit {
                out.set_attr(attr("data-unit", unit.clone()));
            }
            head.children.push(out.with(format!("{}{}", trim_float(value), self.unit.unwrap_or_default())).into_node());
        }
        if !head.children.is_empty() {
            el.children.push(head.into_node());
        }
        let mut input = Element::new_void("input")
            .with(attr("class", "nr-slider-input"))
            .with(attr("type", "range"))
            .with(attr("id", id.clone()))
            .with(attr("min", trim_float(self.min)))
            .with(attr("max", trim_float(max)))
            .with(attr("value", trim_float(value)))
            .with(flag("disabled", self.disabled));
        if let Some(step) = self.step {
            input.set_attr(attr("step", trim_float(step)));
        }
        if let Some(name) = &self.texts.name {
            input.set_attr(attr("name", name.clone()));
        }
        if let Some(described) = self.texts.describedby(&id) {
            input.set_attr(described);
        }
        apply(&mut input, attrs);
        el.children.push(input.into_node());
        el.children.extend(self.texts.below(&id));
        styled(el)
    }
}

/// A one-time code or PIN. It is a single input (so paste, autofill from
/// SMS and screen readers all work) drawn as separate cells; no script.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(PinInput![label = "Code", name = "code", length = 6]);
/// assert!(html.contains(r#"autocomplete="one-time-code""#) && html.contains(r#"maxlength="6""#));
/// assert!(html.contains(r#"pattern="[0-9]{6}""#), "{html}");
/// ```
#[derive(Default)]
pub struct PinInput {
    texts: Texts,
    id: Option<String>,
    length: Option<u8>,
    value: Option<String>,
    alphanumeric: bool,
    masked: bool,
    required: bool,
    disabled: bool,
    color: Color,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl PinInput {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    texts_methods!();
    /// Add an attribute to the input; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.extra.push(part);
        self
    }
    props! {
        /// Number of characters (6 by default, 12 at most).
        length: opt<u8>,
        value: maybe,
        /// Letters as well as digits.
        alphanumeric: val<bool>,
        /// Hide the characters, like a password.
        masked: val<bool>,
        required: val<bool>,
        disabled: val<bool>,
        color: val<Color>,
        size: val<Size>,
    }
}

impl View for PinInput {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let id = self.id.unwrap_or_else(|| next_id("nr-pin"));
        let n = self.length.unwrap_or(6).clamp(1, 12);
        let size = format!("nr-pin-{}", self.size.suffix());
        let mut el = root("div", &["nr-pin", self.color.class(), &size], self.unstyled, class);
        el.set_attr(attr("style", format!("--nr-pin-n:{n}")));
        el.set_attr(flag("data-invalid", self.texts.error_message.is_some()));
        el.children.extend(self.texts.label(&id, self.required));
        let mut cells = part("span", "nr-pin-cells").with(attr("aria-hidden", "true"));
        for _ in 0..n {
            cells.children.push(Element::new("i").into_node());
        }
        let class_set = if self.alphanumeric { "[0-9A-Za-z]" } else { "[0-9]" };
        let mut input = Element::new_void("input")
            .with(attr("class", "nr-pin-input"))
            .with(attr("type", if self.masked { "password" } else { "text" }))
            .with(attr("id", id.clone()))
            .with(attr("inputmode", if self.alphanumeric { "text" } else { "numeric" }))
            .with(attr("autocomplete", "one-time-code"))
            .with(attr("maxlength", n.to_string()))
            .with(attr("pattern", format!("{class_set}{{{n}}}")))
            .with(attr("spellcheck", "false"))
            .with(flag("required", self.required))
            .with(flag("disabled", self.disabled));
        if let Some(name) = &self.texts.name {
            input.set_attr(attr("name", name.clone()));
        }
        if let Some(value) = self.value {
            input.set_attr(attr("value", value));
        }
        if self.texts.label.is_none() {
            input.set_attr(attr("aria-label", "Code"));
        }
        if let Some(described) = self.texts.describedby(&id) {
            input.set_attr(described);
        }
        if self.texts.error_message.is_some() {
            input.set_attr(attr("aria-invalid", "true"));
        }
        apply(&mut input, attrs);
        el.children.push(part("div", "nr-pin-box").with(cells).with(input).into_node());
        el.children.extend(self.texts.below(&id));
        styled(el)
    }
}

/// A star rating. As a form control it is a group of radio buttons, so it
/// works with the keyboard and with no script; `readonly` shows a value
/// (halves and other fractions included).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Rating![name = "stars", label = "Your rating", value = 4.0]);
/// assert_eq!(html.matches(r#"type="radio""#).count(), 5);
/// assert!(html.contains(r#"value="4" checked"#), "{html}");
/// let shown = render_static(Rating![value = 4.5, readonly = true]);
/// assert!(shown.contains(r#"role="img" aria-label="4.5 out of 5""#) && shown.contains("width:90%"), "{shown}");
/// ```
#[derive(Default)]
pub struct Rating {
    name: Option<String>,
    label: Option<String>,
    value: Option<f64>,
    max: Option<u8>,
    readonly: bool,
    required: bool,
    disabled: bool,
    color: Color,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl Rating {
    pub fn new() -> Self {
        Self { color: Color::Warning, ..Self::default() }
    }
    common_methods!();
    with_parts!();
    props! {
        name: text,
        /// The legend of the group.
        label: text,
        value: opt<f64>,
        /// Number of stars (5 by default).
        max: opt<u8>,
        /// Show a value instead of asking for one.
        readonly: val<bool>,
        required: val<bool>,
        disabled: val<bool>,
        /// `Warning` (amber) by default.
        color: val<Color>,
        size: val<Size>,
    }
}

impl View for Rating {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let max = self.max.unwrap_or(5).clamp(1, 10);
        let size = format!("nr-rating-{}", self.size.suffix());
        let star = || icon(icons::star());
        if self.readonly {
            let value = self.value.unwrap_or(0.0).clamp(0.0, f64::from(max));
            let mut el =
                root("span", &["nr-rating", "nr-rating-static", self.color.class(), &size], self.unstyled, class);
            el.set_attr(attr("role", "img"));
            el.set_attr(attr("aria-label", format!("{} out of {max}", trim_float(value))));
            apply(&mut el, attrs);
            let mut stars = part("span", "nr-rating-stars");
            let mut fill = part("span", "nr-rating-fill").with(attr(
                "style",
                format!("width:{}%", trim_float((value / f64::from(max) * 1000.0).round() / 10.0)),
            ));
            for _ in 0..max {
                stars.children.push(star());
                fill.children.push(star());
            }
            stars.children.push(fill.into_node());
            el.children.push(stars.into_node());
            return styled(el);
        }
        let mut el = root("fieldset", &["nr-rating", self.color.class(), &size], self.unstyled, class);
        el.set_attr(flag("disabled", self.disabled));
        apply(&mut el, attrs);
        let label = self.label.unwrap_or_else(|| "Rating".to_owned());
        el.children.push(part("legend", "nr-rating-legend").with(label).into_node());
        let name = self.name.unwrap_or_else(|| next_id("nr-rating"));
        let chosen = self.value.map(|v| v.round() as u8);
        let mut items = part("span", "nr-rating-items");
        for i in 1..=max {
            items.children.push(
                part("label", "nr-rating-item")
                    .with(
                        Element::new_void("input")
                            .with(attr("class", "nr-rating-input"))
                            .with(attr("type", "radio"))
                            .with(attr("name", name.clone()))
                            .with(attr("value", i.to_string()))
                            .with(flag("checked", chosen == Some(i)))
                            .with(flag("required", self.required && i == 1)),
                    )
                    .with(star())
                    .with(part("span", "nr-sr").with(format!("{i} star{}", if i == 1 { "" } else { "s" })))
                    .into_node(),
            );
        }
        el.children.push(items.into_node());
        styled(el)
    }
}

/// One choice of a [`SegmentedControl`].
#[derive(Default)]
pub struct Segment {
    value: Option<String>,
    start_content: Option<Node>,
    disabled: bool,
    extra: Extra,
}

impl Segment {
    pub fn new() -> Self {
        Self::default()
    }
    with_parts!();
    props! {
        /// The submitted value (the label when not given).
        value: text,
        /// Content before the label, typically an icon.
        start_content: node,
        disabled: val<bool>,
    }
}

/// Values accepted inside `SegmentedControl![..]`.
pub trait SegmentPart {
    fn add_to(self, control: &mut SegmentedControl);
}
impl SegmentPart for Segment {
    fn add_to(self, control: &mut SegmentedControl) {
        control.segments.push(self);
    }
}
impl SegmentPart for Vec<Segment> {
    fn add_to(self, control: &mut SegmentedControl) {
        control.segments.extend(self);
    }
}
impl SegmentPart for Attr {
    fn add_to(self, control: &mut SegmentedControl) {
        control.extra.push(self);
    }
}
/// `("value", "Label")`.
impl SegmentPart for (&str, &str) {
    fn add_to(self, control: &mut SegmentedControl) {
        control.segments.push(Segment::new().value(self.0).with(self.1.to_owned()));
    }
}
impl<const N: usize> SegmentPart for [(&str, &str); N] {
    fn add_to(self, control: &mut SegmentedControl) {
        for item in self {
            item.add_to(control);
        }
    }
}

/// A few exclusive choices side by side: radio buttons drawn as one
/// control. No script.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = SegmentedControl![name = "view", value = "week", label = "View", [("day", "Day"), ("week", "Week")]];
/// let html = render_static(s);
/// assert!(html.contains(r#"role="radiogroup""#) && html.contains(r#"value="week" checked"#), "{html}");
/// ```
#[derive(Default)]
pub struct SegmentedControl {
    name: Option<String>,
    value: Option<String>,
    label: Option<String>,
    segments: Vec<Segment>,
    full_width: bool,
    color: Color,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl SegmentedControl {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    props! {
        name: text,
        /// The chosen value: `value = form.value("view")`.
        value: maybe,
        /// Accessible name of the group.
        label: text,
        full_width: val<bool>,
        color: val<Color>,
        size: val<Size>,
    }
    /// Add choices ([`Segment`], `("value", "Label")`) or attributes.
    pub fn with(mut self, part: impl SegmentPart) -> Self {
        part.add_to(&mut self);
        self
    }
}

impl View for SegmentedControl {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let size = format!("nr-segmented-{}", self.size.suffix());
        let classes =
            ["nr-segmented", self.color.class(), &size, if self.full_width { "nr-segmented-full" } else { "" }];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(attr("role", "radiogroup"));
        if let Some(label) = self.label {
            el.set_attr(attr("aria-label", label));
        }
        apply(&mut el, attrs);
        let name = self.name.unwrap_or_else(|| next_id("nr-segmented"));
        for segment in self.segments {
            let (_, seg_attrs, children) = segment.extra.split();
            let value = segment.value.unwrap_or_else(|| text_of(&children));
            let mut input = Element::new_void("input")
                .with(attr("class", "nr-segment-input"))
                .with(attr("type", "radio"))
                .with(attr("name", name.clone()))
                .with(attr("value", value.clone()))
                .with(flag("checked", self.value.as_deref() == Some(value.as_str())))
                .with(flag("disabled", segment.disabled));
            apply(&mut input, seg_attrs);
            let mut text = part("span", "nr-segment-label");
            text.children.extend(segment.start_content);
            text.children.extend(children);
            el.children.push(part("label", "nr-segment").with(input).with(text).into_node());
        }
        styled(el)
    }
}

/// The plain text of nodes (a value for a choice given only a label).
fn text_of(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Element(el) => out.push_str(&text_of(&el.children)),
            Node::Fragment(inner) => out.push_str(&text_of(inner)),
            _ => {}
        }
    }
    out
}

/// A file field drawn as a drop area. Dropping files on it and choosing
/// them with a click are the browser's own, so both work with no script;
/// the script adds the highlight while dragging and lists the chosen files.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(FileUpload![name = "photo", label = "Photo", accept = "image/*", hint = "PNG or JPG, up to 5 MB"]);
/// assert!(html.contains(r#"type="file""#) && html.contains(r#"accept="image/*""#), "{html}");
/// ```
#[derive(Default)]
pub struct FileUpload {
    texts: Texts,
    id: Option<String>,
    accept: Option<String>,
    multiple: bool,
    required: bool,
    disabled: bool,
    title: Option<String>,
    hint: Option<String>,
    icon: Option<Node>,
    color: Color,
    unstyled: bool,
    extra: Extra,
}

impl FileUpload {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    texts_methods!();
    /// Add an attribute to the file input; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.extra.push(part);
        self
    }
    props! {
        /// Accepted types: `"image/*"`, `".pdf,.docx"`.
        accept: text,
        multiple: val<bool>,
        required: val<bool>,
        disabled: val<bool>,
        /// The main line ("Click to upload or drag and drop" by default).
        title: text,
        /// A muted line under it: formats, size limits.
        hint: text,
        /// Your own icon.
        icon: node,
        color: val<Color>,
    }
}

impl View for FileUpload {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let id = self.id.unwrap_or_else(|| next_id("nr-upload"));
        let mut el = root("div", &["nr-upload", self.color.class()], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "upload"));
        el.set_attr(flag("data-disabled", self.disabled));
        el.children.extend(self.texts.label(&id, self.required));
        let mut zone = part("div", "nr-dropzone");
        zone.children.push(
            part("span", "nr-dropzone-icon").with(self.icon.unwrap_or_else(|| icon(icons::cloud_upload()))).into_node(),
        );
        zone.children.push(
            part("span", "nr-dropzone-title")
                .with(self.title.unwrap_or_else(|| "Click to upload or drag and drop".to_owned()))
                .into_node(),
        );
        if let Some(hint) = self.hint {
            zone.children.push(part("span", "nr-dropzone-hint").with(hint).into_node());
        }
        let mut input = Element::new_void("input")
            .with(attr("class", "nr-dropzone-input"))
            .with(attr("type", "file"))
            .with(attr("id", id.clone()))
            .with(flag("multiple", self.multiple))
            .with(flag("required", self.required))
            .with(flag("disabled", self.disabled));
        if let Some(name) = &self.texts.name {
            input.set_attr(attr("name", name.clone()));
        }
        if let Some(accept) = self.accept {
            input.set_attr(attr("accept", accept));
        }
        if self.texts.label.is_none() {
            input.set_attr(attr("aria-label", "Upload a file"));
        }
        if let Some(described) = self.texts.describedby(&id) {
            input.set_attr(described);
        }
        apply(&mut input, attrs);
        zone.children.push(input.into_node());
        zone.children.push(
            part("ul", "nr-dropzone-files").with(flag("hidden", true)).with(attr("aria-live", "polite")).into_node(),
        );
        el.children.push(zone.into_node());
        el.children.extend(self.texts.below(&id));
        styled(el)
    }
}

/// A color field: the browser's color picker behind a swatch.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(ColorPicker![label = "Brand color", name = "brand", value = "#7828c8"]);
/// assert!(html.contains(r##"type="color" value="#7828c8""##), "{html}");
/// ```
#[derive(Default)]
pub struct ColorPicker {
    label: Option<String>,
    name: Option<String>,
    value: Option<String>,
    disabled: bool,
    size: Size,
    unstyled: bool,
    extra: Extra,
}

impl ColorPicker {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    /// Add an attribute to the color input; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.extra.push(part);
        self
    }
    props! {
        label: text,
        name: text,
        /// `#rrggbb`.
        value: maybe,
        disabled: val<bool>,
        size: val<Size>,
    }
}

impl View for ColorPicker {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let size = format!("nr-color-{}", self.size.suffix());
        let mut el = root("label", &["nr-color", &size], self.unstyled, class);
        el.set_attr(attr("data-nr-ui", "color"));
        el.set_attr(flag("data-disabled", self.disabled));
        let value = self.value.unwrap_or_else(|| "#006fee".to_owned());
        let mut input = Element::new_void("input")
            .with(attr("class", "nr-color-input"))
            .with(attr("type", "color"))
            .with(attr("value", value.clone()))
            .with(flag("disabled", self.disabled));
        if let Some(name) = self.name {
            input.set_attr(attr("name", name));
        }
        if self.label.is_none() {
            input.set_attr(attr("aria-label", "Color"));
        }
        apply(&mut input, attrs);
        el.children.push(input.into_node());
        let mut text = part("span", "nr-color-text");
        if let Some(label) = self.label {
            text.children.push(part("span", "nr-color-label").with(label).into_node());
        }
        text.children.push(part("output", "nr-color-value").with(attr("data-nr-echo", "")).with(value).into_node());
        el.children.push(text.into_node());
        styled(el)
    }
}

/// A start and an end date: two [`DatePicker`](struct@crate::DatePicker)s that
/// bound each other (the start cannot be after the end). The form receives
/// `<name>_start` and `<name>_end`.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(DateRangePicker![label = "Stay", name = "stay", start = "2026-09-01", end = "2026-09-05"]);
/// assert!(html.contains(r#"name="stay_start""#) && html.contains(r#"name="stay_end""#));
/// assert!(html.contains(r#"role="group""#) && html.contains(r#"max="2026-09-05""#), "{html}");
/// ```
#[derive(Default)]
pub struct DateRangePicker {
    label: Option<String>,
    name: Option<String>,
    start: Option<String>,
    end: Option<String>,
    start_label: Option<String>,
    end_label: Option<String>,
    min: Option<String>,
    max: Option<String>,
    required: bool,
    first_day_of_week: Option<u8>,
    color: Color,
    size: Size,
    variant: FieldVariant,
    unstyled: bool,
    extra: Extra,
}

impl DateRangePicker {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// Label of the pair.
        label: text,
        /// Prefix of the two field names: `stay` gives `stay_start` and `stay_end`.
        name: text,
        /// The start date (`YYYY-MM-DD`).
        start: maybe,
        /// The end date (`YYYY-MM-DD`).
        end: maybe,
        /// "From" by default.
        start_label: text,
        /// "To" by default.
        end_label: text,
        min: text,
        max: text,
        required: val<bool>,
        /// 0 = Sunday (default), 1 = Monday, ...
        first_day_of_week: opt<u8>,
        color: val<Color>,
        size: val<Size>,
        variant: val<FieldVariant>,
    }
}

impl View for DateRangePicker {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root("div", &["nr-daterange"], self.unstyled, class);
        el.set_attr(attr("role", "group"));
        apply(&mut el, attrs);
        if let Some(label) = self.label {
            let id = next_id("nr-daterange");
            el.set_attr(attr("aria-labelledby", id.clone()));
            el.children.push(part("span", "nr-field-label").with(attr("id", id)).with(label).into_node());
        }
        let picker = |label: String, suffix: &str, value: &Option<String>, min: Option<String>, max: Option<String>| {
            let mut p = DatePicker::new()
                .label(label)
                .value(value.clone())
                .required(self.required)
                .color(self.color)
                .size(self.size)
                .variant(self.variant);
            if let Some(name) = &self.name {
                p = p.name(format!("{name}_{suffix}"));
            }
            if let Some(min) = min {
                p = p.min(min);
            }
            if let Some(max) = max {
                p = p.max(max);
            }
            if let Some(day) = self.first_day_of_week {
                p = p.first_day_of_week(day);
            }
            p
        };
        let from = picker(
            self.start_label.clone().unwrap_or_else(|| "From".to_owned()),
            "start",
            &self.start,
            self.min.clone(),
            self.end.clone().or_else(|| self.max.clone()),
        );
        let to = picker(
            self.end_label.clone().unwrap_or_else(|| "To".to_owned()),
            "end",
            &self.end,
            self.start.clone().or_else(|| self.min.clone()),
            self.max.clone(),
        );
        el.children.push(
            part("div", "nr-daterange-fields")
                .with(from)
                .with(
                    part("span", "nr-daterange-sep").with(attr("aria-hidden", "true")).with(icon(icons::arrow_right())),
                )
                .with(to)
                .into_node(),
        );
        styled(el)
    }
}

/// A label, help text and an error slot around any control: a native
/// `select!`, a group of checkboxes, your own widget. The error slot is
/// wired to `name`, so server-action validation errors appear in it.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let f = FormField![label = "Avatar", name = "avatar", control_id = "avatar", required = true,
///     input![id("avatar"), r#type("file"), name("avatar")]];
/// let html = render_static(f);
/// assert!(html.contains(r#"<label class="nr-field-label" for="avatar">"#) && html.contains(r#"data-nr-error="avatar""#));
/// ```
#[derive(Default)]
pub struct FormField {
    texts: Texts,
    control_id: Option<String>,
    required: bool,
    horizontal: bool,
    unstyled: bool,
    extra: Extra,
}

impl FormField {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// The `id` of the control inside, so the label points at it.
        control_id: text,
        /// An asterisk after the label.
        required: val<bool>,
        /// Label on the left, control on the right (from 640px up).
        horizontal: val<bool>,
    }
    /// Text of the label.
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.texts.label = Some(v.into());
        self
    }
    /// Help text under the control.
    pub fn description(mut self, v: impl MaybeText) -> Self {
        self.texts.description = v.into_text();
        self
    }
    /// Error text: `error_message = form.error("avatar")`.
    pub fn error_message(mut self, v: impl MaybeText) -> Self {
        self.texts.error_message = v.into_text();
        self
    }
    /// The field name whose validation errors show here.
    pub fn name(mut self, v: impl Into<String>) -> Self {
        self.texts.name = Some(v.into());
        self
    }
}

impl View for FormField {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let classes = ["nr-formfield", if self.horizontal { "nr-formfield-row" } else { "" }];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(flag("data-invalid", self.texts.error_message.is_some()));
        apply(&mut el, attrs);
        let id = self.control_id.unwrap_or_else(|| next_id("nr-formfield"));
        el.children.extend(self.texts.label(&id, self.required));
        let mut main = part("div", "nr-formfield-main");
        main.children.push(part("div", "nr-formfield-control").with(children).into_node());
        main.children.extend(self.texts.below(&id));
        el.children.push(main.into_node());
        styled(el)
    }
}

/// A titled part of a long form: the title and description on the left,
/// the fields on the right (stacked on small screens).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = FormSection![title = "Profile", description = "Shown on your public page.", Input![label = "Name"]];
/// assert!(render_static(s).contains(r#"<h3 class="nr-formsection-title">Profile</h3>"#));
/// ```
#[derive(Default)]
pub struct FormSection {
    title: Option<String>,
    description: Option<String>,
    unstyled: bool,
    extra: Extra,
}

impl FormSection {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        title: text,
        description: text,
    }
}

impl View for FormSection {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let mut el = root("section", &["nr-formsection"], self.unstyled, class);
        apply(&mut el, attrs);
        let mut head = part("div", "nr-formsection-head");
        if let Some(title) = self.title {
            head.children.push(part("h3", "nr-formsection-title").with(title).into_node());
        }
        if let Some(description) = self.description {
            head.children.push(part("p", "nr-formsection-description").with(description).into_node());
        }
        el.children.push(head.into_node());
        el.children.push(part("div", "nr-formsection-body").with(children).into_node());
        styled(el)
    }
}

/// The row of buttons at the end of a form.
#[derive(Default)]
pub struct FormActions {
    justify: Option<Justify>,
    bordered: bool,
    unstyled: bool,
    extra: Extra,
}

impl FormActions {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();
    with_parts!();
    props! {
        /// `End` by default; `Between` puts a secondary action on the other side.
        justify: opt<Justify>,
        /// A line above.
        bordered: val<bool>,
    }
}

impl View for FormActions {
    fn into_node(self) -> Node {
        let (class, attrs, children) = self.extra.split();
        let justify = match self.justify {
            Some(Justify::Start) => "nr-formactions-start",
            Some(Justify::Center) => "nr-formactions-center",
            Some(Justify::Between) => "nr-formactions-between",
            _ => "",
        };
        let classes = ["nr-formactions", justify, if self.bordered { "nr-formactions-bordered" } else { "" }];
        let mut el = root("div", &classes, self.unstyled, class);
        apply(&mut el, attrs);
        el.children = children;
        styled(el)
    }
}
