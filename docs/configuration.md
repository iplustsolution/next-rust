# Configuration

Configuration lives in `next-rust.toml`. `next-rust.json` with the same shape
is also accepted. **Every key is optional.** A project without a config file
uses the defaults below. Unknown keys are errors, so typos surface
immediately.

## Discovery

1. `NEXT_RUST_CONFIG=/path/to/file` wins.
2. Otherwise, the working directory and its ancestors are searched for
   `next-rust.toml` or `next-rust.json`. Having both in one directory is an
   error.
3. A compiled app that can't find its configuration from the working
   directory falls back to the project root recorded at build time. This
   lets `./target/release/my-app` run from anywhere.

Relative paths resolve against the **directory containing the config file**,
never the process working directory. `..` is allowed.

## Reference

```toml
[app]
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
site_id = "abc"
```

`{nonce}` in `security.csp` is replaced with the per-request nonce that the
framework puts on its own inline scripts.

## Environment variables

`.env` files are loaded at startup, from lowest to highest priority:

1. `.env`
2. `.env.{development|production|test}`
3. `.env.local` (skipped in `test`)
4. `.env.{environment}.local`

Variables already set in the process environment always win over files.
`NEXT_RUST_ENV` selects the environment. It defaults to `development` for
debug builds and `production` for release builds, and `next-rust dev` /
`start` set it for you.

**Server-only vs public.** All variables are available to server code via
`std::env::var`. Only variables starting with the public prefix
(`NEXT_RUST_PUBLIC_`) are ever sent to browsers, and only on pages that
render islands (as `window.nextRust.env`). A server secret never reaches the
client unless you render it into HTML yourself.

Framework variables:

| variable | effect |
|---|---|
| `NEXT_RUST_ENV` | `development`, `production`, `test` |
| `NEXT_RUST_CONFIG` | explicit config file |
| `PORT`, `HOST` | override `[server]` |
| `NO_COLOR` | disable ANSI colours |
| `NEXT_RUST_FRAMEWORK_PATH` | `next-rust new` uses a local checkout |

## Monorepos and workspaces

Each application package has its own `next-rust.toml`. The routing directory
can live outside the package:

```text
repo/
├── Cargo.toml                 [workspace]
├── shared/pages/              routes shared by several apps
└── apps/
    ├── site/
    │   ├── Cargo.toml
    │   ├── build.rs
    │   └── next-rust.toml     [app] directory = "../../shared/pages"
    └── admin/
        └── next-rust.toml     [app] directory = "app"
```

Workspace members build as usual (`cargo build -p site`). Run the CLI
commands from the app's directory.
