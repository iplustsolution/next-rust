use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    if Args::new(args).flag(&["-h", "--help"]) {
        println!("next-rust clean\n\nRemove the build output directory (default .next-rust).");
        return Ok(());
    }
    let config = project::load_config()?;
    let out = config.output_dir();
    if !out.starts_with(&config.root) {
        return Err(format!("refusing to delete {} because it is outside the project", out.display()));
    }
    match std::fs::remove_dir_all(&out) {
        Ok(()) => ui::ok(&format!("Removed {}", out.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ui::ok("Nothing to clean"),
        Err(e) => return Err(e.to_string()),
    }
    Ok(())
}
