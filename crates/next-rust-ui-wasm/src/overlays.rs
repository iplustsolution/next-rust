//! Overlays and menus: modals, dropdown and context menus, the command
//! palette, the toaster, closable alerts and the app shell's drawer.
//!
//! Almost all of this is native behaviour the module only sharpens. A
//! `<dialog>` opens, traps focus and closes on `Escape` by itself; a
//! `popover` with `popovertarget` opens and closes by itself; the drawer is
//! a checkbox and `:has()`. What the module adds is what HTML has no answer
//! for: roving focus and type-ahead inside a menu, placement where the
//! browser has no anchor positioning, a toaster that stays above a modal,
//! and the palette's filtering.

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use next_rust_wasm::event::ListenOpts;
use next_rust_wasm::global::install_forwarder;
use next_rust_wasm::{
    El, Event, Metric, active, css_supports, document, on_document, on_document_opts, on_window, on_window_opts, raf,
    spawn,
};

use crate::json::Options;
use crate::util::{CROSS, svg};

/// Icon path data per toast colour, matching the icons the renderer uses.
const TOAST_ICONS: [(&str, &str); 4] = [
    ("default", "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 16v-4\"/><path d=\"M12 8h.01\"/>"),
    ("success", "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"m9 12 2 2 4-4\"/>"),
    (
        "warning",
        "<path d=\"m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3\"/>\
         <path d=\"M12 9v4\"/><path d=\"M12 17h.01\"/>",
    ),
    ("danger", "<circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"m15 9-6 6\"/><path d=\"m9 9 6 6\"/>"),
];

/// Whether CSS anchor positioning places popovers for us: where it does, the
/// module leaves placement alone entirely.
///
/// The JavaScript probes once and remembers. A `no_std` module has nowhere to
/// remember it without a `static mut`, and the answer is one host call, so it
/// is asked each time — the trade [`crate::pop::top_layer`] already makes.
fn anchored() -> bool {
    css_supports("position-area: block-end")
}

