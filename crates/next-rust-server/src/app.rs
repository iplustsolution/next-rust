//! Route definitions, application builder and request dispatch.
//!
//! The types in the first half of this module are the contract between the
//! build-time code generator and the runtime. Application code normally never
//! constructs them by hand: `next_rust::routes!()` expands to generated code
//! that does.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures_util::FutureExt;
use http::{Method, StatusCode};
use next_rust_cache::{Cache, CacheStore, MemoryStore};
use next_rust_core::{Config, Environment};
use next_rust_router::{Matcher, Params, PatternSegment, RoutePattern, SegmentKind, parse_segment};
use next_rust_view::{Children, Metadata, Node, Slots};

use crate::context::Ctx;
use crate::error::Result;
use crate::middleware::{BoxFuture, Endpoint, FnMiddleware, Middleware, Next};
use crate::request::Request;
use crate::response::{IntoResponse, Response};
use crate::seo::{Robots, Sitemap};

pub type RenderFn = fn(Ctx) -> BoxFuture<Result<Node>>;
pub type LayoutFn = fn(Ctx, Children, Slots) -> BoxFuture<Result<Node>>;
pub type LoadingFn = fn(Ctx) -> Node;
pub type ErrorFn = fn(Ctx, ErrorInfo) -> Node;
pub type MetadataFn = fn(Ctx) -> BoxFuture<Result<Metadata>>;
pub type MiddlewareFn = fn(Request, Next) -> BoxFuture<Response>;
pub type HandlerFn = fn(Request) -> BoxFuture<Response>;
pub type ParamsFn = fn() -> BoxFuture<Result<Vec<Params>>>;
pub type SitemapFn = fn() -> BoxFuture<Sitemap>;
pub type RobotsFn = fn() -> BoxFuture<Robots>;

/// Rendering mode declared by a page: `pub const RENDERING: Rendering = Rendering::Dynamic;`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rendering {
    #[default]
    Auto,
    Static,
    Dynamic,
    /// Partial prerendering: a static shell, cached like a static page, with
    /// [`dynamic`](crate::dynamic) holes rendered for every request.
    Partial,
}

impl Rendering {
    /// The page (or its shell) is rendered ahead of requests and cached.
    pub fn is_prerendered(self) -> bool {
        matches!(self, Rendering::Static | Rendering::Partial)
    }
}

/// Information passed to `error.rs` / `global-error.rs`.
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    pub status: u16,
    /// User-safe message in production, detailed message in development.
    pub message: String,
    /// Random identifier that is also written to the server log.
    pub digest: String,
}

impl ErrorInfo {
    pub(crate) fn from_error(e: &crate::Error, dev: bool) -> Self {
        let digest = crate::random_hex(6);
        let status = e.status().as_u16();
        if status >= 500 {
            crate::log::error(&format!("render error [{digest}]: {}", e.detailed_message()));
        }
        ErrorInfo { status, message: if dev { e.detailed_message() } else { e.public_message() }, digest }
    }
}

/// Files attached to one route segment (directory).
#[derive(Clone, Default)]
pub struct SegmentDef {
    pub layout: Option<LayoutFn>,
    pub template: Option<LayoutFn>,
    pub loading: Option<LoadingFn>,
    pub error: Option<ErrorFn>,
    pub not_found: Option<RenderFn>,
    pub metadata: Option<MetadataFn>,
    /// Nested `middleware.rs` (never the app-root one, which is global).
    pub middleware: Option<MiddlewareFn>,
    pub slots: Vec<SlotDef>,
    /// Identity of `layout` (its file, relative to the project), used to
    /// recognize a layout the browser already shows.
    pub layout_id: &'static str,
    /// The build found that `layout` (and its `load`) reads nothing from the
    /// request, so client navigations may keep it on screen instead of
    /// rendering it again.
    pub layout_reusable: bool,
    /// `layout` reads route parameters (`Params` / `Path`): it is only reused
    /// while every parameter keeps its value.
    pub layout_uses_params: bool,
}

#[derive(Clone)]
pub struct SlotDef {
    pub name: &'static str,
    /// The matching slot page or its `default.rs`.
    pub render: Option<RenderFn>,
}

pub enum PageBody {
    Rust(RenderFn),
    /// Contents of `page.html`.
    Html(&'static str),
}

pub struct PageDef {
    /// Filesystem-style pattern, e.g. `/blog/[slug]`.
    pub pattern: &'static str,
    pub source: &'static str,
    pub body: PageBody,
    /// Root → leaf segments.
    pub segments: Vec<SegmentDef>,
    pub metadata: Option<MetadataFn>,
    /// Resolved by the build (`Auto` is treated as `Dynamic`).
    pub rendering: Rendering,
    pub revalidate: Option<u64>,
    pub generate_params: Option<ParamsFn>,
    /// Render params not returned by `generate_params` on demand.
    pub dynamic_params: bool,
    /// Cache tags for `revalidate_tag`.
    pub tags: &'static [&'static str],
    /// Set for intercepting routes: the URL pattern they intercept from.
    pub intercept_from: Option<&'static str>,
}

