use std::path::PathBuf;

use next_rust_build::{Generator, analyze_project, generate_code};
use next_rust_core::Config;

struct Tmp(PathBuf);

impl Tmp {
    fn new(files: &[(&str, &str)]) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "nr-codegen-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&root);
        for (path, content) in files {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }
        Tmp(root)
    }

    fn config(&self, sub: &str) -> Config {
        Config::discover(self.0.join(sub)).unwrap()
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const PAGE: &str = "use next_rust::prelude::*;\npub fn Page() -> impl View { h1![\"hi\"] }\n";

fn codes(p: &next_rust_build::Project) -> Vec<String> {
    p.diagnostics.iter().map(|d| d.code.clone()).collect()
}

#[test]
fn infers_rendering_modes_with_reasons() {
    let t = Tmp::new(&[
        ("app/page.rs", PAGE),
        (
            "app/layout.rs",
            "use next_rust::prelude::*;\npub fn Layout(children: Children) -> impl View { div![children] }\n",
        ),
        ("app/me/page.rs", "use next_rust::prelude::*;\npub fn Page(cookies: Cookies) -> impl View { h1![\"x\"] }\n"),
        (
            "app/posts/[slug]/page.rs",
            "use next_rust::prelude::*;\npub async fn generate_params() -> Vec<Params> { vec![] }\npub fn Page(p: Params) -> impl View { h1![\"x\"] }\n",
        ),
        ("app/users/[id]/page.rs", "use next_rust::prelude::*;\npub fn Page(p: Params) -> impl View { h1![\"x\"] }\n"),
        (
            "app/forced/page.rs",
            "use next_rust::prelude::*;\npub const RENDERING: Rendering = Rendering::Dynamic;\npub fn Page() -> impl View { h1![\"x\"] }\n",
        ),
        (
            "app/isr/page.rs",
            "use next_rust::prelude::*;\npub const REVALIDATE: u64 = 30;\npub fn Page() -> impl View { h1![\"x\"] }\n",
        ),
        (
            "app/api/x/route.rs",
            "use next_rust::prelude::*;\npub async fn GET() -> Response { Response::text(\"x\") }\npub async fn DELETE(req: Request) -> Response { Response::status(204) }\n",
        ),
    ]);
    let project = analyze_project(&t.config(""));
    assert!(!project.has_errors(), "{}", project.diagnostics);
    let modes: Vec<(String, &str, Option<String>)> = project
        .routes
        .iter()
        .map(|r| (r.route.pattern.to_fs_string(), r.rendering, r.dynamic_reason.clone()))
        .collect();
    let find = |p: &str| modes.iter().find(|m| m.0 == p).unwrap().clone();
    assert_eq!(find("/").1, "static");
    assert_eq!(find("/me").2.as_deref(), Some("`Cookies` in app/me/page.rs::Page"));
    assert_eq!(find("/posts/[slug]").1, "static");
    assert_eq!(find("/users/[id]").2.as_deref(), Some("dynamic segment without generate_params"));
    assert_eq!(find("/forced").2.as_deref(), Some("rendering = dynamic"));
    let isr = project.routes.iter().find(|r| r.route.pattern.to_fs_string() == "/isr").unwrap();
    assert_eq!(isr.revalidate, Some(30));
    let api = project.routes.iter().find(|r| r.route.pattern.to_fs_string() == "/api/x").unwrap();
    assert_eq!(api.methods, vec!["GET", "DELETE"]);

    let manifest = project.manifest();
    let me = manifest.routes.iter().find(|r| r.pattern == "/me").unwrap();
    assert_eq!(me.rendering, "dynamic");
    assert_eq!(me.layouts, vec!["app/layout.rs"]);
}

#[test]
fn reports_missing_exports_and_bad_signatures() {
    let t = Tmp::new(&[
        ("app/page.rs", "use next_rust::prelude::*;\nfn Page() -> impl View { h1![\"private\"] }\n"),
        ("app/layout.rs", "pub fn Wrapper() {}\n"),
        ("app/loading.rs", "use next_rust::prelude::*;\npub async fn Loading() -> impl View { p![\"x\"] }\n"),
        ("app/api/route.rs", "pub fn helper() {}\n"),
        (
            "app/api/bad/route.rs",
            "use next_rust::prelude::*;\npub async fn GET(a: Request, b: Request) -> Response { todo!() }\n",
        ),
        (
            "app/mw/middleware.rs",
            "use next_rust::prelude::*;\npub async fn middleware(req: Request) -> Response { todo!() }\n",
        ),
        ("app/mw/page.rs", "use next_rust::prelude::*;\npub fn Page(c: Children) -> impl View { c }\n"),
        ("app/data/page.rs", "use next_rust::prelude::*;\npub fn Page(Data(d): Data<u32>) -> impl View { d }\n"),
    ]);
    let project = analyze_project(&t.config(""));
    let mut c = codes(&project);
    c.sort();
    assert_eq!(
        c,
        vec!["NR0201", "NR0201", "NR0203", "NR0204", "NR0205", "NR0205", "NR0206", "NR0206"],
        "{}",
        project.diagnostics
    );
    let rendered = project.diagnostics.render(false);
    assert!(rendered.contains("`Page` exists but is not `pub`"));
    assert!(rendered.contains("help: pub fn Layout(children: Children) -> impl View"));
}

#[test]
fn syntax_errors_stop_generation_with_location() {
    let t = Tmp::new(&[("app/page.rs", "pub fn Page( {")]);
    let project = analyze_project(&t.config(""));
    assert_eq!(codes(&project), vec!["NR0200"]);
    assert!(project.diagnostics.0[0].locations[0].to_string_lossy().contains("app/page.rs:1:"));
}

#[test]
fn custom_and_monorepo_app_directories() {
    let t = Tmp::new(&[
        (
            "apps/site/next-rust.toml",
            "[app]\ndirectory = \"../../shared/pages\"\n[api]\ndirectory = \"backend\"\nprefix = \"/v2\"\n",
        ),
        ("apps/site/Cargo.toml", "[package]\nname = \"site\"\n"),
        (
            "apps/site/backend/users/route.rs",
            "use next_rust::prelude::*;\npub async fn GET() -> Response { Response::text(\"x\") }\n",
        ),
        ("shared/pages/page.rs", PAGE),
        ("shared/pages/(marketing)/pricing/page.rs", PAGE),
    ]);
    let config = t.config("apps/site");
    assert!(config.app_dir().ends_with("shared/pages"));
    let project = analyze_project(&config);
    assert!(!project.has_errors(), "{}", project.diagnostics);
    let paths: Vec<String> = project.routes.iter().map(|r| r.route.pattern.to_display_string()).collect();
    assert_eq!(paths, vec!["/", "/pricing", "/v2/users"]);
    let code = generate_code(&project);
    assert!(code.contains("shared/pages/(marketing)/pricing/page.rs"));
    assert!(code.contains("pattern: \"/v2/users\""));
}

#[test]
fn generated_code_shape() {
    let t = Tmp::new(&[
        (
            "app/layout.rs",
            "use next_rust::prelude::*;\npub fn metadata() -> Metadata { Metadata::new() }\npub fn Layout(children: Children, mut slots: Slots) -> impl View { div![children, slots.take(\"side\")] }\n",
        ),
        ("app/@side/default.rs", PAGE),
        (
            "app/middleware.rs",
            "use next_rust::prelude::*;\npub async fn middleware(req: Request, next: Next) -> Response { next.run(req).await }\n",
        ),
        (
            "app/page.rs",
            "use next_rust::prelude::*;\n#[derive(serde::Deserialize)] pub struct In { a: u8 }\n#[server_action]\npub async fn save(input: In) -> Result<()> { Ok(()) }\npub async fn load(p: Params) -> Result<u32> { Ok(1) }\npub async fn Page(Data(n): Data<u32>, q: Query) -> Result<impl View> { Ok(p![n]) }\n",
        ),
        (
            "app/blog/[slug]/page.rs",
            "use next_rust::prelude::*;\npub const REVALIDATE: u64 = 60;\npub const TAGS: &[&str] = &[\"posts\"];\npub fn generate_params() -> Vec<Params> { vec![] }\npub fn Page() -> impl View { p![\"x\"] }\n",
        ),
        ("app/legacy/page.html", "<p>legacy</p>"),
        ("app/not-found.rs", "use next_rust::prelude::*;\npub fn NotFound() -> impl View { p![\"404\"] }\n"),
        (
            "app/global-error.rs",
            "use next_rust::prelude::*;\npub fn GlobalError(info: ErrorInfo) -> impl View { p![info.message] }\n",
        ),
        ("app/sitemap.rs", "use next_rust::prelude::*;\npub async fn sitemap() -> Sitemap { Sitemap::new() }\n"),
    ]);
    let project = analyze_project(&t.config(""));
    assert!(!project.has_errors(), "{}", project.diagnostics);
    let code = generate_code(&project);
    for needle in [
        "use ::next_rust::__private as __nr;",
        "let __data = m",
        "::load(__nr::FromContext::from_context(&ctx)?).await?;",
        "::Page(__nr::Data(__data), __nr::FromContext::from_context(&ctx)?).await)",
        "body: __nr::PageBody::Html(include_str!(",
        "rendering: __nr::Rendering::Dynamic",
        "rendering: __nr::Rendering::Static",
        "::REVALIDATE as u64)",
        "::TAGS",
        "slots: vec![__nr::SlotDef { name: \"side\", render: Some(__render_m",
        "middleware: Some(__middleware_m",
        "global_error: Some(__error_m",
        "sitemap: Some(__sitemap_m",
        "__nr::ActionDef { id: m",
        "::__NR_ACTION_ID_save, handler: m",
    ] {
        assert!(code.contains(needle), "missing `{needle}` in generated code:\n{code}");
    }
    // The app-root middleware is global and never duplicated per segment.
    assert_eq!(code.matches("middleware: Some(").count(), 1, "{code}");

    // Deterministic output.
    assert_eq!(code, generate_code(&analyze_project(&t.config(""))));

    // Generator writes the file and manifest.
    let out = t.0.join("out");
    let project = Generator::new().manifest_dir(&t.0).out_dir(&out).try_run().unwrap();
    assert_eq!(project.routes.len(), 3);
    assert!(out.join("next_rust_routes.rs").is_file());
    assert!(out.join("next_rust_manifest.json").is_file());
}

#[test]
fn generator_reports_errors_as_text() {
    let t = Tmp::new(&[("app/a/page.rs", PAGE), ("app/(g)/a/page.rs", PAGE)]);
    let err = Generator::new().manifest_dir(&t.0).out_dir(t.0.join("out")).try_run().err().unwrap();
    assert!(err.contains("NR0102"), "{err}");
    assert!(err.contains("Duplicate route"));
}
