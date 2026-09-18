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
pub mod tailwind;

use std::path::{Path, PathBuf};

pub use codegen::{AnalyzedRoute, CodegenOptions, Project, analyze_project, generate_code, generate_code_with};
use next_rust_core::{Config, Diagnostic};

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
        let class_names = if tailwind {
            generate_tailwind(&config, &out_dir, minify_classes)?
        } else if minify_classes {
            // No Tailwind: only the component classes.
            let seed = class_names::seed();
            let shortened = class_names::shorten("", &config, seed, &class_names::component_classes());
            write_if_changed(&out_dir.join(class_names::FILE), class_names::table_source(&shortened.names).as_bytes())
                .map_err(|e| e.to_string())?;
            Some(seed)
        } else {
            None
        };
        let mut code = generate_code_with(&project, CodegenOptions { minify_html, embed_files, tailwind, class_names });
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

/// Compile Tailwind CSS for the app into `$OUT_DIR` (the stylesheet and its id).
/// Returns the class name seed when classes were shortened.
fn generate_tailwind(config: &Config, out_dir: &Path, minify_classes: bool) -> Result<Option<u64>, String> {
    println!("cargo:rerun-if-env-changed=NEXT_RUST_TAILWIND_BIN");
    for dir in tailwind::scanned_dirs(config) {
        println!("cargo:rerun-if-changed={}", dir.display());
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
    let mut seed = None;
    if minify_classes {
        let build_seed = class_names::seed();
        let shortened = class_names::shorten(&css, config, build_seed, &class_names::component_classes());
        css = shortened.css;
        write_if_changed(&out_dir.join("next_rust_tailwind.css"), css.as_bytes()).map_err(|e| e.to_string())?;
        write_if_changed(&out_dir.join(class_names::FILE), class_names::table_source(&shortened.names).as_bytes())
            .map_err(|e| e.to_string())?;
        seed = Some(build_seed);
    }
    let id = format!("tw-{}", &next_rust_assets::content_hash(css.as_bytes())[..10]);
    write_if_changed(&out_dir.join("next_rust_tailwind.id"), id.as_bytes()).map_err(|e| e.to_string())?;
    Ok(seed)
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