pub struct ApiDef {
    pub pattern: &'static str,
    pub source: &'static str,
    pub handlers: Vec<(Method, HandlerFn)>,
    pub middleware: Vec<MiddlewareFn>,
}

pub struct ActionDef {
    /// Stable identifier (`module::path::fn_name`).
    pub id: &'static str,
    pub handler: HandlerFn,
}

/// Everything discovered in the app directory.
#[derive(Default)]
pub struct Routes {
    pub pages: Vec<PageDef>,
    pub apis: Vec<ApiDef>,
    pub actions: Vec<ActionDef>,
    /// `not-found.rs` / `layout.rs` / `metadata` of the app root, used for unmatched URLs.
    pub root: SegmentDef,
    /// App-root `middleware.rs`: runs for every request, before routing.
    pub middleware: Option<MiddlewareFn>,
    pub global_error: Option<ErrorFn>,
    pub sitemap: Option<SitemapFn>,
    pub robots: Option<RobotsFn>,
    /// Absolute project root recorded at build time (fallback for config discovery).
    pub project_root: Option<&'static str>,
    /// Configuration and static files compiled into release binaries.
    pub embedded: Option<&'static crate::embed::Embedded>,
    /// Changes whenever the application's source changes. Part of every layout
    /// key, so a browser never keeps a layout from an older deployment.
    pub build_id: &'static str,
    /// Reads `next-rust.toml` at runtime. Set by the generated code for
    /// development builds; release builds embed their configuration as JSON
    /// and leave this `None`, so no TOML parser is linked.
    pub toml: Option<next_rust_core::config::TomlParser>,
    /// Stylesheets added to every page, such as the generated Tailwind CSS.
    pub stylesheets: &'static [&'static next_rust_view::Stylesheet],
    /// Release builds with Tailwind: `(class, short name)` pairs sorted by
    /// class, applied to every page this app renders.
    pub class_names: next_rust_view::class_names::ClassNames,
    /// Classes the build knows a rule, script or static file for (sorted);
    /// empty means every class is kept (see `next_rust_view::class_names::set_known_classes`).
    pub known_classes: &'static [&'static str],
    /// Scripts prepared for `[build] inline_assets` (see [`crate::inline`]);
    /// empty when the switch is off.
    pub inline_scripts: &'static [crate::inline::InlineScript],
    /// Scripts declared in Rust (`client_script!`, `asset_script!`) as
    /// `("client/chart.js", source)`, for development builds; release builds
    /// embed them with the files of `client/` and `assets/`.
    pub declared_scripts: &'static [(&'static str, &'static str)],
    /// Generated files that are not scripts, for development builds: the
    /// WebAssembly module of `[build] wasm_islands`. Release builds embed it
    /// with the files of `client/` instead, so this is empty there.
    pub declared_binaries: &'static [(&'static str, &'static [u8])],
    /// Release builds (`[build] minify_names`): short names for `data-*`
    /// attributes, custom properties, ids, keyframes and layers, sorted by
    /// key. Every HTML response is written with them (see [`crate::markup`]).
    pub names: crate::markup::NameTable,
    /// Release builds (`[build] minify_html`, `svg_precision`): how HTML
    /// responses are written.
    pub markup: crate::markup::Markup,
}

pub(crate) type CustomHandlers = Vec<(Option<Method>, Arc<dyn Endpoint>)>;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Target {
    Page(usize),
    Api(usize),
    Custom(usize),
}

struct CustomRoute {
    pattern: String,
    method: Option<Method>,
    endpoint: Arc<dyn Endpoint>,
}

pub(crate) struct Intercept {
    pub page: usize,
    pub target: Matcher<()>,
    pub context: RoutePattern,
}

pub(crate) type LifecycleHook = Arc<dyn Fn() -> crate::middleware::BoxFuture<()> + Send + Sync>;

