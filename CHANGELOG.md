# Changelog

All notable changes to this project are documented here. The project follows
[Semantic Versioning](https://semver.org/); until 1.0, minor versions may
contain breaking changes.

## Unreleased

- **The UI components' browser behaviour, in Rust: `[ui] script = "wasm"`.** The 113 shipped components keep their
  keyboard and ARIA contract — the select's listbox and type-ahead, the calendar's arrow keys, menu roving focus,
  the command palette, the combobox, tags, toasts, the carousel, `on_press` — but the behaviour is now also
  written as Rust (`next-rust-ui-wasm`, `no_std`) and compiled into the one WebAssembly module a site shares,
  with a two-line loader in the page instead of 27 KB of JavaScript. The markup, the stylesheet and the
  zero-script fallback are unchanged, so it is a delivery choice rather than a behaviour one, and `"js"` stays the
  default. It costs bytes rather than saving them — about 31 KB brotli of module against 7.6 KB of minified
  script — so take it because you want the behaviour in Rust, or because the site already ships a module the kit
  can ride along in. Verified in a browser on both paths: the same 41 keyboard and interaction checks pass in
  light and dark at 390 and 1440, and the eight no-script checks pass unchanged.

- **Fixed: a failed server action threw away `code` and `data`.** `nextRust.action` rejected with an `Error`
  carrying only `.errors` and `.status`, while `nextRust.request` and `nextRust.stream` both carried `.data`.
  An action that refuses for a reason the page can act on — a quota reached, a plan that needs upgrading, a
  mode that is blocked — says so in `body.code`, with whatever goes with it in `body.data`, and both were
  discarded before any caller could see them. Every `catch` that branched on a code was dead, and the visitor
  got the raw server sentence instead of the button that would have fixed it. The thrown error now carries
  `.code` and `.data` as well, in both the JavaScript and the WebAssembly runtime. No other behaviour changes,
  and code that only reads `.message`, `.errors` or `.status` is unaffected.

- **A browser API for Rust islands: `next-rust-wasm`.** An island crate now depends on one crate and writes no
  boilerplate: `next_rust_wasm::islands! { hydrate_AuthSwitch => crate::auth_switch::hydrate }` emits the
  allocator, the panic handler and every export the shim calls. It brings elements and lists, strings read
  *out* of the DOM, listeners as Rust closures (so two instances of one island no longer share a `static`),
  `async` on a small in-crate executor (`spawn`, `timeout`, `raf`, `fetch`, `action`, `sse`), timers and
  observers, storage, history, cookies, `Intl` formatting done by the browser rather than by linking ICU, and
  `Val` — a handle to any JavaScript value, which reaches files, `FormData`, `URL` and a vendored SDK without
  an opcode each. Props are a compact binary blob whose field names are compile-time hashes, so no field name
  appears in the module and nothing in the browser parses JSON. The allocator is a real one — size-class free
  lists over `memory.grow` — because the bump allocator it replaces never freed: fine for an island that runs
  once at hydration, a slow leak for one that allocates per event or per animation frame. `next-rust build` can
  report per-island wasm bytes (`wasm_size::report`), and the shim, still a pure opcode interpreter with no
  application behaviour in it, is now its own file. The honest limits are unchanged and worth repeating: a
  JavaScript shim always remains, because `WebAssembly.instantiate`, the import object and `addEventListener`
  can only be reached from JavaScript; and wasm is obfuscation, not invisibility — the module is downloadable,
  `wasm2wat`-able, and every string literal in it is plain text.
- **Islands can be written in Rust.** With `[build] wasm_islands = true` the project's `islands/` crate
  (`cdylib`, `no_std`, `panic = "abort"`, no dependencies) is compiled to one WebAssembly module shared by the
  whole site, and `#[client(wasm = "AuthSwitch")]` binds a component to its `hydrate_AuthSwitch` export. A small
  JavaScript shim hands the module its DOM operations and the runtime helpers (`nextRust.action`, `navigate`)
  through an opcode table; props arrive as plain strings, so nothing has to parse JSON in the island. Needs
  `rustup target add wasm32-unknown-unknown`; `next-rust build` runs `wasm-opt -Oz` when it is on `PATH`, the CSP
  gains `'wasm-unsafe-eval'` only while the key is on, and a missing export, a refused compile or a browser
  without WebAssembly leaves the server-rendered markup in place. What this buys is one language across the app
  and a payload far harder to read or tamper with than minified JavaScript (the real `AuthSwitch` island is
  865 B brotli; three islands share 1037 B). What it is **not** is invisibility: the shim is JavaScript, the
  `.wasm` can be saved and disassembled with `wasm2wat`, and every string literal in it — selectors, class names,
  copy, URLs — is plain text. Props stay public, secrets stay on the server, and for a single small island the
  module plus shim is bigger than the JavaScript it replaces. Existing `#[client(script = …)]` islands are
  unaffected.
- **The browser runtime itself is now Rust.** `crates/next-rust-runtime` is the same program as `RUNTIME_JS`
  — client navigation with the layout-marker protocol and the 30 s page cache, hover prefetching, the loading
  bar, form enhancement, server actions, `nextRust.request`/`stream`, island hydration and the declarative
  bindings — written in Rust and compiled to one `wasm32-unknown-unknown` module served at
  `/_next-rust/runtime.wasm`. It is `no_std` with its own allocator and its own thread locals, has no
  dependencies, and parses the JSON it needs (`{ok,data,errors,redirect}`) and the numbers it reads
  (`parseFloat`) itself: `serde_json` and `core`'s `dec2flt` each cost more than the entire JavaScript runtime
  they would serve. Behind it is a shim that is a table indexed by opcode and nothing else — a test asserts it
  contains no `data-nr-*`, no `x-nr-*`, no `__next_rust_env`, no `eval`: every rule the runtime has lives in
  Rust. `[client] runtime = "wasm"` switches a page to it and `"js"` (the default) keeps the JavaScript one;
  both are shipped, both are tested, and an application can go back with one key.
  **On size, plainly: this is bigger, not smaller.** Measured: the JavaScript runtime is 21,216 B minified /
  6,820 B brotli; the WebAssembly one is 203,826 B / 68,749 B plus a 9,707 B / 3,252 B shim — **about ten
  times the bytes over the wire**. That is after `no_std` (with `std`, the bare skeleton was already twice
  the whole JavaScript runtime), after writing `parseFloat` by hand (`core::num::dec2flt` alone is ~10 KB of
  tables) and after a size-tuned cargo profile (405 KB → 203 KB). What is left is the ported logic itself.
  Rust does not compile to anything near the density of hand-written JavaScript for work that is mostly DOM
  calls and string handling; the full breakdown is in `crates/next-rust-runtime/src/lib.rs`. What it buys is
  one language across the whole application, the runtime's rules reviewable as Rust and reachable by the same
  tests as the server, and logic far harder to read or tamper with than minified JavaScript. What it costs is
  payload. Leave `[client] runtime = "js"` on unless you want that trade.
- **The whole browser runtime can be WebAssembly, and shipping it is now plumbed end to end.** Two new
  switches join `[build] wasm_islands`: `[client] runtime = "js" | "wasm"` moves the framework's own browser
  runtime (client navigation, prefetching, the loading bar, form enhancement, hydration) into Rust, and
  `[ui] script = "js" | "wasm"` does the same for the shipped components. Both default to `"js"`, so nothing
  changes until you ask. Everything that is on compiles into **one** module rather than one per crate, because
  they share an allocator and a string table. `[build] wasm_opt` (default `true`) and `[build] wasm_keep_names`
  (default `false`, and not the same key as `keep_names`) control `wasm-opt -Oz` and the wasm name section.
  Delivery caught up with it: the module is served `application/wasm`, brotli- and gzip-precompressed with
  `vary: accept-encoding`, under a content-hashed `?v=` URL cached `immutable`, and preloaded with
  `<link rel="preload" as="fetch" crossorigin>` so the download is not serialised behind HTML parsing; with
  `[build] inline_assets` the shim is inlined and the `.wasm` stays a fetch. `'wasm-unsafe-eval'` is now added
  for **any** of the three switches, not just islands, and composes with the origins `[analytics]` adds to the
  same directive. Classes and `data-*` names that only the `islands/` crate writes now keep their spelling
  (they are compiled into the module, where nothing can rewrite them) — previously they were renamed in the
  stylesheet and pruned as unused, which broke only in release builds. And nothing about WebAssembly can fail
  a build any more except your own code: a missing `wasm32-unknown-unknown` target, a missing `wasm-opt` or a
  missing `islands/` crate is a warning naming the fix, and the binary still ships with the behaviour left
  out. `next-rust build` prints the module and shim sizes with a line per island, the JSON build report
  carries them, and the new `/docs/wasm-islands` page documents all of it — including, plainly, that a shim
  always remains, that this is obfuscation and **not** a security boundary, that string literals live in the
  binary's data section in plain text, and that secrets belong on the server.
- **Release builds send smaller pages.** HTML is written without what browsers do not need: quotes around
  simple attribute values, `=""`, default attributes (`method="get"`, `type="text/javascript"`,
  `type="text/css"`) and the end tags HTML makes optional (`</li>`, `</td>`, `</tr>`, `</p>`, `</option>`,
  `</dd>`, `</tbody>`). Text is never touched, not one space, in or out of `<pre>` and `<textarea>`; scripts and
  JSON are copied as written. `data-*` attributes, CSS custom properties (`--primary` → `--b3`), `@keyframes`
  and `@layer` names and the ids the UI components generate (`nr-field-3`, `nr-listbox-0-1`) get short random
  names, the same ones in the HTML, in every stylesheet of the page, in inline `style` attributes, in the
  component script and in the project's `client/`/`assets/` scripts (their string literals and `el.dataset.x`
  accesses are rewritten at build time); `for`, `form`, `list`, `popovertarget`, `aria-controls`,
  `aria-labelledby`, `aria-describedby`, `aria-activedescendant`, `aria-owns`, `headers`, `href="#id"` and
  `url(#id)` in SVG `fill`/`stroke`/`filter`/`mask`/`clip-path` are rewritten with the ids. A name keeps its
  spelling when something the build cannot rewrite knows it: a file under `public/`, a name in a regular
  expression or `dataset["…"]`, a name built at runtime (`"data-" + key`), a selector or script inside a Rust
  string literal, a name the `islands/` crate uses (it becomes WebAssembly), everything under `data-nr-`; the
  build report records what was kept and why. SVG path data
  (`d`, `points`) is rounded, and `console.*` calls are removed from the scripts that are minified. New keys:
  `[build] minify_html`, `minify_names`, `keep_names` (names to leave alone, `data-chart-*` for a family),
  `rename_ids` (ids the build may rename; ids are public, so no author-written id is renamed by default),
  `svg_precision` and `drop_console`, with `NEXT_RUST_MINIFY_HTML`/`NEXT_RUST_MINIFY_NAMES` overriding. This
  makes pages smaller and hard to read; it hides nothing.
