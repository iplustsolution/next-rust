//! The site's WebAssembly module.
//!
//! One `islands!` invocation is the whole crate: the macro emits the
//! allocator, the panic handler and every export the shim calls. `start` runs
//! once after the module is instantiated and is where the UI components wire
//! themselves up — they are not per-island, so they cannot hydrate like one.

#![no_std]

extern crate alloc;

next_rust_wasm::islands! {
    start: next_rust_ui_wasm::boot,
}
