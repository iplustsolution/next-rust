use std::path::PathBuf;

use next_rust_router::parse_segment;

use crate::{Args, project, templates, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    let pos = a.positional(&[]);
    if a.flag(&["-h", "--help"]) || pos.len() < 2 {
        println!(
            "next-rust generate <kind> <route>\n\nKinds: page, layout, template, loading, error, not-found, api, middleware\n\nExamples:\n  next-rust generate page /products/[id]\n  next-rust generate api /api/users\n  next-rust generate layout /dashboard"
        );
        return if pos.len() < 2 && !a.flag(&["-h", "--help"]) {
            Err("expected <kind> <route>".into())
        } else {
            Ok(())
        };
    }
    let (kind, route) = (pos[0], pos[1]);
    let Some((file, content)) = templates::generate(kind, route) else {
        return Err(format!("unknown kind `{kind}`"));
    };
    let config = project::load_config()?;
    let mut dir: PathBuf = config.app_dir();
    for seg in route.split('/').filter(|s| !s.is_empty()) {
        parse_segment(seg).map_err(|e| format!("invalid route segment: {e}"))?;
        dir.push(seg);
    }
    let path = dir.join(file);
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    ui::ok(&format!("Created {}", path.strip_prefix(&config.root).unwrap_or(&path).display()));
    Ok(())
}
