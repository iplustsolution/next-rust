//! Build-time code generation for Next Rust.
//!
//! Add to your application's `build.rs`:
//!
//! ```ignore
//! fn main() {
//!     next_rust_build::generate();
//! }
//! ```
//!
//! and to `src/main.rs`:
//!
//! ```ignore
//! next_rust::app!();
//! ```
//!
//! The generator scans the configured app directory, validates it, analyzes
//! the signatures of special files and writes
//! `$OUT_DIR/next_rust_routes.rs`, which declares every special file as a
//! module (via `#[path]`) and builds the route table. Generated code is a
//! build artifact; it is never checked in or edited.

#![forbid(unsafe_code)]
// Diagnostics are large but only produced on the (cold) error path.
#![allow(clippy::result_large_err)]

pub mod analyze;
pub mod class_names;
pub mod codegen;
pub mod css_usage;
pub mod js_classes;
pub mod js_minify;
pub mod report;
pub mod tailwind;

use std::path::{Path, PathBuf};

pub use codegen::{AnalyzedRoute, CodegenOptions, Project, analyze_project, generate_code, generate_code_with};
use next_rust_core::{Config, Diagnostic};
pub use report::BuildReport;

/// Build-time plugin hooks.
pub trait BuildPlugin {
    fn name(&self) -> &str;

    /// Inspect or adjust the analyzed project (add diagnostics, change
    /// rendering modes, ...).
    fn on_project(&self, _project: &mut Project) {}

    /// Extra Rust code appended to the generated file.
    fn extra_code(&self, _project: &Project) -> Option<String> {
        None
    }
}

/// Adjusts the `[tailwind]` configuration from `build.rs`.
type TailwindSetup = Box<dyn FnOnce(&mut next_rust_core::config::TailwindConfig)>;

/// Configurable generator.
#[derive(Default)]
pub struct Generator {
    plugins: Vec<Box<dyn BuildPlugin>>,
    manifest_dir: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    tailwind: Option<TailwindSetup>,
}

