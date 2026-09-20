//! Data display: the date picker's calendar, tab panels, a table's
//! select-all box, the code block's copy button and the avatar's fallback.

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use next_rust_wasm::event::ListenOpts;
use next_rust_wasm::{
    El, Event, Feature, NativeEvent, Str, by_id, html, intl, on_document, set_timeout, supports, window,
};

use crate::pop;
use crate::util::{CHEVRON_DOWN, CHEVRON_LEFT, CHEVRON_RIGHT, Date, svg};

// ---------------------------------------------------------------- calendar

/// How long the copy button says "Copied".
const COPIED_MS: u32 = 1_500;

/// Cells a month grid always has: six weeks, so the calendar does not change
/// height as the months go by.
const CELLS: i32 = 42;

// The five `Intl.DateTimeFormat` option sets the kit needs, as compile-time
// literals — no locale data is ever linked into the module, and the options
// never come from anywhere but here.
//
// Every one of them names `"timeZone":"UTC"`, because a calendar date has no
// instant until a zone is chosen and [`format_date`] pins it to UTC midnight.
// The two have to agree, or a date would be formatted as the day before it.

/// The readable date in the visible field.
const MEDIUM: &str = r#"{"dateStyle":"medium","timeZone":"UTC"}"#;

/// A day cell's `aria-label`.
const FULL: &str = r#"{"dateStyle":"full","timeZone":"UTC"}"#;

/// The calendar's header in the day view.
const MONTH_YEAR: &str = r#"{"month":"long","year":"numeric","timeZone":"UTC"}"#;

/// A cell of the twelve-month grid.
const MONTH_SHORT: &str = r#"{"month":"short","timeZone":"UTC"}"#;

/// A column heading of the day grid.
const WEEKDAY_SHORT: &str = r#"{"weekday":"short","timeZone":"UTC"}"#;

/// `setAttribute(name, on)` with the stringified boolean ARIA wants.
fn aria(el: El, name: &str, on: bool) {
    el.set_attr(name, if on { "true" } else { "false" });
}

/// The `input` then `change` pair a control fires when the kit, rather than
/// the person, changed its value.
fn changed(el: El) {
    el.dispatch_native(NativeEvent::Input, true);
    el.dispatch_native(NativeEvent::Change, true);
}

/// Today, local, as the browser reckons it.
///
/// The host counts months from one, the way `Date` does in the markup;
/// [`Date`] counts from zero, the way `Date.getMonth()` does.
fn today() -> Date {
    let (year, month, day) = next_rust_wasm::global::today();
    Date::new(year, month as i32 - 1, day as i32)
}

/// `Intl.DateTimeFormat(locale, opts).format(date)`.
///
/// Month and weekday names, and the readable date in the field, are the
/// host's job: shipping locale data in the module would cost more than the
/// whole kit. An empty locale is the document's, which is what the host reads
/// an unstaged locale as.
///
/// The date is pinned to UTC midnight so that a calendar date — which has no
/// instant of its own — cannot drift a day either way; every options literal
/// above names `"timeZone":"UTC"` to match.
fn format_date(locale: &str, date: Date, opts: &str) -> String {
    intl::date_in(locale, f64::from(date.ordinal()) * 86_400_000.0, opts).map(Str::into_string).unwrap_or_default()
}

/// Everything one date picker needs, so its handlers can call each other
/// without a knot of closures each holding the next.
struct Picker {
    /// The component root.
    root: El,
    /// The visible, human-readable field.
    input: El,
    /// The calendar panel.
    cal: El,
    /// The field's box, which the calendar is anchored to.
    wrap: El,
    /// The field that actually submits, as `YYYY-MM-DD`.
    hidden: El,
    /// The locale the dates are written in; empty means the document's.
    locale: String,
    /// `0` for a week starting on Sunday, `1` on Monday.
    first_day: i32,
    /// The range the field accepts.
    min: Option<Date>,
    /// The range the field accepts.
    max: Option<Date>,
    /// What is chosen, which view is showing and where the keyboard is.
    state: RefCell<PickerState>,
}

