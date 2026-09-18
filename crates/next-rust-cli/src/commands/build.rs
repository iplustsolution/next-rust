//! `next-rust build`.

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use next_rust_build::analyze_project;
use next_rust_router::RouteKind;

use crate::project::ProjectInfo;
use crate::{Args, project, ui};

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

    ui::banner();
    eprintln!("   {} {}  {}", ui::accent("◆"), ui::bold("Production build"), ui::dim(&info.bin_name));
    eprintln!();

    // 1. Routes
    let analyzed = analyze_project(&info.config);
    let pages = analyzed.routes.iter().filter(|r| r.route.kind != RouteKind::Api).count();
    let apis = analyzed.routes.len() - pages;
    let statics = analyzed.routes.iter().filter(|r| r.route.kind != RouteKind::Api && r.rendering == "static").count();
    if analyzed.has_errors() {
        ui::LiveLine::new().fail("Routes", &ui::red("problems found"));
        eprintln!();
        return super::report(&analyzed);
    }
    for d in analyzed.diagnostics.iter() {
        eprintln!("{}", d.render(ui::color()));
    }
    ui::done_step(
        "Routes",
        &format!(
            "{} · {} static · {} dynamic",
            plural(pages, "page")
                + &if apis > 0 { format!(" · {}", plural(apis, "API route")) } else { String::new() },
            statics,
            pages - statics
        ),
    );

    // Tailwind CSS: the engine must be ready before cargo runs the build script.
    if crate::tailwind::prepare(&info)? {
        ui::done_step(
            "Tailwind CSS",
            &format!(
                "v{} · classes from {} · minified",
                next_rust_build::tailwind::VERSION,
                next_rust_build::tailwind::scanned_dirs(&info.config)
                    .iter()
                    .map(|d| d.strip_prefix(&info.root).unwrap_or(d).display().to_string() + "/")
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    // 2. Compile
    let compile_started = Instant::now();
    let mut live = ui::LiveLine::new();
    let mut state = CompileState::default();
    let envs = release_env(&info);
    let envs: Vec<(&str, &str)> = envs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let result = project::cargo_build_live(&info, true, &envs, true, &mut |event| {
        state.update(event);
        let (label, detail) = state.describe(compile_started.elapsed().as_secs_f64());
        live.draw(&label, &detail);
        state.log_milestone();
    });
    let (exe, warnings) = match result {
        Ok(v) => v,
        Err(report) => {
            live.fail("Compiling", &ui::red("failed"));
            eprintln!();
            eprintln!("{}", unmap_diagnostic_paths(&report));
            return Err("compilation failed".into());
        }
    };
    let secs = compile_started.elapsed().as_secs_f64();
    let compiled = match state.total {
        0 => format!("up to date · {secs:.1}s"),
        n => format!(
            "{n} units · {} · stripped · {secs:.1}s",
            match info.config.build.optimize {
                next_rust_core::config::Optimize::Size => "optimized for size",
                next_rust_core::config::Optimize::Speed => "optimized for speed",
            }
        ),
    };
    live.finish("Compiled", &compiled);
    if !warnings.is_empty() {
        eprintln!();
        eprint!("{}", unmap_diagnostic_paths(&warnings));
        eprintln!();
    }

    // 3. Embedded files (compiled into the binary by the release build).
    let embedded: Vec<String> = next_rust_build::codegen::embedded_dirs(&info.config)
        .iter()
        .filter_map(|(name, dir)| match next_rust_build::codegen::embeddable_files(dir).len() {
            0 => None,
            n => Some(format!("{n} in {name}/")),
        })
        .collect();
    let config_file = info.config.source.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
    let packed: Vec<String> = config_file.into_iter().chain(embedded).collect();
    ui::done_step("Packed", &if packed.is_empty() { "nothing extra to embed".into() } else { packed.join(" · ") });

    // Only the binary is written; earlier build layouts are removed.
    for old in ["server", "static", "cache", "manifest"] {
        let _ = std::fs::remove_dir_all(out.join(old));
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let dest = out.join(exe.file_name().ok_or("cargo reported an invalid executable path")?);
    let _ = std::fs::remove_file(&dest);
    std::fs::copy(&exe, &dest).map_err(|e| format!("copy {}: {e}", dest.display()))?;

    // 4. Check the binary on its own.
    let mut report = serde_json::Value::Null;
    if !a.flag(&["--no-check", "--no-export"]) {
        let check_started = Instant::now();
        let mut live = ui::LiveLine::new();
        let handle = {
            let dest = dest.clone();
            std::thread::spawn(move || check_binary(&dest))
        };
        while !handle.is_finished() {
            live.draw("Pre-rendering", &ui::dim("running the binary from an empty folder"));
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
        match handle.join().map_err(|_| "static generation crashed")? {
            Ok(r) => report = r,
            Err((stderr, e)) => {
                live.fail("Pre-rendering", &ui::red("failed"));
                eprintln!();
                eprintln!("{stderr}");
                return Err(e);
            }
        }
        let rendered = report["pages"].as_array().map_or(0, Vec::len);
        live.finish(
            "Pre-rendered",
            &format!(
                "{} · needs no other files · {} ms",
                plural(rendered, "static page"),
                check_started.elapsed().as_millis()
            ),
        );
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
    let bin_size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let shown = dest.strip_prefix(&info.root).unwrap_or(&dest).display().to_string();
    ui::done_step("Output", &format!("{shown} · {}", ui::bytes(bin_size)));

    // Route table
    eprintln!();
    eprintln!("   {}", ui::bold(&format!("{:<42} {:>10}", "Route", "Size")));
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
        eprintln!("   {symbol} {:<40} {:>10}  {extra}", r.route.pattern.to_display_string(), size);
    }
    eprintln!();
    eprintln!(
        "   {}",
        ui::dim(&format!(
            "{}  static (pre-rendered)   {}  dynamic (server-rendered)   {}  API",
            ui::green("○"),
            ui::yellow("λ"),
            ui::magenta("ƒ")
        ))
    );
    eprintln!();

    let port = info.config.server.port;
    ui::boxed(&[
        format!(
            "{} {}",
            ui::green("✔"),
            ui::bold(&format!("Build complete in {:.1}s", started.elapsed().as_secs_f64()))
        ),
        String::new(),
        format!("{}  {}", ui::pad(&ui::bold(&shown), 30), ui::dim(&ui::bytes(bin_size))),
        ui::dim("the whole app in one file: pages, config and static files"),
    ]);
    eprintln!();
    eprintln!("   {}", ui::bold("Run it"));
    eprintln!();
    eprintln!(
        "     {} {}      {}",
        ui::dim("$"),
        ui::accent("next-rust start"),
        ui::dim(&format!("→ http://localhost:{port}"))
    );
    eprintln!("     {} {}", ui::dim("$"), ui::accent(&format!("./{shown}")));
    eprintln!();
    eprintln!("   {}", ui::dim("Deploy: copy that one file to your server and run it."));
    eprintln!();
    Ok(())
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

/// Progress of `cargo build` as shown on the live line.
#[derive(Default)]
struct CompileState {
    done: usize,
    total: usize,
    active: String,
    phase: &'static str,
    /// Last 25% step printed in non-interactive output.
    milestone: usize,
}

impl CompileState {
    fn update(&mut self, event: project::CargoEvent) {
        match event {
            project::CargoEvent::Progress { done, total, active } => {
                self.done = done;
                self.total = total;
                self.active = active.to_owned();
                self.phase = "Compiling";
            }
            project::CargoEvent::Status { verb, rest } => match verb {
                "Updating" | "Locking" | "Adding" => self.phase = "Resolving",
                "Downloading" | "Downloaded" => self.phase = "Downloading",
                "Compiling" if self.total == 0 => {
                    self.phase = "Compiling";
                    self.active = rest.split_whitespace().next().unwrap_or("").to_owned();
                }
                _ => {}
            },
            project::CargoEvent::Tick => {}
        }
    }

    fn describe(&self, secs: f64) -> (String, String) {
        let time = ui::dim(&format!("{secs:.0}s"));
        match self.phase {
            "Resolving" => ("Resolving".into(), format!("{}  {time}", ui::dim("dependency versions"))),
            "Downloading" => ("Downloading".into(), format!("{}  {time}", ui::dim("crates from crates.io"))),
            _ if self.total > 0 => {
                let fraction = self.done as f64 / self.total as f64;
                // The last unit is the app itself; with link-time optimization
                // it takes a while after everything else is done.
                let linking = self.total - self.done <= 1;
                let label = if linking { "Optimizing" } else { "Compiling" };
                let now = if linking {
                    "link-time optimization, stripping symbols".to_owned()
                } else {
                    let mut names: Vec<&str> = Vec::new();
                    for name in self.active.split(", ").map(|n| n.split('(').next().unwrap_or(n)) {
                        if !names.contains(&name) {
                            names.push(name);
                        }
                    }
                    names.into_iter().take(3).collect::<Vec<_>>().join(", ")
                };
                (
                    label.into(),
                    format!(
                        "{} {}  {}  {}  {}",
                        ui::progress_bar(fraction, 24),
                        ui::bold(&format!("{:>3}%", (fraction * 100.0).floor() as u32)),
                        ui::dim(&format!("{}/{}", self.done, self.total)),
                        time,
                        ui::dim(&now)
                    ),
                )
            }
            _ => (
                "Compiling".into(),
                format!("{}  {time}", ui::dim(if self.active.is_empty() { "starting cargo" } else { &self.active })),
            ),
        }
    }

    /// Outside a terminal (CI logs), print a line at every quarter instead of
    /// redrawing.
    fn log_milestone(&mut self) {
        if ui::interactive() || self.total == 0 {
            return;
        }
        let quarter = self.done * 4 / self.total;
        if quarter > self.milestone {
            self.milestone = quarter;
            eprintln!("   … compiling {}% ({}/{})", quarter * 25, self.done, self.total);
        }
    }
}

/// Cargo settings for the production binary. They override the project's
/// `[profile.release]`, so every `next-rust build` gets the same result:
///
/// * `opt-level = "z"` (or `3` with `[build] optimize = "speed"`), fat LTO and
///   one codegen unit, so unused code across all crates is removed;
/// * symbols stripped and every build path rewritten, so the binary contains
///   no names or directories from the machine that built it;
/// * `panic` from `[build] panic` (`unwind` by default).
fn release_env(info: &ProjectInfo) -> Vec<(String, String)> {
    use next_rust_core::config::{Optimize, PanicStrategy};
    let build = &info.config.build;
    let mut envs: Vec<(String, String)> = vec![
        ("NEXT_RUST_EMBED".into(), "1".into()),
        (
            "CARGO_PROFILE_RELEASE_OPT_LEVEL".into(),
            match build.optimize {
                Optimize::Size => "z",
                Optimize::Speed => "3",
            }
            .into(),
        ),
        ("CARGO_PROFILE_RELEASE_LTO".into(), "fat".into()),
        ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS".into(), "1".into()),
        ("CARGO_PROFILE_RELEASE_STRIP".into(), "symbols".into()),
        ("CARGO_PROFILE_RELEASE_DEBUG".into(), "false".into()),
        ("CARGO_PROFILE_RELEASE_INCREMENTAL".into(), "false".into()),
        (
            "CARGO_PROFILE_RELEASE_PANIC".into(),
            match build.panic {
                PanicStrategy::Unwind => "unwind",
                PanicStrategy::Abort => "abort",
            }
            .into(),
        ),
    ];

    // Panic messages and `file!()` embed source paths. Rewrite them so no
    // user name, home directory or project location ends up in the binary.
    let mut flags: Vec<String> = match std::env::var("CARGO_ENCODED_RUSTFLAGS") {
        Ok(encoded) => encoded.split('\x1f').filter(|f| !f.is_empty()).map(str::to_owned).collect(),
        Err(_) => std::env::var("RUSTFLAGS").unwrap_or_default().split_whitespace().map(str::to_owned).collect(),
    };
    let mut remap = |from: std::path::PathBuf, to: &str| {
        if let Some(from) = from.to_str().filter(|f| !f.is_empty()) {
            flags.push(format!("--remap-path-prefix={from}={to}"));
        }
    };
    // Later mappings win, so the most specific come last.
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        remap(home.into(), "~");
    }
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".cargo")));
    if let Some(cargo_home) = cargo_home {
        remap(cargo_home, "cargo");
    }
    remap(info.root.clone(), PROJECT_ALIAS);
    // Generated code (routes) lives in Cargo's target directory.
    remap(info.target_dir.clone(), "target");
    envs.push(("CARGO_ENCODED_RUSTFLAGS".into(), flags.join("\x1f")));
    envs
}

/// Where `next-rust analyze` finds the last build's reports.
pub fn reports_dir(info: &ProjectInfo) -> std::path::PathBuf {
    info.target_dir.join("next-rust")
}

/// Run the binary from an empty directory and render every static page. This
/// proves it needs no files besides itself and catches rendering errors at
/// build time.
fn check_binary(bin: &Path) -> Result<serde_json::Value, (String, String)> {
    let empty = std::env::temp_dir().join(format!("next-rust-build-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).map_err(|e| (String::new(), e.to_string()))?;
    let output = Command::new(bin)
        .arg("--export")
        .current_dir(&empty)
        .env("NEXT_RUST_ENV", "production")
        .env_remove("NEXT_RUST_CONFIG")
        .output();
    let _ = std::fs::remove_dir_all(&empty);
    let output = output.map_err(|e| (String::new(), e.to_string()))?;
    if !output.status.success() {
        return Err((String::from_utf8_lossy(&output.stderr).into_owned(), "static generation failed".into()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().last().and_then(|l| serde_json::from_str(l).ok()).unwrap_or(serde_json::Value::Null))
}

/// What [`release_env`] renames the project directory to.
const PROJECT_ALIAS: &str = "app";

/// Diagnostics of a release build name files as `app/<path>` (see
/// [`release_env`]). Show them relative to the project again, so
/// `app/app/page.rs` reads (and is clickable) as `app/page.rs`.
fn unmap_diagnostic_paths(rendered: &str) -> String {
    let alias = format!("{PROJECT_ALIAS}/");
    let mut out = String::with_capacity(rendered.len());
    let mut rest = rendered;
    while let Some(at) = ["--> ", "::: "].iter().filter_map(|m| rest.find(m)).min() {
        let (head, tail) = rest.split_at(at + 4);
        out.push_str(head);
        // Colored output puts a reset sequence between the arrow and the path.
        let ansi = tail.len()
            - tail.trim_start_matches(|c: char| c == '\x1b' || c == '[' || c.is_ascii_digit() || c == 'm').len();
        out.push_str(&tail[..ansi]);
        rest = tail[ansi..].strip_prefix(alias.as_str()).unwrap_or(&tail[ansi..]);
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod unmap_tests {
    use super::unmap_diagnostic_paths;

    #[test]
    fn project_paths_are_shown_relative_to_the_project() {
        assert_eq!(unmap_diagnostic_paths("  --> app/app/page.rs:2:5\n"), "  --> app/page.rs:2:5\n");
        assert_eq!(
            unmap_diagnostic_paths("  \x1b[1m\x1b[94m--> \x1b[0mapp/src/lib.rs:1:1"),
            "  \x1b[1m\x1b[94m--> \x1b[0msrc/lib.rs:1:1"
        );
        assert_eq!(unmap_diagnostic_paths("   ::: app/src/x.rs:3:1"), "   ::: src/x.rs:3:1");
        assert_eq!(unmap_diagnostic_paths("  --> ~/.cargo/x.rs:1:1"), "  --> ~/.cargo/x.rs:1:1");
        assert_eq!(unmap_diagnostic_paths("no paths here"), "no paths here");
    }
}
