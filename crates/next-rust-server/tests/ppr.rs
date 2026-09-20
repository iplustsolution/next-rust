//! Partial prerendering: a cached static shell with `dynamic` holes that are
//! rendered for every request and streamed after it; cached fragments.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;
use next_rust_cache::{Cache, CacheOptions, MemoryStore};
use next_rust_core::{Config, Environment};
use next_rust_server::app::{LayoutFn, RenderFn};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// Stands in for the generated Tailwind CSS.
static TW: Stylesheet = Stylesheet {
    id: "tw-ppr",
    css: "@layer utilities{.p-4{padding:1rem}.mt-2{margin-top:.5rem}.px-2{padding-inline:.5rem}}",
    per_class: Some(&[]),
    scripts: &[],
};
static SHEETS: &[&Stylesheet] = &[&TW];
static WIDGET_CSS: Stylesheet = Stylesheet { id: "widgetcss", css: ".w{color:red}", per_class: None, scripts: &[] };

static SHELL_RUNS: AtomicUsize = AtomicUsize::new(0);
static HOLE_RUNS: AtomicUsize = AtomicUsize::new(0);
static COUNTED_SHELL_RUNS: AtomicUsize = AtomicUsize::new(0);
static COUNTED_HOLE_RUNS: AtomicUsize = AtomicUsize::new(0);
static SHARED_SHELL_RUNS: AtomicUsize = AtomicUsize::new(0);
static SHARED_HOLE_RUNS: AtomicUsize = AtomicUsize::new(0);
static LAYOUT_HOLE_RUNS: AtomicUsize = AtomicUsize::new(0);
static TAGGED_RUNS: AtomicUsize = AtomicUsize::new(0);
static FRAGMENT_RUNS: AtomicUsize = AtomicUsize::new(0);
static SHELL_FRAGMENT_RUNS: AtomicUsize = AtomicUsize::new(0);
static SHELL_WITH_FRAGMENT_RUNS: AtomicUsize = AtomicUsize::new(0);

fn greeting(ctx: &Ctx) -> String {
    format!("Hello {}", ctx.cookies.get("user").unwrap_or_else(|| "guest".into()))
}

/// A static product page with the visitor's name in a hole.
fn page_product(_ctx: Ctx) -> R {
    product(&SHELL_RUNS, &HOLE_RUNS)
}

fn page_shared(_ctx: Ctx) -> R {
    product(&SHARED_SHELL_RUNS, &SHARED_HOLE_RUNS)
}

/// The same page where only one test counts its renders.
fn page_counted(_ctx: Ctx) -> R {
    product(&COUNTED_SHELL_RUNS, &COUNTED_HOLE_RUNS)
}

fn product(shells: &'static AtomicUsize, holes: &'static AtomicUsize) -> R {
    Box::pin(async move {
        shells.fetch_add(1, Ordering::SeqCst);
        Ok(main![
            class("p-4"),
            h1!["Product"],
            dynamic(p!["Loading…"], move |ctx| async move {
                holes.fetch_add(1, Ordering::SeqCst);
                let q = ctx.extract::<Query>()?;
                Ok::<_, Error>(p![class("mt-2"), greeting(&ctx), format!(" ref={}", q.0.get("ref").unwrap_or("-"))])
            }),
            footer!["static footer"],
        ]
        .into_node())
    })
}

/// A hole that takes 300 ms.
fn page_slow(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![
            h1!["Slow"],
            dynamic(p!["…"], |ctx| async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                p![greeting(&ctx)]
            }),
        ]
        .into_node())
    })
}

/// The same work without a hole: the whole page waits.
fn page_blocking(ctx: Ctx) -> R {
    Box::pin(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(main![h1!["Slow"], p![greeting(&ctx)]].into_node())
    })
}

fn page_plain(_ctx: Ctx) -> R {
    Box::pin(async { Ok(main![h1!["Slow"], p!["Hello"]].into_node()) })
}

