//! `next-rust build`.

use std::process::Command;
use std::time::Instant;

use next_rust_build::analyze_project;
use next_rust_router::RouteKind;

use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust build [--no-export]\n\nCompile in release mode, pre-render static pages and write the build output:\n\n  .next-rust/\n    server/<bin>          production server binary\n    static/               pre-rendered HTML (also usable for static hosting)\n    cache/pages/          ISR page cache seeded with the static pages\n    manifest/routes.json  route manifest\n    manifest/build.json   static generation report"
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

    ui::step("Compiling (release)");
    let exe = project::cargo_build(&info, true, false).map_err(|report| {
        eprintln!("{report}");
        "compilation failed".to_owned()
    })?;

    let server_dir = out.join("server");
    std::fs::create_dir_all(&server_dir).map_err(|e| e.to_string())?;
    let dest = server_dir.join(&info.bin_name);
    std::fs::copy(&exe, &dest).map_err(|e| format!("copy {}: {e}", dest.display()))?;

    let mut report = serde_json::Value::Null;
    if !a.flag(&["--no-export"]) {
        ui::step("Generating static pages");
        let output = Command::new(&dest)
            .arg("--export")
            .current_dir(&info.root)
            .env("NEXT_RUST_ENV", "production")
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            return Err("static generation failed".into());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        report = stdout.lines().last().and_then(|l| serde_json::from_str(l).ok()).unwrap_or(serde_json::Value::Null);
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
    let manifest_dir = out.join("manifest");
    std::fs::create_dir_all(&manifest_dir).map_err(|e| e.to_string())?;
    std::fs::write(manifest_dir.join("routes.json"), manifest.to_json()).map_err(|e| e.to_string())?;
    std::fs::write(manifest_dir.join("build.json"), serde_json::to_string_pretty(&report).unwrap_or_default())
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
    ui::ok(&format!(
        "Built in {:.1}s → {} (server binary {})",
        started.elapsed().as_secs_f64(),
        out.strip_prefix(&info.root).unwrap_or(&out).display(),
        ui::bytes(bin_size)
    ));
    eprintln!("  Run it with {}", ui::bold("next-rust start"));
    Ok(())
}