/// A picker's mutable state.
struct PickerState {
    /// The chosen date, if any.
    value: Option<Date>,
    /// Whether the twelve-month view is showing rather than the days.
    months: bool,
    /// The date the keyboard is on, which is also the month on show.
    focus: Date,
}

impl Picker {
    /// Whether a date is inside the field's range.
    fn allowed(&self, date: Date) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    /// Write the chosen date into both fields: readable in the visible one,
    /// `YYYY-MM-DD` in the one that submits.
    fn write(&self) {
        let value = self.state.borrow().value;
        match value {
            Some(date) => {
                self.input.set_value(&format_date(&self.locale, date, MEDIUM));
                self.hidden.set_value(&date.iso());
            }
            None => {
                self.input.set_value("");
                self.hidden.set_value("");
            }
        }
    }

    /// A calendar button.
    fn button(&self, class: &str, html: &str) -> El {
        let el = El::create("button");
        el.set_attr("type", "button");
        el.set_prop_str("className", class);
        el.set_html_trusted(html);
        el
    }

    /// Draw the calendar, and optionally move focus into it.
    fn render(&self, move_focus: bool) {
        let (value, months, focus) = {
            let state = self.state.borrow();
            (state.value, state.months, state.focus)
        };
        self.cal.set_text("");
        self.cal.set_attr("data-view", if months { "months" } else { "days" });

        let header = El::create("div");
        header.set_prop_str("className", "nr-cal-header");
        let in_days = !months;
        // In the month view the arrows step a year, and the edge that decides
        // whether they are usable is the far end of that year.
        let prev_last =
            if in_days { focus.month_start().add_months(-1).month_end() } else { Date::new(focus.year - 1, 11, 31) };
        let next_first = if in_days { focus.month_start().add_months(1) } else { Date::new(focus.year + 1, 0, 1) };

        let prev = self.button("nr-cal-nav", &svg(CHEVRON_LEFT));
        prev.set_attr("data-cal", "prev");
        prev.set_attr("aria-label", if in_days { "Previous month" } else { "Previous year" });
        prev.set_prop_bool("disabled", self.min.is_some_and(|min| prev_last < min));

        let title_text = if in_days { format_date(&self.locale, focus, MONTH_YEAR) } else { focus.year.to_string() };
        let mut title_html = String::from("<span>");
        // The title is the only place a value from the host is put into
        // markup, so it is escaped rather than trusted.
        escape_into(&title_text, &mut title_html);
        title_html.push_str("</span>");
        title_html.push_str(&svg(CHEVRON_DOWN));
        let title = self.button("nr-cal-title", &title_html);
        title.set_attr("data-cal", "view");
        title.set_attr("aria-live", "polite");

        let next = self.button("nr-cal-nav", &svg(CHEVRON_RIGHT));
        next.set_attr("data-cal", "next");
        next.set_attr("aria-label", if in_days { "Next month" } else { "Next year" });
        next.set_prop_bool("disabled", self.max.is_some_and(|max| next_first > max));

        header.append(prev);
        header.append(title);
        header.append(next);
        self.cal.append(header);

        let today = today();
        if in_days {
            let grid = El::create("div");
            grid.set_prop_str("className", "nr-cal-grid");
            grid.set_attr("role", "grid");
            for i in 0..7 {
                let cell = El::create("div");
                cell.set_prop_str("className", "nr-cal-weekday");
                cell.set_attr("role", "columnheader");
                cell.set_text(&format_date(&self.locale, weekday_sample(i, self.first_day), WEEKDAY_SHORT));
                grid.append(cell);
            }
            let start = focus.month_start();
            let lead = start.offset_in_week(self.first_day);
            for i in 0..CELLS {
                let date = start.add_days(i - lead);
                let day = self.button("nr-cal-day", "");
                day.set_text(&date.day.to_string());
                day.set_attr("role", "gridcell");
                day.set_attr("data-date", &date.iso());
                day.set_attr("aria-label", &format_date(&self.locale, date, FULL));
                aria(day, "aria-selected", value == Some(date));
                day.toggle_attr("data-today", date == today);
                day.toggle_attr("data-outside", date.month != focus.month);
                day.set_attr("tabindex", if date == focus { "0" } else { "-1" });
                day.set_prop_bool("disabled", !self.allowed(date));
                grid.append(day);
            }
            self.cal.append(grid);
        } else {
            let list = El::create("div");
            list.set_prop_str("className", "nr-cal-months");
            for month in 0..12 {
                let first = Date::new(focus.year, month, 1);
                let last = first.month_end();
                let cell = self.button("nr-cal-month", "");
                cell.set_text(&format_date(&self.locale, first, MONTH_SHORT));
                cell.set_attr("data-month", &month.to_string());
                aria(cell, "aria-selected", value.is_some_and(|v| v.year == focus.year && v.month == month));
                cell.toggle_attr("data-current", month == focus.month);
                cell.set_attr("tabindex", if month == focus.month { "0" } else { "-1" });
                cell.set_prop_bool(
                    "disabled",
                    self.min.is_some_and(|min| last < min) || self.max.is_some_and(|max| first > max),
                );
                list.append(cell);
            }
            self.cal.append(list);
        }

        let footer = El::create("div");
        footer.set_prop_str("className", "nr-cal-footer");
        let today_link = self.button("nr-cal-link", "");
        today_link.set_text("Today");
        today_link.set_attr("data-cal", "today");
        today_link.set_prop_bool("disabled", !self.allowed(today));
        let clear = self.button("nr-cal-link", "");
        clear.set_text("Clear");
        clear.set_attr("data-cal", "clear");
        footer.append(today_link);
        footer.append(clear);
        self.cal.append(footer);

        // The two views differ in height, so an open calendar has to be
        // placed again or it would hang off the field.
        if !self.cal.prop_bool("hidden") {
            pop::position(self.cal, self.wrap, false);
        }
        if move_focus {
            self.focus_cell();
        }
    }

