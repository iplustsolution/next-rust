//! Browser logic compiled to WebAssembly.
//!
//! **One module for the whole site**, `client/__nrw/islands.wasm`, shared by
//! every island on every page. Three different things can put code in it, and
//! each is a separate switch:
//!
//! | Contributor | Turned on by | Crate |
//! | --- | --- | --- |
//! | the app's islands | `[build] wasm_islands = true` | `<root>/islands` |
//! | the browser runtime | `[client] runtime = "wasm"` | `next-rust-runtime` |
//! | the UI kit's components | `[ui] script = "wasm"` | the UI kit's `cdylib` |
//!
//! `Config::uses_wasm()` — any of the three — decides whether a module is
//! built at all; `[build] wasm_islands` decides only whether the app's own
//! islands are part of it. They do not duplicate an allocator, a callback
//! table or string machinery between them, which one module per crate would.
//! The app's `islands/` crate is the `cdylib` that is actually compiled: the
//! framework's crates reach the module by being linked into it, so what this
//! file does for them is check that they are *there* and put them on cargo's
//! watch list.
//!
//! **Except the browser runtime, today.** `[client] runtime = "wasm"` is
//! served from `/_next-rust/runtime.wasm`, a second module that
//! `next-rust-server` carries pre-built (`next-rust-runtime/regenerate.sh`
//! writes it, and `internal.rs` says in as many words that there are two
//! modules and not one). So that switch needs nothing from the project: no
//! `islands/` crate, no wasm target, no `wasm-opt`. It is still watched here
//! — when the crate is in the checkout, editing it should rebuild — but it
//! is not counted as a contributor, because counting it made the key demand
//! a crate it had no use for.
//!
//! # Nothing here may fail quietly
//!
//! Every way this pipeline can produce a page that looks right and does
//! nothing has a code in the `NR03xx` family, listed in [`DIAGNOSTICS`], and
//! a `help:` line naming the exact edit. They are found in two passes:
//!
//! - [`preflight`] reads the *project*: the attributes in `app/` and `src/`,
//!   the islands crate's manifest and its `islands!` invocation. It needs no
//!   toolchain, so it runs before the nested cargo build and is what
//!   `next-rust doctor` shows.
//! - `unwired` reads the *artifact*: the module's export section, which is
//!   the only thing that can confirm what really shipped.
//!
//! The two overlap deliberately — a crate can declare what the module does
//! not export — and the overlap is deduplicated at the call site rather than
//! removed, because each pass catches cases the other cannot.
//!
//! They are all **warnings**. Nothing about WebAssembly fails a build except
//! a compile error in the author's own Rust; see `unwired` for the
//! argument. The value is the diagnosis, not the refusal.
//!
//! The glue is hand written and tiny: no `wasm-bindgen`. The module exports
//! `alloc(len) -> ptr`, optionally `set_prop(i, ptr, len)`, one
//! `hydrate_<Island>` per island and one shared `on_event(cb_id, detail)`,
//! and imports the DOM operations it needs as `e.h`. The island names are
//! read back out of the module's export section, so the app declares them
//! once — in Rust. Only `hydrate_`-prefixed function exports count, so shared
//! exports like `start` and `on_resume` are not mistaken for islands.
//!
//! # Finding the framework's own crates
//!
//! `islands/` sits next to `next-rust.toml`, so it is simply there. The two
//! framework crates are not: an application that depends on the published
//! `next-rust-build` from crates.io has no source for them at all. They are
//! therefore resolved in this order, and a failure at the end of it is never
//! fatal:
//!
//! 1. **`NEXT_RUST_WASM_<NAME>`** (`NEXT_RUST_WASM_RUNTIME`,
//!    `NEXT_RUST_WASM_UI`, `NEXT_RUST_WASM_ISLANDS`) — an explicit override.
//!    Pointed at a `.wasm` file it is used as is, with nothing compiled;
//!    pointed at a directory (or a `Cargo.toml`) that crate is used.
//!    **Prefer prebuilt bytes**: shipping a module beside the application is
//!    the only thing that works when the framework is a registry dependency.
//! 2. **A sibling crate in the framework checkout** — walking up from this
//!    crate's own `CARGO_MANIFEST_DIR` looking for `crates/<crate>/Cargo.toml`
//!    that declares `crate-type = ["cdylib"]`. This is the path-dependency
//!    case: a git checkout of the framework, or the framework's own workspace.
//! 3. **Neither** — a `cargo:warning=` names the crate and the two ways out,
//!    and that contribution is skipped. The build keeps going and the page
//!    falls back to the JavaScript it would have shipped anyway.
//!
//! # What is reproducible, and what is not
//!
//! Verified by a test: the loader, the per-island shims and the island list
//! are pure functions of their inputs and byte-identical across runs, and
//! every directory read is sorted.
//!
//! Verified by hand, not by a test: two consecutive builds of an unchanged
//! islands crate produce a byte-identical `.wasm`, from a warm target
//! directory and from a cold one, and `wasm-opt -Oz` is itself deterministic.
//! A real wasm build needs a toolchain, a target and (on a cold cache) the
//! network, which is too much for a unit test, so this is not asserted in CI.
//!
//! One caveat about the *file*, as opposed to its contents: everything here
//! goes through `write_if_changed`, but `rust_sources::write` runs first and
//! deletes every file under the generated directory that it did not itself
//! declare — which includes this module and its shims, since they are written
//! afterwards. So the bytes do not change and the modification time does.
//! Nothing downstream watches that timestamp (cargo does not track `$OUT_DIR`,
//! and `include_bytes!` fingerprints content), so it costs nothing today, but
//! do not read the unchanged timestamp as a guarantee.
//!
//! **Not** guaranteed, and deliberately not claimed:
//!
//! - across toolchains. A different rustc, a different `wasm-opt`, a
//!   different set of dependency versions all change the bytes.
//! - across machines *unless* every absolute path that reaches the module is
//!   remapped. [`CARGO_ENCODED_RUSTFLAGS`] is set here with
//!   `--remap-path-prefix` for the crate directory and for `CARGO_HOME`,
//!   which covers panic messages and debug info; anything a crate bakes in
//!   itself (`env!`, `include_str!`, a build script writing a timestamp) is
//!   beyond this file's reach.
//! - when `wasm-opt` is absent on one machine and present on another. That
//!   changes the bytes by design, and the size report says which happened.
//!
//! [`CARGO_ENCODED_RUSTFLAGS`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use next_rust_core::{Config, Diagnostic};

use crate::wasm_shim::LOADER;

use crate::report::{WasmIslandReport, WasmReport};

/// Every way this pipeline can produce a page that looks right and does
/// nothing, with the code it is reported under.
///
/// The codes are a family of their own (`NR03xx`) because none of the
/// existing ones fit: these are not configuration errors, routing errors or
/// export-signature errors, and they are the only diagnostics in the
/// framework that can be true of a build that *succeeded*.
///
/// They are all warnings. Nothing about WebAssembly fails a build except a
/// compile error in the author's own Rust — see `unwired` for why — so the
/// value here is the diagnosis, not the refusal. Each one therefore has to
/// carry the key that is on, what it produced, and the exact line to write:
/// `next-rust doctor` renders them in full, and a build script gets them
/// flattened onto the one line `cargo:warning=` allows.
pub const DIAGNOSTICS: &[(&str, &str)] = &[
    (NO_CRATE, "the islands crate a switch needs is missing"),
    (NOT_CDYLIB, "the islands crate does not build a cdylib"),
    (NO_TARGET, "the wasm32-unknown-unknown target is not installed"),
    (NO_FRAMEWORK_CRATE, "a framework WebAssembly crate has no source beside this build"),
    (BAD_OVERRIDE, "a NEXT_RUST_WASM_* override points at nothing usable"),
    (IGNORED_PROFILE, "the islands crate's own release profile is ignored by cargo"),
    (NO_WASM_OPT, "wasm-opt is not on PATH, so the module ships unoptimized"),
    (WASM_OPT_FAILED, "wasm-opt ran and produced nothing smaller"),
    (NO_START, "`[ui] script = \"wasm\"` is on and the module exports no `start`"),
    (NO_ISLANDS, "`[build] wasm_islands` is on and the module exports no island"),
    (ISLANDS_OFF, "a `#[client(wasm = \"…\")]` island is used while `[build] wasm_islands` is off"),
    (UNKNOWN_ISLAND, "a `#[client(wasm = \"…\")]` island has no `hydrate_` behind it"),
    (UNUSED_ISLAND, "the module exports an island no component asks for"),
    (NO_DECLARATION, "the islands crate declares no islands at all"),
    (MISSING_EXPORT, "the module is missing an export the loader calls"),
];

const NO_CRATE: &str = "NR0301";
const NOT_CDYLIB: &str = "NR0302";
const NO_TARGET: &str = "NR0303";
const NO_FRAMEWORK_CRATE: &str = "NR0304";
const BAD_OVERRIDE: &str = "NR0305";
const IGNORED_PROFILE: &str = "NR0306";
const NO_WASM_OPT: &str = "NR0307";
const WASM_OPT_FAILED: &str = "NR0308";
const NO_START: &str = "NR0310";
const NO_ISLANDS: &str = "NR0311";
const ISLANDS_OFF: &str = "NR0312";
const UNKNOWN_ISLAND: &str = "NR0313";
const UNUSED_ISLAND: &str = "NR0314";
const NO_DECLARATION: &str = "NR0315";
const MISSING_EXPORT: &str = "NR0316";

/// A [`Diagnostic`] squeezed onto the single line `cargo:warning=` gives it,
/// with the code and the `help:` intact — those two are what end an
/// investigation, and a message that loses them is a message that sends the
/// reader back to the source of this file.
fn cargo_line(d: &Diagnostic) -> String {
    let mut out = format!("Next Rust [{}]: {}", d.code, d.title);
    if !d.message.is_empty() {
        out.push(' ');
        out.push_str(&d.message);
    }
    if let Some(help) = &d.help {
        out.push_str(" help: ");
        out.push_str(help);
    }
    flatten(&out)
}

/// Print a diagnostic where a build script will show it.
fn emit(d: &Diagnostic) {
    println!("cargo:warning={}", cargo_line(d));
}

/// A WebAssembly diagnostic, with the three parts every one of them has.
fn diagnose(code: &str, title: impl Into<String>, message: impl Into<String>, help: impl Into<String>) -> Diagnostic {
    Diagnostic::warning(code, title).message(message).help(help)
}

/// Whitespace collapsed onto one line, nothing dropped.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Directory of the module and its glue, below the generated `client/`.
pub(crate) const DIR: &str = "__nrw";

/// The loader, below the generated `client/` and beside [`DIR`]. Written here
/// and left alone by `rust_sources::write`, which sweeps that tree.
pub(crate) const LOADER_FILE: &str = "__nrw.js";

/// Export prefix marking an island's hydrate function.
const PREFIX: &str = "hydrate_";

/// Target directory of the nested cargo build, below `$OUT_DIR`.
const TARGET_DIR: &str = "next_rust_wasm";

/// The browser target every module is built for.
const TARGET: &str = "wasm32-unknown-unknown";

/// Module name of the app's islands crate. Its file name and URL are part of
/// what is already shipping: do not rename it.
const ISLANDS: &str = "islands";

/// The URL the loader fetches the module from, exactly as `wasm_shim.rs`
/// spells it. It is a literal there and a literal here because the loader is
/// a plain string constant; the `debug_assert!` beside the substitution in
/// [`build`] fails loudly if the two ever drift apart.
const MODULE_URL: &str = "/_next-rust/client/__nrw/islands.wasm";

/// Crates whose code the framework contributes to the module, in the order
/// they are tried. They are libraries linked into the app's `cdylib`, never
/// modules of their own — see [`is_library`].
const RUNTIME_CRATES: [&str; 1] = ["next-rust-runtime"];
const UI_CRATES: [&str; 1] = ["next-rust-ui-wasm"];

/// The foundation every module links, whoever else contributes: the host ABI,
/// the allocator, the `islands!` macros. Unlike the two above it is not
/// optional and is not chosen by a config key, so it is never a "contributor"
/// — but it is *source the module is built from*, and cargo has to be told so.
///
/// It was not, and the failure was silent in the worst way: a fix landed in
/// `next-rust-wasm`, `cargo build --release` said `Finished in 0.59s`, and the
/// build shipped a stale module still carrying the bug. Nothing rebuilt
/// because nothing had told cargo this tree was an input; `touch`ing a file in
/// the app's own `islands/` crate was the only way to pick it up. A framework
/// fix that looks built and is not costs a day and shows up in no gate.
const FOUNDATION_CRATES: [&str; 1] = ["next-rust-wasm"];

