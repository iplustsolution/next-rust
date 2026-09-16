use std::process::Command;

use next_rust_build::analyze_project;

use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    if Args::new(args).flag(&["-h", "--help"]) {
        println!("next-rust check\n\nValidate routes and special files, then run `cargo check`.");
        return Ok(());
    }
    let config = project::load_config()?;
    ui::step("Validating routes");
    let project_info = analyze_project(&config);
    super::report(&project_info)?;
    ui::ok(&format!("{} routes valid", project_info.routes.len()));
    ui::step("Type-checking");
    let status = Command::new(project::cargo())
        .arg("check")
        .current_dir(&config.root)
        .status()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if status.success() {
        ui::ok("No problems found");
        Ok(())
    } else {
        Err("cargo check failed".into())
    }
}
