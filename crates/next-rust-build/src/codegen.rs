//! Project analysis and Rust code generation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use next_rust_core::{Config, Diagnostic, Diagnostics, RenderingMode};
use next_rust_router::manifest::{RouteManifest, relative};
use next_rust_router::{Route, RouteKind, ScanOutput, SegmentEntry, scan_project};

use crate::analyze::{ArgKind, FileInfo, FnInfo, analyze_file, scan_src_actions};

pub const HTTP_METHODS: &[&str] = &["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];

/// A route with build-time analysis results.
#[derive(Debug, Clone)]
pub struct AnalyzedRoute {
    pub route: Route,
    /// `static` or `dynamic` (pages) / `dynamic` (API routes).
    pub rendering: &'static str,
    /// Why the route is dynamic (for `next-rust routes` / analyze).
    pub dynamic_reason: Option<String>,
    pub methods: Vec<String>,
    pub revalidate: Option<u64>,
}

/// Everything known about a project at build time.
#[derive(Debug, Clone)]
pub struct Project {
    pub config: Config,
    pub scan: ScanOutput,
    pub files: BTreeMap<PathBuf, FileInfo>,
    pub routes: Vec<AnalyzedRoute>,
    /// (module path or file, fn name, argument count, file for app-dir actions)
    pub actions: Vec<ActionSource>,
    pub diagnostics: Diagnostics,
}

#[derive(Debug, Clone)]
pub struct ActionSource {
    /// `Some(file)` for actions declared in app-directory special files.
    pub file: Option<PathBuf>,
    /// `crate::...` module path for actions in `src/`.
    pub module: Option<String>,
    pub name: String,
    pub argc: usize,
}

impl Project {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    pub fn manifest(&self) -> RouteManifest {
        let mut m =
            RouteManifest::new(&self.routes.iter().map(|r| r.route.clone()).collect::<Vec<_>>(), &self.config.root);
        for (entry, r) in m.routes.iter_mut().zip(&self.routes) {
            entry.rendering = r.rendering.to_owned();
            entry.revalidate = r.revalidate;
            if r.route.kind == RouteKind::Api {
                entry.methods = r.methods.clone();
            }
        }
        m
    }
}

/// Scan and analyze a project.
pub fn analyze_project(config: &Config) -> Project {
    let scan = scan_project(config);
    let mut diagnostics = scan.diagnostics.clone();
    let mut files = BTreeMap::new();

    let mut rs_files: BTreeSet<PathBuf> = BTreeSet::new();
    for tree in &scan.trees {
        collect_rs(tree, &mut rs_files);
    }
    for path in &rs_files {
        match analyze_file(path) {
            Ok(info) => {
                files.insert(path.clone(), info);
            }
            Err(d) => diagnostics.push(d),
        }
    }

    let mut project =
        Project { config: config.clone(), scan, files, routes: Vec::new(), actions: Vec::new(), diagnostics };
    if project.diagnostics.has_errors() {
        return project;
    }
    check_exports(&mut project);

    let routes = project.scan.routes.clone();
    for route in routes {
        let analyzed = analyze_route(&project, route);
        project.routes.push(analyzed);
    }

    for (path, info) in &project.files {
        for (name, argc) in &info.actions {
            project.actions.push(ActionSource {
                file: Some(path.clone()),
                module: None,
                name: name.clone(),
                argc: *argc,
            });
        }
    }
    let src = config.root.join("src");
    if src.is_dir() {
        let skip: Vec<PathBuf> = rs_files.iter().cloned().collect();
        let (found, diags) = scan_src_actions(&src, &skip);
        project.diagnostics.0.extend(diags);
        for (module, name, argc) in found {
            project.actions.push(ActionSource { file: None, module: Some(module), name, argc });
        }
    }
    for a in &project.actions {
        if a.argc > 2 {
            project.diagnostics.push(
                Diagnostic::error("NR0210", format!("Server action `{}` has too many arguments", a.name))
                    .message("Server actions take zero arguments, one input argument, or (ActionContext, input).")
                    .help("group the inputs into one struct deriving `Deserialize`"),
            );
        }
    }
    project
}

fn collect_rs(node: &next_rust_router::RouteNode, out: &mut BTreeSet<PathBuf>) {
    for f in node.files.iter() {
        if f.extension().is_some_and(|e| e == "rs") {
            out.insert(f.clone());
        }
    }
    for c in &node.children {
        collect_rs(c, out);
    }
}

fn required_export(file_name: &str) -> Option<&'static str> {
    Some(match file_name {
        "page.rs" | "default.rs" => "Page",
        "layout.rs" => "Layout",
        "template.rs" => "Template",
        "loading.rs" => "Loading",
        "error.rs" => "ErrorBoundary",
        "global-error.rs" => "GlobalError",
        "not-found.rs" => "NotFound",
        "middleware.rs" => "middleware",
        "metadata.rs" => "metadata",
        "sitemap.rs" => "sitemap",
        "robots.rs" => "robots",
        _ => return None,
    })
}

