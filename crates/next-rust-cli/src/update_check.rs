//! Once-a-day check for a newer Next Rust release.
//!
//! The check downloads the workspace `Cargo.toml` from GitHub and compares its
//! version with this CLI's version. For projects that track the GitHub
//! repository, it also compares the commit recorded in `Cargo.lock` with the
//! repository's latest commit. Nothing about the user or project is sent.
//!
//! It never changes anything: it only suggests `next-rust upgrade`. It runs
//! at most once every 24 hours, gives up after a few seconds, stays silent on
//! any failure, and is disabled by `NEXT_RUST_NO_UPDATE_CHECK=1` or in CI.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::ui;

const RAW_CARGO_TOML: &str = "https://raw.githubusercontent.com/iplustsolution/next-rust/main/Cargo.toml";
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub fn notify_if_outdated() {
    if std::env::var_os("NEXT_RUST_NO_UPDATE_CHECK").is_some() || std::env::var_os("CI").is_some() {
        return;
    }
    let Some(stamp) = stamp_file() else { return };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let last: u64 = std::fs::read_to_string(&stamp).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    if now.saturating_sub(last) < INTERVAL.as_secs() {
        return;
    }
    if let Some(parent) = stamp.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&stamp, now.to_string());

    let mut messages = Vec::new();
    if let Some(latest) = latest_version()
        && is_newer(&latest, env!("CARGO_PKG_VERSION"))
    {
        messages.push(format!("Next Rust {latest} is available (you have {}).", env!("CARGO_PKG_VERSION")));
    }
    if let Some((locked, remote)) = project_commits()
        && !remote.starts_with(&locked)
        && !locked.starts_with(&remote)
    {
        messages.push("This project's framework is behind the latest commit on GitHub.".into());
    }
    if !messages.is_empty() {
        eprintln!();
        for m in messages {
            ui::warn(&m);
        }
        eprintln!("  Run {} to update.\n", ui::bold("next-rust upgrade"));
    }
}

fn stamp_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".next-rust").join("last-update-check"))
}

/// Run a command, returning stdout if it succeeds within `timeout`.
pub fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Option<String> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null()).spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut out = String::new();
                child.stdout.take()?.read_to_string(&mut out).ok()?;
                return Some(out);
            }
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn latest_version() -> Option<String> {
    let text = run_with_timeout(
        Command::new("curl").args(["-fsSL", "--max-time", "4", RAW_CARGO_TOML]),
        Duration::from_secs(5),
    )?;
    parse_workspace_version(&text)
}

pub fn parse_workspace_version(cargo_toml: &str) -> Option<String> {
    let section = cargo_toml.split("[workspace.package]").nth(1)?;
    section
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .find_map(|l| l.trim().strip_prefix("version").map(str::trim))
        .and_then(|v| v.strip_prefix('='))
        .map(|v| v.trim().trim_matches('"').to_owned())
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> { v.split(['.', '-']).take(3).map(|p| p.parse().unwrap_or(0)).collect() };
    parse(candidate) > parse(current)
}

/// Commit of `next-rust` in ./Cargo.lock and the latest remote commit.
fn project_commits() -> Option<(String, String)> {
    let locked = locked_commit(&std::fs::read_to_string("Cargo.lock").ok()?)?;
    let out =
        run_with_timeout(Command::new("git").args(["ls-remote", crate::REPO_URL, "HEAD"]), Duration::from_secs(5))?;
    let remote = out.split_whitespace().next()?.to_owned();
    Some((locked, remote))
}

/// The git commit `next-rust` is locked to, if it comes from the repository.
pub fn locked_commit(lock: &str) -> Option<String> {
    lock.split("[[package]]")
        .find(|p| p.lines().any(|l| l.trim() == "name = \"next-rust\""))?
        .lines()
        .find_map(|l| l.trim().strip_prefix("source = \"git+"))
        .and_then(|s| s.rsplit_once('#'))
        .map(|(_, sha)| sha.trim_end_matches('"').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        let toml = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.2.1\"\nedition = \"2024\"\n";
        assert_eq!(parse_workspace_version(toml).as_deref(), Some("0.2.1"));
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn lockfile_commit() {
        let lock = "[[package]]\nname = \"next-rust\"\nversion = \"0.1.0\"\nsource = \"git+https://github.com/iplustsolution/next-rust#7713a8c46efb4eff195653105e02a556efc09bc3\"\n\n[[package]]\nname = \"serde\"\n";
        assert_eq!(locked_commit(lock).as_deref(), Some("7713a8c46efb4eff195653105e02a556efc09bc3"));
        assert_eq!(locked_commit("[[package]]\nname = \"next-rust\"\nversion = \"0.1.0\"\n"), None);
    }
}
