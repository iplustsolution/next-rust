//! `next-rust build`.

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use next_rust_build::analyze_project;
use next_rust_router::RouteKind;

use crate::project::ProjectInfo;
use crate::{Args, project, ui};

/// Release profile used when the project's Cargo.toml doesn't set its own:
/// full link-time optimization and a stripped binary.
const RELEASE_PROFILE: &[(&str, &str)] = &[
    ("CARGO_PROFILE_RELEASE_OPT_LEVEL", "3"),
    ("CARGO_PROFILE_RELEASE_LTO", "fat"),
    ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1"),
    ("CARGO_PROFILE_RELEASE_STRIP", "symbols"),
];

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust build [--no-check]\n\nCompile a single, self-contained production binary:\n\n  .next-rust/<bin>   the whole app: server, pages, next-rust.toml, public/, assets/ and client/\n\nThe binary is optimized with link-time optimization and stripped of symbols. Deploy\nthat one file and run it; static pages are pre-rendered in memory when it starts.\n\n  --no-check   skip rendering every static page after compiling"
        );
        return Ok(());
    }
    let started = Instant::now();
    let info = project::load()?;
    let out = info.config.output_dir();

    ui::header("production build");
    ui::step("Validating routes");
    let analyzed = analyze_project(&info.config);
    super::report(&analyzed)?;

    ui::step("Compiling a single stripped binary");
    let exe = project::cargo_build_with(&info, true, false, &release_env(&info)).map_err(|report| {
        eprintln!("{report}");
        "compilation failed".to_owned()
    })?;

    // Only the binary is written; earlier build layouts are removed.
    for old in ["server", "static", "cache", "manifest"] {
        let _ = std::fs::remove_dir_all(out.join(old));
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let dest = out.join(exe.file_name().ok_or("cargo reported an invalid executable path")?);
    let _ = std::fs::remove_file(&dest);
    std::fs::copy(&exe, &dest).map_err(|e| format!("copy {}: {e}", dest.display()))?;

    let mut report = serde_json::Value::Null;
    if !a.flag(&["--no-check", "--no-export"]) {
        ui::step("Rendering static pages");
        report = check_binary(&dest)?;
    }

    let mut manifest = analyzed.manifest();
    if let Some(pages) = report["pages"].as_array() {
        for entry in &mut manifest.routes {
            entry.static_params = pages
                .iter()
                .filter(|p| p["pattern"].as_str() == Some(entry.pattern.as_str()))
                .filter_map(|p| serde_json::from_value(p["params"].clone()).ok())
                .collect();
        }
    }
    // Build reports for `next-rust analyze` live in Cargo's target directory,
    // never next to the binary.
    let reports = reports_dir(&info);
    std::fs::create_dir_all(&reports).map_err(|e| e.to_string())?;
    std::fs::write(reports.join("routes.json"), manifest.to_json()).map_err(|e| e.to_string())?;
    std::fs::write(reports.join("build.json"), serde_json::to_string_pretty(&report).unwrap_or_default())
        .map_err(|e| e.to_string())?;

    eprintln!();
    println!("{}", ui::bold(&format!("{:<44} {:>10}  {}", "Route", "Size", "")));
    for r in &analyzed.routes {
        let pattern = r.route.pattern.to_fs_string();
        let (symbol, size, extra) = match (r.route.kind, r.rendering) {
            (RouteKind::Api, _) => (ui::magenta("ƒ"), String::new(), ui::dim("api")),
            (_, "static") => {
                let pages: Vec<&serde_json::Value> = report["pages"]
                    .as_array()
                    .map(|p| p.iter().filter(|x| x["pattern"].as_str() == Some(pattern.as_str())).collect())
                    .unwrap_or_default();
                let bytes: u64 = pages.iter().filter_map(|p| p["bytes"].as_u64()).sum();
                let extra = match (pages.len(), r.revalidate) {
                    (0, _) => ui::dim("rendered on first request"),
                    (n, Some(s)) => ui::dim(&format!("{n} page{} · revalidate {s}s", if n == 1 { "" } else { "s" })),
                    (n, None) => ui::dim(&format!("{n} page{}", if n == 1 { "" } else { "s" })),
                };
                (ui::green("○"), if bytes > 0 { ui::bytes(bytes) } else { String::new() }, extra)
            }
            _ => (ui::yellow("λ"), String::new(), ui::dim(r.dynamic_reason.as_deref().unwrap_or("dynamic"))),
        };
        println!("{symbol} {:<42} {:>10}  {extra}", r.route.pattern.to_display_string(), size);
    }
    let bin_size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    println!(
        "\n{}  static (pre-rendered)   {}  dynamic (server-rendered)   {}  API",
        ui::green("○"),
        ui::yellow("λ"),
        ui::magenta("ƒ")
    );
    eprintln!();
    let shown = dest.strip_prefix(&info.root).unwrap_or(&dest);
    ui::ok(&format!(
        "Built in {:.1}s → {} ({})",
        started.elapsed().as_secs_f64(),
        shown.display(),
        ui::bytes(bin_size)
    ));
    eprintln!(
        "  One file to deploy. Run it with {} or {}",
        ui::bold("next-rust start"),
        ui::bold(&format!("./{}", shown.display()))
    );
    Ok(())
}

/// Stripped, fully optimized release settings, unless the project configures
/// `[profile.release]` itself or the variables are already set.
fn release_env(info: &ProjectInfo) -> Vec<(&'static str, &'static str)> {
    let manifest = std::fs::read_to_string(info.root.join("Cargo.toml")).unwrap_or_default();
    let mut envs: Vec<(&str, &str)> = vec![("NEXT_RUST_EMBED", "1")];
    if !manifest.contains("[profile.release]") {
        envs.extend(RELEASE_PROFILE.iter().copied().filter(|(k, _)| std::env::var_os(k).is_none()));
    }
    envs
}

/// Where `next-rust analyze` finds the last build's reports.
pub fn reports_dir(info: &ProjectInfo) -> std::path::PathBuf {
    info.target_dir.join("next-rust")
}

/// Run the binary from an empty directory and render every static page. This
/// proves it needs no files besides itself and catches rendering errors at
/// build time.
fn check_binary(bin: &Path) -> Result<serde_json::Value, String> {
    let empty = std::env::temp_dir().join(format!("next-rust-build-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).map_err(|e| e.to_string())?;
    let output = Command::new(bin)
        .arg("--export")
        .current_dir(&empty)
        .env("NEXT_RUST_ENV", "production")
        .env_remove("NEXT_RUST_CONFIG")
        .output();
    let _ = std::fs::remove_dir_all(&empty);
    let output = output.map_err(|e| e.to_string())?;
    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("static generation failed".into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().last().and_then(|l| serde_json::from_str(l).ok()).unwrap_or(serde_json::Value::Null))
}