fn check_exports(project: &mut Project) {
    let mut diags = Vec::new();
    for (path, info) in &project.files {
        let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or_default();
        if file_name == "route.rs" {
            if !HTTP_METHODS.iter().any(|m| info.get(m).is_some()) {
                diags.push(
                    Diagnostic::error("NR0203", "route.rs exports no HTTP handler")
                        .location(path)
                        .message(
                            "An API route must export at least one of GET, POST, PUT, PATCH, DELETE, OPTIONS or HEAD.",
                        )
                        .help("pub async fn GET(req: Request) -> Response { Response::json(&\"ok\") }"),
                );
            }
            for m in HTTP_METHODS {
                if let Some(f) = info.get(m)
                    && (f.args.len() > 1 || f.args.first().is_some_and(|a| *a != ArgKind::Request))
                {
                    diags.push(
                        Diagnostic::error("NR0205", format!("Invalid signature for `{m}`"))
                            .location(format!("{}:{}", path.display(), f.line))
                            .message("API handlers take either no arguments or a single `Request`.")
                            .help(format!("pub async fn {m}(req: Request) -> Response")),
                    );
                }
            }
            continue;
        }
        let Some(export) = required_export(file_name) else { continue };
        match info.get(export) {
            None => {
                let hint = match export {
                    "Page" => "pub fn Page() -> impl View { h1![\"Hello\"] }",
                    "Layout" => "pub fn Layout(children: Children) -> impl View { div![children] }",
                    "Template" => "pub fn Template(children: Children) -> impl View { div![children] }",
                    "Loading" => "pub fn Loading() -> impl View { p![\"Loading…\"] }",
                    "ErrorBoundary" => "pub fn ErrorBoundary(info: ErrorInfo) -> impl View { p![info.message] }",
                    "GlobalError" => {
                        "pub fn GlobalError(info: ErrorInfo) -> impl View { h1![\"Something went wrong\"] }"
                    }
                    "NotFound" => "pub fn NotFound() -> impl View { h1![\"Not found\"] }",
                    "middleware" => {
                        "pub async fn middleware(req: Request, next: Next) -> Response { next.run(req).await }"
                    }
                    "metadata" => "pub fn metadata() -> Metadata { Metadata::new().title(\"…\") }",
                    "sitemap" => "pub async fn sitemap() -> Sitemap { Sitemap::new().url(\"https://example.com/\") }",
                    _ => "pub async fn robots() -> Robots { Robots::allow_all() }",
                };
                let private = info.fns.contains_key(export);
                diags.push(
                    Diagnostic::error("NR0201", format!("`{file_name}` must export `pub fn {export}`"))
                        .location(path)
                        .message(if private {
                            format!("`{export}` exists but is not `pub`.")
                        } else {
                            format!("No public function named `{export}` was found.")
                        })
                        .help(hint),
                );
            }
            Some(f) => {
                let sync_only = matches!(export, "Loading" | "ErrorBoundary" | "GlobalError");
                if sync_only && f.is_async {
                    diags.push(
                        Diagnostic::error("NR0204", format!("`{export}` must not be async"))
                            .location(format!("{}:{}", path.display(), f.line))
                            .message("Loading and error UIs render immediately and cannot wait for data."),
                    );
                }
                if export == "middleware" && !(f.args == [ArgKind::Request, ArgKind::Next]) {
                    diags.push(
                        Diagnostic::error("NR0205", "Invalid middleware signature")
                            .location(format!("{}:{}", path.display(), f.line))
                            .help("pub async fn middleware(req: Request, next: Next) -> Response"),
                    );
                }
                for (fn_name, fi) in &info.fns {
                    if !fi.is_pub {
                        continue;
                    }
                    for arg in &fi.args {
                        let bad = match arg {
                            ArgKind::Children => !matches!(fn_name.as_str(), "Layout" | "Template"),
                            ArgKind::Slots => fn_name != "Layout",
                            ArgKind::ErrorInfo => !matches!(fn_name.as_str(), "ErrorBoundary" | "GlobalError"),
                            ArgKind::Data => !info.fns.contains_key("load"),
                            _ => false,
                        };
                        if bad
                            && matches!(
                                fn_name.as_str(),
                                "Page"
                                    | "Layout"
                                    | "Template"
                                    | "Loading"
                                    | "ErrorBoundary"
                                    | "GlobalError"
                                    | "NotFound"
                                    | "metadata"
                            )
                        {
                            diags.push(
                                Diagnostic::error("NR0206", format!("`{fn_name}` cannot receive this argument"))
                                    .location(format!("{}:{}", path.display(), fi.line))
                                    .message(match arg {
                                        ArgKind::Children => "`Children` is only passed to `Layout` and `Template`.".to_owned(),
                                        ArgKind::Slots => "`Slots` is only passed to `Layout`.".to_owned(),
                                        ArgKind::ErrorInfo => "`ErrorInfo` is only passed to error boundaries.".to_owned(),
                                        _ => "`Data<T>` requires a `pub async fn load(..) -> Result<T>` in the same file.".to_owned(),
                                    }),
                            );
                        }
                    }
                }
            }
        }
    }
    project.diagnostics.0.extend(diags);
}

