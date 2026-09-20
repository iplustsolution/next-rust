//! This site's browser logic, compiled to one WebAssembly module.
//!
//! Everything here runs in the browser and nothing here runs on the server,
//! so it is `no_std`: no files, no sockets, no threads. Reach the page
//! through `next_rust_wasm::prelude`.
//!
//! The `islands!` invocation below is the module's export list. Each
//! `hydrate_<Name>` entry is what `#[client(wasm = "<Name>")]` binds to on
//! the server, spelled exactly and case-sensitively — a name that does not
//! match is a component that renders and never reacts, which the build
//! reports as `NR0313`.

#![no_std]

extern crate alloc;

#[allow(unused_imports)]
use next_rust_wasm::prelude::*;

next_rust_wasm::islands! {
    start: next_rust_ui_wasm::boot,
    // hydrate_Counter => crate::counter,
}

// Write one `fn(Ctx)` per island and list it above. `cx.root` is the
// `<nr-island>` element the server rendered and `cx.props` carries the
// component's arguments, keyed by their Rust names.
//
// pub fn counter(cx: Ctx) {
//     cx.root.query("button").on("click", move |_| log("clicked"));
// }