pub(crate) struct AppInner {
    pub routes: Routes,
    pub matcher: Matcher<Target>,
    pub intercepts: Vec<Intercept>,
    pub customs: Vec<CustomHandlers>,
    pub config: Arc<Config>,
    pub env: Environment,
    pub public_dir: PathBuf,
    pub client_dir: PathBuf,
    pub dev_status_file: PathBuf,
    /// Files compiled into the binary; `None` reads from disk.
    pub embedded: Option<&'static crate::embed::Embedded>,
    pub cache: Cache,
    pub page_store: Arc<dyn CacheStore>,
    pub global_stack: Arc<[Arc<dyn Middleware>]>,
    pub page_stacks: Vec<Arc<[Arc<dyn Middleware>]>>,
    pub api_stacks: Vec<Arc<[Arc<dyn Middleware>]>>,
    /// Keyed action id → index into `routes.actions`.
    pub(crate) actions: HashMap<crate::action_token::ActionKey, usize>,
    pub revalidating: Mutex<HashSet<String>>,
    pub static_params: Mutex<HashMap<usize, Arc<Vec<Params>>>>,
    /// Shells of partially prerendered pages (their HTML is in `page_store`).
    pub(crate) shells: crate::partial::Shells,
    pub plugins: Vec<Arc<dyn crate::plugin::Plugin>>,
    /// Run when the server starts listening / after it has drained (see `AppBuilder::on_serve`).
    pub(crate) on_serve: Vec<LifecycleHook>,
    pub(crate) on_shutdown: Vec<LifecycleHook>,
    pub public_env_json: Option<String>,
    /// The motion stylesheet (`[animation]`) as `(id, css)`, first in every
    /// `<head>`; `None` when animation is off.
    pub motion: Option<(String, String)>,
    /// The loading-bar stylesheet (`[loader]`) as `(id, css)`; `None` when off.
    pub loader: Option<(String, String)>,
    /// `[verification]` and `[analytics]`, ready to write.
    pub(crate) tracking: crate::tracking::Tracking,
    /// Rewrites HTML responses as they leave (release builds).
    pub(crate) finisher: Option<crate::markup::Finisher>,
}

/// A configured application. Cheap to clone.
#[derive(Clone)]
pub struct App {
    pub(crate) inner: Arc<AppInner>,
}

/// Builder returned by [`App::new`].
pub struct AppBuilder {
    routes: Routes,
    config: Option<Config>,
    env: Option<Environment>,
    middleware: Vec<Arc<dyn Middleware>>,
    cache: Option<Cache>,
    page_store: Option<Arc<dyn CacheStore>>,
    customs: Vec<CustomRoute>,
    plugins: Vec<Arc<dyn crate::plugin::Plugin>>,
    on_serve: Vec<LifecycleHook>,
    on_shutdown: Vec<LifecycleHook>,
}

/// Convert `/users/:id`, `/files/*path` or `/users/[id]` into a pattern.
pub fn parse_pattern(pattern: &str) -> std::result::Result<RoutePattern, String> {
    let mut segs = Vec::new();
    for part in pattern.split('/').filter(|p| !p.is_empty()) {
        let seg = if let Some(name) = part.strip_prefix(':') {
            PatternSegment::Dynamic(name.to_owned())
        } else if let Some(name) = part.strip_prefix('*') {
            match name.strip_suffix('?') {
                Some(n) => PatternSegment::OptionalCatchAll(n.to_owned()),
                None => PatternSegment::CatchAll(name.to_owned()),
            }
        } else {
            match parse_segment(part)? {
                Some(SegmentKind::Static(s)) => PatternSegment::Static(s),
                Some(SegmentKind::Dynamic(s)) => PatternSegment::Dynamic(s),
                Some(SegmentKind::CatchAll(s)) => PatternSegment::CatchAll(s),
                Some(SegmentKind::OptionalCatchAll(s)) => PatternSegment::OptionalCatchAll(s),
                _ => return Err(format!("unsupported segment `{part}` in pattern `{pattern}`")),
            }
        };
        segs.push(seg);
    }
    Ok(RoutePattern(segs))
}