    /// Put focus on the one cell the roving `tabindex` points at.
    fn focus_cell(&self) {
        let cell = self.cal.query("[tabindex=\"0\"]:not(:disabled)");
        if cell.is_some() {
            cell.focus();
        }
    }

    /// Open the calendar.
    fn show(&self) {
        if !self.cal.prop_bool("hidden") || self.input.prop_bool("disabled") || self.input.prop_bool("readOnly") {
            return;
        }
        pop::close_all(self.root);
        let today = today();
        {
            let mut state = self.state.borrow_mut();
            state.months = false;
            // Opening on a month the field would refuse every day of is a
            // dead end, so an out-of-range today falls back to the range.
            state.focus =
                state.value.unwrap_or(if self.allowed(today) { today } else { self.min.or(self.max).unwrap_or(today) });
        }
        self.render(false);
        self.root.set_attr("data-open", "");
        pop::show(self.cal, self.root, pop::Anchor::Wrap);
        let toggle = self.root.query(".nr-datepicker-toggle");
        if toggle.is_some() {
            aria(toggle, "aria-expanded", true);
        }
        self.focus_cell();
    }

    /// Close the calendar, optionally putting focus back in the field.
    fn hide(&self, return_focus: bool) {
        if self.cal.prop_bool("hidden") {
            return;
        }
        pop::hide(self.cal);
        self.root.remove_attr("data-open");
        let toggle = self.root.query(".nr-datepicker-toggle");
        if toggle.is_some() {
            aria(toggle, "aria-expanded", false);
        }
        if return_focus {
            self.input.focus();
        }
    }

    /// Choose a date and close.
    fn select(&self, date: Option<Date>) {
        self.state.borrow_mut().value = date;
        self.write();
        changed(self.hidden);
        self.hide(true);
    }

    /// Move the keyboard to a date, clamped into the field's range.
    fn move_to(&self, date: Date) {
        let date = clamp(date, self.min, self.max);
        self.state.borrow_mut().focus = date;
        self.render(true);
    }
}

