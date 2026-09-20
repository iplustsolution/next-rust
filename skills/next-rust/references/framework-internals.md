# Working on the framework itself

Read this before changing anything under `crates/`. Verified against Next Rust 0.1.7 (Rust 2024 edition,
MSRV 1.89).

## Contents

- [Crate map](#crate-map)
- [Dependency rules](#dependency-rules)
- [Where to change what](#where-to-change-what)
- [Build-time codegen](#build-time-codegen)
- [The browser runtime is duplicated on purpose](#the-browser-runtime-is-duplicated-on-purpose)
- [Documentation lives in Rust](#documentation-lives-in-rust)
- [Tests](#tests)
- [Checks before you push](#checks-before-you-push)
- [Release automation owns the version](#release-automation-owns-the-version)
- [Traps](#traps)

## Crate map

Workspace members: `crates/*`, `examples/*`, `benchmarks`, `website`. All twelve published crates share one
version through `version.workspace = true`. A new publishable crate must also be added, in dependency order, to
both crate lists in `.github/workflows/release.yml`; the release stops at "Pick the version" otherwise.

| Crate | Responsibility |
| --- | --- |
| `next-rust` | Facade: the prelude, `app!`/`routes!`, feature forwarding. What applications depend on. |
| `next-rust-server` | HTTP runtime on hyper 1: request pipeline, SSR and streaming, partial rendering, ISR, middleware, static files, server actions, the embedded client runtime. |
| `next-rust-view` | The HTML DSL: elements, attributes, escaping, metadata, streaming renderer, short class names. |
| `next-rust-ui` | Ready-made components (buttons, fields, select, date picker, toggles, avatars, cards, layout), their stylesheet and browser script, all as Rust. |
| `next-rust-icons` | Every Lucide icon, generated into `src/generated.rs` by `examples/generate.rs`. |
| `next-rust-auth` | Optional auth kit (`next_rust::auth`, feature `auth`): sealed tokens, `__Host-` cookies, rotating sessions over a `SessionStore` trait, argon2id passwords, one-time codes, in-memory/Redis limiters. Depends only on `next-rust-server`. Redis tests run when `REDIS_URL` is set. |
| `next-rust-router` | Filesystem routing: scan, segment parsing, validation, ranking, the trie matcher. |
| `next-rust-cache` | Cache abstraction with in-memory and filesystem stores, tags, revalidation. |
| `next-rust-jobs` | Background jobs (`next_rust::jobs`): queues with retries and dead letters, cron with a per-tick lock, in-memory and Redis (`redis` feature) backends. Depends on no other framework crate; the `#[job]`/`#[cron]` macros live in `next-rust-macros`, the `AppBuilder::jobs` glue in `next-rust`. |
| `next-rust-core` | Config, `.env` loading, environment/rendering modes, diagnostics, brand strings. |
| `next-rust-assets` | Content hashing, MIME detection, CSS minification, CSS-module scoping, CSS pruning and class renaming. |
| `next-rust-macros` | `#[client]`, `#[server]`, `#[server_action]`, `css_module!`, `global_css!`, `asset!`, `action!`, `client_script!`, `asset_script!`, `stylesheet!`. |
| `next-rust-build` | Everything `build.rs` does: analysis, diagnostics, codegen, Tailwind compilation, class shortening. |
| `next-rust-cli` | The `next-rust` binary. |

## Dependency rules

These are architectural constraints, not preferences — breaking one changes what ends up in a user's binary:

- **`next-rust-build` and `next-rust-cli` must never reach an application binary.** `next-rust-build` pulls in
  `syn`; it is a build dependency only.
- **`next-rust-view` depends only on `futures-util`.** It has to render without a server and compile for
  `wasm32`.
- **`next-rust-router` stays free of async and of I/O beyond directory scanning.** The build script, the CLI
  and the runtime all share it.
- **`next-rust-assets` has zero dependencies.** Proc macros (compile time) and the server (runtime) both use
  it.
- **The CLI depends on nothing beyond the framework crates and serde**: `std::process` plus polling — no
  `clap`, no file-watching crate, no async runtime.
- New external dependencies need a row in the dependency table on the Architecture documentation page, and
  default features off wherever possible.

Publish order (which is also dependency order): assets, core, router, view, cache, server, macros, jobs,
build, next-rust, cli.

## Where to change what

| Symptom | Start here |
| --- | --- |
| Wrong URL, wrong route precedence, segment parsing | `crates/next-rust-router/src/{scan,segment,rank,matcher}.rs`, tests in `tests/routing.rs` |
| Missing or wrong diagnostic, wrong generated code | `crates/next-rust-build/src/{analyze,codegen}.rs`, tests in `tests/codegen.rs` |
| A request answered wrongly | `dispatch` in `crates/next-rust-server/src/app.rs` |
| Job retries, claims, cron ticks, shutdown | `crates/next-rust-jobs/src/{runner,cron,backoff}.rs`, `backend/{memory,redis}.rs`; one scenario suite in `tests/common/mod.rs` runs against both backends (`REDIS_URL=… cargo test -p next-rust-jobs --features redis`) |
| HTML output, escaping, streaming | `crates/next-rust-view/src/{render,node,attrs}.rs` |
| Partial navigation, layout reuse | `crates/next-rust-server/src/render.rs`, tests in `tests/partial.rs` |
| Partial prerendering (`dynamic` holes, shells, `cached`) | `crates/next-rust-view/src/render.rs` (`render_shell`, `resume_document`, `Node::Hole`/`Node::Rendered`), `crates/next-rust-server/src/{partial,render}.rs`, tests in `tests/ppr.rs` |
| Static files, embedded assets | `crates/next-rust-server/src/{static_files,embed}.rs` |
| CSS minify/scope/prune/rename | `crates/next-rust-assets/src/css.rs`, `crates/next-rust-build/src/{css_usage,class_names}.rs` |
| Short `data-*`/custom property/keyframes/layer/id names, compact HTML (`[build] minify_names`, `minify_html`) | planning in `crates/next-rust-build/src/names.rs`, shared rules in `crates/next-rust-assets/src/names.rs`, response rewriting in `crates/next-rust-server/src/markup.rs`, tests in `crates/next-rust-server/tests/markup.rs` |
| Scripts/styles declared in Rust (`client_script!`, `stylesheet!`) | `crates/next-rust-build/src/rust_sources.rs` (extraction to `$OUT_DIR/next_rust_src`), `Config::generated` |
| Inline delivery (`[build] inline_assets`) | `crates/next-rust-build/src/js_inline.rs` (modules → factories, lazy modules), `crates/next-rust-server/src/inline.rs` (rewrites the finished response; `/_next-rust/m/<hash>`), `loadModule`/`inlined` in the runtime |
| Tailwind engine, download, input CSS | `crates/next-rust-build/src/tailwind.rs`, `crates/next-rust-cli/src/tailwind.rs` |
| A config key | `crates/next-rust-core/src/config.rs` **and** `website/src/content/configuration.rs` |
| CLI output, progress UI, starter template | `crates/next-rust-cli/src/{ui,project,starter,templates}.rs`, `commands/` |
| The wasm pipeline: what is built, and every way it can fail quietly | `crates/next-rust-build/src/{wasm_islands,wasm_shim,wasm_size}.rs` (`NR03xx`) |
| Turning wasm on, and checking it (`new --wasm`, `generate islands`, `doctor`) | `crates/next-rust-cli/src/wasm.rs` |

## Build-time codegen

`next_rust_build::generate()` (or `Generator::new()…run()`) writes into `$OUT_DIR`:
`next_rust_routes.rs`, `next_rust_manifest.json`, and with Tailwind `next_rust_tailwind.css`,
`next_rust_tailwind.id`, `next_rust_class_names.rs`, plus the CSS-usage list. Everything goes through
`write_if_changed`, so don't rewrite output needlessly — it forces recompiles.

Special files are included with absolute `#[path]` module attributes, which is why directories named `[slug]`
and `(group)` work at all. Output must be **deterministic**: sorted directory entries, `BTreeMap` rather than
`HashMap`. A test asserts byte-identical output across runs.

Release-only behaviour is env-driven so it can be toggled in tests: `NEXT_RUST_EMBED`, `NEXT_RUST_MINIFY`,
`NEXT_RUST_PRUNE_CSS`, `NEXT_RUST_MINIFY_CLASSES`, `NEXT_RUST_CLASS_SEED`, `NEXT_RUST_MINIFY_HTML`,
`NEXT_RUST_MINIFY_NAMES` (see `cli-and-config.md`).

## The browser runtime is duplicated on purpose

`crates/next-rust-server/src/client_runtime.rs` holds:

- `RUNTIME_JS` — readable, served in development. **Edit this one.**
- `RUNTIME_JS_MIN` — minified, served in production, must stay a single line.
- `RUNTIME_JS_SOURCE_HASH` — a hash of `RUNTIME_JS`.

The test `minified_runtime_matches_source` fails whenever you edit the readable copy without regenerating the
minified one. The procedure (from `CONTRIBUTING.md`), run in a scratch directory outside the repository so no
`node_modules` lands in the tree:

```sh
npx terser runtime.js --module --compress passes=3 --mangle toplevel=true -o runtime.min.js
```

Paste the result into `RUNTIME_JS_MIN` and update `RUNTIME_JS_SOURCE_HASH` to the hash the failing test
prints. A second test, `minified_runtime_keeps_public_contract`, checks the minified copy still contains
`window.nextRust`, `nr-island`, `data-nr-reload`, `pointerover`, `x-nr-nav`, `x-nr-from`, `x-nr-action`,
`hydrate`, `nr-t`, `nr-b`, is under three quarters the source length and has no newline.

If Node is unavailable, a careful hand-edit of the minified copy is acceptable as long as both tests pass —
but say so in the change description.

### And it exists a third time, in Rust

`[client] runtime = "wasm"` serves the same behaviour from `crates/next-rust-runtime`, a `no_std` crate
compiled to `wasm32-unknown-unknown`, behind `crates/next-rust-server/src/wasm_runtime/shim.js`. Both the module
(`runtime.wasm`) and the minified shim are generated and checked in, by `crates/next-rust-runtime/regenerate.sh`,
so an application never needs the wasm target. `RUNTIME_SHIM_SOURCE_HASH` guards the shim the way
`RUNTIME_JS_SOURCE_HASH` guards the runtime.

Two things will bite you here:

- **The opcode numbers in `next-rust-runtime/src/host.rs` are the ABI.** Append; never renumber or reuse. The
  shim indexes a table with the number the module passes, so a mismatch is silent wrong behaviour.
- **The shim must decide nothing.** `the_shim_carries_no_framework_behaviour` fails if it contains `data-nr-*`,
  `x-nr-*`, `__next_rust_env`, `eval` and friends. Framework rules live in Rust; the shim calls browser APIs.

`wasm-opt` needs `--enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int
--enable-sign-ext --enable-mutable-globals`, or it rejects the module and writes nothing — which looks like
success. `wasm_islands.rs::opt_args` passes all of them, and `optimize` warns (`NR0307`/`NR0308`) on every
path where a `wasm-opt` that exists did not produce something smaller.

**There are two modules today, not one**, whatever `wasm_islands.rs`'s module doc says about "one module for
the whole site". `[client] runtime = "wasm"` is served from `/_next-rust/runtime.wasm` — the prebuilt module
`next-rust-server` carries — and needs nothing from the application: no `islands/` crate, no wasm target.
Only `[build] wasm_islands` and `[ui] script = "wasm"` share `client/__nrw/islands.wasm`. A page with both
fetches the shared machinery twice (~68 kB brotli); `internal.rs::wasm_module_url` documents this as a known
duplication. `wasm_islands.rs` still *watches* `next-rust-runtime` so editing it rebuilds, but no longer
counts it as a contributor — counting it made the key demand a crate it had no use for.

### Diagnostics for the wasm pipeline: `NR03xx`

`wasm_islands.rs` owns a family of its own, listed in its `DIAGNOSTICS` table (a test asserts every code is
spelled out exactly once, as a `const`). They come from two passes, and both are `Diagnostic` values so the
build script and `next-rust doctor` cannot disagree:

- `wasm_islands::preflight(&config)` — source only: the `#[client(wasm = "…")]` attributes in `app/`/`src/`,
  the islands crate's manifest, and its `islands!` invocation. No toolchain needed, so it runs *before* the
  nested cargo build and is what `doctor` shows.
- `wasm_islands::unwired(..)` — the built module's export section, the only thing that can confirm what
  shipped. The overlap with `preflight` is deduplicated by `(code, title)` at the call site.

If you scan Rust source in this repo, use `wasm_islands::blank_noncode`: it blanks the *contents* of comments
and string literals while keeping every byte offset, so structure is checked on the blanked copy and values
read back out of the original. Without it the framework's own website — whose documentation pages are Rust
files full of `r#"#[client(wasm = "AuthSwitch")]"#` — warned on every build about four islands that do not
exist.

`next-rust doctor`'s wasm section and `next-rust generate islands` live in `crates/next-rust-cli/src/wasm.rs`.
The scaffolded manifest's empty `[workspace]` table and `crate-type = ["cdylib"]` are both load-bearing and
both fail silently; `set_key` edits `next-rust.toml` as text on purpose, so the comments in it survive.

Size, so nobody is surprised: the JavaScript runtime is 21,188 B minified / 6,808 B brotli; the wasm one is
202,922 / 68,353 plus a 9,709 / 3,262 shim — about ten times over the wire. `"js"` is the default for that
reason. Three things dominated and are already done, so do not redo them: `no_std` (with `std` the empty
skeleton was 13 KB brotli), avoiding `str::parse::<f64>()` (`core::num::dec2flt` is ~10 KB of tables), and a
size-tuned cargo profile set through `CARGO_PROFILE_RELEASE_*` in `regenerate.sh`, because a profile in a
crate's own manifest is ignored — cargo reads them from the workspace root only. That last one alone was
405 KB → 203 KB, and it is worth checking whether the islands build has the same problem.

## Documentation lives in Rust

The documentation site is `website/`, a workspace member built with the framework itself:

- Pages are Rust modules in `website/src/content/*.rs` returning a `Node`. There are no Markdown files.
- A **new** page also needs an entry in `website/src/docs.rs`.
- Feature status goes in `website/src/content/status.rs`.
- Design tokens and Tailwind theme live in `website/build.rs`; shared UI in `website/src/ui.rs`.
- Preview with `next-rust dev` inside `website/`.

Because the website depends on `../crates/next-rust` by path, a breaking framework change breaks CI there.
That is intentional: it keeps the framework honest. It also means the website build needs the Tailwind engine,
so CI (and any sandbox) needs network access on a cold cache, or `NEXT_RUST_TAILWIND_BIN`.

Anything user-visible you add or change should be reflected in `website/src/content/`, the README, and
`CHANGELOG.md`.

## Tests

Both styles are used deliberately:

- `#[cfg(test)] mod tests` next to the code for unit logic (config, CSS, hashing, class names, cookies, the
  client runtime contract, CLI templates, …).
- `tests/` directories for behaviour across a crate: `next-rust-router/tests/routing.rs` (builds real
  directory trees on disk), `next-rust-build/tests/codegen.rs`, `next-rust-server/tests/{pipeline,partial}.rs`,
  `next-rust-view/tests/view.rs`.
- Each example is its own test crate: `examples/<name>/tests/<name>.rs`. `cargo test -p example-dashboard` is
  the fastest end-to-end check, and `cargo run -p example-basic` serves on port 3000.
- A scratch project against your checkout:
  `cargo run -p next-rust-cli -- new /tmp/try-it --framework-path "$PWD"`.

Some config and env tests write into the system temp directory keyed by process id, so they are not safe to
run twice concurrently in the same directory.

## Checks before you push

CI runs these in order on Linux, macOS and Windows (Windows is `continue-on-error`, so green does not prove
Windows works), plus an MSRV job with Rust 1.89:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo test --workspace --all-features --no-fail-fast
cargo +1.89 check --workspace --all-features    # the separate MSRV job
```

`rustfmt.toml` is `max_width = 120`, `use_small_heuristics = "Max"` — run `cargo fmt --all` rather than
hand-wrapping. There is no clippy config: the lint bar is exactly `-D warnings`. `cargo doc -D warnings` is
easy to forget locally and will fail CI on a broken intra-doc link.

Crate roots carry `#![forbid(unsafe_code)]` (`next-rust-core` uses `deny`). Keep it that way.

## Release automation owns the version

Every push to `main` that passes CI triggers `release.yml`: it reads the latest version from crates.io, bumps
the patch, writes it into the workspace `Cargo.toml`, publishes all ten crates in dependency order, commits
`release: vX.Y.Z [skip ci]` back to `main`, tags it and creates a GitHub release. Consequences:

- **Never bump versions by hand** for an ordinary change.
- **Pull before pushing** — the release commit means your local `main` is behind. `git pull --rebase`.
- Secrets `CARGO_REGISTRY_TOKEN` and `RELEASE_TOKEN` must exist for it to work.

## Traps

1. **Config is `deny_unknown_fields`.** Documenting a key you didn't add to the struct breaks every project
   that copies it; adding a field means updating `config.rs` and the configuration documentation page.
2. **Diagnostics are a feature.** A new failure mode should ship a `Diagnostic` with a code (`NR00xx` config,
   `NR01xx` routing, `NR02xx` special files and exports) and a `help:` line that names the fix.
3. **The starter logo is duplicated.** `starter::FAVICON_SVG` must byte-match `website/public/logo.svg`; a
   test enforces it (normalizing line endings).
4. **Tailwind is pinned by checksum.** `tailwind.rs` holds the version plus name, exact byte size and SHA-256
   for seven platforms. Bumping Tailwind means updating all three values per platform from the release's
   `sha256sums.txt`.
5. **`next-rust build` overrides the project's `[profile.release]`** through `CARGO_PROFILE_RELEASE_*`
   variables. Debug binary size or panic behaviour through `[build]` in `next-rust.toml`.
6. **Class renaming is random per build.** Anything naming a class outside the view tree needs
   `keep_classes`; use `NEXT_RUST_CLASS_SEED` for reproducible builds.
7. **Disk and network:** a cold Tailwind cache downloads ~110 MB, and `target/` grows fast. If a build fails
   with "No space left on device", clean `target/debug/incremental` before assuming a code problem.
