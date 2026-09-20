//! The macros an island crate writes, and the per-instance state table.
//!
//! Between them they are the whole of an island crate's boilerplate: the
//! allocator, the panic handler and every export the shim calls are emitted
//! here, so an island file contains island behaviour and nothing else.

use alloc::vec::Vec;

use crate::dom::El;
use crate::props::Props;

/// Which instance of an island is running.
///
/// An island can appear more than once on a page — two pricing tables, a
/// dozen file uploads — and the first version of this ABI could not: its
/// state lived in a `static`, so a second instance quietly overwrote the
/// first. Everything per-instance is keyed by this.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Inst(pub u32);

/// What a hydrate function is handed.
pub struct Ctx {
    /// The `<nr-island>` element the server rendered.
    pub root: El,
    /// This instance's identity, for [`Slots`].
    pub inst: Inst,
    /// The props the server sent, already decoded.
    ///
    /// **Keyed by the server component's argument names**, which is the trap
    /// worth knowing before you debug it. `#[client]` serialises one entry per
    /// argument, so a component written as
    ///
    /// ```text
    /// #[client(wasm = "Attempt")]
    /// pub fn AttemptIsland(props: AttemptProps) -> impl View { … }
    /// ```
    ///
    /// sends `{ "props": { … } }` — the fields are one level down, under the
    /// argument's own name. `cx.props.i64(k!("seconds_remaining"))` then
    /// answers `None` for a field that is plainly in the HTML, which reads as
    /// a decoder bug and is not one. Use [`Ctx::arg`], or reach through with
    /// `cx.props.map(k!("props"))` once and pass that down.
    ///
    /// A component with several arguments has them side by side at the top
    /// level, which is why this is not unwrapped for you: there is nothing to
    /// unwrap in the general case.
    pub props: Props<'static>,
}

impl Ctx {
    /// The props nested under one of the server component's arguments.
    ///
    /// For the common shape — a component taking a single struct — this is
    /// what you want: `cx.arg(k!("props"))`. Empty when the argument is absent
    /// or is not a map, so it is safe to read straight through.
    pub fn arg(&self, key: crate::props::Key) -> Props<'static> {
        self.props.map(key).unwrap_or(Props::EMPTY)
    }
}

/// Per-instance state for one island.
///
/// Most islands should capture their state in the closures they register —
/// that is simpler and needs nothing from here. This is for state that
/// several handlers share and mutate, where cloning an `Rc` into each closure
/// would be the same thing with more ceremony.
pub struct Slots<T> {
    items: Vec<Option<T>>,
}

impl<T> Slots<T> {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn set(&mut self, inst: Inst, value: T) {
        let at = inst.0 as usize;
        while self.items.len() <= at {
            self.items.push(None);
        }
        self.items[at] = Some(value);
    }

    pub fn get(&mut self, inst: Inst) -> Option<&mut T> {
        self.items.get_mut(inst.0 as usize).and_then(|slot| slot.as_mut())
    }

