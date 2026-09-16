use std::net::TcpListener;
use std::process::Command;

use next_rust_build::analyze_project;

use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    if Args::new(args).flag(&["-h", "--help"]) {
        println!("next-rust doctor\n\nCheck the toolchain, configuration, project layout and routes.");
        return Ok(());
    }
    let problems = std::cell::Cell::new(0usize);
    let check = |ok: bool, good: &str, bad: &str| {
        if ok {
            ui::ok(good);
        } else {
            ui::fail(bad);
            problems.set(problems.get() + 1);
        }
    };

    let version = |cmd: &str| {
        Command::new(cmd)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    let rustc = version("rustc");
    check(
        rustc.is_some(),
        &format!("rustc: {}", rustc.clone().unwrap_or_default()),
        "rustc not found — install Rust from https://rustup.rs",
    );
    let cargo = version(&project::cargo());
    check(cargo.is_some(), &format!("cargo: {}", cargo.clone().unwrap_or_default()), "cargo not found");
    if let Some(v) = rustc.as_deref().and_then(|v| v.split_whitespace().nth(1)) {
        let minor: u32 = v.split('.').nth(1).and_then(|m| m.parse().ok()).unwrap_or(0);
        check(
            minor >= 88,
            "Rust version supports Next Rust (≥ 1.88)",
            &format!("Rust {v} is too old; Next Rust requires 1.88 or newer"),
        );
    }

    let config = match project::load_config() {
        Ok(c) => c,
        Err(e) => {
            ui::fail(&format!("configuration: {e}"));
            return Err(String::new());
        }
    };
    check(
        true,
        &format!(
            "configuration: {}",
            config
                .source
                .as_ref()
                .map(|s| s.display().to_string())
                .unwrap_or_else(|| "defaults (no next-rust.toml)".into())
        ),
        "",
    );
    for d in config.validate() {
        eprintln!("{}", d.render(ui::color()));
        problems.set(problems.get() + usize::from(d.is_error()));
    }
    let root = &config.root;
    let has = |p: &str| root.join(p).exists();
    check(has("Cargo.toml"), "Cargo.toml found", "Cargo.toml missing");
    check(has("build.rs"), "build.rs found", "build.rs missing — add `fn main() { next_rust_build::generate(); }`");
    let main_rs = std::fs::read_to_string(root.join("src/main.rs")).unwrap_or_default();
    check(
        main_rs.contains("next_rust::app!") || main_rs.contains("next_rust::routes!"),
        "src/main.rs includes the generated routes",
        "src/main.rs does not call `next_rust::app!()` or `next_rust::routes!()`",
    );
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    check(
        cargo_toml.contains("next-rust-build"),
        "next-rust-build is a build dependency",
        "add `next-rust-build` to [build-dependencies]",
    );
    check(
        !has(".env") || std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default().contains(".env"),
        ".env files are git-ignored",
        "a .env file exists but .gitignore does not mention .env — secrets may be committed",
    );

    let port = config.server.port;
    check(
        TcpListener::bind(("127.0.0.1", port)).is_ok(),
        &format!("port {port} is free"),
        &format!("port {port} is in use (set PORT or [server] port)"),
    );

    let analyzed = analyze_project(&config);
    for d in analyzed.diagnostics.iter() {
        eprintln!("{}", d.render(ui::color()));
    }
    let errors = analyzed.diagnostics.errors().count();
    check(
        errors == 0,
        &format!("{} routes, no route errors", analyzed.routes.len()),
        &format!("{errors} route error(s)"),
    );

    let problems = problems.get();
    if problems == 0 {
        eprintln!("\n{}", ui::green("Everything looks good."));
        Ok(())
    } else {
        Err(format!("{problems} problem(s) found"))
    }
}