impl App {
    /// Start building an application from generated routes.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(routes: Routes) -> AppBuilder {
        next_rust_view::class_names::set_class_names(routes.class_names);
        next_rust_view::class_names::set_known_classes(routes.known_classes);
        crate::internal::install_short_component_names(routes.class_names, routes.names);
        crate::internal::register_framework_scripts(routes.stylesheets);
        AppBuilder {
            routes,
            config: None,
            env: None,
            middleware: Vec::new(),
            cache: None,
            page_store: None,
            customs: Vec::new(),
            plugins: Vec::new(),
            on_serve: Vec::new(),
            on_shutdown: Vec::new(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn environment(&self) -> Environment {
        self.inner.env
    }

    pub fn cache(&self) -> &Cache {
        &self.inner.cache
    }

    /// Handle one request. This is the adapter surface: the built-in server,
    /// tests and serverless adapters all call it.
    pub async fn handle(&self, req: Request) -> Response {
        // Error verbosity follows the app that answers, not the last one
        // built in this process (tests and embeddings build several).
        crate::with_dev(self.inner.env.is_dev(), self.handle_request(req)).await
    }

    async fn handle_request(&self, mut req: Request) -> Response {
        let inner = self.inner.clone();
        let nonce = crate::random_hex(16);
        req.insert_extension(crate::CspNonce(nonce.clone()));
        req.insert_extension(inner.config.clone());
        req.limit_body(inner.config.server.body_limit);

        if let Some(res) = self.pre_routing(&mut req).await {
            return self.finish(res, &req_cookies_placeholder(), &nonce, "");
        }
        let cookies = req.cookies().clone();
        let path = req.path().to_owned();
        let inline = crate::inline::known(&inner, &req);
        let endpoint: Arc<dyn Endpoint> = Arc::new(RouteEndpoint { inner: inner.clone() });
        // A panic in application code answers 500 here, not only in the
        // built-in server: every adapter (serverless, tests, an embedding)
        // goes through this method. Cookies set before the panic are
        // dropped with the half-finished response they belong to.
        let handled =
            std::panic::AssertUnwindSafe(Next::new(inner.global_stack.clone(), endpoint).run(req)).catch_unwind().await;
        let mut res = match handled {
            Ok(res) => res,
            Err(panic) => {
                crate::log::error(&format!("request handler panicked: {}", panic_message(&panic)));
                let res = Response::text("Internal Server Error").with_status(500);
                return self.finish(res, &req_cookies_placeholder(), &nonce, "");
            }
        };
        // `[build] inline_assets`: scripts go into the document.
        crate::inline::apply(&inner, &mut res, &nonce, inline);
        // Release builds: short names and compact markup, last of all.
        crate::markup::apply(&inner, &mut res);
        self.finish(res, &cookies, &nonce, &path)
    }

    /// Framework assets, base path, configured redirects and trailing slashes.
    async fn pre_routing(&self, req: &mut Request) -> Option<Response> {
        let inner = &self.inner;
        let config = &inner.config;

        let base = config.app.base_path.as_str();
        if !base.is_empty() {
            let path = req.path().to_owned();
            let stripped = if path == base {
                "/".to_owned()
            } else if let Some(rest) = path.strip_prefix(base).filter(|r| r.starts_with('/')) {
                rest.to_owned()
            } else {
                return Some(Response::not_found());
            };
            let query = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
            if req.set_path(&format!("{stripped}{query}")).is_err() {
                return Some(Response::status(400));
            }
        }

        let path = req.path();
        if path.starts_with("/_next-rust/")
            && let Some(res) = crate::internal::handle(inner, req).await
        {
            return Some(res);
        }

        // `[verification]` files: /google….html, /BingSiteAuth.xml, /ads.txt,
        // /.well-known/assetlinks.json … (before the trailing-slash redirect:
        // their URLs are fixed by whoever fetches them).
        if (req.method() == Method::GET || req.method() == Method::HEAD)
            && let Some(res) = inner.tracking.file(path)
        {
            return Some(res);
        }

        for rule in &config.redirects {
            if let Some(dest) = crate::redirects::apply(&rule.source, &rule.destination, path) {
                let query = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
                let location = if dest.contains('?') { dest } else { format!("{dest}{query}") };
                return Some(if rule.permanent {
                    Response::permanent_redirect(&location)
                } else {
                    Response::redirect(&location)
                });
            }
        }

        if path.len() > 1 {
            let has_slash = path.ends_with('/');
            let looks_like_file = path.rsplit('/').next().is_some_and(|last| last.contains('.'));
            if has_slash != config.app.trailing_slash && !looks_like_file {
                let fixed = if has_slash { path.trim_end_matches('/').to_owned() } else { format!("{path}/") };
                // One leading slash only: `//evil.example/` must not become a
                // redirect to `//evil.example`, which browsers read as a host.
                let fixed = format!("/{}", fixed.trim_start_matches(['/', '\\']));
                let query = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
                return Some(Response::permanent_redirect(&format!("{base}{fixed}{query}")));
            }
        }
        None
    }

    fn finish(&self, mut res: Response, cookies: &crate::Cookies, nonce: &str, path: &str) -> Response {
        let config = &self.inner.config;
        crate::action_token::seal(&mut res, cookies, config.security.action_token_ttl, config.security.csrf);
        // A complete body is marked `private, no-store` when it carried an
        // action marker. A streamed one is read after the head is sent, so
        // whether it holds a token is not known here: every streamed HTML
        // response gets this visitor's binding cookie, so none of them may
        // be kept by a shared cache.
        if matches!(res.body, crate::response::Body::Stream(_))
            && res.header("content-type").is_some_and(|c| c.starts_with("text/html"))
        {
            res.set_header("cache-control", crate::render::NO_STORE);
        }
        for value in cookies.set_cookie_headers() {
            res.append_header("set-cookie", &value);
        }
        if !path.is_empty() {
            for rule in &config.headers {
                if crate::redirects::matches(&rule.source, path) {
                    for (k, v) in &rule.headers {
                        res.set_header(k, v);
                    }
                }
            }
        }
        if config.security.headers {
            crate::middleware::apply_security_headers(
                &mut res,
                // `strict` allows the origins of the configured `[analytics]`.
                self.inner.tracking.csp(),
                nonce,
                config.security.hsts_max_age,
                // Any of `[build] wasm_islands`, `[client] runtime = "wasm"`
                // and `[ui] script = "wasm"` makes a page compile a module.
                config.uses_wasm(),
            );
        }
        res
    }

    /// Re-render `path` on its next request (ISR) and drop data-cache
    /// entries stored under the same path key.
    pub async fn revalidate_path(&self, path: &str) -> bool {
        let path = if path.len() > 1 { path.trim_end_matches('/') } else { path };
        let page = self.inner.page_store.delete(&next_rust_cache::CacheKey::page(path)).await.unwrap_or(false);
        page | self.inner.cache.revalidate_path(path).await.unwrap_or(false)
    }

    /// Invalidate pages and data tagged with `tag`. Returns the number of entries removed.
    pub async fn revalidate_tag(&self, tag: &str) -> usize {
        self.inner.page_store.delete_tag(tag).await.unwrap_or(0)
            + self.inner.cache.revalidate_tag(tag).await.unwrap_or(0)
    }

    /// Render every static route without keeping the result: used by
    /// `next-rust build` to check that static generation succeeds.
    pub async fn export(&self) -> std::result::Result<crate::export::ExportReport, String> {
        crate::export::prerender(&self.inner, &MemoryStore::new(usize::MAX)).await
    }

    /// Render every static route into the page cache, so the first visitor
    /// gets a cached page. Production servers do this in the background at
    /// startup.
    pub async fn prerender(&self) -> std::result::Result<crate::export::ExportReport, String> {
        crate::export::prerender(&self.inner, self.inner.page_store.as_ref()).await
    }

    /// Route table as `(kind, pattern, source)` for diagnostics.
    pub fn route_table(&self) -> Vec<(&'static str, &'static str, &'static str)> {
        let r = &self.inner.routes;
        let mut out: Vec<_> = r.pages.iter().map(|p| ("page", p.pattern, p.source)).collect();
        out.extend(r.apis.iter().map(|a| ("api", a.pattern, a.source)));
        out
    }
}

/// What a caught panic said, for the log.
pub(crate) fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned())
}

