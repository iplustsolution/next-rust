//! Text fields: [`Input`], [`PasswordInput`], [`Textarea`], and the field
//! frame shared with [`crate::Select`] and [`crate::DatePicker`].

use next_rust_view::{Attr, Element, Node, View};

use crate::icons::{self, icon};
use crate::{
    Color, Extra, FieldVariant, MaybeText, Radius, Size, apply, attr, flag, next_id, part_class, root, styled,
};

/// Where a field's label goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPlacement {
    /// Inside the box: it floats above the value once there is one.
    #[default]
    Inside,
    /// Above the box.
    Outside,
    /// To the left of the box.
    OutsideLeft,
}

/// Properties every field has.
#[derive(Default)]
pub(crate) struct Field {
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub description: Option<String>,
    pub error_message: Option<String>,
    pub invalid: bool,
    pub name: Option<String>,
    pub id: Option<String>,
    pub value: Option<String>,
    pub required: bool,
    pub disabled: bool,
    pub readonly: bool,
    pub color: Color,
    pub size: Size,
    pub variant: FieldVariant,
    pub radius: Option<Radius>,
    pub label_placement: LabelPlacement,
    pub start_content: Option<Node>,
    pub end_content: Option<Node>,
    pub full_width: Option<bool>,
    pub label_class: Option<String>,
    pub wrapper_class: Option<String>,
    pub input_class: Option<String>,
    pub description_class: Option<String>,
    pub error_class: Option<String>,
    pub unstyled: bool,
    pub extra: Extra,
}

/// The builder methods for [`Field`] properties.
macro_rules! field_methods {
    () => {
        /// Text of the label.
        pub fn label(mut self, v: impl Into<String>) -> Self {
            self.field.label = Some(v.into());
            self
        }
        pub fn placeholder(mut self, v: impl Into<String>) -> Self {
            self.field.placeholder = Some(v.into());
            self
        }
        /// Help text under the field.
        pub fn description(mut self, v: impl crate::MaybeText) -> Self {
            self.field.description = v.into_text();
            self
        }
        /// Error text under the field; also marks it invalid.
        /// `error_message = form.error("email")` shows server validation errors.
        pub fn error_message(mut self, v: impl crate::MaybeText) -> Self {
            self.field.error_message = v.into_text();
            self
        }
        /// Show the field as invalid without a message.
        pub fn invalid(mut self, v: bool) -> Self {
            self.field.invalid = v;
            self
        }
        /// Form field name. Server-action validation errors for this name
        /// appear under the field automatically.
        pub fn name(mut self, v: impl Into<String>) -> Self {
            self.field.name = Some(v.into());
            self
        }
        /// `id` of the control (generated when not given).
        pub fn id(mut self, v: impl Into<String>) -> Self {
            self.field.id = Some(v.into());
            self
        }
        /// Initial value: `value = form.value("email")`.
        pub fn value(mut self, v: impl crate::MaybeText) -> Self {
            self.field.value = v.into_text();
            self
        }
        pub fn required(mut self, v: bool) -> Self {
            self.field.required = v;
            self
        }
        pub fn disabled(mut self, v: bool) -> Self {
            self.field.disabled = v;
            self
        }
        pub fn readonly(mut self, v: bool) -> Self {
            self.field.readonly = v;
            self
        }
        /// Color of the focused field.
        pub fn color(mut self, v: crate::Color) -> Self {
            self.field.color = v;
            self
        }
        pub fn size(mut self, v: crate::Size) -> Self {
            self.field.size = v;
            self
        }
        pub fn variant(mut self, v: crate::FieldVariant) -> Self {
            self.field.variant = v;
            self
        }
        pub fn radius(mut self, v: crate::Radius) -> Self {
            self.field.radius = Some(v);
            self
        }
        pub fn label_placement(mut self, v: crate::LabelPlacement) -> Self {
            self.field.label_placement = v;
            self
        }
        /// Content at the start of the box, typically an icon.
        pub fn start_content(mut self, v: impl next_rust_view::View) -> Self {
            self.field.start_content = Some(v.into_node());
            self
        }
        /// Content at the end of the box.
        pub fn end_content(mut self, v: impl next_rust_view::View) -> Self {
            self.field.end_content = Some(v.into_node());
            self
        }
        /// Take the full width of the container (the default).
        pub fn full_width(mut self, v: bool) -> Self {
            self.field.full_width = Some(v);
            self
        }
        /// Extra classes for the label.
        pub fn label_class(mut self, v: impl Into<String>) -> Self {
            self.field.label_class = Some(v.into());
            self
        }
        /// Extra classes for the box around the control.
        pub fn wrapper_class(mut self, v: impl Into<String>) -> Self {
            self.field.wrapper_class = Some(v.into());
            self
        }
        /// Extra classes for the control itself.
        pub fn input_class(mut self, v: impl Into<String>) -> Self {
            self.field.input_class = Some(v.into());
            self
        }
        pub fn description_class(mut self, v: impl Into<String>) -> Self {
            self.field.description_class = Some(v.into());
            self
        }
        pub fn error_class(mut self, v: impl Into<String>) -> Self {
            self.field.error_class = Some(v.into());
            self
        }
        /// Extra classes for the outer element. They win over the defaults.
        pub fn class(mut self, v: impl Into<String>) -> Self {
            self.field.extra.push(next_rust_view::attrs::class(v.into()));
            self
        }
        /// Leave out the default classes of the outer element.
        pub fn unstyled(mut self, v: bool) -> Self {
            self.field.unstyled = v;
            self
        }
    };
}
pub(crate) use field_methods;

