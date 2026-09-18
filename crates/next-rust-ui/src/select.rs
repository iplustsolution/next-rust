//! A custom select with a listbox popover.

use next_rust_view::{Attr, Element, Node, View};

use crate::icons::{self, icon};
use crate::input::{Field, Frame, field_methods};
use crate::{Extra, apply, attr, flag, next_id};

/// One option of a [`Select`]. The text children are its label.
///
/// `SelectItem![value = "in", description = "+91", "India"]`
#[derive(Default)]
pub struct SelectItem {
    value: Option<String>,
    label: Option<String>,
    description: Option<String>,
    start_content: Option<Node>,
    disabled: bool,
    extra: Extra,
}

impl SelectItem {
    pub fn new() -> Self {
        Self::default()
    }
    /// Submitted value (defaults to the label).
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }
    /// The text shown; children text works too.
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    /// Secondary text under the label.
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = Some(v.into());
        self
    }
    /// An icon or avatar before the label.
    pub fn start_content(mut self, v: impl View) -> Self {
        self.start_content = Some(v.into_node());
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    /// Add an attribute to the option, or children (their text is the label).
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.extra.push(part);
        self
    }
}

/// Values accepted inside `Select![..]`: items and attributes for the control.
pub trait SelectPart {
    fn add_to(self, select: &mut Select);
}

impl SelectPart for SelectItem {
    fn add_to(self, select: &mut Select) {
        select.items.push(self);
    }
}
impl SelectPart for Vec<SelectItem> {
    fn add_to(self, select: &mut Select) {
        select.items.extend(self);
    }
}
impl SelectPart for Attr {
    fn add_to(self, select: &mut Select) {
        select.field.extra.push(self);
    }
}
/// `("value", "Label")`.
impl SelectPart for (&str, &str) {
    fn add_to(self, select: &mut Select) {
        select.items.push(SelectItem::new().value(self.0).label(self.1));
    }
}
impl<const N: usize> SelectPart for [(&str, &str); N] {
    fn add_to(self, select: &mut Select) {
        for item in self {
            item.add_to(select);
        }
    }
}

/// A select with a styled listbox: keyboard navigation, type-ahead, single or
/// multiple choice. Submits like a native `<select>` (it is one underneath).
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let s = Select![label = "Country", name = "country", value = "in",
///     SelectItem![value = "in", "India"], SelectItem![value = "us", "United States"]];
/// let html = render_static(s);
/// assert!(html.contains(r#"<option value="in" selected>India</option>"#));
/// assert!(html.contains(r#"role="option""#));
/// ```
#[derive(Default)]
pub struct Select {
    field: Field,
    items: Vec<SelectItem>,
    multiple: bool,
    values: Vec<String>,
}

impl Select {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();

    /// Add options ([`SelectItem`], `("value", "Label")`) or attributes for
    /// the control; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl SelectPart) -> Self {
        part.add_to(&mut self);
        self
    }
    /// Options, as a list.
    pub fn items(mut self, v: impl IntoIterator<Item = SelectItem>) -> Self {
        self.items.extend(v);
        self
    }
    /// Allow choosing several options.
    pub fn multiple(mut self, v: bool) -> Self {
        self.multiple = v;
        self
    }
    /// Selected values when `multiple`.
    pub fn values<S: Into<String>>(mut self, v: impl IntoIterator<Item = S>) -> Self {
        self.values = v.into_iter().map(Into::into).collect();
        self
    }
}

/// Text of the text nodes among `nodes`.
fn text_of(nodes: &[Node]) -> String {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Fragment(children) => out.push_str(&text_of(children)),
            _ => {}
        }
    }
    out.trim().to_owned()
}

