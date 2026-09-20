//! `thread_local!` for a place that has no threads.
//!
//! The browser runs this module on one thread and `wasm32-unknown-unknown`
//! has no thread support at all, so the runtime's state can live in ordinary
//! statics. It cannot use `thread_local!` for them, because that lives in
//! `std` and `std` costs about eleven kilobytes of brotli here — twice the
//! whole JavaScript runtime this crate replaces.
//!
//! [`crate::local!`] takes `thread_local!`'s syntax and expands to `std`'s macro off
//! the browser, where the tests run on real threads and need real thread
//! locals, and to `Local` on wasm, where it is a plain static. Call sites
//! are identical either way: `STATE.with(|s| s.borrow_mut().…)`.

/// State that behaves like a thread local, wherever it is compiled.
///
/// ```ignore
/// crate::local! {
///     static PAGES: RefCell<BTreeMap<String, Page>> = RefCell::new(BTreeMap::new());
/// }
/// PAGES.with(|pages| pages.borrow_mut().clear());
/// ```
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! local {
    ($(#[$meta:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr; $($rest:tt)*) => {
        $(#[$meta])* $vis static $name: $crate::cell::Local<$ty> = $crate::cell::Local::new(|| $init);
        $crate::local! { $($rest)* }
    };
    () => {};
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! local {
    ($($body:tt)*) => { std::thread_local! { $($body)* } };
}

/// A static that is initialised the first time it is used, with
/// `thread_local!`'s `with` interface.
///
/// It is `Sync` because `wasm32-unknown-unknown` is single threaded: there is
/// no second thread that could observe the cell, and the crate is only
/// compiled for that target without `std`.
#[cfg(target_arch = "wasm32")]
pub struct Local<T: 'static> {
    slot: core::cell::UnsafeCell<Option<T>>,
    init: fn() -> T,
}

// SAFETY: `wasm32-unknown-unknown` has no threads, so the cell is never shared.
#[cfg(target_arch = "wasm32")]
unsafe impl<T> Sync for Local<T> {}

#[cfg(target_arch = "wasm32")]
impl<T: 'static> Local<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self { slot: core::cell::UnsafeCell::new(None), init }
    }

    /// Borrow the value, initialising it first if this is its first use.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        // SAFETY: single threaded, and the exclusive borrow ends before `f`
        // runs, so a handler that reaches this static again sees only shared
        // borrows. `init` is a constructor and cannot reach the slot itself.
        let empty = unsafe { (*self.slot.get()).is_none() };
        if empty {
            let value = (self.init)();
            unsafe { *self.slot.get() = Some(value) };
        }
        // SAFETY: as above. The slot was filled a moment ago, so the
        // `unreachable` arm never runs; `expect` would cost `core::fmt`.
        match unsafe { (*self.slot.get()).as_ref() } {
            Some(value) => f(value),
            None => unreachable!(),
        }
    }
}
