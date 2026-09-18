//! Checkbox, switch and radio buttons. Real form inputs, styled; no script.

use next_rust_view::{Attr, Element, Node, View};

use crate::button::common_methods;
use crate::icons::{self, icon};
use crate::{Color, Extra, MaybeText, Radius, Size, apply, attr, flag, next_id, root, styled};

/// Properties shared by checkbox, switch and radio.
#[derive(Default)]
struct Toggle {
    name: Option<String>,
    value: Option<String>,
    checked: bool,
    disabled: bool,
    required: bool,
    color: Color,
    size: Size,
    radius: Option<Radius>,
    description: Option<String>,
    unstyled: bool,
    extra: Extra,
}

macro_rules! toggle_methods {
    () => {
        pub fn name(mut self, v: impl Into<String>) -> Self {
            self.t.name = Some(v.into());
            self
        }
        /// Submitted value (`on` when not given).
        pub fn value(mut self, v: impl Into<String>) -> Self {
            self.t.value = Some(v.into());
            self
        }
        pub fn checked(mut self, v: bool) -> Self {
            self.t.checked = v;
            self
        }
        pub fn disabled(mut self, v: bool) -> Self {
            self.t.disabled = v;
            self
        }
        pub fn required(mut self, v: bool) -> Self {
            self.t.required = v;
            self
        }
        pub fn color(mut self, v: Color) -> Self {
            self.t.color = v;
            self
        }
        pub fn size(mut self, v: Size) -> Self {
            self.t.size = v;
            self
        }
        pub fn radius(mut self, v: Radius) -> Self {
            self.t.radius = Some(v);
            self
        }
        /// Secondary text under the label.
        pub fn description(mut self, v: impl MaybeText) -> Self {
            self.t.description = v.into_text();
            self
        }
        /// Extra classes for the outer element. They win over the defaults.
        pub fn class(mut self, v: impl Into<String>) -> Self {
            self.t.extra.push(next_rust_view::attrs::class(v.into()));
            self
        }
        pub fn unstyled(mut self, v: bool) -> Self {
            self.t.unstyled = v;
            self
        }
        /// Add an attribute to the `<input>` or a child to the label;
        /// `class(..)` goes to the outer element.
        pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
            self.t.extra.push(part);
            self
        }
    };
}

impl Toggle {
    /// `<label class=…><input …>{control}<span label>…</span></label>`.
    fn render(self, kind: &'static str, input_type: &str, role: Option<&str>, control: Node) -> Node {
        let (class, attrs, children) = self.extra.split();
        let size = format!("{kind}-{}", self.size.suffix());
        let classes = [kind, self.color.class(), &size, self.radius.map(Radius::class).unwrap_or("")];
        let mut el = root("label", &classes, self.unstyled, class);
        el.set_attr(flag("data-disabled", self.disabled));
        let id = next_id(kind);
        let mut input = Element::new_void("input")
            .with(attr("class", format!("{kind}-input")))
            .with(attr("type", input_type.to_owned()))
            .with(flag("checked", self.checked))
            .with(flag("disabled", self.disabled))
            .with(flag("required", self.required));
        if let Some(role) = role {
            input.set_attr(attr("role", role.to_owned()));
        }
        if let Some(n) = self.name {
            input.set_attr(attr("name", n));
        }
        if let Some(v) = self.value {
            input.set_attr(attr("value", v));
        }
        if self.description.is_some() {
            input.set_attr(attr("aria-describedby", format!("{id}-description")));
        }
        apply(&mut input, attrs);
        el.children.push(input.into_node());
        el.children.push(control);
        if !children.is_empty() || self.description.is_some() {
            let mut text = Element::new("span").with(attr("class", format!("{kind}-text")));
            if !children.is_empty() {
                text = text.with(Element::new("span").with(attr("class", format!("{kind}-label"))).with(children));
            }
            if let Some(d) = self.description {
                text = text.with(
                    Element::new("span")
                        .with(attr("class", "nr-toggle-description"))
                        .with(attr("id", format!("{id}-description")))
                        .with(d),
                );
            }
            el.children.push(text.into_node());
        }
        styled(el)
    }
}

/// A checkbox with a label.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Checkbox![name = "terms", required = true, "I agree"]);
/// assert!(html.contains(r#"<input class="nr-checkbox-input" type="checkbox" required name="terms">"#));
/// ```
#[derive(Default)]
pub struct Checkbox {
    t: Toggle,
}

impl Checkbox {
    pub fn new() -> Self {
        Self::default()
    }
    toggle_methods!();
}

impl View for Checkbox {
    fn into_node(self) -> Node {
        let box_ = Element::new("span")
            .with(attr("class", "nr-checkbox-box"))
            .with(attr("aria-hidden", "true"))
            .with(icon(icons::check().stroke_width(3.0)))
            .into_node();
        self.t.render("nr-checkbox", "checkbox", None, box_)
    }
}

