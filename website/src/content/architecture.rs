//! The "architecture" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        div![
            class("callout"),
            p![
                strong!["In plain words."],
                " Think of a restaurant. Before opening, an inspector walks through the building and writes down which table is where; that is the build step reading your ",
                code!["app/"],
                " folder. During service, a guest's order goes to the door, past the host, to the right table, and the kitchen sends out the dish in courses; that is a request passing through middleware, the router and the renderer, which streams the page. This page describes both halves, and why the framework is cut into the pieces it is.",
            ],
        ],
        p![
            "Terms used on this page: a ",
            strong!["crate"],
            " is a Rust package, the unit that is compiled and published; ",
            strong!["codegen"],
            " (code generation) is a program writing Rust source for the compiler to check; a ",
            strong!["trie"],
            " is a lookup tree that is walked one URL segment at a time; ",
            strong!["SSR"],
            " is server-side rendering, producing a page's HTML on the server; ",
            strong!["ISR"],
            " is incremental static regeneration, refreshing a cached page in the background; a ",
            strong!["binary"],
            " is the finished program as one file.",
        ],
        h2![id("crate-graph"), a![class("anchor"), href("#crate-graph"), "Crate graph"]],
        p![
            "An application depends on one crate, ",
            code!["next-rust"],
            ", which re-exports the others. Each box shows a crate, what it contains, and which sibling crates it uses. ",
            strong!["Why it matters:"],
            " tools that only run while building never end up in the program you deploy, which keeps the binary small.",
        ],
        div![
            class("crate-grid"),
            div![
                class("crate"),
                code!["next-rust"],
                span!["prelude · app!"],
                small!["uses server, view, router, cache, core, macros, jobs, ui, icons"],
            ],
            div![
                class("crate"),
                code!["next-rust-server"],
                span!["pipeline · SSR · ISR · actions · hyper"],
                small!["uses core, router, view, assets, cache"],
            ],
            div![
                class("crate"),
                code!["next-rust-view"],
                span!["DSL · metadata · streaming"],
                small!["no internal dependencies"],
            ],
            div![
                class("crate"),
                code!["next-rust-router"],
                span!["scan · validate · rank · match"],
                small!["uses core"],
            ],
            div![
                class("crate"),
                code!["next-rust-ui"],
                span!["components · styles · script"],
                small!["uses view, icons"],
            ],
            div![class("crate"), code!["next-rust-icons"], span!["Lucide icons"], small!["uses view"]],
            div![class("crate"), code!["next-rust-cache"], span!["stores · tags"], small!["uses assets"]],
            div![
                class("crate"),
                code!["next-rust-core"],
                span!["config · env · diagnostics"],
                small!["no internal dependencies"],
            ],
            div![
                class("crate"),
                code!["next-rust-assets"],
                span!["hash · mime · css"],
                small!["no internal dependencies"],
            ],
            div![
                class("crate"),
                code!["next-rust-macros"],
                span!["#[client] · #[server_action] · css_module!"],
                small!["uses assets"],
            ],
            div![
                class("crate"),
                code!["next-rust-jobs"],
                span!["queues · retries · cron"],
                small!["no internal dependencies"],
            ],
            div![
                class("crate"),
                code!["next-rust-auth"],
                span!["sessions · passwords · limiters (optional)"],
                small!["uses server"],
            ],
            div![class("crate"), code!["next-rust-build"], span!["analysis · codegen"], small!["uses core, router"],],
            div![
                class("crate"),
                code!["next-rust-cli"],
                span!["the next-rust command"],
                small!["uses build, router, core"],
            ],
        ],
        p!["Crates are split by dependency weight and by where they run:"],
        ul![
            li![
                code!["next-rust-build"],
                " (syn) and ",
                code!["next-rust-cli"],
                " never end up in the application binary.",
            ],
            li![
                code!["next-rust-view"],
                " depends only on ",
                code!["futures-util"],
                ", so it can render HTML outside a server (emails, tests) and could compile to ",
                code!["wasm32"],
                ".",
            ],
            li![
                code!["next-rust-router"],
                " has no async or I/O dependencies beyond ",
                code!["std::fs"],
                " scanning, and is shared by the build, the CLI and the runtime.",
            ],
            li![
                code!["next-rust-assets"],
                " has ",
                strong!["zero"],
                " dependencies and is shared by the procedural macros (compile time) and the server (runtime).",
            ],
        ],
        h2![id("build-pipeline"), a![class("anchor"), href("#build-pipeline"), "Build pipeline"]],
        p![
            "This diagram follows your source files from the ",
            code!["app/"],
            " directory to the deployable binary. Each arrow names the function or tool that does the step, and the branches to the right are where build errors (",
            code!["NR"],
            " codes) come from. It describes what ",
            code!["build.rs"],
            " and the compiler do; there is nothing here to write yourself:",
        ],
        pre![code![
            class("language-text"),
            r#"app/ directory
    │  next-rust-router::scan_project
    ▼
RouteNode tree ── flatten ──▶ Vec<Route> ── validate ──▶ diagnostics (NR01xx)
    │  next-rust-build::analyze (syn: signatures, consts, #[server_action])
    ▼
Project { routes + rendering modes + exports } ── check_exports ──▶ diagnostics (NR02xx)
    │  generate_code
    ▼
$OUT_DIR/next_rust_routes.rs     #[path = "/abs/app/page.rs"] mod m0; …
    │                            fn __render_m0_Page(ctx) -> BoxFuture<Result<Node>> { … }
    │                            pub fn routes() -> Routes { … }
    ▼
cargo/rustc  ── type checks every page against its extractors ──▶ server binary
    │  release: next-rust.toml, public/, assets/, client/ embedded; stripped
    ▼
.next-rust/<name>   one self-contained file; static pages render into memory at startup"#,
        ],],
        p![
            strong!["Why it matters:"],
            " everything that can be wrong with a route is found in this pipeline, on your machine, and not by a visitor.",
        ],
        p!["Design decisions:"],
        ol![
            li![
                strong!["Special files are real modules"],
                ", included with absolute ",
                code!["#[path]"],
                " attributes. This supports an app directory anywhere on disk, directory names that aren't Rust identifiers (",
                code!["[slug]"],
                ", ",
                code!["(group)"],
                "), and Unicode and spaces, with no file copying. Editor tooling keeps working because the files are ordinary Rust.",
            ],
            li![
                strong!["Signature analysis instead of type-level tricks."],
                " ",
                code!["syn"],
                " reads each exported function's arguments. The generator emits a direct call with one ",
                code!["FromContext::from_context(&ctx)?"],
                " per argument, so the compiler checks every extractor. There are no ",
                code!["Handler"],
                " trait towers with 16-arity impls, and error messages point at a single call.",
            ],
            li![
                strong!["Rendering mode inference"],
                " is syntactic: argument types are classified as request-bound or not. Runtime ",
                code!["DynamicUsage"],
                " errors back it up if inference is bypassed.",
            ],
            li![
                strong!["Deterministic output."],
                " Directory entries are sorted, maps are ",
                code!["BTreeMap"],
                "s, and the generated file is rewritten only when its content changes, which avoids needless recompiles.",
            ],
        ],
        h2![id("request-lifecycle"), a![class("anchor"), href("#request-lifecycle"), "Request lifecycle"]],
        p![
            "This diagram shows what happens, in order, from the moment a browser connects until the response leaves. Indentation means \"inside\": middleware wraps routing, routing wraps the page. It is a map of ",
            code!["crates/next-rust-server"],
            ", useful when you need to know whether, say, a redirect rule runs before or after your middleware:",
        ],
        pre![code![
            class("language-text"),
            r"hyper connection (HTTP/1.1 keep-alive or HTTP/2)
  └─ App::handle_hyper: request timeout, request log, gzip
      └─ App::handle
          ├─ CSP nonce, body limit
          ├─ pre-routing: base path, /_next-rust/* framework assets, [[redirects]], trailing slash
          ├─ global middleware stack (App::middleware + plugins + app/middleware.rs)
          │   └─ RouteEndpoint::dispatch
          │       ├─ /_next-rust/action/* → origin check → signed token → action handler
          │       ├─ sitemap.xml / robots.txt generators
          │       ├─ intercepting routes (soft navigation headers)
          │       ├─ trie match → page | API | programmatic route
          │       │    └─ nested middleware.rs stack → endpoint
          │       │         page: static? → page store (HIT/STALE/MISS) : render
          │       ├─ public/ file
          │       └─ 404 page
          └─ finish: Set-Cookie from the shared jar, [[headers]], security headers",
        ],],
        p!["Page rendering (", code!["render.rs"], ") then builds the page from the outside in, in three steps:",],
        ol![
            li!["Metadata functions for every segment run concurrently, then merge root to leaf."],
            li![
                code!["render_level"],
                " recurses from the root segment to the page. At each level it wraps the level below in a suspense boundary (if ",
                code!["loading.rs"],
                " and streaming), applies ",
                code!["not-found.rs"],
                " and ",
                code!["error.rs"],
                " boundaries, wraps in ",
                code!["template.rs"],
                ", renders slots, and calls ",
                code!["layout.rs"],
                ".",
            ],
            li![
                "The resulting ",
                code!["Node"],
                " tree goes to ",
                code!["stream_document"],
                ". It serializes the shell, hoists used stylesheets into ",
                code!["<head>"],
                ", and streams resolved boundaries as they complete (",
                code!["FuturesUnordered"],
                ").",
            ],
        ],
        h2![
            id("runtime-choice-tokio-hyper"),
            a![class("anchor"), href("#runtime-choice-tokio-hyper"), "Runtime choice: Tokio + hyper"],
        ],
        ul![
            li![
                strong!["Tokio"],
                " is the most widely deployed Rust async runtime, with mature I/O, timers, signal handling and ecosystem support (database drivers, HTTP clients). Most crates users will call from pages already require it. The framework's public API exposes plain ",
                code!["Future"],
                "s, ",
                code!["Request"],
                "/",
                code!["Response"],
                " and ",
                code!["BoxFuture"],
                ". Tokio is used inside ",
                code!["next-rust-server"],
                " (listener, timers for SSE and timeouts, background ISR regeneration, jobs) and by the file cache store. The view layer, router and build tooling don't depend on it, and any executor that can drive Tokio-compatible futures can call ",
                code!["App::handle"],
                ".",
            ],
            li![
                strong!["hyper 1.x"],
                " provides production-grade HTTP/1.1 and HTTP/2 without a framework on top. Next Rust owns routing, middleware and rendering, so axum or actix would add a second, competing abstraction layer.",
            ],
        ],
        h2![id("dependencies"), a![class("anchor"), href("#dependencies"), "Dependencies"]],
        p![
            "Every external dependency, with its reason. Mature protocol implementations are reused, not reimplemented. Small utilities (dates, ",
            code![".env"],
            ", CSS, cookies, hashing, MIME types, argument parsing) are implemented internally, because their total size was a few hundred lines each.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["crate"], th!["used by"], th!["why"], th!["alternatives considered"]]],
                tbody![
                    tr![
                        td![code!["tokio"]],
                        td!["server, cache"],
                        td!["async runtime, networking, timers, signals, fs"],
                        td!["async-std (less maintained), smol (smaller ecosystem)"],
                    ],
                    tr![
                        td![code!["hyper"], ", ", code!["hyper-util"]],
                        td!["server"],
                        td!["HTTP/1.1 server (HTTP/2 with the http2 feature), upgrades"],
                        td!["writing HTTP parsing (security risk)"],
                    ],
                    tr![
                        td![code!["http"], ", ", code!["http-body"], ", ", code!["http-body-util"]],
                        td!["server"],
                        td!["standard request/response types used across the ecosystem"],
                        td!["—"],
                    ],
                    tr![
                        td![code!["bytes"]],
                        td!["server"],
                        td!["zero-copy body buffers (required by hyper)"],
                        td!["—"],
                    ],
                    tr![
                        td![code!["futures-util"]],
                        td!["view, server"],
                        td![code!["FuturesUnordered"], ", stream combinators for streaming SSR"],
                        td!["hand-written executor combinators"],
                    ],
                    tr![
                        td![code!["serde"], ", ", code!["serde_json"]],
                        td!["most"],
                        td!["config, manifests, JSON APIs, action I/O"],
                        td!["—"],
                    ],
                    tr![
                        td![code!["serde_urlencoded"]],
                        td!["server"],
                        td!["query strings and forms into typed structs"],
                        td!["internal parser (edge cases in encoding)"],
                    ],
                    tr![td![code!["toml"], " (parse only)"], td!["core"], td![code!["next-rust.toml"]], td!["—"],],
                    tr![
                        td![code!["getrandom"]],
                        td!["server"],
                        td!["CSPRNG for nonces, CSRF tokens, session ids, digests"],
                        td![code!["rand"], " (larger)"],
                    ],
                    tr![
                        td![code!["hmac"], ", ", code!["sha2"]],
                        td!["server"],
                        td!["HMAC-SHA256 for signed server-action URLs (RustCrypto, audited, constant-time verify)"],
                        td!["hand-written SHA-256 (not worth the risk)"],
                    ],
                    tr![
                        td![code!["syn"], ", ", code!["quote"], ", ", code!["proc-macro2"]],
                        td!["build, macros"],
                        td!["parsing Rust source and generating code"],
                        td!["regex-based parsing (fragile)"],
                    ],
                    tr![
                        td![code!["flate2"], ", ", code!["brotli"], " (feature ", code!["compression"], ", default)"],
                        td!["server"],
                        td![
                            "response compression: Brotli when the client accepts it, gzip (pure-Rust miniz backend) otherwise"
                        ],
                        td!["gzip only (larger responses)"],
                    ],
                    tr![
                        td![code!["tokio-tungstenite"], " (feature ", code!["websocket"], ")"],
                        td!["server"],
                        td!["RFC 6455 WebSocket protocol"],
                        td!["—"],
                    ],
                    tr![
                        td![code!["redis"], " (feature ", code!["jobs-redis"], ")"],
                        td!["jobs"],
                        td!["the reliable job queue: Lua scripts over a reconnecting multiplexed connection"],
                        td!["a hand-written RESP client"],
                    ],
                    tr![
                        td![code!["tracing"], " (feature ", code!["tracing"], ")"],
                        td!["server"],
                        td!["ecosystem-standard instrumentation"],
                        td!["—"],
                    ],
                ],
            ],
        ],
        p![
            "All dependencies are MIT and/or Apache-2.0 licensed and actively maintained at the time of writing. Default features are disabled where possible (",
            code!["toml"],
            " parse-only, ",
            code!["tokio"],
            " feature-selected, ",
            code!["futures-util"],
            " without executor).",
        ],
        p![
            "The CLI has no dependencies beyond the framework crates and serde. It uses ",
            code!["std::process"],
            " and polling instead of ",
            code!["clap"],
            ", ",
            code!["notify"],
            " or an async runtime.",
        ],
        h2![id("frontend-runtime"), a![class("anchor"), href("#frontend-runtime"), "Frontend runtime"]],
        p![
            code!["/_next-rust/runtime.js"],
            " is the only script the framework itself sends to browsers, and only to pages that need it. It is one dependency-free ES module (",
            fragment![
                "about 4 KB gzipped) with four responsibilities: client navigation, streaming swaps after navigation, form and action enhancement, and island hydration. It never uses ",
                code!["eval"],
                " or ",
                code!["new Function"],
                ". Island hydration is attribute-driven, or delegated to ES modules through the ",
                code!["hydrate(element, props)"],
                " contract, which WASM glue code can implement. The streaming swap function is separate and inlined (about ",
            ],
            "150 bytes) only on pages that suspend, so streaming works without the runtime.",
        ],
    ]
}
