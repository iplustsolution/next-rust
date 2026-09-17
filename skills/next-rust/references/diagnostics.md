# Diagnostics: every NR code

Next Rust reports problems at build time with a code, a location and a `help:` line. Read the help line first
— it names the fix. This table is for when you need the bigger picture or the message is truncated.

Codes are grouped: `NR00xx` configuration, `NR01xx` routing and the app directory, `NR02xx` special files,
exports and signatures.

## Configuration (NR0001–NR0007)

| Code | Problem | Usual fix |
| --- | --- | --- |
| `NR0001` | The app directory in `[app] directory` doesn't exist | Create `app/`, or point the key at the right directory |
| `NR0002` | The app directory path is not a directory | A file is in the way |
| `NR0003` | `[api] directory` doesn't exist | Create it or remove the key |
| `NR0004` | `[api] prefix` doesn't start with `/` | `prefix = "/api"` |
| `NR0005` | Invalid `[app] base_path` | No leading slash, no trailing slash |
| `NR0006` | A `[[redirects]] source` doesn't start with `/` | `source = "/old"` |
| `NR0007` | Unknown `[logging] level` (warning, falls back to `info`) | One of `error`, `warn`, `info`, `debug`, `trace` |

A config key that doesn't exist at all is a plain parse error naming the key, because every section is
`deny_unknown_fields`.

## Routing and the app directory (NR0101–NR0121)

| Code | Problem | Usual fix |
| --- | --- | --- |
| `NR0101` | `page.rs` and `page.html` define the same route | Delete one, or set `[app] html_precedence = "rs"` / `"html"` |
| `NR0102` | Duplicate route: two files resolve to the same URL | Usually two groups containing the same path, e.g. `(a)/about` and `(b)/about` |
| `NR0103` | Conflicting dynamic segment names at the same position | Use the same parameter name in sibling routes (`[id]` everywhere, not `[id]` and `[slug]`) |
| `NR0104` | A page and a `route.rs` resolve to the same URL | Move the API route under a different path or the API prefix |
| `NR0105` | A catch-all is not the last segment | `[...slug]` must be the deepest directory |
| `NR0106` | An optional catch-all conflicts with a page at its parent path | `[[...slug]]` already matches the parent URL — remove the parent `page.rs` or use `[...slug]` |
| `NR0107` | Duplicate parameter name in one route | Two `[id]` directories on the same path |
| `NR0108` | Invalid segment name | Parameter and slot names must be identifiers: `[user_id]`, not `[user-id]`; `[]`, `[...]`, `[[slug]]`, `(.)` alone are invalid |
| `NR0109` | A directory could not be read | Permissions |
| `NR0110` | A symlink points back to a parent directory | Would loop; remove it |
| `NR0111` | A file is only valid in the app root | `sitemap.rs`, `robots.rs`, `global-error.rs` |
| `NR0112` | A file is ignored inside the API directory | Only `route.rs` and `middleware.rs` are used there |
| `NR0113` | A non-UTF-8 path was skipped | Rename the file |
| `NR0114` | A broken symlink was ignored | — |
| `NR0115` | Nested intercepting routes | Only one level of interception is supported |
| `NR0116` | `[...x]` and `[[...x]]` as siblings | Keep one |
| `NR0117` | A slot `@name` has no `page.rs` or `default.rs` | Add one, otherwise the slot can never render |
| `NR0118` | No routes found | The app directory has no `page.rs`/`route.rs` anywhere |
| `NR0119` | Interception only applies during client navigation (informational) | A direct page load renders the normal route |
| `NR0120` | A filename looks like a special file but the case is wrong | `Page.rs` → `page.rs`, `Layout.rs` → `layout.rs`, `not_found.rs` → `not-found.rs` |
| `NR0121` | Not a Next Rust file | A stray file that looks special but isn't recognized; helper modules are fine and simply ignored |

## Special files, exports and signatures (NR0200–NR0210)

| Code | Problem | Usual fix |
| --- | --- | --- |
| `NR0200` | The file could not be parsed as Rust | Fix the syntax error at the reported line |
| `NR0201` | The file doesn't export the required function | `page.rs` → `pub fn Page`, `layout.rs` → `pub fn Layout`, `error.rs` → `pub fn ErrorBoundary`, `global-error.rs` → `pub fn GlobalError`, `not-found.rs` → `pub fn NotFound`, `metadata.rs` → `pub fn metadata`, `sitemap.rs`/`robots.rs` → `pub fn sitemap`/`robots`. Check `pub` too |
| `NR0203` | A `route.rs` exports no HTTP method | Export at least one of `GET HEAD POST PUT PATCH DELETE OPTIONS` (uppercase) |
| `NR0204` | An export that must be synchronous is `async` | `Loading`, `ErrorBoundary` and `GlobalError` render instantly and cannot await |
| `NR0205` | Invalid signature | API handlers take nothing or exactly one `Request`; `middleware` takes exactly `(Request, Next)` |
| `NR0206` | An argument isn't allowed in this function | `Children` only in `Layout`/`Template`, `Slots` only in `Layout`, `ErrorInfo` only in the error boundaries, `Data<T>` only where the same file exports `load` |
| `NR0210` | A server action has too many arguments | At most two: `(ActionContext, input)` |

## Errors without a code

Some failures are plain compiler or runtime errors:

- **`load` returning a bare `T`** — the generated code applies `?`, so it must return `Result<T>`.
- **A `load` in a different module than the export** — never called; move it into the same file.
- **`Data<T>` type mismatch** — `Page(Data(x): Data<A>)` must match what `load` returns.
- **A new file under `app/` not routed** — the build script didn't rerun; build again (`next-rust dev` does
  this automatically).
- **`x-nr-cache: BYPASS` where you expected a static page** — the route is dynamic; run
  `next-rust routes --layouts` to see which extractor or layout forced it.
- **Styles missing only in production** — a class name that exists only outside the view tree was renamed or
  pruned; add it to `[tailwind] keep_classes` or `[assets] css_safelist`.
- **"No space left on device" during a build** — `target/` filled the disk; clean
  `target/debug/incremental` first.
