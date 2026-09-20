//! Form controls: the custom select, the combobox, the tags input, and the
//! document-level field actions (password toggle, clear, steppers, slider).
//!
//! Every one of these has a working control underneath it. The select *is* a
//! `<select>`; the combobox *is* an `<input list>`; the tags field *is* a
//! text input that submits `a, b, c`. The module replaces what they look
//! like, never what they are, so a page with no script still submits the
//! same form — which is why the `name` is moved to a hidden input in the same
//! breath it is taken off the visible one, and never before.

use alloc::borrow::ToOwned;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use next_rust_wasm::{El, Event, NativeEvent, on_document, set_timeout};

use crate::pop;
use crate::typed;
use crate::util::percent;

/// How long a type-ahead buffer lives before the next key starts a new one.
const TYPE_AHEAD_MS: f64 = 700.0;

/// `setAttribute(name, on)` with the stringified boolean ARIA wants.
///
/// `aria-expanded` and `aria-selected` are attributes holding the *words*
/// `true` and `false`, not IDL booleans, so they never go through
/// `set_prop_bool`.
fn aria(el: El, name: &str, on: bool) {
    el.set_attr(name, if on { "true" } else { "false" });
}

/// The `input` then `change` pair a control fires when the kit, rather than
/// the person, changed its value — so forms and app code see it.
fn changed(el: El) {
    el.dispatch_native(NativeEvent::Input, true);
    el.dispatch_native(NativeEvent::Change, true);
}

// ---------------------------------------------------------------- select

