use std::sync::atomic::AtomicUsize;

next_rust::routes!();

/// How often the home page's shell was rendered (shown on the page, checked by the test).
pub static SHELL_RENDERS: AtomicUsize = AtomicUsize::new(0);
