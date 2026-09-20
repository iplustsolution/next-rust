//! Listeners, the event being handled, timers and observers.
//!
//! A listener is a Rust closure. That is the whole answer to the problem the
//! first island had: its state lived in a `static`, so a second instance of the
//! same island on one page would have overwritten the first. A closure captures
//! per-instance state by construction, and two instances capture two different
//! sets of it.
//!
//! The closures live in a table here and the browser only ever holds their
//! index, because a JavaScript function cannot hold a Rust one. The shim calls
//! `on_event(index, event)` and [`dispatch`] finds the closure again.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::dom::{El, Val};
use crate::host::{self, call, call_s0, call0, call1, call2, op, s};
use crate::text::Str;

/// The event currently being dispatched.
///
/// It is a handle like any other, so anything the browser exposes on an event
/// is one property read away, including the ones the original flag-based ABI
/// could not tell apart — `metaKey` and `ctrlKey` are separate here, which is
/// what distinguishes a Command-K from a Control-K.
///
/// Valid only for the duration of the handler: the shim releases the handle
/// when the call returns, so storing one keeps a number that will name
/// something else later.
#[derive(Clone, Copy, Debug)]
pub struct Event(pub(crate) u32);

/// How a listener is registered.
#[derive(Clone, Copy, Debug, Default)]
pub struct ListenOpts {
    /// Promises never to call `preventDefault`, which lets the browser scroll
    /// without waiting for the handler.
    pub passive: bool,
    /// Run during the capture phase, before the event reaches its target.
    /// Ordering-critical handlers — a confirm gate that has to beat the
    /// framework's own submit handling — need this.
    pub capture: bool,
    /// Remove the listener after it fires once.
    pub once: bool,
}

impl ListenOpts {
    pub const PASSIVE: ListenOpts = ListenOpts { passive: true, capture: false, once: false };
    pub const CAPTURE: ListenOpts = ListenOpts { passive: false, capture: true, once: false };
    pub const ONCE: ListenOpts = ListenOpts { passive: false, capture: false, once: true };

    fn bits(self) -> u32 {
        (self.passive as u32) | ((self.capture as u32) << 1) | ((self.once as u32) << 2)
    }
}

/// A registered listener, so it can be removed again.
#[derive(Clone, Copy, Debug)]
pub struct Listener(u32);

impl Listener {
    pub fn off(self) {
        if self.0 != 0 {
            call1(op::OFF, self.0);
        }
    }
}

/// A `setTimeout` or `setInterval`.
#[derive(Clone, Copy, Debug)]
pub struct Timer(u32);

impl Timer {
    pub fn clear(self) {
        if self.0 != 0 {
            call1(op::CLEAR_TIMER, self.0);
        }
    }
}

/// An `IntersectionObserver`, `ResizeObserver` or `MutationObserver`.
#[derive(Clone, Copy, Debug)]
pub struct Observer(u32);

impl Observer {
    pub fn stop(self) {
        if self.0 != 0 {
            call1(op::UNOBSERVE, self.0);
        }
    }
}

/// What a `MutationObserver` should watch.
#[derive(Clone, Copy, Debug, Default)]
pub struct MutationOpts {
    pub attributes: bool,
    pub children: bool,
    pub subtree: bool,
    pub text: bool,
}

impl MutationOpts {
    /// What a mount hook wants: anything appearing anywhere below the root.
    pub const TREE: MutationOpts = MutationOpts { attributes: false, children: true, subtree: true, text: false };

    fn bits(self) -> u32 {
        (self.attributes as u32)
            | ((self.children as u32) << 1)
            | ((self.subtree as u32) << 2)
            | ((self.text as u32) << 3)
    }
}

// --- the callback table ------------------------------------------------------

