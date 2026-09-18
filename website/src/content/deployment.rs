//! The "deployment" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "A Next Rust application deploys as ",
            strong!["one file"],
            ": the binary that ",
            code!["next-rust build"],
            " writes to ",
            code![".next-rust/<name>"],
            ". It needs no runtime, interpreter, Node.js or files next to it.",
        ],
        h2![id("what-to-ship"), a![class("anchor"), href("#what-to-ship"), "What to ship"]],
        pre![code![class("language-text"), r".next-rust/my-app    # that's all"]],
        p!["Release builds compile these into the binary:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["part"], th!["how it gets in"]]],
                tbody![
                    tr![
                        td!["pages, layouts, API routes, server actions"],
                        td!["compiled Rust (", code!["page.html"], " files as minified strings)"],
                    ],
                    tr![td![code!["next-rust.toml"]], td!["embedded; it becomes the configuration of the binary"],],
                    tr![
                        td![code!["public/"], ", ", code!["assets/"], ", ", code!["client/"]],
                        td!["embedded and served from memory, with ETags and range requests"],
                    ],
                    tr![
                        td!["CSS (", code!["global_css!"], ", ", code!["css_module!"], ") and the browser runtime",],
                        td!["embedded"],
                    ],
                ],
            ],
        ],
        p![
            "Static pages are rendered into memory in the background when the binary starts, so the server never writes to disk. The build also runs the binary from an empty directory to prove it needs nothing else.",
        ],
        p![
            "Large files in ",
            code!["public/"],
            " (videos, archives) grow the binary by their size; serve those from a CDN or object storage instead.",
        ],
        h3![id("binary-size"), a![class("anchor"), href("#binary-size"), "Binary size"]],
        p!["Only what the app uses goes into the binary:"],
        ul![
            li![
                "Rust compiles only modules that are reachable from the app: special files in ",
                code!["app/"],
                " and what they (and ",
                code!["src/main.rs"],
                ") import. Other files in the folders are never compiled. Link-time optimization across all crates then removes every function that is never called.",
            ],
            li![
                "Files in ",
                code!["assets/"],
                " are embedded only when some ",
                code!["asset!(\"…\")"],
                " refers to them.",
            ],
            li![
                "The configuration is embedded as JSON, so no TOML parser is linked. ",
                code!["NEXT_RUST_CONFIG"],
                " can still replace it with a ",
                code![".json"],
                " file.",
            ],
            li![
                "HTTP/2 over plain TCP is a cargo feature (",
                code!["http2"],
                "), off by default: browsers only use HTTP/2 over TLS, which your proxy terminates."
            ],
            li!["Only the minified browser runtime is included; the readable copy used in development is left out."],
        ],
        p![
            "The compiler optimizes for size (",
            code!["opt-level = \"z\""],
            "), with fat LTO, one codegen unit and stripped symbols. Paths of the machine that built it (home directory, project folder, Cargo registry) are rewritten, so none appear in the binary. Measured on a new project:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["settings"], th!["size"], th!["requests/s (static page)"]]],
                tbody![
                    tr![td!["before these optimizations (opt-level 3, HTTP/2, TOML)"], td!["2.39 MB"], td!["–"]],
                    tr![td![code!["optimize = \"size\""], " (default)"], td!["1.20 MB"], td!["120,559"]],
                    tr![td![code!["optimize = \"speed\""]], td!["1.73 MB"], td!["121,516"]],
                    tr![td![code!["optimize = \"size\""], ", ", code!["panic = \"abort\""]], td!["0.97 MB"], td!["–"]],
                ],
            ],
        ],
        p![
            code!["panic = \"abort\""],
            " saves another 20% but changes failure behaviour: with the default ",
            code!["unwind"],
            ", a panic in a page or API handler fails only that request and the server keeps running. With ",
            code!["abort"],
            ", one panic stops the process, so run it under something that restarts it (systemd, Docker, Kubernetes).",
        ],
        p![
            "Environment: ",
            code!["NEXT_RUST_ENV=production"],
            ", ",
            code!["PORT"],
            ", ",
            code!["HOST"],
            ", ",
            code!["NEXT_RUST_SECRET"],
            " (32+ random bytes, e.g. ",
            code!["openssl rand -hex 32"],
            "; signs server-action URLs, and must be the same on every instance), and your secrets. ",
            code![".env"],
            " files are not embedded, so set secrets in the environment of the process. ",
            code!["NEXT_RUST_CONFIG=/path/to/next-rust.toml"],
            " replaces the embedded configuration.",
        ],
        p![
            code!["cargo build --release"],
            " produces the same self-contained binary in ",
            code!["target/release/"],
            ". Set ",
            code!["NEXT_RUST_EMBED=0"],
            " to read files from disk instead.",
        ],
        h2![
            id("what-production-builds-send-to-browsers"),
            a![
                class("anchor"),
                href("#what-production-builds-send-to-browsers"),
                "What production builds send to browsers",
            ],
        ],
        p![
            "Release builds (",
            code!["next-rust build"],
            ") are optimized for size, and make the code reaching the browser hard for people to read:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["output"], th!["development"], th!["production"]]],
                tbody![
                    tr![td!["HTML rendered from Rust"], td!["compact, no formatting whitespace"], td!["same"]],
                    tr![
                        td![code!["page.html"], " files"],
                        td!["served as written"],
                        td!["comments and indentation removed at build time"],
                    ],
                    tr![
                        td!["CSS (", code!["global_css!"], ", ", code!["css_module!"], ")"],
                        td!["minified"],
                        td!["minified, class names hashed"],
                    ],
                    tr![
                        td!["client runtime (", code!["/_nr/runtime.js"], ")"],
                        td!["readable source"],
                        td!["minified with local names mangled (~4 KB gzipped)"],
                    ],
                    tr![td!["error pages"], td!["full details"], td!["generic message and a digest"]],
                    tr![
                        td!["server binary"],
                        td!["debug info, reads files from disk"],
                        td!["one stripped file with everything embedded, optimized for size, no build paths"],
                    ],
                ],
            ],
        ],
        p![
            "Development keeps the readable versions so errors are easy to debug. Set ",
            code!["NEXT_RUST_MINIFY=1"],
            " to minify ",
            code!["page.html"],
            " files in a debug build, or ",
            code!["NEXT_RUST_MINIFY=0"],
            " to keep them readable in a release build.",
        ],
        p![
            "Minification isn't a security boundary. Anything a browser can run, a person can open in the browser's developer tools and pretty-print. Keep secrets, business logic and credentials in server code (pages, API routes, server actions), which never leaves the server.",
        ],
        h3![id("startup-screen"), a![class("anchor"), href("#startup-screen"), "Startup screen"]],
        p![
            "When you run the binary in a terminal (",
            code!["./.next-rust/my-app"],
            " or ",
            code!["next-rust start"],
            "), it shows the Next Rust wordmark, the local and network addresses, the number of routes and how to stop it, and confirms a graceful stop on Ctrl+C. Under Docker, systemd or any process manager stdout is not a terminal, so it prints nothing but errors. Set ",
            code!["NEXT_RUST_BANNER=0"],
            " to hide the screen in a terminal too.",
        ],
        h2![id("bare-metal-vps"), a![class("anchor"), href("#bare-metal-vps"), "Bare metal / VPS"]],
        pre![code![
            class("language-sh"),
            r"next-rust build
scp .next-rust/my-app server:/srv/my-app/
ssh server 'NEXT_RUST_ENV=production PORT=8080 /srv/my-app/my-app'",
        ],],
        p!["A systemd unit:"],
        pre![code![
            class("language-ini"),
            r"[Unit]
Description=my-app
After=network.target

[Service]
WorkingDirectory=/srv/my-app
ExecStart=/srv/my-app/my-app
Environment=NEXT_RUST_ENV=production PORT=8080
EnvironmentFile=-/srv/my-app/.env.production.local
Restart=on-failure
DynamicUser=yes
StateDirectory=my-app

[Install]
WantedBy=multi-user.target",
        ],],
        p![
            "The server shuts down gracefully on ",
            code!["SIGTERM"],
            ". It stops accepting connections and waits up to ",
            code!["[server] shutdown_timeout"],
            " seconds for in-flight requests.",
        ],
        h2![id("docker"), a![class("anchor"), href("#docker"), "Docker"]],
        pre![code![
            class("language-sh"),
            r"next-rust docker
docker build -t my-app .
docker run -p 3000:3000 my-app",
        ],],
        p!["The generated Dockerfile:"],
        ul![
            li![
                "builds in ",
                code!["rust:1-slim"],
                " with BuildKit cache mounts for the registry and target directory;",
            ],
            li![
                "copies the single binary into ",
                code!["gcr.io/distroless/cc-debian12:nonroot"],
                ", so the image holds that one file on a minimal base and runs as an unprivileged user.",
            ],
        ],
        h2![id("kubernetes"), a![class("anchor"), href("#kubernetes"), "Kubernetes"]],
        ul![
            li![
                "Liveness and readiness: add a lightweight route, e.g. ",
                code!["App::new(routes()).get(\"/healthz\", |_| async { \"ok\" })"],
                ".",
            ],
            li![code!["terminationGracePeriodSeconds"], " should exceed ", code!["[server] shutdown_timeout"], ".",],
            li![
                "For more than one replica, use a shared ",
                code!["CacheStore"],
                " (see ",
                a![href("/docs/caching"), "caching"],
                ") so ISR revalidation and ",
                code!["revalidate_tag"],
                " affect every replica. With the default in-memory store, each pod has its own page cache, filled when it starts.",
            ],
            li!["The built-in rate limiter and ", code!["MemorySessionStore"], " are per pod."],
        ],
        h2![
            id("flyio-railway-render-and-similar-platforms"),
            a![
                class("anchor"),
                href("#flyio-railway-render-and-similar-platforms"),
                "Fly.io, Railway, Render and similar platforms",
            ],
        ],
        p![
            "These platforms detect a Dockerfile. ",
            code!["PORT"],
            " is honoured automatically. Set ",
            code!["NEXT_RUST_ENV=production"],
            " and ",
            code!["[server] trust_proxy = true"],
            " so client IPs and the scheme come from the platform's proxy.",
        ],
        h2![
            id("reverse-proxies-tls-and-http3"),
            a![class("anchor"), href("#reverse-proxies-tls-and-http3"), "Reverse proxies, TLS and HTTP/3"],
        ],
        p![
            "The built-in server speaks HTTP/1.1, and HTTP/2 over plain TCP (h2c) when the ",
            code!["http2"],
            " feature is enabled and ",
            code!["[server] http2 = true"],
            ". Terminate ",
            strong!["TLS"],
            " and ",
            strong!["HTTP/3"],
            " at a reverse proxy or load balancer (Caddy, nginx, Envoy, a cloud load balancer). This is the usual production setup and keeps certificate management out of the application.",
        ],
        pre![code![
            class("language-caddyfile"),
            r"example.com {
    reverse_proxy 127.0.0.1:3000
}",
        ],],
        p!["With a proxy in front:"],
        ul![
            li!["set ", code!["[server] trust_proxy = true"], ";"],
            li!["set ", code!["[security] hsts_max_age = 31536000"], " once the site is HTTPS-only;"],
            li![
                "leave ",
                code!["[server] compression = true"],
                " unless the proxy compresses. Streamed HTML flushes per chunk either way, but proxies must not buffer it. Nginx honours the ",
                code!["x-accel-buffering: no"],
                " header the framework sends for SSE; for streamed HTML, set ",
                code!["proxy_buffering off"],
                " on those locations.",
            ],
        ],
        p![
            "Native TLS and HTTP/3 in the server are on the roadmap (",
            a![href("/docs/status"), "Status & roadmap"],
            ").",
        ],
        h2![id("serverless-and-edge"), a![class("anchor"), href("#serverless-and-edge"), "Serverless and edge"],],
        p![
            "The framework core is independent of the listener. ",
            code!["App::handle(Request)"],
            " takes a request and returns a response:",
        ],
        pre![code![
            class("language-rust"),
            r#"next_rust::routes!();

// Example adapter for any platform that gives you an http::Request<Bytes>.
pub async fn handle(req: http::Request<bytes::Bytes>) -> http::Response<Vec<u8>> {
    static APP: std::sync::OnceLock<next_rust::App> = std::sync::OnceLock::new();
    let app = APP.get_or_init(|| next_rust::App::new(routes()).build().expect("app"));
    let res = app.handle(next_rust::Request::from_http(req)).await;
    let (status, headers) = (res.status, res.headers.clone());
    let body = res.into_bytes().await.unwrap_or_default().to_vec();
    let mut out = http::Response::new(body);
    *out.status_mut() = status;
    *out.headers_mut() = headers;
    out
}"#,
        ],],
        p![
            "Wrap this in ",
            code!["lambda_http"],
            ", a Cloudflare Workers WASM shim, or a similar adapter. Constraints on serverless platforms:",
        ],
        ul![
            li![
                "the process may be frozen between requests, so background ISR regeneration may be delayed. Prefer on-demand ",
                code!["revalidate_tag"],
                ";",
            ],
            li!["configure a ", code!["CacheStore"], " backed by the platform's KV store;"],
            li![
                "edge runtimes compiled to ",
                code!["wasm32"],
                " can't use Tokio's networking or the filesystem store. The request pipeline itself doesn't need them, but no official edge adapter ships yet.",
            ],
        ],
        h2![id("observability"), a![class("anchor"), href("#observability"), "Observability"]],
        ul![
            li![
                strong!["Production is silent except for errors."],
                " No startup banner, no request lines, no warnings: only errors are printed, as one JSON object per line (",
                code!["{\"level\":\"error\",\"msg\":\"render error [fa28afc233e6]: …\",\"ts\":…}"],
                "). Development prints everything in a readable format.",
            ],
            li![
                "To see more in production, opt in with ",
                code!["[logging] level = \"info\""],
                " and/or ",
                code!["requests = true"],
                ".",
            ],
            li![
                code!["request_id()"],
                " middleware adds ",
                code!["x-request-id"],
                ", and error digests appear in both the log and the error UI.",
            ],
            li![
                "With the ",
                code!["tracing"],
                " feature, all framework log events go through ",
                code!["tracing"],
                ", so any subscriber (OpenTelemetry, Datadog, JSON) collects them.",
            ],
            li![
                "Secrets aren't logged. The framework logs method, path, status and duration, never headers, cookies or bodies.",
            ],
        ],
    ]
}
