//! Partial navigation: shared layouts are not rendered or sent again.

use std::cell::Cell;

use bytes::Bytes;
use next_rust_cache::MemoryStore;
use next_rust_core::{Config, Environment};
use next_rust_router::Params;
use next_rust_server::app::{LayoutFn, RenderFn};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

// Per test thread: tests run in parallel, each on its own thread.
thread_local! {
    static ROOT_RUNS: Cell<usize> = const { Cell::new(0) };
    static DOCS_RUNS: Cell<usize> = const { Cell::new(0) };
}

fn root_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    ROOT_RUNS.set(ROOT_RUNS.get() + 1);
    Box::pin(async move {
        Ok(body_wrap![
            header![a![href("/docs/a"), active_class("on"), "A"], a![href("/docs"), active_class_prefix("in"), "Docs"]],
            children
        ])
    })
}
macro_rules! body_wrap {
    ($($part:expr),* $(,)?) => { div![id("site"), $($part),*].into_node() };
}
use body_wrap;

fn docs_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    DOCS_RUNS.set(DOCS_RUNS.get() + 1);
    Box::pin(async move { Ok(section![id("docs-shell"), nav!["sidebar"], children].into_node()) })
}
fn params_layout(ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        let params = Params::from_context(&ctx)?;
        Ok(div![class("team"), format!("team {}", params.get("team").unwrap_or("")), children].into_node())
    })
}
fn cookie_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move { Ok(div![class("account"), children].into_node()) })
}
fn page_a(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["Page A"].into_node()) })
}
/// Stands in for the generated Tailwind CSS.
static APP_CSS: Stylesheet = Stylesheet { id: "tw-app", css: ".p-4{padding:1rem}" };
static APP_STYLESHEETS: &[&Stylesheet] = &[&APP_CSS];
static PAGE_B_CSS: Stylesheet = Stylesheet { id: "pagebcss", css: "h1{color:red}" };
fn page_b(_ctx: Ctx) -> R {
    Box::pin(async { Ok(fragment![&PAGE_B_CSS, h1!["Page B"]]) })
}
fn page_member(ctx: Ctx) -> R {
    Box::pin(async move { Ok(h1![format!("member {}", ctx.params.get("member").unwrap_or(""))].into_node()) })
}
fn root_metadata(_ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async { Ok(Metadata::new().icon("/logo.svg").apple_touch_icon("/touch.png").description("Acme docs")) })
}
fn metadata_b(_ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async { Ok(Metadata::new().title("B")) })
}

fn seg(layout: LayoutFn, id: &'static str) -> SegmentDef {
    SegmentDef { layout: Some(layout), layout_id: id, layout_reusable: true, ..Default::default() }
}

fn page(pattern: &'static str, body: RenderFn, segments: Vec<SegmentDef>) -> PageDef {
    PageDef {
        pattern,
        source: "test",
        body: PageBody::Rust(body),
        segments,
        metadata: None,
        rendering: Rendering::Dynamic,
        revalidate: None,
        generate_params: None,
        dynamic_params: true,
        tags: &[],
        intercept_from: None,
    }
}