    pub fn remove(&mut self, inst: Inst) -> Option<T> {
        self.items.get_mut(inst.0 as usize).and_then(|slot| slot.take())
    }

    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|slot| slot.is_none())
    }
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Declare a per-island state table.
///
/// ```ignore
/// next_rust_wasm::state!(STATE: State);
///
/// pub fn hydrate(cx: Ctx) {
///     STATE.set(cx.inst, State::new());
///     let inst = cx.inst;
///     tabs.on("click", move |_| {
///         STATE.with(inst, |state| state.active = Mode::Mobile);
///     });
/// }
/// ```
///
/// Reach the state with `with`, which lends it for the duration of a closure:
///
/// ```ignore
/// STATE.with(inst, |state| state.active = Mode::Mobile);
/// ```
///
/// `get` also exists and hands back a `&'static mut`, which **the compiler
/// does not check against anything**. Two calls give two `&mut` to one value,
/// which is undefined behaviour that compiles silently — and has already cost
/// a day, because the optimiser acted on the aliasing and dropped half an
/// island's DOM updates while the rest worked. If you use `get`, pass the
/// `&mut` down to helpers; never pass the `Inst` down so a helper can look the
/// state up again.
///
/// The table is a `static mut` reached through a raw pointer. That is sound
/// here and only here: wasm islands run on the browser's single thread, so
/// there is no second thread to race with, and no `&mut` to the static itself
/// is ever created.
#[macro_export]
macro_rules! state {
    ($name:ident : $ty:ty) => {
        #[allow(non_camel_case_types)]
        struct $name;

        impl $name {
            #[allow(dead_code)]
            fn table(&self) -> &'static mut $crate::Slots<$ty> {
                static mut TABLE: $crate::Slots<$ty> = $crate::Slots::new();
                // Single-threaded by construction; see the macro's docs.
                unsafe { &mut *(&raw mut TABLE) }
            }

            #[allow(dead_code)]
            fn set(&self, inst: $crate::Inst, value: $ty) {
                self.table().set(inst, value);
            }

            /// Borrow this instance's state for the duration of a closure.
            ///
            /// **Prefer this to `get`.** The state is taken out of the slot
            /// while the closure runs and put back afterwards, so a nested
            /// `with` on the same instance finds it absent and answers `None`
            /// instead of handing out a second `&mut` to the same value.
            #[allow(dead_code)]
            fn with<R>(&self, inst: $crate::Inst, f: impl FnOnce(&mut $ty) -> R) -> ::core::option::Option<R> {
                let mut held = self.table().remove(inst)?;
                let out = f(&mut held);
                self.table().set(inst, held);
                ::core::option::Option::Some(out)
            }

            /// The raw borrow, for code that cannot be expressed as a closure.
            ///
            /// The `'static` here is a loaded gun and is worth understanding
            /// before you reach for it: it is **not** checked against anything,
            /// so two calls produce two `&mut` to the same value and the
            /// compiler will not say a word. That is undefined behaviour, and
            /// it is not theoretical — it has already cost a day, because the
            /// optimiser acted on the aliasing and silently dropped half of an
            /// island's DOM updates while the rest worked.
            ///
            /// The rule: pass the `&mut $ty` down to helpers. Never pass an
            /// `Inst` down so a helper can look the state up again, and never
            /// call this while you already hold a borrow from it.
            #[allow(dead_code)]
            fn get(&self, inst: $crate::Inst) -> ::core::option::Option<&'static mut $ty> {
                self.table().get(inst)
            }

            #[allow(dead_code)]
            fn remove(&self, inst: $crate::Inst) -> ::core::option::Option<$ty> {
                self.table().remove(inst)
            }
        }
    };
}

/// Everything the shim calls that is not an island: the allocator, the panic
/// handler and the shared exports.
///
/// Emitted by [`islands!`]; there is no reason to write it yourself.
#[doc(hidden)]
#[macro_export]
macro_rules! __runtime {
    () => {
        #[global_allocator]
        static __NR_ALLOC: $crate::Heap = $crate::Heap;

        /// Linear memory for the shim to write into: props, resumed values,
        /// strings being handed back.
        // These are the module's C ABI: the shim calls them, so they cannot be
        // `unsafe fn`, and clippy's raw-pointer lint has no way to know that.
        // The allow belongs here rather than in every island crate.
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(no_mangle)]
        pub extern "C" fn alloc(len: usize) -> *mut u8 {
            $crate::__alloc(len)
        }

        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(no_mangle)]
        pub extern "C" fn dealloc(at: *mut u8, len: usize) {
            // SAFETY: the shim only passes back what `alloc` gave it.
            unsafe { $crate::__dealloc(at, len) }
        }

        /// A listener fired. `event` is a handle to the event object, valid
        /// only for the duration of this call.
        #[unsafe(no_mangle)]
        pub extern "C" fn on_event(cb: u32, event: u32) {
            $crate::__on_event(cb, event)
        }

        /// An awaited host call produced a value.
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(no_mangle)]
        pub extern "C" fn on_resume(token: u32, ptr: u32, len: u32) {
            $crate::__on_resume(token, ptr, len)
        }

        // Only for the real build. A `cargo test` or `clippy --all-targets`
        // pass links `std`, which brings its own handler and would collide.
        #[cfg(all(target_arch = "wasm32", not(test)))]
        #[panic_handler]
        fn __nr_panic(_: &::core::panic::PanicInfo) -> ! {
            ::core::arch::wasm32::unreachable()
        }
    };
}