/// `wasm-opt` is missing at most once per build, and one warning is enough.
static WASM_OPT_WARNED: AtomicBool = AtomicBool::new(false);

/// Environment a build script inherits that would confuse a nested cargo.
const CLEARED_ENV: [&str; 5] =
    ["CARGO_TARGET_DIR", "CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS", "CARGO_BUILD_TARGET", "CARGO_MAKEFLAGS"];

/// Compile the one wasm module the site loads and write it, its loader and
/// one shim per island below `generated/client`.
///
/// Returns a report for the module, or nothing when none was built. Anything
/// that stops it from being built (no islands crate, no wasm target, no
/// source for a framework crate) is a `cargo:warning=` and an empty result,
/// never a build failure. A genuine compile error in the author's own Rust
/// *is* a hard error: that is their bug to fix, and hiding it would be worse.
pub fn build(config: &Config, generated: &Path) -> Result<Vec<WasmReport>, String> {
    // Everything that can be known without a compiler, known before the
    // compiler runs. `NR0312` in particular is reachable with no wasm
    // configuration at all, so it is checked before the early return below:
    // a `#[client(wasm = "…")]` in a project that never turned the key on is
    // an island that renders and does nothing, and used to say so nowhere.
    let diagnostics = preflight(config);
    for d in &diagnostics {
        emit(d);
    }
    if !config.uses_wasm() {
        return Ok(Vec::new());
    }
    // Who contributes code to the module. The app's `islands/` crate is the
    // one that is actually compiled — it is the `cdylib` everything else is
    // linked into — so it is looked for whenever anything *that lives in this
    // module* asked for one. The framework's crates are presence checks: they
    // say whether the code the configuration asked for can reach the module
    // at all, and put the crate on cargo's watch list when it can.
    let unit = islands_crate(config);
    // `[build] wasm_islands` decides only whether the app's own islands count
    // as a contributor; the crate is still the thing that gets compiled.
    let mut contributors = usize::from(config.build.wasm_islands && unit.is_some());
    if config.client.runtime.is_wasm() {
        // Watched, not counted. `[client] runtime = "wasm"` is served from
        // `/_next-rust/runtime.wasm`, a module `next-rust-server` carries,
        // built and checked in by `crates/next-rust-runtime/regenerate.sh` —
        // see `internal.rs`, which says in as many words that there are two
        // modules today and not one. Counting it here made the key demand an
        // `islands/` crate it has no use for: a project that turned on one
        // switch, needed nothing else, and was told at every build to create
        // a crate that would have contributed nothing.
        contributes(
            "runtime",
            "`[client] runtime = \"wasm\"`",
            "the page keeps the JavaScript browser runtime",
            &RUNTIME_CRATES,
        );
    }
    if config.ui.script.is_wasm() {
        contributors += usize::from(contributes(
            "ui",
            "`[ui] script = \"wasm\"`",
            "the components keep their JavaScript behaviour",
            &UI_CRATES,
        ));
    }
    // Nothing to put in a module, and every source has already said why it is
    // absent; one more line would only repeat them.
    if contributors == 0 {
        return Ok(Vec::new());
    }
    // Whatever the contributors are, the module is built from the foundation
    // too. Watch it, or a change there produces a "Finished" that shipped the
    // previous module. Silent, so it gets its own registration rather than
    // riding on whichever optional crate happens to be enabled.
    if let Some(manifest) = sibling_crate(&FOUNDATION_CRATES)
        && let Some(dir) = manifest.parent()
    {
        watch(dir);
    }
    // `islands_crate` named the switches and the path it looked at.
    let Some(unit) = unit else { return Ok(Vec::new()) };

    // `generated` is `$OUT_DIR/next_rust_src`: the nested build keeps its own
    // target directory beside it, out of the outer build's way.
    let target_dir = generated.parent().unwrap_or(generated).join(TARGET_DIR);
    let wasm = match unit {
        Located::Prebuilt(path) => Some(path),
        Located::Crate(manifest) => compile(&manifest, &target_dir)?,
        Located::Missing => None,
    };
    let Some(wasm) = wasm else { return Ok(Vec::new()) };
    let (wasm, optimized) = optimize(config, &wasm, &target_dir);
    let bytes = std::fs::read(&wasm).map_err(|e| format!("cannot read {}: {e}", wasm.display()))?;
    let exports = exports(&bytes)?;
    let islands = island_names(&exports);
    // The module exists. Whether it does what the configuration promised is a
    // different question, and only the built artifact can answer it.
    //
    // `preflight` has already read the same questions out of the crate's
    // source; anything it reported is skipped here rather than said twice,
    // because two lines about one mistake read as two mistakes.
    let already: Vec<(&str, &str)> = diagnostics.iter().map(|d| (d.code.as_str(), d.title.as_str())).collect();
    for d in unwired(config, &exports, &islands, &requested_islands(config)) {
        if !already.contains(&(d.code.as_str(), d.title.as_str())) {
            emit(&d);
        }
    }

    let dir = generated.join("client").join(DIR);
    crate::write_if_changed(&dir.join("islands.wasm"), &bytes).map_err(|e| e.to_string())?;
    for name in &islands {
        let module = format!("import{{h}}from\"../__nrw.js\";export const hydrate=(e,p)=>h(\"{name}\",e,p);");
        crate::write_if_changed(&dir.join(format!("{name}.js")), module.as_bytes()).map_err(|e| e.to_string())?;
    }
    // Stamp the module's content hash into the URL the loader fetches.
    //
    // The loader spells it as a bare literal, and the server answers a bare
    // path under `/_next-rust/client/` with `max-age=3600` — so the module
    // was re-validated every hour and re-downloaded whenever it fell out of
    // the cache, although its bytes are immutable for the life of the build.
    // With `?v=<hash>` the URL names one exact build, which is the case
    // `versioned_cache_control` already answers `immutable`, and a new build
    // changes the URL rather than the meaning of the old one. This is the
    // same treatment `/_next-rust/runtime.wasm` gets from the server side.
    let loader = LOADER.replace(
        &format!("\"{MODULE_URL}\""),
        &format!("\"{MODULE_URL}?v={}\"", &next_rust_assets::content_hash(&bytes)[..10]),
    );
    debug_assert!(loader != LOADER, "the loader no longer spells {MODULE_URL} as a bare literal");
    crate::write_if_changed(&generated.join("client").join(LOADER_FILE), loader.as_bytes())
        .map_err(|e| e.to_string())?;
    let [brotli, gzip] = compressed_sizes(&bytes);
    // What each island costs inside the shared module, from a walk of the
    // module's own call graph. A module this cannot parse means a report that
    // is skipped, not a build that fails: the sizes are information, and
    // losing them is not worth refusing to build over.
    let sizes = crate::wasm_size::report(&bytes).ok();
    let size_of = |name: &str| match sizes.as_ref().and_then(|s| s.islands.iter().find(|i| i.name == name)) {
        Some(i) => (Some(i.exclusive), Some(i.reachable)),
        None => (None, None),
    };
    Ok(vec![WasmReport {
        module: ISLANDS.to_owned(),
        bytes: bytes.len(),
        brotli,
        gzip,
        optimized,
        shim: Some(shim_report()),
        runtime: config.client.runtime.is_wasm().then(runtime_tradeoff),
        islands: islands
            .into_iter()
            .map(|name| {
                let (exclusive, reachable) = size_of(&name);
                WasmIslandReport { name, exclusive, reachable }
            })
            .collect(),
    }])
}

/// Size of the JavaScript browser runtime that a wasm runtime replaces, so
/// the build can print the trade-off instead of letting a project discover it
/// in production.
///
/// These are constants rather than measurements because `next-rust-build` does
/// not — and by the dependency rules must not — depend on `next-rust-server`,
/// where the runtime lives. **Refresh them from `client_runtime::RUNTIME_JS_MIN`
/// whenever that file changes.** They are a comparison, not a contract: a few
/// percent stale is survivable, absent is not.
const RUNTIME_JS_BYTES: usize = 21_188;
const RUNTIME_JS_BROTLI: usize = 6_808;

/// Brotli size of `next-rust-server/src/wasm_runtime/runtime.wasm`, the
/// prebuilt module `[client] runtime = "wasm"` actually serves.
///
/// It is a **separate download** from the islands module, which is the whole
/// reason this is a constant and not a measurement: this file compiles the
/// islands module and has never seen the runtime one. Reporting the islands
/// module's size here — which it did — understated the switch's cost by more
/// than half, because the two modules are fetched independently and each
/// carries its own copy of the shared machinery.
///
/// **Refresh from `crates/next-rust-server/src/wasm_runtime/runtime.wasm`
/// whenever `regenerate.sh` rewrites it.** Measured at 203,751 bytes raw.
const RUNTIME_WASM_BROTLI: usize = 68_571;

/// The wasm runtime's cost next to the JavaScript one it replaces.
fn runtime_tradeoff() -> crate::report::RuntimeReport {
    crate::report::RuntimeReport {
        wasm_brotli: RUNTIME_WASM_BROTLI,
        js_bytes: RUNTIME_JS_BYTES,
        js_brotli: RUNTIME_JS_BROTLI,
    }
}

/// The app's own `islands/` crate: the override, then `<root>/islands`.
///
/// This is the crate that is compiled, whichever switch asked for a module,
/// because it is the `cdylib` the framework's contributors are linked into.
/// It reports nothing: everything wrong with the crate — that it is absent,
/// that it is not a `cdylib`, that its profile is ignored — has already been
/// said by [`preflight`], which runs first precisely so those are known
/// before a wasm32 compile rather than after one.
fn islands_crate(config: &Config) -> Option<Located> {
    if let Some(located) = override_path(ISLANDS) {
        return Some(located);
    }
    let crate_dir = config.root.join(ISLANDS);
    let manifest = crate_dir.join("Cargo.toml");
    if !manifest.is_file() {
        return None;
    }
    watch(&crate_dir);
    Some(Located::Crate(manifest))
}

/// The configuration keys that asked for a module, as a reader would name
/// them. Never empty: a caller only reaches this when one of them is on.
fn asked_for_wasm(config: &Config) -> String {
    let keys = [
        (config.build.wasm_islands, "`[build] wasm_islands = true`"),
        (config.client.runtime.is_wasm(), "`[client] runtime = \"wasm\"`"),
        (config.ui.script.is_wasm(), "`[ui] script = \"wasm\"`"),
    ];
    let on: Vec<&str> = keys.iter().filter(|(set, _)| *set).map(|(_, key)| *key).collect();
    match on.as_slice() {
        [] => "the configuration".to_owned(),
        [one] => (*one).to_owned(),
        [most @ .., last] => format!("{} and {last}", most.join(", ")),
    }
}

// ---------------------------------------------------------------------------
// Checks that need no compiler
// ---------------------------------------------------------------------------