/// What a field renders around its control.
pub(crate) struct Frame {
    /// Component class: `nr-input`, `nr-select`, ...
    pub kind: &'static str,
    /// The label floats inside the box (the control shows no placeholder).
    pub floats: bool,
    /// Popovers, placed at the end of the box.
    pub after_box: Vec<Node>,
    /// Extra root attributes (`data-nr-ui`, ...).
    pub root_attrs: Vec<Attr>,
    /// Extra end content (toggle buttons).
    pub end: Vec<Node>,
}

impl Field {
    /// The id of the control, generated once.
    pub fn control_id(&mut self) -> String {
        self.id.get_or_insert_with(|| next_id("nr-field")).clone()
    }

    pub fn is_invalid(&self) -> bool {
        self.invalid || self.error_message.is_some()
    }

    /// Attributes every control gets: id, name, required, disabled, aria.
    pub fn control_attrs(&mut self) -> Vec<Attr> {
        let id = self.control_id();
        let mut attrs = vec![attr("id", id.clone())];
        if let Some(name) = &self.name {
            attrs.push(attr("name", name.clone()));
        }
        attrs.push(flag("required", self.required));
        attrs.push(flag("disabled", self.disabled));
        let mut described = Vec::new();
        if self.description.is_some() {
            described.push(format!("{id}-description"));
        }
        if self.error_message.is_some() || self.name.is_some() {
            described.push(format!("{id}-error"));
        }
        if !described.is_empty() {
            attrs.push(attr("aria-describedby", described.join(" ")));
        }
        if self.is_invalid() {
            attrs.push(attr("aria-invalid", "true"));
        }
        attrs
    }

    fn label_node(&self, id: &str) -> Option<Node> {
        let label = self.label.as_ref()?;
        let mut el = Element::new("label")
            .with(part_class("nr-field-label", &self.label_class))
            .with(attr("for", id.to_owned()))
            .with(attr("id", format!("{id}-label")))
            .with(label.clone());
        if self.required {
            el = el.with(
                Element::new("span")
                    .with(attr("class", "nr-field-required"))
                    .with(attr("aria-hidden", "true"))
                    .with("*"),
            );
        }
        Some(el.into_node())
    }

