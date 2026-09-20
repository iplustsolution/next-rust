//! Anchored popovers: the top layer, the placement fallback, and the rule
//! that at most one of them is open.
//!
//! The select's listbox, the combobox's list and the date picker's calendar
//! all hang off a field and must escape whatever clips them — a card, a
//! dialog, an `overflow: hidden` table. Where the browser has a top layer
//! they are shown in it, positioned by hand against their anchor and
//! followed while the page scrolls; where it does not, they stay in flow and
//! only flip with `data-placement` / `data-align`, which the stylesheet
//! already understands.
//!
//! # Why there is no registry here
//!
//! The JavaScript keeps two `Map`s: open popovers, and popovers following an
//! anchor. Both are process-wide mutable state, which in a `no_std` module
//! means a `static mut` and the `unsafe` to reach it. It is also state that
//! can go stale — a client-side navigation replaces the page and the map
//! still holds the elements that left with it.
//!
//! So the DOM is the registry instead. A component that opens marks its root
//! `data-open` — which it did anyway, for the stylesheet — and answers
//! [`CLOSE`]; a popover that wants to be followed says so with
//! `data-nr-follow`, naming the anchor. Closing everything is a query and an
//! event, and nothing outlives the element it describes.

use next_rust_wasm::event::ListenOpts;
use next_rust_wasm::{El, Event, Feature, Metric, document, on_document_opts, on_window_opts, supports, window};

use crate::util::px;
use next_rust_wasm::num::abs;

/// Distance between an anchor and its popover.
const GAP: f64 = 6.0;

/// Smallest distance to the viewport's edge.
const EDGE: f64 = 8.0;

/// The event a component answers by closing its popover.
pub const CLOSE: &str = "nr:close-popover";

/// Which anchor a followed popover is placed against.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anchor {
    /// The field's box, and as wide as it: a select's or a combobox's list.
    Field,
    /// The field's box, at the popover's own width: the calendar.
    Wrap,
    /// The button that opened it: a dropdown or a popover.
    Trigger,
}

impl Anchor {
    /// How the anchor is written into `data-nr-follow`.
    fn name(self) -> &'static str {
        match self {
            Anchor::Field => "field",
            Anchor::Wrap => "wrap",
            Anchor::Trigger => "trigger",
        }
    }

    /// Read it back.
    fn parse(name: &str) -> Option<Anchor> {
        match name {
            "field" => Some(Anchor::Field),
            "wrap" => Some(Anchor::Wrap),
            "trigger" => Some(Anchor::Trigger),
            _ => None,
        }
    }

    /// Whether the popover is made as wide as its anchor.
    fn match_width(self) -> bool {
        self == Anchor::Field
    }

    /// Find the anchor inside a component root.
    fn find(self, root: El) -> El {
        match self {
            Anchor::Field | Anchor::Wrap => root.query(".nr-field-wrap"),
            Anchor::Trigger => root.query("[popovertarget]"),
        }
    }
}

/// Whether popovers go into the browser's top layer.
pub fn top_layer() -> bool {
    supports(Feature::Popover)
}

/// The viewport, in CSS pixels.
pub fn viewport() -> (f64, f64) {
    let window = window().el();
    (window.metric(Metric::InnerWidth), window.metric(Metric::InnerHeight))
}

/// Place `popup` against `anchor`: below it, or above it when there is no
/// room below, and inside the viewport horizontally. With `match_width` the
/// popover is made as wide as its anchor, which is what a field's list wants.
pub fn position(popup: El, anchor: El, match_width: bool) {
    if popup.is_null() || anchor.is_null() {
        return;
    }
    let r = anchor.rect();
    let (inner_w, inner_h) = viewport();
    let bottom = r.y + r.h;
    if !top_layer() {
        // With no top layer the stylesheet positions the box, and all this
        // can do is tell it which way to flip.
        popup.remove_attr("data-placement");
        popup.remove_attr("data-align");
        let h = popup.metric(Metric::OffsetHeight);
        if bottom + h + GAP * 2.0 > inner_h && r.y - h - GAP * 2.0 > 0.0 {
            popup.set_attr("data-placement", "top");
        }
        let w = popup.metric(Metric::OffsetWidth);
        if r.x + w + EDGE > inner_w && r.x + r.w - w > 0.0 {
            popup.set_attr("data-align", "end");
        }
        return;
    }
    let width = if match_width { r.w + 4.0 } else { popup.metric(Metric::OffsetWidth) };
    // A field has a 2px border its list lines up with; a menu lines up with
    // its trigger, and `data-nr-align="end"` hangs it off the right edge.
    let align = popup.attr("data-nr-align");
    let start = if align.as_ref().is_some_and(|a| a.eq_str("end")) {
        r.x + r.w - width
    } else {
        r.x - if align.is_some() { 0.0 } else { 2.0 }
    };
    let left = start.min(inner_w - width - EDGE).max(EDGE);
    let h = popup.metric(Metric::OffsetHeight);
    let below = bottom + GAP + h <= inner_h - EDGE || r.y - GAP - h < EDGE;
    popup.set_attr("data-placement", if below { "bottom" } else { "top" });
    popup.style_set("position", "fixed");
    popup.style_set("inset", "auto");
    popup.style_set("margin", "0");
    popup.style_set("left", &px(left));
    popup.style_set("top", &px(if below { bottom + GAP } else { r.y - GAP - h }));
    // An empty value clears the width a previous open set, which matters
    // when one list is reused for a narrower field.
    let width = if match_width { px(width) } else { alloc::string::String::new() };
    popup.style_set("width", &width);
}