impl Generator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plugin(mut self, plugin: impl BuildPlugin + 'static) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Override `CARGO_MANIFEST_DIR` (tests, custom tooling).
    pub fn manifest_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.manifest_dir = Some(dir.into());
        self
    }

    /// Override `OUT_DIR`.
    pub fn out_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(dir.into());
        self
    }

    /// Configure Tailwind CSS in Rust instead of `[tailwind]` in
    /// `next-rust.toml` (applied on top of it). In `build.rs`:
    ///
    /// ```no_run
    /// next_rust_build::Generator::new()
    ///     .tailwind(|tw| {
    ///         tw.enabled = true;
    ///         tw.theme.insert("color-brand".into(), "#f26b2a".into());
    ///         tw.utilities.insert("btn".into(), "rounded-lg bg-brand px-4 py-2".into());
    ///     })
    ///     .run();
    /// ```
    pub fn tailwind(mut self, setup: impl FnOnce(&mut next_rust_core::config::TailwindConfig) + 'static) -> Self {
        self.tailwind = Some(Box::new(setup));
        self
    }

    /// Run inside a build script: prints `cargo:` directives and exits the
    /// process with a readable report on errors.
    pub fn run(self) {
        match self.try_run() {
            Ok(project) => {
                for d in project.diagnostics.warnings() {
                    println!(
                        "cargo:warning=[{}] {}{}",
                        d.code,
                        d.title,
                        d.locations.first().map(|l| format!(" ({})", l.display())).unwrap_or_default()
                    );
                }
            }
            Err(report) => {
                eprintln!("\n{report}");
                println!("cargo:warning=Next Rust: route generation failed (see the error output above)");
                std::process::exit(1);
            }
        }
    }

    /// Generate without printing or exiting. Returns the rendered error
    /// report on failure.
    pub fn try_run(mut self) -> Result<Project, String> {
        let manifest_dir = self
            .manifest_dir
            .or_else(|| std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from))
            .ok_or("CARGO_MANIFEST_DIR is not set; call generate() from build.rs")?;
        let out_dir =
            self.out_dir.or_else(|| std::env::var_os("OUT_DIR").map(PathBuf::from)).ok_or("OUT_DIR is not set")?;

        let mut config = Config::discover(&manifest_dir).map_err(|e| e.to_string())?;
        if let Some(setup) = self.tailwind.take() {
            setup(&mut config.tailwind);
        }
        emit_rerun(&config, &manifest_dir);

        let mut project = analyze_project(&config);
        for d in config.tailwind_diagnostics() {
            project.diagnostics.push(d);
        }
        for p in &self.plugins {
            p.on_project(&mut project);
        }
        if project.has_errors() {
            let color = std::env::var_os("NO_COLOR").is_none();
            return Err(project.diagnostics.errors().map(|d| d.render(color)).collect::<Vec<_>>().join("\n"));
        }
        // Release builds ship minified HTML; `NEXT_RUST_MINIFY=0|1` overrides.
        let minify_html = match std::env::var("NEXT_RUST_MINIFY").ok().as_deref() {
            Some("0") => false,
            Some(_) => true,
            None => std::env::var("PROFILE").is_ok_and(|p| p == "release"),
        };
        println!("cargo:rerun-if-env-changed=NEXT_RUST_MINIFY");
        // Release builds carry their config and static files, so the binary
        // is the whole deployment; `NEXT_RUST_EMBED=0|1` overrides.
        let embed_files = match std::env::var("NEXT_RUST_EMBED").ok().as_deref() {
            Some("0") => false,
            Some(_) => true,
            None => std::env::var("PROFILE").is_ok_and(|p| p == "release"),
        };
        println!("cargo:rerun-if-env-changed=NEXT_RUST_EMBED");
        if embed_files {
            for (_, dir) in codegen::embedded_dirs(&config) {
                // Cargo scans existing directories for changes; a missing
                // path would rerun the build script on every build.
                if dir.is_dir() {
                    println!("cargo:rerun-if-changed={}", dir.display());
                }
            }
        }
        // Release builds leave unused rules out of hand-written CSS: the CSS
        // macros read the names used in the project from this file.
        let usage_file = out_dir.join(css_usage::FILE);
        let prune_css = match std::env::var("NEXT_RUST_PRUNE_CSS").ok().as_deref() {
            Some("0") => false,
            Some(_) => true,
            None => config.assets.prune_css && std::env::var("PROFILE").is_ok_and(|p| p == "release"),
        };
        println!("cargo:rerun-if-env-changed=NEXT_RUST_PRUNE_CSS");
        if prune_css {
            let names = css_usage::collect(&config);
            write_if_changed(&usage_file, names.join("\n").as_bytes()).map_err(|e| e.to_string())?;
        } else {
            let _ = std::fs::remove_file(&usage_file);
        }
        let tailwind = config.tailwind.enabled;
        // Release builds give Tailwind and UI component classes short random
        // names; `NEXT_RUST_MINIFY_CLASSES=0|1` overrides.
        let minify_classes = match std::env::var("NEXT_RUST_MINIFY_CLASSES").ok().as_deref() {
            Some("0") => false,
            Some(_) => true,
            None => config.tailwind.minify_classes && std::env::var("PROFILE").is_ok_and(|p| p == "release"),
        };
        println!("cargo:rerun-if-env-changed=NEXT_RUST_MINIFY_CLASSES");
        println!("cargo:rerun-if-env-changed=NEXT_RUST_CLASS_SEED");
        // The UI components' classes are shortened too, when the app uses them.
        let components =
            if class_names::uses_components(&config) { class_names::component_classes() } else { Vec::new() };
        // Release builds leave out generated-looking classes nothing knows;
        // `NEXT_RUST_DROP_CLASSES=0|1` overrides.
        let drop_classes = match std::env::var("NEXT_RUST_DROP_CLASSES").ok().as_deref() {
            Some("0") => false,
            Some(_) => true,
            None => config.build.drop_unused_classes && std::env::var("PROFILE").is_ok_and(|p| p == "release"),
        };
        println!("cargo:rerun-if-env-changed=NEXT_RUST_DROP_CLASSES");
        let mut report = report::BuildReport::default();
        let naming = if tailwind {
            generate_tailwind(&config, &out_dir, minify_classes, &components, &mut report)?
        } else if minify_classes && !components.is_empty() {
            // No Tailwind: only the component classes.
            Some(name_classes("", &config, &out_dir, &components, &mut report)?)
        } else {
            None
        };
        if drop_classes {
            let known = class_names::known_classes("", &config, &components, tailwind.then_some(out_dir.as_path()));
            report.unknown_classes = class_names::unknown_in_source(&config, &known);
            write_if_changed(&out_dir.join("next_rust_known_classes.rs"), class_names::list_source(&known).as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let class_names = naming.as_ref().map(|n| n.seed);
        let renames = naming.map(|n| n.renames);
        // Release builds embed the scripts in `client/` and `assets/`
        // minified; `NEXT_RUST_MINIFY_JS=0|1` overrides. The minified copies
        // live under `$OUT_DIR/next_rust_min` (`asset!` hashes those), and are
        // removed when minification is off so a stale copy is never used.
        let minify_js = match std::env::var("NEXT_RUST_MINIFY_JS").ok().as_deref() {
            Some("0") => false,
            Some(_) => true,
            None => config.build.minify_js && std::env::var("PROFILE").is_ok_and(|p| p == "release"),
        };
        println!("cargo:rerun-if-env-changed=NEXT_RUST_MINIFY_JS");
        // Scripts are also rewritten with the short class names, so a copy is
        // needed whenever classes were shortened.
        // Embedded files are also compressed ahead of time;
        // `NEXT_RUST_PRECOMPRESS=0|1` overrides.
        let precompress = embed_files && !matches!(std::env::var("NEXT_RUST_PRECOMPRESS").ok().as_deref(), Some("0"));
        println!("cargo:rerun-if-env-changed=NEXT_RUST_PRECOMPRESS");
        let minified_dir = out_dir.join(codegen::MINIFIED_DIR);
        let minified_dir = if embed_files && (minify_js || renames.is_some() || precompress) {
            Some(minified_dir)
        } else {
            let _ = std::fs::remove_dir_all(&minified_dir);
            None
        };
        // Classes that scripts add outside the view tree keep their rules on
        // every page: the CSS macros read them from this file.
        let keep: Vec<&String> = config.tailwind.keep_classes.iter().chain(&config.assets.css_safelist).collect();
        let keep = keep.iter().map(|c| c.as_str()).collect::<Vec<_>>().join("\n");
        write_if_changed(&out_dir.join(css_usage::KEEP_FILE), keep.as_bytes()).map_err(|e| e.to_string())?;
        let (mut code, generated) = codegen::generate_code_reporting(
            &project,
            CodegenOptions {
                minify_html,
                embed_files,
                tailwind,
                class_names,
                minified_dir,
                minify_js,
                renames,
                precompress,
                drop_classes,
            },
        );
        report.scripts = generated.scripts;
        report.files = generated.files;
        write_if_changed(
            &out_dir.join(report::FILE),
            serde_json::to_string_pretty(&report).unwrap_or_default().as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        for p in &self.plugins {
            if let Some(extra) = p.extra_code(&project) {
                code.push_str(&format!("\n// plugin: {}\n{extra}\n", p.name()));
            }
        }
        write_if_changed(&out_dir.join("next_rust_routes.rs"), code.as_bytes()).map_err(|e| e.to_string())?;
        write_if_changed(&out_dir.join("next_rust_manifest.json"), project.manifest().to_json().as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(project)
    }
}

/// The short class names of a build.
struct ClassNaming {
    seed: u64,
    renames: codegen::ScriptRenames,
}

/// Give the classes of `css` and `extra` short names and write the table to
/// `$OUT_DIR`.
fn name_classes(
    css: &str,
    config: &Config,
    out_dir: &Path,
    extra: &[String],
    report: &mut report::BuildReport,
) -> Result<ClassNaming, String> {
    let seed = class_names::seed();
    let shortened = class_names::shorten(css, config, seed, extra);
    let mut classes = report::ClassReport { total: shortened.names.len(), ..Default::default() };
    for (_, short) in &shortened.names {
        match short.len() {
            1 => classes.one_char += 1,
            2 => classes.two_chars += 1,
            3 => classes.three_chars += 1,
            _ => classes.longer += 1,
        }
    }
    let renamed: std::collections::BTreeSet<&str> = shortened.names.iter().map(|(c, _)| c.as_str()).collect();
    classes.kept = next_rust_assets::css::class_selectors(css)
        .leading
        .into_iter()
        .filter(|c| !renamed.contains(c.as_str()))
        .collect();
    report.classes = Some(classes);
    write_if_changed(&out_dir.join(class_names::FILE), class_names::table_source(&shortened.names).as_bytes())
        .map_err(|e| e.to_string())?;
    let mut known: std::collections::BTreeSet<String> = next_rust_assets::css::class_selectors(css).all;
    known.extend(extra.iter().cloned());
    Ok(ClassNaming { seed, renames: codegen::ScriptRenames { known, table: shortened.names.into_iter().collect() } })
}

/// Compile Tailwind CSS for the app into `$OUT_DIR` (the stylesheet and its id).
/// Returns the short class names when classes were shortened.
fn generate_tailwind(
    config: &Config,
    out_dir: &Path,
    minify_classes: bool,
    components: &[String],
    report: &mut report::BuildReport,
) -> Result<Option<ClassNaming>, String> {
    println!("cargo:rerun-if-env-changed=NEXT_RUST_TAILWIND_BIN");
    for dir in tailwind::scanned_dirs(config) {
        println!("cargo:rerun-if-changed={}", dir.display());
    }
    // A missing file would rerun the build script on every build; config
    // validation already reports it (NR0008).
    for sheet in tailwind::stylesheet_paths(config).into_iter().filter(|p| p.is_file()) {
        println!("cargo:rerun-if-changed={}", sheet.display());
    }
    // `next-rust dev` / `build` download the engine with a progress bar
    // first; a plain `cargo build` downloads it here, silently.
    let bin = tailwind::ensure(&mut |_, _| {})?;
    // Always minified: browser devtools format CSS on their own, so readable
    // output in development would only cost bytes.
    let mut css = tailwind::compile(&bin, config, out_dir, true)?;
    // Before renaming: these classes keep their names.
    let keep = class_names::outside_classes(&css, config);
    write_if_changed(&out_dir.join("next_rust_tailwind_keep.rs"), class_names::list_source(&keep).as_bytes())
        .map_err(|e| e.to_string())?;
    let scripts = class_names::script_classes(&css, config);
    write_if_changed(&out_dir.join("next_rust_tailwind_scripts.rs"), class_names::map_source(&scripts).as_bytes())
        .map_err(|e| e.to_string())?;
    // The classes the stylesheet defines, for `known_classes`.
    let all_classes = next_rust_assets::css::class_selectors(&css).all.into_iter().collect::<Vec<_>>().join("\n");
    write_if_changed(&out_dir.join("next_rust_tailwind_classes.txt"), all_classes.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut naming = None;
    if minify_classes {
        let named = name_classes(&css, config, out_dir, components, report)?;
        css = next_rust_assets::css::rename_classes(&css, &|class| named.renames.table.get(class).cloned());
        write_if_changed(&out_dir.join("next_rust_tailwind.css"), css.as_bytes()).map_err(|e| e.to_string())?;
        naming = Some(named);
    }
    report.tailwind_css = Some(css.len());
    let id = format!("tw-{}", &next_rust_assets::content_hash(css.as_bytes())[..10]);
    write_if_changed(&out_dir.join("next_rust_tailwind.id"), id.as_bytes()).map_err(|e| e.to_string())?;
    Ok(naming)
}

/// Run the default generator from `build.rs`.
pub fn generate() {
    Generator::new().run();
}

fn emit_rerun(config: &Config, manifest_dir: &Path) {
    println!("cargo:rerun-if-changed={}", config.app_dir().display());
    if let Some(api) = config.api_dir() {
        println!("cargo:rerun-if-changed={}", api.display());
    }
    match &config.source {
        Some(src) => println!("cargo:rerun-if-changed={}", src.display()),
        None => {
            println!("cargo:rerun-if-changed={}", manifest_dir.join(next_rust_core::config::TOML_FILE).display());
        }
    }
    let src = manifest_dir.join("src");
    if src.is_dir() {
        println!("cargo:rerun-if-changed={}", src.display());
    }
    println!("cargo:rerun-if-env-changed=NEXT_RUST_CONFIG");
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

/// Render diagnostics for terminal output.
pub fn render_diagnostics(diags: &[Diagnostic], color: bool) -> String {
    diags.iter().map(|d| d.render(color)).collect::<Vec<_>>().join("\n")
}