fn app(env: Environment, static_b: bool) -> App {
    let root = || SegmentDef { metadata: Some(root_metadata), ..seg(root_layout, "app/layout.rs") };
    let docs = || seg(docs_layout, "app/docs/layout.rs");
    let mut b = page("/docs/b", page_b, vec![root(), docs(), SegmentDef::default()]);
    b.metadata = Some(metadata_b);
    if static_b {
        b.rendering = Rendering::Static;
    }
    let team = SegmentDef { layout_uses_params: true, ..seg(params_layout, "app/teams/[team]/layout.rs") };
    let account = SegmentDef {
        layout: Some(cookie_layout),
        layout_id: "app/account/layout.rs",
        layout_reusable: false,
        ..Default::default()
    };
    let with_template = SegmentDef { template: Some(docs_layout), ..seg(docs_layout, "app/t/layout.rs") };
    let routes = Routes {
        pages: vec![
            page("/docs/a", page_a, vec![root(), docs(), SegmentDef::default()]),
            b,
            page("/teams/[team]/[member]", page_member, vec![root(), team.clone(), team, SegmentDef::default()]),
            page("/account/a", page_a, vec![root(), account.clone(), SegmentDef::default()]),
            page("/account/b", page_b, vec![root(), account, SegmentDef::default()]),
            page("/t/a", page_a, vec![root(), with_template.clone(), SegmentDef::default()]),
            page("/t/b", page_b, vec![root(), with_template, SegmentDef::default()]),
        ],
        root: SegmentDef { metadata: Some(root_metadata), ..seg(root_layout, "app/layout.rs") },
        build_id: "build-1",
        stylesheets: APP_STYLESHEETS,
        ..Default::default()
    };
    let root =
        std::env::temp_dir().join(format!("nr-partial-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(root.join("public")).unwrap();
    std::fs::write(root.join("public/logo.svg"), "<svg/>").unwrap();
    let config = Config { root, ..Config::default() };
    App::new(routes).config(config).environment(env).page_store(MemoryStore::new(100)).build().unwrap()
}

struct Res {
    status: u16,
    headers: http::HeaderMap,
    body: String,
}

async fn get(app: &App, path: &str, layouts: Option<&[String]>) -> Res {
    get_with_styles(app, path, layouts, "").await
}

async fn get_with_styles(app: &App, path: &str, layouts: Option<&[String]>, styles: &str) -> Res {
    let mut req = http::Request::get(path);
    if let Some(keys) = layouts {
        req = req.header("x-nr-nav", "1").header("x-nr-layouts", keys.join(","));
    }
    if !styles.is_empty() {
        req = req.header("x-nr-styles", styles);
    }
    let res = app.handle(Request::from_http(req.body(Bytes::new()).unwrap())).await;
    let status = res.status.as_u16();
    let headers = res.headers.clone();
    Res { status, headers, body: res.into_text().await }
}

/// Layout keys in document order, as the client runtime collects them.
fn keys(html: &str) -> Vec<String> {
    html.split("<!--nr-l:").skip(1).filter_map(|rest| rest.split("-->").next()).map(str::to_owned).collect()
}

#[tokio::test]
async fn full_pages_mark_every_reusable_layout() {
    let app = app(Environment::Test, false);
    let res = get(&app, "/docs/a", None).await;
    assert_eq!(res.status, 200);
    assert_eq!(keys(&res.body).len(), 2, "root and docs layouts: {}", res.body);
    let k = keys(&res.body);
    let (root_open, docs_open) = (format!("<!--nr-l:{}-->", k[0]), format!("<!--nr-l:{}-->", k[1]));
    let (root_close, docs_close) = (format!("<!--/nr-l:{}-->", k[0]), format!("<!--/nr-l:{}-->", k[1]));
    let pos = |needle: &str| res.body.find(needle).unwrap_or_else(|| panic!("{needle} missing"));
    assert!(pos(&root_open) < pos("docs-shell") && pos("docs-shell") < pos(&docs_open));
    assert!(pos(&docs_open) < pos("Page A") && pos("Page A") < pos(&docs_close) && pos(&docs_close) < pos(&root_close));
    assert!(res.headers.get("x-nr-partial").is_none());
}

#[tokio::test]
async fn navigation_renders_only_below_the_deepest_shared_layout() {
    let app = app(Environment::Test, false);
    let full = get(&app, "/docs/a", None).await;
    let shown = keys(&full.body);

    let (root_before, docs_before) = (ROOT_RUNS.get(), DOCS_RUNS.get());
    let res = get(&app, "/docs/b", Some(&shown)).await;
    assert_eq!(res.status, 200);
    assert_eq!(res.headers["x-nr-partial"], shown[1].as_str(), "inside the docs layout");
    assert!(res.headers["cache-control"].to_str().unwrap().contains("no-store"));
    assert!(res.body.contains("Page B"));
    assert!(res.body.contains("<title>B</title>"), "metadata is still complete: {}", res.body);
    assert!(res.body.contains(r#"<meta name="description" content="Acme docs">"#), "{}", res.body);
    assert!(full.body.contains(r#"<link rel="icon" href="/logo.svg?v="#), "a full page sends icons: {}", full.body);
    assert!(full.body.contains(r#"rel="apple-touch-icon""#));
    assert!(
        !res.body.contains("rel=\"icon\"") && !res.body.contains("apple-touch-icon"),
        "icons are not sent again: {}",
        res.body
    );
    assert!(
        !res.body.contains("docs-shell") && !res.body.contains("site"),
        "shared layouts are not sent: {}",
        res.body
    );
    assert_eq!(ROOT_RUNS.get(), root_before, "root layout did not run");
    assert_eq!(DOCS_RUNS.get(), docs_before, "docs layout did not run");

    // Only the root layout on screen: render inside it.
    let res = get(&app, "/docs/b", Some(&shown[..1])).await;
    assert_eq!(res.headers["x-nr-partial"], shown[0].as_str());
    assert!(res.body.contains("docs-shell") && res.body.contains("Page B") && !res.body.contains("id=\"site\""));
    assert_eq!(keys(&res.body), vec![shown[1].clone()], "the docs layout inside the region is marked again");

    // Stylesheets the browser already has are not sent again.
    assert!(get(&app, "/docs/b", Some(&shown)).await.body.contains(r#"<style data-nr-css="pagebcss">"#));
    let res = get_with_styles(&app, "/docs/b", Some(&shown), "other,pagebcss").await;
    assert!(!res.body.contains("pagebcss") && res.body.contains("Page B"), "{}", res.body);

    // Unknown keys (another build, another page): full page.
    let res = get(&app, "/docs/b", Some(&["0123456789abcdef".to_owned()])).await;
    assert!(res.headers.get("x-nr-partial").is_none());
    assert!(res.body.contains("id=\"site\""));
}

#[tokio::test]
async fn layouts_reading_the_request_or_params_are_rendered_again() {
    let app = app(Environment::Test, false);

    // A layout that is not reusable (it reads cookies) is never skipped.
    let full = get(&app, "/account/a", None).await;
    let shown = keys(&full.body);
    assert_eq!(shown.len(), 1, "only the root layout is marked");
    let res = get(&app, "/account/b", Some(&shown)).await;
    assert_eq!(res.headers["x-nr-partial"], shown[0].as_str());
    assert!(res.body.contains("account") && res.body.contains("Page B"));

    // A layout reading params is reused only while the params are equal.
    let full = get(&app, "/teams/red/ann", None).await;
    let shown = keys(&full.body);
    assert_eq!(shown.len(), 3);
    let same_team = get(&app, "/teams/red/bob", Some(&shown)).await;
    assert!(same_team.headers.get("x-nr-partial").is_some());
    assert!(same_team.body.contains("team red"), "params changed (member), so the team layouts render again");
    let other_team = get(&app, "/teams/blue/ann", Some(&shown)).await;
    assert_eq!(other_team.headers["x-nr-partial"], shown[0].as_str());
    assert!(other_team.body.contains("team blue"));

    // A template renders on every navigation: nothing below it is reused.
    let full = get(&app, "/t/a", None).await;
    let shown = keys(&full.body);
    let res = get(&app, "/t/b", Some(&shown)).await;
    assert_eq!(res.headers["x-nr-partial"], shown[1].as_str(), "the template is inside its own layout's region");
}

#[tokio::test]
async fn static_pages_are_cut_from_the_cache() {
    let app = app(Environment::Production, true);
    let full = get(&app, "/docs/a", None).await;
    let shown = keys(&full.body);
    let first = get(&app, "/docs/b", Some(&shown)).await;
    assert_eq!(first.headers["x-nr-partial"], shown[1].as_str());
    assert_eq!(first.headers["x-nr-cache"], "MISS");
    let second = get(&app, "/docs/b", Some(&shown)).await;
    assert_eq!(second.headers["x-nr-cache"], "HIT");
    assert!(second.body.contains("Page B") && !second.body.contains("docs-shell"), "{}", second.body);
    assert!(second.headers["cache-control"].to_str().unwrap().contains("private"), "never cached by a CDN");
    assert!(second.body.contains("pagebcss"));
    assert!(!second.body.contains("rel=\"icon\"") && !second.body.contains("apple-touch-icon"), "{}", second.body);
    assert!(second.body.contains(r#"<meta name="description" content="Acme docs">"#));
    let known = get_with_styles(&app, "/docs/b", Some(&shown), "pagebcss").await;
    assert!(!known.body.contains("pagebcss") && known.body.contains("Page B"), "{}", known.body);
    let plain = get(&app, "/docs/b", None).await;
    assert!(plain.body.contains("docs-shell"), "normal requests still get the whole page");
    assert!(plain.body.contains(r#"<link rel="icon" href="/logo.svg?v="#), "and the icons");
}

#[tokio::test]
async fn active_links_are_marked_on_the_server() {
    let app = app(Environment::Test, false);
    let res = get(&app, "/docs/a", None).await;
    assert!(
        res.body.contains(r#"<a href="/docs/a" data-nr-active="on" class="on" aria-current="page">A</a>"#),
        "{}",
        res.body
    );
    assert!(res.body.contains(r#"data-nr-active-prefix="in" class="in" aria-current="page">Docs</a>"#), "{}", res.body);
    let res = get(&app, "/account/a", None).await;
    assert!(!res.body.contains("aria-current"), "{}", res.body);
}

#[tokio::test]
async fn not_found_pages_keep_the_root_layout() {
    let app = app(Environment::Test, false);
    let page = get(&app, "/docs/a", None).await;
    let shown = keys(&page.body);
    let full = get(&app, "/missing", None).await;
    assert_eq!(full.status, 404);
    assert_eq!(keys(&full.body), vec![shown[0].clone()], "the 404 page marks the root layout: {}", full.body);
    let nav = get(&app, "/missing", Some(&shown)).await;
    assert_eq!(nav.status, 404);
    assert_eq!(nav.headers["x-nr-partial"], shown[0].as_str());
    assert!(!nav.body.contains("id=\"site\""), "the root layout is not sent again: {}", nav.body);
    assert!(full.body.contains("rel=\"icon\"") && !nav.body.contains("rel=\"icon\""), "{}", nav.body);
}

#[tokio::test]
async fn icons_use_versioned_urls_that_browsers_keep() {
    let app = app(Environment::Test, false);
    let page = get(&app, "/docs/a", None).await;
    let start = page.body.find("/logo.svg?v=").expect("versioned icon URL") + "/logo.svg?v=".len();
    let version = &page.body[start..start + 10];
    assert!(version.bytes().all(|b| b.is_ascii_hexdigit()), "{version}");
    assert!(page.body.contains(r#"<link rel="apple-touch-icon" href="/touch.png">"#), "missing files keep their URL");

    let icon = get(&app, &format!("/logo.svg?v={version}"), None).await;
    assert_eq!(icon.status, 200);
    assert_eq!(icon.headers["cache-control"], "public, max-age=31536000, immutable");
    let plain = get(&app, "/logo.svg", None).await;
    assert_eq!(plain.headers["cache-control"], "public, max-age=0");
    let stale = get(&app, "/logo.svg?v=0000000000", None).await;
    assert_eq!(stale.headers["cache-control"], "public, max-age=0", "an old version is not cached forever");
}

#[tokio::test]
async fn app_stylesheets_are_on_every_page_once() {
    let app = app(Environment::Test, false);
    let full = get(&app, "/docs/b", None).await;
    let head = full.body.split("</head>").next().unwrap();
    assert_eq!(head.matches(r#"<style data-nr-css="tw-app">"#).count(), 1, "{}", full.body);
    assert!(head.find("tw-app").unwrap() < head.find("pagebcss").unwrap(), "app styles come before page styles");
    let missing = get(&app, "/missing", None).await;
    assert!(missing.body.contains(r#"data-nr-css="tw-app""#), "404 pages are styled too");

    let shown = keys(&get(&app, "/docs/a", None).await.body);
    let partial = get_with_styles(&app, "/docs/b", Some(&shown), "tw-app").await;
    assert!(!partial.body.contains("tw-app") && partial.body.contains("Page B"), "{}", partial.body);
}
