//! The motion stylesheet (`[animation]`): first in every document, sent once
//! per browser, and gone when turned off.

use bytes::Bytes;
use next_rust_cache::MemoryStore;
use next_rust_core::config::{AnimationConfig, LoaderConfig};
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

static TW: Stylesheet = Stylesheet {
    id: "tw-test",
    css: "@layer theme,base,components,utilities;@layer utilities{.p-4{padding:1rem}}",
    per_class: Some(&[]),
    scripts: &[],
};

static SHEETS: &[&Stylesheet] = &[&TW];

fn page_a(_ctx: Ctx) -> R {
    Box::pin(async { Ok(div![class("p-4"), Link!(href("/b"), "b")].into_node()) })
}

fn routes() -> Routes {
    Routes {
        pages: vec![PageDef {
            pattern: "/a",
            source: "test",
            body: PageBody::Rust(page_a),
            segments: vec![SegmentDef::default()],
            metadata: None,
            rendering: Rendering::Dynamic,
            revalidate: None,
            generate_params: None,
            dynamic_params: true,
            tags: &[],
            intercept_from: None,
        }],
        stylesheets: SHEETS,
        ..Default::default()
    }
}

fn build(animation: AnimationConfig) -> App {
    let config = Config { root: std::env::temp_dir(), animation, ..Config::default() };
    App::new(routes())
        .config(config)
        .environment(Environment::Production)
        .page_store(MemoryStore::new(10))
        .build()
        .unwrap()
}

async fn get(app: &App, known: &[&str]) -> String {
    let mut req = http::Request::get("/a");
    if !known.is_empty() {
        req = req.header("x-nr-nav", "1").header("x-nr-styles", known.join(","));
    }
    let res = app.handle(Request::from_http(req.body(Bytes::new()).unwrap())).await;
    assert_eq!(res.status, 200);
    res.into_text().await
}

/// `(id, css)` of every `<style data-nr-css>` in `html`, in order.
fn styles(html: &str) -> Vec<(String, String)> {
    html.split("<style data-nr-css=\"")
        .skip(1)
        .map(|rest| {
            let (id, rest) = rest.split_once("\">").unwrap();
            (id.to_owned(), rest.split("</style>").next().unwrap().to_owned())
        })
        .collect()
}

#[tokio::test]
async fn the_motion_sheet_opens_the_head_and_is_sent_once() {
    let app = build(AnimationConfig::default());
    let html = get(&app, &[]).await;
    let sheets = styles(&html);
    let (id, css) = &sheets[0];
    assert!(id.starts_with("nr-motion-"), "{id}");
    assert!(css.starts_with("@layer nr-motion,theme,base,components,utilities;"));
    assert!(css.contains("--nr-page-fade:250ms") && css.contains("@view-transition{navigation:auto}"));
    assert!(css.contains("*,::before,::after{transition-property:color,"));
    // Before Tailwind, so the app's layers come after (and win).
    assert!(sheets.iter().skip(1).any(|(id, _)| id.starts_with("tw-test")));
    let head = html.split("</head>").next().unwrap();
    assert!(head.contains(&format!("<style data-nr-css=\"{id}\">")));
    assert!(head.find("nr-motion-").unwrap() < head.find("tw-test").unwrap());
    // The runtime is there to run the fade on client navigations.
    assert!(html.contains("/_next-rust/runtime.js"));

    // A client navigation says it has the sheet: it is not sent again.
    let next = get(&app, &[id]).await;
    assert!(!next.contains("nr-motion-"), "sent twice");
    assert!(styles(&next).iter().all(|(id, _)| !id.starts_with("nr-motion-")));
}

#[tokio::test]
async fn animation_off_leaves_nothing_behind() {
    let app = build(AnimationConfig { enabled: false, ..AnimationConfig::default() });
    let html = get(&app, &[]).await;
    assert!(!html.contains("nr-motion") && !html.contains("view-transition") && !html.contains("--nr-page-fade"));
    // Only the loading bar remains ahead of Tailwind.
    let ids: Vec<String> = styles(&html).into_iter().map(|(id, _)| id).collect();
    assert!(ids[0].starts_with("nr-loader-") && ids[1].starts_with("tw-test"), "{ids:?}");
}

#[tokio::test]
async fn the_loading_bar_comes_with_every_document_once() {
    let app = build(AnimationConfig::default());
    let html = get(&app, &[]).await;
    let sheets = styles(&html);
    let (id, css) = sheets.iter().find(|(id, _)| id.starts_with("nr-loader-")).expect("loader sheet");
    assert!(css.contains("--nr-loader:1") && css.contains("html[data-nr-loading]::before"));
    assert!(css.contains("100% 4px no-repeat") && css.contains("var(--nr-primary, #006fee)"));
    // After the motion sheet, before Tailwind.
    let ids: Vec<&str> = sheets.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids[0].starts_with("nr-motion-") && ids[1] == id && ids[2].starts_with("tw-test"), "{ids:?}");
    // Client navigations that have it are not sent it again.
    let next = get(&app, &[id]).await;
    assert!(!next.contains("nr-loader-"));
}

#[tokio::test]
async fn the_loading_bar_can_be_restyled_or_removed() {
    let loader = LoaderConfig { color: "#f26b2a".into(), height: 3, spinner: true, ..LoaderConfig::default() };
    let config = Config { root: std::env::temp_dir(), loader, ..Config::default() };
    let app = App::new(routes()).config(config).environment(Environment::Production).build().unwrap();
    let html = get(&app, &[]).await;
    let (_, css) = styles(&html).into_iter().find(|(id, _)| id.starts_with("nr-loader-")).unwrap();
    assert!(
        css.contains("100% 3px no-repeat") && css.contains("drop-shadow(0 0 5px #f26b2a)") && css.contains("nr-spin")
    );

    let config = Config {
        root: std::env::temp_dir(),
        loader: LoaderConfig { enabled: false, ..LoaderConfig::default() },
        ..Config::default()
    };
    let app = App::new(routes()).config(config).environment(Environment::Production).build().unwrap();
    let html = get(&app, &[]).await;
    assert!(!html.contains("nr-loader") && !html.contains("data-nr-loading"));
}

#[tokio::test]
async fn pages_and_hover_are_separate_switches() {
    let app = build(AnimationConfig { pages: false, duration: 120, ..AnimationConfig::default() });
    let (_, css) = styles(&get(&app, &[]).await).swap_remove(0);
    assert!(css.contains("--nr-page-fade:0ms") && !css.contains("@view-transition"));
    assert!(css.contains("--nr-motion:120ms") && css.contains("transition-property:color"));

    let quiet = build(AnimationConfig { hover: false, ..AnimationConfig::default() });
    let (_, css) = styles(&get(&quiet, &[]).await).swap_remove(0);
    assert!(css.contains("@view-transition") && !css.contains("transition-property:color"));
}
