use std::path::Path;
use std::time::Instant;

use crate::{Args, starter, templates, ui};

/// A progress step: label, detail, and the files it writes.
type Step<'a> = (&'a str, &'a str, Vec<(&'a str, String)>);

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust new <name> [--git | --framework-path <path to next-rust checkout>] [--no-tailwind]\n\nCreates a project with app/, public/, build.rs and next-rust.toml, styled with Tailwind CSS\n(configured in next-rust.toml, no CSS files). --no-tailwind uses a plain CSS file instead.\nThe project depends on next-rust from crates.io (matching this CLI's version) when it is published there;\notherwise, or with --git, on the GitHub repository."
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
    let source = match framework {
        Some(p) => templates::FrameworkSource::Path(p),
        None if a.flag(&["--git"]) => templates::FrameworkSource::Git,
        None if published_on_crates_io() => templates::FrameworkSource::Registry,
        None => templates::FrameworkSource::Git,
    };

    let tailwind = !a.flag(&["--no-tailwind"]);
    let config =
        if tailwind { format!("{}{}", templates::CONFIG, starter::TW_CONFIG) } else { templates::CONFIG.into() };

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
                ("Cargo.toml", templates::cargo_toml(pkg, &source)),
                ("build.rs", templates::BUILD_RS.into()),
                ("src/main.rs", starter::MAIN_RS.into()),
            ],
        ),
        (
            "Configuration",
            "next-rust.toml, .env.example",
            vec![("next-rust.toml", config), (".env.example", templates::ENV_EXAMPLE.into())],
        ),
        if tailwind {
            (
                "Tailwind CSS",
                "theme & utilities in next-rust.toml, no CSS files",
                vec![("app/layout.rs", starter::tw_layout_rs(pkg))],
            )
        } else {
            (
                "Theme & layout",
                "one centered hero, dark & light",
                vec![("app/layout.rs", starter::layout_rs(pkg)), ("app/globals.css", starter::GLOBALS_CSS.into())],
            )
        },
        (
            "Pages",
            "home  ·  404",
            if tailwind {
                vec![("app/page.rs", starter::TW_PAGE_RS.into()), ("app/not-found.rs", starter::TW_NOT_FOUND_RS.into())]
            } else {
                vec![("app/page.rs", starter::PAGE_RS.into()), ("app/not-found.rs", starter::NOT_FOUND_RS.into())]
            },
        ),
        ("API route", "GET /api/hello", vec![("app/api/hello/route.rs", templates::API_RS.into())]),
        (
            "Editor setup",
            "VS Code snippets (nrpage, nrroute, …)",
            super::editor::files().iter().map(|(path, contents)| (*path, (*contents).to_owned())).collect(),
        ),
        (
            "Logo & static files",
            "favicon.svg, robots.txt, README.md",
            vec![
                ("public/favicon.svg", starter::FAVICON_SVG.into()),
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

    ui::done_step("Framework", &source.describe());
    eprintln!();

    let route = |path: &str, file: &str| format!("{}  {}", ui::pad(&ui::bold(path), 13), ui::dim(file));
    ui::boxed(&[
        format!("{} {}", ui::green("✔"), ui::bold(&format!("{name} is ready")))
            + &ui::dim(&format!("   {count} files · {} ms", started.elapsed().as_millis())),
        String::new(),
        ui::dim("Routes"),
        route("/", "app/page.rs"),
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

/// Whether this CLI's version of `next-rust` is available on crates.io.
/// Offline or on any error the answer is "no", and the project uses GitHub.
fn published_on_crates_io() -> bool {
    let url = format!("https://crates.io/api/v1/crates/next-rust/{}", env!("CARGO_PKG_VERSION"));
    let out = crate::update_check::run_with_timeout(
        std::process::Command::new("curl").args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "--max-time",
            "4",
            "-A",
            "next-rust-cli (https://github.com/iplustsolution/next-rust)",
            &url,
        ]),
        std::time::Duration::from_secs(5),
    );
    out.is_some_and(|code| code.trim() == "200")
}