/// Everything wrong with a project's WebAssembly setup that can be seen
/// without compiling anything.
///
/// This is deliberately separate from `unwired`, which reads the built
/// module. Two different callers need two different moments:
///
/// - `build.rs` runs this **before** the nested cargo build, so a project
///   that cannot possibly work is told so in seconds rather than after a
///   wasm32 compile.
/// - `next-rust doctor` runs it with no build at all, which is the only way
///   to answer "why is my date picker inert?" on a machine where the last
///   build's warnings have long scrolled away.
///
/// Every check here is textual on purpose. The islands crate's `islands!`
/// invocation is the list of exports, so reading it answers "will there be a
/// `hydrate_Foo`?" without a toolchain, a target or a network — and being
/// occasionally wrong about an exotic spelling costs a warning, while being
/// silent costs the afternoon this whole file exists to give back.
pub fn preflight(config: &Config) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let requested = requested_islands(config);
    let crate_dir = config.root.join(ISLANDS);
    let manifest = crate_dir.join("Cargo.toml");

    // The quietest mistake of the lot, and the only one that needs no wasm
    // configuration at all to reach: the component is written, the attribute
    // is right, and the key that would compile it is off. `#[client]` still
    // points the island at `/_next-rust/client/__nrw/<Name>.js`, the build
    // writes no such file, the dynamic import 404s, and the island keeps its
    // server-rendered markup — looking exactly like the finished feature.
    if !config.build.wasm_islands && !requested.is_empty() {
        out.push(
            diagnose(
                ISLANDS_OFF,
                format!("`{}` is a wasm island, but `[build] wasm_islands` is off", first_key(&requested)),
                format!(
                    "{} in this project use `#[client(wasm = \"…\")]`, which takes its behaviour from the islands \
                     crate. With the key off no module is built and no glue is written, so the browser asks for \
                     /_next-rust/client/__nrw/<Island>.js, gets a 404, and the island renders its server markup and \
                     never reacts to anything.",
                    listing(requested.keys(), "component", "components")
                ),
                "set `wasm_islands = true` under `[build]` in next-rust.toml (`next-rust doctor` checks the rest of \
                 the setup), or give the component a JavaScript island instead: `#[client]` for the declarative \
                 runtime, `#[client(script = \"…\")]` for a module of your own.",
            )
            .location(requested.values().next().cloned().unwrap_or_default()),
        );
    }

    // What actually needs the app's crate. `[client] runtime = "wasm"` does
    // not: it is served from the module `next-rust-server` carries, which is
    // built and checked in by `crates/next-rust-runtime/regenerate.sh`. It
    // used to be listed here, and the result was a project that turned on one
    // key, needed nothing else, and was told to create a crate it had no use
    // for.
    let needs_crate = config.build.wasm_islands || config.ui.script.is_wasm();
    let overridden =
        std::env::var_os(format!("NEXT_RUST_WASM_{}", ISLANDS.to_uppercase())).is_some_and(|value| !value.is_empty());
    if !manifest.is_file() {
        if needs_crate && !overridden {
            out.push(
                diagnose(
                    NO_CRATE,
                    format!("{} needs an islands crate, and there is none", asked_for_wasm(config)),
                    format!(
                        "There is no Cargo.toml at {}. It is the cdylib everything else links into, so without it \
                         nothing is compiled for the browser: islands keep their server-rendered markup and the \
                         components keep their JavaScript behaviour.",
                        manifest.display()
                    ),
                    "run `next-rust generate islands` in the project root — it writes the crate, its cdylib and \
                     size profile, and the `next_rust_wasm::islands! { … }` the enabled keys need. Or set the keys \
                     back to their defaults.",
                )
                .location(manifest),
            );
        }
        return out;
    }

    // Late is better than never, but "it built and wrote no .wasm" is a poor
    // way to learn this, and it costs a whole nested compile to get there.
    if !declares_cdylib(&manifest) {
        out.push(
            diagnose(
                NOT_CDYLIB,
                "the islands crate does not declare a cdylib",
                format!(
                    "{} has no `[lib] crate-type = [\"cdylib\"]`, so cargo builds an rlib and there is no module \
                     for the browser to load. The build will compile the crate and then find nothing.",
                    manifest.display()
                ),
                "add `[lib]` with `crate-type = [\"cdylib\"]` to that manifest.",
            )
            .location(manifest.clone()),
        );
    }
    if let Some(d) = ignored_profile(&manifest) {
        out.push(d);
    }

    // The `islands!` invocation *is* the export list, so it answers both of
    // the questions `unwired` can only answer from a built module.
    let declared = declared_exports(&crate_dir);
    let Some(declared) = declared else {
        out.push(
            diagnose(
                NO_DECLARATION,
                "the islands crate declares no islands",
                format!(
                    "No file under {} invokes `next_rust_wasm::islands! {{ … }}`, which is what emits the \
                     allocator, the panic handler and every export the loader calls. A crate without it compiles \
                     to a module that exports nothing usable.",
                    crate_dir.join("src").display()
                ),
                "add `next_rust_wasm::islands! { … }` at the top level of the crate — `start: \
                 next_rust_ui_wasm::boot,` when `[ui] script = \"wasm\"` is on, then one `hydrate_<Island> => \
                 path::to::hydrate,` per island.",
            )
            .location(crate_dir.join("src")),
        );
        return out;
    };
    if config.ui.script.is_wasm() && !declared.start {
        out.push(no_start(&crate_dir));
    }
    if config.build.wasm_islands {
        for (name, file) in &requested {
            if !declared.islands.contains(name) {
                out.push(unknown_island(name, file, &declared.islands, &crate_dir));
            }
        }
    }
    out
}

/// What an islands crate's `islands!` invocations declare.
struct Declared {
    /// A `start:` entry, which is what wires up `[ui] script = "wasm"`.
    start: bool,
    /// The `hydrate_<Island>` export names, without the prefix.
    islands: Vec<String>,
}

/// Read the islands crate's declarations out of its source.
///
/// `None` means no `islands!` invocation was found at all, which is a
/// different problem from an invocation that declares nothing.
///
/// Only `src/` is read: `islands/target` is a full wasm build tree and holds
/// nothing the crate's own sources do not, the same rule `css_usage` and
/// `class_names` already follow for the same directory.
fn declared_exports(crate_dir: &Path) -> Option<Declared> {
    let mut found = false;
    let mut out = Declared { start: false, islands: Vec::new() };
    for (_, path) in crate::codegen::embeddable_files(&crate_dir.join("src")) {
        if !path.extension().is_some_and(|e| e == "rs") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        // Comments first. The crate this very file scaffolds has a module
        // doc comment that spells out `#[client(wasm = "<Name>")]`, and
        // reading it as code found a `[`, took everything to the next `}` as
        // the macro body, and lost the one entry that was really there — so
        // a perfectly wired crate reported `NR0313` against itself. Prose
        // that describes the syntax is exactly what a good islands crate
        // contains, so it has to be removed rather than tolerated.
        let source = blank_noncode(&source);
        let mut rest = source.as_str();
        while let Some(at) = rest.find("islands!") {
            rest = &rest[at + "islands!".len()..];
            let Some((body, consumed)) = macro_body(rest) else { continue };
            found = true;
            out.start |= body.contains("start:");
            for entry in body.split(',') {
                if let Some(name) = entry.split_once("=>").map(|(n, _)| n.trim())
                    && let Some(name) = name.strip_prefix(PREFIX)
                    && !name.is_empty()
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    out.islands.push(name.to_owned());
                }
            }
            rest = &rest[consumed..];
        }
    }
    out.islands.sort();
    out.islands.dedup();
    found.then_some(out)
}

/// The delimited body of a macro invocation that starts at `after` (the text
/// straight after the `!`), nested delimiters included, and how much of
/// `after` it consumed including both delimiters.
fn macro_body(after: &str) -> Option<(&str, usize)> {
    let trimmed = after.trim_start();
    let open = trimmed.chars().next().filter(|c| matches!(c, '{' | '(' | '['))?;
    let close = match open {
        '{' => '}',
        '(' => ')',
        _ => ']',
    };
    let start = after.len() - trimmed.len() + open.len_utf8();
    let mut depth = 1usize;
    for (i, c) in after[start..].char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some((&after[start..start + i], start + i + c.len_utf8()));
            }
        }
    }
    // Unbalanced: the crate does not compile either, and rustc says so
    // better than this could.
    None
}

/// Rust source with the *contents* of comments and string literals replaced
/// by spaces, byte for byte, so a scan for syntax cannot be fooled by prose
/// that quotes it — and so every offset in the result is still an offset in
/// the original.
///
/// Quote delimiters are kept, newlines are kept, and raw strings (`r"…"`,
/// `r#"…"#`, `b"…"`, `br##"…"##`) are recognised, because the documentation
/// this exists to ignore is written in them.
///
/// Not a Rust lexer, and does not need to be: it has to decide "is this `#[`
/// inside a string?" on files that compile.
fn blank_noncode(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = vec![b' '; bytes.len()];
    let identifier = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut i = 0usize;
    let code = |out: &mut Vec<u8>, at: usize| out[at] = bytes[at];
    while i < bytes.len() {
        let b = bytes[i];
        // Comments: everything inside goes, newlines survive so line
        // structure (and anything reading it) is unchanged.
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let mut depth = 1usize;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    i += 2;
                    continue;
                }
                if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    i += 2;
                    continue;
                }
                if bytes[i] == b'\n' {
                    out[i] = b'\n';
                }
                i += 1;
            }
            continue;
        }
        // A raw string, possibly byte-prefixed. `r` and `b` are ordinary
        // identifier characters too, so this only counts at the start of one.
        if (b == b'r' || b == b'b') && (i == 0 || !identifier(bytes[i - 1])) {
            let mut j = i + 1;
            if b == b'b' && bytes.get(j) == Some(&b'r') {
                j += 1;
            }
            let hashes = j;
            while bytes.get(j) == Some(&b'#') {
                j += 1;
            }
            let hashes = j - hashes;
            if bytes.get(j) == Some(&b'"') && (hashes > 0 || j > i + 1 || b == b'r') {
                for at in i..=j {
                    code(&mut out, at);
                }
                i = j + 1;
                let close: Vec<u8> = std::iter::once(b'"').chain(std::iter::repeat_n(b'#', hashes)).collect();
                while i < bytes.len() && !bytes[i..].starts_with(&close) {
                    if bytes[i] == b'\n' {
                        out[i] = b'\n';
                    }
                    i += 1;
                }
                for at in i..(i + close.len()).min(bytes.len()) {
                    code(&mut out, at);
                }
                i += close.len();
                continue;
            }
        }
        if b == b'"' {
            code(&mut out, i);
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\n' {
                    out[i] = b'\n';
                }
                // An escaped quote does not end the literal.
                i += usize::from(bytes[i] == b'\\') + 1;
            }
            if i < bytes.len() {
                code(&mut out, i);
            }
            i += 1;
            continue;
        }
        code(&mut out, i);
        i += 1;
    }
    // Only ASCII delimiters were ever kept or replaced, and every replacement
    // is one byte for one byte, so the result is still UTF-8 of the same
    // length — which is what makes the offsets usable.
    String::from_utf8(out).unwrap_or_else(|_| source.to_owned())
}

/// The islands the app asks for with `#[client(wasm = "…")]`, and one file
/// each that asks for them.
fn requested_islands(config: &Config) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    let mut dirs = vec![config.app_dir(), config.root.join("src")];
    if let Some(api) = config.api_dir() {
        dirs.push(api);
    }
    for dir in dirs {
        for (_, path) in crate::codegen::embeddable_files(&dir) {
            if !path.extension().is_some_and(|e| e == "rs") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else { continue };
            for name in client_wasm_names(&source) {
                out.entry(name).or_insert_with(|| path.clone());
            }
        }
    }
    out
}

/// The island names one Rust file asks for with `#[client(wasm = "…")]`.
///
/// A text scan rather than a `syn` parse: this runs over every file of the
/// app on every build-script run, the attribute has exactly one spelling, and
/// a file that does not parse is already reported as `NR0200` by the analyzer
/// long before anything here matters.
///
/// It does have to know code from prose, though. The framework's own
/// documentation site is a Next Rust application whose pages are Rust files
/// full of `r#"#[client(wasm = "AuthSwitch")]"#` — and reading those as
/// components made the website warn, on every build, about four islands that
/// do not exist. So the scan runs over [`blank_noncode`], which blanks the
/// contents of comments and string literals while keeping every byte offset,
/// and the name is then read back out of the real source at the same place.
fn client_wasm_names(source: &str) -> Vec<String> {
    let blanked = blank_noncode(source);
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = blanked[from..].find("#[client") {
        let at = from + rel + "#[client".len();
        from = at;
        let Some(end) = blanked[at..].find(']') else { break };
        let attr = &blanked[at..at + end];
        let Some(rest) = after(attr, "(").and_then(|r| after(r, "wasm")).and_then(|r| after(r, "=")) else {
            continue;
        };
        let Some(open) = rest.trim_start().strip_prefix('"') else { continue };
        // Offsets survive the blanking, so the quotes found here are the
        // quotes in the file and the name between them is the real one.
        let Some(len) = open.find('"').filter(|len| *len > 0) else { continue };
        let start = at + end - open.len();
        out.push(source[start..start + len].to_owned());
    }
    out
}

/// `text` after a leading `prefix`, ignoring whitespace before it.
fn after<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.trim_start().strip_prefix(prefix)
}

