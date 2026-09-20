//! Turning WebAssembly on, and checking that it is on properly.
//!
//! Three keys in `next-rust.toml` move browser logic from JavaScript to Rust
//! (`[build] wasm_islands`, `[client] runtime`, `[ui] script`), and two of
//! them need a crate that does not exist until somebody writes it. Writing it
//! by hand means getting four separate things right — a `cdylib`, an empty
//! `[workspace]` table so cargo reads the size profile, the right
//! dependencies, and an `islands!` invocation whose entry names match the
//! `#[client(wasm = "…")]` attributes in the app — and every one of them
//! fails *quietly*: the page renders, looks finished, and does nothing.
//!
//! So the CLI writes it. [`scaffold`] produces a crate that is correct on the
//! first build, and [`checks`] re-reads the same things afterwards for
//! `next-rust doctor`, sharing the `NR03xx` diagnostics with the build script
//! rather than inventing a second set of opinions.

use std::path::{Path, PathBuf};
use std::process::Command;

use next_rust_core::{Config, Diagnostic};

use crate::templates::FrameworkSource;

/// The islands crate's directory name. It is what `next-rust-build` looks
/// for, next to `next-rust.toml`.
pub const DIR: &str = "islands";

/// The demo route `--demo` writes, so a scaffolded project is end-to-end
/// correct rather than a crate with nothing pointing at it.
pub const DEMO_ROUTE: &str = "wasm";

/// What a scaffolded islands crate should contain.
#[derive(Clone, Copy, Default)]
pub struct Options {
    /// Take the shipped components' behaviour from Rust too
    /// (`[ui] script = "wasm"`): one dependency and a `start:` entry.
    pub ui: bool,
    /// Write the example island and the page that renders it.
    pub demo: bool,
}

/// Dependency lines for the islands crate, from wherever the app takes the
/// framework.
///
/// `next-rust-wasm` and `next-rust-ui-wasm` are not published to crates.io —
/// they are not in the release workflow's crate lists — so a registry
/// requirement would not resolve. A git dependency does, and cargo finds both
/// packages by name inside the one repository.
fn dependency(source: &FrameworkSource, crate_name: &str) -> String {
    match source {
        FrameworkSource::Path(p) => {
            let p = p.trim_end_matches('/');
            // The checkout is named relative to the app root, but this line is
            // written into `islands/Cargo.toml`, one directory deeper: a
            // relative path has to climb back out first. Absolute paths mean
            // the same thing from either directory.
            let from_islands = if Path::new(p).is_absolute() {
                p.to_string()
            } else if p == "." {
                "..".to_string()
            } else {
                format!("../{p}")
            };
            format!("{{ path = \"{from_islands}/crates/{crate_name}\" }}")
        }
        FrameworkSource::Registry | FrameworkSource::Git => format!("{{ git = \"{}\" }}", crate::REPO_URL),
    }
}

/// Where this project takes the framework from, read back out of its own
/// `Cargo.toml`, so the islands crate depends on the same copy.
///
/// Getting this wrong is not a style problem: an islands crate that pulls
/// `next-rust-wasm` from git while the app uses a local checkout links two
/// different versions of the host ABI, and the opcode numbers are the ABI.
pub fn framework_source(root: &Path) -> FrameworkSource {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    let Some(line) = text.lines().find(|l| l.trim_start().starts_with("next-rust ")) else {
        return FrameworkSource::Registry;
    };
    if let Some(at) = line.find("path = \"")
        && let Some(end) = line[at + 8..].find('"')
    {
        let path = &line[at + 8..at + 8 + end];
        // The app names `<checkout>/crates/next-rust`; the islands crate
        // names sibling crates in the same checkout.
        let checkout = Path::new(path).parent().and_then(Path::parent).unwrap_or(Path::new(path));
        return FrameworkSource::Path(checkout.to_string_lossy().into_owned());
    }
    if line.contains("git = \"") { FrameworkSource::Git } else { FrameworkSource::Registry }
}

/// The package name from a project's `Cargo.toml`, for naming the islands
/// crate after it.
pub fn package_name(root: &Path) -> String {
    std::fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|l| l.trim_start().starts_with("name"))
                .and_then(|l| l.split('"').nth(1).map(str::to_owned))
        })
        .unwrap_or_else(|| "app".to_owned())
}