/// The date picker's calendar.
///
/// The field underneath stays a real date input until the calendar exists:
/// without the module it is `<input type=date>` with the browser's own
/// picker, which is why the type is changed and the `name` moved only after
/// the hidden field is in the DOM.
pub fn date_picker(root: El) {
    let (input, cal, wrap) =
        (root.query(".nr-datepicker-input"), root.query(".nr-calendar"), root.query(".nr-field-wrap"));
    if input.is_null() || cal.is_null() || wrap.is_null() {
        return;
    }

    let hidden = El::create("input");
    hidden.set_attr("type", "hidden");
    hidden.set_attr("name", input.prop_str("name").as_deref().unwrap_or_default());
    input.remove_attr("name");
    // `input.after(hidden)`: `nextSibling`, not `nextElementSibling`, so the
    // hidden field lands exactly where the script put it. A null one appends.
    input.parent().insert_before(hidden, input.val().get("nextSibling").el());

    let locale = root
        .attr("data-locale")
        .or_else(|| html().attr("lang"))
        .filter(|l| !l.is_empty())
        .map(Str::into_string)
        .unwrap_or_default();
    let first_day = root.attr("data-first-day").and_then(|d| d.as_str().parse().ok()).unwrap_or(0);
    let value = Date::parse(&input.value());
    let today = today();

    let picker = Rc::new(Picker {
        root,
        input,
        cal,
        wrap,
        hidden,
        locale,
        first_day,
        min: input.attr("min").as_deref().and_then(Date::parse),
        max: input.attr("max").as_deref().and_then(Date::parse),
        state: RefCell::new(PickerState { value, months: false, focus: value.unwrap_or(today) }),
    });

    // The field becomes a readable, read-only face for the hidden one.
    input.set_prop_str("type", "text");
    input.set_prop_str("autocomplete", "off");
    input.set_attr("inputmode", "none");
    input.set_attr("aria-haspopup", "dialog");
    input.on("beforeinput", |ev: Event| ev.prevent_default());
    picker.write();

    // How the outside-press listener and `pop::close_all` reach this picker,
    // in place of the popover registry the script kept. A close from there
    // never returns focus: the press already moved it somewhere deliberate.
    root.on(pop::CLOSE, {
        let picker = picker.clone();
        move |_| picker.hide(false)
    });

    cal.on("click", {
        let picker = picker.clone();
        move |ev: Event| {
            let button = ev.closest("button");
            if button.is_null() || button.prop_bool("disabled") {
                return;
            }
            if let Some(date) = button.attr("data-date") {
                picker.select(Date::parse(&date));
            } else if let Some(month) = button.attr("data-month").and_then(|m| m.as_str().parse::<i32>().ok()) {
                {
                    let mut state = picker.state.borrow_mut();
                    // The 28th always exists, so choosing a month never
                    // lands on a day the month does not have.
                    let focus = state.focus;
                    state.focus = Date::new(focus.year, month, focus.day.min(28));
                    state.months = false;
                }
                picker.render(true);
            } else {
                match button.attr("data-cal").as_deref() {
                    Some("prev") | Some("next") => {
                        let by = if button.attr("data-cal").as_deref() == Some("prev") { -1 } else { 1 };
                        {
                            let mut state = picker.state.borrow_mut();
                            state.focus = if state.months {
                                Date::new(state.focus.year + by, state.focus.month, 1)
                            } else {
                                state.focus.add_months(by)
                            };
                        }
                        picker.render(false);
                    }
                    Some("view") => {
                        {
                            let mut state = picker.state.borrow_mut();
                            state.months = !state.months;
                        }
                        picker.render(true);
                    }
                    Some("today") => picker.select(Some(self::today())),
                    Some("clear") => picker.select(None),
                    _ => {}
                }
            }
        }
    });

    cal.on("keydown", {
        let picker = picker.clone();
        move |ev: Event| {
            if ev.key_is("Escape") {
                ev.prevent_default();
                picker.hide(true);
                return;
            }
            let target = ev.target();
            let on_day = target.has_attr("data-date");
            let on_month = target.has_attr("data-month");
            if !on_day && !on_month {
                return;
            }
            let focus = picker.state.borrow().focus;
            // A week is a row of days; a row of months is three.
            if let Some(step) = arrow_step(&ev.key(), on_day) {
                ev.prevent_default();
                if on_day {
                    picker.move_to(focus.add_days(step));
                } else {
                    let month = (focus.month + step).clamp(0, 11);
                    picker.state.borrow_mut().focus = Date::new(focus.year, month, 1);
                    picker.render(true);
                }
            } else if on_day && (ev.key_is("PageUp") || ev.key_is("PageDown")) {
                ev.prevent_default();
                // Shift turns a page of a month into a page of a year.
                let by = if ev.key_is("PageUp") { -1 } else { 1 };
                let span = if ev.shift() { 12 } else { 1 };
                picker.move_to(focus.add_months(by * span));
            } else if on_day && (ev.key_is("Home") || ev.key_is("End")) {
                ev.prevent_default();
                let offset = focus.offset_in_week(picker.first_day);
                picker.move_to(focus.add_days(if ev.key_is("Home") { -offset } else { 6 - offset }));
            }
        }
    });

    // Tabbing out of the picker closes it; clicking away is the outside-press
    // listener's job, and a `focusout` with no new target is not a leave.
    root.on("focusout", {
        let picker = picker.clone();
        move |ev: Event| {
            let to = ev.related_target();
            if !picker.cal.prop_bool("hidden") && to.is_some() && !picker.root.contains(to) {
                picker.hide(false);
            }
        }
    });

    input.on("click", {
        let picker = picker.clone();
        move |_| picker.show()
    });
    input.on("keydown", {
        let picker = picker.clone();
        move |ev: Event| {
            if ev.key_is("Enter") || ev.key_is(" ") || ev.key_is("ArrowDown") {
                ev.prevent_default();
                picker.show();
            } else if (ev.key_is("Backspace") || ev.key_is("Delete"))
                && picker.state.borrow().value.is_some()
                && !picker.input.prop_bool("required")
            {
                // Clearing from the keyboard, without opening anything.
                ev.prevent_default();
                picker.state.borrow_mut().value = None;
                picker.write();
                changed(picker.hidden);
            }
        }
    });
    wrap.on("click", {
        let picker = picker.clone();
        move |ev: Event| {
            if ev.closest(".nr-calendar").is_some() {
                return;
            }
            if ev.closest(".nr-datepicker-toggle").is_some() && !picker.cal.prop_bool("hidden") {
                picker.hide(true);
            } else {
                picker.show();
            }
        }
    });
    let form = input.form();
    if form.is_some() {
        form.on("reset", move |_| {
            let picker = picker.clone();
            // The control's value comes back after this event.
            set_timeout(0, move |_| {
                picker.state.borrow_mut().value =
                    picker.input.prop_str("defaultValue").as_deref().and_then(Date::parse);
                picker.write();
            });
        });
    }
}