/// `[ui] script = "wasm"` with nothing behind it, said the same way whether
/// it was read out of the crate's source or out of the built module.
fn no_start(crate_dir: &Path) -> Diagnostic {
    diagnose(
        NO_START,
        "`[ui] script = \"wasm\"` is on and nothing starts the components",
        format!(
            "The key moved the components' behaviour out of /_next-rust/ui.js, and the module exports no `{START}` \
             to put it back. The loader calls `{START}()` only if it is there, so nothing fails: the components \
             render, look correct and are completely inert, `<html>` never gets `nr-ui-js`, and the stylesheet \
             leaves every enhanced control at `display: none` and 0×0 while a browser test clicks at it until its \
             budget runs out."
        ),
        format!(
            "add `{START}: next_rust_ui_wasm::boot,` as the first entry of the `next_rust_wasm::islands! {{ … }}` \
             in {} — or name your own `fn` there, as long as it calls `next_rust_ui_wasm::boot()`. Set `[ui] \
             script = \"js\"` to go back to the JavaScript components.",
            crate_dir.join("src").display()
        ),
    )
    .location(crate_dir.join("src"))
}

/// A `#[client(wasm = "…")]` with no `hydrate_` behind it: a typo, or an
/// island that was written and never listed.
fn unknown_island(name: &str, file: &Path, available: &[String], crate_dir: &Path) -> Diagnostic {
    let known = if available.is_empty() {
        "The islands crate declares none.".to_owned()
    } else {
        format!("The islands crate declares: {}.", available.join(", "))
    };
    diagnose(
        UNKNOWN_ISLAND,
        format!("`#[client(wasm = \"{name}\")]` has no `{PREFIX}{name}` behind it"),
        format!(
            "{known} The loader looks the export up by name and returns quietly when it is absent, so the island \
             renders its server markup and never reacts to anything — there is no error in the console and no \
             failed request to find it by."
        ),
        format!(
            "add `{PREFIX}{name} => path::to::hydrate,` to the `next_rust_wasm::islands! {{ … }}` in {}, or correct \
             the name in the attribute — it is the export name, and it is case-sensitive.",
            crate_dir.join("src").display()
        ),
    )
    .location(file.to_path_buf())
}

/// "2 components", or the one name when there is only one.
fn listing<'a>(mut names: impl Iterator<Item = &'a String>, one: &str, many: &str) -> String {
    let first = names.next().cloned().unwrap_or_default();
    let count = 1 + names.count();
    if count == 1 { format!("1 {one} (`{first}`)") } else { format!("{count} {many}") }
}

/// The first island name of a set, for a title.
fn first_key(map: &BTreeMap<String, PathBuf>) -> String {
    map.keys().next().cloned().unwrap_or_default()
}

/// Where a module's bytes come from.
enum Located {
    /// A `.wasm` file to use as it is.
    Prebuilt(PathBuf),
    /// A `Cargo.toml` to compile.
    Crate(PathBuf),
    /// An override was set and points at nothing usable; it has been reported.
    Missing,
}

/// `NEXT_RUST_WASM_<NAME>`, when it is set. Step (1) of the resolution order.
fn override_path(name: &str) -> Option<Located> {
    let key = format!("NEXT_RUST_WASM_{}", name.to_uppercase());
    println!("cargo:rerun-if-env-changed={key}");
    let value = std::env::var_os(&key)?;
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_file() {
        return Some(if path.extension().is_some_and(|e| e == "toml") {
            Located::Crate(path)
        } else {
            Located::Prebuilt(path)
        });
    }
    let manifest = path.join("Cargo.toml");
    if manifest.is_file() {
        watch(&path);
        return Some(Located::Crate(manifest));
    }
    emit(
        &diagnose(
            BAD_OVERRIDE,
            format!("{key} points at nothing usable"),
            format!(
                "{} is neither a .wasm module nor a directory holding a Cargo.toml, so the {name} contribution was \
                 skipped and the page falls back to the JavaScript it would have shipped anyway.",
                path.display()
            ),
            format!("point {key} at a prebuilt .wasm file or at a crate directory, or unset it."),
        )
        .location(path),
    );
    Some(Located::Missing)
}

/// Whether one of the framework's own wasm crates can reach the module: the
/// override, then a sibling crate in the checkout, then a warning. Steps
/// (1)–(3) of the resolution order.
///
/// A crate that is there is put on cargo's watch list, so editing it rebuilds
/// the module like editing an island does.
fn contributes(name: &str, what: &str, fallback: &str, candidates: &[&str]) -> bool {
    match override_path(name) {
        Some(Located::Prebuilt(_)) => return true,
        Some(Located::Crate(manifest)) => {
            if let Some(dir) = manifest.parent() {
                watch(dir);
            }
            return true;
        }
        Some(Located::Missing) => return false,
        None => {}
    }
    if let Some(manifest) = sibling_crate(candidates) {
        if let Some(dir) = manifest.parent() {
            watch(dir);
        }
        return true;
    }
    emit(&diagnose(
        NO_FRAMEWORK_CRATE,
        format!("{what} needs `{}`, and there is no source for it here", candidates[0]),
        format!(
            "`{}` is not next to this copy of next-rust-build, and a build against the published crates has no \
             source for it. Its contribution was skipped: {fallback}.",
            candidates[0]
        ),
        format!(
            "point NEXT_RUST_WASM_{} at a prebuilt {name}.wasm shipped beside the application, or depend on \
             next-rust by path from a checkout of the framework.",
            name.to_uppercase()
        ),
    ));
    false
}

/// A `crates/<candidate>/Cargo.toml` above this crate's own source directory
/// that declares a `cdylib`. `CARGO_MANIFEST_DIR` is baked in at compile time
/// and so points at *this* crate's source, which exists only when
/// `next-rust-build` is a path dependency — exactly the case this step is for.
fn sibling_crate(candidates: &[&str]) -> Option<PathBuf> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    for ancestor in here.ancestors() {
        for candidate in candidates {
            let manifest = ancestor.join("crates").join(candidate).join("Cargo.toml");
            if manifest.is_file() && is_library(&manifest) {
                return Some(manifest);
            }
        }
    }
    None
}

/// Whether a manifest asks for a `cdylib`, which the **app's** islands crate
/// must: it is the one the browser loads, and a crate that is not a `cdylib`
/// produces no module at all.
fn declares_cdylib(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest).is_ok_and(|text| text.contains("cdylib"))
}

/// Whether a manifest describes a library, which is all a **framework
/// contributor** has to be.
///
/// Deliberately *not* [`declares_cdylib`]. The contributors are `rlib`s by
/// design and are linked into the app's `cdylib`; each carrying its own
/// `#[panic_handler]` and `#[global_allocator]` would collide at link time,
/// and making them `cdylib`s breaks `cargo build --workspace` on the host.
/// Requiring a `cdylib` here made `[ui] script = "wasm"` report the UI crate
/// as missing while it sat in the checkout.
fn is_library(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest).is_ok_and(|text| text.contains("[package]") && !text.contains("[[bin]]"))
}

/// Tell cargo to rerun the build script when a crate changes.
fn watch(crate_dir: &Path) {
    println!("cargo:rerun-if-changed={}", crate_dir.join("src").display());
    println!("cargo:rerun-if-changed={}", crate_dir.join("Cargo.toml").display());
}

/// Build the crate for the browser and return the module it wrote.
///
/// `Ok(None)` means the module could not be built for a reason that is not
/// the author's mistake — today only a missing `wasm32-unknown-unknown` — and
/// has been reported as a warning.
fn compile(manifest: &Path, target_dir: &Path) -> Result<Option<PathBuf>, String> {
    let name = ISLANDS;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg(TARGET)
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(target_dir);
    for key in CLEARED_ENV {
        command.env_remove(key);
    }
    // Absolute paths otherwise reach the module through panic messages and
    // debug info, which would make the bytes depend on where the crate sits.
    // The encoded form takes a unit separator, so a path with a space in it
    // survives where a space-separated `RUSTFLAGS` would not.
    if let Some(flags) = remap_flags(manifest) {
        command.env("CARGO_ENCODED_RUSTFLAGS", flags);
    }
    let output = command.output().map_err(|e| format!("cannot run cargo for the {name} crate: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if missing_target(&stderr) {
            emit(&missing_target_diagnostic());
            return Ok(None);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "the {name} crate failed to compile to WebAssembly ({}):\n\n{stderr}{stdout}",
            output.status
        ));
    }
    let release = target_dir.join(TARGET).join("release");
    let mut modules: Vec<PathBuf> = std::fs::read_dir(&release)
        .map_err(|e| format!("cannot read {}: {e}", release.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "wasm"))
        .collect();
    modules.sort();
    modules.into_iter().next().map(Some).ok_or_else(|| {
        format!(
            "the {name} crate built, but wrote no .wasm module to {}.\n\
             Its Cargo.toml needs `[lib] crate-type = [\"cdylib\"]`.",
            release.display()
        )
    })
}

/// The islands crate carrying a `[profile.*]` section that cargo will ignore.
///
/// Cargo reads profiles from the **workspace root only**. A crate that is a
/// workspace member therefore gets the root's release profile, not its own,
/// and an islands crate that asked for `opt-level = "z"`, fat LTO, one
/// codegen unit and `panic = "abort"` silently gets none of it — measured
/// elsewhere in this project at roughly twice the bytes.
///
/// This warns rather than forcing the profile through
/// `CARGO_PROFILE_RELEASE_*`. Measured: setting those variables changes
/// cargo's metadata hash and so the module's bytes, even when the effective
/// profile is identical, which costs reproducibility for a setting the author
/// can fix once in their own manifest.
fn ignored_profile(manifest: &Path) -> Option<Diagnostic> {
    let text = std::fs::read_to_string(manifest).ok()?;
    // No profile to ignore, or the crate is its own workspace root, in which
    // case cargo does read it.
    if !text.contains("[profile.") || text.contains("[workspace]") {
        return None;
    }
    let crate_dir = manifest.parent()?;
    // A workspace that excludes this crate does not claim its profile.
    // Spelling out the member globs is more than a warning is worth; an
    // `exclude` naming the directory is the case worth honouring.
    let quoted = format!("\"{}\"", crate_dir.file_name().unwrap_or_default().to_string_lossy());
    let claims_it = |dir: &&Path| {
        std::fs::read_to_string(dir.join("Cargo.toml"))
            .is_ok_and(|t| t.contains("[workspace]") && !(t.contains("exclude") && t.contains(&quoted)))
    };
    let root = crate_dir.ancestors().skip(1).find(claims_it)?;
    Some(
        diagnose(
            IGNORED_PROFILE,
            "the islands crate's release profile is ignored",
            format!(
                "{} sets its own `[profile.*]`, but it is a member of the workspace at {} and cargo reads profiles \
                 from the workspace root only — so the module is built with the workspace's release profile, not \
                 its own, and is typically about twice the size it should be.",
                manifest.display(),
                root.display()
            ),
            format!(
                "give the islands crate an empty `[workspace]` table so it is its own root (what `next-rust \
                 generate islands` writes), or move those settings into {}/Cargo.toml.",
                root.display()
            ),
        )
        .location(manifest.to_path_buf()),
    )
}

/// `--remap-path-prefix` for everything that can end up inside a module,
/// encoded for `CARGO_ENCODED_RUSTFLAGS` (unit-separated).
fn remap_flags(manifest: &Path) -> Option<String> {
    let mut flags: Vec<String> = Vec::new();
    if let Some(dir) = manifest.parent() {
        flags.push(format!("--remap-path-prefix={}=/next-rust/crate", dir.display()));
    }
    if let Some(home) = cargo_home() {
        flags.push(format!("--remap-path-prefix={}=/next-rust/cargo", home.display()));
    }
    if flags.is_empty() { None } else { Some(flags.join("\u{1f}")) }
}

/// `CARGO_HOME`, or the default `~/.cargo`: where the registry sources that
/// get compiled into a module live.
fn cargo_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CARGO_HOME") {
        return Some(PathBuf::from(home));
    }
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| PathBuf::from(h).join(".cargo"))
}

/// The browser target this whole pipeline is built for, so a caller that
/// checks for it (`next-rust doctor`) and the build that needs it name the
/// same string.
pub const WASM_TARGET: &str = TARGET;

/// No `wasm32-unknown-unknown`. Shared by the build, which learns it from a
/// failed cargo run, and by `next-rust doctor`, which asks rustup before
/// anything is compiled.
pub fn missing_target_diagnostic() -> Diagnostic {
    diagnose(
        NO_TARGET,
        format!("the `{TARGET}` target is not installed"),
        "Nothing can be compiled for the browser without it, so no module is built: islands keep their \
         server-rendered markup and gain no behaviour, and the components keep their JavaScript behaviour. The \
         build still succeeds — a missing toolchain component is the machine's problem, not the code's.",
        format!("run `rustup target add {TARGET}` and build again."),
    )
}

