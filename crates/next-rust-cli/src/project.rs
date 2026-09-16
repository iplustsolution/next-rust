//! Project discovery shared by commands.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use next_rust_core::Config;

use crate::ui;

pub struct ProjectInfo {
    pub config: Config,
    pub root: PathBuf,
    pub bin_name: String,
}

pub fn load_config() -> Result<Config, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Config::discover(&cwd).map_err(|e| e.to_string())
}

/// Config plus Cargo metadata (binary name, target directory).
pub fn load() -> Result<ProjectInfo, String> {
    let config = load_config()?;
    let root = config.root.clone();
    if !root.join("Cargo.toml").is_file() {
        return Err(format!(
            "no Cargo.toml in {} — run this command inside a Next Rust project (or create one with `next-rust new my-app`)",
            root.display()
        ));
    }
    let output = Command::new(cargo())
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if !output.status.success() {
        return Err("`cargo metadata` failed".into());
    }
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let manifest = root.join("Cargo.toml");
    let package = meta["packages"]
        .as_array()
        .and_then(|pkgs| pkgs.iter().find(|p| p["manifest_path"].as_str().is_some_and(|m| Path::new(m) == manifest)))
        .ok_or("could not find this package in cargo metadata")?;
    let bin_name = package["targets"]
        .as_array()
        .and_then(|t| t.iter().find(|t| t["kind"].as_array().is_some_and(|k| k.iter().any(|k| k == "bin"))))
        .and_then(|t| t["name"].as_str())
        .ok_or("this package has no binary target; add src/main.rs with `next_rust::app!();`")?
        .to_owned();
    Ok(ProjectInfo { config, root, bin_name })
}

pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Run `cargo build`, returning the executable path or the rendered
/// compiler errors.
pub fn cargo_build(info: &ProjectInfo, release: bool, quiet: bool) -> Result<PathBuf, String> {
    let mut cmd = Command::new(cargo());
    cmd.arg("build").arg("--bin").arg(&info.bin_name).arg("--message-format=json-diagnostic-rendered-ansi");
    if release {
        cmd.arg("--release");
    }
    if ui::color() {
        cmd.arg("--color=always");
    }
    cmd.current_dir(&info.root).stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().map_err(|e| format!("failed to run cargo: {e}"))?;
    let mut executable = None;
    let mut rendered = String::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        match msg["reason"].as_str() {
            Some("compiler-artifact") => {
                if let Some(exe) = msg["executable"].as_str() {
                    executable = Some(PathBuf::from(exe));
                }
            }
            Some("compiler-message") => {
                let level = msg["message"]["level"].as_str().unwrap_or("");
                if (level == "error" || (!quiet && level == "warning"))
                    && let Some(r) = msg["message"]["rendered"].as_str()
                {
                    rendered.push_str(r);
                }
            }
            _ => {}
        }
    }
    if output.status.success() {
        if !rendered.is_empty() && !quiet {
            eprint!("{rendered}");
        }
        executable.ok_or_else(|| "cargo did not report an executable".into())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Build script failures (route diagnostics) are reported on stderr.
        let script: String =
            stderr.lines().skip_while(|l| !l.contains("--- stderr")).skip(1).collect::<Vec<_>>().join("\n");
        let mut report = rendered;
        if !script.trim().is_empty() {
            report.push_str(script.trim());
            report.push('\n');
        }
        if report.trim().is_empty() {
            report = stderr.into_owned();
        }
        Err(report)
    }
}
