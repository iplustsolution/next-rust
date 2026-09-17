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
pub mod codegen;

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

/// Configurable generator.
#[derive(Default)]
pub struct Generator {
    plugins: Vec<Box<dyn BuildPlugin>>,
    manifest_dir: Option<PathBuf>,
    out_dir: Option<PathBuf>,
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
    pub fn try_run(self) -> Result<Project, String> {
        let manifest_dir = self
            .manifest_dir
            .or_else(|| std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from))
            .ok_or("CARGO_MANIFEST_DIR is not set; call generate() from build.rs")?;
        let out_dir =
            self.out_dir.or_else(|| std::env::var_os("OUT_DIR").map(PathBuf::from)).ok_or("OUT_DIR is not set")?;

        let config = Config::discover(&manifest_dir).map_err(|e| e.to_string())?;
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
        let mut code = generate_code_with(&project, CodegenOptions { minify_html, embed_files });
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
