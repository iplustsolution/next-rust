//! The Next Rust UI components' browser behaviour, in Rust.
//!
//! This is the same behaviour as [`next_rust_ui::UI_JS`] — the custom select,
//! the calendar, the password toggle, `on_press`, the drawer, tab panels,
//! modals, closable alerts, menu keys, the command palette, toasts, the
//! combobox list, tags, carousel buttons and the copy button — written
//! against [`next_rust_wasm`] instead of the DOM, so a page can carry a
//! logic-free shim and one shared `.wasm` module rather than 27 KB of
//! readable JavaScript.
//!
//! Which one an app serves is `[ui] script` in `next-rust.toml`. Both paths
//! are kept working; `"js"` is the default until this one has been proven in
//! every browser the kit supports.
//!
//! # What this layer may not do
//!
//! Everything here is an *enhancement*. Every component works with no script
//! at all — native `<dialog>`, `popover`, `<details>`, radio groups,
//! scroll-snap and `:has()` carry the zero-JavaScript path — and nothing here
//! may take that away. In particular it never hides a control before its
//! replacement exists, and it never takes a `name` off a field without
//! putting an equivalent one back in the same breath.
//!
//! # Why this is a crate of its own
//!
//! `next-rust-ui` renders markup on the server and is a `std` crate. This is
//! compiled into the module a browser downloads, where `std` costs about
//! 12 KB brotli on its own — more than every component here put together. So
//! the behaviour lives in a `no_std` crate beside the renderer rather than
//! inside it, and the two agree through the `data-nr-ui` values in
//! [`next_rust_ui::UI_WASM_KINDS`].
//!
//! [`next_rust_ui::UI_JS`]: https://docs.rs/next-rust-ui
//! [`next_rust_ui::UI_WASM_KINDS`]: https://docs.rs/next-rust-ui

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod controls;
pub mod data;
pub mod json;
pub mod misc;
pub mod overlays;
pub mod pop;
pub mod util;

use next_rust_wasm::event::MutationOpts;
use next_rust_wasm::{El, Event, document, html};

/// Component kinds `data-nr-ui` names, and what mounts each one.
///
/// The strings are the contract with the renderer: it writes them, and
/// nothing else in this crate decides what a component is.
fn mount_one(kind: &str, el: El) {
    match kind {
        "select" => controls::select(el),
        "datepicker" => data::date_picker(el),
        "avatar" => data::avatar(el),
        "tabs" => data::tabs(el),
        "modal" => overlays::modal(el),
        "dropdown" | "popover" => overlays::popover(el),
        "context" => overlays::context_menu(el),
        "command" => overlays::command(el),
        "toaster" => overlays::show_toaster(el),
        "combobox" => controls::combobox(el),
        "tags" => controls::tags(el),
        "upload" => misc::upload(el),
        "carousel" => misc::carousel(el),
        "banner" => misc::banner(el),
        "checkall" => data::check_all(el),
        // `alert`, `code`, `color`, `field`, `shell` and `slider` have no
        // mount: one delegated listener each serves every one of them on the
        // page, wherever it is and however it arrived.
        _ => {}
    }
}

/// Mount one `[data-nr-ui]` root, once.
///
/// The script keeps a `WeakSet` of what it has mounted. A module-wide set
/// would be a `static mut` here, and one that goes stale the moment a
/// client-side navigation replaces the page — so the mark goes on the
/// element instead, as an IDL property rather than an attribute, which keeps
/// it out of the markup and out of the release build's rename table.
fn mount(el: El) {
    let Some(kind) = el.attr("data-nr-ui") else { return };
    if el.prop_bool("nrUiMounted") {
        return;
    }
    el.set_prop_bool("nrUiMounted", true);
    mount_one(kind.as_str(), el);
}

/// Mount everything under `scope`, and `scope` itself when it is a component.
fn mount_all(scope: El) {
    if scope.matches("[data-nr-ui]") {
        mount(scope);
    }
    for el in scope.query_all("[data-nr-ui]").iter() {
        mount(el);
    }
}