impl View for Select {
    fn into_node(mut self) -> Node {
        let (class, attrs, _) = std::mem::take(&mut self.field.extra).split();
        if let Some(class) = class {
            self.field.extra.push(class);
        }
        let selected: Vec<String> = match (&self.field.value, self.multiple) {
            (_, true) => self.values.clone(),
            (Some(v), false) => vec![v.clone()],
            (None, false) => Vec::new(),
        };
        let id = self.field.control_id();
        let list_id = next_id("nr-listbox");
        let placeholder = self.field.placeholder.clone();

        let mut native = Element::new("select").with(attr(
            "class",
            match &self.field.input_class {
                Some(c) => format!("nr-field-input nr-select-native {c}"),
                None => "nr-field-input nr-select-native".to_owned(),
            },
        ));
        apply(&mut native, self.field.control_attrs());
        native.set_attr(flag("multiple", self.multiple));
        apply(&mut native, attrs);
        if !self.multiple {
            // The empty choice: the placeholder, or nothing.
            native.children.push(
                Element::new("option")
                    .with(attr("value", ""))
                    .with(flag("selected", selected.is_empty()))
                    .with(flag("disabled", self.field.required))
                    .with(placeholder.clone().unwrap_or_default())
                    .into_node(),
            );
        }

        let mut list = Element::new("ul")
            .with(attr("class", "nr-select-list"))
            .with(attr("id", list_id.clone()))
            .with(attr("role", "listbox"))
            .with(attr("tabindex", "-1"))
            .with(flag("hidden", true));
        if self.multiple {
            list.set_attr(attr("aria-multiselectable", "true"));
        }

        let mut chosen = Vec::new();
        for (i, item) in self.items.into_iter().enumerate() {
            let (_, item_attrs, children) = item.extra.split();
            let label = item.label.clone().unwrap_or_else(|| text_of(&children));
            let value = item.value.clone().unwrap_or_else(|| label.clone());
            let is_selected = selected.contains(&value);
            if is_selected {
                chosen.push(label.clone());
            }
            native.children.push(
                Element::new("option")
                    .with(attr("value", value.clone()))
                    .with(flag("selected", is_selected))
                    .with(flag("disabled", item.disabled))
                    .with(label.clone())
                    .into_node(),
            );
            let mut option = Element::new("li")
                .with(attr("class", "nr-select-option"))
                .with(attr("id", format!("{list_id}-{i}")))
                .with(attr("role", "option"))
                .with(attr("data-value", value))
                .with(attr("aria-selected", if is_selected { "true" } else { "false" }));
            if item.disabled {
                option.set_attr(attr("aria-disabled", "true"));
            }
            apply(&mut option, item_attrs);
            if let Some(start) = item.start_content {
                option
                    .children
                    .push(Element::new("span").with(attr("class", "nr-select-option-start")).with(start).into_node());
            }
            let mut text = Element::new("span")
                .with(attr("class", "nr-select-option-text"))
                .with(Element::new("span").with(attr("class", "nr-select-option-label")).with(label));
            if let Some(d) = item.description {
                text = text.with(Element::new("span").with(attr("class", "nr-select-option-description")).with(d));
            }
            option.children.push(text.into_node());
            option.children.push(
                Element::new("span")
                    .with(attr("class", "nr-select-check"))
                    .with(icon(icons::check().stroke_width(3.0)))
                    .into_node(),
            );
            list.children.push(option.into_node());
        }

        let shown = if chosen.is_empty() { placeholder.clone().unwrap_or_default() } else { chosen.join(", ") };
        let mut trigger = Element::new("button")
            .with(attr("type", "button"))
            .with(attr("class", "nr-select-trigger"))
            .with(attr("aria-haspopup", "listbox"))
            .with(attr("aria-expanded", "false"))
            .with(attr("aria-controls", list_id))
            .with(flag("disabled", self.field.disabled))
            .with(
                Element::new("span")
                    .with(attr("class", "nr-select-value"))
                    .with(flag("data-placeholder", chosen.is_empty()))
                    .with(shown),
            );
        if self.field.label.is_some() {
            trigger.set_attr(attr("aria-labelledby", format!("{id}-label {id}-trigger")));
        }
        trigger.set_attr(attr("id", format!("{id}-trigger")));

        let floats = placeholder.is_none();
        let root_attrs: Vec<Attr> = vec![
            attr("data-nr-ui", "select"),
            flag("data-filled", !chosen.is_empty()),
            flag("data-multiple", self.multiple),
        ];
        let end = vec![
            Element::new("span").with(attr("class", "nr-select-chevron")).with(icon(icons::chevron_down())).into_node(),
        ];
        let control = Node::Fragment(vec![native.into_node(), trigger.into_node()]);
        self.field
            .render(control, Frame { kind: "nr-select", floats, after_box: vec![list.into_node()], root_attrs, end })
    }
}
