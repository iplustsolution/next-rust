//! `next-rust dev`: watch → rebuild → restart, with browser live reload.
//!
//! * Rust sources, routes and configuration trigger an incremental
//!   `cargo build` (Cargo only recompiles what changed). While it runs, the
//!   previous server keeps serving.
//! * On success the server restarts; connected browsers reconnect and reload.
//! * On failure the error is printed, written to `.next-rust/dev/status.json`
//!   and shown as an overlay in the browser by the still-running server.
//! * `public/` and `client/` changes need no rebuild: the running server
//!   notices them and reloads the page.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};

use next_rust_build::analyze_project;

use crate::project::{self, ProjectInfo};
use crate::{Args, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust dev [--port <port>]\n\nStart the development server with rebuild-on-change and browser live reload."
        );
        return Ok(());
    }
    crate::update_check::notify_if_outdated();
    let info = project::load()?;
    let port = a.value(&["--port", "-p"]).map(str::to_owned).unwrap_or_else(|| info.config.server.port.to_string());
    let status_file = info.config.output_dir().join("dev/status.json");
    let poll = Duration::from_millis(info.config.dev.poll_interval.max(50));

    ui::header("development server");
    ui::boxed(&[
        format!("{}  {}", ui::dim("Local  "), ui::bold(&format!("http://localhost:{port}"))),
        format!("{}  {}", ui::dim("App dir"), info.config.app_dir().display()),
        format!("{}  {}", ui::dim("Mode   "), "development · rebuild on save · live reload"),
    ]);
    eprintln!();

    let mut routes = route_set(&info);
    let mut child: Option<Child> = None;

    let mut fingerprint = watch_fingerprint(&info);
    // Fill empty special files that exist at startup, then watch for new ones.
    let mut scaffolder = crate::scaffold::Scaffolder::default();
    let scaffold_roots: Vec<std::path::PathBuf> =
        std::iter::once(info.config.app_dir()).chain(info.config.api_dir()).collect();
    if info.config.dev.scaffold {
        report_filled(&info, scaffolder.fill_new(&scaffold_roots));
    }

    if rebuild(&info, &status_file, &port, &mut child) {
        eprintln!("  {}\n", ui::dim(&format!("{} routes", routes.len())));
    }

    loop {
        std::thread::sleep(poll);
        if let Some(c) = child.as_mut()
            && let Ok(Some(status)) = c.try_wait()
        {
            ui::warn(&format!("server exited ({status}); waiting for changes"));
            child = None;
        }
        if info.config.dev.scaffold {
            report_filled(&info, scaffolder.fill_new(&scaffold_roots));
        }
        let next = watch_fingerprint(&info);
        if next == fingerprint {
            continue;
        }
        // Debounce bursts of writes (editors, formatters).
        std::thread::sleep(Duration::from_millis(80));
        fingerprint = watch_fingerprint(&info);

        let new_routes = route_set(&info);
        for added in new_routes.difference(&routes) {
            ui::ok(&format!("route added   {}", ui::bold(added)));
        }
        for removed in routes.difference(&new_routes) {
            ui::warn(&format!("route removed {}", ui::bold(removed)));
        }
        routes = new_routes;
        rebuild(&info, &status_file, &port, &mut child);
    }
}

fn report_filled(info: &ProjectInfo, files: Vec<std::path::PathBuf>) {
    for file in files {
        let shown = file.strip_prefix(&info.root).unwrap_or(&file).display().to_string();
        ui::ok(&format!("filled {} with starter code", ui::bold(&shown)));
    }
}

fn route_set(info: &ProjectInfo) -> BTreeSet<String> {
    let scan = next_rust_router::scan_project(&info.config);
    scan.routes
        .iter()
        .filter(|r| r.intercept.is_none())
        .map(|r| {
            format!(
                "{} {}",
                if r.kind == next_rust_router::RouteKind::Api { "api " } else { "page" },
                r.pattern.to_display_string()
            )
        })
        .collect()
}

fn write_status(file: &Path, ok: bool, message: &str) {
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::json!({ "ok": ok, "message": ui::strip_ansi(message) });
    let _ = std::fs::write(file, json.to_string());
}

fn rebuild(info: &ProjectInfo, status_file: &Path, port: &str, child: &mut Option<Child>) -> bool {
    let start = Instant::now();
    eprintln!("{} {}", ui::cyan("○"), ui::dim("compiling…"));

    // Validate routes first for fast, readable errors.
    let project = analyze_project(&info.config);
    if project.has_errors() {
        let report: String = project.diagnostics.errors().map(|d| d.render(ui::color())).collect::<Vec<_>>().join("\n");
        eprintln!("{report}");
        write_status(status_file, false, &report);
        ui::fail("route errors — fix them to continue (the previous build keeps running)");
        return false;
    }
    for w in project.diagnostics.warnings() {
        eprintln!("{}", w.render(ui::color()));
    }

    match project::cargo_build(info, false, true) {
        Ok(exe) => {
            if let Some(mut old) = child.take() {
                let _ = old.kill();
                let _ = old.wait();
            }
            match Command::new(&exe)
                .current_dir(&info.root)
                .env("NEXT_RUST_ENV", "development")
                .env("PORT", port)
                .spawn()
            {
                Ok(c) => *child = Some(c),
                Err(e) => {
                    ui::fail(&format!("could not start {}: {e}", exe.display()));
                    return false;
                }
            }
            write_status(status_file, true, "");
            ui::ok(&format!("compiled in {:.1}s", start.elapsed().as_secs_f64()));
            true
        }
        Err(report) => {
            eprintln!("{report}");
            write_status(status_file, false, &report);
            ui::fail("compilation failed — the previous build keeps running");
            false
        }
    }
}

/// Fingerprint of everything that requires a Rust rebuild.
fn watch_fingerprint(info: &ProjectInfo) -> u64 {
    let c = &info.config;
    let mut paths: Vec<PathBuf> = vec![
        c.app_dir(),
        info.root.join("src"),
        info.root.join("Cargo.toml"),
        info.root.join("assets"),
        info.root.join("build.rs"),
    ];
    if let Some(api) = c.api_dir() {
        paths.push(api);
    }
    if let Some(src) = &c.source {
        paths.push(src.clone());
    }
    paths.extend(c.dev.watch.iter().map(|p| c.resolve(p)));
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    };
    let mut stack = paths;
    let mut visited = 0;
    while let Some(p) = stack.pop() {
        let Ok(meta) = std::fs::metadata(&p) else { continue };
        if meta.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&p) {
                let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                entries.sort();
                for e in entries {
                    let name = e.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "target" && name != "node_modules" {
                        stack.push(e);
                    }
                }
            }
        } else {
            visited += 1;
            if visited > 50_000 {
                break;
            }
            mix(p.to_string_lossy().as_bytes());
            mix(&meta.len().to_le_bytes());
            let m = meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            mix(&m.to_le_bytes());
        }
    }
    h
}
