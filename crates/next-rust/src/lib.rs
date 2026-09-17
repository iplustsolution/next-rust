//! # Next Rust
//!
//! A Rust-native full-stack web framework: the filesystem is your router,
//! `page.rs` is a page, `layout.rs` wraps its children, and the build turns
//! the app directory into a compiled, type-checked route table.
//!
//! ```text
//! app/
//! ├── layout.rs          → wraps every page
//! ├── page.rs            → /
//! ├── about/page.rs      → /about
//! └── blog/[slug]/page.rs → /blog/:slug
//! ```
//!
//! ```ignore
//! // app/page.rs
//! use next_rust::prelude::*;
//!
//! pub fn Page() -> impl View {
//!     div![h1!["Hello World"], p!["Welcome to Next Rust."]]
//! }
//! ```
//!
//! ```ignore
//! // build.rs
//! fn main() { next_rust_build::generate(); }
//!
//! // src/main.rs
//! next_rust::app!();
//! ```
//!
//! Guides live on the documentation website (`website/` in the repository).

#![forbid(unsafe_code)]

mod font;

pub use font::LocalFont;
pub use next_rust_cache::{Cache, CacheEntry, CacheKey, CacheOptions, CacheStore, FileStore, MemoryStore};
pub use next_rust_core::{Config, Diagnostic, Environment, RenderingMode, config, env as dotenv};
pub use next_rust_macros::{action, asset, client, css_module, global_css, server, server_action};
pub use next_rust_router::{ParamValue, Params};
pub use next_rust_server::*;
pub use next_rust_view::*;

/// Re-exported crates for advanced use.
pub mod deps {
    pub use next_rust_cache as cache;
    pub use next_rust_core as core;
    pub use next_rust_router as router;
    pub use next_rust_server as server;
    pub use next_rust_view as view;
}

/// Memoize an async computation in the data cache.
pub use next_rust_cache::{cache, invalidate};
/// ISR / data-cache invalidation.
pub use next_rust_server::{revalidate_path, revalidate_tag};

/// Everything needed in page, layout and route files.
pub mod prelude {
    pub use next_rust_macros::{action, asset, client, css_module, global_css, server, server_action};
    pub use next_rust_router::Params;
    pub use next_rust_server::{
        ActionContext, Auth, AuthUser, Body, Cookie, Cookies, CsrfToken, Data, Error, ErrorInfo, Extension, Feed,
        FeedEntry, FormState, Headers, Html, IntoResponse, Json, Next, Nonce, OrNotFound, Path, Query, Rendering,
        Request, RequestInfo, Response, ResponseHeaders, Result, Robots, RobotsRule, SameSite, Session, Sitemap,
        SitemapEntry, SseEvent, not_found, permanent_redirect, redirect,
    };
    pub use next_rust_view::metadata::{Icon, OgImage, OpenGraph, Twitter};
    pub use next_rust_view::*;

    pub use crate::LocalFont;
}

/// Include the generated route table (`routes()`) in the current module.
///
/// Normally used through [`app!`]; use it directly to customize the app:
///
/// ```ignore
/// next_rust::routes!();
///
/// fn main() {
///     next_rust::run_with(next_rust::App::new(routes()).middleware(next_rust::request_id()));
/// }
/// ```
#[macro_export]
macro_rules! routes {
    () => {
        #[doc(hidden)]
        #[allow(non_snake_case, clippy::all, clippy::pedantic)]
        mod __next_rust_routes {
            include!(concat!(env!("OUT_DIR"), "/next_rust_routes.rs"));
        }
        #[allow(unused_imports)]
        pub use self::__next_rust_routes::routes;
    };
}

/// Define `main` for a Next Rust application with default settings.
#[macro_export]
macro_rules! app {
    () => {
        $crate::routes!();

        fn main() {
            $crate::run(routes());
        }
    };
}

#[doc(hidden)]
pub mod __private {
    pub use next_rust_server::__private::*;

    use next_rust_view::{Element, attrs};

    pub fn to_json_value<T: serde::Serialize + ?Sized>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
    }

    pub fn island_props(entries: &[(&str, serde_json::Value)]) -> String {
        let map: serde_json::Map<String, serde_json::Value> =
            entries.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect();
        serde_json::Value::Object(map).to_string()
    }

    pub fn island(component: &str, module: Option<&str>, props: String, view: impl View) -> Node {
        let mut el =
            Element::new("nr-island").with(attrs::data("component", component)).with(attrs::data("props", props));
        if let Some(m) = module {
            el = el.with(attrs::data("module", m));
        }
        el.with(view).into_node()
    }
}
