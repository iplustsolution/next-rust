//! The "configuration" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Configuration lives in ",
            code!["next-rust.toml"],
            ". ",
            code!["next-rust.json"],
            " with the same shape is also accepted. ",
            strong!["Every key is optional."],
            " A project without a config file uses the defaults below. Unknown keys are errors, so typos surface immediately.",
        ],
        h2![id("discovery"), a![class("anchor"), href("#discovery"), "Discovery"]],
        ol![
            li![code!["NEXT_RUST_CONFIG=/path/to/file"], " wins."],
            li![
                "Otherwise, the working directory and its ancestors are searched for ",
                code!["next-rust.toml"],
                " or ",
                code!["next-rust.json"],
                ". Having both in one directory is an error.",
            ],
            li![
                "A compiled app that can't find its configuration from the working directory falls back to the project root recorded at build time. This lets ",
                code!["./target/release/my-app"],
                " run from anywhere.",
            ],
        ],
        p![
            "Relative paths resolve against the ",
            strong!["directory containing the config file"],
            ", never the process working directory. ",
            code![".."],
            " is allowed.",
        ],
        h2![id("reference"), a![class("anchor"), href("#reference"), "Reference"]],
        pre![code![
            class("language-toml"),
            r#"[app]
directory = "app"            # routing directory: "src/web", "../shared/pages", "/abs/path"
public = "public"
base_path = ""               # mount everything under e.g. "/docs" (links and assets are not rewritten: include the prefix yourself)
trailing_slash = false       # true: /about → /about/
lang = "en"                  # <html lang>
html_precedence = "error"    # page.rs + page.html in one directory: error | rs | html
follow_symlinks = true       # symlink loops are detected either way

[api]
directory = "api"            # optional second tree with route.rs files only
prefix = "/api"

[server]
host = "0.0.0.0"
port = 3000                  # PORT / HOST environment variables override
body_limit = 2097152         # bytes
header_timeout = 10          # seconds to receive request headers
request_timeout = 60         # seconds until a handler is aborted with 504 (0 = off)
shutdown_timeout = 10        # graceful shutdown grace period
compression = true           # gzip
http2 = true                 # h2c / proxied HTTP/2
trust_proxy = false          # trust X-Forwarded-* headers

[build]
output = ".next-rust"
concurrency = 8              # reserved: static generation currently renders pages sequentially

[assets]
optimize = true              # reserved: CSS is always minified at compile time
public_max_age = 0

[rendering]
default = "auto"             # auto | static | dynamic ("server" = dynamic)
streaming = true
revalidate = 300             # default ISR interval for static pages (omit = never)
client_navigation = true     # Link! loads the client runtime

[images]
sizes = [640, 750, 828, 1080, 1200, 1920, 2048, 3840]   # reserved for the image optimizer; Image! uses these defaults
quality = 75                 # reserved for the image optimizer
max_age = 2592000            # Cache-Control max-age of /_nr/image responses

[security]
headers = true               # nosniff, SAMEORIGIN, referrer policy, COOP, permissions policy
csp = "default-src 'self'; script-src 'self' 'nonce-{nonce}'; style-src 'self' 'unsafe-inline'"
csrf = "origin"              # origin | token | off (server actions)
hsts_max_age = 0             # e.g. 31536000 when served only over HTTPS
allowed_origins = []         # extra origins allowed to call server actions

[env]
public_prefix = "NEXT_RUST_PUBLIC_"

[dev]
poll_interval = 250          # ms
overlay = true
watch = []                   # extra paths that trigger rebuilds
scaffold = true              # fill new, empty page.rs/layout.rs/route.rs/... with starter code

[logging]
format = "auto"              # auto (pretty in dev, JSON in prod) | pretty | json
level = "info"               # error | warn | info | debug | trace (default: info in development, error in production)
requests = true              # one line per request (default: on in development, off in production)

[[redirects]]
source = "/old/:slug"
destination = "/new/:slug"
permanent = true

[[headers]]
source = "/api/:path*"
headers = { "x-robots-tag" = "noindex" }

[plugins.analytics]          # free-form, read by plugins
site_id = "abc""#,
        ],],
        p![
            code!["{nonce}"],
            " in ",
            code!["security.csp"],
            " is replaced with the per-request nonce that the framework puts on its own inline scripts.",
        ],
        h2![id("environment-variables"), a![class("anchor"), href("#environment-variables"), "Environment variables"],],
        p![code![".env"], " files are loaded at startup, from lowest to highest priority:"],
        ol![
            li![code![".env"]],
            li![code![".env.{development|production|test}"]],
            li![code![".env.local"], " (skipped in ", code!["test"], ")"],
            li![code![".env.{environment}.local"]],
        ],
        p![
            "Variables already set in the process environment always win over files. ",
            code!["NEXT_RUST_ENV"],
            " selects the environment. It defaults to ",
            code!["development"],
            " for debug builds and ",
            code!["production"],
            " for release builds, and ",
            code!["next-rust dev"],
            " / ",
            code!["start"],
            " set it for you.",
        ],
        p![
            strong!["Server-only vs public."],
            " All variables are available to server code via ",
            code!["std::env::var"],
            ". Only variables starting with the public prefix (",
            code!["NEXT_RUST_PUBLIC_"],
            ") are ever sent to browsers, and only on pages that render islands (as ",
            code!["window.nextRust.env"],
            "). A server secret never reaches the client unless you render it into HTML yourself.",
        ],
        p!["Framework variables:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["variable"], th!["effect"]]],
                tbody![
                    tr![
                        td![code!["NEXT_RUST_ENV"]],
                        td![code!["development"], ", ", code!["production"], ", ", code!["test"]],
                    ],
                    tr![td![code!["NEXT_RUST_CONFIG"]], td!["explicit config file"]],
                    tr![td![code!["PORT"], ", ", code!["HOST"]], td!["override ", code!["[server]"]]],
                    tr![td![code!["NO_COLOR"]], td!["disable ANSI colours"]],
                    tr![td![code!["NEXT_RUST_FRAMEWORK_PATH"]], td![code!["next-rust new"], " uses a local checkout"],],
                ],
            ],
        ],
        h2![
            id("monorepos-and-workspaces"),
            a![class("anchor"), href("#monorepos-and-workspaces"), "Monorepos and workspaces"],
        ],
        p![
            "Each application package has its own ",
            code!["next-rust.toml"],
            ". The routing directory can live outside the package:",
        ],
        pre![code![
            class("language-text"),
            r#"repo/
├── Cargo.toml                 [workspace]
├── shared/pages/              routes shared by several apps
└── apps/
    ├── site/
    │   ├── Cargo.toml
    │   ├── build.rs
    │   └── next-rust.toml     [app] directory = "../../shared/pages"
    └── admin/
        └── next-rust.toml     [app] directory = "app""#,
        ],],
        p![
            "Workspace members build as usual (",
            code!["cargo build -p site"],
            "). Run the CLI commands from the app's directory.",
        ],
    ]
}
