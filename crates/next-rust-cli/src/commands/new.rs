use std::path::Path;
use std::time::Instant;

use crate::{Args, templates, ui};

/// A progress step: label, detail, and the files it writes.
type Step<'a> = (&'a str, &'a str, Vec<(&'a str, String)>);

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust new <name> [--framework-path <path to next-rust checkout>]\n\nCreates a project with app/, public/, build.rs and next-rust.toml.\nBy default the project depends on the Next Rust GitHub repository."
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

    let started = Instant::now();
    ui::banner();
    eprintln!("   {}", ui::bold("Thank you for building with Next Rust."));
    eprintln!("   {}", ui::dim("Let's set up your new project."));
    eprintln!();
    eprintln!("   {} Creating {}", ui::accent("◆"), ui::bold(name));
    eprintln!();

    let groups: Vec<Step> = vec![
        (
            "Project manifest",
            "Cargo.toml, build.rs, src/main.rs",
            vec![
                ("Cargo.toml", templates::cargo_toml(pkg, framework.as_deref())),
                ("build.rs", templates::BUILD_RS.into()),
                ("src/main.rs", templates::MAIN_RS.into()),
            ],
        ),
        (
            "Configuration",
            "next-rust.toml, .env.example",
            vec![("next-rust.toml", templates::CONFIG.into()), (".env.example", templates::ENV_EXAMPLE.into())],
        ),
        (
            "Root layout & styles",
            "app/layout.rs, app/globals.css",
            vec![("app/layout.rs", templates::layout_rs(pkg)), ("app/globals.css", templates::GLOBALS_CSS.into())],
        ),
        (
            "Pages",
            "/  ·  /about  ·  404",
            vec![
                ("app/page.rs", templates::PAGE_RS.into()),
                ("app/about/page.rs", templates::ABOUT_RS.into()),
                ("app/not-found.rs", templates::NOT_FOUND_RS.into()),
            ],
        ),
        ("API route", "GET /api/hello", vec![("app/api/hello/route.rs", templates::API_RS.into())]),
        (
            "Static files & git",
            "public/, .gitignore, README.md",
            vec![
                ("public/robots.txt", "User-agent: *\nAllow: /\n".into()),
                (".gitignore", templates::GITIGNORE.into()),
                ("README.md", templates::readme(pkg)),
            ],
        ),
    ];
    let mut count = 0;
    for (label, detail, files) in &groups {
        for (path, content) in files {
            let p = dir.join(path);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&p, content).map_err(|e| format!("{}: {e}", p.display()))?;
            count += 1;
        }
        ui::done_step(label, detail);
    }
    let source = match &framework {
        Some(p) => format!("local checkout · {p}"),
        None => crate::REPO_URL.trim_start_matches("https://").to_owned(),
    };
    ui::done_step("Framework", &source);
    eprintln!();

    let route = |path: &str, file: &str| format!("{}  {}", ui::pad(&ui::bold(path), 13), ui::dim(file));
    ui::boxed(&[
        format!("{} {}", ui::green("✔"), ui::bold(&format!("{name} is ready")))
            + &ui::dim(&format!("   {count} files · {} ms", started.elapsed().as_millis())),
        String::new(),
        ui::dim("Routes"),
        route("/", "app/page.rs"),
        route("/about", "app/about/page.rs"),
        route("/api/hello", "app/api/hello/route.rs"),
    ]);
    eprintln!();

    eprintln!("   {}", ui::bold("Get started"));
    eprintln!();
    eprintln!("     {} {}", ui::dim("$"), ui::accent(&format!("cd {name}")));
    eprintln!("     {} {}      {}", ui::dim("$"), ui::accent("next-rust dev"), ui::dim("→ http://localhost:3000"));
    eprintln!();
    eprintln!("   {}", ui::bold("Useful commands"));
    eprintln!();
    for (cmd, what) in [
        ("next-rust routes", "list every route and how it renders"),
        ("next-rust generate page /x", "scaffold a new page"),
        ("next-rust build", "production build with static pages"),
        ("next-rust upgrade", "update to the latest Next Rust"),
    ] {
        eprintln!("     {}  {}", ui::pad(&ui::cyan(cmd), 28), ui::dim(what));
    }
    eprintln!();
    eprintln!("   {}  https://github.com/iplustsolution/next-rust#readme", ui::dim("Docs"));
    eprintln!();
    eprintln!("   {}", ui::dim("Happy building! Made with ♥ by I Plus T Solution · https://www.iplust.in"));
    eprintln!();
    crate::update_check::notify_if_outdated();
    Ok(())
}