/// The islands crate's manifest.
///
/// Two things in it are load-bearing and both are invisible when wrong:
///
/// - **`crate-type = ["cdylib"]`.** Without it cargo builds an rlib, the
///   build finds no `.wasm`, and the page falls back to markup that does
///   nothing (`NR0302`).
/// - **the empty `[workspace]` table.** Cargo reads `[profile.*]` from the
///   workspace root *only*. A crate that is a member of the app's workspace
///   silently gets the app's release profile instead of this one, and the
///   module comes out at roughly twice the size (`NR0306`).
pub fn cargo_toml(package: &str, source: &FrameworkSource, options: Options) -> String {
    let ui_dep = match options.ui {
        true => format!("\nnext-rust-ui-wasm = {}", dependency(source, "next-rust-ui-wasm")),
        false => String::new(),
    };
    format!(
        r#"# The WebAssembly module this site loads.
#
# One module is shared by the whole site: the app's islands and, with
# `[ui] script = "wasm"`, the shipped components' behaviour all link into this
# crate. `next-rust build` compiles it for wasm32-unknown-unknown and serves
# the result from /_next-rust/client/__nrw/islands.wasm.
#
# Two lines below are load-bearing and fail quietly when they are wrong:
#
#   [lib] crate-type = ["cdylib"]   without it cargo builds an rlib, there is
#                                   no module, and every island is inert
#   [workspace]                     cargo reads [profile.*] from the workspace
#                                   root only, so without this empty table the
#                                   crate inherits the app's profile and the
#                                   module is about twice the size
#
# `next-rust doctor` checks both.
[workspace]

[package]
name = "{package}"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
next-rust-wasm = {wasm}{ui_dep}

# Size is the whole point: the browser downloads this.
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#,
        wasm = dependency(source, "next-rust-wasm"),
    )
}

/// The islands crate's source.
///
/// The `islands!` invocation is the module's export list and the contract
/// with the server: each `hydrate_<Name>` entry is what a
/// `#[client(wasm = "<Name>")]` component binds to, spelled exactly, and
/// case-sensitively. A name that does not match is `NR0313`.
pub fn lib_rs(options: Options) -> String {
    let start = match options.ui {
        // `start` runs once after the module is instantiated. It is what
        // `[ui] script = "wasm"` needs: the components' behaviour is
        // delegated listeners on the document, not something that hydrates
        // per island, so there is nowhere else to put it. Without this entry
        // every component renders correctly and stays inert (`NR0310`).
        true => "    start: next_rust_ui_wasm::boot,\n",
        false => "",
    };
    let island = match options.demo {
        true => "    hydrate_Counter => crate::counter,\n",
        false => "    // hydrate_Counter => crate::counter,\n",
    };
    let body = match options.demo {
        true => {
            r#"
/// The island behind `#[client(wasm = "Counter")]`.
///
/// `cx.root` is the `<nr-island>` element the server rendered, and
/// `cx.props` carries the component's arguments, keyed by their Rust names —
/// so `count: i32` on the server is `k!("count")` here. Props are public:
/// they are in the HTML, and the module is downloadable. Secrets stay on the
/// server.
pub fn counter(cx: Ctx) {
    let output = cx.root.query("[data-count]");
    let mut count = cx.props.i64(next_rust_wasm::k!("count")).unwrap_or(0);
    cx.root.query("[data-step]").on("click", move |event| {
        count += event.current().attr("data-step").and_then(|s| s.as_str().parse::<i64>().ok()).unwrap_or(1);
        output.set_text(&num::number_to_string(count as f64));
    });
}
"#
        }
        false => {
            r#"
// Write one `fn(Ctx)` per island and list it above. `cx.root` is the
// `<nr-island>` element the server rendered and `cx.props` carries the
// component's arguments, keyed by their Rust names.
//
// pub fn counter(cx: Ctx) {
//     cx.root.query("button").on("click", move |_| log("clicked"));
// }
"#
        }
    };
    format!(
        r#"//! This site's browser logic, compiled to one WebAssembly module.
//!
//! Everything here runs in the browser and nothing here runs on the server,
//! so it is `no_std`: no files, no sockets, no threads. Reach the page
//! through `next_rust_wasm::prelude`.
//!
//! The `islands!` invocation below is the module's export list. Each
//! `hydrate_<Name>` entry is what `#[client(wasm = "<Name>")]` binds to on
//! the server, spelled exactly and case-sensitively — a name that does not
//! match is a component that renders and never reacts, which the build
//! reports as `NR0313`.

#![no_std]

extern crate alloc;

#[allow(unused_imports)]
use next_rust_wasm::prelude::*;

next_rust_wasm::islands! {{
{start}{island}}}
{body}"#
    )
}

