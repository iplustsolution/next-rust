# CLI, configuration and environment reference

Verified against Next Rust 0.1.7. Read this when you need an exact flag, key or default.

## Contents

- [CLI commands](#cli-commands)
- [What `next-rust build` does](#what-next-rust-build-does)
- [Flags on the app binary itself](#flags-on-the-app-binary-itself)
- [next-rust.toml: every key](#next-rusttoml-every-key)
  - [WebAssembly](#webassembly)
- [Config discovery](#config-discovery)
- [Environment variables](#environment-variables)
- [.env files](#env-files)
- [What `next-rust new` generates](#what-next-rust-new-generates)

## CLI commands

`next-rust <command> [options]`. Every command takes `-h`/`--help`; the binary takes `-V`/`--version`.
Flags accept both `--port 3000` and `--port=3000`. Unknown flags are ignored silently, so check spelling.

| Command | Flags | What it does |
| --- | --- | --- |
| `new <name>` | `--git`, `--framework-path <path>`, `--no-tailwind`, `--wasm`, `--ui` | Creates a project. Tailwind is on by default. `--wasm` also writes the `islands/` crate, sets `[build] wasm_islands = true` and adds a working `#[client(wasm = "Counter")]` island at `/wasm`; `--ui` (with `--wasm`) also sets `[ui] script = "wasm"` and wires `start: next_rust_ui_wasm::boot`. The dependency source is `--framework-path`, else `NEXT_RUST_FRAMEWORK_PATH`, else `--git`, else crates.io when this CLI's version is published, else the Git repo. The name must be ASCII letters/digits/`-`/`_` and not start with a digit; the directory must be empty or absent. |
| `dev` | `--port`/`-p` | Watches files, rebuilds (debug) and runs the app with `NEXT_RUST_ENV=development`. On a compile or route error the previous build keeps serving. |
| `build` | `--no-check` | Production build: one stripped binary in `.next-rust/`. See below. |
| `start` | `--port`/`-p` | Runs the built binary with `NEXT_RUST_ENV=production`. Fails if you haven't built. |
| `check` | — | Validates routes and special files, then runs `cargo check`. |
| `routes` | `--json`, `--tree`, `--layouts` | Prints the route table (or the JSON manifest, the scanned tree, or the layout chain and why a route is dynamic). The fastest way to see how the framework understood your `app/` directory. |
| `analyze` | — | Reads the reports written by `build`: static/dynamic/API counts, the largest pre-rendered pages, binary size. Requires a previous `next-rust build`. |
| `generate <kind> <route>` | — | Scaffolds a special file. Kinds: `page`, `layout`, `template`, `loading`, `error`, `not-found`, `api` (alias `route`), `middleware`, and the undocumented `metadata`, `default`. Alias: `g`. Refuses to overwrite. |
| `generate islands` | `--ui`, `--no-demo` | Takes **no route**: it writes the `islands/` crate (a `cdylib` with its own `[workspace]` table and size profile, the right dependency source read back out of the project's own `Cargo.toml`, and a `next_rust_wasm::islands! { … }`), then sets `[build] wasm_islands = true` in `next-rust.toml` in place, keeping its comments. `--ui` adds `next-rust-ui-wasm`, `start: next_rust_ui_wasm::boot,` and `[ui] script = "wasm"`. Unless `--no-demo` it also writes `app/wasm/page.rs`, a working `#[client(wasm = "Counter")]`, so the project is diagnostic-clean and end-to-end correct on its first build. Never overwrites. |
| `doctor` | — | Checks the toolchain (needs Rust 1.89+), the config, `build.rs`, `src/main.rs`, whether `.env` is git-ignored, whether the port is free, and route diagnostics. When the project uses WebAssembly (a switch is on, **or** an `islands/` crate exists) it adds a section: `wasm32-unknown-unknown` installed (via rustup; silent when there is no rustup), `wasm-opt` on `PATH` (advisory — size only, does not fail), and every `NR03xx` source-level check the build script runs. Exits 1 when it finds problems. |
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
trusted_proxies = 1          # with trust_proxy: how many proxies are yours; client_ip() is that many
                             # X-Forwarded-For entries from the right (2 = CDN + load balancer)

[build]
output = ".next-rust"
concurrency = 8              # reserved
optimize = "size"            # size | speed
panic = "unwind"             # unwind | abort (abort is smaller; a panic kills the server)
signature = true             # <meta name="generator"> on every page, banner on the framework scripts
minify_js = true             # release: minify + mangle client/ and assets/ scripts (oxc)
drop_unused_classes = true   # release: leave out generated-looking classes (sm:x, bg-[…], w-1/3) nothing knows
inline_assets = false        # true: scripts (runtime, ui, islands, asset! scripts) are written into the page; lazy import()s → /_next-rust/m/<hash>
minify_html = true           # release: HTML without quotes around simple values, ="", default attributes, optional end tags; text is never touched
minify_names = true          # release: short random data-* attributes, custom properties (--primary → --b3), @keyframes, @layer, generated ids
keep_names = []              # names minify_names must leave alone: "data-state", "--brand", a keyframes/layer name, an id; "data-chart-*" = a family
rename_ids = []              # ids minify_names may rename ("newsletter-email", "auth-*"); ids are public, so no author-written id is renamed by default
svg_precision = 3            # decimals kept in SVG path data (d, points); 0, or minify_html = false, leaves them as written
drop_console = true          # release: remove console.* calls from the scripts that are minified (debugger statements always go)
wasm_islands = false         # true: the project's islands/ crate contributes to the site's one wasm module, for #[client(wasm = "…")].
                             # Needs the wasm32-unknown-unknown target; a missing target is a warning and the islands lose their behaviour, never
                             # a failed build. Obfuscation, not invisibility: the module is downloadable, disassemblable (wasm2wat) and its strings
                             # are plain text, so props stay public and secrets stay on the server. See "WebAssembly" below.
wasm_opt = true              # shrink every wasm module with `wasm-opt -Oz` when it is on PATH (~10-15%); false ships what the linker wrote.
                             # A missing or failing wasm-opt is a warning either way, never a failed build
wasm_keep_names = false      # true: keep the wasm name section, so functions carry their Rust names in a profiler and a stack trace. Costs a few
                             # kilobytes and undoes most of the obfuscation. NOT the same key as keep_names above, which is about minify_names

[assets]
optimize = true              # reserved
public_max_age = 0           # Cache-Control max-age for public/ files
prune_css = true             # release: drop CSS rules whose classes/ids are never used
css_safelist = []            # names built at runtime that keep their rules: classes, ids, and per page attributes ("data-mode") and elements ("<dialog")
css_keep_vars = []           # custom properties ("--brand") every page keeps though nothing on it reads them
css_static_attributes = []   # attributes a script names but that only have the values the server renders ("data-theme"): rules for other values are left out

[rendering]
default = "auto"             # auto | static | dynamic (alias: server)
streaming = true
# revalidate = 300           # default ISR interval for static pages; unset = never
client_navigation = true     # ship the client runtime so links navigate without a reload

[client]
enhance_forms = true         # same-origin forms (GET search/filters, POST to route.rs) are sent by the runtime, no page
                             # reload; false leaves all but server-action forms to the browser. Needs client_navigation.
runtime = "js"               # "wasm": ship the framework's browser runtime (navigation, prefetch, loading bar, form enhancement,
                             # hydration) as Rust compiled to WebAssembly instead of /_next-rust/runtime.js. Behaviour is identical

[ui]
script = "js"                # "wasm": take the shipped components' behaviour (select, date picker, toggles, dialogs) from Rust
                             # compiled to WebAssembly. Markup, stylesheet and keyboard contract are unchanged

[animation]                  # motion every page gets by default; see "Animation" below
enabled = true               # false leaves the motion stylesheet out entirely
pages = true                 # pages fade into each other (view transitions; plain fade elsewhere)
hover = true                 # every hover/focus/active/class change eases (colors, size, spacing, transforms…)
duration = 200               # ms for hover and state changes
page_duration = 250          # ms for the fade between two pages
easing = "cubic-bezier(0.2, 0, 0, 1)"

[loader]                     # the bar along the top of the page while the next page loads
enabled = true
color = "var(--nr-primary, #006fee)"   # any CSS color: "#f26b2a", "var(--color-brand)" from [tailwind.theme]
height = 4                   # px
shadow = true                # a glow, brightest under the leading edge
spinner = false              # small spinner in the corner
delay = 100                  # ms a load must take before the bar shows (prefetched pages never flash it)
speed = 200                  # ms to fill and fade once the page is there
easing = "ease"              # of the fill
z_index = 1600
at_bottom = false

[images]
sizes = [640, 750, 828, 1080, 1200, 1920, 2048, 3840]   # reserved
quality = 75                                             # reserved
max_age = 2592000

[security]
headers = true               # nosniff, frame options, referrer policy, COOP, permissions policy
# csp = "strict"             # preset: nonce-only scripts + 'strict-dynamic', everything else 'self' (next_rust::STRICT_CSP)
# csp = "default-src 'self'" # or your own; {nonce} is substituted per response
csrf = "origin"              # origin | token | off (off skips only the origin check)
hsts_max_age = 0             # 0 = no header
allowed_origins = []
action_token_ttl = 43200     # seconds a signed server-action URL stays valid

[env]
public_prefix = "NEXT_RUST_PUBLIC_"   # only these variables reach the browser; a prefix under 4 characters is
                                      # refused (an empty one would publish the whole environment)

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

[verification]               # ownership proofs; a real <meta> in every document, every environment
google = ""                  # content of google-site-verification (also what Merchant Center accepts)
bing = ""                    # msvalidate.01
yandex = ""                  # yandex-verification
pinterest = ""               # p:domain_verify
facebook = ""                # facebook-domain-verification
baidu = ""                   # baidu-site-verification
naver = ""                   # naver-site-verification
seznam = ""                  # seznam-wmt
other = {}                   # { "meta-name" = "content" }
google_file = ""             # "google1234abcd.html" → answered at /google1234abcd.html
bing_file = ""               # the code inside BingSiteAuth.xml → /BingSiteAuth.xml
ads_txt = []                 # records of /ads.txt
app_ads_txt = []             # records of /app-ads.txt
assetlinks = ""              # JSON → /.well-known/assetlinks.json (public/ never serves dot-directories)
apple_app_site_association = ""   # JSON → /.well-known/apple-app-site-association and /apple-app-site-association

[analytics]                  # tags written by the framework; an empty/absent key writes nothing
google_analytics = ""        # GA4 "G-XXXXXXXXXX" ("UA-…" is NR0011)
google_tag_manager = ""      # "GTM-XXXXXXX" (head script + <noscript> iframe right after <body>)
google_ads = ""              # "AW-123456789"
microsoft_clarity = ""       # project id
hotjar = ""                  # site id
meta_pixel = ""              # digits
pinterest_tag = ""           # digits
linkedin_insight = ""        # partner id, digits
tiktok_pixel = ""            # upper-case letters and digits
cloudflare_web_analytics = ""   # 32-hex beacon token
# plausible = "example.com"  # or { domain = "…", script = "https://…/js/script.js" } or { script = "https://plausible.io/js/pa-XXXX.js" }
# umami = "<website uuid>"   # or { website_id = "…", script = "https://stats.example.com/script.js" }
enabled_in_dev = false       # analytics load in production only; verification is always written
respect_dnt = true           # DNT / Global Privacy Control: nothing loads, nothing preconnects
consent = "none"             # none | required
exclude_paths = []           # "/login" (that page), "/student/*" (it and everything below), "/blog/*/edit"
skip_noindex = false         # true: none on pages whose robots metadata says noindex (incl. 404/error pages)
page_view_on_navigation = true   # nr:navigate → pintrk("page"), dataLayer {event:"nr.page_view"}; false also sets fbq.disablePushState
anonymize_ip = false         # adds anonymize_ip to the Google tag config (GA4 ignores it)

[plugins.myplugin]           # free-form JSON for build plugins

[tailwind]
enabled = false
preflight = true             # Tailwind's base styles
dark_mode = "media"          # media | class | attribute
plugins = []                 # "@tailwindcss/typography", "@tailwindcss/forms"
safelist = []                # classes to generate even if not found in the source
sources = []                 # extra directories to scan for classes
stylesheets = []             # CSS files compiled with Tailwind (@theme, @layer, @apply…), in order; NR0008 if missing
css = ""                     # raw Tailwind CSS appended last (@keyframes, @layer base, …)
minify_classes = true        # release: short random class names
keep_classes = []            # classes that keep their names

[tailwind.theme]             # color-brand = "#f26b2a" → bg-brand, text-brand, …
[tailwind.utilities]         # btn = "rounded-lg px-4 py-2"  |  x = { content-visibility = "auto" }
[tailwind.variants]          # hocus = "&:hover, &:focus"
```

Config validation emits diagnostics `NR0001`–`NR0011` (missing app directory, bad API prefix, bad base path,
redirect source without a leading `/`, unknown log level, malformed `[verification]`/`[analytics]` values, …).

### Analytics and verification

Never hand-write a verification `<meta>`, a gtag/GTM/pixel snippet or an `ads.txt` route: set the key and the
framework does it, in `next-rust dev` and in builds.

- **What is written:** verification `<meta>` tags in every document; for analytics one minified inline
  `<script nonce>` in `<head>` that defines the vendor globals (`dataLayer`, `gtag`, `fbq`, `clarity`, `pintrk`,
  `lintrk`, `ttq`, `hj`, `plausible`) and loads each vendor script `async` with the nonce, `preconnect` hints,
  GTM's `<noscript>` iframe right after `<body>` and `<noscript>` pixels for Meta/LinkedIn/Pinterest (not when
  `consent = "required"`). App code may call `gtag("event", …)`, `fbq("track", …)` etc. directly.
- **Where:** exactly once per document — dynamic, streamed, static/cached, PPR, error, 404 and full-document
  `page.html`. Client-navigation responses (`x-nr-nav: 1`) carry none.
- **Opting out:** `exclude_paths`, `Metadata::new().analytics(false)` (page or layout; a child can pass `true`),
  `skip_noindex`. Loaded tags watch the History API, so a client navigation that crosses between a page with
  analytics and one without is answered with `204` + `x-nr-reload: 1` and the browser loads the URL in full.
  Navigations inside either area stay client-side.
- **Consent:** `consent = "required"` sets Consent Mode v2 defaults to `denied` (`ad_storage`, `ad_user_data`,
  `ad_personalization`, `analytics_storage`, `wait_for_update: 500`) before the Google tags load; Clarity and
  Hotjar wait for `analytics`, Meta/Pinterest/LinkedIn/TikTok for `ads`; Plausible, Umami and Cloudflare are
  cookieless and do not wait. The banner calls `nextRust.consent.grant()` / `.grant(["analytics"])` /
  `.revoke(["ads"])` / `.state()`; the choice is kept in `localStorage["nr-consent"]`.
- **Page views on client navigations:** GA4 (enhanced measurement), Meta, TikTok, Clarity, Hotjar, Plausible,
  Umami and Cloudflare follow the History API themselves. The framework calls `pintrk("page")` and pushes
  `{event: "nr.page_view", page_location, page_title}` to `dataLayer` for GTM triggers. LinkedIn has no such call.
- **CSP:** `csp = "strict"` is widened by exactly the configured tags' origins (`script-src`, `connect-src`,
  `img-src`, `frame-src`, Hotjar also `font-src`). A hand-written policy is not touched. Google Signals/Ads
  remarketing also contact `https://www.google.<tld>` country domains, which no policy can wildcard.
- **Merchant Center** has no tag of its own: it accepts the Search Console verification (`google`), GTM or GA.

### Animation

Every document starts with a motion stylesheet built from `[animation]` (nothing to write, nothing to import):

- **Pages fade into each other.** Client navigations swap the page inside `document.startViewTransition`;
  full loads use cross-document view transitions (`@view-transition { navigation: auto }`); browsers without
  either get a plain fade out/in. Streamed suspense boundaries fade in the same way.
- **Hover, focus, active and class changes ease** on every element: colors, shadows, opacity, transforms,
  size, spacing and type transition over `duration` ms with `easing`. A `hover:scale-110` or `hover:w-64`
  therefore animates instead of jumping.
- The rule sits in the lowest cascade layer (`nr-motion`, declared before Tailwind's), so anything the app
  writes wins: its own `transition`, Tailwind `transition-*`/`duration-*`/`transition-none` utilities, or
  `data-nr-motion="off"` on an element to exclude it and its subtree.
- `--nr-page-fade`, `--nr-motion` and `--nr-motion-ease` on `<html>` carry the values; override them in
  CSS for one page (`html { --nr-page-fade: 0ms }` turns the fade off there). `prefers-reduced-motion`
  sets both durations to `0ms`.
- Turn parts off with `pages = false` / `hover = false`, or everything with `enabled = false`. The
  runtime reads the values from the stylesheet, so no script changes are needed.

### Loading bar

`[loader]` draws a thin bar along the top of the page while the next page is on its way (like
`nextjs-toploader`), and it disappears once the page is there. It runs during client navigations,
server-action form submits and full-page link clicks (other origins, `data-nr-reload`). It is
`html[data-nr-loading]::before`, styled entirely from the config: `color` (your brand color or a CSS
variable), `height`, `shadow`, `spinner`, `delay`, `speed`, `easing`, `z_index`, `at_bottom`. It shows only
once a load has taken `delay` ms, so instant (prefetched) navigations never flash it. The bar spans the page
and is slid by a transform (`--nr-load`, no layout): fast at first, ever slower, never resting; on arrival
`data-nr-loading="done"` fills it with `easing` in `speed` ms, then fades it in `speed` ms. `shadow` adds a glow
and a brighter peg under the leading edge. Native form submits and full page loads keep the bar: the next
document resumes it (sessionStorage key `nr-load`) and completes it on `load`. Reduced motion gets a bar that
fades in place. From your own scripts:
`const done = nextRust.loader.start(); await work; done();`. `enabled = false` removes it.

### WebAssembly

Five keys, three of them switches that each move one piece of browser logic from JavaScript to Rust. All
three default to JavaScript, so upgrading changes nothing until you edit the file.

**Do not set this up by hand.** `next-rust generate islands` (or `next-rust new --wasm`) writes the crate
and the keys together; `next-rust doctor` checks the rest. The manual version is four separate things that
each fail silently.

| Key | Default | Effect | Needs the `islands/` crate? |
| --- | --- | --- | --- |
| `[build] wasm_islands` | `false` | The project's `islands/` crate contributes to the module; `#[client(wasm = "Name")]` binds to its `hydrate_Name` export. | yes |
| `[ui] script` | `"js"` | `"wasm"` takes the shipped components' behaviour from Rust. | yes, with `start: next_rust_ui_wasm::boot,` in its `islands!` |
| `[client] runtime` | `"js"` | `"wasm"` ships the framework's browser runtime as Rust instead of `/_next-rust/runtime.js`. | **no** |
| `[build] wasm_opt` | `true` | Run `wasm-opt -Oz` when it is on `PATH`. | — |
| `[build] wasm_keep_names` | `false` | Keep the wasm name section (readable Rust function names, a few kB larger). | — |

`[client] runtime = "wasm"` is the odd one out and worth knowing: it is served from
`/_next-rust/runtime.wasm`, a **second, prebuilt module carried inside `next-rust-server`** (written by
`crates/next-rust-runtime/regenerate.sh`). It needs no `islands/` crate, no wasm target and no `wasm-opt`.
So "everything links into one module" is the design, not yet the fact — there are two today, and a page with
both fetches the shared machinery twice.

The app's islands and the UI kit do share one module, `/_next-rust/client/__nrw/islands.wasm`, because they
link against the same base crate and separate modules would duplicate the allocator and string machinery in
each copy. Per-island cost shows up in the build report, not on the wire.

- **Any** of the three being on is what makes a page compile a module, and therefore what adds
  `'wasm-unsafe-eval'` to `script-src`. That keyword permits `WebAssembly.instantiate` only: not `eval`, not
  inline scripts. It composes with `[analytics]`, which widens the same directive with the tag origins.
- **Nothing here can fail a build except your own code.** A missing `wasm32-unknown-unknown` target, a
  missing `wasm-opt`, a missing `islands/` crate, an island the module does not export: each is an `NR03xx`
  `cargo:warning=` naming the fix, and the build produces a working binary with the behaviour left out. A
  compile error inside your `islands/` crate is a hard error, like any other compile error. **This is why
  you read the warnings**: the runtime symptom is a page that renders, looks finished and does nothing.
- `next-rust build` prints the module raw and brotli, the shim's raw/minified/brotli beside it, and a line
  per island; the same figures are in the JSON build report under `wasm`.

Measured on the project `next-rust new --wasm` generates (one counter island, `wasm-opt -Oz`, brotli is what
is served):

| In the module | raw | brotli |
| --- | --- | --- |
| one island, nothing else | 6,395 B | 3,095 B |
| the same island plus the shipped components (`--ui`) | 86,282 B | 32,920 B |
| the loader beside it (`__nrw.js`, minified, one per site) | 16,082 B | 5,138 B |

So the first island costs roughly 8 kB brotli and each one after it costs its own code (this counter: 3.5 kB
of the module). WebAssembly here is a way to write browser behaviour in Rust, **not** a size optimisation.
- Delivery: `application/wasm` (a browser will not stream-compile anything else), brotli/gzip
  precompressed with `vary: accept-encoding`, a content-hashed `?v=` URL served
  `public, max-age=31536000, immutable`, and a `<link rel="preload" as="fetch" crossorigin>` so the
  download is not serialised behind HTML parsing. `crossorigin` is required even same-origin, or the module
  is fetched twice.
- `[build] inline_assets = true` inlines the shim like any other script; the `.wasm` stays a separate fetch
  (a data URI would be ~33% larger, could not be stream-compiled, and would never be cached).
- **It is obfuscation, not a security boundary.** A readable JavaScript shim always remains, the module is
  downloadable and disassemblable (`wasm2wat`), and every string literal sits in the data section in plain
  text. Props are public. Secrets belong on the server.
- A class or `data-*` name that only the islands crate writes keeps its spelling automatically: the build
  reads the crate's string literals, because a name compiled into the module cannot be rewritten afterwards.

## Config discovery

1. `NEXT_RUST_CONFIG` wins if set (an explicit file path).
2. Otherwise the framework walks up from the current directory for `next-rust.toml` or `next-rust.json`.
   Having both in one directory is an error. The walk stops at a directory with both `Cargo.toml` and `app/`.
3. Otherwise defaults, rooted at the current directory.

Relative paths resolve against the directory holding the config file. `PORT` and a non-empty `HOST` override
`[server]`. A release binary carries its config inside it; `NEXT_RUST_CONFIG` still overrides.

`next-rust-build`'s `minify-js` feature pins `bumpalo = 3.19`; if `cargo build` reports a `bumpalo` version
conflict in an application, run `cargo update -p bumpalo --precise 3.19.0` once.

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
| `NEXT_RUST_MINIFY_JS` | `1`/`0`. Minify and mangle `client/` and `assets/` scripts. Default: `[build] minify_js` on release. |
| `NEXT_RUST_MINIFY_HTML` | `1`/`0`. Compact HTML (quotes, `=""`, default attributes, optional end tags). Default: `[build] minify_html` on release. |
| `NEXT_RUST_MINIFY_NAMES` | `1`/`0`. Short `data-*`, custom property, `@keyframes`, `@layer` and generated id names. Default: `[build] minify_names` on release. |
| `NEXT_RUST_DROP_CLASSES` | `1`/`0`. Leave out generated-looking classes no stylesheet, script or static file knows. Default: `[build] drop_unused_classes` on release. |
| `NEXT_RUST_PRECOMPRESS` | `1`/`0`. Compress embedded files with Brotli and gzip at build time. Default: on when files are embedded. |
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