- **Seventy more UI components.** `next-rust-ui` grows from an app kit into one a whole site can be built
  with, zero-JS first. Overlays: `Dropdown` (+`DropdownItem`/`DropdownSection`), `Popover`, `ContextMenu` (native
  `popover`, CSS anchor positioning with a scripted fallback, arrow keys and type-ahead), `Drawer`,
  `ConfirmDialog`, `CommandPalette` (⌘K, filter, arrow keys), `Toaster`/`Toast` (server-rendered flashes fade
  out with CSS alone; `nextRust.toast(..)`, `Press::toast(..)`, an `nr:toast` event), `LoadingOverlay`. Forms:
  `SearchInput`, `Combobox` (datalist fallback), `TagsInput`, `NumberInput`, `Slider`, `PinInput` (one real
  input), `Rating`, `SegmentedControl`, `FileUpload`, `TimePicker`, `ColorPicker`, `DateRangePicker`,
  `FormField`/`FormSection`/`FormActions`. Sections and layout: `Hero`, `Section`, `FeatureGrid`,
  `PricingTable`/`PricingCard`, `Testimonial`, `LogoCloud`, `Cta`, `Faq` (with `FAQPage` JSON-LD), `Timeline`,
  `Carousel` (scroll-snap), `Marquee`, `Bento`, `Footer`, `Pagination`, `AspectRatio`, `ScrollArea`, `Spacer`,
  `Show`/`Hide`, `Sticky`. Data: `Sparkline`, `BarChart`, `LineChart`, `Donut` (server-rendered, hover values
  without a script, `--nr-chart-1…8`), `Meter`, `List`, `DescriptionList`, `ChipGroup`, `Code`/`CodeBlock` (copy
  button), `Callout`, `Banner` (CSS-only dismiss, optional persistence), `Figure`, `VideoEmbed` (poster facade,
  no third-party requests before play), `ResultBlock`, `SortHeader`/`TableSelectAll`/`TableSelect` with
  `Table![header = ..]`, and `Avatar![status = ..]`. `Press::open_modal` now also renders `commandfor`/`command`
  so dialogs open without the script where invoker commands exist. Logical properties, focus rings, reduced
  motion and both themes throughout; `examples/ui` has a page per category.
- **Forms never reload the page.** The runtime now sends every same-origin form itself, with no app code: a GET
  form (a search box on Enter, filters, a picker calling `requestSubmit()`, a lone input with no button or
  action) becomes a client navigation to the URL it builds (native serialization including the submitter,
  `formaction`/`formmethod`; history pushed, or replaced for the same URL; layouts kept; always fresh; the focused
  field keeps focus, typed text and caret), and a POST to a `route.rs` is posted with `fetch` (urlencoded or
  multipart with files, `x-csrf-token` included): redirects are followed as navigations, an HTML answer of any
  status becomes the page, an attachment is saved, and JSON/text/204 reach the form like a server action's result
  (`data-nr-state`, `nr:success`/`nr:error`, `data-nr-error`, `stay_on_success`). Sending sets `aria-busy`,
  disables the submitter and ignores re-submits (server-action forms too); `form.submit()` takes the same path.
  New attributes: `live_search(250)` (`data-nr-live`, `data-nr-debounce`: debounced search as you type, previous
  request aborted, history replaced) and `swap_target("#results")` (`data-nr-target`: only that element is
  updated, in place, keeping focus and caret). The browser keeps other origins, `target`, `method="dialog"`,
  `text/plain` and anything marked `reload(true)` (`data-nr-reload`/`data-nr-native`, on the form or a submit
  button). Pages with a form now load the runtime. `[client] enhance_forms = false` turns it off (server actions
  unaffected). Everything still works without JavaScript. `RenderFlags::forms` is new in `next-rust-view`.