/// The page the demo island is rendered on.
pub fn demo_page() -> String {
    r#"use next_rust::prelude::*;

/// A button whose behaviour is Rust compiled to WebAssembly.
///
/// The component itself is server-rendered like any other: `#[client]` wraps
/// it in `<nr-island>` and serialises its arguments into the HTML as props.
/// `wasm = "Counter"` is the only difference — it points the island at the
/// `hydrate_Counter` export of `islands/`, so the click handler is Rust
/// rather than JavaScript.
///
/// The markup below is what a visitor sees before the module loads, and what
/// they keep if it never does. Render something that already makes sense.
#[client(wasm = "Counter")]
pub fn Counter(count: i32) -> impl View {
    div![
        class("flex items-center gap-4"),
        button![
            r#type("button"),
            data("step", "1"),
            class("rounded-lg border px-4 py-2"),
            "Add one",
        ],
        output![data("count", ""), class("text-xl font-semibold tabular-nums"), count.to_string()],
    ]
}

pub fn Page() -> impl View {
    main![
        class("mx-auto max-w-2xl px-6 py-16"),
        h1![class("text-3xl font-bold"), "WebAssembly island"],
        p![class("mt-2 text-zinc-500"), "The button below is handled by Rust, in the browser."],
        div![class("mt-8"), Counter(0)],
    ]
}
"#
    .to_owned()
}

/// Write the islands crate (and, with [`Options::demo`], the page that
/// renders its island) under `root`.
///
/// Returns the files written, relative to `root`. Nothing is overwritten: a
/// path that already exists is left alone and reported, because this command
/// is as likely to be run on a project that has half of it as on an empty one.
pub fn scaffold(
    root: &Path,
    package: &str,
    source: &FrameworkSource,
    options: Options,
    app_dir: &Path,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut files: Vec<(PathBuf, String)> = vec![
        (PathBuf::from(DIR).join("Cargo.toml"), cargo_toml(&format!("{package}-islands"), source, options)),
        (PathBuf::from(DIR).join("src").join("lib.rs"), lib_rs(options)),
    ];
    if options.demo {
        let page = app_dir.join(DEMO_ROUTE).join("page.rs");
        let page = page.strip_prefix(root).unwrap_or(&page).to_path_buf();
        files.push((page, demo_page()));
    }
    let (mut written, mut skipped) = (Vec::new(), Vec::new());
    for (path, contents) in files {
        let full = root.join(&path);
        let shown = path.display().to_string().replace('\\', "/");
        if full.exists() {
            skipped.push(shown);
            continue;
        }
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&full, contents).map_err(|e| format!("{}: {e}", full.display()))?;
        written.push(shown);
    }
    Ok((written, skipped))
}

// ---------------------------------------------------------------------------
// next-rust.toml
// ---------------------------------------------------------------------------