/// Registered closures. The index is what the browser holds.
///
/// Slot zero is never handed out, so a callback id of `0` can keep meaning
/// "none" the way every other handle does. A listener's slot is never
/// reclaimed, even when the listener is removed: reusing it would let a late
/// event from a removed listener reach whatever closure landed in its place.
/// The cost is a pointer per listener ever registered, which for a page of
/// islands is tens of pointers.
///
/// A one-shot — a `setTimeout` — is different, and is released when it fires.
/// The flag beside each closure is which kind it is.
type Callback = Box<dyn FnMut(Event)>;

static mut CALLBACKS: Vec<Option<(Callback, bool)>> = Vec::new();

/// Wasm islands run on the browser's single thread, which is what makes these
/// accesses sound. A raw pointer rather than `&mut CALLBACKS` because taking a
/// reference to a `static mut` is what `static_mut_refs` forbids.
#[allow(clippy::deref_addrof)]
fn callbacks() -> &'static mut Vec<Option<(Callback, bool)>> {
    unsafe { &mut *(&raw mut CALLBACKS) }
}

fn register(f: impl FnMut(Event) + 'static) -> u32 {
    register_with(f, false)
}

/// A callback that runs at most once and then releases its slot.
///
/// The distinction matters more than it looks. A listener's closure has to
/// outlive every event it will ever see, so its slot is permanent. A
/// `setTimeout` callback fires once and is then garbage — and an island that
/// schedules a step every 22 milliseconds, which a typing animation does,
/// would otherwise leak a boxed closure and a slot per tick for as long as
/// the page is open.
fn register_once(f: impl FnMut(Event) + 'static) -> u32 {
    register_with(f, true)
}

fn register_with(f: impl FnMut(Event) + 'static, once: bool) -> u32 {
    let table = callbacks();
    if table.is_empty() {
        table.push(None);
    }
    table.push(Some((Box::new(f), once)));
    (table.len() - 1) as u32
}

/// Run the closure a listener registered. The shim calls this through the
/// module's `on_event` export.
///
/// The closure is taken out of the table for the duration of the call and put
/// back afterwards, so a handler that fires itself — a click that dispatches a
/// click — borrows nothing twice. A re-entrant call finds an empty slot and
/// does nothing, which is a dropped event rather than a panic; that trade is
/// deliberate and it is the only place the crate silently swallows something.
pub fn dispatch(cb: u32, event: u32) {
    let table = callbacks();
    let Some(slot) = table.get_mut(cb as usize) else { return };
    let Some((mut f, once)) = slot.take() else { return };
    f(Event(event));
    if once {
        // Dropping `f` here is the point: a one-shot's closure is released
        // rather than kept alive by a table nothing ever prunes.
        return;
    }
    if let Some(slot) = callbacks().get_mut(cb as usize) {
        *slot = Some((f, false));
    }
}

// --- registering listeners ---------------------------------------------------

impl El {
    /// Listen for an event on this element.
    pub fn on(self, event: &str, f: impl FnMut(Event) + 'static) -> Listener {
        self.on_opts(event, ListenOpts::default(), f)
    }

    pub fn on_opts(self, event: &str, opts: ListenOpts, f: impl FnMut(Event) + 'static) -> Listener {
        if self.is_null() {
            return Listener(0);
        }
        let cb = register(f);
        let (p, l) = s(event);
        Listener(call(op::ON_OPTS, self.0, p, l, cb, opts.bits()))
    }

    /// One listener on this element for a whole family of targets below it.
    ///
    /// The handler runs only when the event's target has an ancestor matching
    /// `selector` within this element, and [`Event::current`] is that ancestor
    /// — so a list of a hundred rows costs one listener, and rows added later
    /// work without re-registering anything.
    pub fn on_delegate(self, selector: &str, event: &str, f: impl FnMut(Event) + 'static) -> Listener {
        if self.is_null() {
            return Listener(0);
        }
        let cb = register(f);
        let (sp, sl) = s(selector);
        call(op::ARG, 0, sp, sl, 0, 0);
        let (p, l) = s(event);
        Listener(call(op::ON_DELEGATE, self.0, p, l, cb, 0))
    }