fn req_cookies_placeholder() -> crate::Cookies {
    crate::Cookies::default()
}

/// Find the configuration.
///
/// `NEXT_RUST_CONFIG` wins. A release binary then uses the configuration it
/// was built with, rooted at the working directory, so it runs on its own.
/// Otherwise the working directory (and its ancestors) is searched, then the
/// project root recorded at build time, so `./target/debug/my-app` works from
/// any directory.
pub fn discover_config(
    project_root: Option<&str>,
    embedded: Option<&crate::embed::Embedded>,
    toml: Option<next_rust_core::config::TomlParser>,
) -> std::result::Result<Config, String> {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if std::env::var_os("NEXT_RUST_CONFIG").is_none()
        && let Some((text, json)) = embedded.and_then(|e| e.config)
    {
        let parsed = match (json, toml) {
            (true, _) => Config::from_json_str(text),
            (false, Some(parse)) => parse(text),
            (false, None) => Err("embedded TOML configuration needs a TOML parser".to_owned()),
        };
        let mut config = parsed.map_err(|e| format!("embedded configuration: {e}"))?;
        config.root = start;
        config.apply_env_overrides();
        return Ok(config);
    }
    let from_cwd = Config::discover_with(&start, toml).map_err(|e| e.to_string())?;
    if from_cwd.source.is_some() || from_cwd.app_dir().is_dir() {
        return Ok(from_cwd);
    }
    match project_root {
        Some(root) if std::path::Path::new(root).is_dir() => {
            Config::discover_with(root, toml).map_err(|e| e.to_string())
        }
        _ => Ok(from_cwd),
    }
}