/// Set `key = value` under `[section]`, adding the section when it is absent
/// and replacing a commented-out line when there is one.
///
/// `None` when the file already says exactly that, so a caller can tell
/// "changed" from "was already right" without diffing.
///
/// Deliberately a text edit and not a TOML round-trip: `next-rust.toml` is a
/// file people read, and the generated one is most of the documentation a
/// small project has. Reserialising it would cost every comment in it.
pub fn set_key(text: &str, section: &str, key: &str, value: &str) -> Option<String> {
    let header = format!("[{section}]");
    let line = format!("{key} = {value}");
    let mut out: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut done = false;
    let mut inserted_at: Option<usize> = None;
    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') {
            // Leaving the section without having found the key: put it right
            // under the header, where it reads as part of that section
            // rather than orphaned at the end of the file.
            if in_section && !done {
                out.insert(inserted_at.unwrap_or(out.len()), line.clone());
                done = true;
            }
            in_section = trimmed == header;
            if in_section {
                inserted_at = Some(out.len() + 1);
            }
        } else if in_section && !done {
            let body = trimmed.trim_start_matches('#').trim_start();
            if body.starts_with(key)
                && body[key.len()..].trim_start().starts_with('=')
                && body[key.len()..].trim_start()[1..].trim().split('#').next().unwrap_or_default().trim() != value
            {
                out.push(line.clone());
                done = true;
                continue;
            }
            if body.starts_with(key) && body[key.len()..].trim_start().starts_with('=') {
                // Already set to exactly this.
                return None;
            }
        }
        out.push(raw.to_owned());
    }
    if in_section && !done {
        out.insert(inserted_at.unwrap_or(out.len()), line.clone());
        done = true;
    }
    if !done {
        if !out.last().is_some_and(|l| l.trim().is_empty()) {
            out.push(String::new());
        }
        out.push(header);
        out.push(line);
    }
    let mut joined = out.join("\n");
    if text.ends_with('\n') || text.is_empty() {
        joined.push('\n');
    }
    Some(joined)
}

/// The keys a set of options turns on, in the order a reader meets them.
pub fn keys(options: Options) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut out = vec![("build", "wasm_islands", "true")];
    if options.ui {
        out.push(("ui", "script", "\"wasm\""));
    }
    out
}

// ---------------------------------------------------------------------------
// next-rust doctor
// ---------------------------------------------------------------------------

/// One line of `next-rust doctor`'s WebAssembly section.
pub struct Check {
    /// What was checked, as a reader would say it.
    pub label: String,
    /// `None` when it passed.
    pub problem: Option<Diagnostic>,
    /// A problem that costs size or time but not behaviour.
    pub advisory: bool,
}

impl Check {
    fn ok(label: impl Into<String>) -> Self {
        Check { label: label.into(), problem: None, advisory: false }
    }

    fn bad(label: impl Into<String>, problem: Diagnostic) -> Self {
        Check { label: label.into(), problem: Some(problem), advisory: false }
    }
}

/// Whether this project has anything to do with WebAssembly at all.
///
/// A project that has never heard of it should not read a section about it,
/// and a project that wrote `#[client(wasm = "…")]` and forgot the key is
/// exactly the one that needs to.
pub fn relevant(config: &Config) -> bool {
    config.uses_wasm() || config.root.join(DIR).join("Cargo.toml").is_file()
}

/// Everything `next-rust doctor` can say about a project's WebAssembly setup
/// without building it.
///
/// The three prerequisites first — the target, the optimizer, the crate —
/// then [`next_rust_build::wasm_islands::preflight`], which is the same set
/// of `NR03xx` checks the build script runs, so the two can never disagree.
pub fn checks(config: &Config) -> Vec<Check> {
    let mut out = Vec::new();
    let target = installed_targets();
    match target {
        Some(list) if list.iter().any(|t| t == next_rust_build::wasm_islands::WASM_TARGET) => {
            out.push(Check::ok(format!("{} is installed", next_rust_build::wasm_islands::WASM_TARGET)));
        }
        Some(_) => out.push(Check::bad(
            format!("{} is not installed", next_rust_build::wasm_islands::WASM_TARGET),
            next_rust_build::wasm_islands::missing_target_diagnostic(),
        )),
        // No rustup (a distribution toolchain, a container). The target may
        // well be there; saying it is missing would be a guess, and a guess
        // that sends someone to install rustup is worse than silence.
        None => {}
    }
    if config.build.wasm_opt {
        match tool_version("wasm-opt") {
            Some(v) => out.push(Check::ok(format!("wasm-opt: {v}"))),
            None => out.push(Check {
                label: "wasm-opt is not on PATH".into(),
                problem: Some(next_rust_build::wasm_islands::no_wasm_opt(0)),
                // Size, not behaviour: a module without it works.
                advisory: true,
            }),
        }
    }
    let manifest = config.root.join(DIR).join("Cargo.toml");
    if manifest.is_file() {
        out.push(Check::ok(format!("islands crate: {}", manifest.display())));
    }
    for problem in next_rust_build::wasm_islands::preflight(config) {
        out.push(Check::bad(problem.title.clone(), problem));
    }
    if out.iter().all(|c| c.problem.is_none()) {
        out.push(Check::ok(format!("WebAssembly: {}", enabled(config))));
    }
    out
}