    /// The whole field around `control`.
    pub fn render(mut self, control: Node, frame: Frame) -> Node {
        let id = self.control_id();
        let (class, attrs, _) = std::mem::take(&mut self.extra).split();
        let size = format!("nr-field-{}", self.size.suffix());
        let placement = match self.label_placement {
            LabelPlacement::Inside => "nr-label-inside",
            LabelPlacement::Outside => "nr-label-outside",
            LabelPlacement::OutsideLeft => "nr-label-left",
        };
        let has_label = self.label.is_some();
        let classes = [
            "nr-field",
            frame.kind,
            self.variant.class(),
            self.color.class(),
            &size,
            if has_label { placement } else { "nr-label-none" },
            self.radius.map(Radius::class).unwrap_or(""),
            if self.full_width.unwrap_or(true) { "nr-field-full" } else { "" },
        ];
        let mut el = root("div", &classes, self.unstyled, class);
        el.set_attr(flag("data-invalid", self.is_invalid()));
        el.set_attr(flag("data-disabled", self.disabled));
        el.set_attr(flag("data-float", frame.floats && has_label && self.label_placement == LabelPlacement::Inside));
        apply(&mut el, frame.root_attrs);
        apply(&mut el, attrs);

        let inside = self.label_placement == LabelPlacement::Inside;
        if !inside {
            el.children.extend(self.label_node(&id));
        }
        let mut wrap = Element::new("div").with(part_class("nr-field-wrap", &self.wrapper_class));
        if let Some(start) = self.start_content.take() {
            wrap.children.push(Element::new("span").with(attr("class", "nr-field-start")).with(start).into_node());
        }
        let mut inner = Element::new("div").with(attr("class", "nr-field-inner"));
        if inside {
            inner.children.extend(self.label_node(&id));
        }
        inner.children.push(control);
        wrap.children.push(inner.into_node());
        let mut end: Vec<Node> = self.end_content.take().into_iter().collect();
        end.extend(frame.end);
        if !end.is_empty() {
            wrap.children.push(Element::new("span").with(attr("class", "nr-field-end")).with(end).into_node());
        }
        // Popovers are positioned against the box.
        wrap.children.extend(frame.after_box);
        let mut main = Element::new("div").with(attr("class", "nr-field-main"));
        main.children.push(wrap.into_node());
        if let Some(description) = &self.description {
            main.children.push(
                Element::new("p")
                    .with(part_class("nr-field-description", &self.description_class))
                    .with(attr("id", format!("{id}-description")))
                    .with(description.clone())
                    .into_node(),
            );
        }
        if self.error_message.is_some() || self.name.is_some() {
            // Kept even when empty: the client runtime writes server-action
            // validation errors for `name` into it.
            let mut err = Element::new("p")
                .with(part_class("nr-field-error", &self.error_class))
                .with(attr("id", format!("{id}-error")))
                .with(attr("aria-live", "polite"));
            if let Some(name) = &self.name {
                err = err.with(attr("data-nr-error", name.clone()));
            }
            main.children.push(err.with(self.error_message.clone()).into_node());
        }
        el.children.push(main.into_node());
        styled(el)
    }
}

/// A text input.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(Input![label = "Email", name = "email", kind = "email", required = true]);
/// assert!(html.contains(r#"<input class="nr-field-input" type="email" placeholder=" " id="#));
/// assert!(html.contains(r#"data-nr-error="email""#));
/// ```
#[derive(Default)]
pub struct Input {
    field: Field,
    kind: Option<String>,
    clearable: bool,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();

    /// Add an attribute to the `<input>` (`attr("autocomplete", "email")`,
    /// `aria(..)`, ...); `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.field.extra.push(part);
        self
    }
    /// The input `type`: `"email"`, `"number"`, `"search"`, ... (default `"text"`).
    pub fn kind(mut self, v: impl Into<String>) -> Self {
        self.kind = Some(v.into());
        self
    }
    /// A button that clears the value.
    pub fn clearable(mut self, v: bool) -> Self {
        self.clearable = v;
        self
    }

    fn render(mut self, password: bool) -> Node {
        let kind = if password { "password".to_owned() } else { self.kind.take().unwrap_or_else(|| "text".into()) };
        let (class, attrs, _) = std::mem::take(&mut self.field.extra).split();
        if let Some(class) = class {
            self.field.extra.push(class);
        }
        let floats = self.field.placeholder.is_none();
        let mut input = Element::new_void("input")
            .with(part_class("nr-field-input", &self.field.input_class))
            .with(attr("type", kind))
            .with(attr("placeholder", self.field.placeholder.clone().unwrap_or_else(|| " ".into())));
        apply(&mut input, self.field.control_attrs());
        if let Some(value) = &self.field.value {
            input.set_attr(attr("value", value.clone()));
        }
        input.set_attr(flag("readonly", self.field.readonly));
        if password && !attrs.iter().any(|a| a.name == "autocomplete") {
            input.set_attr(attr("autocomplete", "current-password"));
        }
        apply(&mut input, attrs);

        let mut end = Vec::new();
        let mut root_attrs = Vec::new();
        if self.clearable {
            root_attrs.push(attr("data-nr-ui", "field"));
            end.push(icon_button("nr-field-clear", "Clear", "clear", icon(icons::close())));
        }
        if password {
            root_attrs.push(attr("data-nr-ui", "field"));
            end.push(
                Element::new("button")
                    .with(attr("type", "button"))
                    .with(attr("class", "nr-field-action nr-password-toggle"))
                    .with(attr("data-nr-field-action", "toggle-password"))
                    .with(attr("aria-label", "Show password"))
                    .with(attr("aria-pressed", "false"))
                    .with(Element::new("span").with(attr("class", "nr-password-show")).with(icon(icons::eye())))
                    .with(Element::new("span").with(attr("class", "nr-password-hide")).with(icon(icons::eye_off())))
                    .into_node(),
            );
        }
        let kind = if password { "nr-input nr-password" } else { "nr-input" };
        self.field.render(input.into_node(), Frame { kind, floats, after_box: Vec::new(), root_attrs, end })
    }
}

