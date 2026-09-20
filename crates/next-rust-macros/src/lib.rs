//! Procedural macros for Next Rust.
//!
//! Every macro here expands to plain, readable Rust; see each macro's
//! documentation for its exact expansion.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{FnArg, ItemFn, LitStr, Pat, parse_macro_input};

mod jobs;

fn manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// Absolute path of the source file invoking the macro. Relative paths
/// reported by the compiler are relative to its working directory (the
/// workspace root for workspace members), not to the crate.
fn caller_file() -> Option<PathBuf> {
    let file = proc_macro::Span::call_site().local_file()?;
    if file.is_absolute() {
        return Some(file);
    }
    let from_cwd = std::env::current_dir().ok().map(|d| d.join(&file));
    match from_cwd {
        Some(p) if p.exists() => Some(p),
        _ => Some(manifest_dir().join(file)),
    }
}

/// Directory of the source file invoking the macro (falls back to the crate root).
fn caller_dir() -> PathBuf {
    caller_file().and_then(|f| f.parent().map(Path::to_path_buf)).unwrap_or_else(manifest_dir)
}

fn caller_file_relative() -> Option<String> {
    let abs = caller_file()?;
    let abs = std::fs::canonicalize(&abs).unwrap_or(abs);
    let manifest = std::fs::canonicalize(manifest_dir()).unwrap_or_else(|_| manifest_dir());
    let rel = abs.strip_prefix(&manifest).map(Path::to_path_buf).unwrap_or(abs);
    Some(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"))
}

fn error(span: Span, msg: impl std::fmt::Display) -> TokenStream {
    syn::Error::new(span, msg).to_compile_error().into()
}

// ---------------------------------------------------------------------------
// #[client]
// ---------------------------------------------------------------------------

struct ClientArgs {
    module: Option<LitStr>,
}

impl Parse for ClientArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ClientArgs { module: None });
        }
        let key: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        let value: LitStr = input.parse()?;
        if key == "module" {
            return Ok(ClientArgs { module: Some(value) });
        }
        if key == "script" {
            // A script in `client/`, or one declared with `client_script!`.
            let name = value.value();
            if let Err(e) = script_name(&name) {
                return Err(syn::Error::new(value.span(), e));
            }
            return Ok(ClientArgs { module: Some(LitStr::new(&format!("/_next-rust/client/{name}"), value.span())) });
        }
        if key == "wasm" {
            // An island compiled from the project's `islands/` crate. The
            // build writes a one-line glue module per island beside the
            // shared loader and the one `.wasm` every island shares.
            let name = value.value();
            if let Err(e) = wasm_island_name(&name) {
                return Err(syn::Error::new(value.span(), e));
            }
            return Ok(ClientArgs {
                module: Some(LitStr::new(&format!("/_next-rust/client/__nrw/{name}.js"), value.span())),
            });
        }
        Err(syn::Error::new(
            key.span(),
            "expected `module = \"/path/to/island.js\"`, `script = \"island.js\"` or `wasm = \"IslandName\"`",
        ))
    }
}