/// The date whose weekday name heads column `column`, for a week starting on
/// `first_day`. 1 January 2023 was a Sunday, so it anchors the cycle.
fn weekday_sample(column: i32, first_day: i32) -> Date {
    Date::new(2023, 0, 1 + (column + first_day).rem_euclid(7))
}

/// How far an arrow key moves: a week in the day grid, a row of three in the
/// month grid.
fn arrow_step(key: &str, on_day: bool) -> Option<i32> {
    match key {
        "ArrowLeft" => Some(-1),
        "ArrowRight" => Some(1),
        "ArrowUp" => Some(if on_day { -7 } else { -3 }),
        "ArrowDown" => Some(if on_day { 7 } else { 3 }),
        _ => None,
    }
}

/// Hold a date inside the field's range.
fn clamp(date: Date, min: Option<Date>, max: Option<Date>) -> Date {
    let date = match min {
        Some(min) if date < min => min,
        _ => date,
    };
    match max {
        Some(max) if date > max => max,
        _ => date,
    }
}

/// Escape text for the one place the calendar puts a host string into
/// markup: the title, which `Intl` produced and the module did not.
fn escape_into(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

// ---------------------------------------------------------------- tabs

/// Tab panels.
///
/// With no module the panels are all visible, one after another, under their
/// headings — a page that reads correctly rather than one that is broken.
pub fn tabs(root: El) {
    let list = root.query("[role=\"tablist\"]");
    if list.is_null() {
        return;
    }

    let select = move |tab: El, focus: bool| {
        if !tab.is_some() || tab.prop_bool("disabled") {
            return;
        }
        for other in list.query_all("[role=\"tab\"]").iter() {
            let on = other.val().same(tab.val());
            other.class_toggle("nr-tab-selected", on);
            aria(other, "aria-selected", on);
            other.set_prop_num("tabIndex", if on { 0.0 } else { -1.0 });
            let panel = other.attr("aria-controls").map_or(El::NULL, |id| by_id(&id));
            if panel.is_some() {
                panel.set_prop_bool("hidden", !on);
            }
        }
        if focus {
            tab.focus();
        }
        let mut detail = String::from("{\"key\":\"");
        detail.push_str(&tab.attr("data-key").as_deref().unwrap_or_default().replace('"', "\\\""));
        detail.push_str("\"}");
        root.dispatch("nr:change", &detail);
    };

    // `select` captures nothing but element handles, so it is `Copy`: both
    // listeners get their own without a clone.
    list.on("click", move |ev: Event| {
        let tab = ev.closest("[role=\"tab\"]");
        if tab.is_some() && list.contains(tab) {
            select(tab, false);
        }
    });
    list.on("keydown", move |ev: Event| {
        let all: Vec<El> = list.query_all("[role=\"tab\"]").iter().filter(|tab| !tab.prop_bool("disabled")).collect();
        let tab = ev.closest("[role=\"tab\"]");
        if tab.is_null() {
            return;
        }
        let Some(here) = all.iter().position(|t| t.val().same(tab.val())) else { return };
        let Some(to) = tab_target(&all, here, &ev.key()) else { return };
        ev.prevent_default();
        // Moving focus selects: a tab list follows focus, which is what
        // `aria-activedescendant`-free roving tab panels do.
        select(all[to], true);
    });
}

/// Which tab a key press moves to, wrapped.
fn tab_target<T>(items: &[T], here: usize, key: &str) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    let len = items.len() as i32;
    let to = match key {
        "ArrowRight" => here as i32 + 1,
        "ArrowLeft" => here as i32 - 1,
        "Home" => 0,
        "End" => len - 1,
        _ => return None,
    };
    Some(to.rem_euclid(len) as usize)
}

