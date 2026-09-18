# CLI, configuration and environment reference

Verified against Next Rust 0.1.7. Read this when you need an exact flag, key or default.

## Contents

- [CLI commands](#cli-commands)
- [What `next-rust build` does](#what-next-rust-build-does)
- [Flags on the app binary itself](#flags-on-the-app-binary-itself)
- [next-rust.toml: every key](#next-rusttoml-every-key)
- [Config discovery](#config-discovery)
- [Environment variables](#environment-variables)
- [.env files](#env-files)
- [What `next-rust new` generates](#what-next-rust-new-generates)

## CLI commands

`next-rust <command> [options]`. Every command takes `-h`/`--help`; the binary takes `-V`/`--version`.
Flags accept both `--port 3000` and `--port=3000`. Unknown flags are ignored silently, so check spelling.

| Command | Flags | What it does |
| --- | --- | --- |
| `new <name>` | `--git`, `--framework-path <path>`, `--no-tailwind` | Creates a project. Tailwind is on by default. The dependency source is `--framework-path`, else `NEXT_RUST_FRAMEWORK_PATH`, else `--git`, else crates.io when this CLI's version is published, else the Git repo. The name must be ASCII letters/digits/`-`/`_` and not start with a digit; the directory must be empty or absent. |
| `dev` | `--port`/`-p` | Watches files, rebuilds (debug) and runs the app with `NEXT_RUST_ENV=development`. On a compile or route error the previous build keeps serving. |
| `build` | `--no-check` | Production build: one stripped binary in `.next-rust/`. See below. |
| `start` | `--port`/`-p` | Runs the built binary with `NEXT_RUST_ENV=production`. Fails if you haven't built. |
| `check` | — | Validates routes and special files, then runs `cargo check`. |
| `routes` | `--json`, `--tree`, `--layouts` | Prints the route table (or the JSON manifest, the scanned tree, or the layout chain and why a route is dynamic). The fastest way to see how the framework understood your `app/` directory. |
| `analyze` | — | Reads the reports written by `build`: static/dynamic/API counts, the largest pre-rendered pages, binary size. Requires a previous `next-rust build`. |
| `generate <kind> <route>` | — | Scaffolds a special file. Kinds: `page`, `layout`, `template`, `loading`, `error`, `not-found`, `api` (alias `route`), `middleware`, and the undocumented `metadata`, `default`. Alias: `g`. Refuses to overwrite. |
| `doctor` | — | Checks the toolchain (needs Rust 1.88+), the config, `build.rs`, `src/main.rs`, whether `.env` is git-ignored, whether the port is free, and route diagnostics. Exits 1 when it finds problems. |
| `docker` | `--force` | Writes a `Dockerfile` and `.dockerignore`. |
| `editor` | `--force` | Writes the `.vscode/` snippets and settings. |
| `clean` | — | Deletes the output directory. |
| `upgrade` | `--cli`, `--project` | With no flags, both: `cargo update -p next-rust -p next-rust-build`, and reinstalls the CLI from Git. Alias: `update`. |

## What `next-rust build` does

1. Analyzes routes and stops on errors.
2. Downloads and verifies the Tailwind engine if needed (with a progress bar).
3. Runs `cargo build --release` for the app binary with a size profile forced through the environment:
   `NEXT_RUST_EMBED=1`, opt-level `z` (or `3` when `[build] optimize = "speed"`), fat LTO, one codegen unit,
   `strip=symbols`, no debug info, no incremental, panic from `[build] panic`, and `--remap-path-prefix` so no
   machine paths end up in the binary.
   **These override the project's `[profile.release]`** — to change size or panic behaviour, edit `[build]` in
   `next-rust.toml`, not `Cargo.toml`.
4. Copies the executable to `.next-rust/<name>`. That single file is the whole deployment: pages, config,
   `public/`, used `assets/` and `client/` are embedded.
5. Unless `--no-check`, runs the binary with `--export` in an empty directory to pre-render every static page.
   A failure here fails the build, which is what catches "works in dev, panics in prod" bugs.
6. Writes `routes.json` and `build.json` under `target/next-rust/` for `analyze`.

## Flags on the app binary itself

The compiled app accepts `--routes` (print the route table and exit) and `--export` (pre-render static pages,
print a JSON report, exit). Anything else serves.

## next-rust.toml: every key

`next-rust.json` with the same shape also works. Every section is optional, but **unknown keys are an error**
(`deny_unknown_fields`), so a typo fails the build instead of being ignored.

```toml
[app]
directory = "app"            # routing directory
public = "public"            # static files served from /
base_path = ""               # serve the app under a prefix (no leading slash, no trailing slash)
trailing_slash = false
lang = "en"                  # <html lang>
html_precedence = "error"    # error | rs | html — what wins when page.rs and page.html both exist
follow_symlinks = true

[api]
# directory = "api"          # default: unset, API routes live in app/**/route.rs
prefix = "/api"              # must start with /

[server]
host = "0.0.0.0"
port = 3000
body_limit = 2097152         # 2 MiB
header_timeout = 10          # seconds
request_timeout = 60         # seconds, 0 = never
shutdown_timeout = 10        # seconds to finish in-flight requests
compression = true
http2 = false
trust_proxy = false          # honour X-Forwarded-* headers

[build]
output = ".next-rust"
concurrency = 8              # reserved
optimize = "size"            # size | speed
panic = "unwind"             # unwind | abort (abort is smaller; a panic kills the server)

[assets]
optimize = true              # reserved
public_max_age = 0           # Cache-Control max-age for public/ files
prune_css = true             # release: drop CSS rules whose classes/ids are never used
css_safelist = []            # class/id names built at runtime that must keep their rules

[rendering]
default = "auto"             # auto | static | dynamic (alias: server)
streaming = true
# revalidate = 300           # default ISR interval for static pages; unset = never
client_navigation = true     # ship the client runtime so links navigate without a reload

[images]
sizes = [640, 750, 828, 1080, 1200, 1920, 2048, 3840]   # reserved
quality = 75                                             # reserved
max_age = 2592000

[security]
headers = true               # nosniff, frame options, referrer policy, COOP, permissions policy
# csp = "default-src 'self'" # {nonce} is substituted per response
csrf = "origin"              # origin | token | off (off skips only the origin check)
hsts_max_age = 0             # 0 = no header
allowed_origins = []
action_token_ttl = 43200     # seconds a signed server-action URL stays valid

[env]
public_prefix = "NEXT_RUST_PUBLIC_"   # only these variables reach the browser

[dev]
poll_interval = 250          # ms
overlay = true               # error overlay in the browser
watch = []                   # extra paths to watch
scaffold = true

[logging]
format = "auto"              # auto | pretty | json
# level = "info"             # unset: info in development, error otherwise
# requests = true            # unset: on in development, off otherwise

[[redirects]]
source = "/old"              # required, must start with /
destination = "/new"         # required
permanent = false

[[headers]]
source = "/assets/*"         # required
headers = { "cache-control" = "public, max-age=31536000, immutable" }   # required

[plugins.myplugin]           # free-form JSON for build plugins

[tailwind]
enabled = false
preflight = true             # Tailwind's base styles
dark_mode = "media"          # media | class | attribute
plugins = []                 # "@tailwindcss/typography", "@tailwindcss/forms"
safelist = []                # classes to generate even if not found in the source
sources = []                 # extra directories to scan for classes
css = ""                     # raw Tailwind CSS appended last (@keyframes, @layer base, …)
minify_classes = true        # release: short random class names
keep_classes = []            # classes that keep their names

[tailwind.theme]             # color-brand = "#f26b2a" → bg-brand, text-brand, …
[tailwind.utilities]         # btn = "rounded-lg px-4 py-2"  |  x = { content-visibility = "auto" }
[tailwind.variants]          # hocus = "&:hover, &:focus"
```

Config validation emits diagnostics `NR0001`–`NR0007` (missing app directory, bad API prefix, bad base path,
redirect source without a leading `/`, unknown log level, …).

## Config discovery

1. `NEXT_RUST_CONFIG` wins if set (an explicit file path).
2. Otherwise the framework walks up from the current directory for `next-rust.toml` or `next-rust.json`.
   Having both in one directory is an error. The walk stops at a directory with both `Cargo.toml` and `app/`.
3. Otherwise defaults, rooted at the current directory.

Relative paths resolve against the directory holding the config file. `PORT` and a non-empty `HOST` override
`[server]`. A release binary carries its config inside it; `NEXT_RUST_CONFIG` still overrides.

## Environment variables

| Variable | Effect |
| --- | --- |
| `NEXT_RUST_ENV` | `development` \| `production` \| `test`. Unset: development for debug builds, production for release. |
| `NEXT_RUST_CONFIG` | Path to the config file; beats discovery and a release binary's embedded config. |
| `NEXT_RUST_PUBLIC_*` | The only variables exposed to the browser (prefix configurable with `[env] public_prefix`). |
| `NEXT_RUST_EMBED` | `1`/`0`. Embed config and static files in the binary. Default: on for release. |
| `NEXT_RUST_MINIFY` | `1`/`0`. Minify generated HTML. Default: on for release. |
| `NEXT_RUST_PRUNE_CSS` | `1`/`0`. Drop unused CSS rules. Default: `[assets] prune_css` on release. |
| `NEXT_RUST_MINIFY_CLASSES` | `1`/`0`. Short Tailwind class names. Default: `[tailwind] minify_classes` on release. |
| `NEXT_RUST_CLASS_SEED` | Fixes the short-class-name seed: reproducible builds, or several servers behind one load balancer. |
| `NEXT_RUST_TAILWIND_BIN` | Use this Tailwind executable instead of downloading one (offline, locked-down CI). |
| `NEXT_RUST_CACHE_DIR` | Moves the engine cache (default `~/.cache/next-rust`, `%LOCALAPPDATA%\next-rust`). |
| `NEXT_RUST_BANNER` | `0` suppresses the production startup banner. |
| `NEXT_RUST_FRAMEWORK_PATH` | Default `--framework-path` for `next-rust new`. |
| `NEXT_RUST_NO_UPDATE_CHECK` | Disables the daily update notice (also skipped when `CI` is set). |

Also read: `PORT`, `HOST`, `NO_COLOR`, `CI`, `CARGO`.

## .env files

Loaded lowest to highest precedence: `.env`, `.env.<environment>`, `.env.local` (skipped when the environment
is `test`), `.env.<environment>.local`. Real environment variables always win. Only `NEXT_RUST_PUBLIC_*`
variables reach the browser.

## What `next-rust new` generates

`Cargo.toml`, `build.rs`, `src/main.rs`, `next-rust.toml`, `.env.example`, `app/layout.rs`, `app/page.rs`,
`app/not-found.rs`, `app/api/hello/route.rs`, `public/favicon.svg`, `public/robots.txt`, `.gitignore`,
`README.md` and three `.vscode/` files. With `--no-tailwind` you also get `app/globals.css`; the default
Tailwind project has no CSS file at all.

The whole build script is:

```rust
// build.rs
fn main() {
    next_rust_build::generate();
}
```

and the whole binary is:

```rust
// src/main.rs
next_rust::app!();
```

`app!()` expands to a `main` that loads the config, builds the route table generated by the build script and
serves it. Use `Generator::new()…run()` in `build.rs` instead of `generate()` when you want to configure
Tailwind from Rust or add build plugins.