    /// Fire a `CustomEvent` that bubbles.
    ///
    /// `detail` is JSON, and the page receives the *parsed* value: a listener
    /// written against the JavaScript components reads `e.detail.key`, not a
    /// string it has to parse. Pass `"null"` for no detail. Text that is not
    /// JSON arrives as itself, so a bare word is still a usable detail.
    pub fn dispatch(self, name: &str, detail: &str) {
        crate::host::call_ss(op::DISPATCH, self.0, name, detail);
    }

    /// Fire a real `input`, `change`, `submit`, `click`, `focus`, `blur` or
    /// `reset` event, so listeners that expect the native kind see one.
    pub fn dispatch_native(self, kind: NativeEvent, bubbles: bool) {
        call(op::DISPATCH_NATIVE, self.0, kind as u32, bubbles as u32, 0, 0);
    }
}

/// The native events an island fires by hand after changing a value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum NativeEvent {
    Input = 0,
    Change = 1,
    Submit = 2,
    Click = 3,
    Focus = 4,
    Blur = 5,
    Reset = 6,
}

pub fn on_window(event: &str, f: impl FnMut(Event) + 'static) -> Listener {
    on_window_opts(event, ListenOpts::default(), f)
}

pub fn on_window_opts(event: &str, opts: ListenOpts, f: impl FnMut(Event) + 'static) -> Listener {
    let cb = register(f);
    let (p, l) = s(event);
    Listener(call(op::LISTEN_WINDOW, p, l, cb, opts.bits(), 0))
}

pub fn on_document(event: &str, f: impl FnMut(Event) + 'static) -> Listener {
    on_document_opts(event, ListenOpts::default(), f)
}

pub fn on_document_opts(event: &str, opts: ListenOpts, f: impl FnMut(Event) + 'static) -> Listener {
    let cb = register(f);
    let (p, l) = s(event);
    Listener(call(op::LISTEN_DOC, p, l, cb, opts.bits(), 0))
}

// --- reading the event -------------------------------------------------------

/// A modifier key was held, or the pointer button was not the primary one:
/// the browser should be left to do its own thing.
pub const MODIFIED: u32 = 1;

impl Event {
    /// The untyped handle, for anything this type does not name.
    pub fn val(self) -> Val {
        Val(self.0)
    }

    pub fn target(self) -> El {
        if self.0 == 0 { El(call0(op::EVT_TARGET)) } else { self.val().get("target").el() }
    }

    /// The element the listener is on, or — for a delegated listener — the
    /// element that matched the selector.
    pub fn current(self) -> El {
        El(call0(op::EVT_CURRENT))
    }

    pub fn closest(self, selector: &str) -> El {
        El(call_s0(op::EVT_CLOSEST, selector))
    }

    pub fn related_target(self) -> El {
        self.val().get("relatedTarget").el()
    }

    /// Which button submitted a form. There is no other way to tell a
    /// "previous", "next" and "submit" button on one form apart.
    pub fn submitter(self) -> El {
        self.val().get("submitter").el()
    }

    pub fn key(self) -> Str {
        Str::new(host::take(call0(op::EVT_KEY)).unwrap_or_default())
    }

    /// Compare `event.key` without copying it into linear memory.
    pub fn key_is(self, key: &str) -> bool {
        call_s0(op::EVT_KEY_IS, key) != 0
    }

    pub fn kind(self) -> Str {
        self.val().get("type").str()
    }

    pub fn detail(self) -> Str {
        Str::new(host::take(call0(op::EVT_DETAIL)).unwrap_or_default())
    }

    /// The value of the event's target, for an input or change event.
    pub fn value(self) -> Str {
        self.target().value()
    }

    pub fn checked(self) -> bool {
        self.target().prop_bool("checked")
    }

    pub fn meta(self) -> bool {
        self.val().get("metaKey").truthy()
    }

    pub fn ctrl(self) -> bool {
        self.val().get("ctrlKey").truthy()
    }

    pub fn shift(self) -> bool {
        self.val().get("shiftKey").truthy()
    }

    pub fn alt(self) -> bool {
        self.val().get("altKey").truthy()
    }