// ---------------------------------------------------------------- table

/// A table's select-all checkbox.
pub fn check_all(all: El) {
    let table = all.closest("table");
    if table.is_null() {
        return;
    }

    let sync = move || {
        let boxes = table.query_all("tbody .nr-table-checkbox");
        let total = boxes.len();
        let checked = boxes.iter().filter(|box_| box_.prop_bool("checked")).count() as u32;
        all.set_prop_bool("checked", checked > 0 && checked == total);
        // Some but not all is the third state a checkbox has and HTML has no
        // attribute for, so it can only be set from here.
        all.set_prop_bool("indeterminate", checked > 0 && checked < total);
    };

    table.on("change", move |ev: Event| {
        let target = ev.target();
        if target.val().same(all.val()) {
            for box_ in table.query_all("tbody .nr-table-checkbox").iter() {
                box_.set_prop_bool("checked", all.prop_bool("checked"));
            }
        } else if !target.matches(".nr-table-checkbox") {
            return;
        }
        sync();
        let selected: Vec<String> = table
            .query_all("tbody .nr-table-checkbox")
            .iter()
            .filter(|box_| box_.prop_bool("checked"))
            .map(|box_| box_.value().into_string())
            .collect();
        table.dispatch("nr:change", &selected_json(&selected));
    });
    sync();
}

/// `{"selected":[…]}` for the `nr:change` detail.
fn selected_json(values: &[String]) -> String {
    let mut out = String::from("{\"selected\":[");
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        for c in value.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c if (c as u32) < 0x20 => {}
                c => out.push(c),
            }
        }
        out.push('"');
    }
    out.push_str("]}");
    out
}

