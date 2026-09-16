use next_rust_build::analyze_project;
use next_rust_router::RouteKind;
use next_rust_router::manifest::relative;

use crate::{Args, project, ui};

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust routes [--json] [--tree] [--layouts]\n\nPrint every route with its methods, rendering mode and source file."
        );
        return Ok(());
    }
    let config = project::load_config()?;
    let project = analyze_project(&config);
    if a.flag(&["--json"]) {
        println!("{}", project.manifest().to_json());
        return super::report(&project);
    }
    if a.flag(&["--tree"]) {
        for t in &project.scan.trees {
            print!("{}", t.render_tree(&std::path::PathBuf::from(relative(&t.dir, &config.root))));
        }
        return super::report(&project);
    }
    print_table(&project, a.flag(&["--layouts"]));
    super::report(&project)
}

pub fn print_table(project: &next_rust_build::Project, layouts: bool) {
    let root = &project.config.root;
    let rows: Vec<(String, String, String, String)> = project
        .routes
        .iter()
        .map(|r| {
            let methods = r.methods.iter().filter(|m| *m != "HEAD").cloned().collect::<Vec<_>>().join(",");
            let kind = match (r.route.kind, r.rendering) {
                (RouteKind::Api, _) => ui::magenta("api"),
                (_, "static") => ui::green("○ static"),
                _ => ui::yellow("λ dynamic"),
            };
            let mut path = r.route.pattern.to_display_string();
            if let Some(i) = &r.route.intercept {
                path = format!("{path} {}", ui::dim(&format!("(intercepts from {})", i.context.to_display_string())));
            }
            (methods, path, kind, relative(&r.route.source, root))
        })
        .collect();
    let w0 = rows.iter().map(|r| r.0.len()).max().unwrap_or(6).max(6);
    let w1 = rows.iter().map(|r| ui::strip_ansi(&r.1).chars().count()).max().unwrap_or(4).max(4);
    let w2 = 10;
    println!("{}", ui::bold(&format!("{:<w0$}  {:<w1$}  {:<w2$}  {}", "METHOD", "PATH", "RENDER", "SOURCE")));
    for (r, ar) in rows.iter().zip(&project.routes) {
        println!("{}  {}  {}  {}", ui::pad(&r.0, w0), ui::pad(&r.1, w1), ui::pad(&r.2, w2), ui::dim(&r.3));
        if layouts && ar.route.kind != RouteKind::Api {
            for (depth, l) in ar.route.layouts().enumerate() {
                println!(
                    "{}  {}{} {}",
                    " ".repeat(w0),
                    "  ".repeat(depth + 1),
                    ui::dim("└"),
                    ui::dim(&relative(l, root))
                );
            }
            if let Some(reason) = &ar.dynamic_reason
                && ar.route.kind != RouteKind::Api
            {
                println!("{}  {}", " ".repeat(w0), ui::dim(&format!("  dynamic because: {reason}")));
            }
        }
    }
    let statics = project.routes.iter().filter(|r| r.rendering == "static").count();
    let apis = project.routes.iter().filter(|r| r.route.kind == RouteKind::Api).count();
    println!(
        "\n{} routes: {} static, {} dynamic, {} api",
        project.routes.len(),
        statics,
        project.routes.len() - statics - apis,
        apis
    );
}
