# Diagnostics reference

Build-time problems are reported with a stable code, the files involved, an
explanation and a suggested fix. Errors fail `next-rust build`, `check` and
`cargo build` (from `build.rs`). Warnings are printed and don't block the
build.

## Configuration (NR00xx)

| code | level | meaning |
|---|---|---|
| NR0001 | error | the configured app directory does not exist |
| NR0002 | error | the app path is not a directory |
| NR0003 | error | `[api] directory` does not exist |
| NR0004 | error | `[api] prefix` does not start with `/` |
| NR0005 | error | invalid `[app] base_path` (needs a leading and no trailing slash) |
| NR0006 | error | a redirect `source` does not start with `/` |
| NR0007 | warning | unknown `[logging] level` |

Parse errors (bad TOML/JSON, unknown keys) report the file, line and key.

## Routing (NR01xx)

| code | level | meaning |
|---|---|---|
| NR0101 | error / warning | `page.rs` and `page.html` in one directory (a warning when `html_precedence` picks one) |
| NR0102 | error | two routes resolve to the same URL |
| NR0103 | error | different parameter names at the same position |
| NR0104 | error | a page and an API route resolve to the same URL |
| NR0105 | error | catch-all segment is not the last segment |
| NR0106 | error | optional catch-all overlaps a route at its parent path |
| NR0107 | error | duplicate parameter name within one route |
| NR0108 | error | invalid directory name (`user[id]`, `[a-b]`, `()`, `@a-b`, …) |
| NR0109 | error | a directory could not be read (permissions) |
| NR0110 | warning | symbolic link loop skipped |
| NR0111 | error | root-only file (`global-error.rs`, `sitemap.rs`, `robots.rs`) in a subdirectory |
| NR0112 | warning | non-route special file inside `[api] directory` |
| NR0113 | warning | non-UTF-8 file or directory name skipped |
| NR0114 | warning | broken symbolic link |
| NR0115 | error | intercepting route reaches above the root, or is nested |
| NR0116 | error | `[...x]` and `[[...x]]` at the same position |
| NR0117 | warning | slot directory with no `page.rs`/`default.rs` |
| NR0118 | warning | no routes found |
| NR0119 | warning | `route.rs` inside an intercepting route (ignored) |
| NR0120 | warning | file name differs from a special file only by case (`Page.rs`) |
| NR0121 | warning | a Next.js-style file (`page.tsx`, `layout.js`, …) — use `.rs` |

## Source analysis (NR02xx)

| code | level | meaning |
|---|---|---|
| NR0200 | error | syntax error in a special file (with `file:line:column`) or unreadable file |
| NR0201 | error | a special file lacks its required `pub fn` (e.g. `Page`, `Layout`) |
| NR0203 | error | `route.rs` exports no HTTP method handler |
| NR0204 | error | `Loading`, `ErrorBoundary` or `GlobalError` is `async` |
| NR0205 | error | invalid signature for an API handler or middleware |
| NR0206 | error | an argument that cannot be supplied there (`Children` in a page, `Data<T>` without `load`, …) |
| NR0210 | error | a server action with more than two arguments |

## Compile errors in generated code

The generated route file (`$OUT_DIR/next_rust_routes.rs`) calls your
functions directly, so some mistakes show up as normal Rust type errors that
point into it:

- `the trait bound X: FromContext is not satisfied`: a page argument whose
  type isn't an extractor. Check the argument list in [routing](routing.md#pages).
- `the trait bound X: View is not satisfied`: a page returns something that
  can't be rendered.
- `cannot find value __NR_ACTION_ID_…`: `action!(path)` points at a function
  without `#[server_action]`.

The generated file is plain, readable Rust. Open it to see exactly how your
function is called.
