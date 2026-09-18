# Changelog

All notable changes to this project are documented here. The project follows
[Semantic Versioning](https://semver.org/); until 1.0, minor versions may
contain breaking changes.

## Unreleased

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
  `/_nr/ui.js`, loaded only on pages that need it; without it every component
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
- **Security: server actions no longer have fixed URLs.** `/_nr/action/<hash>`
  was an unkeyed hash of the file path and function name, so anyone could
  compute it and call the action directly. Every page view now gets its own
  URL: a token signed with HMAC-SHA256 under `NEXT_RUST_SECRET`, expiring
  after `[security] action_token_ttl` (12 hours), and bound to the visitor's
  `__Host-nr_bind` cookie (`HttpOnly`, `SameSite=Strict`). A URL copied into
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