/// The switches that are on, as a reader would name them.
fn enabled(config: &Config) -> String {
    let on: Vec<&str> = [
        (config.build.wasm_islands, "[build] wasm_islands"),
        (config.client.runtime.is_wasm(), "[client] runtime"),
        (config.ui.script.is_wasm(), "[ui] script"),
    ]
    .iter()
    .filter(|(set, _)| *set)
    .map(|(_, name)| *name)
    .collect();
    if on.is_empty() { "nothing is switched on".to_owned() } else { on.join(", ") }
}

/// The targets rustup has installed, or `None` when there is no rustup to ask.
fn installed_targets() -> Option<Vec<String>> {
    let out = Command::new("rustup").args(["target", "list", "--installed"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_owned()).collect())
}

/// A tool's `--version`, or `None` when it is not on `PATH`.
fn tool_version(tool: &str) -> Option<String> {
    let out = Command::new(tool).arg("--version").output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().lines().next().unwrap_or("").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest carries the two settings that are invisible when wrong,
    /// and the crate name follows the package so two projects in one
    /// directory tree do not collide.
    #[test]
    fn the_manifest_has_the_two_lines_that_fail_quietly() {
        let toml = cargo_toml("my-app-islands", &FrameworkSource::Git, Options::default());
        assert!(toml.contains("crate-type = [\"cdylib\"]"), "{toml}");
        assert!(toml.contains("\n[workspace]\n"), "cargo reads profiles from the workspace root only");
        assert!(toml.contains("[profile.release]") && toml.contains("opt-level = \"z\""));
        assert!(toml.contains("name = \"my-app-islands\""));
        // The UI kit is a dependency only when it is asked for: an unused one
        // would be linked into every visitor's download.
        assert!(!toml.contains("next-rust-ui-wasm"));
        let with_ui = cargo_toml("x", &FrameworkSource::Git, Options { ui: true, demo: false });
        assert!(with_ui.contains("next-rust-ui-wasm = { git = "), "{with_ui}");
        // A local checkout gets path dependencies, with no trailing slash
        // doubled.
        let local = cargo_toml("x", &FrameworkSource::Path("/tmp/next-rust/".into()), Options { ui: true, demo: true });
        assert!(local.contains("path = \"/tmp/next-rust/crates/next-rust-wasm\""), "{local}");
        assert!(local.contains("path = \"/tmp/next-rust/crates/next-rust-ui-wasm\""), "{local}");
        // A relative checkout is named from the app root, but this file lives
        // in `islands/`: without the extra `..` cargo looks for
        // `<app>/crates/...` and the whole build fails to resolve.
        let rel = cargo_toml("x", &FrameworkSource::Path("..".into()), Options { ui: true, demo: true });
        assert!(rel.contains("path = \"../../crates/next-rust-wasm\""), "{rel}");
        assert!(rel.contains("path = \"../../crates/next-rust-ui-wasm\""), "{rel}");
        let here = cargo_toml("x", &FrameworkSource::Path(".".into()), Options::default());
        assert!(here.contains("path = \"../crates/next-rust-wasm\""), "{here}");
    }

    /// `start:` is what `[ui] script = "wasm"` needs and the one entry whose
    /// absence produces a page that renders perfectly and does nothing.
    #[test]
    fn the_source_wires_exactly_what_was_asked_for() {
        let plain = lib_rs(Options::default());
        assert!(plain.contains("#![no_std]") && plain.contains("next_rust_wasm::islands! {"));
        assert!(!plain.contains("start: next_rust_ui_wasm::boot"), "{plain}");
        // Nothing is declared, and the entry is shown commented so the shape
        // is there to copy.
        assert!(plain.contains("// hydrate_Counter =>"));

        let full = lib_rs(Options { ui: true, demo: true });
        assert!(full.contains("start: next_rust_ui_wasm::boot,"), "{full}");
        assert!(full.contains("hydrate_Counter => crate::counter,") && full.contains("pub fn counter(cx: Ctx)"));
        // The demo island and the component that renders it name the same
        // island, which is the pairing NR0313 and NR0314 exist to catch.
        assert!(demo_page().contains("#[client(wasm = \"Counter\")]"));
        assert!(full.contains("hydrate_Counter"));
    }

    /// Editing `next-rust.toml` must not cost the comments in it: the
    /// generated file is most of the documentation a small project has.
    #[test]
    fn a_key_is_set_in_place_and_keeps_the_file_readable() {
        let text = "# Next Rust configuration\n\n[app]\ndirectory = \"app\"\n\n[server]\nport = 3000\n";
        let out = set_key(text, "build", "wasm_islands", "true").unwrap();
        assert!(out.starts_with("# Next Rust configuration\n"), "{out}");
        assert!(out.contains("[build]\nwasm_islands = true"), "{out}");
        assert!(out.contains("[app]\ndirectory = \"app\"") && out.contains("port = 3000"));
        assert!(out.ends_with('\n'));

        // An existing section gains the key directly under its header, not at
        // the end of the file where it would read as part of the next one.
        let text = "[build]\nminify_js = true\n\n[server]\nport = 3000\n";
        let out = set_key(text, "build", "wasm_islands", "true").unwrap();
        assert_eq!(out, "[build]\nwasm_islands = true\nminify_js = true\n\n[server]\nport = 3000\n");

        // A commented-out line is the one to replace: it is what the starter
        // template writes, and leaving it would produce the key twice.
        let text = "[ui]\n# script = \"js\"     # \"wasm\": …\n";
        let out = set_key(text, "ui", "script", "\"wasm\"").unwrap();
        assert_eq!(out, "[ui]\nscript = \"wasm\"\n");

        // Already right: nothing to write, and the caller can say so.
        assert_eq!(set_key("[build]\nwasm_islands = true\n", "build", "wasm_islands", "true"), None);
        // A value that differs is replaced rather than duplicated.
        let out = set_key("[build]\nwasm_islands = false\n", "build", "wasm_islands", "true").unwrap();
        assert_eq!(out, "[build]\nwasm_islands = true\n");
        // An empty file, and one without a trailing newline.
        assert_eq!(set_key("", "build", "wasm_islands", "true").unwrap(), "\n[build]\nwasm_islands = true\n");
        assert_eq!(set_key("[app]", "ui", "script", "\"wasm\"").unwrap(), "[app]\n\n[ui]\nscript = \"wasm\"");
    }

    /// The keys each option turns on, which is what the command prints and
    /// what it writes.
    #[test]
    fn the_options_name_their_keys() {
        assert_eq!(keys(Options::default()), [("build", "wasm_islands", "true")]);
        assert_eq!(keys(Options { ui: true, demo: false }).len(), 2);
        assert!(keys(Options { ui: true, demo: false }).contains(&("ui", "script", "\"wasm\"")));
    }

    /// A project that never mentioned WebAssembly is not shown a section
    /// about it; one that wrote the attribute and forgot the key is.
    #[test]
    fn doctor_only_speaks_when_it_is_relevant() {
        let dir = std::env::temp_dir().join(format!("nr-wasm-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("app")).unwrap();
        let mut config = Config { root: dir.clone(), ..Default::default() };
        assert!(!relevant(&config));
        config.build.wasm_islands = true;
        assert!(relevant(&config));
        // Or because the crate is there, whatever the keys say.
        let mut off = Config { root: dir.clone(), ..Default::default() };
        off.build.wasm_islands = false;
        std::fs::create_dir_all(dir.join(DIR)).unwrap();
        std::fs::write(dir.join(DIR).join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert!(relevant(&off));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The scaffold refuses to overwrite: this command is as likely to meet a
    /// project that has half of it as an empty one.
    #[test]
    fn scaffolding_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("nr-wasm-scaffold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app");
        let options = Options { ui: true, demo: true };
        let (written, skipped) = scaffold(&dir, "my-app", &FrameworkSource::Git, options, &app).unwrap();
        assert_eq!(written, ["islands/Cargo.toml", "islands/src/lib.rs", "app/wasm/page.rs"]);
        assert!(skipped.is_empty());
        assert!(dir.join("islands/src/lib.rs").is_file());
        let (written, skipped) = scaffold(&dir, "my-app", &FrameworkSource::Git, options, &app).unwrap();
        assert!(written.is_empty());
        assert_eq!(skipped.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