/// The stringified boolean ARIA wants.
fn aria(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

/// `disabled`, or `aria-disabled="true"`.
fn is_disabled(el: El) -> bool {
    el.prop_bool("disabled") || el.attr("aria-disabled").is_some_and(|value| value.eq_str("true"))
}

/// Whether two handles name the same node. Handles are not unique — two
/// queries for one element give two of them — so identity is a host call.
fn same(a: El, b: El) -> bool {
    a.val().same(b.val())
}

/// `event.newState === "open"`, on `beforetoggle` and `toggle`.
fn opening(ev: Event) -> bool {
    ev.new_state().eq_str("open")
}

// ---------------------------------------------------------------- menus

/// Dropdown and popover roots: keyboard navigation and placement.
///
/// Without a top layer the popover is a plain box the stylesheet positions,
/// and the browser's own `popovertarget` still opens and closes it — so this
/// returns early rather than degrade it.
pub fn popover(root: El) {
    let pop = root.query("[popover]");
    if pop.is_null() {
        return;
    }
    if !crate::pop::top_layer() {
        return;
    }
    let trigger = root.query("[popovertarget]");
    let is_menu = pop.attr("role").is_some_and(|role| role.eq_str("menu"));

    // The popover is shown by the time the frame runs but not yet painted,
    // which is the only moment its size is known and its position is not
    // visibly wrong.
    pop.on("beforetoggle", move |ev| {
        if trigger.is_null() {
            return;
        }
        if !opening(ev) || anchored() {
            return;
        }
        spawn(async move {
            raf().await;
            crate::pop::position(pop, trigger, false);
            crate::pop::follow_anchor(pop, crate::pop::Anchor::Trigger);
        });
    });

    pop.on("toggle", move |ev| {
        let on = opening(ev);
        if trigger.is_some() {
            trigger.set_attr("aria-expanded", aria(on));
        }
        if on && is_menu {
            let items = menu_items(pop);
            items.first().copied().unwrap_or(pop).focus();
        }
        if !on {
            crate::pop::forget_anchor(pop);
        }
        root.dispatch(if on { "nr:open" } else { "nr:close" }, "null");
    });

    if !is_menu {
        return;
    }

    pop.on("keydown", move |ev| {
        let all = menu_items(pop);
        let focused = active();
        let here = all.iter().position(|item| same(*item, focused));
        let key = ev.key();
        let to = menu_target(&all, here, key.as_str(), ev.ctrl() || ev.meta(), |item| item.text().into_string());
        // Tab leaves the menu, and a menu left open behind the page is worse
        // than one that closes a frame early.
        if ev.key_is("Tab") {
            pop.hide_popover();
        }
        let (Some(to), false) = (to, all.is_empty()) else { return };
        ev.prevent_default();
        all[to.rem_euclid(all.len() as i32) as usize].focus();
    });

    // Choosing an item closes the menu; the item's own handler still runs,
    // because `click` has already reached it by the time this does.
    pop.on("click", move |ev| {
        if ev.closest("[role=\"menuitem\"]").is_some() {
            pop.hide_popover();
        }
    });

    if trigger.is_some() {
        trigger.on("keydown", move |ev| {
            if (ev.key_is("ArrowDown") || ev.key_is("ArrowUp")) && !pop.popover_open() {
                ev.prevent_default();
                trigger.val().call("click");
            }
        });
    }
}

/// The items of a menu that can be focused.
fn menu_items(pop: El) -> Vec<El> {
    pop.query_all("[role=\"menuitem\"]").iter().filter(|item| !is_disabled(*item)).collect()
}

/// Where a key press moves focus in a menu, as an index that may be out of
/// range on either side — the caller wraps it.
///
/// `label` reads an item's text, so the type-ahead can be decided without a
/// DOM: the next item after the current one whose label starts with the key,
/// wrapping around to the ones before it.
fn menu_target<T>(
    items: &[T],
    here: Option<usize>,
    key: &str,
    modified: bool,
    label: impl Fn(&T) -> String,
) -> Option<i32> {
    let i = here.map_or(-1, |i| i as i32);
    match key {
        "ArrowDown" => return Some(i + 1),
        "ArrowUp" => return Some(if i < 0 { -1 } else { i - 1 }),
        "Home" => return Some(0),
        "End" => return Some(items.len() as i32 - 1),
        _ => {}
    }
    let mut chars = key.chars();
    let (Some(first), None) = (chars.next(), chars.next()) else { return None };
    if modified || first.is_whitespace() {
        return None;
    }
    let wanted = first.to_lowercase().to_string();
    let after = (i + 1).max(0) as usize;
    let order = items.iter().enumerate().skip(after).chain(items.iter().enumerate().take(after.min(items.len())));
    order.filter(|(_, item)| label(item).trim().to_lowercase().starts_with(&wanted)).map(|(at, _)| at as i32).next()
}

/// A context menu: a dropdown that opens where the pointer is.
///
/// It is a *manual* popover, because the pointer-up that follows a right
/// click would dismiss an automatic one immediately; everything that closes
/// it is therefore spelled out here.
pub fn context_menu(root: El) {
    popover(root);
    let pop = root.query("[popover]");
    if pop.is_null() {
        return;
    }
    if !crate::pop::top_layer() {
        return;
    }
    let close = move || {
        if pop.popover_open() {
            pop.hide_popover();
        }
    };
    on_document_opts("pointerdown", ListenOpts::CAPTURE, move |ev| {
        if !pop.contains(ev.target()) {
            close();
        }
    });
    on_window_opts("scroll", ListenOpts { capture: true, passive: true, once: false }, move |_| close());
    pop.on("keydown", move |ev| {
        if ev.key_is("Escape") {
            ev.prevent_default();
            close();
        }
    });
    root.on("contextmenu", move |ev| {
        if pop.contains(ev.target()) {
            return;
        }
        ev.prevent_default();
        close();
        pop.show_popover();
        let (x, y) = (ev.client_x(), ev.client_y());
        let (inner_w, inner_h) = crate::pop::viewport();
        let left = crate::pop::inside(x, pop.metric(Metric::OffsetWidth), inner_w);
        let top = crate::pop::inside(y, pop.metric(Metric::OffsetHeight), inner_h);
        pop.style_set("left", &crate::util::px(left));
        pop.style_set("top", &crate::util::px(top));
    });
}

// ---------------------------------------------------------------- modals

/// Open a `<dialog>` as a real modal.
pub fn open_modal(dialog: El) {
    if dialog.is_null() || dialog.dialog_open() {
        return;
    }
    dialog.show_modal();
    // The toaster enters the top layer after the dialog, so toasts stay
    // above it rather than behind it.
    let toaster = document().query("[data-nr-ui=\"toaster\"]");
    if toaster.is_some() {
        show_toaster(toaster);
    }
    dialog.dispatch("nr:open", "null");
}

/// A `<dialog>`.
pub fn modal(dialog: El) {
    // Rendered with `open`, it is a non-modal dialog: the page behind it is
    // live and focus is not kept. Reopen it properly.
    if dialog.dialog_open() {
        dialog.close_dialog();
        open_modal(dialog);
    }
    dialog.on("click", move |ev| {
        // A click on the backdrop lands on the dialog itself, never on its
        // box, which is how a dismissable modal tells the two apart.
        if same(ev.target(), dialog) && dialog.has_attr("data-dismissable") {
            dialog.close_dialog();
        }
    });
    dialog.on("close", move |_| dialog.dispatch("nr:close", "null"));
}

// ---------------------------------------------------------------- palette

/// The command palette.
pub fn command(dialog: El) {
    modal(dialog);
    let input = dialog.query(".nr-command-input");
    if input.is_null() {
        return;
    }
    let empty = dialog.query(".nr-command-empty");
    let active: Rc<RefCell<El>> = Rc::new(RefCell::new(El::NULL));

    let set_active = {
        let active = active.clone();
        move |el: El| {
            let mut active = active.borrow_mut();
            if active.is_some() {
                active.remove_attr("data-active");
                active.set_attr("aria-selected", aria(false));
            }
            *active = el;
            if active.is_some() {
                active.set_attr("data-active", "");
                active.set_attr("aria-selected", aria(true));
                if let Some(id) = active.prop_str("id") {
                    input.set_attr("aria-activedescendant", id.as_str());
                }
                active.scroll_into_view(false);
            } else {
                input.remove_attr("aria-activedescendant");
            }
        }
    };

    let filter = {
        let set_active = set_active.clone();
        move |_: Event| {
            let value = input.value();
            let query = value.as_str().to_lowercase();
            let words: Vec<&str> = query.split_whitespace().collect();
            for item in dialog.query_all(".nr-command-item").iter() {
                let mut text = item.text().into_string();
                text.push(' ');
                if let Some(keywords) = item.attr("data-keywords") {
                    text.push_str(keywords.as_str());
                }
                item.set_prop_bool("hidden", !matches_query(&text, &words));
            }
            for group in dialog.query_all(".nr-command-group").iter() {
                group.set_prop_bool("hidden", group.query(".nr-command-item:not([hidden])").is_null());
            }
            let shown = dialog.query_all(".nr-command-item:not([hidden])");
            if empty.is_some() {
                empty.set_prop_bool("hidden", !shown.is_empty());
            }
            set_active(shown.first());
        }
    };

    input.on("input", filter.clone());
    input.on("keydown", {
        let (active, set_active) = (active.clone(), set_active.clone());
        move |ev| {
            let shown = dialog.query_all(".nr-command-item:not([hidden])");
            let step = if ev.key_is("ArrowDown") {
                1
            } else if ev.key_is("ArrowUp") {
                -1
            } else {
                0
            };
            if step != 0 && !shown.is_empty() {
                ev.prevent_default();
                let current = *active.borrow();
                let here = shown.iter().position(|item| same(item, current)).map_or(-1, |i| i as i32);
                let len = shown.len() as i32;
                set_active(shown.get((here + step).rem_euclid(len) as u32));
            } else if ev.key_is("Enter") && active.borrow().is_some() {
                ev.prevent_default();
                active.borrow().val().call("click");
            }
        }
    });

    dialog.on("pointermove", {
        let (active, set_active) = (active.clone(), set_active.clone());
        move |ev| {
            // Read the current item out before `set_active` borrows it
            // again: a `RefCell` held across the call would panic.
            let current = *active.borrow();
            let item = ev.closest(".nr-command-item");
            if item.is_some() && !same(item, current) {
                set_active(item);
            }
        }
    });

    dialog.on("click", move |ev| {
        if ev.closest(".nr-command-item").is_some() {
            dialog.close_dialog();
        }
    });

    dialog.on("nr:open", move |ev| {
        input.set_value("");
        filter(ev);
        // The palette has just opened and the person is being sent to its
        // field, so letting the browser scroll to it is right here; every
        // other move of focus in the kit must not scroll.
        input.focus_visible();
    });

    if let Some(hotkey) = dialog.attr("data-hotkey") {
        let hotkey = hotkey.as_str().to_lowercase();
        on_document("keydown", move |ev| {
            if !(ev.meta() || ev.ctrl())
                || ev.alt()
                || ev.key().as_str().to_lowercase() != hotkey
                || !dialog.prop_bool("isConnected")
            {
                return;
            }
            ev.prevent_default();
            if dialog.dialog_open() {
                dialog.close_dialog();
            } else {
                open_modal(dialog);
            }
        });
    }
}

/// Whether an item's searchable text matches every word of the query.
///
/// Every word, not the whole phrase: typing `set pro` finds "Profile
/// settings" the way a fuzzy palette is expected to.
fn matches_query(text: &str, words: &[&str]) -> bool {
    let text = text.to_lowercase();
    words.iter().all(|word| text.contains(word))
}

// ---------------------------------------------------------------- toasts

/// Lift the toaster into the top layer, above any modal dialog.
///
/// The top layer is ordered by entry, so a toaster shown before a dialog
/// ends up behind it: when a dialog is open it is taken out and put back.
pub fn show_toaster(host: El) {
    if !crate::pop::top_layer() || host.prop_num("childElementCount").unwrap_or(0.0) == 0.0 {
        return;
    }
    if host.popover_open() {
        if document().query("dialog:modal").is_null() {
            return;
        }
        host.hide_popover();
    }
    // The toaster is a plain region in the markup — see `pop::show` — so it
    // has to become a popover before it can enter the top layer.
    host.set_prop_str("popover", "manual");
    host.show_popover();
}

/// Publish `nr:toast` and clear toasts that have faded out.
pub fn toast_api() {
    // `nextRust.toast(…)` is the kit's public API and app code calls it
    // directly, so it has to exist on both paths. WebAssembly cannot make a
    // JavaScript function, so the shim installs a forwarder that turns the
    // call into the `nr:toast` event the listener below already answers.
    //
    // One documented difference from the script: the forwarder returns
    // nothing, where `toast()` returned the element it had just added.
    install_forwarder("toast", "nr:toast");
    on_window("nr:toast", |ev| {
        let detail = ev.detail();
        toast(&crate::json::parse(detail.as_str()));
    });
    // The stylesheet times the fade; a toast that has finished fading is
    // invisible and must not keep taking up room.
    on_document("animationend", |ev| {
        let el = ev.target();
        if el.matches(".nr-toast") && el.style("visibility").is_some_and(|value| value.eq_str("hidden")) {
            el.remove();
        }
    });
}

/// Show a toast. Returns the element, or [`El::NULL`] when the page has no
/// toaster to put it in.
pub fn toast(options: &Options) -> El {
    let host = document().query("[data-nr-ui=\"toaster\"]");
    if host.is_null() {
        return El::NULL;
    }
    let color = options.get("color").unwrap_or("default");
    let duration = options.number("duration");

    let el = El::create("div");
    // A toast with `duration: 0` never fades, so it must not carry the
    // animation the stylesheet would otherwise remove it with.
    el.set_prop_str("className", if duration == Some(0.0) { "nr-toast nr-toast-sticky" } else { "nr-toast" });
    el.set_attr("data-color", color);
    el.set_attr("role", if color == "danger" || color == "warning" { "alert" } else { "status" });
    if let Some(ms) = duration.filter(|ms| *ms > 0.0) {
        let mut value = crate::util::number(ms);
        value.push_str("ms");
        el.style_set("--nr-toast-d", &value);
    }

    let icon = El::create("span");
    icon.set_prop_str("className", "nr-toast-icon");
    icon.set_html_trusted(&svg(icon_for(color)));
    el.append(icon);

    let body = El::create("div");
    body.set_prop_str("className", "nr-toast-body");
    for (tag, class, text) in
        [("strong", "nr-toast-title", options.get("title")), ("div", "nr-toast-text", options.get("message"))]
    {
        let Some(text) = text.filter(|t| !t.is_empty()) else { continue };
        let part = El::create(tag);
        part.set_prop_str("className", class);
        part.set_text(text);
        body.append(part);
    }
    el.append(body);

    let close = El::create("button");
    close.set_attr("type", "button");
    close.set_prop_str("className", "nr-toast-close");
    close.set_attr("aria-label", "Dismiss");
    close.set_attr("data-nr-dismiss", "");
    close.set_html_trusted(&svg(CROSS));
    el.append(close);

    host.append(el);
    show_toaster(host);
    el
}

/// The icon for a toast colour, falling back to the neutral one.
fn icon_for(color: &str) -> &'static str {
    TOAST_ICONS.iter().find(|(name, _)| *name == color).map_or(TOAST_ICONS[0].1, |(_, body)| *body)
}