fn page_errors(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![
            dynamic(p!["a…"], |_ctx| async { Err::<Node, _>(Error::msg("boom")) }),
            Dynamic::new(p!["b…"])
                .on_error(|info| p![class("local"), format!("local {}", info.status)])
                .render(|_ctx| async { Err::<Node, _>(Error::http(409, "conflict")) }),
            dynamic(p!["c…"], |_ctx| async {
                if true {
                    panic!("hole panicked");
                }
                p!["never"]
            }),
            dynamic(p!["d…"], |_ctx| async { Err::<Node, _>(redirect("/login")) }),
            dynamic(p!["e…"], |ctx| async move { p![format!("fine {}", greeting(&ctx))] }),
        ]
        .into_node())
    })
}

fn error_boundary(_ctx: Ctx, info: ErrorInfo) -> Node {
    p![class("boundary"), format!("boundary caught {}", info.status)].into_node()
}

fn page_sets_cookie(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![dynamic((), |ctx| async move {
            ctx.cookies.set(Cookie::new("late", "1"));
            p!["tried"]
        })]
        .into_node())
    })
}

/// Reads the request outside a hole: not allowed in a shell.
fn page_leaky(ctx: Ctx) -> R {
    Box::pin(async move {
        let cookies = ctx.extract::<Cookies>()?;
        Ok(p![cookies.get("user").unwrap_or_default()].into_node())
    })
}

fn page_tagged(_ctx: Ctx) -> R {
    Box::pin(async {
        let n = TAGGED_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(main![p![format!("shell #{n}")], dynamic((), |ctx| async move { p![greeting(&ctx)] })].into_node())
    })
}

/// A hole whose content suspends again, and uses a class the shell does not.
fn page_nested(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![
            class("p-4"),
            dynamic(p!["outer…"], |ctx| async move {
                let name = greeting(&ctx);
                div![
                    class("px-2"),
                    &WIDGET_CSS,
                    suspense(p!["inner…"], async move {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        p![class("mt-2"), format!("inner {name}")]
                    })
                ]
            }),
        ]
        .into_node())
    })
}

fn page_fragment_in_hole(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![dynamic((), |ctx| async move {
            let list = cached("ppr:list", CacheOptions::revalidate(60).tag("ppr-list"), || async {
                let n = FRAGMENT_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
                ul![class("w mt-2"), &WIDGET_CSS, li![Link!(href = "/x", format!("list #{n}"))]]
            })
            .await?;
            Ok::<_, Error>(section![p![greeting(&ctx)], list])
        })]
        .into_node())
    })
}

fn page_fragment_in_shell(_ctx: Ctx) -> R {
    Box::pin(async {
        let n = SHELL_WITH_FRAGMENT_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
        let prices = cached("ppr:prices", CacheOptions::default().tag("ppr-prices"), || async {
            let n = SHELL_FRAGMENT_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
            p![format!("prices #{n}")]
        })
        .await?;
        Ok(main![p![format!("page #{n}")], prices, dynamic((), |ctx| async move { p![greeting(&ctx)] })].into_node())
    })
}

fn root_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        Ok(div![
            id("site"),
            header![dynamic(span!["…"], |ctx| async move {
                LAYOUT_HOLE_RUNS.fetch_add(1, Ordering::SeqCst);
                span![class("me"), greeting(&ctx)]
            })],
            children
        ]
        .into_node())
    })
}

fn page_in_layout(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![h1!["Inside"], dynamic(p!["…"], |ctx| async move { p![class("in"), greeting(&ctx)] })].into_node())
    })
}

fn page(pattern: &'static str, body: RenderFn, rendering: Rendering) -> PageDef {
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

fn layout_seg(layout: LayoutFn) -> SegmentDef {
    SegmentDef { layout: Some(layout), layout_id: "app/layout.rs", layout_reusable: true, ..Default::default() }
}

/// One data cache for every app of this process, as `cached` uses the global one.
fn shared_cache() -> Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Cache::new(MemoryStore::new(1000))).clone()
}

