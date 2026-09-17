//! Project discovery shared by commands.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use next_rust_core::Config;

use crate::ui;

pub struct ProjectInfo {
    pub config: Config,
    pub root: PathBuf,
    pub bin_name: String,
    /// Cargo's target directory.
    pub target_dir: PathBuf,
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
    let target_dir = meta["target_directory"].as_str().map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    Ok(ProjectInfo { config, root, bin_name, target_dir })
}

pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Run `cargo build`, returning the executable path or the rendered
/// compiler errors.
pub fn cargo_build(info: &ProjectInfo, release: bool, quiet: bool) -> Result<PathBuf, String> {
    cargo_build_with(info, release, quiet, &[])
}

/// [`cargo_build`] with extra environment variables for cargo.
pub fn cargo_build_with(
    info: &ProjectInfo,
    release: bool,
    quiet: bool,
    envs: &[(&str, &str)],
) -> Result<PathBuf, String> {
    let (exe, warnings) = cargo_build_live(info, release, envs, false, &mut |_| {})?;
    if !warnings.is_empty() && !quiet {
        eprint!("{warnings}");
    }
    Ok(exe)
}

/// What cargo is doing, reported while it runs.
pub enum CargoEvent<'a> {
    /// A status line such as `Compiling serde v1.0.0` (`verb`, `rest`).
    Status { verb: &'a str, rest: &'a str },
    /// Cargo's own unit counter: `done` of `total` units, and what is
    /// compiling right now.
    Progress { done: usize, total: usize, active: &'a str },
    /// Nothing new; called regularly so a spinner can animate.
    Tick,
}

/// Run `cargo build` and report progress as it happens. Returns the
/// executable and any rendered warnings, or the rendered errors.
pub fn cargo_build_live(
    info: &ProjectInfo,
    release: bool,
    envs: &[(&str, &str)],
    progress: bool,
    on_event: &mut dyn FnMut(CargoEvent),
) -> Result<(PathBuf, String), String> {
    use std::io::{BufRead, Read};
    use std::sync::mpsc;
    use std::time::Duration;

    let mut cmd = Command::new(cargo());
    cmd.envs(envs.iter().copied());
    if progress {
        // Cargo draws its progress bar only on terminals; ask for it anyway so
        // the unit counter can be read from the pipe.
        cmd.env("CARGO_TERM_PROGRESS_WHEN", "always").env("CARGO_TERM_PROGRESS_WIDTH", "1000");
    }
    cmd.arg("build").arg("--bin").arg(&info.bin_name).arg("--message-format=json-diagnostic-rendered-ansi");
    if release {
        cmd.arg("--release");
    }
    if ui::color() {
        cmd.arg("--color=always");
    }
    cmd.current_dir(&info.root).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("failed to run cargo: {e}"))?;
    let stdout = child.stdout.take().ok_or("cargo stdout unavailable")?;
    let mut stderr = child.stderr.take().ok_or("cargo stderr unavailable")?;

    // JSON messages: the executable and rendered diagnostics.
    let messages = std::thread::spawn(move || {
        let mut executable = None;
        let (mut errors, mut warnings) = (String::new(), String::new());
        for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
            let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            match msg["reason"].as_str() {
                Some("compiler-artifact") => {
                    if let Some(exe) = msg["executable"].as_str() {
                        executable = Some(PathBuf::from(exe));
                    }
                }
                Some("compiler-message") => {
                    let rendered = msg["message"]["rendered"].as_str().unwrap_or("");
                    match msg["message"]["level"].as_str() {
                        Some("error") => errors.push_str(rendered),
                        Some("warning") => warnings.push_str(rendered),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        (executable, errors, warnings)
    });

    // Human output on stderr, split on both `\n` and the `\r` of progress redraws.
    let (tx, rx) = mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut pending = Vec::new();
        while let Ok(n) = stderr.read(&mut buf) {
            if n == 0 {
                break;
            }
            for &b in &buf[..n] {
                if b == b'\n' || b == b'\r' {
                    if !pending.is_empty() {
                        let _ = tx.send(String::from_utf8_lossy(&pending).into_owned());
                        pending.clear();
                    }
                } else {
                    pending.push(b);
                }
            }
        }
        if !pending.is_empty() {
            let _ = tx.send(String::from_utf8_lossy(&pending).into_owned());
        }
    });

    let mut log = String::new();
    loop {
        match rx.recv_timeout(Duration::from_millis(80)) {
            Ok(segment) => {
                let plain = ui::strip_ansi(&segment);
                let line = plain.trim();
                if let Some((done, total, active)) = parse_progress(line) {
                    on_event(CargoEvent::Progress { done, total, active });
                } else if !line.is_empty() {
                    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
                    if !verb.is_empty()
                        && verb.chars().all(|c| c.is_ascii_alphabetic())
                        && verb.starts_with(char::is_uppercase)
                    {
                        on_event(CargoEvent::Status { verb, rest: rest.trim() });
                    }
                    log.push_str(&segment);
                    log.push('\n');
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => on_event(CargoEvent::Tick),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = reader.join();
    let status = child.wait().map_err(|e| e.to_string())?;
    let (executable, errors, warnings) = messages.join().map_err(|_| "cargo output reader failed")?;

    if status.success() {
        return executable.map(|exe| (exe, warnings)).ok_or_else(|| "cargo did not report an executable".into());
    }
    // Build script failures (route diagnostics) are reported on stderr.
    let script: String = log.lines().skip_while(|l| !l.contains("--- stderr")).skip(1).collect::<Vec<_>>().join("\n");
    let mut report = errors;
    if !script.trim().is_empty() {
        report.push_str(script.trim());
        report.push('\n');
    }
    if report.trim().is_empty() {
        report = log;
    }
    Err(report)
}

/// `Building [=====>   ] 57/245: serde, tokio(build)` → (57, 245, "serde, tokio(build)").
fn parse_progress(line: &str) -> Option<(usize, usize, &str)> {
    let rest = line.strip_prefix("Building [")?;
    let rest = &rest[rest.find("] ")? + 2..];
    let (counts, active) = rest.split_once(':').unwrap_or((rest, ""));
    let (done, total) = counts.trim().split_once('/')?;
    Some((done.parse().ok()?, total.parse().ok()?, active.trim()))
}

#[cfg(test)]
mod tests {
    use super::parse_progress;

    #[test]
    fn reads_cargo_progress_lines() {
        assert_eq!(
            parse_progress("Building [=====>        ] 57/245: serde, tokio(build)"),
            Some((57, 245, "serde, tokio(build)"))
        );
        assert_eq!(parse_progress("Building [                    ] 0/24"), Some((0, 24, "")));
        assert_eq!(parse_progress("Compiling serde v1.0.0"), None);
    }
}
