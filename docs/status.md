# Status & roadmap

Version **0.1.0**. The public API may change in minor releases until 1.0.

Legend: ✅ implemented and covered by tests · 🟡 implemented with documented
limitations · ❌ not implemented yet

## Routing

| feature | status | notes |
|---|---|---|
| `page.rs`, nested `layout.rs`, `template.rs` | ✅ | unlimited nesting |
| `page.html` (fragments and full documents) | ✅ | conflict with `page.rs` is an error or configurable precedence |
| dynamic, catch-all, optional catch-all segments | ✅ | typed `Path<T>` |
| route groups, private folders | ✅ | |
| formal route ranking | ✅ | property-tested against a brute-force definition |
| configurable/monorepo app directory, separate API directory | ✅ | |
| conflict and naming diagnostics | ✅ | 35 diagnostic codes |
| symlink loops, Unicode, spaces | ✅ | |
| parallel routes (`@slot`, `default.rs`) | ✅ | |
| intercepting routes | 🟡 | page-level; not combined with slots; soft navigation only |
| programmatic routes (`App::get/route/ws`) | ✅ | |

## Rendering & data

| feature | status | notes |
|---|---|---|
| SSR with typed extractors, `load` + `Data<T>` | ✅ | |
| automatic static/dynamic inference | ✅ | reasons shown by the CLI |
| SSG + `generate_params` + `DYNAMIC_PARAMS` | ✅ | |
| ISR (`REVALIDATE`, stale-while-revalidate, `revalidate_path/tag`) | ✅ | |
| streaming SSR (`loading.rs`, `suspense`) | ✅ | out-of-order streaming, CSP nonces |
| `error.rs`, `global-error.rs`, `not-found.rs`, redirects | ✅ | |
| metadata merging, title templates, OG/Twitter | ✅ | |
| `sitemap.rs`, `robots.rs` | ✅ | |
| generated Open Graph images | ❌ | use static files |
| parallel static generation (`[build] concurrency`) | ❌ | pages render sequentially |

## Client

| feature | status | notes |
|---|---|---|
| zero-JS pages by default | ✅ | runtime loaded only when needed |
| `Link!` client navigation, prefetch, history API | ✅ | markup tested; browser behaviour not covered by automated browser tests |
| `#[client]` islands with declarative behaviour | 🟡 | CSP-safe; limited operation set |
| module islands (`hydrate(element, props)`) | ✅ | |
| WASM client components | 🟡 | protocol supports wasm-bindgen output; the build pipeline (`cargo build --target wasm32` + `wasm-bindgen`) is manual |
| Rust reactive DOM library / fine-grained signals in WASM | ❌ | |
| `#[server]` boundary | ✅ | compiled out on `wasm32` |

## Backend

| feature | status | notes |
|---|---|---|
| `route.rs` API routes, IntoResponse, JSON/text/bytes/streams | ✅ | |
| middleware (global, root, nested) | ✅ | |
| cookies, sessions, CORS, rate limiting, request ids, CSRF | ✅ | rate limiting and memory sessions are per process |
| server actions + progressive forms + validation | ✅ | multipart unsupported |
| SSE | ✅ | |
| WebSockets | ✅ | feature `websocket`; covered by compilation, not by an automated test |
| background/scheduled jobs, in-process queue | 🟡 | interval and daily UTC schedules; no cron expressions or persistent queue |
| HTTP/1.1, HTTP/2 (h2c), keep-alive, gzip, graceful shutdown | ✅ | |
| native TLS | ❌ | terminate at a proxy (see deployment) |
| HTTP/3 | ❌ | terminate at a proxy/CDN |
| brotli compression | ❌ | gzip only |

## Assets

| feature | status | notes |
|---|---|---|
| `public/` with ETag, ranges, traversal protection | ✅ | |
| `global_css!`, `css_module!` (scoping + minification at compile time) | ✅ | |
| fingerprinted `asset!` with immutable caching | ✅ | |
| `Image!` markup (srcset, lazy, priority, dimensions) | ✅ | |
| image resizing, WebP/AVIF conversion | ❌ | endpoint serves originals |
| `LocalFont` (`@font-face`, preload) | ✅ | |
| JS minification / bundling | ❌ | the framework ships one hand-written runtime; no user JS bundler |
| source maps | ❌ | not applicable without a bundler |

## Tooling

| feature | status | notes |
|---|---|---|
| `new`, `dev`, `build`, `start`, `check`, `routes`, `analyze`, `generate`, `doctor`, `docker`, `upgrade`, `clean` | ✅ | |
| update notice + `next-rust upgrade` from GitHub | ✅ | notifies once a day; never installs without the user running `upgrade` |
| dev: rebuild on change, keep serving on errors, browser overlay, live reload | ✅ | restarts the server process; no in-process hot-patching of Rust code |
| CSS-only hot swap without reload | ❌ | CSS is compiled into the binary, so changes rebuild and reload |
| plugin traits (runtime + build) | 🟡 | minimal hook set, API may change |
| serverless adapter surface (`App::handle`) | 🟡 | no packaged adapters for specific platforms |
| edge (wasm32) runtime | ❌ | |
| Redis cache store | ❌ | implement `CacheStore` (example in docs) |

## Publishing

- The crate names `next-rust`, `next-rust-core`, `next-rust-router`,
  `next-rust-view`, `next-rust-server`, `next-rust-macros`,
  `next-rust-build`, `next-rust-cache`, `next-rust-assets` and
  `next-rust-cli` were **not taken on crates.io** when checked on 2026-09-17.
  Check again right before publishing.
- `repository`/`homepage` point to `github.com/iplustsolution/next-rust`.
- Until the crates are published, `next-rust new` creates projects that
  depend on the GitHub repository. Switch the template to crates.io versions
  after the first release.
- Publish order (dependencies first): assets → core → router → view → cache →
  server → macros → build → next-rust → cli.

## Roadmap (proposed)

1. `next-rust build --wasm`: compile and bind WASM island crates automatically.
2. Image optimizer behind an `image` feature (resize, WebP; AVIF optional).
3. Native TLS (rustls) behind a `tls` feature; HTTP/3 evaluation (quinn/h3).
4. Parallel static generation.
5. Browser-level end-to-end tests for client navigation and islands.
6. Redis `CacheStore`/`SessionStore` in a companion crate.
