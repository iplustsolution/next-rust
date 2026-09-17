# Changelog

All notable changes to this project are documented here. The project follows
[Semantic Versioning](https://semver.org/); until 1.0, minor versions may
contain breaking changes.

## Unreleased

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