impl AppBuilder {
    pub(crate) fn project_root(&self) -> Option<&'static str> {
        self.routes.project_root
    }

    pub(crate) fn embedded(&self) -> Option<&'static crate::embed::Embedded> {
        self.routes.embedded
    }

    pub(crate) fn toml_parser(&self) -> Option<next_rust_core::config::TomlParser> {
        self.routes.toml
    }

    /// Use an explicit configuration instead of discovering `next-rust.toml`.
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn environment(mut self, env: Environment) -> Self {
        self.env = Some(env);
        self
    }

    /// Add global middleware (runs before `middleware.rs` files, in order).
    pub fn middleware(mut self, mw: impl Middleware) -> Self {
        self.middleware.push(Arc::new(mw));
        self
    }

    /// Share one value with every page, layout, loader, API route and server
    /// action: it is cloned into each request's extensions, so handlers take
    /// it as `Extension<T>`. Wrap anything expensive to clone in an `Arc`:
    ///
    /// ```ignore
    /// App::new(routes()).state(Arc::new(AppState { db, redis }))
    /// // pub async fn Page(Extension(state): Extension<Arc<AppState>>) -> impl View
    /// ```
    pub fn state<T: Clone + Send + Sync + 'static>(self, value: T) -> Self {
        self.middleware(move |mut req: Request, next: crate::middleware::Next| {
            req.insert_extension(value.clone());
            next.run(req)
        })
    }

    /// Data cache (default: in-memory).
    pub fn cache(mut self, cache: Cache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Store for statically rendered pages (default: files in the build
    /// output in production, memory in development).
    pub fn page_store(mut self, store: impl CacheStore) -> Self {
        self.page_store = Some(Arc::new(store));
        self
    }

    pub fn plugin(mut self, plugin: impl crate::plugin::Plugin) -> Self {
        self.plugins.push(Arc::new(plugin));
        self
    }

    /// Run `hook` when the server starts serving (not for `--routes`,
    /// `--export` or [`App::handle`] in tests): the place to start background
    /// workers that should live exactly as long as the server.
    pub fn on_serve<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_serve.push(Arc::new(move || Box::pin(hook())));
        self
    }

    /// Run `hook` during graceful shutdown, after open connections have
    /// drained and before the process exits. Hooks run in order and are
    /// awaited, so they bound their own duration.
    pub fn on_shutdown<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_shutdown.push(Arc::new(move || Box::pin(hook())));
        self
    }

    /// Register a handler programmatically for any method.
    /// Patterns accept `/users/:id`, `/files/*path` or `/users/[id]`.
    pub fn route<H, Fut, R>(self, method: Method, pattern: &str, handler: H) -> Self
    where
        H: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
    {
        self.custom(Some(method), pattern, handler)
    }

    pub fn get<H, Fut, R>(self, pattern: &str, handler: H) -> Self
    where
        H: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
    {
        self.custom(Some(Method::GET), pattern, handler)
    }

    pub fn post<H, Fut, R>(self, pattern: &str, handler: H) -> Self
    where
        H: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
    {
        self.custom(Some(Method::POST), pattern, handler)
    }

    /// WebSocket endpoint: `.ws("/socket", |socket| async move { .. })`.
    #[cfg(feature = "websocket")]
    pub fn ws<H, Fut>(self, pattern: &str, handler: H) -> Self
    where
        H: Fn(crate::ws::WebSocket) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.custom(Some(Method::GET), pattern, move |req: Request| {
            let handler = handler.clone();
            async move { crate::ws::upgrade(req, handler) }
        })
    }

    fn custom<H, Fut, R>(mut self, method: Option<Method>, pattern: &str, handler: H) -> Self
    where
        H: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
    {
        let handler = Arc::new(handler);
        let endpoint = move |req: Request| {
            let handler = handler.clone();
            async move { handler(req).await.into_response() }
        };
        self.customs.push(CustomRoute { pattern: pattern.to_owned(), method, endpoint: Arc::new(endpoint) });
        self
    }

    /// Validate routes and build the application.
    pub fn build(self) -> std::result::Result<App, String> {
        let env = self.env.unwrap_or_else(Environment::from_env);
        let config = match self.config {
            Some(c) => c,
            None => discover_config(self.routes.project_root, self.routes.embedded, self.routes.toml)?,
        };
        crate::set_dev(env.is_dev());
        let json = match config.logging.format {
            next_rust_core::config::LogFormat::Json => true,
            next_rust_core::config::LogFormat::Pretty => false,
            next_rust_core::config::LogFormat::Auto => !env.is_dev(),
        };
        // Production prints only errors unless `[logging]` says otherwise.
        crate::log::configure(json, config.logging.level(env));
        crate::set_trust_proxy(config.server.trust_proxy);
        crate::set_trusted_proxies(usize::from(config.server.trusted_proxies));

        let mut matcher = Matcher::new();
        let mut intercepts = Vec::new();
        for (i, page) in self.routes.pages.iter().enumerate() {
            let pattern = parse_pattern(page.pattern)?;
            if let Some(from) = page.intercept_from {
                let mut target = Matcher::new();
                target.insert(pattern.segments(), ()).map_err(|e| format!("{}: {e}", page.source))?;
                intercepts.push(Intercept { page: i, target, context: parse_pattern(from)? });
                continue;
            }
            matcher
                .insert(pattern.segments(), Target::Page(i))
                .map_err(|e| format!("route {} ({}): {e}", page.pattern, page.source))?;
        }
        for (i, api) in self.routes.apis.iter().enumerate() {
            let pattern = parse_pattern(api.pattern)?;
            matcher
                .insert(pattern.segments(), Target::Api(i))
                .map_err(|e| format!("route {} ({}): {e}", api.pattern, api.source))?;
        }
        // Programmatic routes: group handlers by pattern.
        let mut customs: Vec<CustomHandlers> = Vec::new();
        let mut custom_index: HashMap<String, usize> = HashMap::new();
        for c in self.customs {
            let pattern = parse_pattern(&c.pattern)?;
            let shape = pattern.shape();
            let idx = match custom_index.get(&shape) {
                Some(i) => *i,
                None => {
                    let i = customs.len();
                    matcher
                        .insert(pattern.segments(), Target::Custom(i))
                        .map_err(|e| format!("route {}: {e} (conflicts with a file route)", c.pattern))?;
                    customs.push(Vec::new());
                    custom_index.insert(shape, i);
                    i
                }
            };
            customs[idx].push((c.method, c.endpoint));
        }

        let mut global: Vec<Arc<dyn Middleware>> = self.middleware;
        for p in &self.plugins {
            global.extend(p.middleware());
        }
        if let Some(mw) = self.routes.middleware {
            global.push(Arc::new(FnMiddleware(mw)));
        }
        let stack_of = |fns: Vec<MiddlewareFn>| -> Arc<[Arc<dyn Middleware>]> {
            fns.into_iter().map(|f| Arc::new(FnMiddleware(f)) as Arc<dyn Middleware>).collect::<Vec<_>>().into()
        };
        let page_stacks = self
            .routes
            .pages
            .iter()
            .map(|p| stack_of(p.segments.iter().filter_map(|s| s.middleware).collect()))
            .collect();
        let api_stacks = self.routes.apis.iter().map(|a| stack_of(a.middleware.clone())).collect();
        crate::action_token::check_secret()?;
        if !self.routes.actions.is_empty() && !env.is_dev() && !crate::action_token::keys().from_env {
            crate::log::warn(&format!(
                "{} is not set: server-action links are signed with a random key, so open pages stop working \
                 after a restart and links don't work across instances",
                crate::SECRET_ENV
            ));
        }
        let actions =
            self.routes.actions.iter().enumerate().map(|(i, a)| (crate::action_token::action_key(a.id), i)).collect();

        let output_dir = config.output_dir();
        let page_store: Arc<dyn CacheStore> = match self.page_store {
            Some(s) => s,
            None if env.is_dev() => Arc::new(MemoryStore::new(1000)),
            // Production keeps pages in memory: the binary writes no files.
            None => Arc::new(MemoryStore::new(10_000)),
        };
        let cache = self.cache.unwrap_or_default();
        next_rust_cache::install_global(cache.clone());
        let _ = crate::PAGE_STORE.set(page_store.clone());

        let public_env = next_rust_core::env::EnvVars::load(&config.root, env, &config.env.public_prefix)
            .map(|e| e.public())
            .unwrap_or_default();
        let public_env_json = (!public_env.is_empty()).then(|| serde_json::to_string(&public_env).unwrap_or_default());

        let inner = AppInner {
            finisher: crate::markup::Finisher::new(self.routes.names, self.routes.markup)
                .map(crate::markup::Finisher::install),
            public_dir: config.public_dir(),
            client_dir: config.root.join("client"),
            dev_status_file: output_dir.join("dev/status.json"),
            embedded: self.routes.embedded.filter(|e| !e.is_empty()),
            routes: self.routes,
            matcher,
            intercepts,
            customs,
            env,
            cache,
            page_store,
            global_stack: global.into(),
            page_stacks,
            api_stacks,
            actions,
            revalidating: Mutex::new(HashSet::new()),
            static_params: Mutex::new(HashMap::new()),
            shells: Default::default(),
            plugins: self.plugins,
            on_serve: self.on_serve,
            on_shutdown: self.on_shutdown,
            public_env_json,
            motion: crate::motion::stylesheet(&config.animation),
            loader: crate::motion::loader(&config.loader),
            tracking: crate::tracking::Tracking::new(&config, env),
            config: Arc::new(config),
        };
        for p in &inner.plugins {
            p.on_start(&inner.config);
        }
        Ok(App { inner: Arc::new(inner) })
    }
}

