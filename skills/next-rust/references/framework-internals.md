# Working on the framework itself

Read this before changing anything under `crates/`. Verified against Next Rust 0.1.7 (Rust 2024 edition,
MSRV 1.88).

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

Workspace members: `crates/*`, `examples/*`, `benchmarks`, `website`. All ten published crates share one
version through `version.workspace = true`.

| Crate | Responsibility |
| --- | --- |
| `next-rust` | Facade: the prelude, `app!`/`routes!`, feature forwarding. What applications depend on. |
| `next-rust-server` | HTTP runtime on hyper 1: request pipeline, SSR and streaming, partial rendering, ISR, middleware, static files, server actions, the embedded client runtime. |
| `next-rust-view` | The HTML DSL: elements, attributes, escaping, metadata, streaming renderer, short class names. |
| `next-rust-router` | Filesystem routing: scan, segment parsing, validation, ranking, the trie matcher. |
| `next-rust-cache` | Cache abstraction with in-memory and filesystem stores, tags, revalidation. |
| `next-rust-core` | Config, `.env` loading, environment/rendering modes, diagnostics, brand strings. |
| `next-rust-assets` | Content hashing, MIME detection, CSS minification, CSS-module scoping, CSS pruning and class renaming. |
| `next-rust-macros` | `#[client]`, `#[server]`, `#[server_action]`, `css_module!`, `global_css!`, `asset!`, `action!`. |
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

Publish order (which is also dependency order): assets, core, router, view, cache, server, macros, build,
next-rust, cli.

## Where to change what

| Symptom | Start here |
| --- | --- |
| Wrong URL, wrong route precedence, segment parsing | `crates/next-rust-router/src/{scan,segment,rank,matcher}.rs`, tests in `tests/routing.rs` |
| Missing or wrong diagnostic, wrong generated code | `crates/next-rust-build/src/{analyze,codegen}.rs`, tests in `tests/codegen.rs` |
| A request answered wrongly | `dispatch` in `crates/next-rust-server/src/app.rs` |
| HTML output, escaping, streaming | `crates/next-rust-view/src/{render,node,attrs}.rs` |
| Partial navigation, layout reuse | `crates/next-rust-server/src/render.rs`, tests in `tests/partial.rs` |
| Static files, embedded assets | `crates/next-rust-server/src/{static_files,embed}.rs` |
| CSS minify/scope/prune/rename | `crates/next-rust-assets/src/css.rs`, `crates/next-rust-build/src/{css_usage,class_names}.rs` |
| Tailwind engine, download, input CSS | `crates/next-rust-build/src/tailwind.rs`, `crates/next-rust-cli/src/tailwind.rs` |
| A config key | `crates/next-rust-core/src/config.rs` **and** `website/src/content/configuration.rs` |
| CLI output, progress UI, starter template | `crates/next-rust-cli/src/{ui,project,starter,templates}.rs`, `commands/` |

## Build-time codegen

`next_rust_build::generate()` (or `Generator::new()…run()`) writes into `$OUT_DIR`:
`next_rust_routes.rs`, `next_rust_manifest.json`, and with Tailwind `next_rust_tailwind.css`,
`next_rust_tailwind.id`, `next_rust_class_names.rs`, plus the CSS-usage list. Everything goes through
`write_if_changed`, so don't rewrite output needlessly — it forces recompiles.

Special files are included with absolute `#[path]` module attributes, which is why directories named `[slug]`
and `(group)` work at all. Output must be **deterministic**: sorted directory entries, `BTreeMap` rather than
`HashMap`. A test asserts byte-identical output across runs.

Release-only behaviour is env-driven so it can be toggled in tests: `NEXT_RUST_EMBED`, `NEXT_RUST_MINIFY`,
`NEXT_RUST_PRUNE_CSS`, `NEXT_RUST_MINIFY_CLASSES`, `NEXT_RUST_CLASS_SEED` (see `cli-and-config.md`).

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
Windows works), plus an MSRV job with Rust 1.88:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo test --workspace --all-features --no-fail-fast
cargo +1.88 check --workspace --all-features    # the separate MSRV job
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
