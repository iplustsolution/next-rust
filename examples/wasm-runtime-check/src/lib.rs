//! A test bed for the WebAssembly browser runtime.
//!
//! Every page here exists to make one behaviour of `crates/next-rust-runtime`
//! observable from outside the browser: a counter that only changes when a
//! layout is re-rendered, a form whose validation errors land in named slots,
//! an island that hydrates, a page that streams.

pub mod actions;
pub mod components;
pub mod module_island;

/// Bumped every time a layout or page function runs on the server, so a test
/// can tell a re-rendered layout from a reused one.
pub mod render_counter {
    use std::sync::atomic::{AtomicU64, Ordering};

    static LAYOUT: AtomicU64 = AtomicU64::new(0);

    pub fn next_layout() -> u64 {
        LAYOUT.fetch_add(1, Ordering::Relaxed) + 1
    }
}

next_rust::routes!();