    pub fn button(self) -> i32 {
        self.val().get("button").num().unwrap_or(0.0) as i32
    }

    pub fn client_x(self) -> f64 {
        self.val().get("clientX").num().unwrap_or(0.0)
    }

    pub fn client_y(self) -> f64 {
        self.val().get("clientY").num().unwrap_or(0.0)
    }

    pub fn pointer_type(self) -> Str {
        self.val().get("pointerType").str()
    }

    /// `beforetoggle` and `toggle` carry the state the element is moving to.
    pub fn new_state(self) -> Str {
        self.val().get("newState").str()
    }

    pub fn is_composing(self) -> bool {
        self.val().get("isComposing").truthy()
    }

    /// The modifier and button flags OR-ed together, as the original ABI gave
    /// them. Kept because it is one call rather than four; use [`Event::meta`]
    /// and friends when the difference between the modifiers matters.
    pub fn flags(self) -> u32 {
        call0(op::EVT_FLAGS)
    }

    pub fn modified(self) -> bool {
        self.flags() & MODIFIED != 0
    }

    pub fn prevent_default(self) {
        call0(op::EVT_PREVENT);
    }

    pub fn stop_propagation(self) {
        call0(op::EVT_STOP);
    }

    /// Stop other listeners **on this same node** from running too.
    ///
    /// The difference from [`Event::stop_propagation`] is load-bearing where
    /// an island and the framework's own form runtime both listen on one form:
    /// only this one keeps the form from being submitted twice.
    pub fn stop_immediate_propagation(self) {
        call0(op::EVT_STOP_IMMEDIATE);
    }
}

// --- timers and observers ----------------------------------------------------

pub fn set_timeout(ms: u32, f: impl FnMut(Event) + 'static) -> Timer {
    Timer(call2(op::SET_TIMEOUT, ms, register_once(f)))
}

pub fn set_interval(ms: u32, f: impl FnMut(Event) + 'static) -> Timer {
    Timer(call2(op::SET_INTERVAL, ms, register(f)))
}

impl El {
    /// Watch for the element crossing a visibility threshold, given as a
    /// percentage so no float has to cross the boundary. In the callback,
    /// `ev.flags() & 1` is whether it is now intersecting.
    pub fn observe_intersection(self, threshold_pct: u32, f: impl FnMut(Event) + 'static) -> Observer {
        Observer(call(op::OBSERVE, 0, self.0, register(f), threshold_pct, 0))
    }

    pub fn observe_resize(self, f: impl FnMut(Event) + 'static) -> Observer {
        Observer(call(op::OBSERVE, 1, self.0, register(f), 0, 0))
    }

    /// Watch the tree for changes.
    ///
    /// The callback is told *that* something changed, never what: a mutation
    /// record is a list of added and removed nodes, and one event value cannot
    /// carry a list. [`Event::target`] and [`Event::current`] are therefore
    /// both [`El::NULL`] here — unlike in [`El::observe_resize`] and
    /// [`El::observe_intersection`], where the entry's element *is* the
    /// current target — so a callback that wants to know what arrived has to
    /// look for it, from this element down.
    ///
    /// It also fires once per batch rather than once per node, so that look is
    /// one query per microtask, not one per element.
    pub fn observe_mutation(self, opts: MutationOpts, f: impl FnMut(Event) + 'static) -> Observer {
        Observer(call(op::OBSERVE, 2, self.0, register(f), opts.bits(), 0))
    }
}