/// An on/off switch.
#[derive(Default)]
pub struct Switch {
    t: Toggle,
}

impl Switch {
    pub fn new() -> Self {
        Self::default()
    }
    toggle_methods!();
}

impl View for Switch {
    fn into_node(self) -> Node {
        let track = Element::new("span")
            .with(attr("class", "nr-switch-track"))
            .with(attr("aria-hidden", "true"))
            .with(Element::new("span").with(attr("class", "nr-switch-thumb")))
            .into_node();
        self.t.render("nr-switch", "checkbox", Some("switch"), track)
    }
}

/// One choice of a [`RadioGroup`].
#[derive(Default)]
pub struct Radio {
    t: Toggle,
}

impl Radio {
    pub fn new() -> Self {
        Self::default()
    }
    toggle_methods!();
}

impl View for Radio {
    fn into_node(self) -> Node {
        let dot =
            Element::new("span").with(attr("class", "nr-radio-circle")).with(attr("aria-hidden", "true")).into_node();
        self.t.render("nr-radio", "radio", None, dot)
    }
}

/// Values accepted inside `RadioGroup![..]`.
pub trait RadioGroupPart {
    fn add_to(self, group: &mut RadioGroup);
}
impl RadioGroupPart for Radio {
    fn add_to(self, group: &mut RadioGroup) {
        group.radios.push(self);
    }
}
impl RadioGroupPart for Vec<Radio> {
    fn add_to(self, group: &mut RadioGroup) {
        group.radios.extend(self);
    }
}
impl RadioGroupPart for Attr {
    fn add_to(self, group: &mut RadioGroup) {
        group.extra.push(self);
    }
}

/// Radio buttons sharing a name, with a label, description and error.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(RadioGroup![label = "Plan", name = "plan", value = "pro",
///     Radio![value = "free", "Free"], Radio![value = "pro", "Pro"]]);
/// assert!(html.contains(r#"type="radio" checked name="plan" value="pro""#));
/// ```
#[derive(Default)]
pub struct RadioGroup {
    label: Option<String>,
    name: Option<String>,
    value: Option<String>,
    description: Option<String>,
    error_message: Option<String>,
    horizontal: bool,
    disabled: bool,
    required: bool,
    color: Color,
    size: Size,
    radios: Vec<Radio>,
    unstyled: bool,
    extra: Extra,
}

impl RadioGroup {
    pub fn new() -> Self {
        Self::default()
    }
    common_methods!();

    /// Add radios or attributes for the group; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl RadioGroupPart) -> Self {
        part.add_to(&mut self);
        self
    }
    pub fn label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    /// Name shared by the radios.
    pub fn name(mut self, v: impl Into<String>) -> Self {
        self.name = Some(v.into());
        self
    }
    /// The selected value.
    pub fn value(mut self, v: impl MaybeText) -> Self {
        self.value = v.into_text();
        self
    }
    pub fn description(mut self, v: impl MaybeText) -> Self {
        self.description = v.into_text();
        self
    }
    pub fn error_message(mut self, v: impl MaybeText) -> Self {
        self.error_message = v.into_text();
        self
    }
    /// Lay the radios out in a row.
    pub fn horizontal(mut self, v: bool) -> Self {
        self.horizontal = v;
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    pub fn required(mut self, v: bool) -> Self {
        self.required = v;
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
}

impl View for RadioGroup {
    fn into_node(self) -> Node {
        let (class, attrs, _) = self.extra.split();
        let mut el = root(
            "fieldset",
            &["nr-radio-group", if self.horizontal { "nr-radio-group-row" } else { "" }],
            self.unstyled,
            class,
        );
        el.set_attr(flag("disabled", self.disabled));
        el.set_attr(flag("data-invalid", self.error_message.is_some()));
        el.set_attr(attr("role", "radiogroup"));
        apply(&mut el, attrs);
        if let Some(label) = self.label {
            el.children.push(Element::new("legend").with(attr("class", "nr-field-label")).with(label).into_node());
        }
        let mut items = Element::new("div").with(attr("class", "nr-radio-group-items"));
        for mut radio in self.radios {
            if radio.t.name.is_none() {
                radio.t.name = self.name.clone();
            }
            if self.value.is_some() && radio.t.value == self.value {
                radio.t.checked = true;
            }
            radio.t.color = self.color;
            radio.t.size = self.size;
            radio.t.required |= self.required;
            items.children.push(radio.into_node());
        }
        el.children.push(items.into_node());
        if let Some(d) = self.description {
            el.children.push(Element::new("p").with(attr("class", "nr-field-description")).with(d).into_node());
        }
        if self.error_message.is_some() || self.name.is_some() {
            let mut err = Element::new("p").with(attr("class", "nr-field-error")).with(attr("aria-live", "polite"));
            if let Some(name) = &self.name {
                err = err.with(attr("data-nr-error", name.clone()));
            }
            el.children.push(err.with(self.error_message).into_node());
        }
        styled(el)
    }
}
