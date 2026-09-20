use std::path::PathBuf;

use next_rust_router::parse_segment;

use crate::{Args, project, templates, ui, wasm};

const HELP: &str = "next-rust generate <kind> <route>\n\nKinds: page, layout, template, loading, error, not-found, api, middleware\n\nExamples:\n  next-rust generate page /products/[id]\n  next-rust generate api /api/users\n  next-rust generate layout /dashboard\n\nnext-rust generate islands [--ui] [--no-demo]\n\n  Writes the islands/ crate this project's WebAssembly switches need and\n  turns them on in next-rust.toml. --ui also moves the shipped components'\n  behaviour to Rust ([ui] script = \"wasm\"); --no-demo leaves out the example\n  island and the page that renders it.";

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    let pos = a.positional(&[]);
    if a.flag(&["-h", "--help"]) {
        println!("{HELP}");
        return Ok(());
    }
    // The one kind that is a crate rather than a file under `app/`, so it
    // takes no route.
    if pos.first() == Some(&"islands") {
        return islands(&a);
    }
    if pos.len() < 2 {
        println!("{HELP}");
        return Err("expected <kind> <route>".into());
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

/// `next-rust generate islands` — the whole of "turn WebAssembly on".
///
/// It exists because the manual version is four separate things that each
/// fail silently (see `crate::wasm`), and because the one question a project
/// actually has — "why is my island inert?" — is nearly always one of them.
/// Writing the crate *and* the keys together is what makes the answer "it
/// just worked" instead of "it built and did nothing".
fn islands(a: &Args<'_>) -> Result<(), String> {
    let config = project::load_config()?;
    let root = config.root.clone();
    let options = wasm::Options { ui: a.flag(&["--ui"]), demo: !a.flag(&["--no-demo"]) };
    let source = wasm::framework_source(&root);
    let package = wasm::package_name(&root);
    let (written, skipped) = wasm::scaffold(&root, &package, &source, options, &config.app_dir())?;
    for file in &written {
        ui::ok(&format!("Created {file}"));
    }
    for file in &skipped {
        ui::warn(&format!("{file} already exists — left alone"));
    }

    // The keys. A crate nothing switches on is exactly the half-done setup
    // this command exists to stop happening.
    let config_path = config.source.clone().unwrap_or_else(|| root.join("next-rust.toml"));
    let mut text = std::fs::read_to_string(&config_path).unwrap_or_default();
    let mut changed = Vec::new();
    for (section, key, value) in wasm::keys(options) {
        if let Some(next) = wasm::set_key(&text, section, key, value) {
            text = next;
            changed.push(format!("[{section}] {key} = {value}"));
        }
    }
    if !changed.is_empty() {
        std::fs::write(&config_path, &text).map_err(|e| format!("{}: {e}", config_path.display()))?;
        for key in &changed {
            ui::ok(&format!("Set {key} in {}", config_path.display()));
        }
    }

    eprintln!();
    eprintln!("   {}", ui::bold("Next"));
    eprintln!();
    eprintln!(
        "     {} {}   {}",
        ui::dim("$"),
        ui::accent("rustup target add wasm32-unknown-unknown"),
        ui::dim("once per machine")
    );
    eprintln!(
        "     {} {}                     {}",
        ui::dim("$"),
        ui::accent("next-rust doctor"),
        ui::dim("checks every prerequisite")
    );
    eprintln!(
        "     {} {}                        {}",
        ui::dim("$"),
        ui::accent("next-rust dev"),
        ui::dim(if options.demo { "→ http://localhost:3000/wasm" } else { "→ http://localhost:3000" })
    );
    eprintln!();
    eprintln!("   {}", ui::dim("Each island is one `hydrate_<Name>` entry in islands/src/lib.rs and one"));
    eprintln!("   {}", ui::dim("`#[client(wasm = \"<Name>\")]` on the server. The names must match exactly."));
    eprintln!();
    Ok(())
}