/// Wire the whole kit. The shim calls this once, after instantiation, before
/// any island hydrates.
pub fn boot() {
    // The stylesheet uses this to hand behaviour that only works with the
    // module over to it: the zero-script path styles itself differently.
    html().class_toggle("nr-ui-js", true);
    pop::start();
    misc::press();
    controls::fields();
    overlays::shell();
    overlays::alerts();
    data::copy();
    overlays::toast_api();
    mount_all(document());
    // A client-side navigation swaps the page under us, and streamed content
    // arrives after it: mount whatever turns up.
    //
    // The whole document, not the event's target: a mutation callback is told
    // that something changed and never what — see
    // [`El::observe_mutation`] — so the only way to find what arrived is to
    // look. It fires once per batch, and `mount` is a property read per
    // component, so this is one query and a handful of reads per change.
    html().observe_mutation(MutationOpts::TREE, |_| mount_all(document()));
}

/// Whether an element is one the kit should leave alone because it is busy or
/// switched off. Shared by `on_press` and the field actions.
pub(crate) fn inactive(el: El) -> bool {
    el.matches(":disabled") || el.attr("aria-disabled").is_some_and(|v| v.eq_str("true")) || el.has_attr("data-loading")
}

/// `Enter` or `Space` on something that is not a button but acts like one.
pub(crate) fn activates(ev: Event) -> bool {
    ev.key_is("Enter") || ev.key_is(" ")
}

/// A plain printable character, the way type-ahead in a listbox or a menu
/// tests for one: exactly one character, and no modifier that would make it
/// the browser's key rather than the component's.
pub(crate) fn typed(ev: Event) -> Option<alloc::string::String> {
    if ev.ctrl() || ev.meta() || ev.alt() {
        return None;
    }
    let key = ev.key();
    let mut chars = key.as_str().chars();
    let (Some(first), None) = (chars.next(), chars.next()) else { return None };
    Some(alloc::string::String::from(first))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    /// Every kind the renderer writes is either mounted here or delegated.
    /// A kind that is neither renders markup nothing wires up: the component
    /// would simply not work, with no error anywhere.
    #[test]
    fn every_component_kind_is_accounted_for() {
        const DELEGATED: &[&str] = &["alert", "code", "color", "field", "press", "shell", "slider"];
        const MOUNTED_KINDS: &[&str] = &[
            "avatar",
            "banner",
            "carousel",
            "checkall",
            "combobox",
            "command",
            "context",
            "datepicker",
            "dropdown",
            "modal",
            "popover",
            "select",
            "tabs",
            "tags",
            "toaster",
            "upload",
        ];
        // The renderer's list, copied here rather than depended on: this
        // crate must not pull the `std` renderer into the browser build.
        const RENDERED: &[&str] = &[
            "alert",
            "avatar",
            "banner",
            "carousel",
            "checkall",
            "code",
            "color",
            "combobox",
            "command",
            "context",
            "datepicker",
            "dropdown",
            "field",
            "modal",
            "popover",
            "press",
            "select",
            "shell",
            "slider",
            "tabs",
            "tags",
            "toaster",
            "upload",
        ];
        for kind in RENDERED {
            assert!(
                MOUNTED_KINDS.contains(kind) || DELEGATED.contains(kind),
                "`data-nr-ui=\"{kind}\"` is neither mounted nor delegated"
            );
        }
        for kind in MOUNTED_KINDS.iter().chain(DELEGATED) {
            assert!(RENDERED.contains(kind), "nothing renders `data-nr-ui=\"{kind}\"` any more");
        }
        assert_eq!(RENDERED.len(), MOUNTED_KINDS.len() + DELEGATED.len());
        let mut sorted = vec![];
        sorted.extend_from_slice(RENDERED);
        let mut deduped = sorted.clone();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(sorted.len(), deduped.len(), "a kind is listed twice");
    }

    /// The keys the kit reads off an event, in one place, so a rename in
    /// the ABI shows up here rather than in the browser.
    #[test]
    fn the_mount_mark_is_not_an_attribute() {
        // An attribute would end up in the markup and in the release build's
        // rename table; a property is invisible to both.
        assert!(!"nrUiMounted".starts_with("data-"));
        assert!(!"nrUiMounted".contains('-'), "an IDL property name, not an attribute name");
    }
}