fn fn_of<'a>(project: &'a Project, path: &Option<PathBuf>, name: &str) -> Option<&'a FnInfo> {
    path.as_ref().and_then(|p| project.files.get(p)).and_then(|f| f.get(name))
}

fn segment_metadata_fn<'a>(project: &'a Project, seg: &'a SegmentEntry) -> Option<(&'a PathBuf, &'a FnInfo)> {
    if let Some(p) = &seg.metadata
        && let Some(f) = project.files.get(p).and_then(|i| i.get("metadata"))
    {
        return Some((p, f));
    }
    let p = seg.layout.as_ref()?;
    project.files.get(p).and_then(|i| i.get("metadata")).map(|f| (p, f))
}

fn analyze_route(project: &Project, route: Route) -> AnalyzedRoute {
    if route.kind == RouteKind::Api {
        let info = project.files.get(&route.source);
        let methods =
            HTTP_METHODS.iter().filter(|m| info.is_some_and(|i| i.get(m).is_some())).map(|m| (*m).to_owned()).collect();
        return AnalyzedRoute {
            route,
            rendering: "dynamic",
            dynamic_reason: Some("API route".into()),
            methods,
            revalidate: None,
        };
    }
    let page_info = project.files.get(&route.source);
    let revalidate = page_info.and_then(FileInfo::revalidate_literal).or(project.config.rendering.revalidate);
    let explicit = page_info.and_then(FileInfo::rendering);
    let default = match project.config.rendering.default {
        RenderingMode::Auto => "auto",
        RenderingMode::Static => "static",
        RenderingMode::Dynamic => "dynamic",
    };
    let requested = explicit.unwrap_or(default);
    let (rendering, reason) = match requested {
        "static" => ("static", None),
        "dynamic" => ("dynamic", Some("rendering = dynamic".to_owned())),
        _ => match auto_dynamic_reason(project, &route) {
            Some(reason) => ("dynamic", Some(reason)),
            None => ("static", None),
        },
    };
    let methods = vec!["GET".into(), "HEAD".into()];
    AnalyzedRoute { route, rendering, dynamic_reason: reason, methods, revalidate }
}

