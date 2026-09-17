//! `next-rust upgrade`: update the CLI and the current project's framework.

use std::process::Command;

use crate::update_check::locked_commit;
use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust upgrade [--cli] [--project]\n\n\
             Update to the latest Next Rust from {}.\n\n\
             With no flags, both are updated:\n  \
             --cli       reinstall the `next-rust` command from GitHub\n  \
             --project   move this project's framework dependency to the latest commit\n\n\
             Set NEXT_RUST_NO_UPDATE_CHECK=1 to turn off the daily update notice.",
            crate::REPO_URL
        );
        return Ok(());
    }
    let only_cli = a.flag(&["--cli"]) && !a.flag(&["--project"]);
    let only_project = a.flag(&["--project"]) && !a.flag(&["--cli"]);

    if !only_cli {
        upgrade_project(only_project)?;
    }
    if !only_project {
        upgrade_cli()?;
    }
    Ok(())
}

fn upgrade_project(required: bool) -> Result<(), String> {
    let Ok(manifest) = std::fs::read_to_string("Cargo.toml") else {
        if required {
            return Err("no Cargo.toml here; run `next-rust upgrade --project` inside a project".into());
        }
        return Ok(());
    };
    let Some(dep) =
        manifest.lines().find(|l| l.trim_start().starts_with("next-rust ") || l.trim_start().starts_with("next-rust="))
    else {
        if required {
            return Err("this Cargo.toml does not depend on next-rust".into());
        }
        return Ok(());
    };

    if dep.contains("path") {
        ui::warn("this project uses a local framework checkout (path dependency)");
        eprintln!("  Update it with `git pull` in that checkout, then rebuild.");
        return Ok(());
    }

    ui::step("Updating the project's framework");
    let before = std::fs::read_to_string("Cargo.lock").ok().and_then(|l| locked_commit(&l));
    let status = Command::new(project::cargo())
        .args(["update", "-p", "next-rust", "-p", "next-rust-build"])
        .status()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if !status.success() {
        return Err("`cargo update` failed".into());
    }
    let after = std::fs::read_to_string("Cargo.lock").ok().and_then(|l| locked_commit(&l));
    match (before, after) {
        (Some(b), Some(a)) if b == a => ui::ok(&format!("Framework already up to date ({})", &a[..a.len().min(8)])),
        (Some(b), Some(a)) => ui::ok(&format!("Framework updated {} → {}", &b[..b.len().min(8)], &a[..a.len().min(8)])),
        _ => ui::ok("Framework dependencies updated"),
    }
    Ok(())
}

fn upgrade_cli() -> Result<(), String> {
    ui::step("Reinstalling the next-rust CLI from GitHub");
    let status = Command::new(project::cargo())
        .args(["install", "--git", crate::REPO_URL, "next-rust-cli", "--force"])
        .status()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if status.success() {
        ui::ok("CLI updated. Run `next-rust --version` to check.");
        Ok(())
    } else {
        Err(format!(
            "reinstalling the CLI failed. On Windows, close other `next-rust` processes, or run:\n  cargo install --git {} next-rust-cli --force",
            crate::REPO_URL
        ))
    }
}
