# Diagnostics: every NR code

Next Rust reports problems at build time with a code, a location and a `help:` line. Read the help line first
— it names the fix. This table is for when you need the bigger picture or the message is truncated.

Codes are grouped: `NR00xx` configuration, `NR01xx` routing and the app directory, `NR02xx` special files,
exports and signatures, `NR03xx` WebAssembly.

`NR03xx` is unlike the rest: **every one of them is a warning on a build that succeeded**, and the symptom
is a page that renders, looks finished and does nothing. Read them, or run `next-rust doctor`.

## Configuration (NR0001–NR0011)

| Code | Problem | Usual fix |
| --- | --- | --- |
| `NR0001` | The app directory in `[app] directory` doesn't exist | Create `app/`, or point the key at the right directory |
| `NR0002` | The app directory path is not a directory | A file is in the way |
| `NR0003` | `[api] directory` doesn't exist | Create it or remove the key |
| `NR0004` | `[api] prefix` doesn't start with `/` | `prefix = "/api"` |
| `NR0005` | Invalid `[app] base_path` | No leading slash, no trailing slash |
| `NR0006` | A `[[redirects]] source` doesn't start with `/` | `source = "/old"` |
| `NR0007` | Unknown `[logging] level` (warning, falls back to `info`) | One of `error`, `warn`, `info`, `debug`, `trace` |
| `NR0008` | A file in `[tailwind] stylesheets` doesn't exist (checked when Tailwind is on; stops the build) | Fix the path — it is relative to `next-rust.toml` — or remove the entry |
| `NR0009` | A Rust file listed in `[tailwind] stylesheets` has no `stylesheet!(r#"…"#);` at module level | Put the CSS in `next_rust::stylesheet!(…)` in that file, or list a `.css` file |
| `NR0010` | A `[verification]` value is malformed: a whole `<meta>` tag instead of its `content`, a `google_file` that isn't `google….html`, a `bing_file` that isn't the bare code, a bad `ads.txt` record, `assetlinks` that isn't JSON | Paste only the value the help line names |
| `NR0011` | An `[analytics]` id has the wrong shape: `UA-…` (Universal Analytics is gone — use the GA4 `G-…` id), a `GTM-`/`AW-` id under the wrong key, non-digits in a pixel id, a Plausible domain with `https://`, a non-UUID Umami id, a script URL that isn't `https://`, an `exclude_paths` pattern without a leading `/` | Fix the id; the help line shows the expected form |

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

## Special files, exports and signatures (NR0200–NR0214)

| Code | Problem | Usual fix |
| --- | --- | --- |
| `NR0200` | The file could not be parsed as Rust | Fix the syntax error at the reported line |
| `NR0201` | The file doesn't export the required function | `page.rs` → `pub fn Page`, `layout.rs` → `pub fn Layout`, `error.rs` → `pub fn ErrorBoundary`, `global-error.rs` → `pub fn GlobalError`, `not-found.rs` → `pub fn NotFound`, `metadata.rs` → `pub fn metadata`, `sitemap.rs`/`robots.rs` → `pub fn sitemap`/`robots`. Check `pub` too |
| `NR0203` | A `route.rs` exports no HTTP method | Export at least one of `GET HEAD POST PUT PATCH DELETE OPTIONS` (uppercase) |
| `NR0204` | An export that must be synchronous is `async` | `Loading`, `ErrorBoundary` and `GlobalError` render instantly and cannot await |
| `NR0205` | Invalid signature | API handlers take nothing or exactly one `Request`; `middleware` takes exactly `(Request, Next)` |
| `NR0206` | An argument isn't allowed in this function | `Children` only in `Layout`/`Template`, `Slots` only in `Layout`, `ErrorInfo` only in the error boundaries, `Data<T>` only where the same file exports `load` |
| `NR0210` | A server action has too many arguments | At most two: `(ActionContext, input)` |
| `NR0211` | A private module declared by a route file holds server actions | Declare it `pub mod name;` so the route table can reach it |
| `NR0212` | `RENDERING = Rendering::Partial`, but the page, a layout/template above it, a slot page, `load` or `metadata` takes a request-bound extractor (or the route intercepts) | Move the read into a hole: `dynamic(fallback, \|ctx\| async move { .. })` and use `ctx.cookies` / `ctx.extract::<T>()?` there; the shell is rendered without a request |
| `NR0213` | Two `client_script!` (or two `asset_script!`) declarations name the same path | Rename one; a file in `client/`/`assets/` with that path would win over both |
| `NR0214` | A `client_script!` / `asset_script!` path that is not a plain relative one (`..`, absolute, quotes, `<`/`>`, control characters) | The path is a file below `client/`/`assets/`, a URL and a key in the page's module registry: use letters, digits, `.`, `-`, `_` and `/` |

