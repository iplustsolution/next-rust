//! Project and file templates.

pub fn cargo_toml(name: &str, framework_path: Option<&str>) -> String {
    let (dep, build_dep) = match framework_path {
        Some(p) => (
            format!("{{ path = \"{}/crates/next-rust\" }}", p.trim_end_matches('/')),
            format!("{{ path = \"{}/crates/next-rust-build\" }}", p.trim_end_matches('/')),
        ),
        // Until the crates are published, projects track the GitHub repository.
        // `next-rust upgrade` moves them to the latest commit.
        None => (format!("{{ git = \"{}\" }}", crate::REPO_URL), format!("{{ git = \"{}\" }}", crate::REPO_URL)),
    };
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
next-rust = {dep}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"

[build-dependencies]
next-rust-build = {build_dep}

[profile.release]
lto = "thin"
codegen-units = 4
"#
    )
}

pub const BUILD_RS: &str = "fn main() {\n    next_rust_build::generate();\n}\n";

pub const MAIN_RS: &str = "next_rust::app!();\n";

pub const CONFIG: &str = r#"# Next Rust configuration. Every setting is optional.

[app]
directory = "app"
# public = "public"

[server]
port = 3000

# [rendering]
# default = "auto"     # auto | static | dynamic
# streaming = true
"#;

pub const GITIGNORE: &str = "/target\n/.next-rust\n.env*.local\n";

pub const ENV_EXAMPLE: &str = "# Server-only secrets (never sent to the browser)\nDATABASE_URL=postgres://localhost/app\n\n# Values safe for client code must use the public prefix\nNEXT_RUST_PUBLIC_SITE_NAME=My App\n";

pub fn layout_rs(name: &str) -> String {
    format!(
        r#"use next_rust::prelude::*;

pub fn metadata() -> Metadata {{
    Metadata::new()
        .title("{name}")
        .title_template("%s · {name}")
        .description("Built with Next Rust")
}}

pub fn Layout(children: Children) -> impl View {{
    div![
        global_css!("globals.css"),
        header![nav![Link!(href = "/", "Home"), Link!(href = "/about", "About")]],
        main![children],
    ]
}}
"#
    )
}

pub const PAGE_RS: &str = r#"use next_rust::prelude::*;

pub fn Page() -> impl View {
    div![
        h1!["Hello World"],
        p!["Welcome to Next Rust. Edit app/page.rs and save to reload."],
    ]
}
"#;

pub const ABOUT_RS: &str = r#"use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("About")
}

pub fn Page() -> impl View {
    article![h1!["About"], p!["This page lives in app/about/page.rs."]]
}
"#;

pub const API_RS: &str = r#"use next_rust::prelude::*;

pub async fn GET(req: Request) -> Response {
    let name = req.query_param("name").unwrap_or_else(|| "world".into());
    Response::json(&serde_json::json!({ "hello": name }))
}
"#;

pub const NOT_FOUND_RS: &str = r#"use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    div![h1!["404"], p!["This page could not be found."], Link!(href = "/", "Go home")]
}
"#;

pub const GLOBALS_CSS: &str = r#"body {
  font-family: system-ui, -apple-system, sans-serif;
  max-width: 48rem;
  margin: 0 auto;
  padding: 2rem 1rem;
  line-height: 1.6;
}

nav a {
  margin-right: 1rem;
}
"#;

pub fn readme(name: &str) -> String {
    format!(
        "# {name}\n\nA [Next Rust](https://github.com/iplustsolution/next-rust) application.\n\n```sh\nnext-rust dev     # http://localhost:3000\nnext-rust build   # production build + static generation\nnext-rust start   # run the production build\n```\n\nRoutes live in `app/`: `page.rs` files are pages, `layout.rs` files wrap their children, `route.rs` files are API endpoints.\n"
    )
}

/// Template for `next-rust generate <kind>`.
pub fn generate(kind: &str, route: &str) -> Option<(&'static str, String)> {
    let title = route.trim_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or("Home").to_owned();
    let params: Vec<String> = route
        .split('/')
        .filter_map(|s| s.strip_prefix('[').and_then(|s| s.strip_suffix(']')))
        .map(|s| s.trim_start_matches('[').trim_end_matches(']').trim_start_matches("...").to_owned())
        .collect();
    Some(match kind {
        "page" if !params.is_empty() => (
            "page.rs",
            format!(
                "use next_rust::prelude::*;\n\npub fn Page(params: Params) -> impl View {{\n    div![\n{}    ]\n}}\n",
                params.iter().map(|p| format!("        p![format!(\"{p}: {{:?}}\", params.get_all({p:?}))],\n")).collect::<String>()
            ),
        ),
        "page" => (
            "page.rs",
            format!(
                "use next_rust::prelude::*;\n\npub fn metadata() -> Metadata {{\n    Metadata::new().title({title:?})\n}}\n\npub fn Page() -> impl View {{\n    h1![{title:?}]\n}}\n"
            ),
        ),
        "layout" => ("layout.rs", "use next_rust::prelude::*;\n\npub fn Layout(children: Children) -> impl View {\n    section![children]\n}\n".into()),
        "template" => ("template.rs", "use next_rust::prelude::*;\n\npub fn Template(children: Children) -> impl View {\n    div![children]\n}\n".into()),
        "loading" => ("loading.rs", "use next_rust::prelude::*;\n\npub fn Loading() -> impl View {\n    p![aria(\"busy\", \"true\"), \"Loading…\"]\n}\n".into()),
        "error" => (
            "error.rs",
            "use next_rust::prelude::*;\n\npub fn ErrorBoundary(info: ErrorInfo) -> impl View {\n    div![role(\"alert\"), h2![\"Something went wrong\"], p![info.message], small![format!(\"Reference: {}\", info.digest)]]\n}\n".into(),
        ),
        "not-found" => ("not-found.rs", "use next_rust::prelude::*;\n\npub fn NotFound() -> impl View {\n    h1![\"Not found\"]\n}\n".into()),
        "api" | "route" => (
            "route.rs",
            "use next_rust::prelude::*;\n\npub async fn GET(req: Request) -> Response {\n    Response::json(&serde_json::json!({ \"path\": req.path() }))\n}\n\npub async fn POST(mut req: Request) -> Result<Response> {\n    let body: serde_json::Value = req.json().await?;\n    Ok(Response::json(&body).with_status(201))\n}\n".into(),
        ),
        "middleware" => (
            "middleware.rs",
            "use next_rust::prelude::*;\n\npub async fn middleware(req: Request, next: Next) -> Response {\n    next.run(req).await\n}\n".into(),
        ),
        _ => return None,
    })
}

pub fn dockerfile(bin: &str) -> String {
    format!(
        r#"# syntax=docker/dockerfile:1
# Generated by `next-rust docker`.

FROM rust:1-slim AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --bin {bin} \
 && cp target/release/{bin} /usr/local/bin/app \
 && mkdir -p public && (test -f next-rust.toml || touch next-rust.toml) \
 && NEXT_RUST_ENV=production /usr/local/bin/app --export

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 app
WORKDIR /app
COPY --from=build /usr/local/bin/app /usr/local/bin/app
COPY --from=build /src/next-rust.toml ./
COPY --from=build /src/public ./public
COPY --from=build /src/.next-rust ./.next-rust
RUN mkdir -p /app/app && chown -R app /app/.next-rust
USER app
ENV NEXT_RUST_ENV=production HOST=0.0.0.0 PORT=3000
EXPOSE 3000
CMD ["/usr/local/bin/app"]
"#
    )
}

pub const DOCKERIGNORE: &str = "target\n.next-rust\n.git\n.env*.local\n";
