# Changelog

All notable changes to this project are documented here. The project follows
[Semantic Versioning](https://semver.org/); until 1.0, minor versions may
contain breaking changes.

## 0.1.0 — unreleased

Initial release.

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
- `next-rust new` depends on the GitHub repository; `next-rust upgrade`
  updates projects and the CLI, with a daily update notice.