// ---------------------------------------------------------------- avatar

/// An avatar's image fallback.
///
/// The initials are underneath the image in the markup, so a broken image
/// only has to be taken out of the way — there is nothing to build.
pub fn avatar(root: El) {
    let img = root.query(".nr-avatar-img");
    if img.is_null() {
        return;
    }
    // An image that already failed fires no `error`: it has to be caught
    // here, on the state it left behind — loaded, no pixels, and a `src` that
    // was actually asked for.
    if img.prop_bool("complete") && img.prop_num("naturalWidth") == Some(0.0) && img.has_attr("src") {
        img.set_prop_bool("hidden", true);
        return;
    }
    img.on_opts("error", ListenOpts::ONCE, move |_| {
        img.set_prop_bool("hidden", true);
    });
}

// ---------------------------------------------------------------- copy

/// The code block's copy button, on the document.
pub fn copy() {
    on_document("click", |ev: Event| {
        let button = ev.closest("[data-nr-copy]");
        // Checked before chaining rather than after: the shim's handle
        // allocator turns a null receiver into a handle holding `0` instead
        // of nothing, so `NULL.closest(..).query(..)` throws rather than
        // reading as absent. Reported; guarding here is better code anyway,
        // and it is what the script did (`btn && btn.closest(..)`).
        if button.is_null() {
            return;
        }
        let block = button.closest(".nr-codeblock");
        if block.is_null() {
            return;
        }
        let pre = block.query("pre");
        if pre.is_null() {
            return;
        }
        // A browser that has no clipboard at all must not claim to have
        // copied anything, which is half of what the script's `catch` did.
        //
        // TODO(abi): `next_rust_wasm::clipboard_write(text) -> impl Future<Output = bool>`,
        // over opcode 161 `CLIPBOARD_WRITE` — the shim implements it and resumes the
        // token with success or failure, but the crate wraps no such call yet. Until it
        // does, a browser that *refuses* the clipboard cannot be told from one that
        // accepted it, and the button says "Copied" either way.
        if !supports(Feature::Clipboard) {
            return;
        }
        window().get("navigator").get("clipboard").call_str("writeText", &pre.text());
        button.set_attr("data-copied", "");
        button.set_attr("aria-label", "Copied");
        button.dispatch("nr:copy", "null");
        set_timeout(COPIED_MS, move |_| {
            button.remove_attr("data-copied");
            button.set_attr("aria-label", "Copy code");
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;
    use std::vec;

    #[test]
    fn the_weekday_headings_start_on_the_right_day() {
        // 1 January 2023 was a Sunday.
        assert_eq!(weekday_sample(0, 0).weekday(), 0, "a week starting on Sunday");
        assert_eq!(weekday_sample(0, 1).weekday(), 1, "a week starting on Monday");
        assert_eq!(weekday_sample(6, 1).weekday(), 0, "…ends on Sunday");
        for first_day in 0..7 {
            let days: Vec<i32> = (0..7).map(|i| weekday_sample(i, first_day).weekday()).collect();
            let mut sorted = days.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), 7, "every weekday appears once for first_day {first_day}");
        }
    }

    #[test]
    fn arrows_move_a_week_in_days_and_a_row_in_months() {
        assert_eq!(arrow_step("ArrowUp", true), Some(-7));
        assert_eq!(arrow_step("ArrowDown", true), Some(7));
        assert_eq!(arrow_step("ArrowUp", false), Some(-3));
        assert_eq!(arrow_step("ArrowDown", false), Some(3));
        assert_eq!(arrow_step("ArrowLeft", true), Some(-1));
        assert_eq!(arrow_step("ArrowRight", false), Some(1));
        assert_eq!(arrow_step("Enter", true), None);
    }

    #[test]
    fn a_date_is_held_inside_the_fields_range() {
        let (min, max) = (Date::new(2026, 0, 10), Date::new(2026, 0, 20));
        assert_eq!(clamp(Date::new(2026, 0, 1), Some(min), Some(max)), min);
        assert_eq!(clamp(Date::new(2026, 0, 31), Some(min), Some(max)), max);
        assert_eq!(clamp(Date::new(2026, 0, 15), Some(min), Some(max)), Date::new(2026, 0, 15));
        assert_eq!(clamp(Date::new(1900, 0, 1), None, None), Date::new(1900, 0, 1));
        assert_eq!(clamp(Date::new(2026, 0, 1), None, Some(max)), Date::new(2026, 0, 1));
    }

    /// Six weeks always fit a month and its neighbours, whatever day it
    /// starts on — that is why the grid is a fixed 42 cells.
    #[test]
    fn six_weeks_always_cover_the_month() {
        for month in 0..12 {
            for first_day in [0, 1] {
                let start = Date::new(2026, month, 1);
                let lead = start.offset_in_week(first_day);
                let last = start.month_end();
                assert!(lead + last.day <= CELLS, "{month} starting on day {first_day} does not fit");
            }
        }
        // February 2026 starts on a Sunday: the tightest and the loosest fit.
        assert!(Date::new(2026, 1, 1).offset_in_week(1) + 28 <= CELLS);
    }

    #[test]
    fn tabs_wrap_at_both_ends() {
        let tabs = [0, 1, 2];
        assert_eq!(tab_target(&tabs, 0, "ArrowRight"), Some(1));
        assert_eq!(tab_target(&tabs, 2, "ArrowRight"), Some(0), "wraps to the first");
        assert_eq!(tab_target(&tabs, 0, "ArrowLeft"), Some(2), "wraps to the last");
        assert_eq!(tab_target(&tabs, 1, "Home"), Some(0));
        assert_eq!(tab_target(&tabs, 1, "End"), Some(2));
        assert_eq!(tab_target(&tabs, 1, "x"), None);
        assert_eq!(tab_target::<i32>(&[], 0, "Home"), None);
    }

    #[test]
    fn the_selection_detail_is_valid_json() {
        assert_eq!(selected_json(&[]), "{\"selected\":[]}");
        assert_eq!(selected_json(&["a".to_owned()]), "{\"selected\":[\"a\"]}");
        assert_eq!(selected_json(&["a\"b".to_owned()]), "{\"selected\":[\"a\\\"b\"]}");
    }

    #[test]
    fn the_calendar_title_is_escaped_before_it_becomes_markup() {
        let mut out = String::new();
        escape_into("a<b>&\"c\"", &mut out);
        assert_eq!(out, "a&lt;b&gt;&amp;&quot;c&quot;");
        let mut plain = String::new();
        escape_into("September 2026", &mut plain);
        assert_eq!(plain, "September 2026");
    }

    /// Every options literal the calendar hands `Intl` names UTC, because the
    /// date it hands over is pinned to UTC midnight. A missing `timeZone`
    /// would render the day before wherever the browser is west of it.
    #[test]
    fn every_date_format_is_pinned_to_the_same_zone_as_the_date() {
        for opts in [MEDIUM, FULL, MONTH_YEAR, MONTH_SHORT, WEEKDAY_SHORT] {
            assert!(opts.contains(r#""timeZone":"UTC""#), "{opts}");
        }
        // The pinning itself: a date is midnight UTC on its own ordinal.
        assert_eq!(f64::from(Date::new(1970, 0, 1).ordinal()) * 86_400_000.0, 0.0);
        assert_eq!(f64::from(Date::new(1970, 0, 2).ordinal()) * 86_400_000.0, 86_400_000.0);
    }

    /// Off the browser there is no host, so `Intl` answers nothing — and an
    /// empty answer must be an empty label rather than a panic.
    #[test]
    fn a_calendar_survives_a_host_with_no_intl() {
        assert_eq!(format_date("", Date::new(2026, 8, 20), MEDIUM), "");
        assert_eq!(format_date("en-GB", Date::new(2026, 8, 20), FULL), "");
        let _ = vec![MEDIUM];
    }
}
