//! Utility CSS (Tailwind) is sent per page: only the rules for the classes a
//! page renders, and on client navigations only what the browser lacks.

use std::time::Duration;

use bytes::Bytes;
use next_rust_cache::MemoryStore;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// Stands in for the generated Tailwind CSS (minified, as the build writes it).
static TW: Stylesheet = Stylesheet {
    id: "tw-test",
    css: "@layer theme,base,components,utilities;@layer theme{:root{--c:red}}@layer base{*{margin:0}}\
@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}.mt-2{margin-top:.5rem}\
@media (width>=40rem){.sm\\:p-10{padding:2.5rem}}.js-open{display:block}}",
    per_class: Some(&["js-open"]),
    // `client/home.js` toggles `mt-2`: pages loading it get the rule.
    scripts: &[("client/home.js", &["mt-2"])],
};
static SHEETS: &[&Stylesheet] = &[&TW];

fn page_a(_ctx: Ctx) -> R {
    Box::pin(async { Ok(div![class("p-4"), "A"].into_node()) })
}
fn page_b(_ctx: Ctx) -> R {
    Box::pin(async { Ok(div![class("p-4 px-2"), "B"].into_node()) })
}
fn page_px(_ctx: Ctx) -> R {
    Box::pin(async { Ok(div![class("px-2"), "PX"].into_node()) })
}
fn page_island(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(div![
            class("p-4"),
            Element::new("nr-island").with(attr("data-module", "/_next-rust/client/home.js")).with("x"),
        ]
        .into_node())
    })
}
fn page_static(_ctx: Ctx) -> R {
    Box::pin(async { Ok(div![class("sm:p-10 p-4"), "S"].into_node()) })
}
fn page_stream(_ctx: Ctx) -> R {
    Box::pin(async {
        let slow = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            p![class("mt-2"), "late"]
        };
        Ok(div![class("p-4"), suspense(p!["…"], slow)].into_node())
    })
}

fn page(pattern: &'static str, body: app::RenderFn, rendering: Rendering) -> PageDef {
    PageDef {
        pattern,
        source: "test",
        body: PageBody::Rust(body),
        segments: vec![SegmentDef::default()],
        metadata: None,
        rendering,
        revalidate: None,
        generate_params: None,
        dynamic_params: true,
        tags: &[],
        intercept_from: None,
    }
}

fn app(env: Environment) -> App {
    let routes = Routes {
        pages: vec![
            page("/a", page_a, Rendering::Dynamic),
            page("/b", page_b, Rendering::Dynamic),
            page("/px", page_px, Rendering::Dynamic),
            page("/island", page_island, Rendering::Dynamic),
            page("/static", page_static, Rendering::Static),
            page("/stream", page_stream, Rendering::Dynamic),
        ],
        stylesheets: SHEETS,
        ..Default::default()
    };
    let config = Config { root: std::env::temp_dir(), ..Config::default() };
    App::new(routes).config(config).environment(env).page_store(MemoryStore::new(100)).build().unwrap()
}

async fn get(app: &App, path: &str, known: &[&str]) -> String {
    let mut req = http::Request::get(path);
    if !known.is_empty() {
        req = req.header("x-nr-nav", "1").header("x-nr-styles", known.join(","));
    }
    let res = app.handle(Request::from_http(req.body(Bytes::new()).unwrap())).await;
    assert_eq!(res.status, 200);
    res.into_text().await
}

/// `(id, css)` of every utility `<style>` in `html`.
fn styles(html: &str) -> Vec<(String, String)> {
    html.split("<style data-nr-css=\"")
        .skip(1)
        .map(|rest| {
            let (id, rest) = rest.split_once("\">").unwrap();
            (id.to_owned(), rest.split("</style>").next().unwrap().to_owned())
        })
        .collect()
}

const BASE: &str = "@layer theme,base,components,utilities;@layer theme{:root{--c:red}}@layer base{*{margin:0}}";

#[tokio::test]
async fn pages_get_only_the_rules_they_use() {
    let app = app(Environment::Test);
    let a = styles(&get(&app, "/a", &[]).await);
    assert_eq!(a.len(), 1);
    assert!(a[0].0.starts_with("tw-test~"));
    // `js-open` is listed for code outside the view tree, so it is always kept.
    assert_eq!(a[0].1, format!("{BASE}@layer utilities{{.p-4{{padding:1rem}}.js-open{{display:block}}}}"));

    // Navigating to /b: no base CSS again, only the missing rule.
    let b = styles(&get(&app, "/b", &[&a[0].0]).await);
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].1, "@layer utilities{.px-2{padding-inline:.5rem}}");

    // Everything known: nothing sent.
    assert!(styles(&get(&app, "/b", &[&a[0].0, &b[0].0]).await).is_empty());
}

#[tokio::test]
async fn script_classes_go_only_to_pages_loading_the_script() {
    let app = app(Environment::Test);
    let a = styles(&get(&app, "/a", &[]).await);
    assert!(!a[0].1.contains(".mt-2"), "no island here");
    let island = styles(&get(&app, "/island", &[]).await);
    assert!(island[0].1.contains(".mt-2{margin-top:.5rem}"), "{}", island[0].1);
    assert_eq!(script_key("/_next-rust/assets/js/site.1a2b3c4d5e6f7a8b.js?v=1"), "assets/js/site.js");
    assert_eq!(script_key("/menu.js"), "menu.js");
}

#[tokio::test]
async fn rules_keep_their_order_across_navigations() {
    let app = app(Environment::Test);
    let px = styles(&get(&app, "/px", &[]).await);
    // /b needs `p-4`, which comes before `px-2`: `px-2` is sent again after
    // it so it still wins. `js-open` sets another property: not repeated.
    let b = styles(&get(&app, "/b", &[&px[0].0]).await);
    assert_eq!(b[0].1, "@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}}");
}

#[tokio::test]
async fn cached_static_pages_send_only_what_the_browser_lacks() {
    let app = app(Environment::Production);
    let first = styles(&get(&app, "/static", &[]).await);
    assert!(first[0].1.starts_with(BASE), "a first visit gets everything");
    assert!(first[0].1.contains(".sm\\:p-10") && !first[0].1.contains(".px-2"));
    let a = styles(&get(&app, "/a", &[]).await);
    let nav = styles(&get(&app, "/static", &[&a[0].0]).await);
    assert_eq!(nav[0].1, "@layer utilities{@media (width>=40rem){.sm\\:p-10{padding:2.5rem}}}");
}

#[tokio::test]
async fn streamed_chunks_bring_their_own_rules() {
    let app = app(Environment::Test);
    let html = get(&app, "/stream", &[]).await;
    let all = styles(&html);
    assert_eq!(all.len(), 2, "{html}");
    assert!(!all[0].1.contains("mt-2"), "not needed by the shell");
    assert_eq!(all[1].1, "@layer utilities{.mt-2{margin-top:.5rem}}");
    let chunk_style = html.rfind("<style").unwrap();
    assert!(chunk_style < html.find("<template").unwrap(), "rules arrive before the content");
}