// ---------------------------------------------------------------- shell

/// Open or close the app shell's drawer.
fn set_shell(open: bool) {
    let shell = document().query(".nr-shell");
    if shell.is_null() {
        return;
    }
    shell.toggle_attr("data-open", open);
    for button in document().query_all(".nr-navbar-menu").iter() {
        button.set_attr("aria-expanded", aria(open));
    }
}

/// The app shell's drawer.
pub fn shell() {
    on_document("click", |ev| {
        if ev.closest("[data-nr-shell-toggle]").is_some() {
            set_shell(document().query(".nr-shell[data-open]").is_null());
        } else if ev.closest(".nr-shell-sidebar a[href]").is_some() {
            // Following a link inside the drawer should leave it behind.
            set_shell(false);
        }
    });
    on_window("nr:navigate", |_| set_shell(false));
    on_document("keydown", |ev| {
        if ev.key_is("Escape") && document().query(".nr-shell[data-open]").is_some() {
            set_shell(false);
        }
    });
}

// ---------------------------------------------------------------- alerts

/// Closable alerts and toasts.
pub fn alerts() {
    on_document("click", |ev| {
        let button = ev.closest("[data-nr-dismiss]");
        if button.is_null() {
            return;
        }
        let alert = button.closest("[data-nr-ui=\"alert\"], .nr-toast");
        if alert.is_null() {
            return;
        }
        alert.dispatch("nr:dismiss", "null");
        alert.remove();
    });
}

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned;

    use super::*;

    /// The arrow keys, `Home` and `End` move by position; the index may run
    /// off either end, and the caller wraps it.
    #[test]
    fn menu_keys_move_by_position() {
        let items = ["Edit", "Duplicate", "Delete"];
        let label = |s: &&str| (*s).to_owned();
        assert_eq!(menu_target(&items, Some(0), "ArrowDown", false, label), Some(1));
        assert_eq!(menu_target(&items, Some(2), "ArrowDown", false, label), Some(3), "past the end, to be wrapped");
        assert_eq!(menu_target(&items, Some(0), "ArrowUp", false, label), Some(-1), "before the start");
        assert_eq!(menu_target(&items, None, "ArrowUp", false, label), Some(-1), "with nothing focused, the last");
        assert_eq!(menu_target(&items, None, "ArrowDown", false, label), Some(0));
        assert_eq!(menu_target(&items, Some(1), "Home", false, label), Some(0));
        assert_eq!(menu_target(&items, Some(1), "End", false, label), Some(2));
        assert_eq!(menu_target(&items, Some(1), "Escape", false, label), None);
    }

    /// Type-ahead looks forward from the current item and wraps, so pressing
    /// the same letter walks the items that start with it.
    #[test]
    fn menu_type_ahead_walks_the_items_starting_with_the_key() {
        let items = ["Edit", "Duplicate", "Delete", "Download"];
        let label = |s: &&str| (*s).to_owned();
        assert_eq!(menu_target(&items, None, "d", false, label), Some(1));
        assert_eq!(menu_target(&items, Some(1), "d", false, label), Some(2));
        assert_eq!(menu_target(&items, Some(2), "d", false, label), Some(3));
        assert_eq!(menu_target(&items, Some(3), "d", false, label), Some(1), "wraps back round");
        assert_eq!(menu_target(&items, Some(0), "D", false, label), Some(1), "the key's case does not matter");
        assert_eq!(menu_target(&items, Some(0), "z", false, label), None, "no match moves nothing");
        assert_eq!(menu_target(&items, Some(0), " ", false, label), None, "space is not type-ahead");
        assert_eq!(menu_target(&items, Some(0), "d", true, label), None, "Ctrl+D belongs to the browser");
        assert_eq!(menu_target(&items, Some(0), "Shift", false, label), None, "a named key is not a character");
    }

    /// Leading and trailing space in an item's label must not defeat it.
    #[test]
    fn menu_type_ahead_ignores_surrounding_whitespace() {
        let items = ["  Delete  "];
        assert_eq!(menu_target(&items, None, "d", false, |s: &&str| (*s).to_owned()), Some(0));
    }

    /// Every word must appear somewhere, in any order, anywhere in the text.
    #[test]
    fn the_palette_matches_every_word_anywhere() {
        assert!(matches_query("Profile settings account", &["set", "pro"]));
        assert!(matches_query("Profile settings", &[]), "an empty query shows everything");
        assert!(matches_query("Profile SETTINGS", &["settings"]), "case does not matter");
        assert!(!matches_query("Profile settings", &["set", "zzz"]));
    }

    /// A colour with no icon of its own still gets one.
    #[test]
    fn every_toast_colour_has_an_icon() {
        for (name, body) in TOAST_ICONS {
            assert_eq!(icon_for(name), body);
        }
        assert_eq!(icon_for("primary"), TOAST_ICONS[0].1, "a colour with no icon falls back to the neutral one");
        assert_eq!(icon_for(""), TOAST_ICONS[0].1);
        assert!(svg(icon_for("success")).starts_with("<svg class=\"nr-icon\""));
    }

    /// ARIA booleans are written out, not the attribute's presence.
    #[test]
    fn aria_booleans_are_the_words() {
        assert_eq!(aria(true), "true");
        assert_eq!(aria(false), "false");
    }
}