/// Show `popup`, anchored to the named anchor of its component root.
pub fn show(popup: El, root: El, anchor: Anchor) {
    let target = anchor.find(root);
    popup.set_prop_bool("hidden", false);
    if top_layer() {
        // The kit's lists and calendars are ordinary boxes in the markup, so
        // that a page with no script still shows them. `showPopover()` throws
        // on an element that is not a popover — and the host swallows the
        // throw — so the box becomes one here, the moment it is shown.
        // "manual" rather than "auto": light dismiss would close the list on
        // the same press that opened it, and `pop::start` already closes it.
        popup.set_prop_str("popover", "manual");
        popup.show_popover();
        // Followed from here until it closes: a fixed box would otherwise
        // stay where the page used to be after a scroll.
        popup.set_attr("data-nr-follow", anchor.name());
    }
    position(popup, target, anchor.match_width());
}

/// Hide `popup`.
pub fn hide(popup: El) {
    if top_layer() && popup.popover_open() {
        popup.hide_popover();
    }
    popup.remove_attr("data-nr-follow");
    popup.set_prop_bool("hidden", true);
}

/// Follow an anchor without having been shown through [`show`] — a popover
/// the *browser* opened, from a native `popovertarget`.
pub fn follow_anchor(popup: El, anchor: Anchor) {
    popup.set_attr("data-nr-follow", anchor.name());
}

/// Stop following an anchor.
pub fn forget_anchor(popup: El) {
    popup.remove_attr("data-nr-follow");
}

/// Ask every open popover but `except`'s to close.
///
/// One at a time is the rule, and it is what makes a second dropdown's click
/// feel like a move rather than a stack. A component answers by listening for
/// [`CLOSE`] on its own root.
pub fn close_all(except: El) {
    for root in document().query_all("[data-nr-ui][data-open]").iter() {
        if !root.val().same(except.val()) {
            root.dispatch(CLOSE, "null");
        }
    }
}

/// Re-place every followed popover.
fn follow() {
    for popup in document().query_all("[data-nr-follow]").iter() {
        let Some(anchor) = popup.attr("data-nr-follow").and_then(|name| Anchor::parse(name.as_str())) else {
            continue;
        };
        let root = popup.closest("[data-nr-ui]");
        if root.is_some() {
            position(popup, anchor.find(root), anchor.match_width());
        }
    }
}

/// Install the document-level listeners the kit's popovers need. Called once.
pub fn start() {
    // A press anywhere outside an open popover closes it. Capture, so a
    // handler that stops the event cannot keep a menu open behind a click.
    on_document_opts("pointerdown", ListenOpts { capture: true, passive: false, once: false }, |ev: Event| {
        let target = ev.target();
        for root in document().query_all("[data-nr-ui][data-open]").iter() {
            if !root.contains(target) {
                root.dispatch(CLOSE, "null");
            }
        }
    });
    on_window_opts("scroll", ListenOpts { capture: true, passive: true, once: false }, |_| follow());
    on_window_opts("resize", ListenOpts { capture: false, passive: true, once: false }, |_| follow());
}

/// How far a popover stays from the viewport's edge, for anything placed by
/// hand rather than against an element — the context menu is.
pub fn edge() -> f64 {
    EDGE
}

/// Hold a coordinate inside the viewport, for a popover placed at a point.
pub fn inside(value: f64, size: f64, limit: f64) -> f64 {
    // `abs` keeps an element with no layout yet — a negative or nonsense
    // size — from pushing the popover off the opposite edge.
    value.min(limit - abs(size) - EDGE).max(EDGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_anchor_round_trips_through_the_attribute() {
        for anchor in [Anchor::Field, Anchor::Wrap, Anchor::Trigger] {
            assert_eq!(Anchor::parse(anchor.name()), Some(anchor));
        }
        assert_eq!(Anchor::parse("nonsense"), None);
        assert_eq!(Anchor::parse(""), None);
    }

    /// Only a field's list takes its anchor's width; the calendar and a menu
    /// keep their own.
    #[test]
    fn only_a_fields_list_matches_its_anchors_width() {
        assert!(Anchor::Field.match_width());
        assert!(!Anchor::Wrap.match_width());
        assert!(!Anchor::Trigger.match_width());
    }

    #[test]
    fn a_point_is_held_inside_the_viewport() {
        assert_eq!(inside(500.0, 200.0, 1000.0), 500.0, "room to spare");
        assert_eq!(inside(900.0, 200.0, 1000.0), 792.0, "pushed back from the right edge");
        assert_eq!(inside(0.0, 200.0, 1000.0), EDGE, "held off the left edge");
        assert_eq!(inside(10.0, 2_000.0, 1000.0), EDGE, "too big to fit: the near edge wins");
        assert_eq!(edge(), EDGE);
    }
}
