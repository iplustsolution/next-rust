//! Runtime plugins.
//!
//! Build-time plugins (route scanning, code generation, asset processing)
//! live in `next-rust-build`; runtime plugins can contribute middleware,
//! document head markup and startup hooks.

use std::sync::Arc;

use next_rust_core::Config;

use crate::middleware::Middleware;

pub trait Plugin: Send + Sync + 'static {
    /// Unique name, used in logs and for `[plugins.<name>]` configuration.
    fn name(&self) -> &str;

    /// Called once when the application is built.
    fn on_start(&self, _config: &Config) {}

    /// Middleware added after the application's global middleware.
    fn middleware(&self) -> Vec<Arc<dyn Middleware>> {
        Vec::new()
    }

    /// Trusted markup appended to every document `<head>`.
    fn head(&self) -> Option<String> {
        None
    }
}
