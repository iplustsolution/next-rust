//! `[build] inline_assets`: documents carry their scripts, navigations only
//! what the browser lacks, and lazy modules come from `/_next-rust/m/<hash>`.

use bytes::Bytes;
use next_rust_cache::MemoryStore;
use next_rust_core::{Config, Environment};
use next_rust_server::inline::{InlineKind, InlineScript};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

static SCRIPTS: &[InlineScript] = &[
    InlineScript { key: "assets/js/site.js", id: "51te", kind: InlineKind::Plain, code: "site()", deps: &[] },
    InlineScript {
        key: "client/chart.js",
        id: "c4a7",
        kind: InlineKind::Factory,
        code: "async(i)=>{await i(\"client/scale.js\");return{}}",
        deps: &["client/scale.js"],
    },
    InlineScript { key: "client/scale.js", id: "5ca1e", kind: InlineKind::Factory, code: "async()=>({})", deps: &[] },
    InlineScript {
        key: "client/vendor/big.js",
        id: "0123456789abcdef",
        kind: InlineKind::Lazy,
        code: "export const big=1;",
        deps: &[],
    },
];

static DECLARED: &[(&str, &str)] = &[("client/chart.js", "export function hydrate() {}")];

fn page(_ctx: Ctx) -> R {
    Box::pin(async {
        let island = Element::new("nr-island")
            .with(attr("data-component", "Chart"))
            .with(attr("data-module", "/_next-rust/client/chart.js"))
            .with("chart");
        let site = Element::new("script")
            .with(attr("type", "module"))
            .with(attr("src", "/_next-rust/assets/js/site.0123456789abcdef.js"));
        Ok(div![island, Link!(href("/a"), "again"), site].into_node())
    })
}

/// A page rendered once and kept for every visitor.
fn stored(_ctx: Ctx) -> R {
    Box::pin(async {
        let island = Element::new("nr-island")
            .with(attr("data-component", "Chart"))
            .with(attr("data-module", "/_next-rust/client/chart.js"))
            .with("chart");
        Ok(div![island, p!["kept"]].into_node())
    })
}

fn build(inline_assets: bool) -> App {
    let def = |pattern: &'static str, body: fn(Ctx) -> R, rendering: Rendering| PageDef {
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
    };
    let routes = Routes {
        pages: vec![def("/a", page, Rendering::Dynamic), def("/s", stored, Rendering::Static)],
        inline_scripts: SCRIPTS,
        declared_scripts: DECLARED,
        ..Default::default()
    };
    let mut config = Config { root: std::env::temp_dir().join("nr-inline-test-root"), ..Config::default() };
    config.build.inline_assets = inline_assets;
    App::new(routes)
        .config(config)
        .environment(Environment::Production)
        .page_store(MemoryStore::new(10))
        .build()
        .unwrap()
}

async fn get(app: &App, uri: &str, headers: &[(&str, &str)]) -> Response {
    let mut req = http::Request::get(uri);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    app.handle(Request::from_http(req.body(Bytes::new()).unwrap())).await
}

#[tokio::test]
async fn documents_carry_their_scripts() {
    let app = build(true);
    let html = get(&app, "/a", &[]).await.into_text().await;
    assert!(!html.contains("<script src") && !html.contains(" src=\"/_next-rust/"), "{html}");
    let head = html.split("</head>").next().unwrap();
    assert!(head.contains("data-nr-js=\"5ca1e\"") && head.contains("data-nr-js=\"c4a7\""), "{html}");
    assert!(head.find("5ca1e").unwrap() < head.find("c4a7").unwrap(), "imports are registered first: {html}");
    assert!(html.contains("data-nr-js=\"51te\">site()</script>"), "{html}");
    // The runtime itself is in the page, with the nonce of this response.
    assert!(html.contains("data-nr-js=\"rt-") && html.contains("window.nextRust"), "{html}");
    let nonce = html.split("data-nr-js=\"c4a7\" nonce=\"").nth(1).unwrap().split('"').next().unwrap().to_owned();
    assert_eq!(nonce.len(), 32);
    assert!(html.matches(&format!("nonce=\"{nonce}\"")).count() >= 3, "{html}");
}