/// The custom select over a native `<select>`.
pub fn select(root: El) {
    let (native, trigger, list, value_el) = (
        root.query(".nr-select-native"),
        root.query(".nr-select-trigger"),
        root.query(".nr-select-list"),
        root.query(".nr-select-value"),
    );
    if native.is_null() || trigger.is_null() || list.is_null() || value_el.is_null() {
        return;
    }
    let wrap = root.query(".nr-field-wrap");
    if wrap.is_null() {
        return;
    }

    let multiple = native.prop_bool("multiple");
    // The blank option is the placeholder: it is what the native control
    // shows when nothing is chosen, so the custom one shows it too.
    let placeholder = native
        .query_all("option")
        .iter()
        .find(|option| option.value().is_empty())
        .map(|option| option.text().into_string())
        .unwrap_or_default();

    let active: Rc<RefCell<El>> = Rc::new(RefCell::new(El::NULL));
    let ahead: Rc<RefCell<(String, f64)>> = Rc::new(RefCell::new((String::new(), 0.0)));

    let sync = move || {
        let chosen: Vec<String> = native
            .query_all("option")
            .iter()
            .filter(|option| option.prop_bool("selected"))
            .map(|option| option.value().into_string())
            .filter(|value| !value.is_empty())
            .collect();
        let mut labels: Vec<String> = Vec::new();
        for option in list.query_all("[role=\"option\"]").iter() {
            let on = option.attr("data-value").is_some_and(|value| chosen.iter().any(|c| value.eq_str(c)));
            aria(option, "aria-selected", on);
            let label = option.query(".nr-select-option-label");
            if on && label.is_some() {
                labels.push(label.text().into_string());
            }
        }
        let shown = if labels.is_empty() { placeholder.clone() } else { labels.join(", ") };
        value_el.set_text(&shown);
        value_el.toggle_attr("data-placeholder", labels.is_empty());
        root.toggle_attr("data-filled", !labels.is_empty());
    };

    let set_active = {
        let active = active.clone();
        move |option: El, scroll: bool| {
            let mut active = active.borrow_mut();
            if active.is_some() {
                active.remove_attr("data-active");
            }
            *active = option;
            if active.is_some() {
                active.set_attr("data-active", "");
                if let Some(id) = active.prop_str("id") {
                    trigger.set_attr("aria-activedescendant", &id);
                }
                if scroll {
                    active.scroll_into_view(false);
                }
            } else {
                trigger.remove_attr("aria-activedescendant");
            }
        }
    };

    // Closing is one local function every deliberate close calls. There is no
    // popover registry any more: `pop::close_all` and the outside-press
    // listener both reach a component by dispatching `pop::CLOSE` on its root,
    // which is what the listener below answers.
    let close = {
        let set_active = set_active.clone();
        move |focus: bool| {
            if list.prop_bool("hidden") {
                return;
            }
            pop::hide(list);
            root.remove_attr("data-open");
            aria(trigger, "aria-expanded", false);
            set_active(El::NULL, true);
            // Only a deliberate close — Escape, or choosing — returns focus
            // to the trigger; a click elsewhere must leave focus alone.
            if focus {
                trigger.focus();
            }
        }
    };
    root.on(pop::CLOSE, {
        let close = close.clone();
        move |_| close(false)
    });

    let open = {
        let set_active = set_active.clone();
        move || {
            if !list.prop_bool("hidden") || native.prop_bool("disabled") {
                return;
            }
            pop::close_all(root);
            root.set_attr("data-open", "");
            pop::show(list, root, pop::Anchor::Field);
            aria(trigger, "aria-expanded", true);
            let usable = usable_options(list);
            let start = usable
                .iter()
                .find(|option| option.attr("aria-selected").is_some_and(|v| v.eq_str("true")))
                .or(usable.first())
                .copied()
                .unwrap_or(El::NULL);
            set_active(start, true);
        }
    };

    let choose = {
        let (sync, close) = (sync.clone(), close.clone());
        move |option: El| {
            if !option.is_some() || option.attr("aria-disabled").is_some_and(|v| v.eq_str("true")) {
                return;
            }
            let Some(wanted) = option.attr("data-value") else { return };
            let Some(native_option) = native.query_all("option").iter().find(|o| o.value().eq_str(wanted.as_str()))
            else {
                return;
            };
            // A multiple select toggles; a single one always lands on the
            // option, even when it was already the chosen one.
            let on = if multiple { !native_option.prop_bool("selected") } else { true };
            native_option.set_prop_bool("selected", on);
            changed(native);
            sync();
            if !multiple {
                close(true);
            }
        }
    };

    trigger.on("click", {
        let (open, close) = (open.clone(), close.clone());
        move |_| {
            if list.prop_bool("hidden") {
                open();
            } else {
                close(false);
            }
        }
    });
    wrap.on("click", {
        let (open, close) = (open.clone(), close.clone());
        move |ev: Event| {
            // A click on the field's chrome — the label's gutter, the icon —
            // belongs to the trigger; a click on the list does not.
            if ev.closest(".nr-select-list, .nr-select-trigger").is_some() {
                return;
            }
            trigger.focus();
            if list.prop_bool("hidden") {
                open();
            } else {
                close(false);
            }
        }
    });
    // Pressing inside the list must not take focus off the trigger, or the
    // keyboard would stop working halfway through choosing.
    list.on("mousedown", |ev: Event| ev.prevent_default());
    list.on("pointermove", {
        let (active, set_active) = (active.clone(), set_active.clone());
        move |ev: Event| {
            let current = *active.borrow();
            let option = ev.closest("[role=\"option\"]");
            // Two queries for one node give two different handle numbers, so
            // sameness is a question for the browser rather than for `==`.
            if option.is_some()
                && !option.val().same(current.val())
                && !option.attr("aria-disabled").is_some_and(|v| v.eq_str("true"))
            {
                set_active(option, false);
            }
        }
    });
    list.on("click", {
        let choose = choose.clone();
        move |ev: Event| {
            ev.stop_propagation();
            let option = ev.closest("[role=\"option\"]");
            if option.is_some() {
                choose(option);
            }
        }
    });
    // Space activates a button on key *up*, which would reopen the list the
    // key-down just closed.
    trigger.on("keyup", |ev: Event| {
        if ev.key_is(" ") {
            ev.prevent_default();
        }
    });
    trigger.on("keydown", {
        let (active, set_active, open, choose, close) =
            (active.clone(), set_active.clone(), open.clone(), choose.clone(), close.clone());
        move |ev: Event| {
            let items = usable_options(list);
            let current = *active.borrow();
            let here = items.iter().position(|option| option.val().same(current.val()));
            let hidden = list.prop_bool("hidden");
            let key = ev.key();
            match key.as_str() {
                "ArrowDown" => {
                    ev.prevent_default();
                    if hidden {
                        open();
                    } else if let Some(to) = step_within(&items, here, 1) {
                        set_active(to, true);
                    }
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    if hidden {
                        open();
                    } else if let Some(to) = step_within(&items, here, -1) {
                        set_active(to, true);
                    }
                }
                "Home" | "End" => {
                    if !hidden {
                        ev.prevent_default();
                        let to = if key.eq_str("Home") { items.first() } else { items.last() };
                        set_active(to.copied().unwrap_or(El::NULL), true);
                    }
                }
                "Enter" | " " => {
                    ev.prevent_default();
                    if hidden {
                        open();
                    } else {
                        // `current`, not a fresh borrow: choosing closes, and
                        // closing takes `active` mutably.
                        choose(current);
                    }
                }
                "Escape" => {
                    if !hidden {
                        ev.prevent_default();
                        close(true);
                    }
                }
                "Tab" => close(false),
                _ => {
                    let Some(key) = typed(ev) else { return };
                    let query = {
                        let mut ahead = ahead.borrow_mut();
                        *ahead = extend_type_ahead(&ahead.0, ahead.1, &key, next_rust_wasm::date_now());
                        ahead.0.to_lowercase()
                    };
                    if let Some(found) = items.iter().find(|option| starts_with_query(&option.text(), &query)) {
                        if hidden {
                            open();
                        }
                        set_active(*found, true);
                    }
                }
            }
        }
    });
    let label = root.query(".nr-field-label");
    if label.is_some() {
        // The label is `for` the native select, which is hidden: send the
        // click to the thing the person can actually see.
        label.on("click", move |ev: Event| {
            ev.prevent_default();
            trigger.focus();
        });
    }
    native.on("change", {
        let sync = sync.clone();
        move |_| sync()
    });
    let form = native.form();
    if form.is_some() {
        // A reset restores the native control's values after this event, so
        // the custom face has to be redrawn a turn later.
        let sync = sync.clone();
        form.on("reset", move |_| {
            let sync = sync.clone();
            set_timeout(0, move |_| sync());
        });
    }
    sync();
}