fn app_with(env: Environment, configure: impl FnOnce(&mut Config)) -> App {
    build(env, configure, true)
}

/// `leaky`: with the page that cannot be rendered statically.
fn build(env: Environment, configure: impl FnOnce(&mut Config), leaky: bool) -> App {
    let mut errors = page("/errors", page_errors, Rendering::Partial);
    errors.segments = vec![SegmentDef { error: Some(error_boundary), ..Default::default() }];
    let mut tagged = page("/tagged", page_tagged, Rendering::Partial);
    tagged.tags = &["ppr-products"];
    tagged.revalidate = Some(3600);
    let mut in_layout = page("/in-layout", page_in_layout, Rendering::Partial);
    in_layout.segments = vec![layout_seg(root_layout), SegmentDef::default()];
    let routes = Routes {
        pages: vec![
            page("/product", page_product, Rendering::Partial),
            // Counted by one test only: the app that pre-renders every page leaves it out.
            page("/counted", if leaky { page_counted } else { page_product }, Rendering::Partial),
            page("/counted-dynamic", page_counted, Rendering::Dynamic),
            // Found at run time too: nothing declares this page partial.
            page("/undeclared", page_product, Rendering::Static),
            page("/slow", page_slow, Rendering::Partial),
            page("/blocking", page_blocking, Rendering::Dynamic),
            page("/plain", page_plain, Rendering::Static),
            errors,
            page("/sets-cookie", page_sets_cookie, Rendering::Partial),
            page(
                if leaky { "/leaky" } else { "/not-leaky" },
                if leaky { page_leaky } else { page_plain },
                Rendering::Partial,
            ),
            tagged,
            page("/nested", page_nested, Rendering::Partial),
            page("/fragment-in-hole", page_fragment_in_hole, Rendering::Partial),
            // Counted by one test only, like `/counted`: a pre-render of every page in a
            // parallel test would bump its counters and fill its fragment first.
            page("/fragment-in-shell", if leaky { page_fragment_in_shell } else { page_plain }, Rendering::Partial),
            in_layout,
        ],
        build_id: "build-1",
        stylesheets: SHEETS,
        ..Default::default()
    };
    let mut config = Config { root: std::env::temp_dir(), ..Config::default() };
    configure(&mut config);
    App::new(routes)
        .config(config)
        .environment(env)
        .cache(shared_cache())
        .page_store(MemoryStore::new(100))
        .build()
        .unwrap()
}

fn app() -> App {
    app_with(Environment::Production, |_| {})
}

fn request(path: &str, headers: &[(&str, &str)]) -> Request {
    let mut req = http::Request::get(path);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    Request::from_http(req.body(Bytes::new()).unwrap())
}

struct Res {
    status: u16,
    headers: http::HeaderMap,
    body: String,
}

async fn get(app: &App, path: &str, headers: &[(&str, &str)]) -> Res {
    let res = app.handle(request(path, headers)).await;
    let (status, headers) = (res.status.as_u16(), res.headers.clone());
    Res { status, headers, body: res.into_text().await }
}

