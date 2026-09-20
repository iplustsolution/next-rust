//! The Next Rust browser runtime, as Rust.
//!
//! This is the same program as `RUNTIME_JS` in `next-rust-server`, written in
//! Rust and compiled to WebAssembly. It does client-side navigation and
//! prefetching, draws the loading bar, sends every same-origin form without a
//! page reload, hydrates islands, and publishes `window.nextRust`.
//!
//! It talks to the browser through one import ([`host`]) that a shim of about
//! three hundred lines implements as a table indexed by opcode. Nothing in
//! the shim decides anything: every rule the runtime has — which links are
//! internal, when a prefetch fires, how the bar moves, what a form does,
//! whether an island module is allowed to load — is in this crate.
//!
//! ## Layout
//!
//! | Module | What it holds |
//! | --- | --- |
//! | [`host`] | The ABI: opcodes, string passing, `alloc`/`dealloc`. |
//! | [`dom`] | Safe wrappers over it. Decides nothing. |
//! | [`task`] | The executor, timers and the callback table. |
//! | [`json`] | Just enough JSON for `{ok,data,errors,redirect}`. No parser crate. |
//! | [`num`] | `parseFloat` and decimal printing, without `core`'s 14 KB of it. |
//! | [`cell`] | `thread_local!` for a target that has no threads. |
//! | [`heap`] | The allocator, because `no_std` does not come with one. |
//! | [`loader`] | The loading bar. |
//! | [`nav`] | Navigation, prefetching, rendering, morphing, the page cache. |
//! | [`forms`] | Form enhancement and server actions. |
//! | [`islands`] | Island hydration and the declarative bindings. |
//! | [`api`] | `window.nextRust`. |
//!
//! ## What it costs
//!
//! Measured with the compressor the server serves precompressed files with
//! (brotli quality 11, window 22), and it is worse than the JavaScript it
//! replaces:
//!
//! | | raw / minified | brotli |
//! | --- | --- | --- |
//! | `RUNTIME_JS_MIN` | 21,216 | 6,820 |
//! | `runtime.wasm` | 203,826 | 68,749 |
//! | `shim.min.js` | 9,707 | 3,252 |
//! | **together** | **213,533** | **72,001** |
//!
//! About **ten times** the bytes over the wire. That is *after* the three
//! things that mattered most, each of which is worth knowing before anyone
//! tries to shrink it further:
//!
//! * **`no_std`.** With `std` this crate's bare skeleton — no behaviour at
//!   all — was 35,347 raw and 13,079 brotli, twice the whole JavaScript
//!   runtime, almost all of it dlmalloc, the panic machinery and `std`'s
//!   formatting. Without it the same skeleton is 3,006 and 1,414. Hence
//!   [`heap`] and [`cell`].
//! * **No `core::num::dec2flt`.** One `"1.5".parse::<f64>()` anywhere pulls
//!   in about 10 KB of 128-bit power-of-five tables. Hence [`num`].
//! * **The cargo profile.** The workspace's `[profile.release]` is tuned for
//!   the server, and a profile in this crate's own manifest is ignored —
//!   cargo reads them from the workspace root only. `regenerate.sh` sets
//!   `opt-level=z`, fat LTO, one codegen unit and `panic=abort` through the
//!   environment: 405,579 bytes down to 203,826.
//!
//! What is left is the ported logic itself. The four largest functions in the
//! module are `forms::send_form`, `nav::render`, `nav::navigate` and
//! `nav::morph`, and no further trick closes a tenfold gap: Rust does not
//! compile to anything near the density of hand-written JavaScript for work
//! that is mostly DOM calls and string handling.
//!
//! So `[client] runtime` defaults to `"js"`. What this buys is one language
//! across the application, rules that are reviewable as Rust and testable
//! beside the server, and logic far harder to read or tamper with than
//! minified JavaScript. What it costs is payload, and the trade should be
//! made deliberately.
//!
//! ## Building it
//!
//! ```sh
//! cargo build -p next-rust-runtime --release --target wasm32-unknown-unknown
//! wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
//!   --strip-debug --strip-producers \
//!   target/wasm32-unknown-unknown/release/next_rust_runtime.wasm -o runtime.wasm
//! ```
//!
//! `crates/next-rust-server/src/wasm_runtime/runtime.wasm` is that file, checked in
//! the way `RUNTIME_JS_MIN` is, with a test over the source hash to catch a
//! copy that was not regenerated. `crates/next-rust-runtime/regenerate.sh`
//! does the whole thing.

#![cfg_attr(target_arch = "wasm32", no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod cell;
pub mod dom;
pub mod heap;
pub mod host;
pub mod json;
pub mod num;
pub mod task;

pub mod api;
pub mod forms;
pub mod islands;
pub mod loader;
pub mod nav;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: heap::Alloc = heap::Alloc;

/// Everything the runtime does when the module is instantiated, in the order
/// the JavaScript does it: the public API and the environment, the loading
/// bar (which may have one to finish from the document before this one), the
/// navigation and form listeners, then the active links and the islands.
///
/// The shim calls it exactly once.
#[unsafe(no_mangle)]
pub extern "C" fn start() {
    api::install();
    loader::install();
    nav::install();
    forms::install();
    nav::mark_active_links();
    islands::hydrate_document();
}

/// A panic in the browser must not take the page down with it: the runtime
/// stops, the page keeps working, and the console says what broke.
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // `Location` alone, without the message, keeps `core::fmt`'s heavier
    // paths out of the module; the file and line are enough to find it.
    match info.location() {
        Some(at) => {
            let mut message = alloc::string::String::from("[next-rust] runtime panic at ");
            message.push_str(at.file());
            message.push(':');
            // Not `format!`: the panic path is the one place where pulling
            // `core::fmt`'s machinery in would be paid for by every page.
            message.push_str(&crate::num::u32_to_string(at.line()));
            dom::console_error(&message);
        }
        None => dom::console_error("[next-rust] runtime panic"),
    }
    core::arch::wasm32::unreachable()
}