/// The options of a listbox that can be chosen.
fn usable_options(list: El) -> Vec<El> {
    list.query_all("[role=\"option\"]")
        .iter()
        .filter(|option| !option.attr("aria-disabled").is_some_and(|v| v.eq_str("true")))
        .collect()
}

/// The next item in a listbox, clamped at both ends.
///
/// A listbox clamps where a menu wraps: pressing Down at the bottom of an
/// open select stays on the last option, which is what a native select does.
fn step_within<T: Copy>(items: &[T], here: Option<usize>, step: i32) -> Option<T> {
    if items.is_empty() {
        return None;
    }
    let last = items.len() as i32 - 1;
    match here {
        Some(i) => items.get((i as i32 + step).clamp(0, last) as usize).copied(),
        // With nothing active, either end is the first option: that is what
        // `items[min(i + 1, …)] || items[0]` and `items[max(i - 1, 0)]` both
        // come to when `i` is `-1`.
        None => items.first().copied(),
    }
}

/// Extend or restart a type-ahead buffer.
///
/// Within the window the keys build one query, so `de` finds "Delete"
/// rather than jumping to whatever starts with `e`; after it, the next key
/// starts over.
fn extend_type_ahead(buffer: &str, at: f64, key: &str, now: f64) -> (String, f64) {
    let mut next = if now - at > TYPE_AHEAD_MS { String::new() } else { buffer.to_owned() };
    next.push_str(key);
    (next, now)
}

/// Whether a label answers a type-ahead query.
fn starts_with_query(label: &str, query: &str) -> bool {
    label.trim().to_lowercase().starts_with(query)
}

// ---------------------------------------------------------------- combobox