/// `None` when the route can be rendered statically.
fn auto_dynamic_reason(project: &Project, route: &Route) -> Option<String> {
    if route.intercept.is_some() {
        return Some("intercepting route".into());
    }
    if route.kind == RouteKind::Html {
        return None;
    }
    let page = project.files.get(&route.source)?;
    if route.pattern.is_dynamic() && page.get("generate_params").is_none() {
        return Some("dynamic segment without generate_params".into());
    }
    let mut fns: Vec<(PathBuf, &FnInfo)> = Vec::new();
    for seg in &route.chain {
        for (file, name) in [
            (&seg.layout, "Layout"),
            (&seg.template, "Template"),
            (&seg.loading, "Loading"),
            (&seg.not_found, "NotFound"),
        ] {
            if let (Some(p), Some(f)) = (file, fn_of(project, file, name)) {
                fns.push((p.clone(), f));
            }
        }
        let meta_file = seg
            .metadata
            .clone()
            .filter(|p| project.files.get(p).is_some_and(|i| i.get("metadata").is_some()))
            .or_else(|| seg.layout.clone());
        if let (Some(p), Some(f)) = (&meta_file, fn_of(project, &meta_file, "metadata")) {
            fns.push((p.clone(), f));
        }
        for slot in &seg.slots {
            let file = slot.page.clone().or_else(|| slot.default.clone());
            if let (Some(p), Some(f)) = (&file, fn_of(project, &file, "Page")) {
                fns.push((p.clone(), f));
            }
        }
    }
    for name in ["Page", "metadata", "load"] {
        if let Some(f) = page.get(name) {
            fns.push((route.source.clone(), f));
        }
    }
    for (path, f) in fns {
        for arg in &f.args {
            if let ArgKind::Extractor { name, static_safe: false } = arg {
                return Some(format!("`{name}` in {}::{}", relative(&path, &project.config.root), f.name));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

struct Gen<'a> {
    project: &'a Project,
    modules: BTreeMap<PathBuf, usize>,
    wrappers: BTreeSet<String>,
    out: String,
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

impl<'a> Gen<'a> {
    fn module(&mut self, path: &Path) -> String {
        let next = self.modules.len();
        let idx = *self.modules.entry(path.to_path_buf()).or_insert(next);
        format!("m{idx}")
    }

    fn info(&self, path: &Path) -> &'a FileInfo {
        &self.project.files[path]
    }

    /// Argument expressions. `fallible` selects `?` (inside `Result`
    /// contexts) versus returning an empty node.
    fn args(&self, f: &FnInfo, fallible: bool) -> String {
        f.args
            .iter()
            .map(|a| match a {
                ArgKind::Children => "children".to_owned(),
                ArgKind::Slots => "slots".to_owned(),
                ArgKind::Data => "__nr::Data(__data)".to_owned(),
                ArgKind::ErrorInfo => "info".to_owned(),
                ArgKind::Request => "req".to_owned(),
                ArgKind::Next => "next".to_owned(),
                ArgKind::ActionContext => {
                    "compile_error!(\"ActionContext is only available in server actions\")".to_owned()
                }
                ArgKind::Extractor { .. } if fallible => "__nr::FromContext::from_context(&ctx)?".to_owned(),
                ArgKind::Extractor { .. } => {
                    "match __nr::FromContext::from_context(&ctx) { Ok(v) => v, Err(_) => return __nr::Node::Empty }"
                        .to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn call(&self, module: &str, f: &FnInfo, fallible: bool) -> String {
        format!("{module}::{}({}){}", f.name, self.args(f, fallible), if f.is_async { ".await" } else { "" })
    }

    fn emit_once(&mut self, name: &str, body: String) {
        if self.wrappers.insert(name.to_owned()) {
            self.out.push_str(&body);
            self.out.push('\n');
        }
    }

    /// RenderFn for `Page`/`NotFound` of a file.
    fn render_fn(&mut self, path: &Path, export: &str) -> Option<String> {
        let f = self.info(path).get(export)?.clone();
        let m = self.module(path);
        let name = format!("__render_{m}_{export}");
        let load = if f.args.contains(&ArgKind::Data) {
            let load = self.info(path).get("load").cloned();
            match load {
                Some(load) => format!("        let __data = {}?;\n", self.call(&m, &load, true)),
                None => String::new(),
            }
        } else {
            String::new()
        };
        let body = format!(
            "#[allow(unused_variables)]\nfn {name}(ctx: __nr::Ctx) -> __nr::BoxFuture<__nr::Result<__nr::Node>> {{\n    ::std::boxed::Box::pin(async move {{\n{load}        __nr::IntoViewResult::<__nr::Error>::into_view_result({})\n    }})\n}}\n",
            self.call(&m, &f, true)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn layout_fn(&mut self, path: &Path, export: &str) -> Option<String> {
        let f = self.info(path).get(export)?.clone();
        let m = self.module(path);
        let name = format!("__layout_{m}_{export}");
        let load = match (f.args.contains(&ArgKind::Data), self.info(path).get("load").cloned()) {
            (true, Some(load)) => format!("        let __data = {}?;\n", self.call(&m, &load, true)),
            _ => String::new(),
        };
        let body = format!(
            "#[allow(unused_variables, unused_mut)]\nfn {name}(ctx: __nr::Ctx, children: __nr::Children, mut slots: __nr::Slots) -> __nr::BoxFuture<__nr::Result<__nr::Node>> {{\n    ::std::boxed::Box::pin(async move {{\n{load}        __nr::IntoViewResult::<__nr::Error>::into_view_result({})\n    }})\n}}\n",
            self.call(&m, &f, true)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn loading_fn(&mut self, path: &Path) -> Option<String> {
        let f = self.info(path).get("Loading")?.clone();
        let m = self.module(path);
        let name = format!("__loading_{m}");
        let body = format!(
            "#[allow(unused_variables)]\nfn {name}(ctx: __nr::Ctx) -> __nr::Node {{\n    __nr::View::into_node({})\n}}\n",
            self.call(&m, &f, false)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn error_fn(&mut self, path: &Path, export: &str) -> Option<String> {
        let f = self.info(path).get(export)?.clone();
        let m = self.module(path);
        let name = format!("__error_{m}_{export}");
        let body = format!(
            "#[allow(unused_variables)]\nfn {name}(ctx: __nr::Ctx, info: __nr::ErrorInfo) -> __nr::Node {{\n    __nr::View::into_node({})\n}}\n",
            self.call(&m, &f, false)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn metadata_fn(&mut self, path: &Path) -> Option<String> {
        let f = self.info(path).get("metadata")?.clone();
        let m = self.module(path);
        let name = format!("__metadata_{m}");
        let body = format!(
            "#[allow(unused_variables)]\nfn {name}(ctx: __nr::Ctx) -> __nr::BoxFuture<__nr::Result<__nr::Metadata>> {{\n    ::std::boxed::Box::pin(async move {{\n        __nr::IntoResult::<__nr::Metadata>::into_result({})\n    }})\n}}\n",
            self.call(&m, &f, true)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn middleware_fn(&mut self, path: &Path) -> Option<String> {
        let f = self.info(path).get("middleware")?.clone();
        let m = self.module(path);
        let name = format!("__middleware_{m}");
        let body = format!(
            "fn {name}(req: __nr::Request, next: __nr::Next) -> __nr::BoxFuture<__nr::Response> {{\n    ::std::boxed::Box::pin(async move {{ __nr::IntoResponse::into_response({}) }})\n}}\n",
            self.call(&m, &f, true)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn api_fn(&mut self, path: &Path, method: &str) -> Option<String> {
        let f = self.info(path).get(method)?.clone();
        let m = self.module(path);
        let name = format!("__api_{m}_{method}");
        let body = format!(
            "#[allow(unused_variables)]\nfn {name}(req: __nr::Request) -> __nr::BoxFuture<__nr::Response> {{\n    ::std::boxed::Box::pin(async move {{ __nr::IntoResponse::into_response({}) }})\n}}\n",
            self.call(&m, &f, true)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn simple_async<T: std::fmt::Display>(&mut self, path: &Path, export: &str, ty: T, on_err: &str) -> Option<String> {
        let f = self.info(path).get(export)?.clone();
        let m = self.module(path);
        let name = format!("__{export}_{m}");
        let body = format!(
            "fn {name}() -> __nr::BoxFuture<{ty}> {{\n    ::std::boxed::Box::pin(async move {{\n        match __nr::IntoResult::<{inner}>::into_result({call}) {{ Ok(v) => v, Err(e) => {on_err} }}\n    }})\n}}\n",
            inner = ty.to_string().trim_start_matches("__nr::Result<").trim_end_matches('>'),
            call = self.call(&m, &f, true)
        );
        self.emit_once(&name, body);
        Some(name)
    }

    fn opt(v: Option<String>) -> String {
        v.map(|s| format!("Some({s})")).unwrap_or_else(|| "None".to_owned())
    }

    fn segment(&mut self, seg: &SegmentEntry, is_app_root: bool) -> String {
        let layout = seg.layout.as_ref().and_then(|p| self.layout_fn(p, "Layout"));
        let template = seg.template.as_ref().and_then(|p| self.layout_fn(p, "Template"));
        let loading = seg.loading.as_ref().and_then(|p| self.loading_fn(p));
        let error = seg.error.as_ref().and_then(|p| self.error_fn(p, "ErrorBoundary"));
        let not_found = seg.not_found.as_ref().and_then(|p| self.render_fn(p, "NotFound"));
        let metadata =
            segment_metadata_fn(self.project, seg).map(|(p, _)| p.clone()).and_then(|p| self.metadata_fn(&p));
        let middleware = if is_app_root { None } else { seg.middleware.as_ref().and_then(|p| self.middleware_fn(p)) };
        let slots: Vec<String> = seg
            .slots
            .iter()
            .map(|s| {
                let render = s
                    .page
                    .as_ref()
                    .and_then(|p| self.render_fn(p, "Page"))
                    .or_else(|| s.default.as_ref().and_then(|p| self.render_fn(p, "Page")));
                format!("__nr::SlotDef {{ name: {}, render: {} }}", lit(&s.name), Self::opt(render))
            })
            .collect();
        format!(
            "__nr::SegmentDef {{ layout: {}, template: {}, loading: {}, error: {}, not_found: {}, metadata: {}, middleware: {}, slots: vec![{}] }}",
            Self::opt(layout),
            Self::opt(template),
            Self::opt(loading),
            Self::opt(error),
            Self::opt(not_found),
            Self::opt(metadata),
            Self::opt(middleware),
            slots.join(", ")
        )
    }
}

/// Generate the Rust source included by `next_rust::app!()`.
/// Options for [`generate_code_with`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CodegenOptions {
    /// Embed `page.html` files minified instead of including them verbatim.
    /// Enabled automatically for release builds.
    pub minify_html: bool,
}

pub fn generate_code(project: &Project) -> String {
    generate_code_with(project, CodegenOptions::default())
}

/// Generate code with explicit options.
pub fn generate_code_with(project: &Project, options: CodegenOptions) -> String {
    let mut g = Gen { project, modules: BTreeMap::new(), wrappers: BTreeSet::new(), out: String::new() };
    let root = &project.config.root;
    let app_root = project.config.app_dir();
    let mut pages = Vec::new();
    let mut apis = Vec::new();

    for ar in &project.routes {
        let r = &ar.route;
        let source = relative(&r.source, root);
        let is_app_tree = r.chain.first().is_some_and(|c| c.dir == app_root);
        let segments: Vec<String> =
            r.chain.iter().enumerate().map(|(i, s)| g.segment(s, i == 0 && is_app_tree)).collect();
        match r.kind {
            RouteKind::Api => {
                let handlers: Vec<String> = HTTP_METHODS
                    .iter()
                    .filter_map(|m| {
                        g.api_fn(&r.source, m).map(|f| format!("(__nr::Method::{m}, {f} as __nr::HandlerFn)"))
                    })
                    .collect();
                let middleware: Vec<String> = r
                    .chain
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !(*i == 0 && is_app_tree))
                    .filter_map(|(_, s)| s.middleware.as_ref().and_then(|p| g.middleware_fn(p)))
                    .map(|f| format!("{f} as __nr::MiddlewareFn"))
                    .collect();
                apis.push(format!(
                    "        __nr::ApiDef {{ pattern: {}, source: {}, handlers: vec![{}], middleware: vec![{}] }}",
                    lit(&r.pattern.to_fs_string()),
                    lit(&source),
                    handlers.join(", "),
                    middleware.join(", ")
                ));
            }
            RouteKind::Page | RouteKind::Html => {
                let (body, metadata, gen_params, revalidate, dynamic_params, tags) = if r.kind == RouteKind::Html {
                    (
                        match std::fs::read_to_string(&r.source) {
                            Ok(html) if options.minify_html => {
                                format!("__nr::PageBody::Html({})", lit(&next_rust_assets::html::minify(&html)))
                            }
                            _ => format!("__nr::PageBody::Html(include_str!({}))", lit(&r.source.to_string_lossy())),
                        },
                        None,
                        None,
                        ar.revalidate.map(|v| format!("Some({v})")).unwrap_or_else(|| "None".into()),
                        "true".to_owned(),
                        "&[]".to_owned(),
                    )
                } else {
                    let info = &project.files[&r.source];
                    let m = g.module(&r.source);
                    let render = g.render_fn(&r.source, "Page").unwrap_or_default();
                    let gp = info.get("generate_params").map(|f| {
                        let wrapper = format!("__generate_params_{m}");
                        g.emit_once(
                            &wrapper,
                            format!(
                                "fn {wrapper}() -> __nr::BoxFuture<__nr::Result<::std::vec::Vec<__nr::Params>>> {{\n    ::std::boxed::Box::pin(async move {{ __nr::IntoResult::<::std::vec::Vec<__nr::Params>>::into_result({m}::generate_params(){}) }})\n}}\n",
                                if f.is_async { ".await" } else { "" }
                            ),
                        );
                        wrapper
                    });
                    (
                        format!("__nr::PageBody::Rust({render})"),
                        g.metadata_fn(&r.source),
                        gp,
                        if info.consts.contains_key("REVALIDATE") {
                            format!("Some({m}::REVALIDATE as u64)")
                        } else {
                            ar.revalidate.map(|v| format!("Some({v})")).unwrap_or_else(|| "None".into())
                        },
                        if info.consts.contains_key("DYNAMIC_PARAMS") {
                            format!("{m}::DYNAMIC_PARAMS")
                        } else {
                            "true".into()
                        },
                        if info.consts.contains_key("TAGS") { format!("{m}::TAGS") } else { "&[]".into() },
                    )
                };
                pages.push(format!(
                    "        __nr::PageDef {{\n            pattern: {},\n            source: {},\n            body: {},\n            segments: vec![\n                {}\n            ],\n            metadata: {},\n            rendering: __nr::Rendering::{},\n            revalidate: {},\n            generate_params: {},\n            dynamic_params: {},\n            tags: {},\n            intercept_from: {},\n        }}",
                    lit(&r.pattern.to_fs_string()),
                    lit(&source),
                    body,
                    segments.join(",\n                "),
                    Gen::opt(metadata),
                    if ar.rendering == "static" { "Static" } else { "Dynamic" },
                    revalidate,
                    Gen::opt(gen_params),
                    dynamic_params,
                    tags,
                    r.intercept.as_ref().map(|i| format!("Some({})", lit(&i.context.to_fs_string()))).unwrap_or_else(|| "None".into()),
                ));
            }
        }
    }

    // Root segment (404 page and root layout for unmatched URLs).
    let root_tree = project.scan.trees.first();
    let root_seg = root_tree.map(|t| SegmentEntry {
        dir: t.dir.clone(),
        name: String::new(),
        layout: t.files.layout.clone(),
        template: None,
        loading: None,
        error: None,
        not_found: t.files.not_found.clone(),
        middleware: None,
        metadata: t.files.metadata.clone(),
        global_error: None,
        slots: Vec::new(),
    });
    let root_def = match &root_seg {
        Some(s) => g.segment(s, true),
        None => "::std::default::Default::default()".into(),
    };
    let root_mw = root_tree.and_then(|t| t.files.middleware.clone()).and_then(|p| g.middleware_fn(&p));
    let global_error = root_tree.and_then(|t| t.files.global_error.clone()).and_then(|p| g.error_fn(&p, "GlobalError"));
    let sitemap = root_tree.and_then(|t| t.files.sitemap.clone()).and_then(|p| {
        g.simple_async(
            &p,
            "sitemap",
            "__nr::Sitemap",
            "{ __nr::log_error(&format!(\"sitemap failed: {e}\")); ::std::default::Default::default() }",
        )
    });
    let robots = root_tree.and_then(|t| t.files.robots.clone()).and_then(|p| {
        g.simple_async(
            &p,
            "robots",
            "__nr::Robots",
            "{ __nr::log_error(&format!(\"robots failed: {e}\")); ::std::default::Default::default() }",
        )
    });

    let mut actions = Vec::new();
    for a in &project.actions {
        let prefix = match (&a.file, &a.module) {
            (Some(file), _) => g.module(file),
            (None, Some(module)) => module.clone(),
            _ => continue,
        };
        actions.push(format!(
            "        __nr::ActionDef {{ id: {prefix}::__NR_ACTION_ID_{name}, handler: {prefix}::__nr_action_{name} }}",
            name = a.name
        ));
    }

    let mut header = String::from(
        "// @generated by next-rust-build — do not edit.\n// Regenerated whenever files in the app directory change.\n\n#[allow(unused_imports)]\nuse ::next_rust::__private as __nr;\n\n",
    );
    for (path, idx) in &g.modules {
        let _ = writeln!(
            header,
            "#[allow(non_snake_case, dead_code, unused_imports, clippy::all)]\n#[path = {}]\nmod m{idx};",
            lit(&path.to_string_lossy())
        );
    }
    header.push('\n');

    let mut out = header;
    out.push_str(&g.out);
    let _ = write!(
        out,
        "/// All routes discovered in the app directory.\npub fn routes() -> ::next_rust::Routes {{\n    __nr::Routes {{\n        pages: vec![\n{}\n        ],\n        apis: vec![\n{}\n        ],\n        actions: vec![\n{}\n        ],\n        root: {},\n        middleware: {},\n        global_error: {},\n        sitemap: {},\n        robots: {},\n        project_root: Some({}),\n    }}\n}}\n",
        pages.join(",\n"),
        apis.join(",\n"),
        actions.join(",\n"),
        root_def,
        Gen::opt(root_mw),
        Gen::opt(global_error),
        Gen::opt(sitemap),
        Gen::opt(robots),
        lit(&root.to_string_lossy()),
    );
    out
}