/// Final endpoint of the global middleware chain: routing.
struct RouteEndpoint {
    inner: Arc<AppInner>,
}

impl Endpoint for RouteEndpoint {
    fn call(&self, req: Request) -> BoxFuture<Response> {
        let inner = self.inner.clone();
        Box::pin(async move { dispatch(inner, req).await })
    }
}

async fn dispatch(inner: Arc<AppInner>, mut req: Request) -> Response {
    let path = req.path().to_owned();

    if path.starts_with("/_next-rust/action/") {
        return crate::actions::handle(&inner, req).await;
    }
    if req.method() == Method::GET || req.method() == Method::HEAD {
        if path == "/sitemap.xml"
            && let Some(f) = inner.routes.sitemap
        {
            return f().await.into_response();
        }
        if path == "/robots.txt"
            && let Some(f) = inner.routes.robots
        {
            return f().await.into_response();
        }
    }

    // Intercepting routes apply to client-side navigations only.
    if let (Some(from), Some("1")) = (req.header("x-nr-from").map(str::to_owned), req.header("x-nr-nav")) {
        for ic in &inner.intercepts {
            if let Some(hit) = ic.target.at(&path)
                && context_matches(&ic.context, &from)
            {
                req.set_params(hit.params);
                let idx = ic.page;
                // Tells the client runtime this HTML depends on the page it
                // navigated from, so it isn't reused from other pages.
                let mut res = run_page(&inner, idx, req).await;
                res.set_header("x-nr-intercepted", "1");
                return res;
            }
        }
    }

    let hit = inner.matcher.at(&path).map(|m| (*m.value, m.params));
    match hit {
        Some((Target::Page(i), params)) => {
            if req.method() != Method::GET && req.method() != Method::HEAD {
                return Response::text("Method Not Allowed").with_status(405).with_header("allow", "GET, HEAD");
            }
            req.set_params(params);
            run_page(&inner, i, req).await
        }
        Some((Target::Api(i), params)) => {
            req.set_params(params);
            let stack = inner.api_stacks[i].clone();
            let endpoint = ApiEndpoint { inner: inner.clone(), index: i };
            Next::new(stack, Arc::new(endpoint)).run(req).await
        }
        Some((Target::Custom(i), params)) => {
            req.set_params(params);
            let handlers = &inner.customs[i];
            let method = req.method().clone();
            let found = handlers.iter().find(|(m, _)| {
                m.as_ref().is_none_or(|m| *m == method || (method == Method::HEAD && *m == Method::GET))
            });
            match found {
                Some((_, ep)) => {
                    let head = method == Method::HEAD;
                    let res = ep.call(req).await;
                    if head { strip_body(res) } else { res }
                }
                None => method_not_allowed(handlers.iter().filter_map(|(m, _)| m.clone()).collect()),
            }
        }
        None => {
            if (req.method() == Method::GET || req.method() == Method::HEAD)
                && let Some(res) = crate::static_files::serve_public(&inner, &req).await
            {
                return res;
            }
            crate::render::not_found_response(&inner, &req).await
        }
    }
}

