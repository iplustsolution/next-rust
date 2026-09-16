# Plugins

Plugins are ordinary Rust values implementing one of two traits. There are
no dynamic libraries and no global registration: you add plugins where the
application is assembled.

## Runtime plugins

```rust
use next_rust::plugin::Plugin;
use next_rust::{Config, Middleware, Request, Next, Response};
use std::sync::Arc;

pub struct Analytics { site_id: String }

impl Plugin for Analytics {
    fn name(&self) -> &str { "analytics" }

    fn on_start(&self, config: &Config) {
        // read [plugins.analytics] from next-rust.toml
        let _ = config.plugins.get("analytics");
    }

    fn middleware(&self) -> Vec<Arc<dyn Middleware>> {
        let mw = |req: Request, next: Next| async move { next.run(req).await.with_header("x-analytics", "1") };
        vec![Arc::new(mw) as Arc<dyn Middleware>]
    }

    fn head(&self) -> Option<String> {
        Some(format!(r#"<meta name="analytics-site" content="{}">"#, self.site_id))
    }
}
```

```rust
next_rust::routes!();
fn main() {
    next_rust::run_with(next_rust::App::new(routes()).plugin(Analytics { site_id: "abc".into() }));
}
```

| hook | when |
|---|---|
| `on_start(&Config)` | once, when the app is built |
| `middleware()` | added after the app's global middleware and before `app/middleware.rs` |
| `head()` | trusted markup appended to every document `<head>` |

`head()` markup isn't escaped. Plugins are trusted code.

## Build plugins

Build plugins run inside `build.rs`, during route generation (add
`next-rust-core` to `[build-dependencies]` for `Diagnostic`):

```rust
// build.rs
use next_rust_build::{BuildPlugin, Generator, Project};
use next_rust_core::Diagnostic;

struct RequireMetadata;

impl BuildPlugin for RequireMetadata {
    fn name(&self) -> &str { "require-metadata" }

    fn on_project(&self, project: &mut Project) {
        let missing: Vec<_> = project.routes.iter()
            .filter(|r| r.route.is_page())
            .filter(|r| project.files.get(&r.route.source).is_some_and(|f| f.get("metadata").is_none()))
            .map(|r| r.route.source.clone())
            .collect();
        for source in missing {
            project.diagnostics.push(
                Diagnostic::warning("X001", "Page without metadata").location(source)
                    .help("export `pub fn metadata() -> Metadata`"),
            );
        }
    }

    fn extra_code(&self, project: &Project) -> Option<String> {
        Some(format!("pub const ROUTE_COUNT: usize = {};", project.routes.len()))
    }
}

fn main() {
    Generator::new().plugin(RequireMetadata).run();
}
```

| hook | can |
|---|---|
| `on_project(&mut Project)` | read the scanned tree, analyzed files and routes; add diagnostics (errors fail the build); change rendering modes |
| `extra_code(&Project)` | append Rust code to the generated module (e.g. typed route helpers) |

Asset processing and deployment steps can run as build plugins, or as
separate steps around `next-rust build`, using the route manifest
(`.next-rust/manifest/routes.json`) as a stable interface.

**Stability:** both traits are part of the 0.x public API and may change in
minor releases until 1.0.