## WebAssembly (NR0301–NR0316)

All warnings, all emitted as one-line `cargo:warning=` entries during a build and in full by
`next-rust doctor`. Nothing here fails a build — a missing toolchain component is the machine's problem, not
the code's — so the build ships a binary with the WebAssembly behaviour left out. The only visible symptom
is inert markup: the loader looks each export up by name and returns quietly when it is absent, so there is
no console error and no failed request.

| Code | Problem | Usual fix |
| --- | --- | --- |
| `NR0301` | A switch that needs the `islands/` crate is on and there is none | `next-rust generate islands` |
| `NR0302` | The islands crate has no `[lib] crate-type = ["cdylib"]`, so cargo builds an rlib and there is no module | Add it |
| `NR0303` | `wasm32-unknown-unknown` is not installed | `rustup target add wasm32-unknown-unknown` |
| `NR0304` | A framework wasm crate (`next-rust-runtime`, `next-rust-ui-wasm`) has no source beside this build — the case when `next-rust-build` came from crates.io | Depend on `next-rust` by path from a checkout, or point `NEXT_RUST_WASM_<NAME>` at a prebuilt module |
| `NR0305` | `NEXT_RUST_WASM_*` points at neither a `.wasm` file nor a crate directory | Fix or unset it |
| `NR0306` | The islands crate is a workspace member, so cargo ignores its `[profile.release]` — cargo reads profiles from the workspace root only, and the module comes out about twice the size | Give the crate an empty `[workspace]` table (what the CLI scaffolds) |
| `NR0307` | `wasm-opt` is not on `PATH` | `brew install binaryen` / `cargo install wasm-opt`, or `[build] wasm_opt = false`. Size only |
| `NR0308` | `wasm-opt` ran and produced nothing smaller, or refused the module | Usually a binaryen older than the features rustc emits; its own output is quoted in the warning |
| `NR0310` | `[ui] script = "wasm"` is on and the module exports no `start` | Add `start: next_rust_ui_wasm::boot,` as the first entry of the crate's `islands!`. Without it `<html>` never gets `nr-ui-js` and the stylesheet leaves every enhanced control at `display: none`, 0×0 |
| `NR0311` | `[build] wasm_islands` is on and the module exports no `hydrate_<Island>` at all | List the islands in `islands!`, or set the key to false |
| `NR0312` | A component is written `#[client(wasm = "…")]` while `[build] wasm_islands` is off | Turn the key on. The glue module is never written, so the browser 404s on `/_next-rust/client/__nrw/<Island>.js`. **Reachable with no wasm configuration at all** |
| `NR0313` | `#[client(wasm = "Name")]` has no `hydrate_Name` behind it | A typo, or an island never listed. The warning prints the names that *are* declared. Case-sensitive |
| `NR0314` | The module exports an island no component asks for | Point a component at it, or drop the entry — it is dead weight in every visitor's download |
| `NR0315` | The islands crate invokes no `next_rust_wasm::islands! { … }` | That macro emits the allocator, the panic handler and every export the loader calls |
| `NR0316` | The module is missing `alloc` or `on_event` | Only reachable with a hand-written or prebuilt module; an island then runs with no props, or with no events |

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
- **Components or islands that render correctly and do nothing** — always one of the `NR03xx` warnings
  above, and always in the build output you scrolled past. `next-rust doctor` prints them again on demand.
  The classic is `[ui] script = "wasm"` with no `start: next_rust_ui_wasm::boot` in the crate's `islands!`:
  the loader's `X.start && X.start()` calls nothing, `<html>` never gets `nr-ui-js`, the stylesheet leaves
  every enhanced control at `display: none` — so a browser test's `click()` waits out its budget on a 0×0
  element and reads as flake (`NR0310`).