fn context_matches(context: &RoutePattern, from: &str) -> bool {
    let from = from.split('?').next().unwrap_or("");
    let segs: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let n = context.segments().len();
    if segs.len() < n {
        return false;
    }
    let mut m = Matcher::new();
    if m.insert(context.segments(), ()).is_err() {
        return false;
    }
    m.at_segments(&segs[..n]).is_some()
}

async fn run_page(inner: &Arc<AppInner>, index: usize, req: Request) -> Response {
    let stack = inner.page_stacks[index].clone();
    let head = req.method() == Method::HEAD;
    let endpoint = PageEndpoint { inner: inner.clone(), index };
    let res = Next::new(stack, Arc::new(endpoint)).run(req).await;
    if head { strip_body(res) } else { res }
}

struct PageEndpoint {
    inner: Arc<AppInner>,
    index: usize,
}

impl Endpoint for PageEndpoint {
    fn call(&self, req: Request) -> BoxFuture<Response> {
        let inner = self.inner.clone();
        let index = self.index;
        Box::pin(async move { crate::render::page_response(&inner, index, req).await })
    }
}

struct ApiEndpoint {
    inner: Arc<AppInner>,
    index: usize,
}

impl Endpoint for ApiEndpoint {
    fn call(&self, req: Request) -> BoxFuture<Response> {
        let inner = self.inner.clone();
        let index = self.index;
        Box::pin(async move {
            let api = &inner.routes.apis[index];
            let method = req.method().clone();
            if let Some((_, h)) = api.handlers.iter().find(|(m, _)| *m == method) {
                return h(req).await;
            }
            if method == Method::HEAD
                && let Some((_, h)) = api.handlers.iter().find(|(m, _)| *m == Method::GET)
            {
                return strip_body(h(req).await);
            }
            let mut allowed: Vec<Method> = api.handlers.iter().map(|(m, _)| m.clone()).collect();
            if method == Method::OPTIONS {
                allowed.push(Method::OPTIONS);
                let list = allowed.iter().map(Method::as_str).collect::<Vec<_>>().join(", ");
                return Response::status(204).with_header("allow", &list);
            }
            method_not_allowed(allowed)
        })
    }
}

fn method_not_allowed(allowed: Vec<Method>) -> Response {
    let mut list: Vec<&str> = allowed.iter().map(Method::as_str).collect();
    if list.contains(&"GET") && !list.contains(&"HEAD") {
        list.push("HEAD");
    }
    Response::text("Method Not Allowed")
        .with_status(StatusCode::METHOD_NOT_ALLOWED.as_u16())
        .with_header("allow", &list.join(", "))
}

fn strip_body(mut res: Response) -> Response {
    res.body = crate::response::Body::Empty;
    res
}