/// Whether a cargo failure is the missing `wasm32-unknown-unknown` target.
fn missing_target(stderr: &str) -> bool {
    stderr.contains("rustup target add")
        || (stderr.contains("wasm32-unknown-unknown")
            && (stderr.contains("may not be installed")
                || stderr.contains("not installed")
                || stderr.contains("can't find crate for `core`")))
}

/// The `wasm-opt` command line, without the input and output paths.
///
/// The `--enable-*` flags matter more than they look. `rustc` emits
/// `memory.copy`, sign extension and the rest for `wasm32-unknown-unknown`
/// without being asked, and a `wasm-opt` whose defaults predate those
/// proposals **refuses the whole module** rather than passing them through.
/// The failure is silent in the worst way: the module is valid, the browser
/// runs it, and it is simply bigger than it needed to be. Naming the features
/// costs nothing on a module that does not use them.
fn opt_args(config: &Config) -> Vec<&'static str> {
    let mut args = vec![
        "-Oz",
        "--enable-bulk-memory",
        "--enable-bulk-memory-opt",
        "--enable-nontrapping-float-to-int",
        "--enable-sign-ext",
        "--enable-mutable-globals",
    ];
    // `--strip-debug` takes the name section with it. `[build]
    // wasm_keep_names = true` keeps it: the module is bigger, and a stack
    // trace in the browser console names the Rust functions. It can only keep
    // a section that is there — a crate whose own `[profile.release]` sets
    // `strip = true` has already dropped the names before this runs, and the
    // key then makes no difference.
    if !config.build.wasm_keep_names {
        args.push("--strip-debug");
    }
    args.push("--strip-producers");
    args.push("--strip-target-features");
    args
}

/// Shrink the module with `wasm-opt` when it is on `PATH`, and say whether
/// that happened. Neither a missing nor a failing `wasm-opt` is an error: the
/// unoptimized module works, it is only bigger.
///
/// The three outcomes are deliberately not treated alike. `[build] wasm_opt =
/// false` is silent — it was asked for. A `wasm-opt` that is not installed
/// gets one warning per build. A `wasm-opt` that *is* installed and then
/// fails or makes no difference gets a warning every time, because the step
/// whose only job is size quietly shipping something bigger is the worst
/// failure mode this file has.
fn optimize(config: &Config, wasm: &Path, target_dir: &Path) -> (PathBuf, bool) {
    let name = ISLANDS;
    if !config.build.wasm_opt {
        return (wasm.to_path_buf(), false);
    }
    let optimized = target_dir.join(format!("{name}.opt.wasm"));
    let status = Command::new("wasm-opt").args(opt_args(config)).arg(wasm).arg("-o").arg(&optimized).output();
    let size = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    let (before, after) = (size(wasm), size(&optimized));
    if matches!(&status, Ok(output) if output.status.success()) && after > 0 && after < before {
        return (optimized, true);
    }
    let _ = std::fs::remove_file(&optimized);
    match &status {
        // Not installed: worth saying once, not once per module.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if !WASM_OPT_WARNED.swap(true, Ordering::Relaxed) {
                emit(&no_wasm_opt(before));
            }
        }
        // It is installed and it refused. Show its own words: they name the
        // instruction or feature it choked on, which is the whole diagnosis.
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            emit(&diagnose(
                WASM_OPT_FAILED,
                format!("wasm-opt refused {name}.wasm"),
                format!(
                    "It exited {} and the module ships unoptimized at {before} bytes. Its output was: {}",
                    output.status,
                    one_line(&stderr)
                ),
                "that output names the instruction or feature it choked on. A wasm-opt older than the features \
                 rustc emits is the usual cause: `brew upgrade binaryen` or `cargo install wasm-opt`.",
            ));
        }
        // It ran, it succeeded, and it did not help. Rare and worth knowing:
        // it usually means the module was already optimized, or `wasm-opt` is
        // old enough to pass the module through untouched.
        Ok(_) => emit(&diagnose(
            WASM_OPT_FAILED,
            format!("wasm-opt made {name}.wasm no smaller"),
            format!("{before} bytes in, {after} out, so the unoptimized module ships."),
            "a wasm-opt older than the features rustc emits passes the module straight through; `brew upgrade \
             binaryen` or `cargo install wasm-opt` usually fixes it. `[build] wasm_opt = false` stops asking.",
        )),
        Err(e) => emit(&diagnose(
            WASM_OPT_FAILED,
            "wasm-opt could not be run",
            format!("{e}, so {name}.wasm ships unoptimized at {before} bytes."),
            "check that the `wasm-opt` on PATH is executable, or set `[build] wasm_opt = false`.",
        )),
    }
    (wasm.to_path_buf(), false)
}

/// `wasm-opt` is not installed. Shared with `next-rust doctor`, which says so
/// before a build rather than after one; `bytes` is 0 when nothing has been
/// built to measure.
pub fn no_wasm_opt(bytes: u64) -> Diagnostic {
    let size = if bytes > 0 { format!(" at {bytes} bytes") } else { String::new() };
    diagnose(
        NO_WASM_OPT,
        "wasm-opt is not on PATH",
        format!(
            "The module ships exactly as the linker wrote it{size} — `wasm-opt -Oz` usually takes a third to a \
             half of that off. This is a size problem only: the module is valid and the page works."
        ),
        "install it with `brew install binaryen` or `cargo install wasm-opt`, or set `[build] wasm_opt = false` to \
         stop asking.",
    )
}

/// A tool's output squeezed onto one line, so it survives `cargo:warning=`.
fn one_line(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if joined.chars().count() > 300 { joined.chars().take(300).collect::<String>() + "…" } else { joined }
}

/// Brotli and gzip sizes of a module, at the settings the rest of the build
/// compresses embedded files with (see `codegen::precompressed`: Brotli
/// quality 11 window 22, gzip level 9, nothing under a kilobyte). Sizes only:
/// the module is a file under the generated `client/`, so the build
/// precompresses and serves it like any other, and these two numbers are what
/// the size report shows.
fn compressed_sizes(bytes: &[u8]) -> [Option<usize>; 2] {
    use std::io::Write;
    if bytes.len() < 1024 {
        return [None, None];
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::with_capacity(bytes.len() / 2), flate2::Compression::best());
    let gz = gz.write_all(bytes).and_then(|()| gz.finish()).unwrap_or_default();
    [brotli_size(bytes), Some(gz.len()).filter(|n| *n > 0)]
}

/// Brotli size at the quality the server serves precompressed files at.
fn brotli_size(bytes: &[u8]) -> Option<usize> {
    use std::io::Write;
    let mut out = Vec::with_capacity(bytes.len() / 2);
    {
        let mut w = brotli::CompressorWriter::new(&mut out, 4096, 11, 22);
        w.write_all(bytes).ok()?;
    }
    Some(out.len()).filter(|n| *n > 0)
}

/// What the island loader costs, alongside the module it loads: a visitor
/// downloads both. It is a `client/` script like any other, so a release
/// build minifies it — that is the number with the budget on it.
fn shim_report() -> crate::report::ShimReport {
    let minified = minified_shim();
    let served = minified.as_deref().unwrap_or(LOADER);
    crate::report::ShimReport {
        bytes: LOADER.len(),
        minified: minified.as_ref().map(String::len),
        brotli: brotli_size(served.as_bytes()),
    }
}

/// The loader as a release build ships it.
#[cfg(feature = "minify-js")]
fn minified_shim() -> Option<String> {
    crate::js_minify::minify_with(LOADER, true, true).ok()
}

/// Without the minifier there is nothing to measure: the loader is served as
/// written.
#[cfg(not(feature = "minify-js"))]
fn minified_shim() -> Option<String> {
    None
}