/// Watch a media query for changes. The current answer is
/// [`crate::media_matches`]; this is the notification.
pub fn on_media_change(query: &str, f: impl FnMut(Event) + 'static) -> Listener {
    let cb = register(f);
    let (p, l) = s(query);
    Listener(call(op::MEDIA_LISTEN, p, l, cb, 0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    extern crate std;
    use std::rc::Rc;
    use std::sync::{Mutex, MutexGuard};

    /// The callback table is a `static mut`, which is sound in the browser
    /// because there is one thread there — and is a data race here, because
    /// the test harness runs these on several. Every test that registers a
    /// callback takes this first, so they take turns the way the browser
    /// would. Without it `a_one_shot_releases_its_slot_and_a_listener_keeps_it`
    /// fails whenever another test grows the table between its two lines.
    static TABLE: Mutex<()> = Mutex::new(());

    /// Ignores a poisoned lock: a failed assertion in one test must not turn
    /// every other one into a panic about the mutex.
    fn table() -> MutexGuard<'static, ()> {
        TABLE.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn option_bits_are_stable() {
        assert_eq!(ListenOpts::default().bits(), 0);
        assert_eq!(ListenOpts::PASSIVE.bits(), 1);
        assert_eq!(ListenOpts::CAPTURE.bits(), 2);
        assert_eq!(ListenOpts::ONCE.bits(), 4);
        assert_eq!(ListenOpts { passive: true, capture: true, once: true }.bits(), 7);
        assert_eq!(MutationOpts::TREE.bits(), 6);
        assert_eq!(MutationOpts { attributes: true, children: true, subtree: true, text: true }.bits(), 15);
    }

    /// The problem this whole design exists to solve: two instances of one
    /// island must not share state. Each closure captures its own.
    #[test]
    fn two_closures_keep_separate_state() {
        let _guard = table();
        let first = Rc::new(Cell::new(0));
        let second = Rc::new(Cell::new(0));
        let a = {
            let first = first.clone();
            register(move |_| first.set(first.get() + 1))
        };
        let b = {
            let second = second.clone();
            register(move |_| second.set(second.get() + 10))
        };
        assert_ne!(a, b);
        assert_ne!(a, 0, "slot zero stays reserved for `no callback`");
        dispatch(a, 0);
        dispatch(a, 0);
        dispatch(b, 0);
        assert_eq!(first.get(), 2);
        assert_eq!(second.get(), 10);
    }

    /// A one-shot releases its closure when it fires; a listener keeps it.
    /// An island that schedules a step every 22 milliseconds would otherwise
    /// leak a boxed closure per tick for as long as the page is open.
    #[test]
    fn a_one_shot_releases_its_slot_and_a_listener_keeps_it() {
        let _guard = table();
        let runs = Rc::new(Cell::new(0));
        let once = {
            let runs = runs.clone();
            register_once(move |_| runs.set(runs.get() + 1))
        };
        dispatch(once, 0);
        assert_eq!(runs.get(), 1);
        // Firing again finds an empty slot: the closure is gone, not aliased.
        dispatch(once, 0);
        assert_eq!(runs.get(), 1, "a one-shot must not run twice");
        assert!(callbacks()[once as usize].is_none(), "the slot must be released");

        let kept = {
            let runs = runs.clone();
            register(move |_| runs.set(runs.get() + 1))
        };
        dispatch(kept, 0);
        dispatch(kept, 0);
        assert_eq!(runs.get(), 3, "a listener keeps firing");
        assert!(callbacks()[kept as usize].is_some());
    }

    #[test]
    fn dispatching_an_unknown_callback_does_nothing() {
        let _guard = table();
        dispatch(0, 0);
        dispatch(9_999, 0);
    }

    /// A handler that fires itself finds its own slot empty and is skipped,
    /// rather than aliasing the closure it is already inside.
    #[test]
    fn a_reentrant_handler_is_dropped_not_aliased() {
        let _guard = table();
        let runs = Rc::new(Cell::new(0));
        let id = Rc::new(Cell::new(0u32));
        let cb = {
            let runs = runs.clone();
            let id = id.clone();
            register(move |_| {
                runs.set(runs.get() + 1);
                if runs.get() < 3 {
                    dispatch(id.get(), 0);
                }
            })
        };
        id.set(cb);
        dispatch(cb, 0);
        assert_eq!(runs.get(), 1, "the nested dispatch must not run the closure again");
        // And the slot is restored, so the next real event still works.
        dispatch(cb, 0);
        assert_eq!(runs.get(), 2);
    }
}