/// One island's export.
#[doc(hidden)]
#[macro_export]
macro_rules! __island {
    ($export:ident, $hydrate:path) => {
        #[allow(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(no_mangle)]
        pub extern "C" fn $export(root: u32, props: *const u8, len: usize) -> u32 {
            // SAFETY: the shim allocated the props block and keeps it alive
            // for the life of the instance.
            unsafe { $crate::__hydrate(root, props, len, $hydrate) }
        }
    };
}

/// The page-level entry point.
#[doc(hidden)]
#[macro_export]
macro_rules! __start {
    ($start:path) => {
        // The wasm export has to be called `start` — the loader calls
        // `X.start()` — but the Rust item does not, and must not: the
        // natural way to write this is `islands! { start: crate::start }`
        // beside a `fn start()`, and with `#[no_mangle] fn start` that is
        // `error[E0428]: the name \`start\` is defined multiple times`,
        // pointing at the macro rather than at the name that clashed.
        // `export_name` fixes the symbol without taking the identifier.
        #[unsafe(export_name = "start")]
        pub extern "C" fn __nr_start() {
            let f: fn() = $start;
            f();
        }
    };
}

/// Declare a module's islands.
///
/// Each entry is the **export name** — which is what `#[client(wasm = "…")]`
/// names on the server, spelled `hydrate_<Island>` — and the function behind
/// it, a `fn(Ctx)`.
///
/// ```ignore
/// #![no_std]
/// extern crate alloc;
///
/// mod auth_switch;
///
/// next_rust_wasm::islands! {
///     hydrate_AuthSwitch => crate::auth_switch::hydrate,
/// }
/// ```
///
/// With a page-level entry point — code that is not per-island, such as a UI
/// kit installing delegated listeners — put a `start:` first. It runs once
/// after the module instantiates and before any island hydrates:
///
/// ```ignore
/// next_rust_wasm::islands! {
///     start: crate::ui::boot,
///     hydrate_Tabs => crate::tabs::hydrate,
/// }
/// ```
///
/// The export name is written out in full rather than assembled from a short
/// name, because a Rust macro cannot build an identifier out of a string and
/// the build reads these names straight out of the module's export section.
/// Spelling it once, here, is the honest version of that constraint.
#[macro_export]
macro_rules! islands {
    (start: $start:path $(, $export:ident => $hydrate:path)* $(,)?) => {
        $crate::__runtime!();
        $crate::__start!($start);
        $( $crate::__island!($export, $hydrate); )*
    };
    ($($export:ident => $hydrate:path),* $(,)?) => {
        $crate::__runtime!();
        $( $crate::__island!($export, $hydrate); )*
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_are_per_instance() {
        let mut slots: Slots<u32> = Slots::new();
        assert!(slots.is_empty());
        slots.set(Inst(1), 10);
        slots.set(Inst(4), 40);
        assert_eq!(slots.get(Inst(1)).copied(), Some(10));
        assert_eq!(slots.get(Inst(4)).copied(), Some(40));
        // The gap between them is not state, and an instance that never
        // registered reads as absent rather than as someone else's value.
        assert_eq!(slots.get(Inst(2)), None);
        assert_eq!(slots.get(Inst(99)), None);
        *slots.get(Inst(1)).unwrap() = 11;
        assert_eq!(slots.get(Inst(1)).copied(), Some(11));
        assert_eq!(slots.get(Inst(4)).copied(), Some(40), "one instance must not disturb another");
        assert_eq!(slots.remove(Inst(1)), Some(11));
        assert_eq!(slots.get(Inst(1)), None);
        assert_eq!(slots.remove(Inst(1)), None);
    }
}