/// The island names a module declares: its `hydrate_<Island>` exports,
/// sorted, without the prefix.
fn island_names(exports: &[(String, u8)]) -> Vec<String> {
    let mut names: Vec<String> = exports
        .iter()
        .filter(|(name, kind)| *kind == KIND_FUNCTION && name.len() > PREFIX.len() && name.starts_with(PREFIX))
        .map(|(name, _)| name[PREFIX.len()..].to_owned())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The page-level entry point. The loader calls it once after instantiating
/// the module — `X.start && X.start()` — and a module without it is loaded,
/// instantiated and asked for nothing.
const START: &str = "start";

/// What the configuration promised that the built module does not deliver,
/// one sentence per problem. Empty is the ordinary case.
///
/// This is the only check in this file that looks at the **artifact** rather
/// than at the file system, and it exists because the two failures below are
/// the quietest this framework has. A module with no [`START`] loads, the
/// loader's `X.start && X.start()` shrugs, and the page that asked for
/// `[ui] script = "wasm"` renders every component correctly and inertly:
/// `<html>` never gets `nr-ui-js`, so the stylesheet leaves each enhanced
/// control at `display: none`, 0×0, with the native one underneath still
/// working. Nothing throws, nothing logs, and the page looks right — the only
/// visible symptom is a browser test whose `click()` waits out its budget on
/// an element that will never be clickable, which reads as flake. That cost
/// one investigation from scratch; it should cost one line of build output.
///
/// **Warnings, not errors**, in keeping with the rest of this file: nothing
/// about wasm fails a build except a compile error in the author's own Rust.
/// The argument for failing is real — a missing export is deterministic,
/// unlike a missing target or a missing `wasm-opt`, so it cannot be the
/// machine's fault — but two things outweigh it. It is inferred from a parse
/// of a binary, and a build that refuses on a wrong inference (a prebuilt
/// module wired some other way, a crate mid-migration that has not reached
/// its first island yet) is a worse failure than the one being prevented.
/// And the value here is the diagnosis, not the refusal: a named key, a named
/// export and the line to add end the investigation either way.
fn unwired(
    config: &Config,
    exports: &[(String, u8)],
    islands: &[String],
    requested: &BTreeMap<String, PathBuf>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let crate_dir = config.root.join(ISLANDS);
    let exported = |want: &str| exports.iter().any(|(name, kind)| *kind == KIND_FUNCTION && name == want);
    if config.ui.script.is_wasm() && !exported(START) {
        out.push(no_start(&crate_dir));
    }
    if config.build.wasm_islands {
        if islands.is_empty() {
            out.push(
                diagnose(
                    NO_ISLANDS,
                    "`[build] wasm_islands = true` is on and the module exports no island",
                    format!(
                        "The module was built and exports no `{PREFIX}<Island>` at all, so no island of this app \
                         can hydrate from it and every `#[client(wasm = \"…\")]` stays at its server-rendered \
                         markup.{}",
                        match requested.is_empty() {
                            true => String::new(),
                            false => format!(
                                " Waiting for one: {}.",
                                requested.keys().cloned().collect::<Vec<_>>().join(", ")
                            ),
                        }
                    ),
                    format!(
                        "list the islands in the `next_rust_wasm::islands! {{ … }}` in {} \
                         (`{PREFIX}Name => crate::name::hydrate,`), or set `[build] wasm_islands = false`.",
                        crate_dir.join("src").display()
                    ),
                )
                .location(crate_dir.join("src")),
            );
        } else {
            // A name-by-name check, which is the one that catches a typo. The
            // loader looks `hydrate_<name>` up and returns quietly when it is
            // absent, so the only symptom is an island that does nothing.
            for (name, file) in requested {
                if !islands.iter().any(|i| i == name) {
                    out.push(unknown_island(name, file, islands, &crate_dir));
                }
            }
            // And the other way round: bytes in every visitor's download that
            // nothing on the site can reach.
            for name in islands.iter().filter(|name| !requested.contains_key(*name)) {
                out.push(diagnose(
                    UNUSED_ISLAND,
                    format!("the module exports `{PREFIX}{name}` and nothing asks for it"),
                    "No component in this project is written `#[client(wasm = \"…\")]` with that name, so the \
                     island and everything it reaches are compiled into the module every visitor downloads and \
                     never run."
                        .to_owned(),
                    format!(
                        "point a component at it with `#[client(wasm = \"{name}\")]`, or drop the \
                         `{PREFIX}{name} => …` entry from the `next_rust_wasm::islands! {{ … }}`."
                    ),
                ));
            }
        }
    }
    // The shared machinery. `islands!` always emits these, so a module
    // missing one was either hand-written or came in through
    // `NEXT_RUST_WASM_ISLANDS` — and a loader that cannot allocate simply
    // hydrates every island with no props, which looks like a decoder bug.
    if !islands.is_empty() || exported(START) {
        for want in ["alloc", "on_event"] {
            if !exported(want) {
                out.push(diagnose(
                    MISSING_EXPORT,
                    format!("the module exports no `{want}`"),
                    format!(
                        "The loader calls `{want}` on every island that receives props or registers a listener. \
                         Without it the call is skipped and the island runs with nothing: no props, or no events."
                    ),
                    "build the module from `next_rust_wasm::islands! { … }`, which emits the allocator, the panic \
                     handler and every shared export the loader calls. A hand-written or prebuilt module has to \
                     export them itself.",
                ));
            }
        }
    }
    out
}

/// Export kind of a function.
const KIND_FUNCTION: u8 = 0;

/// Section id of the export section.
const EXPORT_SECTION: u8 = 7;

/// A cursor over a wasm binary. Every read is fallible: a truncated or
/// malformed module returns `None`, never panics.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn done(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    /// An unsigned LEB128 integer. Over-long encodings are rejected rather
    /// than wrapped around.
    fn leb128(&mut self) -> Option<u64> {
        let mut value: u64 = 0;
        for shift in 0..10u32 {
            let byte = self.byte()?;
            let bits = u64::from(byte & 0x7f);
            value |= bits.checked_shl(shift * 7).filter(|v| v >> (shift * 7) == bits)?;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn usize(&mut self) -> Option<usize> {
        usize::try_from(self.leb128()?).ok()
    }
}

/// Every export of a module as `(name, kind)`, in the order the export
/// section lists them. A module without an export section has none.
fn exports(wasm: &[u8]) -> Result<Vec<(String, u8)>, String> {
    let mut reader = Reader::new(wasm);
    let header = reader.take(8).ok_or("the WebAssembly module is truncated: no header")?;
    if &header[..4] != b"\0asm" {
        return Err("not a WebAssembly module: the magic bytes are missing".to_owned());
    }
    if header[4..] != [1, 0, 0, 0] {
        return Err(format!("unsupported WebAssembly version {:?}", &header[4..]));
    }
    let section = loop {
        if reader.done() {
            return Ok(Vec::new());
        }
        let id = reader.byte().ok_or(TRUNCATED)?;
        let len = reader.usize().ok_or(TRUNCATED)?;
        let body = reader.take(len).ok_or(TRUNCATED)?;
        if id == EXPORT_SECTION {
            break body;
        }
    };
    let mut reader = Reader::new(section);
    let count = reader.usize().ok_or(TRUNCATED)?;
    let mut out = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let len = reader.usize().ok_or(TRUNCATED)?;
        let name = reader.take(len).ok_or(TRUNCATED)?;
        let name = std::str::from_utf8(name).map_err(|_| "an export name is not valid UTF-8".to_owned())?;
        let kind = reader.byte().ok_or(TRUNCATED)?;
        reader.leb128().ok_or(TRUNCATED)?;
        out.push((name.to_owned(), kind));
    }
    Ok(out)
}

const TRUNCATED: &str = "the WebAssembly module is truncated or malformed";

#[cfg(test)]
mod tests {
    use super::*;

    /// Unsigned LEB128, as the wasm binary format writes lengths and counts.
    fn leb(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    /// A module whose export section lists `(name, kind)`.
    fn module(exports: &[(&str, u8)]) -> Vec<u8> {
        let mut section = leb(exports.len() as u64);
        for (index, (name, kind)) in exports.iter().enumerate() {
            section.extend(leb(name.len() as u64));
            section.extend(name.as_bytes());
            section.push(*kind);
            section.extend(leb(index as u64));
        }
        let mut out = b"\0asm\x01\0\0\0".to_vec();
        // A custom section first: sections before the export section are skipped.
        out.push(0);
        out.extend(leb(5));
        out.extend([4, b'n', b'a', b'm', b'e']);
        out.push(EXPORT_SECTION);
        out.extend(leb(section.len() as u64));
        out.extend(section);
        out
    }

    /// The island names of a module, the way [`build`] derives them: parse
    /// the export section once, then filter it.
    fn islands(wasm: &[u8]) -> Result<Vec<String>, String> {
        Ok(island_names(&exports(wasm)?))
    }

    /// A module wired the way an app that asked for both keys should be.
    fn wired() -> Vec<u8> {
        module(&[
            ("memory", 2),
            ("alloc", KIND_FUNCTION),
            ("on_event", KIND_FUNCTION),
            (START, KIND_FUNCTION),
            ("hydrate_Home", KIND_FUNCTION),
        ])
    }

    /// No `#[client(wasm = …)]` anywhere, which is what an app that only
    /// turned on `[ui] script = "wasm"` looks like.
    fn nothing_requested() -> BTreeMap<String, PathBuf> {
        BTreeMap::new()
    }

    /// The islands a set of names would be requested by.
    fn requested(names: &[&str]) -> BTreeMap<String, PathBuf> {
        names.iter().map(|n| ((*n).to_owned(), PathBuf::from("app/page.rs"))).collect()
    }

    /// Both keys on, both exports present, and the one island the app asks
    /// for is there: the ordinary build says nothing.
    #[test]
    fn a_wired_module_warns_about_nothing() {
        use next_rust_core::config::UiScript;
        let mut config = Config::default();
        config.ui.script = UiScript::Wasm;
        config.build.wasm_islands = true;
        let exports = exports(&wired()).unwrap();
        let warnings = unwired(&config, &exports, &island_names(&exports), &requested(&["Home"]));
        assert!(warnings.is_empty(), "{:?}", codes(&warnings));
    }

    /// The codes of a set of diagnostics, for assertions that care which
    /// problem was reported rather than how it was worded.
    fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.code.as_str()).collect()
    }

    /// The whole point of this file's export check: `[ui] script = "wasm"`
    /// with a module that has no `start`. This is the shape that renders
    /// perfectly and does nothing, and it must be one line of build output.
    #[test]
    fn a_ui_module_without_start_is_reported() {
        use next_rust_core::config::UiScript;
        let mut config = Config::default();
        config.ui.script = UiScript::Wasm;
        // Everything else the module normally exports, and no `start`.
        let bytes = module(&[
            ("memory", 2),
            ("alloc", KIND_FUNCTION),
            ("on_event", KIND_FUNCTION),
            ("hydrate_Home", KIND_FUNCTION),
        ]);
        let listed = exports(&bytes).unwrap();
        let warnings = unwired(&config, &listed, &island_names(&listed), &nothing_requested());
        assert_eq!(codes(&warnings), [NO_START], "{warnings:?}");
        // A reader has to be able to act on it without opening this file:
        // the code, the key that is on, the export that is missing, and the
        // fix — all of it on the one line a cargo warning gets.
        let line = cargo_line(&warnings[0]);
        for part in [NO_START, "[ui] script = \"wasm\"", "start", "next_rust_ui_wasm::boot", "islands!", "help:"] {
            assert!(line.contains(part), "{part} is missing from: {line}");
        }
        assert!(!line.contains('\n'), "a cargo:warning is one line: {line}");
        // It is the export that decides, not the island count: a module with
        // no islands at all but a `start` has the UI kit wired.
        let only_start =
            exports(&module(&[("alloc", KIND_FUNCTION), ("on_event", KIND_FUNCTION), (START, KIND_FUNCTION)])).unwrap();
        assert!(unwired(&config, &only_start, &island_names(&only_start), &nothing_requested()).is_empty());
        // And a `start` that is exported as something other than a function
        // is not one the loader can call.
        let not_a_function = exports(&module(&[
            ("alloc", KIND_FUNCTION),
            ("on_event", KIND_FUNCTION),
            (START, 1),
            ("hydrate_Home", KIND_FUNCTION),
        ]))
        .unwrap();
        assert_eq!(
            codes(&unwired(&config, &not_a_function, &island_names(&not_a_function), &nothing_requested())),
            [NO_START]
        );
    }

    /// `[build] wasm_islands = true` and a module with no `hydrate_` export:
    /// the key is on, the crate compiled, and not one island can hydrate.
    #[test]
    fn islands_that_are_asked_for_and_not_exported_are_reported() {
        let mut config = Config::default();
        config.build.wasm_islands = true;
        let exports =
            exports(&module(&[("alloc", KIND_FUNCTION), ("on_event", KIND_FUNCTION), (START, KIND_FUNCTION)])).unwrap();
        let warnings = unwired(&config, &exports, &island_names(&exports), &requested(&["Counter"]));
        assert_eq!(codes(&warnings), [NO_ISLANDS], "{warnings:?}");
        let line = cargo_line(&warnings[0]);
        assert!(line.contains("[build] wasm_islands = true") && line.contains(PREFIX), "{line}");
        // It names the island the app is waiting for, which is the whole
        // difference between "something is off" and "this is the file".
        assert!(line.contains("Counter"), "{line}");
        // `[ui] script = "wasm"` is not on here, so the absent `start` is not
        // this app's problem and is not mentioned.
        assert!(!line.contains("next_rust_ui_wasm"), "{line}");
    }

    /// The typo case, and the one the loader is silent about: the module has
    /// islands, and not the one this component names.
    #[test]
    fn an_island_the_module_does_not_export_is_named() {
        let mut config = Config::default();
        config.build.wasm_islands = true;
        let listed = exports(&wired()).unwrap();
        let warnings = unwired(&config, &listed, &island_names(&listed), &requested(&["home"]));
        // The name it could not find, and the names it did — a typo is
        // usually obvious once both are on the same line.
        let line = cargo_line(&warnings[0]);
        assert_eq!(warnings[0].code, UNKNOWN_ISLAND, "{line}");
        assert!(line.contains("hydrate_home") && line.contains("Home"), "{line}");
        assert!(line.contains("case-sensitive"), "{line}");
        assert_eq!(warnings[0].locations, [PathBuf::from("app/page.rs")]);
        // The reverse: an island in the module nobody asks for is weight in
        // every visitor's download.
        let unused = unwired(&config, &listed, &island_names(&listed), &nothing_requested());
        assert_eq!(codes(&unused), [UNUSED_ISLAND], "{unused:?}");
        assert!(cargo_line(&unused[0]).contains("hydrate_Home"));
    }

    /// A key that is off is never the subject of a warning, whatever the
    /// module happens to export. Projects that never asked for wasm, and
    /// projects that asked for only one of the two, stay quiet.
    #[test]
    fn keys_that_are_off_are_not_second_guessed() {
        let empty = exports(&module(&[("memory", 2)])).unwrap();
        let config = Config::default();
        assert!(unwired(&config, &empty, &island_names(&empty), &nothing_requested()).is_empty());
        // Both on, nothing exported: both, and only both.
        let mut both = Config::default();
        both.ui.script = next_rust_core::config::UiScript::Wasm;
        both.build.wasm_islands = true;
        assert_eq!(codes(&unwired(&both, &empty, &island_names(&empty), &nothing_requested())), [NO_START, NO_ISLANDS]);
        // `[client] runtime = "wasm"` is served from the module
        // `next-rust-server` carries, not from this one, so it requires
        // nothing here and is not checked.
        let mut runtime = Config::default();
        runtime.client.runtime = next_rust_core::config::ClientRuntime::Wasm;
        assert!(unwired(&runtime, &empty, &island_names(&empty), &nothing_requested()).is_empty());
    }

    /// The shared machinery `islands!` emits. A module without it hydrates
    /// every island with no props at all, which reads as a decoder bug.
    #[test]
    fn a_module_missing_the_shared_exports_is_reported() {
        let mut config = Config::default();
        config.build.wasm_islands = true;
        let listed = exports(&module(&[("memory", 2), ("hydrate_Home", KIND_FUNCTION)])).unwrap();
        let warnings = unwired(&config, &listed, &island_names(&listed), &requested(&["Home"]));
        assert_eq!(codes(&warnings), [MISSING_EXPORT, MISSING_EXPORT], "{warnings:?}");
        assert!(cargo_line(&warnings[0]).contains("alloc"));
        assert!(cargo_line(&warnings[1]).contains("on_event"));
    }

    /// Every code this file can emit is listed in [`DIAGNOSTICS`], and every
    /// entry there is real. The table is what the documentation and the skill
    /// are written from, so a code that only exists in a `format!` is a code
    /// nobody can look up.
    #[test]
    fn every_code_is_listed_once() {
        let source = include_str!("wasm_islands.rs");
        let mut codes: Vec<&str> = DIAGNOSTICS.iter().map(|(code, _)| *code).collect();
        let sorted = {
            let mut c = codes.clone();
            c.sort_unstable();
            c
        };
        assert_eq!(codes, sorted, "the table is in code order");
        codes.dedup();
        assert_eq!(codes.len(), DIAGNOSTICS.len(), "a code is listed twice");
        for (code, title) in DIAGNOSTICS {
            assert!(code.starts_with("NR03") && code.len() == 6, "{code} is not a wasm diagnostic code");
            assert!(!title.is_empty());
            // Spelled out once, as a `const`, and reached by name everywhere
            // else. A code written straight into a `format!` is a code the
            // documentation and the skill cannot be checked against.
            assert_eq!(source.matches(&format!("\"{code}\"")).count(), 1, "{code} is spelled out other than once");
        }
    }

    /// `#[client(wasm = "…")]` is found wherever it is written, and nothing
    /// else is mistaken for it.
    #[test]
    fn island_requests_are_read_out_of_the_source() {
        let source = r#"
            #[client(wasm = "AuthSwitch")]
            pub fn Switch() -> impl View { div![] }

            #[client(wasm="Chart")]
            pub fn Chart(points: Vec<f64>) -> impl View { div![] }

            #[client]
            pub fn Counter(n: i32) -> impl View { div![] }

            #[client(script = "chart.js")]
            pub fn Other() -> impl View { div![] }

            #[client(module = "/_next-rust/client/islands/my_widgets.wasm.js")]
            pub fn Third() -> impl View { div![] }
        "#;
        assert_eq!(client_wasm_names(source), ["AuthSwitch", "Chart"]);
        // Nothing to find, and nothing that panics on a half-written file.
        assert!(client_wasm_names("").is_empty());
        assert!(client_wasm_names("#[client(wasm = ").is_empty());
        assert!(client_wasm_names("#[client(wasm = \"\")]").is_empty());
        // A commented-out island is not one the app asks for, which is the
        // right answer both ways round: it is not hydrated, so warning that
        // it is not wired would be noise.
        assert!(client_wasm_names("// #[client(wasm = \"Ghost\")]").is_empty());
    }

    /// The `islands!` invocation is the export list, and reading it is what
    /// lets `next-rust doctor` answer the question without a toolchain.
    #[test]
    fn declarations_are_read_out_of_the_islands_crate() {
        let dir = scratch("declared");
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        // No invocation at all is a different answer from an empty one.
        std::fs::write(src.join("lib.rs"), "#![no_std]\n").unwrap();
        assert!(declared_exports(&dir).is_none());
        std::fs::write(
            src.join("lib.rs"),
            "#![no_std]\nnext_rust_wasm::islands! {\n    start: next_rust_ui_wasm::boot,\n    hydrate_Counter => \
             crate::counter::hydrate,\n    hydrate_AuthSwitch => crate::auth::hydrate,\n}\n",
        )
        .unwrap();
        let declared = declared_exports(&dir).expect("an invocation is there");
        assert!(declared.start);
        assert_eq!(declared.islands, ["AuthSwitch", "Counter"]);
        // Without a `start:`, which is what `[ui] script = "wasm"` needs.
        std::fs::write(src.join("lib.rs"), "next_rust_wasm::islands! { hydrate_Counter => crate::hydrate }").unwrap();
        let declared = declared_exports(&dir).expect("an invocation is there");
        assert!(!declared.start);
        assert_eq!(declared.islands, ["Counter"]);

        // A crate whose doc comment explains the syntax, which is what a good
        // islands crate looks like — and what broke this before comments were
        // stripped: the `[` of `#[client` was read as the macro's opening
        // delimiter, everything to the next `}` became the body, and the one
        // real entry was lost. A perfectly wired crate then reported NR0313
        // against itself.
        std::fs::write(
            src.join("lib.rs"),
            "//! Each `hydrate_<Name>` entry is what `#[client(wasm = \"<Name>\")]` binds to.\n//! A `}` in prose, \
             and a start: that is only talk.\n#![no_std]\n/* block { comment */\nnext_rust_wasm::islands! {\n    \
             hydrate_Counter => crate::counter,\n}\n\n/// Trailing prose with islands! { hydrate_Ghost => x } in \
             it.\npub fn counter(cx: Ctx) { let s = \"}\"; }\n",
        )
        .unwrap();
        let declared = declared_exports(&dir).expect("an invocation is there");
        assert_eq!(declared.islands, ["Counter"], "prose is not code");
        assert!(!declared.start, "a `start:` in a comment is not a `start:`");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The attribute is still readable once comments are gone — the island
    /// name lives in a string literal, and blanking those would have made
    /// every project look as though it asked for nothing.
    #[test]
    fn code_is_told_from_prose() {
        let source = "/// Written `#[client(wasm = \"Ghost\")]` in prose.\n#[client(wasm = \"Real\")]\npub fn X() {}\n";
        assert_eq!(client_wasm_names(source), ["Real"]);
        // A `//` inside a string does not start a comment.
        let source = "let url = \"https://example.com\";\n#[client(wasm = \"Real\")]\n";
        assert_eq!(client_wasm_names(source), ["Real"]);
        // The case that made the framework's own website warn about four
        // islands it does not have: its documentation pages are Rust files
        // whose code samples are raw strings.
        let website = "pre![code![class(\"language-rust\"), r#\"#[client(wasm = \"AuthSwitch\")]\npub fn X() {}\"#]]";
        assert!(client_wasm_names(website).is_empty(), "a code sample is not a component");
        // …and a real attribute in the same file is still found, which is
        // what makes this worth doing rather than skipping the file.
        let mixed = format!("{website}\n#[client(wasm = \"Real\")]\npub fn Y() {{}}\n");
        assert_eq!(client_wasm_names(&mixed), ["Real"]);

        // Offsets survive blanking: that is what lets the name be read from
        // the real source after the structure was checked on the blanked copy.
        let source = "let s = \"a\";\n#[client(wasm = \"Ünïcode\")]\n";
        assert_eq!(blank_noncode(source).len(), source.len());
        assert_eq!(client_wasm_names(source), ["Ünïcode"]);
        assert!(!blank_noncode(source).contains("\"a\""), "string contents go");
        assert!(blank_noncode(source).contains("#[client(wasm = \""), "structure stays");
        // Nested block comments, a raw string with hashes, and things that
        // are not string prefixes at all.
        assert_eq!(blank_noncode("a/* x /* y */ z */b").replace(' ', ""), "ab");
        assert_eq!(blank_noncode("a/* never closed").replace(' ', ""), "a");
        assert_eq!(blank_noncode("r#\"a\"#suffix").replace(' ', ""), "r#\"\"#suffix");
        assert_eq!(blank_noncode("let bar = render();").replace(' ', ""), "letbar=render();");
        // Line structure is preserved, so a reader counting lines is not lied to.
        let multi = "//! doc\n/* block\nblock */\nr#\"raw\nraw\"#\n";
        assert_eq!(blank_noncode(multi).lines().count(), multi.lines().count());
    }

    /// The failure that needs no wasm configuration to reach, and had nothing
    /// anywhere to explain it: the attribute is written, the key is not.
    #[test]
    fn a_wasm_island_without_the_key_is_reported() {
        let dir = scratch("islands-off");
        std::fs::create_dir_all(dir.join("app")).unwrap();
        std::fs::write(dir.join("app/page.rs"), "#[client(wasm = \"Counter\")]\npub fn Counter() {}\n").unwrap();
        let config = Config { root: dir.clone(), ..Default::default() };
        assert!(!config.uses_wasm(), "nothing in the configuration asked for wasm");
        let found = preflight(&config);
        assert_eq!(codes(&found), [ISLANDS_OFF], "{found:?}");
        let line = cargo_line(&found[0]);
        for part in [ISLANDS_OFF, "Counter", "wasm_islands = true", "404", "help:"] {
            assert!(line.contains(part), "{part} is missing from: {line}");
        }
        // With the key on and a crate present, this one goes away.
        let mut on = config.clone();
        on.build.wasm_islands = true;
        assert!(!codes(&preflight(&on)).contains(&ISLANDS_OFF));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The source-level half of the same two checks the built module gets,
    /// answered before anything is compiled.
    #[test]
    fn preflight_reads_the_crate_before_the_compiler_does() {
        use next_rust_core::config::UiScript;
        let dir = scratch("preflight");
        std::fs::create_dir_all(dir.join("app")).unwrap();
        std::fs::create_dir_all(dir.join("islands/src")).unwrap();
        std::fs::write(dir.join("app/page.rs"), "#[client(wasm = \"Counter\")]\npub fn Counter() {}\n").unwrap();
        let mut config = Config { root: dir.clone(), ..Default::default() };
        config.build.wasm_islands = true;
        config.ui.script = UiScript::Wasm;

        // No crate: one diagnostic, naming both keys and the command that
        // writes the crate.
        let found = preflight(&config);
        assert_eq!(codes(&found), [NO_CRATE], "{found:?}");
        let line = cargo_line(&found[0]);
        assert!(line.contains("next-rust generate islands") && line.contains("wasm_islands"), "{line}");

        // A crate that is not a cdylib, declares nothing, and inherits a
        // workspace profile: each is its own line.
        std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"islands\"]\n").unwrap();
        std::fs::write(
            dir.join("islands/Cargo.toml"),
            "[package]\nname = \"islands\"\n\n[profile.release]\nopt-level = \"z\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("islands/src/lib.rs"), "#![no_std]\n").unwrap();
        let found = preflight(&config);
        assert_eq!(codes(&found), [NOT_CDYLIB, IGNORED_PROFILE, NO_DECLARATION], "{found:?}");

        // Everything right except the two wirings, which is the pair that
        // ships a page that renders and does nothing.
        std::fs::write(
            dir.join("islands/Cargo.toml"),
            "[workspace]\n[package]\nname = \"islands\"\n[lib]\ncrate-type = [\"cdylib\"]\n",
        )
        .unwrap();
        std::fs::write(dir.join("islands/src/lib.rs"), "next_rust_wasm::islands! { hydrate_Other => x }").unwrap();
        let found = preflight(&config);
        assert_eq!(codes(&found), [NO_START, UNKNOWN_ISLAND], "{found:?}");

        // And the wired crate says nothing at all.
        std::fs::write(
            dir.join("islands/src/lib.rs"),
            "next_rust_wasm::islands! { start: next_rust_ui_wasm::boot, hydrate_Counter => x }",
        )
        .unwrap();
        assert!(preflight(&config).is_empty(), "{:?}", preflight(&config));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `[client] runtime = "wasm"` is the one switch that needs nothing from
    /// the project: the module it ships is built into `next-rust-server`.
    /// Demanding an islands crate for it sent projects to write a crate that
    /// would have contributed nothing.
    #[test]
    fn the_wasm_runtime_asks_the_project_for_nothing() {
        let dir = scratch("runtime-only");
        std::fs::create_dir_all(dir.join("app")).unwrap();
        let config = Config {
            root: dir.clone(),
            client: next_rust_core::config::ClientConfig {
                runtime: next_rust_core::config::ClientRuntime::Wasm,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.uses_wasm());
        assert!(preflight(&config).is_empty(), "{:?}", preflight(&config));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn island_names_come_from_the_exports() {
        let wasm = module(&[
            ("memory", 2),
            ("alloc", KIND_FUNCTION),
            ("hydrate_Counter", KIND_FUNCTION),
            ("on_event", KIND_FUNCTION),
            ("hydrate_ThemeToggle", KIND_FUNCTION),
            // Not a function, and an empty name after the prefix: neither is an island.
            ("hydrate_Table", 1),
            ("hydrate_", KIND_FUNCTION),
            // Shared exports of the module's own protocol. They are functions
            // and they are exported, and they are not islands.
            ("start", KIND_FUNCTION),
            ("on_resume", KIND_FUNCTION),
            ("set_prop", KIND_FUNCTION),
        ]);
        assert_eq!(islands(&wasm).unwrap(), ["Counter", "ThemeToggle"]);
        let all = exports(&wasm).unwrap();
        let names: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.len(), 10);
        assert_eq!(names[0], "memory");
    }

    #[test]
    fn a_module_without_an_export_section_has_no_islands() {
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.push(1); // type section
        wasm.extend(leb(1));
        wasm.push(0);
        assert_eq!(exports(&wasm).unwrap(), Vec::<(String, u8)>::new());
        assert_eq!(islands(&wasm).unwrap(), Vec::<String>::new());
        // The header alone is a module too.
        assert_eq!(islands(b"\0asm\x01\0\0\0").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn broken_modules_are_errors_not_panics() {
        // Not wasm at all.
        assert!(islands(b"<!doctype html>").is_err());
        assert!(islands(b"").is_err());
        assert!(islands(b"\0asm\x02\0\0\0").is_err());
        // Every truncation of a good module either errors or reads nothing.
        let wasm = module(&[("hydrate_Counter", KIND_FUNCTION), ("alloc", KIND_FUNCTION)]);
        for len in 0..wasm.len() {
            match islands(&wasm[..len]) {
                Ok(names) => assert!(names.is_empty() || names == ["Counter"]),
                Err(message) => assert!(!message.is_empty()),
            }
        }
        // A section that claims more bytes than it has, and a count that
        // claims more exports than it has.
        let mut lying = b"\0asm\x01\0\0\0".to_vec();
        lying.push(EXPORT_SECTION);
        lying.extend(leb(64));
        lying.extend([1, 2, 3]);
        assert!(islands(&lying).is_err());
        let mut counted = b"\0asm\x01\0\0\0".to_vec();
        counted.push(EXPORT_SECTION);
        counted.extend(leb(1));
        counted.extend(leb(9));
        assert!(islands(&counted).is_err());
        // A LEB128 that never ends.
        let mut endless = b"\0asm\x01\0\0\0".to_vec();
        endless.push(EXPORT_SECTION);
        endless.extend([0x80; 12]);
        assert!(islands(&endless).is_err());
    }

    #[test]
    fn leb128_reads_what_it_writes() {
        for value in [0u64, 1, 63, 64, 127, 128, 255, 624_485, u32::MAX as u64, u64::MAX] {
            let bytes = leb(value);
            assert_eq!(Reader::new(&bytes).leb128(), Some(value), "{value}");
        }
        // Over-long encodings do not wrap around.
        assert_eq!(Reader::new(&[0x80; 10]).leb128(), None);
        assert_eq!(Reader::new(&[0xff; 11]).leb128(), None);
    }

    #[test]
    fn a_missing_target_is_recognised() {
        assert!(missing_target(
            "error[E0463]: can't find crate for `core`\n  = note: the `wasm32-unknown-unknown` target may not be \
             installed\n  = help: consider downloading the target with `rustup target add wasm32-unknown-unknown`"
        ));
        // The wording rustup and cargo have used for this over the years.
        assert!(missing_target(
            "error[E0463]: can't find crate for `std`\n  = note: the `wasm32-unknown-unknown` target may not be \
             installed"
        ));
        assert!(missing_target("error: the 'wasm32-unknown-unknown' target is not installed"));
        assert!(missing_target("help: consider downloading the target with `rustup target add wasm32-wasip1`"));
        // Ordinary compile errors in the author's own code are their bug, not
        // a missing target, and must stay hard failures.
        assert!(!missing_target("error[E0425]: cannot find value `x` in this scope"));
        assert!(!missing_target(
            "error[E0308]: mismatched types\n --> islands/src/lib.rs:4:5\n  = note: expected `u32`, found `&str`"
        ));
        assert!(!missing_target("error: could not compile `islands` (lib) due to 1 previous error"));
        assert!(!missing_target(""));
    }

    /// A scratch directory of this test's own.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nr-wasm-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `Config::uses_wasm()` decides whether a module is built at all. With
    /// every key at its default, nothing runs cargo and nothing is written —
    /// which is what keeps the wasm work off the path of projects that have
    /// not asked for it.
    #[test]
    fn nothing_happens_unless_a_key_asks_for_it() {
        let dir = scratch("off");
        let config = Config::default();
        assert!(!config.uses_wasm());
        assert!(build(&config, &dir.join("gen")).unwrap().is_empty());
        assert!(!dir.join("gen").exists(), "a build that does nothing writes nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `[client] runtime = "wasm"` alone builds nothing here: its module is
    /// the one `next-rust-server` carries. An empty result, no module, and —
    /// since nothing about this configuration is wrong — nothing said.
    #[test]
    fn a_framework_only_module_without_an_islands_crate_is_a_warning() {
        let dir = scratch("framework-only");
        let config = Config {
            root: dir.clone(),
            client: next_rust_core::config::ClientConfig {
                runtime: next_rust_core::config::ClientRuntime::Wasm,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.uses_wasm() && !config.build.wasm_islands);
        assert!(build(&config, &dir.join("gen")).unwrap().is_empty());
        assert!(!dir.join("gen").join("client").exists(), "no module, and no empty one either");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The framework's own wasm crate is found by walking up from this
    /// crate's source directory. Anything that is not there — which is every
    /// crate when the framework came from crates.io — resolves to `None` so
    /// the caller can warn instead of failing.
    #[test]
    fn framework_crates_are_found_beside_this_one() {
        assert!(sibling_crate(&RUNTIME_CRATES).is_some(), "next-rust-runtime is in this checkout");
        assert_eq!(sibling_crate(&["next-rust-not-a-real-crate"]), None);
        assert_eq!(sibling_crate(&[]), None);
    }

    /// The feature flags are not optional decoration. `rustc` emits
    /// `memory.copy` for `wasm32-unknown-unknown` without being asked, and a
    /// `wasm-opt` that is not told bulk memory is allowed rejects the whole
    /// module — measured on this project's islands crate as
    /// `[wasm-validator error in function 9] … memory.copy operations require
    /// bulk memory operations [--enable-bulk-memory-opt]`, exit status 1,
    /// after which the unoptimized 188,850-byte module shipped instead of the
    /// 163,244-byte one. There is no test here that runs the real binary —
    /// CI need not have binaryen, and a synthetic module small enough to
    /// build by hand is already too small for `-Oz` to shrink — so what is
    /// asserted is the command line, and `optimize()` warns loudly on every
    /// path where a `wasm-opt` that exists did not produce something smaller.
    #[test]
    fn the_optimizer_is_told_which_features_rustc_emits() {
        let mut config = Config::default();
        let args = opt_args(&config);
        assert_eq!(args[0], "-Oz");
        for feature in ["--enable-bulk-memory", "--enable-bulk-memory-opt", "--enable-sign-ext"] {
            assert!(args.contains(&feature), "{feature} is missing from {args:?}");
        }
        assert!(args.contains(&"--strip-debug"), "{args:?}");
        // `[build] wasm_keep_names` keeps the name section, and changes
        // nothing else about the command.
        config.build.wasm_keep_names = true;
        let kept = opt_args(&config);
        assert!(!kept.contains(&"--strip-debug"), "{kept:?}");
        assert_eq!(kept.len() + 1, args.len());
        for arg in &kept {
            assert!(args.contains(arg), "{arg} appeared only with wasm_keep_names");
        }
    }

    /// `[build] wasm_opt = false` was asked for, so it is silent and reports
    /// the module as unoptimized rather than pretending otherwise.
    #[test]
    fn turning_the_optimizer_off_leaves_the_module_alone() {
        let dir = scratch("no-opt");
        let wasm = dir.join("islands.wasm");
        std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
        let config = Config {
            build: next_rust_core::config::BuildConfig { wasm_opt: false, ..Default::default() },
            ..Default::default()
        };
        let (path, optimized) = optimize(&config, &wasm, &dir);
        assert_eq!(path, wasm);
        assert!(!optimized, "`optimized` must mean wasm-opt ran and helped, never that it was skipped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A tool's complaint has to survive `cargo:warning=`, which is one line.
    #[test]
    fn tool_output_is_flattened_for_a_cargo_warning() {
        let out = one_line("[wasm-validator error in function 9]\n unexpected false:\tmemory.copy\n\n(memory.copy\n");
        assert!(!out.contains('\n') && !out.contains('\t'), "{out}");
        assert!(out.contains("memory.copy operations") || out.contains("unexpected false: memory.copy"), "{out}");
        assert!(one_line(&"x ".repeat(400)).chars().count() <= 301);
        assert_eq!(one_line("  "), "");
    }

    /// The path prefixes are unit-separated, so a directory with a space in
    /// it does not turn into two flags.
    #[test]
    fn remapped_prefixes_survive_spaces_in_paths() {
        let flags = remap_flags(Path::new("/tmp/my islands/Cargo.toml")).expect("a manifest has a parent");
        let parts: Vec<&str> = flags.split('\u{1f}').collect();
        assert!(parts[0] == "--remap-path-prefix=/tmp/my islands=/next-rust/crate", "{parts:?}");
        assert!(parts.iter().all(|p| p.starts_with("--remap-path-prefix=")), "{parts:?}");
        assert!(!flags.contains('\n'));
    }

    /// The app's crate and the framework's contributors are held to different
    /// standards on purpose. The app's must be a `cdylib` — it is what the
    /// browser loads. The contributors must not be: they are `rlib`s linked
    /// into it, and requiring a `cdylib` of them made `[ui] script = "wasm"`
    /// report a crate that was sitting in the checkout as missing.
    #[test]
    fn the_app_crate_needs_a_cdylib_and_the_contributors_do_not() {
        let dir = scratch("cdylib");
        let manifest = dir.join("Cargo.toml");
        std::fs::write(&manifest, "[package]\nname = \"x\"\n").unwrap();
        assert!(!declares_cdylib(&manifest));
        assert!(is_library(&manifest), "an rlib is a perfectly good contributor");
        std::fs::write(&manifest, "[package]\nname = \"x\"\n[lib]\ncrate-type = [\"rlib\", \"cdylib\"]\n").unwrap();
        assert!(declares_cdylib(&manifest) && is_library(&manifest));
        std::fs::write(&manifest, "[package]\nname = \"x\"\n[[bin]]\nname = \"x\"\n").unwrap();
        assert!(!is_library(&manifest), "a binary contributes nothing to a wasm module");
        assert!(!declares_cdylib(&dir.join("nothing-here.toml")));
        assert!(!is_library(&dir.join("nothing-here.toml")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A diagnostic that names the wrong switch is worse than a vague one, so
    /// the message lists exactly the keys that asked for a module.
    #[test]
    fn the_warning_names_the_keys_that_asked() {
        use next_rust_core::config::{ClientRuntime, UiScript};
        let mut config = Config::default();
        config.ui.script = UiScript::Wasm;
        assert_eq!(asked_for_wasm(&config), "`[ui] script = \"wasm\"`");
        config.client.runtime = ClientRuntime::Wasm;
        assert_eq!(asked_for_wasm(&config), "`[client] runtime = \"wasm\"` and `[ui] script = \"wasm\"`");
        config.build.wasm_islands = true;
        assert_eq!(
            asked_for_wasm(&config),
            "`[build] wasm_islands = true`, `[client] runtime = \"wasm\"` and `[ui] script = \"wasm\"`"
        );
        // Never panics on the combination that cannot reach it.
        assert!(!asked_for_wasm(&Config::default()).is_empty());
    }

    /// A real wasm build needs a toolchain, a target and possibly the
    /// network, so what is asserted here is the part that is pure: the same
    /// island list gives the same loader and the same shims, byte for byte,
    /// however often it is generated. See the module docs for what this does
    /// and does not prove about the `.wasm` itself.
    #[test]
    fn the_glue_is_the_same_every_time() {
        let shims = |names: &[&str]| -> Vec<(String, String)> {
            names
                .iter()
                .map(|name| {
                    (
                        format!("{name}.js"),
                        format!("import{{h}}from\"../__nrw.js\";export const hydrate=(e,p)=>h(\"{name}\",e,p);"),
                    )
                })
                .collect()
        };
        let wasm = module(&[
            ("hydrate_ThemeToggle", KIND_FUNCTION),
            ("alloc", KIND_FUNCTION),
            ("hydrate_Counter", KIND_FUNCTION),
        ]);
        let first = islands(&wasm).unwrap();
        for _ in 0..4 {
            assert_eq!(islands(&wasm).unwrap(), first);
        }
        // Sorted, so the order the exports happen to be in does not leak into
        // the file names.
        assert_eq!(first, ["Counter", "ThemeToggle"]);
        let names: Vec<&str> = first.iter().map(String::as_str).collect();
        assert_eq!(shims(&names), shims(&names));
        assert_eq!(LOADER, LOADER);
        // Compressed sizes are a pure function of the bytes too: the report
        // does not wobble between runs.
        let big = vec![7u8; 4096];
        assert_eq!(compressed_sizes(&big), compressed_sizes(&big));
        let [br, gz] = compressed_sizes(&big);
        assert!(br.is_some_and(|n| n < big.len()) && gz.is_some_and(|n| n < big.len()));
        // Nothing under a kilobyte is measured, exactly as for embedded files.
        assert_eq!(compressed_sizes(&[0u8; 512]), [None, None]);
    }
}