/// The combobox over an `<input list>`.
///
/// Without the module the input keeps its `list` and the browser's own
/// datalist dropdown, so the `list` attribute is only removed once the
/// replacement list exists.
pub fn combobox(root: El) {
    let (input, list) = (root.query(".nr-field-input"), root.query(".nr-combobox-list"));
    if input.is_null() || list.is_null() {
        return;
    }
    // `input.list` is the `<datalist>` the browser resolved — the zero-script
    // path's own dropdown — and it has to be read before `list` is removed.
    let (wrap, datalist) = (root.query(".nr-field-wrap"), input.val().get("list").el());
    if wrap.is_null() || datalist.is_null() {
        return;
    }
    let values: Vec<String> = datalist.query_all("option").iter().map(|o| o.value().into_string()).collect();
    let list_id = list.prop_str("id").map(|id| id.into_string()).unwrap_or_default();

    input.remove_attr("list");
    input.set_attr("role", "combobox");
    input.set_attr("aria-autocomplete", "list");
    aria(input, "aria-expanded", false);
    input.set_attr("aria-controls", &list_id);

    let active: Rc<RefCell<El>> = Rc::new(RefCell::new(El::NULL));

    let set_active = {
        let active = active.clone();
        move |item: El| {
            let mut active = active.borrow_mut();
            if active.is_some() {
                active.remove_attr("data-active");
            }
            *active = item;
            if active.is_some() {
                active.set_attr("data-active", "");
                if let Some(id) = active.prop_str("id") {
                    input.set_attr("aria-activedescendant", &id);
                }
                active.scroll_into_view(false);
            } else {
                input.remove_attr("aria-activedescendant");
            }
        }
    };

    let close = {
        let set_active = set_active.clone();
        move || {
            if list.prop_bool("hidden") {
                return;
            }
            pop::hide(list);
            root.remove_attr("data-open");
            aria(input, "aria-expanded", false);
            set_active(El::NULL);
        }
    };
    root.on(pop::CLOSE, {
        let close = close.clone();
        move |_| close()
    });

    let show = {
        let (values, list_id, set_active) = (values.clone(), list_id.clone(), set_active.clone());
        move || {
            if input.prop_bool("disabled") || input.prop_bool("readOnly") {
                return;
            }
            let current = input.value().into_string();
            list.set_text("");
            let matches = filter_values(&values, &current);
            for (i, value) in matches.iter().enumerate() {
                let item = El::create("li");
                let mut id = list_id.clone();
                id.push('-');
                id.push_str(&i.to_string());
                item.set_attr("id", &id);
                item.set_attr("role", "option");
                aria(item, "aria-selected", **value == current);
                item.set_text(value);
                list.append(item);
            }
            if matches.is_empty() {
                // A list that empties itself would look broken, and a
                // `presentation` row keeps it out of the option count.
                let item = El::create("li");
                item.set_attr("role", "presentation");
                item.set_text("No matches");
                list.append(item);
            }
            set_active(El::NULL);
            if !list.prop_bool("hidden") {
                pop::position(list, wrap, true);
                return;
            }
            pop::close_all(root);
            root.set_attr("data-open", "");
            pop::show(list, root, pop::Anchor::Field);
            aria(input, "aria-expanded", true);
        }
    };

    let choose = {
        let close = close.clone();
        move |item: El| {
            input.set_value(&item.text());
            changed(input);
            close();
        }
    };

    input.on("input", {
        let show = show.clone();
        move |_| show()
    });
    input.on("click", {
        let show = show.clone();
        move |_| show()
    });
    input.on("keydown", {
        let (active, set_active, show, choose, close) =
            (active.clone(), set_active.clone(), show.clone(), choose.clone(), close.clone());
        move |ev: Event| {
            let step = if ev.key_is("ArrowDown") {
                1
            } else if ev.key_is("ArrowUp") {
                -1
            } else {
                0
            };
            if step != 0 {
                ev.prevent_default();
                if list.prop_bool("hidden") {
                    show();
                }
                let all: Vec<El> = list.query_all("[role=\"option\"]").iter().collect();
                if !all.is_empty() {
                    let current = *active.borrow();
                    let here = all.iter().position(|item| item.val().same(current.val())).map_or(-1, |i| i as i32);
                    let len = all.len() as i32;
                    set_active(all[(here + step).rem_euclid(len) as usize]);
                }
            } else if ev.key_is("Enter") && active.borrow().is_some() {
                ev.prevent_default();
                // Copied out first: choosing closes, and closing takes
                // `active` mutably.
                let current = *active.borrow();
                choose(current);
            } else if ev.key_is("Escape") && !list.prop_bool("hidden") {
                ev.prevent_default();
                close();
            }
        }
    });
    // Keep the focus in the input while choosing with the pointer, or the
    // blur below would close the list before the click landed.
    list.on("pointerdown", |ev: Event| ev.prevent_default());
    list.on("click", {
        let choose = choose.clone();
        move |ev: Event| {
            let item = ev.closest("[role=\"option\"]");
            if item.is_some() {
                choose(item);
            }
        }
    });
    input.on("blur", move |_| close());
}

