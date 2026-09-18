//! A date picker with a custom calendar popover.

use next_rust_view::{Element, Node, View};

use crate::icons::{self, icon};
use crate::input::{Field, Frame, field_methods, icon_button};
use crate::{apply, attr, flag};

/// A date field with a calendar: month and year views, keyboard navigation,
/// minimum and maximum dates. The value is an ISO date (`2026-09-18`); it is
/// shown in the visitor's locale.
///
/// ```
/// # use next_rust_ui::*; use next_rust_view::*;
/// let html = render_static(DatePicker![label = "Birthday", name = "birthday", max = "2026-12-31"]);
/// assert!(html.contains(r#"type="date""#) && html.contains(r#"data-nr-ui="datepicker""#));
/// ```
#[derive(Default)]
pub struct DatePicker {
    field: Field,
    min: Option<String>,
    max: Option<String>,
    first_day: Option<u8>,
    locale: Option<String>,
}

impl DatePicker {
    pub fn new() -> Self {
        Self::default()
    }
    field_methods!();

    /// Add an attribute to the date input; `class(..)` goes to the outer element.
    pub fn with(mut self, part: impl next_rust_view::Part) -> Self {
        self.field.extra.push(part);
        self
    }
    /// Earliest date that can be chosen (`YYYY-MM-DD`).
    pub fn min(mut self, v: impl Into<String>) -> Self {
        self.min = Some(v.into());
        self
    }
    /// Latest date that can be chosen (`YYYY-MM-DD`).
    pub fn max(mut self, v: impl Into<String>) -> Self {
        self.max = Some(v.into());
        self
    }
    /// First day of the week: 0 = Sunday (default), 1 = Monday, ...
    pub fn first_day_of_week(mut self, v: u8) -> Self {
        self.first_day = Some(v % 7);
        self
    }
    /// Locale for month and day names (`"de-DE"`); the page language by default.
    pub fn locale(mut self, v: impl Into<String>) -> Self {
        self.locale = Some(v.into());
        self
    }
}

impl View for DatePicker {
    fn into_node(mut self) -> Node {
        let (class, attrs, _) = std::mem::take(&mut self.field.extra).split();
        if let Some(class) = class {
            self.field.extra.push(class);
        }
        let floats = self.field.placeholder.is_none();
        let mut input = Element::new_void("input")
            .with(attr(
                "class",
                match &self.field.input_class {
                    Some(c) => format!("nr-field-input nr-datepicker-input {c}"),
                    None => "nr-field-input nr-datepicker-input".to_owned(),
                },
            ))
            .with(attr("type", "date"))
            .with(attr("placeholder", self.field.placeholder.clone().unwrap_or_else(|| " ".into())));
        apply(&mut input, self.field.control_attrs());
        if let Some(v) = &self.field.value {
            input.set_attr(attr("value", v.clone()));
        }
        if let Some(v) = &self.min {
            input.set_attr(attr("min", v.clone()));
        }
        if let Some(v) = &self.max {
            input.set_attr(attr("max", v.clone()));
        }
        input.set_attr(flag("readonly", self.field.readonly));
        apply(&mut input, attrs);

        let mut root_attrs = vec![attr("data-nr-ui", "datepicker")];
        if let Some(d) = self.first_day {
            root_attrs.push(attr("data-first-day", d.to_string()));
        }
        if let Some(l) = &self.locale {
            root_attrs.push(attr("data-locale", l.clone()));
        }
        let open = icon_button("nr-datepicker-toggle", "Choose date", "open", icon(icons::calendar()));
        let calendar = Element::new("div")
            .with(attr("class", "nr-calendar"))
            .with(attr("role", "dialog"))
            .with(attr("aria-label", "Choose date"))
            .with(flag("hidden", true))
            .into_node();
        self.field.render(
            input.into_node(),
            Frame { kind: "nr-datepicker", floats, after_box: vec![calendar], root_attrs, end: vec![open] },
        )
    }
}
