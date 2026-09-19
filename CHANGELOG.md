# Changelog

All notable changes to this project are documented here. The project follows
[Semantic Versioning](https://semver.org/); until 1.0, minor versions may
contain breaking changes.

## Unreleased

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
