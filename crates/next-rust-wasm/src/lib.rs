//! The browser-side API for Next Rust islands.
//!
//! An island's behaviour is Rust, compiled to one WebAssembly module the whole
//! site shares. This crate is what that Rust calls: elements, events, async,
//! storage, formatting — everything an island needs from a browser, and
//! nothing that would drag `std` in behind it.
//!
//! ```ignore
//! #![no_std]
//! extern crate alloc;
//!
//! use next_rust_wasm::prelude::*;
//!
//! pub fn hydrate(cx: Ctx) {
//!     let scope = cx.root.closest("[data-auth-scope]");
//!     let panel = scope.query("[data-auth-panel=email]");
//!     scope.query("[data-auth-tabs]").on("click", move |ev| {
//!         ev.prevent_default();
//!         panel.set_prop_bool("inert", false);
//!     });
//! }
//!
//! next_rust_wasm::islands! {
//!     hydrate_AuthSwitch => crate::hydrate,
//! }
//! ```
//!
//! # What this buys, and what it does not
//!
//! It buys island logic in the same language as the rest of the application,
//! one dependency graph, one type system across the network boundary, and a
//! payload that is far harder to read than minified JavaScript.
//!
//! It does **not** buy secrecy, and no amount of work here could. The module
//! is served to the browser: it can be downloaded and disassembled, and every
//! string literal in it — selectors, class names, URLs, user-facing copy —
//! sits in the data section in plain text. Props are public. This is
//! obfuscation, not a security boundary. Anything that must stay secret stays
//! on the server.
//!
//! A JavaScript shim also always remains. `WebAssembly.instantiate`, the
//! import object and `addEventListener` can only be reached from JavaScript.
//! What the shim does not contain is behaviour: it is a switch statement over
//! the opcode table in [`host`], a handle table and a resume queue.
//!
//! # Size is the design constraint
//!
//! Every byte here is downloaded by a browser, so the crate is `no_std`, has
//! no dependencies, and will not grow one. A single `std` dependency anywhere
//! in an island's graph costs more than everything this crate replaces:
//! measured on a skeleton module with no behaviour at all, `std` is 13 KB
//! brotli against 1.4 KB for the same skeleton `no_std`, almost all of it
//! `dlmalloc`, the panic machinery and `core::fmt`.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// Host-side tests only, for the call recorder in [`host`]: cargo runs tests on
// several threads, and a `thread_local!` is the only way to record what one of
// them passed the host without the others racing it. Never in a browser build.
#[cfg(test)]
extern crate std;

pub mod dom;
pub mod encode;
pub mod event;
pub mod global;
pub mod heap;
pub mod host;
pub mod macros;
pub mod net;
pub mod num;
pub mod props;
pub mod task;
pub mod text;

pub use dom::{
    El, ElList, Feature, File, FileList, JsType, Keys, Metric, Namespace, Rect, Val, active, body, by_id, by_name,
    confirm, css_supports, document, document_hidden, head, html, media_matches, query, query_all, query_in, scroll_to,
    scroll_y, supports, user_agent, window,
};
pub use event::{
    Event, Listener, MODIFIED, MutationOpts, NativeEvent, Observer, Timer, on_document, on_document_opts,
    on_media_change, on_window, on_window_opts, set_interval, set_timeout,
};
pub use global::{
    cookie, date_now, error, history, install_forwarder, intl, location, log, navigate, navigate_replace, now,
    prefetch, refresh, storage, today,
};
pub use heap::Heap;
pub use macros::{Ctx, Inst, Slots};
pub use net::{
    Action, ActionError, Error, Request, Response, Sse, SseEvent, SseStop, action, action_text, action_val, fetch,
    idle, raf, sse, sse_post, timeout,
};
pub use num::{finite_number, number_to_string};
pub use props::{Key, List, Props, Value, key};
pub use task::spawn;
pub use text::Str;

/// Everything an island file usually wants.
pub mod prelude {
    pub use crate::dom::{El, ElList, File, Metric, Rect, Val};
    pub use crate::event::{Event, ListenOpts, MutationOpts, NativeEvent, on_document, on_window, set_timeout};
    pub use crate::global::{history, intl, location, log, navigate, refresh, storage};
    pub use crate::macros::{Ctx, Inst};
    pub use crate::net::{Action, ActionError, Request, Response, action, fetch, raf, sse, timeout};
    pub use crate::num;
    pub use crate::props::{Props, Value};
    pub use crate::task::spawn;
    pub use crate::text::Str;
    pub use crate::{document, k, query, query_all, state};
}

