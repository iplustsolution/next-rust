//! The "plugins" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Plugins are ordinary Rust values implementing one of two traits. There are no dynamic libraries and no global registration: you add plugins where the application is assembled.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "A plugin is an add-on: a packaged piece of behaviour you can switch on in one line and share between projects, like an extension for a browser. There are two kinds, named after the moment they act. A ",
                strong!["runtime plugin"],
                " works while the site is running and visitors arrive (add a header to every response, a tag to every page). A ",
                strong!["build plugin"],
                " works earlier, while the project is being compiled (enforce a house rule on every page, generate helper code).",
            ],
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th![], th!["runtime plugin"], th!["build plugin"]]],
                tbody![
                    tr![
                        td!["acts"],
                        td!["on every request, inside the running app"],
                        td!["once per build, inside the build script"]
                    ],
                    tr![
                        td!["lives in"],
                        td![code!["src/"], ", added in ", code!["src/main.rs"]],
                        td![code!["build.rs"], " in the project root"],
                    ],
                    tr![td!["ends up in the binary"], td!["yes"], td!["no, only the code it generates"]],
                    tr![
                        td!["typical use"],
                        td!["headers, request logging, a ", code!["<head>"], " tag"],
                        td!["project rules as diagnostics, typed route helpers"],
                    ],
                ],
            ],
        ],
        p![
            "You rarely need one. Analytics and verification tags are ",
            a![href("/docs/analytics"), "configuration"],
            ", request logic for one app is ",
            a![href("/docs/middleware"), "middleware"],
            ". Write a plugin when the same behaviour should be reusable across several apps.",
        ],
        h2![id("runtime-plugins"), a![class("anchor"), href("#runtime-plugins"), "Runtime plugins"]],
        p![
            "A runtime plugin is a struct that implements the ",
            code!["Plugin"],
            " trait (a trait is Rust's word for a set of functions a type promises to offer). Every function except ",
            code!["name"],
            " in this example is an optional hook. This file can live anywhere in your crate, for example ",
            code!["src/analytics_plugin.rs"],
            ". The example is an illustration of the hooks; for real analytics use the ",
            code!["[analytics]"],
            " settings:",
        ],
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
        p![
            "Then add it where the application is assembled. This replaces the one-line ",
            code!["next_rust::app!();"],
            " in ",
            code!["src/main.rs"],
            ":",
        ],
        pre![code![
            class("language-rust"),
            r#"next_rust::routes!();
fn main() {
    next_rust::run_with(next_rust::App::new(routes()).plugin(Analytics { site_id: "abc".into() }));
}"#,
        ],],
        p![
            "The plugin reads its own settings from ",
            code!["[plugins.<name>]"],
            " in ",
            code!["next-rust.toml"],
            ", the one section where keys of your own are allowed (everywhere else an unknown key is a build error). The values arrive as JSON:",
        ],
        pre![code![
            class("language-toml"),
            r#"# next-rust.toml
[plugins.analytics]
site_id = "abc""#,
        ]],
        p!["The hooks:"],
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
        p![
            code!["head()"],
            " markup isn't escaped. Plugins are trusted code: never build that string from visitor input, and read a third-party plugin before adding it, as you would any dependency.",
        ],
        h2![id("build-plugins"), a![class("anchor"), href("#build-plugins"), "Build plugins"]],
        p![
            code!["build.rs"],
            " is the build script: a small Rust program in the project root that Cargo runs before compiling the app. In a Next Rust project it reads the ",
            code!["app/"],
            " folder and generates the route table; normally it is the single line ",
            code!["next_rust_build::generate();"],
            ". A build plugin hooks into that step. Its code never reaches the production binary.",
        ],
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
        p![
            "This example replaces the whole ",
            code!["build.rs"],
            ". It warns about every page that has no ",
            code!["metadata"],
            " function (so no title for search engines) and adds a constant to the generated code:",
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
        p![
            "The warning appears in the terminal like the framework's own ",
            a![href("/docs/diagnostics"), "diagnostics"],
            ", with your code (",
            code!["X001"],
            "), the file and the help line. Use ",
            code!["Diagnostic::error"],
            " instead and the build fails until the page is fixed. The hooks:",
        ],
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
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["Plugin settings outside [plugins.<name>]. "],
                "A key of your own anywhere else in ",
                code!["next-rust.toml"],
                " is an unknown key and stops the build.",
            ],
            li![
                strong!["A runtime plugin that is never added. "],
                "There is no automatic discovery. Without ",
                code![".plugin(..)"],
                " on the ",
                code!["App"],
                " nothing happens; with ",
                code!["next_rust::app!();"],
                " in ",
                code!["src/main.rs"],
                " there is no place to add it, so switch to ",
                code!["routes!()"],
                " and ",
                code!["run_with"],
                ".",
            ],
            li![
                strong!["A build plugin without its build dependencies. "],
                code!["next-rust-build"],
                " and, for ",
                code!["Diagnostic"],
                ", ",
                code!["next-rust-core"],
                " go under ",
                code!["[build-dependencies]"],
                " in ",
                code!["Cargo.toml"],
                ", not under ",
                code!["[dependencies]"],
                ".",
            ],
            li![strong!["Visitor input in head(). "], "It is written into every page unescaped.",],
            li![
                strong!["A vendor's analytics snippet as a plugin. "],
                "Use ",
                code!["[analytics]"],
                ": it handles the CSP nonce, consent and client-side navigations.",
            ],
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        ul![
            li![
                "Runtime: ",
                code!["curl -I http://localhost:3000/"],
                " shows the header your middleware adds; \"View page source\" shows the ",
                code!["head()"],
                " markup.",
            ],
            li![
                "Build: ",
                code!["cargo build"],
                " or ",
                code!["next-rust check"],
                " prints your diagnostics. The generated code is in ",
                code!["$OUT_DIR/next_rust_routes.rs"],
                " (under ",
                code!["target/"],
                ").",
            ],
        ],
        p![
            strong!["Stability:"],
            " both traits are part of the 0.x public API and may change in minor releases until 1.0.",
        ],
    ]
}
