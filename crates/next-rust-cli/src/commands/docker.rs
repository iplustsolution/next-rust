use crate::{Args, project, templates, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!("next-rust docker [--force]\n\nWrite a multi-stage production Dockerfile and .dockerignore.");
        return Ok(());
    }
    let info = project::load()?;
    let path = info.root.join("Dockerfile");
    if path.exists() && !a.flag(&["--force"]) {
        return Err("Dockerfile already exists (use --force to overwrite)".into());
    }
    std::fs::write(&path, templates::dockerfile(&info.bin_name, info.config.tailwind.enabled))
        .map_err(|e| e.to_string())?;
    let ignore = info.root.join(".dockerignore");
    if !ignore.exists() {
        std::fs::write(&ignore, templates::DOCKERIGNORE).map_err(|e| e.to_string())?;
    }
    ui::ok("Wrote Dockerfile and .dockerignore");
    eprintln!("\n  docker build -t {0} .\n  docker run -p 3000:3000 {0}", info.bin_name);
    Ok(())
}
