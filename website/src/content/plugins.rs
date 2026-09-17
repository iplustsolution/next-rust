//! The "plugins" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Plugins are ordinary Rust values implementing one of two traits. There are no dynamic libraries and no global registration: you add plugins where the application is assembled.",
        ],
        h2![id("runtime-plugins"), a![class("anchor"), href("#runtime-plugins"), "Runtime plugins"]],
        pre![code![
            class("language-rust"),
            r##"use next_rust::plugin::Plugin;
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
}"##,
        ],],
        pre![code![
            class("language-rust"),
            r#"next_rust::routes!();
fn main() {
    next_rust::run_with(next_rust::App::new(routes()).plugin(Analytics { site_id: "abc".into() }));
}"#,
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["hook"], th!["when"]]],
                tbody![
                    tr![td![code!["on_start(&Config)"]], td!["once, when the app is built"]],
                    tr![
                        td![code!["middleware()"]],
                        td!["added after the app's global middleware and before ", code!["app/middleware.rs"]],
                    ],
                    tr![td![code!["head()"]], td!["trusted markup appended to every document ", code!["<head>"]],],
                ],
            ],
        ],
        p![code!["head()"], " markup isn't escaped. Plugins are trusted code."],
        h2![id("build-plugins"), a![class("anchor"), href("#build-plugins"), "Build plugins"]],
        p![
            "Build plugins run inside ",
            code!["build.rs"],
            ", during route generation (add ",
            code!["next-rust-core"],
            " to ",
            code!["[build-dependencies]"],
            " for ",
            code!["Diagnostic"],
            "):",
        ],
        pre![code![
            class("language-rust"),
            r#"// build.rs
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
}"#,
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["hook"], th!["can"]]],
                tbody![
                    tr![
                        td![code!["on_project(&mut Project)"]],
                        td![
                            "read the scanned tree, analyzed files and routes; add diagnostics (errors fail the build); change rendering modes",
                        ],
                    ],
                    tr![
                        td![code!["extra_code(&Project)"]],
                        td!["append Rust code to the generated module (e.g. typed route helpers)"],
                    ],
                ],
            ],
        ],
        p![
            "Asset processing and deployment steps can run as build plugins, or as separate steps around ",
            code!["next-rust build"],
            ", using the route manifest (",
            code!["target/next-rust/routes.json"],
            ", or ",
            code!["next-rust routes --json"],
            ") as a stable interface.",
        ],
        p![
            strong!["Stability:"],
            " both traits are part of the 0.x public API and may change in minor releases until 1.0.",
        ],
    ]
}