- **A page gets only the CSS it can use.** Per-page stylesheets (the Tailwind build with the CSS imported into
  it, `global_css!`, the UI components') were cut by class only: theme variables, `:root`/`.dark`/theme blocks,
  the base reset, element and attribute rules, `@font-face` and every `@keyframes` a theme variable named went
  to every page. Now a rule is sent when the page has what its selector needs (elements, ids, attributes and
  their values: `details`, `input[type=checkbox]`, `[data-theme="noir"]`, `::file-selector-button`); a custom
  property when something sent reads it, followed through other variables, fallbacks, `calc()`/`color-mix()`,
  inline `style` attributes, SVG presentation attributes, inline scripts and the other stylesheets of the page; `@font-face`, `@keyframes` (by `animation`, not `float:left`)
  and `@property` when what is sent names them; blocks left empty are not written, neighbouring blocks with one
  selector are merged, identical rules are sent once. Scripts keep their guarantee and extend it: the build lists
  per script the attributes (`setAttribute`, `dataset.x`, `el.hidden`), ids, custom properties and created
  elements next to its classes, and the runtime's and `ui.js`'s names always count. Client navigations stay
  correct although only a region is rendered: the `<style>` ids now carry what the document had. The Tailwind
  output is tightened (`a, b`, `calc(1 / 2)`, `;}`, repeated declarations). New `[assets] css_keep_vars` and
  `css_static_attributes` (`["data-theme"]`: a boot script names it, the server decides it; unused theme blocks
  go); `css_safelist` also takes attributes and `"<element"`. On a 99-page app the sign-in document's CSS went
  from 57 kB to under half of that.

- **Analytics and site verification from `next-rust.toml`.** `[verification]` (`google`, `bing`, `yandex`,
  `pinterest`, `facebook`, `baidu`, `naver`, `seznam`, `other = { name = content }`) writes real `<meta>` proofs
  into every document in every environment, and answers the file-based ones itself: `google_file`
  (`/google….html`), `bing_file` (`/BingSiteAuth.xml`), `ads_txt`/`app_ads_txt`, and `assetlinks` /
  `apple_app_site_association` under `/.well-known/` (which `public/` never serves). `[analytics]` takes ids for
  `google_analytics` (GA4), `google_tag_manager`, `google_ads`, `microsoft_clarity`, `hotjar`, `meta_pixel`,
  `pinterest_tag`, `linkedin_insight`, `tiktok_pixel`, `plausible`, `umami` and `cloudflare_web_analytics`, and
  the framework writes one minified inline script with the CSP nonce into `<head>` that loads them `async`,
  `preconnect` hints, and the `<noscript>` fallbacks right after `<body>` — exactly once in dynamic, streamed,
  static, partially prerendered, error, 404 and full-document `page.html` responses, never in client-navigation
  responses. No app code. Options: `enabled_in_dev` (analytics are production-only by default), `respect_dnt`
  (Do Not Track / Global Privacy Control: nothing loads, nothing connects), `consent = "required"` (Google
  Consent Mode v2 starts `denied`, cookie-setting tags are not loaded until `nextRust.consent.grant([...])`;
  `revoke`, `state`), `exclude_paths` globs, `skip_noindex`, `page_view_on_navigation` (the `nr:navigate` event
  feeds tags that do not follow the History API) and `anonymize_ip`. `Metadata::new().analytics(false)` opts a
  page or layout out. Moving between a page with analytics and one without is made a full page load (the server
  answers the client navigation with `204` + `x-nr-reload: 1`), so nothing about an excluded page reaches a
  vendor. `[security] csp = "strict"` allows exactly the origins of the configured tags (`script-src`,
  `connect-src`, `img-src`, `frame-src`). Ids are validated at build time: `NR0010` (`[verification]`), `NR0011`
  (`[analytics]`; a `UA-` id gets "Universal Analytics is gone, use a `G-` id"). `DocumentParts::body_start` is
  new in `next-rust-view`.
- **Scripts and styles in Rust, and inline delivery.** `client_script!("chart.js", r#"…"#)`,
  `asset_script!("js/site.js", r#"…"#)` and `stylesheet!(r#"…"#)` keep browser code and CSS inside `.rs` files,
  so a project needs no `.js` or `.css` file. The build script reads the sources out of the Rust files
  (`$OUT_DIR/next_rust_src`) and treats them exactly like `client/`, `assets/` and `.css` files: `[tailwind]
  stylesheets` accepts Rust files (`"src/styles/theme.rs"`), scripts are scanned for classes, rewritten with
  the short class names, minified, fingerprinted by `asset!`, embedded and served at the same URLs, and
  modules import each other by relative path. `#[client(script = "chart.js")]` is short for the module URL.
  New diagnostics: `NR0009` (a Rust stylesheet without `stylesheet!`), `NR0213` (a script declared twice).
  `[build] inline_assets = true` writes every script into the document instead of linking it: the runtime and
  the component script, `<script src>` of `assets/`/`client/` scripts, and the modules of the page's islands
  with what they import. ES modules are turned into factories at build time (static imports become registry
  lookups, exports stay live), registered in `window.__nrm` by inline scripts in `<head>` that carry the CSP
  nonce, and the runtime hydrates islands from that registry. On client navigations the runtime reports the
  inline scripts the document has (`x-nr-scripts`), so they are neither sent nor run again. A module reached
  only through `import("./vendor/big.js")` is never inlined: it is served at `/_next-rust/m/<hash>` with
  immutable caching and fetched on demand, which is where a large vendor bundle belongs. The default (linked
  scripts) is unchanged.
- **Partial prerendering.** `dynamic(fallback, |ctx| async move { … })` marks a per-request hole in an
  otherwise static page, layout or component. Everything outside the holes — the shell, with the fallbacks in
  place — is rendered once (at startup or on the first request) and cached under the static rules
  (`REVALIDATE`, `TAGS`, `revalidate_path`/`revalidate_tag`, stale-while-revalidate, `generate_params`). A
  request gets the shell's bytes at once; its holes then run concurrently with the real request context
  (`ctx.cookies`, `ctx.headers`, `ctx.uri`, the new `ctx.extract::<T>()`) and are streamed into the same
  response through the existing `<template>` + `$nr(id)` swap, continuing the shell's boundary ids and sending
  only the CSS the shell has not. Measured over a socket (`tests/ppr.rs`): a page with a 300 ms hole has its
  first byte after 0.5 ms (a static page: 0.2 ms, the same page without a hole: 302 ms). Any static page that
  renders a hole is served this way; `pub const RENDERING: Rendering = Rendering::Partial;` states it, and the
  build reports `NR0212` when such a page reads the request outside a hole. `next-rust routes`, `build` and
  `analyze` show `◐ partial` routes. The shell is rendered without a request, so it cannot hold visitor data;
  responses with holes are `private, no-store` (`x-nr-prerender: partial`, `x-nr-cache` reports the shell)
  while the shell is cached on the server only; an error or panic in a hole renders the nearest `error.rs` (or
  `Dynamic::new(..).on_error(..)`) without ending the stream; a hole that sets a cookie, header or status
  after the head was sent is logged. Crawlers and `[rendering] streaming = false` get every hole rendered in
  place, so no JavaScript is needed there. Client-side navigations get the region below the shared layout
  and only the holes inside it. New example: `examples/partial-prerendering`.
- **Fragment caching: `cached(key, CacheOptions, || async { view })`** caches the rendered HTML of a part of a
  page with its own lifetime and tags, along with the stylesheets, utility classes and client features it
  needs. Inside a hole it keeps the shared part of per-request content out of every request; inside a static
  page or shell, the page inherits the fragment's tags and shortest lifetime, so `revalidate_tag` renders
  the fragment and the pages around it again while their other fragments are reused.
  `invalidate_fragment(key)` drops one entry.

- **`next_rust::jobs`: background jobs and scheduled tasks** (new crate `next-rust-jobs`; the Redis backend
  is behind the `jobs-redis` feature). `#[job]` turns an `async fn` with one serialisable input into a job:
  `enqueue(job, input)`, `enqueue_in(duration, ..)`, `enqueue_at(time, ..)`, retries with exponential
  backoff and jitter, a maximum number of attempts, a per-attempt timeout, a dead-letter list
  (`dead_jobs`/`retry_dead`/`purge_dead`), unique jobs and idempotency keys, priorities and a concurrency
  limit per queue. `#[cron("0 */5 * * * *")]` and `JobRunner::schedule(name, expr, || async { .. })` run
  scheduled tasks (5- or 6-field expressions, names, `@daily`, `@every 5m`; UTC) under a per-tick lock, so
  one instance of a deployment runs each tick. Backends implement one trait: in memory (default) and Redis
  — atomic Lua claims with a visibility timeout, heartbeats, a reaper for jobs whose worker died, server-side
  time. `AppBuilder::jobs(runner)` starts the workers when the server starts serving and, on shutdown, stops
  claiming and lets in-flight jobs finish within a deadline; `on_start`/`on_success`/`on_failure` hooks,
  counters and `status_handler` (a read-only JSON endpoint) cover observability. The old `Scheduler` and
  `Queue` remain. See "Background jobs" in the docs.
- `AppBuilder::on_serve` and `AppBuilder::on_shutdown`: async hooks that run when the server starts
  listening and after its connections have drained.
- **`next_rust::auth`: an auth and rate-limit kit** (new crate `next-rust-auth`, behind the `auth` feature;
  `auth-redis` adds the Redis backends). Database-agnostic building blocks distilled from a production
  sign-in system: `AuthKeys` (typed sealed tokens: AES-256-GCM, an HKDF key per `Purpose`, expiry, previous
  keys for secret rotation), `SecureCookies` (`__Host-` prefixed `HttpOnly` cookies, sealed cookies for
  pending steps), `Sessions` over your `SessionStore` (sealed access token + rotating refresh token, only
  its SHA-256 stored; reuse detection that ends the family; a 15-second grace window in process and
  optionally in Redis; verification against the store on every request; `require`/`load` middleware and the
  `CurrentSession<U>` extractor, which also feeds `Auth<U>`), `Passwords` and `PasswordPolicy` (argon2id on
  the blocking pool, dummy verification, rehash detection), `RateLimiter` (fixed and sliding windows, in
  memory or atomic Lua in Redis, `/24` and `/64` network keys, hashed identifier keys, fail-open or
  fail-closed, a middleware adapter next to `rate_limit`) and `Codes` (one-time codes hashed with user and
  purpose, attempt counting, decoys). See "Auth & rate limits" in the docs, including a complete
  sqlx/Postgres `SessionStore`.
- **Security.** Fixes from an audit of the request pipeline:
  - *Open redirect through the trailing-slash redirect:* `GET //evil.example/`
    answered `308 Location: //evil.example`, which browsers read as another
    host. Leading slashes are now collapsed.
  - *Open redirect through `_redirect` and the referrer:* browsers strip tabs
    and newlines from URLs, so `_redirect=/%09/evil.example` passed the
    local-path check and landed on `//evil.example`. Targets with a
    backslash, whitespace or control character are refused, and
    `Response::redirect*` removes tabs from the location. The check is public
    as `next_rust::is_local_path` for `?next=` parameters of your own.
  - *Script URLs as redirect targets:* a `redirect(..)` to a `javascript:`,
    `vbscript:` or `data:` URL was executed by the client runtime (action
    responses) and by the streamed-redirect script. Those become `/`.
  - *Panic from `If-Modified-Since`:* an out-of-range year or hour overflowed
    the date arithmetic and panicked the request (the whole process with
    `[build] panic = "abort"`). Fields are range-checked, and a panic in any
    handler now answers 500 and is logged instead of dropping the connection.
  - `protected(..)` writes only a local path into `?next=` and encodes `&`,
    `=`, `+` and `#`, so a request can't add a second `next` parameter.
  - `rate_limit` counts IPv6 clients by /64 network; a full bucket table is
    pruned at most once a second and then shares one overflow bucket, instead
    of scanning 100,000 entries on every request.
  - `Request::is_secure` trusts `X-Forwarded-Proto` only with
    `[server] trust_proxy`; same-origin checks use `:authority` when an
    HTTP/2 request has no `Host` header.
  - The flash cookie always fits the 4 KB cookie limit and stays valid JSON
    (values are dropped first); `secret`, `cvv`, `otp`, `pwd` and
    card-number fields are never echoed back.
  - Embedded JSON (`__next_rust_env`, streamed redirects) writes `<` as
    `\u003c`; `__Host-`/`__Secure-` cookies always get the attributes their
    prefix requires; `MemorySessionStore` sweeps expired sessions.
  A second audit covered what was added since (inline scripts, partial
  prerendering, jobs, auth) and ran generated hostile input against the
  parsers a request reaches:
  - *Spoofed client address behind a proxy:* with `[server] trust_proxy`,
    `client_ip()` (and so `rate_limit` and the auth limiter) used the first
    `X-Forwarded-For` entry. Proxies append to that header, so a client that
    sent its own got any address it liked and a fresh rate-limit bucket per
    request. The address is now counted from the right: the entry your proxy
    added, or with the new `[server] trusted_proxies = n` the n-th from the
    right (2 for a CDN in front of a load balancer).
  - *Page-cache poisoning through intercepting routes:* with `RENDERING =
    Static` or `[rendering] default = "static"`, an intercepting route and
    the page at the same URL shared one cache entry, so a request with
    `x-nr-nav`/`x-nr-from` decided which of the two every later visitor got.
    Intercepting routes are rendered per request.
  - *Connections that never speak:* a client that connected and sent nothing
    was held open for good (the header timeout starts with the first byte,
    and HTTP/2 detection has none). The first byte now has to arrive within
    `[server] header_timeout`. HTTP/2 limits are stated explicitly: 128
    streams per connection, 64 KB of headers, keep-alive pings that time out.
  - *WebSocket memory:* `ws::upgrade` accepted 64 MiB messages (the library's
    default). The limit is 1 MiB; `ws::upgrade_with_limit` takes another.
  - *Script injection through a file name (`[build] inline_assets`):* the
    registry key of an inlined module was written into JavaScript unquoted,
    so a file named `client/x"];alert(1);r.f["y.js` ran its own code on every
    page carrying that island, and one named `…</script>…` ended the element
    and injected markup. Keys are JSON-encoded and escaped for a `<script>`,
    and the build refuses a name that cannot be written into a page. A
    script declared in Rust can no longer name a path either
    (`client_script!("../../../tmp/evil.js", …)` wrote outside the build
    directory): the new diagnostic `NR0214`.
  - *An island module imported from another origin:* an island's
    `data-module` was handed to `import()` exactly as it was written, so a
    `#[client(module = ..)]` path built from application data — a CMS field,
    a database row, a route parameter — could name
    `https://evil.example/x.js` and the browser ran that code inside the
    page. The runtime resolves the attribute against the page's origin and
    imports it only when it is a path on that origin (one leading `/`, no
    `//`, no backslash, no scheme); anything else is refused with a console
    error and the island keeps its server-rendered markup and declarative
    behaviour. The module registry of `[build] inline_assets` now has
    null-prototype tables as well, so a module key like `constructor` or
    `__proto__` cannot find a member of `Object.prototype` in place of a
    factory.
  - *An inlined script cut open by its own text:* with a streamed body and
    no `</head>` in the piece, the insertion point for module registrations
    was searched for in the whole output, inlined JavaScript included. App
    code containing the string `"</head>"` had a `<script>` written into the
    middle of it, which then ended early. Only markup is searched now.
  - *A page reading the request's CSP nonce:* cached static HTML carries a
    placeholder where the nonce goes, and it was a fixed literal replaced
    anywhere in the document. A static page that rendered that literal as
    ordinary text — a CMS field, a route parameter, a file name — was handed
    the live nonce of every request. The placeholder is random per process.
  - *Open redirect through a configured redirect:* `http::Uri` accepts `\`
    in a path and browsers read it as `/`, so with
    `source = "/go/:to"`, `destination = "/:to"` a request for
    `/go/\evil.example` answered `Location: /\evil.example`, which is
    `//evil.example` — another origin. This was the only place a piece of
    the request path reached a `Location` header.
  - *XML injection into feeds:* an entry date that did not parse was written
    into `<updated>`, `<published>` and `<pubDate>` unescaped, so
    `published("</published><evil/>")` added elements to an Atom or RSS
    feed — dates routinely come from a CMS. Everything is escaped now, C0
    control characters (which XML cannot carry at all) are dropped, and a
    `NaN` sitemap priority is left out instead of written.
  - `<link rel="preload">` was the one URL in metadata that skipped the
    `javascript:`/`data:` check; `is_safe_url` also ignores a leading
    byte-order mark now.
  - *Error detail decided by another app:* the "show details" flag was one
    per process, written by the last `App` built, so in a process that also
    built a development app (tests, an embedding) a production app's API
    routes and server actions answered with internal messages. It follows
    the app that answers the request now.
  - *Caches bounded by entries only:* `MemoryStore` — the default page store
    and data cache — counted entries, not bytes, so 10,000 pages a visitor
    chose could be gigabytes. It bounds both (`MemoryStore::with_max_bytes`,
    256 MiB by default).
  - *A page frozen by one panic:* a panicking background revalidation left
    its path marked as in flight for the life of the process, so that page
    was never rendered again and the stale copy was served for good. The
    marker is released even when the render unwinds, and the panic is
    logged.
  - *Shell-store thrashing:* asking for many one-off URLs of a static route
    with `dynamic_params` filled the shell map and then cleared it whole, so
    every live page rendered its shell again. It now drops its
    least-recently-used paths.
  - *An empty `[env] public_prefix`:* only variables with the prefix reach
    the browser (`NEXT_RUST_PUBLIC_` by default), but the prefix is a
    configuration string, and `public_prefix = ""` — a plausible typo —
    matched every name, so the whole environment, database URLs and API keys
    included, was written into every page. A prefix shorter than four
    characters is refused with a warning and the default is used.
  - *A panic outside the built-in server:* only the hyper server turned a
    panic in application code into a 500. `App::handle`, which every other
    adapter goes through (serverless, tests, an embedding), let it unwind
    into the caller — with `[build] panic = "abort"` that ends the process.
    It is caught there now, so the adapter gets a 500; cookies set before
    the panic are dropped with the half-finished response.
  - *A streamed page with a form in a shared cache:* every HTML response has
    its action markers replaced with a token bound to the visitor's cookie,
    and a complete body is then marked `private, no-store`. A streamed body
    is read after the head is sent, so it kept whatever `cache-control` the
    application had set: a streamed, publicly cacheable page with a form
    handed one visitor's token and binding cookie to a shared cache.
    Streamed HTML now carries the same `private, no-cache, no-store,
    max-age=0, must-revalidate` as every other response that must not be
    kept.
  - *Inline scripts and shared caches:* a client navigation with `[build]
    inline_assets` leaves out the scripts the browser has, and was cacheable
    with only `Vary` to tell it apart, which several CDNs ignore. Those
    responses are `private, no-store`. Module registrations of a streamed
    body are written next to their island instead of at the start of the
    chunk, which could be inside an attribute value or an entity.
  - A non-ASCII attribute name (`attr("é…", ..)`) panicked the renderer;
    `%+f` was read as a percent escape; a NUL byte (`%00`) never reaches a
    route parameter; a file in `public/` whose real name starts with `.` is
    not served through a link or alias either.
  - `next_rust::jobs`: inputs over 1 MiB are refused
    (`JobClient::max_payload_bytes`); job, queue and unique-key names are
    validated (length, control characters); stored error messages are cut to
    4 KB; a panic in a hook or the log sink no longer kills the worker or
    cron loop; a Redis record that does not parse is dead-lettered at once
    instead of failing every claim; `*/4294967295` and times near `u64::MAX`
    no longer overflow the cron parser; `MemoryBackend::max_jobs`.
  - `next_rust::auth`: passwords over 1024 bytes are refused before argon2
    (`Passwords::max_bytes`), hashing runs at most one per CPU at a time
    (`max_concurrent`), a stored hash cannot demand more than 256 MiB or 16
    passes, and an account without a usable hash costs a dummy verification;
    a full in-memory limiter table shares one counter *per quota* (it used
    to share one for all, which reset limits); limiter keys over 256 bytes
    are hashed; `sign_out` ends the whole session family, signing in again
    revokes the session it replaces, and the refresh grace window also needs
    the device cookie; secrets with fewer than 8 distinct bytes are refused
    (`KeyError::Weak`).
  - Property tests with fixed seeds now run with the suite: escaping in
    every context, path resolution, cookies, dates, redirect targets, sealed
    and action tokens, the inline-script rewriter at every chunk boundary,
    CSS/HTML passes, cron expressions, header and body limits over a socket.
  - `#![forbid(unsafe_code)]` in `next-rust-cli` and `next-rust-macros`.
- **Shared app state and an async startup step.** `next_rust::run_with_setup(routes(), |app| async move { .. })`
  runs an async closure once before the server starts, after the `.env`
  files are loaded and on the runtime that serves requests, so database
  pools and cache clients opened there live as long as the process.
  `AppBuilder::state(value)` clones one value into every request; pages,
  layouts, loaders, API routes and server actions take it as `Extension<T>`.
- **Server actions in a page's own module.** A route file can keep its actions
  in a sibling file it declares, `#[path = "function.rs"] pub mod function;`,
  and reference them as `action!(function::save)`. The build follows `#[path]`
  module declarations of every special file (and of those modules in turn,
  cycles stopped) and registers the actions under the route file's module.
  A private module that holds actions is the new error `NR0211`.
- **Select and date-picker popovers fade instead of sliding.** With motion
  on, the blanket transition also animated the `left`/`top`/`width` the
  script sets when it lifts a popover into the top layer, so the list and
  the calendar slid in from their CSS position. They now set their own
  transition: a plain opacity fade on open (`@starting-style`) and on close
  (`display`/`overlay` held with `allow-discrete`), with no scale or shift.
  Reduced motion shows and hides them instantly.
- **A loading bar on every page.** While the next page is on its way, a
  thin bar runs along the top (client navigations, server-action forms, full
  page loads) and disappears once the page is there; it shows only when a
  load takes longer than `delay` ms, so instant navigations never flash it.
  The new `[loader]` section of `next-rust.toml` styles all of it: `color`
  (your brand color or a CSS variable), `height`, `shadow`, `spinner`,
  `delay`, `speed`, `easing`, `z_index`, `at_bottom`; `enabled = false`
  removes it. Scripts can drive it with `nextRust.loader.start()`.
- **The loading bar moves like one.** It slides in from the edge on a
  transform (no layout work), runs fast at first and ever slower without
  resting or reaching the end, fills with `[loader] easing` when the page
  arrives, then fades; with `shadow` it glows, brightest under its leading
  edge. A load that starts while the last bar is still fading gets a fresh
  bar instead of one dragged backwards. Plain (non-action, multipart and GET)
  form submits now show it, a cancelled click or submit does not, and after a
  full page load the new document takes the bar over where the old one left
  it and completes it. Reduced motion gets a bar that fades in place, and a
  slower spinner.
- **Motion by default.** Pages fade into each other (view transitions on
  client navigations and full loads, a plain fade elsewhere, streamed
  content fades in), and every hover, focus, active and class change eases
  instead of jumping: colors, shadows, opacity, transforms, size, spacing
  and type. Built from the new `[animation]` section of `next-rust.toml`
  (`pages`, `hover`, `duration`, `page_duration`, `easing`; `enabled =
  false` turns it all off). The app's own `transition`s and Tailwind
  `transition-*` utilities win over it, `data-nr-motion="off"` excludes a
  subtree, and reduced-motion visitors get none of it.
- **Brotli everywhere.** Pages and API responses are compressed with Brotli
  when the browser accepts it (gzip otherwise). Files in `public/`, `assets/`
  and `client/` are compressed at build time with Brotli (quality 11) and
  gzip (level 9) and served precompressed; the framework scripts are
  compressed once per process. `NEXT_RUST_PRECOMPRESS=0` turns the build
  step off.
- **Classes that style nothing are left out.** A class that looks generated
  (`sm:flexx`, `bg-[…]`, `w-1/3`, `!p-0`) but that no stylesheet, script or
  static file of the project knows is not sent (`[build]
  drop_unused_classes`, on for release). Plain words are always kept.
- **`next-rust build` reports what it did:** how many classes got 1–2 or 3
  characters and which keep their names, the scripts minified and rewritten,
  the files compressed and by how much, and the classes that match no rule.
- **Shorter class names, per-page CSS without leaks.** Release builds now
  give classes names of one or two characters (three beyond about a thousand
  classes; the most used get the shortest): the name alphabet grew, and only
  strings in the source reserve names, not Rust identifiers. Scripts in
  `client/` and `assets/` are rewritten to use the short names wherever a
  string can only be a class list, so those classes are shortened too instead
  of kept; a lone word that may be a CSS value (`"block"`) still keeps its
  name. Classes a script adds are sent only with pages that load that script
  (islands' modules, included `assets/`/`public/` scripts, their imports)
  instead of with every page. A page with a module island no longer receives
  the whole utility layer. The UI components' classes are shortened only when
  the app uses the components.
- **Fourteen more UI components.** `Alert`, `Badge`, `Progress`,
  `CircularProgress`, `Skeleton`, `Stat`, `Kbd`, `EmptyState`, `Tooltip`,
  `Tabs`/`Tab`, `Breadcrumbs`/`BreadcrumbItem`, `Steps`/`Step`,
  `Accordion`/`AccordionItem`, `Modal`/`ModalBody`/`ModalFooter` and `Table`
  join `next_rust::ui`, built like the others (a builder, a same-named
  macro, every property optional, `nr-*` classes in the `components` layer,
  per-page CSS). Accordions are `<details>`, modals are `<dialog>`, so both
  work without a script; tab panels, `Press::open_modal`/`Press::close_modal`
  and closable alerts are driven by the component script, which grew to
  about 5 KB gzipped.
- **Requests from islands go through the runtime.** `nextRust.request(url,
  { method, body, headers, signal, timeout })` sends same-origin JSON
  requests with the page's cookies and CSRF token (`as: "blob"`, `"text"` or
  `"response"` for other bodies) and throws an error with `.status` and
  `.data` on failure; `nextRust.stream(url, options)` reads a
  server-sent event response as an async iterator of `{ event, data, id }`.
  Both refuse other origins, so credentials never leave the site.
- **Same-origin checks for your own routes.** `req.same_origin()` tells an
  API route or WebSocket handler whether the browser sent the request from
  another site (`Sec-Fetch-Site`, then `Origin` against the host, honouring
  `[security] allowed_origins`); `ws::upgrade` now refuses cross-site
  upgrades with 403. The `Config` is available to routes as a request
  extension (`req.extension::<Arc<Config>>()`).
- **`[security] csp = "strict"`.** A ready-made Content Security Policy
  (`next_rust::STRICT_CSP`): scripts only with the per-request nonce plus
  `'strict-dynamic'`, everything else from this origin, no plugins, no
  framing by other sites.
- **MSRV is Rust 1.89** (the JavaScript minifier's dependencies need it).

- **Breaking: the framework's URLs and cookies are named after it.** What
  was served under `/_nr/` (`runtime.js`, `ui.js`, `client/`, `assets/`,
  `image`, `action/`, `dev/`) lives under `/_next-rust/`, and the cookies are
  `next_rust_csrf`, `next_rust_flash`, `next_rust_session` and
  `next_rust_bind` (`__Host-next_rust_bind` over HTTPS). A
  `#[client(module = "/_nr/client/x.js")]` or a script that fetches
  `/_nr/…` needs the new prefix; `data-nr-*` attributes, `x-nr-*` headers and
  `nr:` events are unchanged.
- **Build signature.** Every document starts its `<head>` with
  `<meta name="generator" content="Next Rust 0.1.x">`, and the framework's
  scripts carry a `/*! Next Rust 0.1.x */` banner. `[build] signature = false`
  turns both off; `next_rust::VERSION` and `next_rust::signature()` give the
  values.
- **Scripts are minified.** Release builds compress the JavaScript in
  `client/` and `assets/` and mangle its local names (module top-level names
  included) with the oxc minifier before embedding it, so the browser gets
  only what runs and the source stays on the build machine. `[build]
  minify_js = false` or `NEXT_RUST_MINIFY_JS=0` turns it off; a file the
  minifier cannot handle is embedded as written with a build warning.
  `asset!` URLs hash the minified file, so they stay immutable. The
  `minify-js` feature of `next-rust-build` (on by default) carries the
  minifier. It pins `bumpalo` to 3.19; an application whose lockfile already
  holds a newer one needs `cargo update -p bumpalo --precise 3.19.0` once.
- **Every class rule is sent per page.** The per-page CSS split that sent
  Tailwind utilities only where used now applies to every rule whose
  selectors start with a class, wherever it is in the stylesheet: hand-written
  CSS imported through `[tailwind] stylesheets`, `@layer components`, and
  `global_css!` sheets (now per page too). A page that renders one of ten
  classes receives one rule. Rules without a class owner (`:root`, elements,
  attribute selectors, `@font-face`) are always sent, once; `[tailwind]
  keep_classes` and `[assets] css_safelist` keep rules for classes that
  scripts add.
- **Per-action body limit**: `#[server_action(body_limit = <bytes>)]` lets one
  action accept a larger request body than `[server] body_limit`, for uploads
  sent as base64, so the server-wide limit can stay small. The value must be a
  constant; it applies after the action's origin and token checks, and every
  other action and route keeps the server's limit. Any other argument is a
  compile error naming the one that is supported.
- **Context-only server actions**: `#[server_action] async fn f(ctx: ActionContext)`
  now receives the request context. A lone `ActionContext` used to be read
  as the action's input and failed to compile; `(ActionContext, ())` was the
  workaround. Like an action without arguments, a submitted form's fields are
  only echoed back into the flash.
- **`ClientIp` extractor**: the client's address in a page, layout, `load` or
  `metadata`, read by the same rule as `Request::client_ip()` (the first
  `X-Forwarded-For` entry only with `[server] trust_proxy`), for code that calls
  a backend on the visitor's behalf and must pass their address on. Like every
  request extractor, it makes a page dynamic.
- **SVG elements**: `text!`, `tspan!`, `textPath!`, `foreignObject!`, `mask!`,
  `pattern!`, `marker!`, `filter!` and the filter primitives `feGaussianBlur!`,
  `feOffset!`, `feFlood!`, `feComposite!`, `feBlend!`, `feColorMatrix!`,
  `feMerge!`, `feMergeNode!`, `feDropShadow!`, in SVG's own casing.
  `next_rust::TAGS` now lists 139 elements.
- **Fixed:** a plain (no-JavaScript) form post rejected with a validation or
  other error was sent to its `_redirect` target, the page meant for
  success, instead of back to the form where the flash is shown. Errors now
  go back to the referring page, and to `_redirect` only when the browser
  sent no referrer. Successful posts are unchanged.
- **Fixed:** `next-rust dev` could overwrite a new special file (`page.rs`, …)
  with starter code while an editor or tool was still writing it: files are
  created empty and written a moment later, and a poll in between saw an empty
  file. New files are now filled only after staying empty for 0.8 s, and
  emptiness is checked again right before writing.
- **`Metadata::theme_color_for(media, color)`** writes a `theme-color` per media
  query (one for light, one for dark), and **`Metadata::icon_sized(href,
  sizes, mime)`** an icon with its size and type.
- **`Request::client_ip()` and `ActionContext::client_ip()`**: the client's
  address, from the first `X-Forwarded-For` entry when `[server] trust_proxy` is
  on and the peer address otherwise, so server actions can rate-limit per
  client. The rate-limit middleware now uses the same rule; a forwarded value
  that is not an IP address falls back to the peer instead of becoming a
  client of its own.
- **Form results in place**: `stay_on_success(true)` on an action form keeps
  the page after a successful submit instead of refreshing or following
  `_redirect` (which then only applies without JavaScript); `action_result(..)`
  marks elements that show what the action returned, as text, and
  `reset_on_success(true)` clears the fields. Forms now carry
  `data-nr-state="success"|"error"` after every enhanced submit, so success and
  error panels need only CSS.
- **`Metadata::html_attribute(name, value)`**: attributes on the `<html>`
  element (`data-theme`, `class="dark"`, a per-page `lang`), rendered by the
  server so a theme applies before any script runs and without JavaScript.
  Children override their parents per name; values are escaped, invalid names
  and `on*` handlers are dropped, and `class` is shortened and styled like any
  other class. Client-side navigations keep the attributes already on the page.
- **`[tailwind] stylesheets`**: CSS files compiled together with the app's
  classes, in order, right after Tailwind itself. Each is a full Tailwind input
  (`@theme`, `@layer`, `@utility`, `@apply` with any variant, `@keyframes`), so a
  design system can live in real CSS files instead of a TOML string. The files
  are watched by `next-rust dev` and rebuild on change; a missing one stops the
  build with the new diagnostic `NR0008` (also reported by `next-rust doctor`).
- **Icons** (`next_rust::icons`, the new `next-rust-icons` crate): all 1,848
  [Lucide](https://lucide.dev) icons plus their 264 former names, as
  `icons::ArrowRight()` and friends. Size, color, stroke width (also absolute),
  fill, classes, accessible title and any attribute can be set; unused icons are
  left out of the binary. `cargo run -p next-rust-icons --example generate`
  updates them from a newer Lucide release. The UI components now use these
  icons.
- **UI components** (`next_rust::ui`, the new `next-rust-ui` crate): Button,
  ButtonGroup, Input, PasswordInput (show/hide), Textarea, Select (custom
  listbox: keyboard, type-ahead, multiple), DatePicker (custom calendar with
  month/year views, min/max, locale), Checkbox, Switch, RadioGroup, Avatar,
  AvatarGroup, Card, Chip, Spinner, Divider, and layout (Container, Stack, Grid,
  Navbar, Sidebar, AppShell). Each is a Rust builder with a macro of the same
  name (`Button![color = Color::Danger, "Delete"]`); every property is optional
  and any class you add wins over the defaults (they live in the CSS
  `components` layer). Themed with CSS variables, dark mode included. Fields
  show server-action validation errors under the right input automatically.
  `on_press` calls a server action (`on_press = action!(save)`), navigates or
  emits an event. The browser behavior is a 4.6 KB (gzipped) script,
  `/_next-rust/ui.js`, loaded only on pages that need it; without it every component
  falls back to the native control.
- The per-page CSS splitter also handles `@layer components` and selector lists.
- Release builds give the UI components' classes short names too (`nr-card` →
  `cn`), together with Tailwind's, and rename them in the component stylesheet
  and script to match. This also happens without Tailwind.
- Cached static pages send only the component CSS the browser lacks on client
  navigations, like Tailwind's.
- **Per-page CSS.** Pages no longer get the whole app's Tailwind stylesheet:
  each one gets only the utility rules for the classes it renders, plus theme
  variables and base styles. `@property` registrations, their fallbacks and
  `@keyframes` are sent only once a rule refers to them. Client-side
  navigations send only the rules the browser lacks, for cached static pages
  too, and streamed content brings its own rules. Rules that could override
  each other are re-sent when needed so the cascade order always matches the
  full stylesheet. Classes used by scripts in `public/`/`client/`,
  `keep_classes` and `safelist` are always included.
- Tailwind CSS is minified in development too.
- **Unused code is reported.** Files in `app/` were compiled with
  `dead_code`, `unused_imports` and all of clippy silenced, so unused
  functions, imports and helpers there never produced a warning. Only the
  `non_snake_case` allowance for `Page`/`Layout` remains. `pub const RENDERING`
  is now type-checked instead of reported as unused.
- `next-rust dev` shows compiler warnings on every rebuild (it used to discard
  them), and `next-rust build` shows file paths relative to the project instead
  of `app/app/page.rs`.
- New projects get a strict `[lints.rust]` table: everything in `unused`, plus
  `unused_qualifications`, `unused_lifetimes`, `unused_import_braces`,
  `unused_macro_rules` and `unused_extern_crates`. Change `"warn"` to `"deny"`
  to fail the build instead.
- **Security: server actions no longer have fixed URLs.** `/_next-rust/action/<hash>`
  was an unkeyed hash of the file path and function name, so anyone could
  compute it and call the action directly. Every page view now gets its own
  URL: a token signed with HMAC-SHA256 under `NEXT_RUST_SECRET`, expiring
  after `[security] action_token_ttl` (12 hours), and bound to the visitor's
  `__Host-next_rust_bind` cookie (`HttpOnly`, `SameSite=Strict`). A URL copied into
  another browser, curl or a cross-site form is rejected before the action
  runs. Set `NEXT_RUST_SECRET` (32+ bytes) in production; a shorter one is a
  startup error.
- **Breaking:** `next_rust::action_url` is removed. In tests use
  `TestClient::action_url(id)`, or take the URL from a page the client loaded.
  `ActionRef::url()` now returns a placeholder that only works inside rendered
  HTML.
- Pages containing action URLs are sent with `Cache-Control: private,
  no-store`, including statically rendered pages (their HTML is still cached
  on the server).
- **Fixed:** `[security] csrf = "token"` was documented as requiring a
  double-submit token for server actions but did not check it. It now does,
  and the client runtime sends `x-csrf-token` automatically.
- **Fixed:** server actions parsed `text/plain` bodies as JSON, which let a
  cross-site HTML form post JSON. Only `application/json` and
  `application/x-www-form-urlencoded` are accepted now (`415` otherwise).
- Server-action responses carry `Cache-Control: no-store`, and action URLs are
  left out of request logs.

- **Feeds.** `Feed` and `FeedEntry` render a syndication feed as RSS 2.0, Atom
  1.0 or JSON Feed 1.1, handling the date format and escaping each one wants.
  Serve them from a route: `Response::xml(feed().to_rss())`.
- `Metadata::feed(title, href, type)` and `Metadata::link(rel, href)` add the
  `<link>` tags feed readers and browsers look for.
- `Response::xml` and `Response::svg`, for feeds, sitemaps and images generated
  on the fly.
- `SitemapEntry` and `RobotsRule` are in the prelude, so `app/sitemap.rs` no
  longer needs an import to set `lastmod` or a crawl rule.
- **Fixed:** a `metadata` function taking `Data<T>` generated code that referred
  to data nobody had loaded. It now gets the same `load` call a page does.
- **Fixed:** a form posting to a server action did not pull in the client
  runtime, so it submitted with a full page load unless the page happened to
  contain a link.
- `Params::with` accepts `&String`.
- **CI.** Checks run as separate jobs with `--locked`: format, Clippy and docs, tests on Linux, macOS and
  Windows, the Redis backends of `next-rust-jobs` and `next-rust-auth` against a `redis:7` service, every
  feature on its own (`cargo hack --each-feature`), the minimum Rust version read from `rust-version`,
  `cargo package` of every crate, and `cargo deny` (licenses, sources; advisories run daily in their own
  workflow, configured in `deny.toml`). The release list lives in `.github/scripts/release-crates.sh` and now
  includes `next-rust-auth` and `next-rust-jobs`; releases attach prebuilt `next-rust` binaries (Linux gnu
  and musl, macOS, Windows) with checksums and build provenance. Actions are pinned to commit SHAs and
  Dependabot keeps them and `Cargo.lock` current.

## 0.0.1

Initial release.

- `next-rust build` writes one self-contained, stripped binary
  (`.next-rust/<name>`) with `next-rust.toml`, `public/`, `assets/` and
  `client/` embedded; static pages render into memory at startup.
- Plain internal links (`a![href("/about")]`) navigate without a page refresh.
- The documentation is a website built with Next Rust (`website/`).

- Filesystem routing: pages, `page.html`, layouts, templates, route groups,
  dynamic/catch-all/optional catch-all segments, parallel slots, intercepting
  routes, formal route ranking, 35 build diagnostics.
- Code generation from `build.rs` with signature analysis and typed extractors.
- SSR, streaming SSR, SSG, ISR with path/tag revalidation, automatic
  static/dynamic inference.
- Loading, error, not-found and global-error boundaries; redirects; metadata
  with templates; generated sitemap and robots.
- API routes, middleware at every level, cookies, sessions, CORS, rate
  limiting, CSRF, SSE, WebSockets (feature).
- Server actions with progressive enhancement and validation.
- View DSL with escaping by default, CSS modules and global CSS compiled at
  build time, fingerprinted assets, `Image!`, `LocalFont`.
- Client runtime (~4 KB gzipped): client navigation, prefetch, islands,
  form enhancement.
- CLI: new, dev, build, start, check, routes, analyze, generate, doctor,
  docker, upgrade, clean.
- `next-rust dev` fills newly created, empty special files with starter code
  (`[dev] scaffold`).
- VS Code snippets and settings (`next-rust editor`, included in new
  projects).
- Production prints only errors: request logs, the startup line and warnings
  are development-only unless enabled with `[logging]`.
- Production builds minify what browsers receive: `page.html` files are
  minified at build time, the client runtime is served minified and
  name-mangled, and new projects build stripped release binaries.
- `next-rust new` depends on the GitHub repository; `next-rust upgrade`
  updates projects and the CLI, with a daily update notice.