// --- what the generated exports call -----------------------------------------
//
// The `islands!` macro emits thin `extern "C"` wrappers around these. Keeping
// the bodies here rather than in the macro means the macro expands to almost
// nothing and this code is compiled, linted and tested normally.

/// Reserve linear memory for the shim to write into.
#[doc(hidden)]
pub fn __alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return core::ptr::NonNull::<u8>::dangling().as_ptr();
    }
    let Ok(layout) = alloc::alloc::Layout::from_size_align(len, 8) else {
        return core::ptr::null_mut();
    };
    // SAFETY: the layout is non-zero-sized, checked above.
    unsafe { alloc::alloc::alloc(layout) }
}

///
/// # Safety
/// `at` must be a pointer `__alloc` returned with the same `len`.
#[doc(hidden)]
pub unsafe fn __dealloc(at: *mut u8, len: usize) {
    if at.is_null() || len == 0 {
        return;
    }
    let Ok(layout) = alloc::alloc::Layout::from_size_align(len, 8) else {
        return;
    };
    // SAFETY: the caller guarantees the pointer and length came from
    // `__alloc`, which is the only way the shim is allowed to call this.
    unsafe { alloc::alloc::dealloc(at, layout) }
}

#[doc(hidden)]
pub fn __on_event(cb: u32, event: u32) {
    event::dispatch(cb, event);
}

/// An awaited host call produced a value. A resume for a token nothing is
/// waiting on is ignored rather than fatal: a future that was dropped while
/// its work was in flight is the normal case, not an error.
#[doc(hidden)]
pub fn __on_resume(token: u32, ptr: u32, len: u32) {
    task::on_resume(token, ptr, len);
}

/// Hydrate one island: wrap the raw arguments the shim passes into a [`Ctx`]
/// and hand it to the island's function.
///
/// The instance number is handed out here, so every island gets a distinct one
/// even when the same island hydrates twice on one page.
///
/// # Safety
/// `props` must be null, or a block of `len` bytes the shim allocated with
/// `__alloc` and keeps alive for the life of the instance.
#[doc(hidden)]
pub unsafe fn __hydrate(root: u32, props: *const u8, len: usize, f: fn(Ctx)) -> u32 {
    static mut NEXT: u32 = 0;
    // Single-threaded; see `state!`.
    let inst = unsafe {
        let next = &raw mut NEXT;
        *next += 1;
        *next
    };
    // SAFETY: the shim allocated this block with `alloc` and keeps it alive
    // for the life of the instance. A null pointer or zero length is the
    // documented "no props" case.
    let blob: &'static [u8] =
        if props.is_null() || len == 0 { &[] } else { unsafe { core::slice::from_raw_parts(props, len) } };
    f(Ctx { root: El::from_handle(root), inst: Inst(inst), props: Props::new(blob) });
    inst
}

impl El {
    /// Wrap a raw handle the shim passed in. Only the generated glue should
    /// need this.
    #[doc(hidden)]
    pub fn from_handle(handle: u32) -> El {
        El(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocating_zero_bytes_is_not_a_null_pointer() {
        let at = __alloc(0);
        assert!(!at.is_null(), "a zero-length props blob must not read as a failure");
        unsafe {
            __dealloc(at, 0);
            __dealloc(core::ptr::null_mut(), 16);
        }
    }

    #[test]
    fn memory_handed_to_the_shim_is_writable_and_distinct() {
        let a = __alloc(32);
        let b = __alloc(32);
        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a, b);
        // SAFETY: both blocks are 32 bytes we just reserved.
        unsafe {
            core::ptr::write_bytes(a, 0xAB, 32);
            core::ptr::write_bytes(b, 0xCD, 32);
            assert_eq!(*a, 0xAB);
            assert_eq!(*b, 0xCD);
        }
        unsafe {
            __dealloc(a, 32);
            __dealloc(b, 32);
        }
    }

    /// The whole point of the instance table: hydrating twice must produce two
    /// identities, never one.
    #[test]
    fn every_hydration_gets_its_own_instance() {
        fn island(_: Ctx) {}
        // SAFETY: a null props pointer is the documented "no props" case.
        let (first, second) =
            unsafe { (__hydrate(0, core::ptr::null(), 0, island), __hydrate(0, core::ptr::null(), 0, island)) };
        assert_ne!(first, second);
        assert_ne!(first, 0, "zero stays available as `no instance`");
    }
}