#[tokio::test]
async fn navigations_skip_what_the_document_has() {
    let app = build(true);
    let res = get(&app, "/a", &[("x-nr-nav", "1"), ("x-nr-scripts", "5ca1e, 51te")]).await;
    assert!(res.header("vary").is_some_and(|v| v.contains("x-nr-scripts")));
    let html = res.into_text().await;
    assert!(!html.contains("data-nr-js=\"rt-") && !html.contains("window.nextRust"), "{html}");
    assert!(!html.contains("data-nr-js=\"5ca1e\"") && !html.contains("site()"), "{html}");
    assert!(html.contains("data-nr-js=\"c4a7\""), "{html}");
}

#[tokio::test]
async fn the_default_links_scripts() {
    let app = build(false);
    let html = get(&app, "/a", &[]).await.into_text().await;
    assert!(html.contains("src=\"/_next-rust/runtime.js?v=") && html.contains("src=\"/_next-rust/assets/js/site."));
    assert!(!html.contains("data-nr-js"), "{html}");
}

#[tokio::test]
async fn lazy_and_declared_modules_are_served() {
    let app = build(true);
    let res = get(&app, "/_next-rust/m/0123456789abcdef", &[]).await;
    assert_eq!(res.status, 200);
    assert_eq!(res.header("cache-control"), Some("public, max-age=31536000, immutable"));
    assert!(res.header("content-type").unwrap().starts_with("text/javascript"));
    assert_eq!(res.into_text().await, "export const big=1;");
    // Only lazy modules, only by their hash.
    assert_eq!(get(&app, "/_next-rust/m/c4a7", &[]).await.status, 404);
    assert_eq!(get(&app, "/_next-rust/m/nope", &[]).await.status, 404);
    // A module declared in Rust has no file; development builds serve its source.
    let res = get(&app, "/_next-rust/client/chart.js", &[]).await;
    assert_eq!(res.status, 200);
    assert_eq!(res.into_text().await, "export function hydrate() {}");
    assert_eq!(get(&app, "/_next-rust/client/../client/chart.js", &[]).await.status, 404);
}

/// `x-nr-scripts` says what the sender's browser has. It must not reach
/// anything shared: the page kept for everyone else still carries every
/// script, and the answer to the sender is nobody else's to cache.
#[tokio::test]
async fn a_navigation_cannot_take_scripts_from_the_next_visitor() {
    let app = build(true);
    let res = get(&app, "/s", &[("x-nr-nav", "1"), ("x-nr-scripts", "5ca1e,c4a7,rt-x")]).await;
    let cache = res.header("cache-control").unwrap_or_default().to_owned();
    assert!(cache.contains("private") && cache.contains("no-store"), "{cache}");
    assert!(res.header("vary").is_some_and(|v| v.contains("x-nr-nav") && v.contains("x-nr-scripts")), "{res:?}");
    let first = res.into_text().await;
    assert!(!first.contains("data-nr-js=\"5ca1e\""), "{first}");
    // The same page, freshly asked for: nothing was dropped from the store.
    let html = get(&app, "/s", &[]).await.into_text().await;
    assert!(html.contains("data-nr-js=\"5ca1e\"") && html.contains("data-nr-js=\"c4a7\""), "{html}");
    assert!(html.contains("window.nextRust"), "{html}");
    // 256 ids at most, and no id at all without the navigation header.
    let many: String = (0..400).map(|i| format!("id{i},")).collect::<String>() + "5ca1e";
    let html = get(&app, "/s", &[("x-nr-scripts", &many)]).await.into_text().await;
    assert!(html.contains("data-nr-js=\"5ca1e\""), "{html}");
}