/// The values that match what has been typed, case-insensitively, anywhere
/// in the value rather than only at its start.
fn filter_values<'a>(values: &'a [String], query: &str) -> Vec<&'a String> {
    let query = query.trim().to_lowercase();
    values.iter().filter(|value| value.to_lowercase().contains(&query)).collect()
}

// ---------------------------------------------------------------- tags

/// The tags input.
///
/// The field submits `a, b, c` from a hidden input — the same string the
/// plain text field submits without the module — so a form works either way.
pub fn tags(root: El) {
    let input = root.query(".nr-field-input");
    if input.is_null() {
        return;
    }

    let hidden = El::create("input");
    hidden.set_attr("type", "hidden");
    hidden.set_attr("name", input.prop_str("name").as_deref().unwrap_or_default());
    input.remove_attr("name");
    let row = El::create("div");
    row.set_prop_str("className", "nr-tags-row");
    // `input.before(row)`: the parent is read before the input moves into the
    // row below, which is the node the row has to be inserted into.
    input.parent().insert_before(row, input);
    row.append(input);
    row.append(hidden);

    let max = root.attr("data-max").and_then(|m| m.as_str().parse::<usize>().ok()).unwrap_or(usize::MAX);
    let floats = root.has_attr("data-float");
    let required = input.prop_bool("required");
    let initial = input.value().into_string();
    let tags: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let sync = {
        let tags = tags.clone();
        move || {
            let tags = tags.borrow();
            hidden.set_value(&tags.join(", "));
            for chip in row.query_all(".nr-tags-chip").iter() {
                chip.remove();
            }
            for (i, tag) in tags.iter().enumerate() {
                let chip = El::create("span");
                chip.set_prop_str("className", "nr-tags-chip");
                chip.set_text(tag);
                let remove = El::create("button");
                remove.set_attr("type", "button");
                remove.set_text("×");
                remove.set_attr("data-index", &i.to_string());
                let mut label = String::from("Remove ");
                label.push_str(tag);
                remove.set_attr("aria-label", &label);
                chip.append(remove);
                row.insert_before(chip, input);
            }
            root.toggle_attr("data-float", floats && tags.is_empty());
            // The field stays required until it holds something, so an empty
            // tags field is caught by the browser like any other.
            input.set_prop_bool("required", required && tags.is_empty());
        }
    };

    let add = {
        let (tags, sync) = (tags.clone(), sync.clone());
        move |text: &str, quiet: bool| {
            let before = tags.borrow().len();
            {
                let mut tags = tags.borrow_mut();
                for tag in split_tags(text) {
                    if tags.len() >= max || tags.contains(&tag) {
                        continue;
                    }
                    tags.push(tag);
                }
            }
            input.set_value("");
            sync();
            if !quiet && tags.borrow().len() != before {
                root.dispatch("nr:change", &values_json(&tags.borrow()));
            }
        }
    };

    add(&initial, true);

    input.on("keydown", {
        let (tags, sync, add) = (tags.clone(), sync.clone(), add.clone());
        move |ev: Event| {
            let typed_text = input.value();
            if (ev.key_is("Enter") || ev.key_is(",")) && !typed_text.trim().is_empty() {
                ev.prevent_default();
                add(typed_text.as_str(), false);
            } else if ev.key_is(",") {
                // A lone comma would otherwise land in the field and then be
                // trimmed away, leaving a stray separator behind.
                ev.prevent_default();
            } else if ev.key_is("Backspace") && typed_text.is_empty() && !tags.borrow().is_empty() {
                tags.borrow_mut().pop();
                sync();
                root.dispatch("nr:change", &values_json(&tags.borrow()));
            }
        }
    });
    input.on("change", {
        let add = add.clone();
        move |_| add(input.value().as_str(), false)
    });
    input.on("blur", {
        let add = add.clone();
        move |_| add(input.value().as_str(), false)
    });
    row.on("click", {
        let (tags, sync) = (tags.clone(), sync.clone());
        move |ev: Event| {
            let remove = ev.closest("button[data-index]");
            if remove.is_null() {
                return;
            }
            let Some(at) = remove.attr("data-index").and_then(|i| i.as_str().parse::<usize>().ok()) else { return };
            if at < tags.borrow().len() {
                tags.borrow_mut().remove(at);
            }
            sync();
            root.dispatch("nr:change", &values_json(&tags.borrow()));
            input.focus();
        }
    });
    let form = input.form();
    if form.is_some() {
        form.on("reset", move |_| {
            let (tags, add, initial) = (tags.clone(), add.clone(), initial.clone());
            set_timeout(0, move |_| {
                tags.borrow_mut().clear();
                add(&initial, true);
            });
        });
    }
}