#[tokio::test]
async fn the_shell_is_rendered_once_and_holes_for_every_request() {
    let app = app();
    let ada = get(&app, "/counted?ref=mail", &[("cookie", "user=Ada")]).await;
    assert_eq!(ada.status, 200);
    assert_eq!(ada.headers["x-nr-cache"], "MISS");
    assert_eq!(ada.headers["x-nr-prerender"], "partial");
    let cc = ada.headers["cache-control"].to_str().unwrap();
    assert!(cc.contains("private") && cc.contains("no-store"), "a response with holes is never stored: {cc}");

    let bob = get(&app, "/counted", &[("cookie", "user=Bob")]).await;
    assert_eq!(bob.headers["x-nr-cache"], "HIT");
    assert!(bob.headers["cache-control"].to_str().unwrap().contains("no-store"));
    assert_eq!(COUNTED_SHELL_RUNS.load(Ordering::SeqCst), 1, "one shell render for both requests");
    assert_eq!(COUNTED_HOLE_RUNS.load(Ordering::SeqCst), 2, "the hole ran for each");

    // A dynamic page renders everything per request; its holes are suspense.
    let res = get(&app, "/counted-dynamic", &[("cookie", "user=Cy")]).await;
    get(&app, "/counted-dynamic", &[]).await;
    assert_eq!(COUNTED_SHELL_RUNS.load(Ordering::SeqCst), 3);
    assert!(res.headers.get("x-nr-cache").is_none());
    assert!(res.body.contains(r#"<template id="nr-t1">"#) && res.body.contains("Hello Cy"), "{}", res.body);

    for (res, name, reference) in [(&ada, "Hello Ada", "ref=mail"), (&bob, "Hello Bob", "ref=-")] {
        let body = &res.body;
        let fallback = body.find(r#"<nr-b id="nr-b1"><p>Loading…</p></nr-b>"#).expect("fallback in the shell");
        let footer = body.find("static footer").unwrap();
        let template = body.find(r#"<template id="nr-t1">"#).expect("the hole is streamed");
        assert!(fallback < footer && footer < template, "the whole shell comes first: {body}");
        assert!(body[template..].contains(name) && body[template..].contains(reference), "{body}");
        assert!(body.contains("$nr(1)</script>") && body.contains("function $nr("), "{body}");
        assert!(body.ends_with("</body></html>"), "{body}");
        assert!(!body[..template].contains("Hello"), "nothing of the visitor in the shell: {body}");
    }
    assert!(!bob.body.contains("Ada"), "{}", bob.body);
}

#[tokio::test]
async fn the_shell_is_sent_before_a_hole_resolves() {
    let app = app();
    get(&app, "/slow", &[]).await; // fill the cache
    let start = Instant::now();
    let res = app.handle(request("/slow", &[("cookie", "user=Ada")])).await;
    let Body::Stream(mut body) = res.body else { panic!("a partial page is streamed") };
    let first = String::from_utf8(body.next().await.unwrap().unwrap().to_vec()).unwrap();
    let first_at = start.elapsed();
    assert!(first.contains("<h1>Slow</h1>") && first.contains(r#"<nr-b id="nr-b1">"#), "{first}");
    assert!(!first.contains("Hello"), "{first}");
    assert!(first_at < Duration::from_millis(100), "the shell does not wait for the hole: {first_at:?}");
    let mut rest = String::new();
    while let Some(chunk) = body.next().await {
        rest.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
    }
    assert!(start.elapsed() >= Duration::from_millis(300));
    assert!(rest.contains("Hello Ada") && rest.ends_with("</body></html>"), "{rest}");
}

#[tokio::test]
async fn holes_are_found_without_a_declaration_and_in_development() {
    let app = app();
    let first = get(&app, "/undeclared", &[("cookie", "user=Ada")]).await;
    let second = get(&app, "/undeclared", &[("cookie", "user=Bob")]).await;
    assert_eq!(second.headers["x-nr-cache"], "HIT");
    assert_eq!(second.headers["x-nr-prerender"], "partial");
    assert!(first.body.contains("Hello Ada") && second.body.contains("Hello Bob") && !second.body.contains("Ada"));

    // Development renders per request as well, with the same output.
    let dev = app_with(Environment::Development, |_| {});
    let res = get(&dev, "/product", &[("cookie", "user=Dev")]).await;
    assert!(res.body.contains(r#"<template id="nr-t1">"#) && res.body.contains("Hello Dev"), "{}", res.body);
}

#[tokio::test]
async fn without_streaming_holes_are_written_in_place() {
    let app = app();
    get(&app, "/product", &[]).await;
    let bot = get(&app, "/product", &[("user-agent", "Googlebot/2.1"), ("cookie", "user=Crawler")]).await;
    assert_eq!(bot.headers["x-nr-cache"], "HIT");
    // In place of the fallback, with the rule of the class only the hole uses.
    let expected = "<h1>Product</h1><style data-nr-css=\"tw-ppr~Ag\">@layer utilities{.mt-2{margin-top:.5rem}}</style>\
<p class=\"mt-2\">Hello Crawler ref=-</p><footer>";
    assert!(bot.body.contains(expected), "{}", bot.body);
    assert!(!bot.body.contains("<template") && !bot.body.contains("Loading…") && !bot.body.contains("<nr-b"));
    assert!(bot.body.ends_with("</body></html>"));

    let off = app_with(Environment::Production, |c| c.rendering.streaming = false);
    let res = get(&off, "/product", &[("cookie", "user=Ada")]).await;
    assert!(res.body.contains("Hello Ada") && !res.body.contains("<template"), "{}", res.body);
    assert!(res.headers["cache-control"].to_str().unwrap().contains("no-store"));
}

#[tokio::test]
async fn errors_in_holes_render_a_boundary_and_the_stream_goes_on() {
    let app = app();
    let res = get(&app, "/errors", &[("cookie", "user=Ada")]).await;
    assert_eq!(res.status, 200, "the head was sent before any hole ran");
    let body = &res.body;
    assert!(body.contains(r#"<p class="boundary">boundary caught 500</p>"#), "error.rs of the page: {body}");
    assert!(body.contains(r#"<p class="local">local 409</p>"#), "the hole's own error view: {body}");
    assert_eq!(body.matches("boundary caught 500").count(), 2, "an error and a panic: {body}");
    assert!(body.contains(r#"location.replace("/login")"#), "a redirect still redirects: {body}");
    assert!(body.contains("fine Hello Ada"), "other holes are not affected: {body}");
    assert_eq!(body.matches("<template id=\"nr-t").count(), 5);
    assert!(body.ends_with("</body></html>"));
    assert!(!body.contains("boom"), "production hides error details: {body}");
}

#[tokio::test]
async fn a_hole_cannot_set_cookies_and_a_shell_cannot_read_them() {
    let app = app();
    let res = get(&app, "/sets-cookie", &[]).await;
    assert!(res.body.contains("tried"));
    let cookies: Vec<_> = res.headers.get_all("set-cookie").iter().filter_map(|v| v.to_str().ok()).collect();
    assert!(!cookies.iter().any(|c| c.starts_with("late=")), "the head was sent before the hole ran: {cookies:?}");

    // Request data outside a hole fails the static render instead of leaking into the cache.
    let res = get(&app, "/leaky", &[("cookie", "user=Ada")]).await;
    assert_eq!(res.status, 500);
    assert!(!res.body.contains("Ada"));
}

#[tokio::test]
async fn shells_follow_the_static_cache_rules() {
    let app = app();
    let first = get(&app, "/tagged", &[("cookie", "user=Ada")]).await;
    let n: usize = first.body.split("shell #").nth(1).unwrap().split('<').next().unwrap().parse().unwrap();
    assert_eq!(first.headers["x-nr-cache"], "MISS");
    let hit = get(&app, "/tagged", &[]).await;
    assert_eq!(hit.headers["x-nr-cache"], "HIT");
    assert!(hit.body.contains(&format!("shell #{n}")));

    assert!(app.revalidate_tag("ppr-products").await >= 1);
    let again = get(&app, "/tagged", &[("cookie", "user=Bob")]).await;
    assert_eq!(again.headers["x-nr-cache"], "MISS");
    assert!(again.body.contains(&format!("shell #{}", n + 1)) && again.body.contains("Hello Bob"), "{}", again.body);

    assert!(app.revalidate_path("/tagged").await);
    assert_eq!(get(&app, "/tagged", &[]).await.headers["x-nr-cache"], "MISS");

    // Pre-rendering at startup stores shells like pages.
    let fresh = build(Environment::Production, |_| {}, false);
    let report = fresh.prerender().await.unwrap();
    let page = report.pages.iter().find(|p| p.path == "/tagged").unwrap();
    assert_eq!((page.holes, page.revalidate), (1, Some(3600)));
    assert_eq!(report.pages.iter().find(|p| p.path == "/plain").unwrap().holes, 0);
    let res = get(&fresh, "/tagged", &[("cookie", "user=Eve")]).await;
    assert_eq!(res.headers["x-nr-cache"], "HIT");
    assert!(res.body.contains("Hello Eve"));
}

#[tokio::test]
async fn pages_without_holes_stay_plain_static_pages() {
    let app = app();
    get(&app, "/plain", &[]).await;
    let res = get(&app, "/plain", &[]).await;
    assert_eq!(res.headers["x-nr-cache"], "HIT");
    assert!(res.headers.get("x-nr-prerender").is_none());
    assert!(res.headers["cache-control"].to_str().unwrap().starts_with("public"));
    assert!(!res.body.contains("$nr") && res.body.ends_with("</body></html>"));
}

#[tokio::test]
async fn streamed_holes_continue_the_shell_ids_and_styles() {
    let app = app();
    get(&app, "/nested", &[]).await;
    let res = get(&app, "/nested", &[("cookie", "user=Ada")]).await;
    let body = &res.body;
    let shell_end = body.find("</main>").unwrap();
    assert!(body[..shell_end].contains(".p-4{padding:1rem}") && !body[..shell_end].contains(".px-2"), "{body}");
    // The hole brings the rules and stylesheets only it needs, before its content.
    let px = body.find(".px-2{padding-inline:.5rem}").expect("rule of the hole");
    assert!(px < body.find(r#"<template id="nr-t1">"#).unwrap(), "{body}");
    assert!(body.contains(r#"<style data-nr-css="widgetcss">"#), "{body}");
    // Suspense inside the hole gets the next id.
    assert!(body.contains(r#"<nr-b id="nr-b2"><p>inner…</p></nr-b>"#), "{body}");
    let inner = body.find(r#"<template id="nr-t2">"#).expect("nested boundary streamed");
    assert!(inner > shell_end && body[inner..].contains("inner Hello Ada"), "{body}");
    assert!(body[inner - 200..inner].contains(".mt-2{margin-top:.5rem}"), "{body}");
    assert_eq!(body.matches(".p-4{padding:1rem}").count(), 1, "nothing is sent twice: {body}");
}

#[tokio::test]
async fn client_navigation_gets_the_region_and_only_its_holes() {
    let app = app();
    let full = get(&app, "/in-layout", &[("cookie", "user=Ada")]).await;
    assert!(full.body.contains(r#"<span class="me">Hello Ada</span>"#) && full.body.contains(r#"<p class="in">"#));
    let key = full.body.split("<!--nr-l:").nth(1).unwrap().split("-->").next().unwrap().to_owned();

    let layout_holes = LAYOUT_HOLE_RUNS.load(Ordering::SeqCst);
    let nav = get(&app, "/in-layout", &[("x-nr-nav", "1"), ("x-nr-layouts", &key), ("cookie", "user=Bob")]).await;
    assert_eq!(nav.headers["x-nr-partial"], key.as_str());
    assert_eq!(nav.headers["x-nr-cache"], "HIT");
    assert!(nav.headers["cache-control"].to_str().unwrap().contains("no-store"));
    assert_eq!(
        LAYOUT_HOLE_RUNS.load(Ordering::SeqCst),
        layout_holes,
        "the layout stays on screen: its hole is not run"
    );
    let body = &nav.body;
    assert!(!body.contains("id=\"site\"") && !body.contains("class=\"me\""), "{body}");
    // What the client runtime does with a fetched page: move each template into its boundary.
    let template = body.find(r#"<template id="nr-t2">"#).expect("the page's hole");
    assert!(body.find(r#"<nr-b id="nr-b2">"#).unwrap() < template, "{body}");
    assert!(body[template..].contains(r#"<p class="in">Hello Bob</p>"#), "{body}");
    assert!(!body.contains("nr-t1"), "{body}");
    assert!(body.ends_with("</body></html>"));
}

#[tokio::test]
async fn cached_fragments_are_rendered_once_and_revalidated_by_tag() {
    let app = app();
    let runs = FRAGMENT_RUNS.load(Ordering::SeqCst);
    let ada = get(&app, "/fragment-in-hole", &[("cookie", "user=Ada")]).await;
    let bob = get(&app, "/fragment-in-hole", &[("cookie", "user=Bob")]).await;
    assert_eq!(FRAGMENT_RUNS.load(Ordering::SeqCst), runs + 1, "rendered for the first request only");
    let list = format!("list #{}", runs + 1);
    assert!(ada.body.contains("Hello Ada") && ada.body.contains(&list), "{}", ada.body);
    assert!(bob.body.contains("Hello Bob") && bob.body.contains(&list), "{}", bob.body);
    // A cached fragment still tells the document what it needs.
    assert!(bob.body.contains(r#"<style data-nr-css="widgetcss">"#), "its stylesheet: {}", bob.body);
    assert!(bob.body.contains(".mt-2{margin-top:.5rem}"), "its utility classes: {}", bob.body);
    assert!(bob.body.contains("/_next-rust/runtime.js"), "its link needs the runtime: {}", bob.body);

    assert!(app.revalidate_tag("ppr-list").await >= 1);
    let res = get(&app, "/fragment-in-hole", &[]).await;
    assert!(res.body.contains(&format!("list #{}", runs + 2)), "{}", res.body);
    assert_eq!(res.headers["x-nr-cache"], "HIT", "the shell was not touched");

    invalidate_fragment("ppr:list").await;
    assert!(get(&app, "/fragment-in-hole", &[]).await.body.contains(&format!("list #{}", runs + 3)));
}

#[tokio::test]
async fn a_shell_is_revalidated_with_the_fragments_in_it() {
    let app = app();
    let first = get(&app, "/fragment-in-shell", &[]).await;
    assert!(first.body.contains("page #1") && first.body.contains("prices #1"), "{}", first.body);
    assert_eq!(get(&app, "/fragment-in-shell", &[]).await.headers["x-nr-cache"], "HIT");

    // The page has no tags of its own: the fragment's tag covers the shell.
    assert!(app.revalidate_tag("ppr-prices").await >= 2, "the fragment and the shell around it");
    let res = get(&app, "/fragment-in-shell", &[("cookie", "user=Ada")]).await;
    assert_eq!(res.headers["x-nr-cache"], "MISS");
    assert!(res.body.contains("page #2") && res.body.contains("prices #2") && res.body.contains("Hello Ada"));

    // The other way round, a fresh fragment is reused by a new shell.
    assert!(app.revalidate_path("/fragment-in-shell").await);
    let res = get(&app, "/fragment-in-shell", &[]).await;
    assert!(res.body.contains("page #3") && res.body.contains("prices #2"), "{}", res.body);
}

/// Two instances over one persistent page store (or one instance before and
/// after a restart): the holes of a stored shell exist only in the process
/// that rendered it.
#[tokio::test]
async fn instances_sharing_a_page_store_do_not_overwrite_each_other() {
    let dir = std::env::temp_dir().join(format!("nr-ppr-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let instance = || {
        let routes = Routes {
            pages: vec![page("/shared", page_shared, Rendering::Partial)],
            build_id: "build-1",
            stylesheets: SHEETS,
            ..Default::default()
        };
        App::new(routes)
            .config(Config { root: std::env::temp_dir(), ..Config::default() })
            .environment(Environment::Production)
            .cache(shared_cache())
            .page_store(next_rust_cache::FileStore::new(&dir))
            .build()
            .unwrap()
    };
    let (a, b) = (instance(), instance());
    assert_eq!(get(&a, "/shared", &[("cookie", "user=Ada")]).await.headers["x-nr-cache"], "MISS");
    // `b` finds `a`'s shell in the store, renders its own once and leaves the entry alone…
    let res = get(&b, "/shared", &[("cookie", "user=Bob")]).await;
    assert!(res.body.contains("Hello Bob") && res.body.ends_with("</body></html>"), "{}", res.body);
    // …so both keep hitting from then on.
    for (app, name) in [(&a, "Hello Cy"), (&b, "Hello Di"), (&a, "Hello Ed"), (&b, "Hello Flo")] {
        let user = format!("user={}", &name[6..]);
        let res = get(app, "/shared", &[("cookie", &user)]).await;
        assert_eq!(res.headers["x-nr-cache"], "HIT");
        assert!(res.body.contains(name), "{}", res.body);
    }
    assert_eq!(SHARED_SHELL_RUNS.load(Ordering::SeqCst), 2, "one render per instance");
    assert_eq!(SHARED_HOLE_RUNS.load(Ordering::SeqCst), 6);

    // Revalidation through either instance reaches both.
    assert!(b.revalidate_path("/shared").await);
    assert_eq!(get(&a, "/shared", &[]).await.headers["x-nr-cache"], "MISS");
    assert_eq!(SHARED_SHELL_RUNS.load(Ordering::SeqCst), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Time to first body byte and to the last one, over a real connection.
async fn timings(addr: std::net::SocketAddr, path: &str) -> (Duration, Duration, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let start = Instant::now();
    let req = format!("GET {path} HTTP/1.1\r\nhost: localhost\r\ncookie: user=Ada\r\nconnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();
    let (mut buf, mut chunk, mut first) = (Vec::new(), [0u8; 8192], None);
    loop {
        let n = stream.read(&mut chunk).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if first.is_none() && String::from_utf8_lossy(&buf).contains("<h1>Slow</h1>") {
            first = Some(start.elapsed());
        }
    }
    (first.expect("page content"), start.elapsed(), String::from_utf8_lossy(&buf).into_owned())
}

/// The measurement behind the documentation's claim: a partial page with a
/// 300 ms hole starts as fast as a static page. Run with `--nocapture` to see
/// the numbers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn time_to_first_byte_of_a_partial_page_is_that_of_a_static_page() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = app();
    tokio::spawn(async move { app.serve_listener(listener).await });
    for path in ["/plain", "/slow"] {
        timings(addr, path).await; // fill the page cache
    }
    let median = |mut v: Vec<Duration>| {
        v.sort();
        v[v.len() / 2]
    };
    let mut report = Vec::new();
    for path in ["/plain", "/slow", "/blocking"] {
        let (mut firsts, mut totals) = (Vec::new(), Vec::new());
        for _ in 0..9 {
            let (first, total, body) = timings(addr, path).await;
            assert!(body.contains("Hello"), "{body}");
            firsts.push(first);
            totals.push(total);
        }
        report.push((path, median(firsts), median(totals)));
    }
    for (path, first, total) in &report {
        println!("{path:<10} first byte {first:>10.3?}   complete {total:>10.3?}");
    }
    let [(_, static_first, _), (_, partial_first, partial_total), (_, blocking_first, _)] = report[..] else {
        unreachable!()
    };
    // The claim is "as fast as a static page, not as slow as the 300 ms hole",
    // so the margins are wide enough for a shared CI runner to be slow without
    // making the test lie: 150 ms is still half of the hole below.
    assert!(partial_first < Duration::from_millis(150), "partial TTFB {partial_first:?} (static {static_first:?})");
    assert!(partial_first < static_first + Duration::from_millis(100), "{partial_first:?} vs {static_first:?}");
    assert!(partial_total >= Duration::from_millis(300), "the hole still takes its time: {partial_total:?}");
    assert!(blocking_first >= Duration::from_millis(300), "without a hole the page waits: {blocking_first:?}");
}
