//! The carousel, the dismissible banner, the file dropzone and `on_press`.

use alloc::string::String;

use next_rust_wasm::event::ListenOpts;
use next_rust_wasm::{El, Event, Metric, action_text, by_id, error, navigate, on_document, refresh, spawn, storage};

use crate::util::{number, parse_number};
use crate::{activates, inactive};
use next_rust_wasm::num::{abs, ceil, round};

/// The stringified boolean ARIA wants.
fn aria(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

// ---------------------------------------------------------------- on_press

/// `on_press`, on the document.
///
/// One listener serves every pressable element on the page, however it got
/// there, which is why nothing here is mounted per component.
pub fn press() {
    on_document("click", |ev| {
        let el = ev.closest("[data-nr-press]");
        if el.is_null() {
            return;
        }
        if inactive(el) {
            return;
        }
        let kind = el.attr("data-nr-press").unwrap_or_default();
        let target = el.attr("data-nr-press-target").unwrap_or_default();
        match kind.as_str() {
            "navigate" => {
                ev.prevent_default();
                navigate(target.as_str());
            }
            "modal" => {
                ev.prevent_default();
                let dialog = by_id(target.as_str());
                if dialog.is_some() {
                    crate::overlays::open_modal(dialog);
                }
            }
            "close-modal" => {
                ev.prevent_default();
                let dialog = el.closest("dialog");
                if dialog.is_some() {
                    dialog.close_dialog();
                }
            }
            "toast" => {
                crate::overlays::toast(&crate::json::parse(&quoted(target.as_str())));
            }
            "emit" => {
                let mut name = String::from("nr:");
                name.push_str(target.as_str());
                el.dispatch(&name, "null");
            }
            "action" => {
                ev.prevent_default();
                run_action(el, target.as_str());
            }
            _ => {}
        }
    });

    // A card or a table row can be pressable without being a button, so it
    // has to answer the keys a button would answer for free.
    on_document("keydown", |ev| {
        let el = ev.target();
        if activates(ev) && el.matches("[data-nr-press][role=\"button\"]") {
            ev.prevent_default();
            el.val().call("click");
        }
    });
}

/// A bare string as JSON, so a `data-nr-press-target` and an error message
/// can go through the same reader a `nextRust.toast(…)` detail does.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // A control character has no short escape and never appears in
            // an attribute the renderer wrote; drop it rather than emit
            // JSON the host cannot parse.
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Call a server action and report the outcome on the element.
///
/// The element outlives the round trip, which is what the handle model is
/// built for: a handle made at mount time is the page's for as long as the
/// element is. The busy marks come off however the action settled — including
/// when it failed, or a second press would find the button still busy.
fn run_action(el: El, url: &str) {
    let input = el.attr("data-nr-press-input").map_or_else(|| String::from("null"), String::from);
    let url = String::from(url);
    el.set_attr("data-loading", "");
    el.set_attr("aria-busy", "true");
    spawn(async move {
        // `action_text` rather than `action`: the kit hands the reply
        // straight to somebody else's `nr:success` listener without looking
        // inside it, and the props blob `action` decodes into hashes field
        // names one way, so a nested reply could not be written back out.
        match action_text(&url, &input).await {
            Ok(reply) => {
                el.dispatch("nr:success", reply.as_str());
                if !el.attr("data-nr-press-refresh").is_some_and(|value| value.eq_str("false")) {
                    refresh();
                }
            }
            Err(failure) => {
                let message = failure.message();
                el.dispatch("nr:error", &quoted(message));
                if !el.has_attr("data-nr-quiet") {
                    let mut line = String::from("[next-rust] action failed: ");
                    line.push_str(message);
                    error(&line);
                }
            }
        }
        el.remove_attr("data-loading");
        el.remove_attr("aria-busy");
    });
}

// ---------------------------------------------------------------- upload

/// A file input's dropzone: the drag highlight, and the list of chosen
/// files.
///
/// Without the module the input is a plain `<input type=file>`, which already
/// accepts a drop and already names what was chosen — this adds the
/// highlight and the readout on top of it.
pub fn upload(root: El) {
    let (zone, input) = (root.query(".nr-dropzone"), root.query(".nr-dropzone-input"));
    if zone.is_null() || input.is_null() {
        return;
    }
    let files = root.query(".nr-dropzone-files");
    for event in ["dragenter", "dragover"] {
        zone.on(event, move |_| {
            zone.set_attr("data-drag", "");
        });
    }
    for event in ["dragleave", "drop"] {
        zone.on(event, move |_| zone.remove_attr("data-drag"));
    }
    input.on("change", move |_| {
        if files.is_null() {
            return;
        }
        files.set_text("");
        let chosen = input.files();
        let count = chosen.len();
        for i in 0..count {
            let file = chosen.get(i);
            let item = El::create("li");
            item.set_text(&file_line(file.name().as_str(), file.size()));
            files.append(item);
        }
        files.set_prop_bool("hidden", count == 0);
    });
}

/// `name (12 KB)` — kilobytes rounded up below a megabyte, megabytes with one
/// decimal above it, which is what the script writes.
fn file_line(name: &str, size: f64) -> String {
    let mut out = String::from(name);
    out.push_str(" (");
    if size < 1e6 {
        out.push_str(&number(ceil(size / 1e3)));
        out.push_str(" KB");
    } else {
        out.push_str(&crate::util::fixed(size / 1e6, 1));
        out.push_str(" MB");
    }
    out.push(')');
    out
}

// ---------------------------------------------------------------- carousel

/// A carousel's arrows.
///
/// The track is CSS scroll-snap and scrolls with no script at all; this adds
/// the two buttons and keeps them marked disabled at the ends.
pub fn carousel(root: El) {
    let (view, prev, next) =
        (root.query(".nr-carousel-viewport"), root.query(".nr-carousel-prev"), root.query(".nr-carousel-next"));
    if view.is_null() || prev.is_null() || next.is_null() {
        return;
    }
    // In a right-to-left track `scrollLeft` counts the other way, so the
    // arrows would swap without this.
    let rtl = view.style("direction").is_some_and(|value| value.eq_str("rtl"));

    let update = move || {
        let x = abs(view.metric(Metric::ScrollLeft));
        prev.set_attr("aria-disabled", aria(x < 2.0));
        next.set_attr(
            "aria-disabled",
            aria(x > view.metric(Metric::ScrollWidth) - view.metric(Metric::ClientWidth) - 2.0),
        );
    };

    root.on("click", move |ev| {
        let btn = ev.closest("[data-nr-slide]");
        if btn.is_null() {
            return;
        }
        if btn.attr("aria-disabled").is_some_and(|value| value.eq_str("true")) {
            return;
        }
        let direction = btn.attr("data-nr-slide").and_then(|d| d.parse_f64()).unwrap_or(0.0);
        view.scroll_by(step(view, direction, rtl));
    });
    view.on_opts("scroll", ListenOpts::PASSIVE, move |_: Event| update());
    view.observe_resize(move |_: Event| update());
    update();
}

/// How far one press of an arrow scrolls: one slide, or a whole page of them
/// when several are visible at once.
fn step(view: El, direction: f64, rtl: bool) -> f64 {
    let gap = parse_number(view.style("column-gap").as_deref().unwrap_or(""));
    let slide = view.query(".nr-carousel-item");
    let by = if slide.is_some() {
        let width = slide.metric(Metric::OffsetWidth) + gap;
        if width > 0.0 {
            width * round(view.metric(Metric::ClientWidth) / width).max(1.0)
        } else {
            view.metric(Metric::ClientWidth)
        }
    } else {
        view.metric(Metric::ClientWidth)
    };
    direction * by * if rtl { -1.0 } else { 1.0 }
}

// ---------------------------------------------------------------- banner

/// A banner that stays dismissed.
///
/// Dismissing it is a native checkbox and `:has()`, so it closes with no
/// script; the module only remembers that it was closed. Storage that throws
/// — a locked-down browser, private mode — leaves the banner behaving as it
/// does without the module rather than breaking it.
pub fn banner(root: El) {
    let Some(id) = root.attr("data-nr-persist") else { return };
    let mut key = String::from("nr-banner:");
    key.push_str(id.as_str());
    if storage::local_get(&key).is_some_and(|value| !value.is_empty()) {
        root.remove();
        return;
    }
    root.on("change", move |_| {
        storage::local_set(&key, "1");
        root.dispatch("nr:dismiss", "null");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_reads_as_kilobytes_below_a_megabyte_and_megabytes_above() {
        assert_eq!(file_line("a.pdf", 0.0), "a.pdf (0 KB)");
        assert_eq!(file_line("a.pdf", 1.0), "a.pdf (1 KB)", "any file at all is at least a kilobyte");
        assert_eq!(file_line("a.pdf", 12_400.0), "a.pdf (13 KB)", "kilobytes round up");
        assert_eq!(file_line("a.pdf", 999_999.0), "a.pdf (1000 KB)");
        assert_eq!(file_line("a.pdf", 1_000_000.0), "a.pdf (1.0 MB)");
        assert_eq!(file_line("a.pdf", 3_450_000.0), "a.pdf (3.5 MB)");
        assert_eq!(file_line("résumé .pdf", 2_000_000.0), "résumé .pdf (2.0 MB)");
    }

    #[test]
    fn a_press_target_becomes_a_json_string() {
        assert_eq!(quoted("Saved"), "\"Saved\"");
        assert_eq!(quoted("a\"b"), "\"a\\\"b\"");
        assert_eq!(quoted("a\\b"), "\"a\\\\b\"");
        assert_eq!(quoted("a\nb"), "\"a\\nb\"");
        assert_eq!(crate::json::parse(&quoted("Saved")).get("message"), Some("Saved"));
        assert_eq!(crate::json::parse(&quoted("a\"b")).get("message"), Some("a\"b"));
        assert_eq!(crate::json::parse(&quoted("héllo — ok")).get("message"), Some("héllo — ok"));
    }

    /// With no DOM every metric is zero, and a zero-width slide must not
    /// divide by it — in the browser that is a carousel that never moves.
    #[test]
    fn a_carousel_step_survives_a_zero_width_slide() {
        assert_eq!(step(El::NULL, 1.0, false), 0.0);
        assert_eq!(step(El::NULL, -1.0, true), -0.0);
    }

    /// ARIA booleans are written out, not the attribute's presence.
    #[test]
    fn aria_booleans_are_the_words() {
        assert_eq!(aria(true), "true");
        assert_eq!(aria(false), "false");
    }
}