/// Mark a component as an interactive island.
///
/// The component is rendered on the server as usual and wrapped in
/// `<nr-island data-component=".." data-props="..">`. Its arguments are
/// serialized (they must implement `serde::Serialize`) and become the
/// island's initial client state.
///
/// Without `module`, the island uses the built-in declarative runtime
/// (`data-nr-text`, `data-nr-on-click="increment:count"`, ...). With
/// `#[client(module = "/_next-rust/client/islands/chart.js")]` the runtime imports
/// that ES module (for example `wasm-bindgen` output) and calls its
/// `hydrate(element, props)` export. `#[client(script = "chart.js")]` is
/// short for the module `client/chart.js`, a file or a script declared in
/// Rust with `client_script!("chart.js", r#"…"#)`.
///
/// `#[client(wasm = "Chart")]` takes the island's behaviour from the
/// project's `islands/` crate instead, which `[build] wasm_islands = true`
/// compiles to one WebAssembly module shared by every island on the site;
/// the name is the `hydrate_Chart` export in it. The logic then ships as a
/// binary rather than as JavaScript, which is much harder to read — but it
/// is **not hidden**: the module can be downloaded and disassembled, and its
/// string literals stay readable. It is not a security boundary, so props
/// are still public and secrets still belong on the server.
///
/// Expansion (simplified):
///
/// ```ignore
/// pub fn Counter(count: i32) -> ::next_rust::Node {
///     let props = /* {"count": count} as JSON */;
///     fn inner(count: i32) -> impl View { /* original body */ }
///     ::next_rust::__private::island("Counter", None, props, inner(count))
/// }
/// ```
#[proc_macro_attribute]
pub fn client(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ClientArgs);
    let func = parse_macro_input!(item as ItemFn);
    if func.sig.asyncness.is_some() {
        return error(
            func.sig.ident.span(),
            "#[client] components must be synchronous; load data in the page and pass it as props",
        );
    }
    if !func.sig.generics.params.is_empty() {
        return error(func.sig.generics.span_or_ident(&func.sig.ident), "#[client] components cannot be generic");
    }
    let mut names = Vec::new();
    for input in &func.sig.inputs {
        match input {
            FnArg::Typed(t) => match &*t.pat {
                Pat::Ident(p) => names.push(p.ident.clone()),
                other => {
                    return error(syn::spanned::Spanned::span(other), "#[client] arguments must be plain identifiers");
                }
            },
            FnArg::Receiver(r) => return error(syn::spanned::Spanned::span(r), "#[client] cannot be a method"),
        }
    }
    let vis = &func.vis;
    let attrs = &func.attrs;
    let name = &func.sig.ident;
    let name_str = name.to_string();
    let inputs = &func.sig.inputs;
    let output = &func.sig.output;
    let block = &func.block;
    let keys = names.iter().map(|n| n.to_string().trim_start_matches("r#").to_owned());
    let module = match &args.module {
        Some(m) => quote!(::core::option::Option::Some(#m)),
        None => quote!(::core::option::Option::None),
    };
    quote! {
        #(#attrs)*
        #[allow(non_snake_case)]
        #vis fn #name(#inputs) -> ::next_rust::Node {
            let __nr_props = ::next_rust::__private::island_props(&[
                #( (#keys, ::next_rust::__private::to_json_value(&#names)) ),*
            ]);
            #[allow(non_snake_case)]
            fn __nr_render(#inputs) #output #block
            ::next_rust::__private::island(#name_str, #module, __nr_props, __nr_render(#(#names),*))
        }
    }
    .into()
}

trait SpanOrIdent {
    fn span_or_ident(&self, ident: &syn::Ident) -> Span;
}

impl SpanOrIdent for syn::Generics {
    fn span_or_ident(&self, ident: &syn::Ident) -> Span {
        self.lt_token.map(|t| t.span).unwrap_or_else(|| ident.span())
    }
}

// ---------------------------------------------------------------------------
// #[server]
// ---------------------------------------------------------------------------

/// Server-only code. The item is compiled only for non-browser targets, so
/// referencing it from code compiled to `wasm32` fails at compile time
/// instead of leaking server logic or secrets into a client bundle.
///
/// Expansion: the item unchanged, prefixed with
/// `#[cfg(not(target_arch = "wasm32"))]`.
#[proc_macro_attribute]
pub fn server(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return error(Span::call_site(), "#[server] takes no arguments");
    }
    let item = proc_macro2::TokenStream::from(item);
    quote!(#[cfg(not(target_arch = "wasm32"))] #item).into()
}

// ---------------------------------------------------------------------------
// #[server_action]
// ---------------------------------------------------------------------------

/// Expose an async function as a server action.
///
/// Supported signatures:
///
/// ```ignore
/// #[server_action] pub async fn refresh() -> Result<Stats>
/// #[server_action] pub async fn create_user(input: NewUser) -> Result<User>
/// #[server_action] pub async fn logout(ctx: ActionContext, input: ()) -> Result<()>
/// ```
///
/// The input must implement `Deserialize` (JSON or form data), the output
/// `Serialize`. The build discovers actions in the app directory and in
/// `src/` and registers them at `POST /_next-rust/action/<hash>`. Reference an
/// action with `action!(create_user)`, e.g. `form![action!(create_user), ..]`.
///
/// `body_limit = <bytes>` sets the largest request body this one action
/// accepts, in place of `[server] body_limit` (for example an upload sent as
/// base64: `#[server_action(body_limit = 30 * 1024 * 1024)]`). It applies
/// only after the request has passed the action's origin and token checks,
/// so only a page that rendered the action can send the larger body.
///
/// Expansion: the original function, a `__NR_ACTION_ID_<name>` constant
/// (stable id derived from the file path) and a `__nr_action_<name>` HTTP
/// handler adapter.
#[proc_macro_attribute]
pub fn server_action(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut body_limit: Option<syn::Expr> = None;
    if !attr.is_empty() {
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("body_limit") {
                if body_limit.is_some() {
                    return Err(meta.error("`body_limit` is given twice"));
                }
                body_limit = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unknown #[server_action] argument; the only one is `body_limit = <bytes>`"))
            }
        });
        parse_macro_input!(attr with parser);
    }
    let func = parse_macro_input!(item as ItemFn);
    let name = &func.sig.ident;
    let argc = func.sig.inputs.len();
    if argc > 2 {
        return error(
            name.span(),
            "server actions take no arguments, one input, an ActionContext, or (ActionContext, input)",
        );
    }
    if matches!(func.sig.inputs.first(), Some(FnArg::Receiver(_))) {
        return error(name.span(), "server actions cannot be methods");
    }
    let id_const = format_ident!("__NR_ACTION_ID_{}", name);
    let handler = format_ident!("__nr_action_{}", name);
    let id = match caller_file_relative() {
        Some(file) => quote!(concat!(#file, "::", stringify!(#name))),
        None => quote!(concat!(module_path!(), "::", stringify!(#name))),
    };
    // A lone `ActionContext` is the request context, not the input.
    let context_only = argc == 1
        && matches!(func.sig.inputs.first(), Some(FnArg::Typed(arg))
            if matches!(&*arg.ty, syn::Type::Path(p)
                if p.path.segments.last().is_some_and(|s| s.ident == "ActionContext")));
    let call = match (argc, func.sig.asyncness.is_some()) {
        (1, true) if context_only => quote!(::next_rust::__private::run_action_ctx0(req, #name)),
        (1, false) if context_only => {
            quote!(::next_rust::__private::run_action_ctx0(req, |c| async move { #name(c) }))
        }
        (0, true) => quote!(::next_rust::__private::run_action0(req, #name)),
        (1, true) => quote!(::next_rust::__private::run_action(req, #name)),
        (2, true) => quote!(::next_rust::__private::run_action_ctx(req, #name)),
        (0, false) => quote!(::next_rust::__private::run_action0(req, || async move { #name() })),
        (1, false) => quote!(::next_rust::__private::run_action(req, |i| async move { #name(i) })),
        _ => quote!(::next_rust::__private::run_action_ctx(req, |c, i| async move { #name(c, i) })),
    };
    let limit = body_limit.map(|bytes| {
        quote! {
            const __NR_BODY_LIMIT: usize = #bytes;
            let mut req = req;
            req.set_body_limit(__NR_BODY_LIMIT);
        }
    });
    quote! {
        #[cfg(not(target_arch = "wasm32"))]
        #func

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub const #id_const: &str = #id;

        #[doc(hidden)]
        #[cfg(not(target_arch = "wasm32"))]
        pub fn #handler(req: ::next_rust::Request) -> ::next_rust::BoxFuture<::next_rust::Response> {
            #limit
            ::std::boxed::Box::pin(#call)
        }
    }
    .into()
}

/// Declare a background job: an `async fn` with one serialisable input.
///
/// ```ignore
/// #[job(queue = "mail", max_attempts = 5, timeout = "30s")]
/// pub async fn send_welcome(input: Welcome) -> Result<(), JobError> { /* ... */ }
///
/// enqueue(send_welcome, Welcome { /* ... */ }).await?;          // from anywhere
/// JobRunner::new().register(send_welcome)               // in main
/// ```
///
/// Accepted signatures: no argument, one input, a `JobContext`, or
/// `(input, JobContext)`; the return type is `()` or `Result<(), E>` where
/// `E` converts into `JobError` (any `std::error::Error` does).
///
/// Arguments, all optional: `name = "…"` (the stored name, default the
/// function name; keep it stable across deployments), `queue = "…"`,
/// `max_attempts = n`, `priority = n` (-100..=100), `timeout = "30s"`,
/// `backoff = "5s"` and `max_backoff = "1h"` (exponential, with jitter), and
/// `path = "next_rust_jobs"` for crates that depend on `next-rust-jobs`
/// directly instead of `next-rust`.
///
/// Expansion: a unit struct named like the function that implements
/// `next_rust::jobs::Job`, with the function itself as `name::perform(..)`.
#[proc_macro_attribute]
pub fn job(attr: TokenStream, item: TokenStream) -> TokenStream {
    jobs::job(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Declare a scheduled task: `#[cron("0 */5 * * * *")] pub async fn cleanup() { /* ... */ }`,
/// registered with `JobRunner::cron(cleanup)`.
///
/// The expression has five fields (`minute hour day month weekday`) or six
/// (seconds first), or is `@hourly`, `@daily`, `@weekly`, `@monthly`,
/// `@yearly` or `@every 5m`; times are UTC. `name = "…"` overrides the task
/// name (default: the function name), which is also the name of the lock
/// that keeps a tick from running on more than one instance.
///
/// Expansion: a unit struct named like the function that implements
/// `next_rust::jobs::CronTask`, with the function itself as `name::perform()`.
#[proc_macro_attribute]
pub fn cron(attr: TokenStream, item: TokenStream) -> TokenStream {
    jobs::cron(attr.into(), item.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Reference a server action: `action!(create_user)` or
/// `action!(crate::actions::create_user)`. Evaluates to an
/// `ActionRef`, usable as a `form![..]` part or via `.url()`.
#[proc_macro]
pub fn action(input: TokenStream) -> TokenStream {
    let mut path = parse_macro_input!(input as syn::Path);
    let Some(last) = path.segments.last_mut() else { return error(Span::call_site(), "expected a function path") };
    last.ident = format_ident!("__NR_ACTION_ID_{}", last.ident);
    quote!(::next_rust::ActionRef::new(#path)).into()
}

// ---------------------------------------------------------------------------
// CSS and assets
// ---------------------------------------------------------------------------

fn read_relative(lit: &LitStr, base: &Path) -> Result<(PathBuf, String), TokenStream> {
    let rel = lit.value();
    let direct = base.join(&rel);
    let path = if direct.is_file() {
        direct
    } else {
        // Editors such as rust-analyzer expand macros without telling them which
        // file they are in, so `base` may be the crate root. Look for the file
        // where it can live instead: the configured app directory, then the crate.
        match find_unique(&search_roots(), Path::new(&rel)) {
            Found::One(p) => p,
            Found::Many(list) => {
                let names: Vec<String> = list.iter().map(|p| p.display().to_string()).collect();
                return Err(error(
                    lit.span(),
                    format!(
                        "`{rel}` matches several files; use a path relative to this source file:\n  {}",
                        names.join("\n  ")
                    ),
                ));
            }
            Found::None => {
                return Err(error(
                    lit.span(),
                    format!("cannot find `{rel}` (looked next to this file and in the project)"),
                ));
            }
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok((path, text)),
        Err(e) => Err(error(lit.span(), format!("cannot read {}: {e}", path.display()))),
    }
}

enum Found {
    None,
    One(PathBuf),
    Many(Vec<PathBuf>),
}

/// The routing directory from `next-rust.toml` (if it can be read), then the crate root.
fn search_roots() -> Vec<PathBuf> {
    let manifest = manifest_dir();
    let mut roots = Vec::new();
    let app = std::fs::read_to_string(manifest.join("next-rust.toml"))
        .ok()
        .and_then(|toml| configured_app_dir(&toml))
        .unwrap_or_else(|| "app".to_owned());
    roots.push(manifest.join(app));
    roots.push(manifest);
    roots
}

/// `directory = "..."` from the `[app]` table, read without a TOML parser.
fn configured_app_dir(toml: &str) -> Option<String> {
    let mut in_app = false;
    for line in toml.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_app = line == "[app]";
        } else if in_app && let Some(value) = line.strip_prefix("directory") {
            let value = value.trim_start().strip_prefix('=')?.trim().trim_matches('"');
            return Some(value.to_owned());
        }
    }
    None
}

/// Files under `roots` whose path ends with `rel`, searched root by root; the
/// first root with any match decides.
fn find_unique(roots: &[PathBuf], rel: &Path) -> Found {
    for root in roots {
        let mut matches = Vec::new();
        collect_matches(root, rel, 0, &mut matches);
        matches.sort();
        matches.dedup();
        match matches.len() {
            0 => continue,
            1 => return Found::One(matches.remove(0)),
            _ => return Found::Many(matches),
        }
    }
    Found::None
}

fn collect_matches(dir: &Path, rel: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 12 || out.len() > 16 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if !name.starts_with('.') && name != "target" && name != "node_modules" {
                collect_matches(&path, rel, depth + 1, out);
            }
        } else if path.ends_with(rel) {
            out.push(path);
        }
    }
}

fn field_ident(class: &str) -> Option<syn::Ident> {
    let name: String = class.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    syn::parse_str::<syn::Ident>(&name).ok().or_else(|| Some(syn::Ident::new_raw(&name, Span::call_site())))
}

/// Import a CSS module (path relative to the current source file).
///
/// Class selectors are renamed to unique, deterministic names at compile
/// time and the CSS is minified. The result has one `CssClass` field per
/// class (`card-title` becomes `card_title`). Unknown fields are compile
/// errors. The stylesheet is emitted once per page that uses one of its
/// classes.
///
/// ```ignore
/// let styles = css_module!("card.module.css");
/// div![class(styles.card), h2![class(styles.card_title), "Hi"]]
/// ```
#[proc_macro]
pub fn css_module(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let (path, text) = match read_relative(&lit, &caller_dir()) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let salt = path.strip_prefix(manifest_dir()).unwrap_or(&path).to_string_lossy().replace('\\', "/");
    let usage = CssUsage::load();
    let text = match &usage {
        // A class is used through its field, `styles.card_title` for `.card-title`.
        Some(u) => next_rust_assets::css::prune(&next_rust_assets::css::minify(&text), &|name| {
            u.names.contains(name) || u.names.contains(&name.replace('-', "_"))
        }),
        None => text,
    };
    let track = usage.as_ref().map(CssUsage::track);
    let out = next_rust_assets::css::scope(&text, &salt, true);
    let id = next_rust_assets::content_hash(out.css.as_bytes())[..12].to_owned();
    let css = out.css;
    let abs = path.to_string_lossy().into_owned();
    let mut seen = std::collections::HashMap::new();
    let mut fields = Vec::new();
    let mut values = Vec::new();
    for (class, scoped) in &out.classes {
        let Some(ident) = field_ident(class) else {
            return error(lit.span(), format!("class `{class}` cannot be used as a Rust field name"));
        };
        if let Some(prev) = seen.insert(ident.to_string(), class.clone()) {
            return error(lit.span(), format!("classes `{prev}` and `{class}` map to the same field `{ident}`"));
        }
        fields.push(quote!(pub #ident: ::next_rust::CssClass));
        values.push(quote!(#ident: ::next_rust::CssClass::new(#scoped, &__NR_SHEET)));
    }
    quote! {{
        const _: &[u8] = include_bytes!(#abs);
        #track
        static __NR_SHEET: ::next_rust::Stylesheet = ::next_rust::Stylesheet { id: #id, css: #css, per_class: None, scripts: &[] };
        #[allow(non_camel_case_types, dead_code)]
        #[derive(Clone, Copy)]
        struct __NrCssModule { #(#fields,)* }
        __NrCssModule { #(#values,)* }
    }}
    .into()
}

/// Include a global stylesheet (path relative to the current source file),
/// minified at compile time. Place the result in the view tree, typically in
/// the root layout: `div![global_css!("globals.css"), children]`.
#[proc_macro]
pub fn global_css(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let (path, text) = match read_relative(&lit, &caller_dir()) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let css = next_rust_assets::css::minify(&text);
    let usage = CssUsage::load();
    let css = match &usage {
        Some(u) => next_rust_assets::css::prune(&css, &|name| u.names.contains(name)),
        None => css,
    };
    let track = usage.as_ref().map(CssUsage::track);
    let id = next_rust_assets::content_hash(css.as_bytes())[..12].to_owned();
    let abs = path.to_string_lossy().into_owned();
    // Sent per page: each document gets the rules for the classes it renders
    // (plus `[tailwind] keep_classes` / `[assets] css_safelist`, which scripts
    // add outside the view tree).
    let keep = CssUsage::keep_classes();
    quote! {{
        const _: &[u8] = include_bytes!(#abs);
        #track
        static __NR_KEEP: &[&str] = &[#(#keep),*];
        static __NR_SHEET: ::next_rust::Stylesheet = ::next_rust::Stylesheet { id: #id, css: #css, per_class: Some(__NR_KEEP), scripts: &[] };
        &__NR_SHEET
    }}
    .into()
}

/// Names used in the project, written by `next_rust_build::generate()` for
/// release builds. Without it (development builds, crates without a build
/// script) stylesheets are kept whole.
struct CssUsage {
    names: std::collections::HashSet<String>,
    file: String,
}

impl CssUsage {
    fn load() -> Option<Self> {
        let file = PathBuf::from(std::env::var_os("OUT_DIR")?).join("next_rust_css_usage.txt");
        let text = std::fs::read_to_string(&file).ok()?;
        Some(CssUsage { names: text.lines().map(str::to_owned).collect(), file: file.to_string_lossy().into_owned() })
    }

    /// `[tailwind] keep_classes` and `[assets] css_safelist`, written by the
    /// build script; empty without one.
    fn keep_classes() -> Vec<String> {
        std::env::var_os("OUT_DIR")
            .map(|out| PathBuf::from(out).join("next_rust_css_keep.txt"))
            .and_then(|file| std::fs::read_to_string(file).ok())
            .map(|text| text.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect())
            .unwrap_or_default()
    }

    /// Recompile when the list changes.
    fn track(&self) -> proc_macro2::TokenStream {
        let file = &self.file;
        quote!(
            const _: &[u8] = include_bytes!(#file);
        )
    }
}

// ---------------------------------------------------------------------------
// Scripts and stylesheets written in Rust
// ---------------------------------------------------------------------------

/// A script path as `client_script!` and `asset_script!` take it: relative,
/// `/`-separated, ending in `.js` or `.mjs`.
fn script_name(name: &str) -> Result<(), String> {
    let bad = name.is_empty()
        || name.starts_with('/')
        || name.contains('\\')
        || name.split('/').any(|s| s.is_empty() || s == ".." || s.starts_with('.'));
    if bad {
        return Err(format!("`{name}` is not a script path; write it like \"islands/chart.js\""));
    }
    if !(name.ends_with(".js") || name.ends_with(".mjs")) {
        return Err(format!("`{name}` must end in .js or .mjs"));
    }
    Ok(())
}

/// A wasm island name as `#[client(wasm = "…")]` takes it. It becomes both a
/// URL segment and the `hydrate_<name>` export looked up in the module, so it
/// is restricted to what is safe in both: an identifier.
fn wasm_island_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!("`{name}` is not a wasm island name; write the name of a `hydrate_…` export, like \"AuthSwitch\""))
    }
}

struct NamedSource {
    name: LitStr,
}

impl Parse for NamedSource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let _source: LitStr = input.parse()?;
        let _ = input.parse::<Option<syn::Token![,]>>()?;
        if !input.is_empty() {
            return Err(input.error("expected `(\"name.js\", r#\"source\"#)`"));
        }
        Ok(NamedSource { name })
    }
}

fn declared_script(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as NamedSource);
    if let Err(e) = script_name(&parsed.name.value()) {
        return error(parsed.name.span(), e);
    }
    // The build script reads the source out of this file; nothing of it is
    // compiled into the crate from here.
    quote!(
        const _: () = ();
    )
    .into()
}

/// Declare a browser module in Rust instead of a file in `client/`:
///
/// ```ignore
/// // src/client/chart.rs
/// next_rust::client_script!("chart.js", r#"
/// import { scale } from "./scale.js";
/// export function hydrate(element, props) { /* ... */ }
/// "#);
/// ```
///
/// The build treats it exactly like the file `client/chart.js`: served at
/// `/_next-rust/client/chart.js`, scanned for classes, rewritten with the
/// short class names and minified in release builds, and importable from
/// other modules by its relative path. Use it from an island with
/// `#[client(script = "chart.js")]`. Write the macro at module level, in a
/// file under `src/` or the app directory; a file in `client/` with the same
/// name wins.
#[proc_macro]
pub fn client_script(input: TokenStream) -> TokenStream {
    declared_script(input)
}

/// Declare a script of `assets/` in Rust: `asset_script!("js/site.js",
/// r#"…"#)` stands for the file `assets/js/site.js`, and
/// `asset!("js/site.js")` gives its fingerprinted URL. See `client_script!`.
#[proc_macro]
pub fn asset_script(input: TokenStream) -> TokenStream {
    declared_script(input)
}

/// Declare a stylesheet in Rust: list the file that holds it in `[tailwind]
/// stylesheets` (`stylesheets = ["src/styles/theme.rs"]`) and the CSS inside
/// `stylesheet!(r#"…"#)` is compiled like a `.css` file listed there.
#[proc_macro]
pub fn stylesheet(input: TokenStream) -> TokenStream {
    let _css = parse_macro_input!(input as LitStr);
    quote!(
        const _: () = ();
    )
    .into()
}

/// Fingerprinted URL of a file in the project's `assets/` directory,
/// computed at compile time: `asset!("fonts/inter.woff2")` →
/// `"/_next-rust/assets/fonts/inter.1a2b3c4d5e6f7a8b.woff2"`. Served with
/// immutable caching; the URL changes whenever the file content changes.
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let rel = lit.value();
    let rel = rel.trim_start_matches('/');
    if rel.split('/').any(|s| s == ".." || s.starts_with('.')) {
        return error(lit.span(), "asset paths must stay inside assets/ and not reference hidden files");
    }
    let mut path = manifest_dir().join("assets").join(rel);
    if !path.is_file() {
        // A script declared with `asset_script!`: the build script wrote it out.
        let declared = std::env::var_os("OUT_DIR")
            .map(|out| PathBuf::from(out).join("next_rust_src").join("assets").join(rel))
            .filter(|p| p.is_file());
        if let Some(declared) = declared {
            path = declared;
        }
    }
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => return error(lit.span(), format!("cannot read {}: {e}", path.display())),
    };
    let (dir, file) = match rel.rsplit_once('/') {
        Some((d, f)) => (format!("{d}/"), f.to_owned()),
        None => (String::new(), rel.to_owned()),
    };
    // Release builds embed scripts minified (`next-rust-build` writes the
    // copies under `$OUT_DIR/next_rust_min`); the URL names what is served.
    let served = std::env::var_os("OUT_DIR")
        .map(|out| PathBuf::from(out).join("next_rust_min").join("assets").join(rel))
        .and_then(|min| std::fs::read(min).ok())
        .unwrap_or(bytes);
    let url = format!("/_next-rust/assets/{dir}{}", next_rust_assets::fingerprint_name(&file, &served));
    let abs = path.to_string_lossy().into_owned();
    quote! {{
        const _: &[u8] = include_bytes!(#abs);
        #url
    }}
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nr-macros-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn touch(path: PathBuf) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "body{}").unwrap();
    }

    #[test]
    fn finds_a_stylesheet_inside_the_app_directory() {
        let root = temp("unique");
        touch(root.join("app/globals.css"));
        touch(root.join("target/debug/globals.css"));
        let roots = vec![root.join("app"), root.clone()];
        assert!(
            matches!(find_unique(&roots, Path::new("globals.css")), Found::One(p) if p == root.join("app/globals.css"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn app_directory_wins_and_ambiguity_is_reported() {
        let root = temp("many");
        touch(root.join("app/a/card.css"));
        touch(root.join("app/b/card.css"));
        touch(root.join("styles/card.css"));
        let roots = vec![root.join("app"), root.clone()];
        assert!(matches!(find_unique(&roots, Path::new("card.css")), Found::Many(list) if list.len() == 2));
        assert!(matches!(find_unique(&roots, Path::new("a/card.css")), Found::One(_)));
        assert!(matches!(find_unique(&roots, Path::new("missing.css")), Found::None));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_the_configured_app_directory() {
        assert_eq!(
            configured_app_dir("[server]\nport = 1\n[app]\ndirectory = \"src/web\" # routes\n").as_deref(),
            Some("src/web")
        );
        assert_eq!(configured_app_dir("[build]\ndirectory = \"x\"\n"), None);
    }
}