/// A small button inside a field (clear, toggle, open).
pub(crate) fn icon_button(class: &str, label: &str, action: &str, content: Node) -> Node {
    Element::new("button")
        .with(attr("type", "button"))
        .with(attr("class", format!("nr-field-action {class}")))
        .with(attr("data-nr-field-action", action.to_owned()))
        .with(attr("aria-label", label.to_owned()))
        .with(attr("tabindex", "-1"))
        .with(content)
        .into_node()
}

impl View for Input {
    fn into_node(self) -> Node {
        self.render(false)
    }
}

/// A password input with a button that shows and hides the password.
#[derive(Default)]
pub struct PasswordInput {
    input: Input,
}

impl PasswordInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an attribute to the `<input>`; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.input = self.input.with(part);
        self
    }
}

/// `PasswordInput` has the same properties as `Input`.
macro_rules! delegate_field {
    ($($name:ident($ty:ty)),* $(,)?) => {
        impl PasswordInput {$(
            #[doc = concat!("See [`Input::", stringify!($name), "`].")]
            pub fn $name(mut self, v: $ty) -> Self {
                self.input = self.input.$name(v);
                self
            }
        )*}
    };
}
delegate_field!(
    label(&str),
    placeholder(&str),
    name(&str),
    id(&str),
    required(bool),
    disabled(bool),
    readonly(bool),
    color(Color),
    size(Size),
    variant(FieldVariant),
    radius(Radius),
    label_placement(LabelPlacement),
    full_width(bool),
    label_class(&str),
    wrapper_class(&str),
    input_class(&str),
    description_class(&str),
    error_class(&str),
    class(&str),
    unstyled(bool),
);

impl PasswordInput {
    pub fn description(mut self, v: impl MaybeText) -> Self {
        self.input = self.input.description(v);
        self
    }
    pub fn error_message(mut self, v: impl MaybeText) -> Self {
        self.input = self.input.error_message(v);
        self
    }
    pub fn invalid(mut self, v: bool) -> Self {
        self.input = self.input.invalid(v);
        self
    }
    pub fn value(mut self, v: impl MaybeText) -> Self {
        self.input = self.input.value(v);
        self
    }
    pub fn start_content(mut self, v: impl View) -> Self {
        self.input = self.input.start_content(v);
        self
    }
    pub fn end_content(mut self, v: impl View) -> Self {
        self.input = self.input.end_content(v);
        self
    }
    /// `autocomplete="new-password"` for sign-up forms.
    pub fn new_password(self, v: bool) -> Self {
        if v { self.with(attr("autocomplete", "new-password")) } else { self }
    }
}

impl View for PasswordInput {
    fn into_node(self) -> Node {
        self.input.render(true)
    }
}

/// A multi-line text input that grows with its content.
#[derive(Default)]
pub struct Textarea {
    field: Field,
    rows: Option<u32>,
}

impl Textarea {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();

    /// Add an attribute to the `<textarea>`; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.field.extra.push(part);
        self
    }
    /// Minimum number of rows (default 3).
    pub fn rows(mut self, v: u32) -> Self {
        self.rows = Some(v);
        self
    }
}

impl View for Textarea {
    fn into_node(mut self) -> Node {
        let (class, attrs, _) = std::mem::take(&mut self.field.extra).split();
        if let Some(class) = class {
            self.field.extra.push(class);
        }
        let floats = self.field.placeholder.is_none();
        let mut area = Element::new("textarea")
            .with(part_class("nr-field-input nr-textarea-input", &self.field.input_class))
            .with(attr("rows", self.rows.unwrap_or(3).to_string()))
            .with(attr("placeholder", self.field.placeholder.clone().unwrap_or_else(|| " ".into())));
        apply(&mut area, self.field.control_attrs());
        area.set_attr(flag("readonly", self.field.readonly));
        apply(&mut area, attrs);
        area.children.extend(self.field.value.clone().map(View::into_node));
        self.field.render(
            area.into_node(),
            Frame { kind: "nr-textarea", floats, after_box: Vec::new(), root_attrs: Vec::new(), end: Vec::new() },
        )
    }
}