/// The tags in a comma-separated string: trimmed, and never empty.
fn split_tags(text: &str) -> Vec<String> {
    text.split(',').map(str::trim).filter(|tag| !tag.is_empty()).map(str::to_owned).collect()
}

/// `{"values":[…]}` for the `nr:change` detail.
fn values_json(tags: &[String]) -> String {
    let mut out = String::from("{\"values\":[");
    for (i, tag) in tags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        for c in tag.chars() {
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

// ---------------------------------------------------------------- fields

/// Field actions and the slider's fill, on the document.
pub fn fields() {
    on_document("click", |ev: Event| {
        let button = ev.closest("[data-nr-field-action]");
        if button.is_some() {
            // The field is found in two steps rather than chained: the
            // shim's allocator turns a null receiver into a handle holding
            // `0`, so a chain off a missing ancestor throws instead of
            // reading as absent.
            let field = button.closest(".nr-field");
            let Some(action) = button.attr("data-nr-field-action") else { return };
            if field.is_null() {
                return;
            }
            let input = field.query("input.nr-field-input, textarea.nr-field-input");
            if input.is_null() {
                return;
            }
            match action.as_str() {
                "toggle-password" => {
                    let show = input.prop_str("type").is_some_and(|t| t.eq_str("password"));
                    input.set_prop_str("type", if show { "text" } else { "password" });
                    aria(button, "aria-pressed", show);
                    button.set_attr("aria-label", if show { "Hide password" } else { "Show password" });
                    input.focus();
                    // Focus lands at the start after a type change, which
                    // would put the caret in the middle of what was typed.
                    let end = input.value().len() as u32;
                    input.set_selection(end, end);
                }
                "clear" => {
                    input.set_value("");
                    changed(input);
                    // The person is being sent back to the field, so the
                    // browser should scroll it into view; everywhere else
                    // the kit moves focus it must not.
                    input.focus_visible();
                }
                "step-up" | "step-down" => {
                    // `stepUp`/`stepDown` throw on a control with no usable
                    // step; the host call swallows it, as the script's `try`
                    // did.
                    input.val().call(if action.eq_str("step-up") { "stepUp" } else { "stepDown" });
                    changed(input);
                }
                _ => {}
            }
            return;
        }
        // A click anywhere in a text field's box focuses the input, the way
        // a click on a native control's padding does.
        let wrap = ev.closest(".nr-input .nr-field-wrap, .nr-textarea .nr-field-wrap");
        if wrap.is_some() && ev.closest("input, textarea, button, a, select").is_null() {
            let input = wrap.query(".nr-field-input");
            if input.is_some() {
                input.focus_visible();
            }
        }
    });

    on_document("input", |ev: Event| {
        let el = ev.target();
        if !el.matches(".nr-slider-input, .nr-color-input") {
            return;
        }
        let root = el.closest("[data-nr-ui]");
        if root.is_null() {
            return;
        }
        if el.prop_str("type").is_some_and(|t| t.eq_str("range")) {
            let min = el.attr("min").unwrap_or_default();
            let max = el.attr("max").unwrap_or_default();
            root.style_set("--nr-v", &percent(fill(&min, &max, &el.value())));
        }
        let out = root.query("[data-nr-echo]");
        if out.is_some() {
            let mut text = el.value().into_string();
            text.push_str(out.attr("data-unit").as_deref().unwrap_or_default());
            out.set_text(&text);
        }
    });
}

/// How far along its track a range input sits, as a percentage.
///
/// An absent `max` is 100, as the HTML default is, and a zero-width range
/// reads as the start rather than as a division by zero.
fn fill(min: &str, max: &str, value: &str) -> f64 {
    let min = crate::util::parse_number(min);
    let max = if max.is_empty() { 100.0 } else { crate::util::parse_number(max) };
    let value = crate::util::parse_number(value);
    let span = max - min;
    (value - min) / if span == 0.0 { 1.0 } else { span } * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn a_listbox_clamps_where_a_menu_would_wrap() {
        let items = [10, 20, 30];
        assert_eq!(step_within(&items, Some(0), 1), Some(20));
        assert_eq!(step_within(&items, Some(2), 1), Some(30), "the last option stays the last");
        assert_eq!(step_within(&items, Some(0), -1), Some(10), "the first option stays the first");
        assert_eq!(step_within(&items, None, 1), Some(10));
        assert_eq!(step_within(&items, None, -1), Some(10));
        assert_eq!(step_within::<i32>(&[], Some(0), 1), None);
    }

    #[test]
    fn type_ahead_builds_a_query_and_then_starts_over() {
        let (buffer, at) = extend_type_ahead("", 0.0, "d", 1_000.0);
        assert_eq!((buffer.as_str(), at), ("d", 1_000.0));
        let (buffer, at) = extend_type_ahead(&buffer, at, "e", 1_300.0);
        assert_eq!(buffer, "de", "a second key within the window extends the query");
        let (buffer, _) = extend_type_ahead(&buffer, at, "f", 1_300.0 + TYPE_AHEAD_MS + 1.0);
        assert_eq!(buffer, "f", "after the window the next key starts over");
        let (buffer, _) = extend_type_ahead("de", 0.0, "l", TYPE_AHEAD_MS);
        assert_eq!(buffer, "del", "exactly at the window is still the same query");
    }

    #[test]
    fn a_type_ahead_query_ignores_case_and_surrounding_space() {
        assert!(starts_with_query("  Delete  ", "del"));
        assert!(starts_with_query("DELETE", "del"));
        assert!(starts_with_query("Delete", ""), "an empty query matches the first option");
        assert!(!starts_with_query("Delete", "x"));
    }

    #[test]
    fn the_combobox_filters_anywhere_in_the_value() {
        let values: Vec<String> = ["Apple", "Pineapple", "Banana"].iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(filter_values(&values, "app"), vec!["Apple", "Pineapple"]);
        assert_eq!(filter_values(&values, " APP "), vec!["Apple", "Pineapple"], "trimmed and case-insensitive");
        assert_eq!(filter_values(&values, ""), vec!["Apple", "Pineapple", "Banana"]);
        assert!(filter_values(&values, "zzz").is_empty());
    }

    #[test]
    fn tags_split_on_commas_and_drop_the_empty_ones() {
        assert_eq!(split_tags("a, b ,c"), vec!["a", "b", "c"]);
        assert_eq!(split_tags(" , , "), Vec::<String>::new());
        assert_eq!(split_tags(""), Vec::<String>::new());
        assert_eq!(split_tags("one"), vec!["one"]);
        assert_eq!(split_tags("a, á"), vec!["a", "á"]);
    }

    #[test]
    fn the_change_detail_is_valid_json() {
        assert_eq!(values_json(&[]), "{\"values\":[]}");
        assert_eq!(values_json(&["a".to_owned(), "b".to_owned()]), "{\"values\":[\"a\",\"b\"]}");
        assert_eq!(values_json(&["a\"b".to_owned()]), "{\"values\":[\"a\\\"b\"]}");
        assert_eq!(values_json(&["a\\b".to_owned()]), "{\"values\":[\"a\\\\b\"]}");
    }

    #[test]
    fn a_sliders_fill_spans_its_range() {
        assert_eq!(fill("0", "100", "50"), 50.0);
        assert_eq!(fill("", "", "25"), 25.0, "an absent max is 100, as HTML says");
        assert_eq!(fill("10", "20", "15"), 50.0);
        assert_eq!(fill("-50", "50", "0"), 50.0);
        assert_eq!(fill("5", "5", "5"), 0.0, "a zero-width range is the start, not a division by zero");
        assert_eq!(fill("x", "y", "z"), 0.0, "nothing numeric reads as the start");
    }
}
