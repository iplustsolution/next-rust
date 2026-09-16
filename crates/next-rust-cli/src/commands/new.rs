use std::path::Path;

use crate::{Args, templates, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust new <name> [--framework-path <path to next-rust checkout>]\n\nCreates a project with app/, public/, build.rs and next-rust.toml."
        );
        return Ok(());
    }
    let positional = a.positional(&["--framework-path"]);
    let Some(name) = positional.first() else {
        return Err("missing project name: next-rust new my-app".into());
    };
    let dir = Path::new(name);
    let pkg = dir.file_name().and_then(|n| n.to_str()).unwrap_or(name);
    if pkg.is_empty()
        || !pkg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || pkg.starts_with(|c: char| c.is_ascii_digit())
    {
        return Err(format!("`{pkg}` is not a valid package name (letters, digits, `-`, `_`)"));
    }
    if dir.exists() && dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(format!("`{}` already exists and is not empty", dir.display()));
    }
    let framework =
        a.value(&["--framework-path"]).map(str::to_owned).or_else(|| std::env::var("NEXT_RUST_FRAMEWORK_PATH").ok());
    let framework = framework.map(|p| std::fs::canonicalize(&p).map(|c| c.to_string_lossy().into_owned()).unwrap_or(p));

    let files: Vec<(&str, String)> = vec![
        ("Cargo.toml", templates::cargo_toml(pkg, framework.as_deref())),
        ("build.rs", templates::BUILD_RS.into()),
        ("src/main.rs", templates::MAIN_RS.into()),
        ("next-rust.toml", templates::CONFIG.into()),
        ("app/layout.rs", templates::layout_rs(pkg)),
        ("app/page.rs", templates::PAGE_RS.into()),
        ("app/globals.css", templates::GLOBALS_CSS.into()),
        ("app/not-found.rs", templates::NOT_FOUND_RS.into()),
        ("app/about/page.rs", templates::ABOUT_RS.into()),
        ("app/api/hello/route.rs", templates::API_RS.into()),
        ("public/robots.txt", "User-agent: *\nAllow: /\n".into()),
        (".gitignore", templates::GITIGNORE.into()),
        (".env.example", templates::ENV_EXAMPLE.into()),
        ("README.md", templates::readme(pkg)),
    ];
    for (path, content) in &files {
        let p = dir.join(path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, content).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    ui::ok(&format!("Created {}", ui::bold(name)));
    eprintln!();
    eprintln!("  {}", ui::dim("app/"));
    eprintln!("  ├── layout.rs");
    eprintln!("  ├── page.rs            {}", ui::dim("→ /"));
    eprintln!("  ├── about/page.rs      {}", ui::dim("→ /about"));
    eprintln!("  └── api/hello/route.rs {}", ui::dim("→ /api/hello"));
    eprintln!();
    eprintln!("  Next steps:");
    eprintln!("    cd {name}");
    eprintln!("    next-rust dev");
    if framework.is_none() {
        eprintln!();
        ui::warn("the project depends on next-rust from crates.io; pass --framework-path to use a local checkout");
    }
    Ok(())
}
