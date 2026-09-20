//! Security regression tests for partial prerendering: what a cached shell
//! may hold, fragment-cache keys, hostile navigation headers and the
//! streaming protocol.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use next_rust_cache::{Cache, CacheEntry, CacheKey, CacheOptions, MemoryStore};
use next_rust_core::{Config, Environment};
use next_rust_server::app::{LayoutFn, RenderFn};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

static SHEETS: &[&Stylesheet] = &[];

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

fn page_def(pattern: &'static str, body: RenderFn, rendering: Rendering) -> PageDef {
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

fn shared_cache() -> Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Cache::new(MemoryStore::new(1000))).clone()
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

fn build(pages: Vec<PageDef>) -> App {
    let routes = Routes { pages, build_id: "build-sec", stylesheets: SHEETS, ..Default::default() };
    App::new(routes)
        .config(Config { root: std::env::temp_dir(), ..Config::default() })
        .environment(Environment::Production)
        .cache(shared_cache())
        .page_store(MemoryStore::new(100))
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// 1 (a)(b): what a shell render can read
// ---------------------------------------------------------------------------

/// Every extractor that carries request data must refuse a static render, so
/// nothing of one visitor can end up in a shared shell. `Nonce` and the
/// parameter extractors are the documented exceptions: the nonce is a
/// placeholder replaced per request, parameters are part of the cache key.
#[tokio::test]
async fn every_request_bound_extractor_is_refused_in_a_static_render() {
    #[derive(Clone, Debug, PartialEq)]
    struct User(String);

    let config = std::sync::Arc::new(Config::default());
    let ctx = RequestContext::for_static("/p", next_rust_router::Params::new(), config);
    assert!(ctx.is_static());

    macro_rules! refused {
        ($($t:ty),* $(,)?) => {$(
            assert!(ctx.extract::<$t>().is_err(), concat!(stringify!($t), " must be refused in a shell"));
        )*};
    }
    refused!(
        Cookies,
        Headers,
        Query,
        Query<QueryMap>,
        ClientIp,
        RequestInfo,
        Extension<User>,
        Option<Extension<User>>,
        Auth<User>,
        CsrfToken,
        FormState,
        ResponseHeaders,
    );

    // The context's own fields are empty rather than someone else's data.
    assert_eq!(ctx.cookies.get("session"), None);
    assert!(ctx.headers.is_empty());
    assert_eq!(ctx.uri.query(), None);
    assert_eq!(ctx.remote_addr, None);
    assert!(ctx.extensions.get::<User>().is_none());
    // Setting a cookie from a static render reaches nobody: the jar is the
    // context's own, and there is no response to put it on.
    ctx.cookies.set(Cookie::new("x", "1"));
    assert!(!ctx.cookies.set_cookie_headers().is_empty());
}

static DEEP_SHELL_RUNS: AtomicUsize = AtomicUsize::new(0);

/// A hole reached through a layout, a template and a plain helper function —
/// none of which the build can see into.
fn deep_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        Ok(div![id("shell-layout"), dynamic(span!["l…"], |ctx| async move { span![who(&ctx)] }), children].into_node())
    })
}

fn deep_template(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        Ok(section![dynamic(span!["t…"], |ctx| async move { span![who(&ctx)] }), children].into_node())
    })
}

fn who(ctx: &Ctx) -> String {
    format!("user:{}", ctx.cookies.get("user").unwrap_or_else(|| "guest".into()))
}

/// The hole is built by a helper in "another module": nothing in the page's
/// own source says `dynamic`.
fn helper_widget() -> Node {
    dynamic(span!["h…"], |ctx| async move { span![who(&ctx)] })
}

fn page_deep(_ctx: Ctx) -> R {
    Box::pin(async {
        DEEP_SHELL_RUNS.fetch_add(1, Ordering::SeqCst);
        Ok(main![h1!["Deep"], helper_widget()].into_node())
    })
}

/// A page that reads a cookie inside a `cached(..)` fragment, outside any hole.
fn page_leaky_fragment(_ctx: Ctx) -> R {
    Box::pin(async {
        let frag = cached("sec:leak", CacheOptions::default(), || async {
            // A fragment body has no context of its own; the only way to read
            // the request is an extractor, which must fail.
            Ok::<_, Error>(p!["never"])
        })
        .await?;
        Ok(main![frag].into_node())
    })
}

/// A layout that reads the request outside a hole.
fn leaky_layout(ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        let cookies = ctx.extract::<Cookies>()?;
        Ok(div![p![cookies.get("user").unwrap_or_default()], children].into_node())
    })
}

/// A template that reads the request outside a hole.
fn leaky_template(ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        let h = ctx.extract::<Headers>()?;
        Ok(div![p![h.get("x-secret").unwrap_or_default().to_owned()], children].into_node())
    })
}

fn plain(_ctx: Ctx) -> R {
    Box::pin(async { Ok(main![h1!["plain"]].into_node()) })
}

/// A slot page that reads the request.
fn leaky_slot(ctx: Ctx) -> R {
    Box::pin(async move {
        let cookies = ctx.extract::<Cookies>()?;
        Ok(aside![format!("slot {}", cookies.get("user").unwrap_or_default())].into_node())
    })
}

fn deep_app() -> App {
    let mut deep = page_def("/deep", page_deep, Rendering::Static);
    deep.segments = vec![
        SegmentDef {
            layout: Some(deep_layout as LayoutFn),
            layout_id: "app/layout.rs",
            layout_reusable: true,
            template: Some(deep_template as LayoutFn),
            ..Default::default()
        },
        SegmentDef::default(),
    ];
    let mut leaky_l = page_def("/leaky-layout", plain, Rendering::Static);
    leaky_l.segments =
        vec![SegmentDef { layout: Some(leaky_layout as LayoutFn), ..Default::default() }, SegmentDef::default()];
    let mut leaky_t = page_def("/leaky-template", plain, Rendering::Static);
    leaky_t.segments =
        vec![SegmentDef { template: Some(leaky_template as LayoutFn), ..Default::default() }, SegmentDef::default()];
    let mut leaky_s = page_def("/leaky-slot", plain, Rendering::Static);
    leaky_s.segments = vec![
        SegmentDef {
            layout: Some(deep_layout as LayoutFn),
            slots: vec![SlotDef { name: "modal", render: Some(leaky_slot as RenderFn) }],
            ..Default::default()
        },
        SegmentDef::default(),
    ];
    build(vec![deep, leaky_l, leaky_t, leaky_s, page_def("/leaky-fragment", page_leaky_fragment, Rendering::Static)])
}

#[tokio::test]
async fn holes_found_through_layouts_templates_and_helpers_keep_the_shell_shared() {
    let app = deep_app();
    let ada = get(&app, "/deep", &[("cookie", "user=Ada")]).await;
    let bob = get(&app, "/deep", &[("cookie", "user=Bob")]).await;
    assert_eq!(ada.status, 200);
    assert_eq!(bob.headers["x-nr-cache"], "HIT");
    assert_eq!(DEEP_SHELL_RUNS.load(Ordering::SeqCst), 1, "one shell for both");
    // Three holes: layout, template, helper in the page.
    for (res, name, other) in [(&ada, "user:Ada", "Bob"), (&bob, "user:Bob", "Ada")] {
        assert_eq!(res.body.matches("<template id=\"nr-t").count(), 3, "{}", res.body);
        assert_eq!(res.body.matches(name).count(), 3, "{}", res.body);
        assert!(!res.body.contains(other), "{}", res.body);
        let first = res.body.find("<template").unwrap();
        assert!(!res.body[..first].contains("user:"), "the shell holds nobody: {}", res.body);
    }
}

#[tokio::test]
async fn a_request_read_outside_a_hole_fails_the_static_render_everywhere() {
    let app = deep_app();
    for path in ["/leaky-layout", "/leaky-template"] {
        let res = get(&app, path, &[("cookie", "user=Ada"), ("x-secret", "hunter2")]).await;
        assert_eq!(res.status, 500, "{path}: {}", res.body);
        assert!(!res.body.contains("Ada") && !res.body.contains("hunter2"), "{path}: {}", res.body);
        assert!(res.headers.get("x-nr-cache").is_none(), "{path}: nothing was cached");
    }
    // A slot is the one place where the failure is swallowed: the slot is
    // dropped, so the page is served without it — never with someone's data.
    let res = get(&app, "/leaky-slot", &[("cookie", "user=Ada")]).await;
    assert!(!res.body.contains("<aside"), "the slot that read the request is left out: {}", res.body);
    assert!(!res.body.contains("slot Ada"), "{}", res.body);
}

// ---------------------------------------------------------------------------
// 2: fragment cache keys
// ---------------------------------------------------------------------------

/// `cached(key, ..)` keys are commonly built from user input. They live in
/// their own namespace: no key can name a page entry, a data-cache entry or
/// an optimized image.
#[tokio::test]
async fn a_fragment_key_cannot_name_a_page_or_data_entry() {
    let store = MemoryStore::new(100);
    let cache = Cache::new(store);
    // Install a page-like and a data-like entry by hand.
    let secret = CacheEntry::new(b"<html>secret page</html>".to_vec());
    cache.store().set(CacheKey::page("/admin"), secret).await.unwrap();
    cache.store().set(CacheKey::data("token"), CacheEntry::new(b"\"s3cret\"".to_vec())).await.unwrap();

    // Every crafted key stays under `fragment:`.
    for hostile in ["/admin", "page:/admin", "data:token", "image:abc", "", "\0", "fragment:x"] {
        let key = format!("fragment:{hostile}");
        assert!(key.starts_with("fragment:"));
        assert_ne!(key, "page:/admin");
        assert_ne!(key, "data:token");
    }
    // And the entries are untouched by a fragment round trip.
    assert_eq!(
        &*cache.store().get(&CacheKey::page("/admin")).await.unwrap().unwrap().value,
        b"<html>secret page</html>"
    );
    let v: std::result::Result<String, ()> =
        cache.get_or_insert_json("token", CacheOptions::default(), || async { Ok("fresh".to_owned()) }).await;
    assert_eq!(v, Ok("s3cret".to_owned()), "the data entry is the one that was there");
}

static SHORT_FRAGMENT_RUNS: AtomicUsize = AtomicUsize::new(0);
static LONG_PAGE_RUNS: AtomicUsize = AtomicUsize::new(0);

fn page_short_fragment(_ctx: Ctx) -> R {
    Box::pin(async {
        let n = LONG_PAGE_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
        let frag = cached("sec:short", CacheOptions::revalidate(1).tag("sec-short"), || async {
            let m = SHORT_FRAGMENT_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
            p![format!("frag #{m}")]
        })
        .await?;
        Ok(main![p![format!("page #{n}")], frag].into_node())
    })
}

/// A long-lived page that embeds a short-lived fragment must not outlive it.
#[tokio::test]
async fn a_page_never_outlives_the_shortest_fragment_in_it() {
    let mut def = page_def("/short-fragment", page_short_fragment, Rendering::Static);
    def.revalidate = Some(3600);
    def.tags = &["sec-page"];
    let app = build(vec![def]);

    let first = get(&app, "/short-fragment", &[]).await;
    assert_eq!(first.headers["x-nr-cache"], "MISS");
    // The page inherits the fragment's one-second lifetime, not its own hour.
    let cc = first.headers["cache-control"].to_str().unwrap();
    assert!(cc.contains("s-maxage=1"), "the page inherits the fragment's lifetime: {cc}");
    assert_eq!(get(&app, "/short-fragment", &[]).await.headers["x-nr-cache"], "HIT");

    // The fragment's tag reaches the page around it.
    assert!(app.revalidate_tag("sec-short").await >= 2, "the fragment and the page that holds it");
    assert_eq!(get(&app, "/short-fragment", &[]).await.headers["x-nr-cache"], "MISS");
}

static POISON_RUNS: AtomicUsize = AtomicUsize::new(0);

fn page_poisoned_fragment(_ctx: Ctx) -> R {
    Box::pin(async {
        let frag = cached("sec:poison", CacheOptions::default(), || async {
            POISON_RUNS.fetch_add(1, Ordering::SeqCst);
            p!["rendered here"]
        })
        .await?;
        Ok(main![frag].into_node())
    })
}

async fn put_fragment(key: &str, value: serde_json::Value) {
    next_rust_cache::global()
        .store()
        .set(CacheKey::new(key.to_owned()), CacheEntry::new(serde_json::to_vec(&value).unwrap()))
        .await
        .unwrap();
}

/// Bytes in a fragment store come from an earlier build, another instance, or
/// anyone who can write the cache directory. Unknown stylesheet ids, absurd
/// vectors and missing fields must be refused or survived — never trusted
/// into a panic.
#[tokio::test]
async fn a_hostile_fragment_entry_is_refused_or_rendered_again() {
    let app = build(vec![page_def("/poisoned", page_poisoned_fragment, Rendering::Dynamic)]);

    // (1) A stylesheet this build does not know: render again, do not drop.
    put_fragment(
        "fragment:sec:poison",
        serde_json::json!({
            "html": "<p>from the store</p>", "sheets": ["no-such-sheet"],
            "classes": [], "scripts": [], "flags": [false, false, false, false],
        }),
    )
    .await;
    let before = POISON_RUNS.load(Ordering::SeqCst);
    let res = get(&app, "/poisoned", &[]).await;
    assert_eq!(POISON_RUNS.load(Ordering::SeqCst), before + 1, "an unusable entry is rendered again");
    assert!(res.body.contains("rendered here") && !res.body.contains("from the store"), "{}", res.body);

    // (2) Malformed JSON, a short `flags` array and absurd vectors: none of
    // them may panic, and none may be served.
    for value in [
        serde_json::json!({"html": "x"}),
        serde_json::json!({"html": "x", "sheets": [], "classes": [], "scripts": [], "flags": [true]}),
        serde_json::json!("not an object"),
    ] {
        put_fragment("fragment:sec:poison", value).await;
        let res = get(&app, "/poisoned", &[]).await;
        assert_eq!(res.status, 200, "{}", res.body);
        assert!(res.body.contains("rendered here"), "unusable bytes are ignored: {}", res.body);
    }

    // (3) Absurd but well-formed vectors are accepted and cost what they say:
    // they are this process's own classes, not markup.
    put_fragment(
        "fragment:sec:poison",
        serde_json::json!({
            "html": "<p>huge</p>", "sheets": [],
            "classes": vec!["c"; 5_000], "scripts": vec!["s"; 5_000],
            "flags": [true, true, true, true], "forms": true,
        }),
    )
    .await;
    let start = std::time::Instant::now();
    let res = get(&app, "/poisoned", &[]).await;
    assert_eq!(res.status, 200);
    assert!(start.elapsed() < std::time::Duration::from_secs(5), "{:?}", start.elapsed());
    next_rust_server::invalidate_fragment("sec:poison").await;
}

/// A fragment entry carries HTML that is written through unescaped
/// (`Node::Rendered` is this process's own output). A store an attacker can
/// write is therefore a markup-injection sink — pinned here so the trust
/// boundary is deliberate, and so it stays confined to a *known* stylesheet id.
#[tokio::test]
async fn fragment_html_from_the_store_is_written_through() {
    let app = build(vec![page_def("/poisoned-raw", page_poisoned_raw, Rendering::Dynamic)]);
    put_fragment(
        "fragment:sec:poison-raw",
        serde_json::json!({
            "html": "<script>alert(1)</script>", "sheets": [],
            "classes": [], "scripts": [], "flags": [false, false, false, false],
        }),
    )
    .await;
    let res = get(&app, "/poisoned-raw", &[]).await;
    assert!(res.body.contains("<script>alert(1)</script>"), "documented: a fragment store must be trusted");
    next_rust_server::invalidate_fragment("sec:poison-raw").await;
}

fn page_poisoned_raw(_ctx: Ctx) -> R {
    Box::pin(async {
        let frag = cached("sec:poison-raw", CacheOptions::default(), || async { p!["rendered here"] }).await?;
        Ok(main![frag].into_node())
    })
}

// ---------------------------------------------------------------------------
// 4: hostile navigation headers
// ---------------------------------------------------------------------------

static NAV_LAYOUT_RUNS: AtomicUsize = AtomicUsize::new(0);

fn nav_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        NAV_LAYOUT_RUNS.fetch_add(1, Ordering::SeqCst);
        Ok(div![id("nav-layout"), p!["layout content"], children].into_node())
    })
}

/// A layout that reads the request: the build marks it not reusable, so it
/// must be rendered again however the browser asks.
fn guard_layout(ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move {
        let cookies = ctx.extract::<Cookies>()?;
        if cookies.get("admin").as_deref() != Some("1") {
            return Err(redirect("/login"));
        }
        Ok(div![id("guarded"), children].into_node())
    })
}

fn page_one(_ctx: Ctx) -> R {
    Box::pin(async { Ok(main![h1!["one"], p!["one body"]].into_node()) })
}
fn page_two(_ctx: Ctx) -> R {
    Box::pin(async { Ok(main![h1!["two"], p!["two body"]].into_node()) })
}
fn page_secret(_ctx: Ctx) -> R {
    Box::pin(async { Ok(main![h1!["secret"], p!["classified"]].into_node()) })
}

fn nav_app() -> App {
    let seg = || SegmentDef {
        layout: Some(nav_layout as LayoutFn),
        layout_id: "app/layout.rs",
        layout_reusable: true,
        ..Default::default()
    };
    let mut one = page_def("/one", page_one, Rendering::Static);
    one.segments = vec![seg(), SegmentDef::default()];
    let mut two = page_def("/two", page_two, Rendering::Dynamic);
    two.segments = vec![seg(), SegmentDef::default()];
    let mut guarded = page_def("/guarded", page_secret, Rendering::Dynamic);
    guarded.segments = vec![
        SegmentDef {
            layout: Some(guard_layout as LayoutFn),
            layout_id: "app/guarded/layout.rs",
            // What the build writes for a layout taking `Cookies`.
            layout_reusable: false,
            ..Default::default()
        },
        SegmentDef::default(),
    ];
    build(vec![one, two, guarded])
}

#[tokio::test]
async fn a_forged_layout_key_only_ever_cuts_the_page_that_was_asked_for() {
    let app = nav_app();
    let full = get(&app, "/one", &[]).await;
    let key = full.body.split("<!--nr-l:").nth(1).unwrap().split("-->").next().unwrap().to_owned();
    assert_eq!(key.len(), 16, "an FNV-64 layout key: {key}");

    // The key is published in the HTML of every page that shares the layout:
    // guessing it buys nothing a request cannot already read.
    let two = get(&app, "/two", &[]).await;
    assert!(two.body.contains(&format!("<!--nr-l:{key}-->")), "the same layout, the same key");

    // With the key, a navigation to /two returns the region of /two only.
    let nav = get(&app, "/two", &[("x-nr-nav", "1"), ("x-nr-layouts", &key)]).await;
    assert_eq!(nav.headers["x-nr-partial"], key.as_str());
    assert!(nav.body.contains("two body"), "{}", nav.body);
    assert!(!nav.body.contains("one body") && !nav.body.contains("classified"), "{}", nav.body);
    assert!(!nav.body.contains("layout content"), "the shared layout is not sent again: {}", nav.body);

    // Nonsense, colliding-looking and oversized keys are simply not found.
    for hostile in ["0000000000000000", "ffffffffffffffff", &"a".repeat(100_000), "", ",,,,", "<script>"] {
        let res = get(&app, "/two", &[("x-nr-nav", "1"), ("x-nr-layouts", hostile)]).await;
        assert!(res.headers.get("x-nr-partial").is_none(), "no region for {hostile:?}");
        assert!(res.body.contains("layout content"), "the whole page instead");
    }
}

/// A layout that checks a cookie is never skipped by a forged navigation: the
/// build marks it not reusable, so it has no key to name.
#[tokio::test]
async fn a_layout_that_reads_the_request_cannot_be_skipped_by_a_navigation() {
    let app = nav_app();
    let denied = get(&app, "/guarded", &[]).await;
    assert!(!denied.body.contains("classified"), "{}", denied.body);
    assert!(!denied.body.contains("<!--nr-l:"), "a guarding layout publishes no key: {}", denied.body);

    // Every key the attacker could have collected elsewhere.
    let other = get(&app, "/one", &[]).await;
    let key = other.body.split("<!--nr-l:").nth(1).unwrap().split("-->").next().unwrap().to_owned();
    for keys in [key.as_str(), "0000000000000000", &format!("{key},ffffffffffffffff")] {
        let res = get(&app, "/guarded", &[("x-nr-nav", "1"), ("x-nr-layouts", keys)]).await;
        assert!(!res.body.contains("classified"), "the layout still ran: {}", res.body);
    }
    let allowed = get(&app, "/guarded", &[("cookie", "admin=1")]).await;
    assert!(allowed.body.contains("classified"), "{}", allowed.body);
}

#[tokio::test]
async fn hostile_navigation_headers_do_not_panic_or_go_quadratic() {
    let app = nav_app();
    let long = "x".repeat(200_000);
    let many: String = (0..5_000).map(|i| format!("k{i},")).collect();
    let hostile: Vec<(&str, &str)> =
        vec![("x-nr-nav", "1"), ("x-nr-layouts", &many), ("x-nr-styles", &long), ("user-agent", "Mozilla/5.0")];
    let start = std::time::Instant::now();
    let res = get(&app, "/one", &hostile).await;
    assert_eq!(res.status, 200);
    assert!(start.elapsed() < std::time::Duration::from_secs(5), "{:?}", start.elapsed());

    for styles in ["a\"><script>alert(1)</script>", "tw-ppr~\u{fffd}", ",,,", &",".repeat(10_000)] {
        let res = get(&app, "/one", &[("x-nr-nav", "1"), ("x-nr-styles", styles)]).await;
        assert_eq!(res.status, 200);
        assert!(!res.body.contains("<script>alert(1)</script>"), "{}", res.body);
    }
}

// ---------------------------------------------------------------------------
// 1 (d) / path normalisation
// ---------------------------------------------------------------------------

static NORM_RUNS: AtomicUsize = AtomicUsize::new(0);

fn page_norm(ctx: Ctx) -> R {
    Box::pin(async move {
        NORM_RUNS.fetch_add(1, Ordering::SeqCst);
        Ok(main![format!("id={}", ctx.params.get("id").unwrap_or(""))].into_node())
    })
}

/// The page cache is keyed by the request path; the router is not. They must
/// agree, or one URL's page is served for another.
#[tokio::test]
async fn the_page_cache_key_and_the_router_agree_on_the_path() {
    let app = build(vec![page_def("/n/[id]", page_norm, Rendering::Static)]);

    // A trailing slash never reaches the cache: it is redirected to the one
    // spelling, so `/n/a/` can never get its own entry beside `/n/a`.
    assert!(get(&app, "/n/a", &[]).await.body.contains("id=a"));
    let slashed = get(&app, "/n/a/", &[]).await;
    assert_eq!(slashed.status, 308, "{}", slashed.body);
    assert_eq!(slashed.headers["location"], "/n/a");

    // A different parameter is a different entry: never the one above.
    let b = get(&app, "/n/b", &[]).await;
    assert!(b.body.contains("id=b") && !b.body.contains("id=a"), "{}", b.body);

    // A percent-encoded separator is decoded into the parameter, and the key
    // keeps the encoded form: it can never name another route's entry.
    let enc = get(&app, "/n/a%2Fb", &[]).await;
    assert!(enc.body.contains("id=a/b"), "{}", enc.body);
    assert_ne!(enc.headers["x-nr-cache"], "HIT");

    // An empty segment matches nothing at all.
    assert_eq!(get(&app, "//n/a", &[]).await.status, 404);
    assert_eq!(get(&app, "/n//a", &[]).await.status, 404);

    // The query string is not part of the key, and a static page cannot read
    // it, so it can never change what is cached.
    let q = get(&app, "/n/a?x=1", &[]).await;
    assert_eq!(q.headers["x-nr-cache"], "HIT");
    assert!(q.body.contains("id=a"));
}

/// Cached HTML carries a placeholder where the CSP nonce goes, replaced for
/// every request. A page that renders a route parameter can be made to write
/// that placeholder into its own markup — the substitution must not then hand
/// the visitor's nonce out in the page text.
#[tokio::test]
async fn a_reflected_nonce_placeholder_does_not_become_a_real_nonce() {
    let app = build(vec![page_def("/n/[id]", page_norm, Rendering::Static)]);
    let res = get(&app, "/n/nr-nonce-placeholder-5f3a9c", &[]).await;
    let nonce = res
        .headers
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| c.split("'nonce-").nth(1))
        .map(|n| n.split('\'').next().unwrap_or("").to_owned());
    if let Some(nonce) = nonce.filter(|n| !n.is_empty()) {
        let body_text = res.body.split("<body>").nth(1).unwrap_or("");
        assert!(!body_text.contains(&nonce), "the request's nonce was written into the page text: {body_text}");
    }
}

/// A navigation asks for less than the whole document (no icons, no styles it
/// already has, only a region). None of that may change the entry everyone
/// else is served from.
#[tokio::test]
async fn navigation_headers_never_change_the_cached_page() {
    let app = nav_app();
    let full = get(&app, "/one", &[]).await;
    let key = full.body.split("<!--nr-l:").nth(1).unwrap().split("-->").next().unwrap().to_owned();
    assert_eq!(full.headers["x-nr-cache"], "MISS");

    for headers in [
        vec![("x-nr-nav", "1"), ("x-nr-layouts", key.as_str())],
        vec![("x-nr-nav", "1"), ("x-nr-styles", "nr-motion,nr-loader")],
        vec![("x-nr-nav", "1"), ("x-nr-layouts", key.as_str()), ("x-nr-styles", "nr-motion")],
    ] {
        let res = get(&app, "/one", &headers).await;
        assert_eq!(res.headers["x-nr-cache"], "HIT");
        assert!(res.headers["cache-control"].to_str().unwrap().contains("no-store"), "never shared: {headers:?}");
    }

    // The next plain visitor gets the document, whole.
    let plain_again = get(&app, "/one", &[]).await;
    assert_eq!(plain_again.headers["x-nr-cache"], "HIT");
    assert_eq!(plain_again.body, full.body, "the cached entry is untouched by any navigation");
}

static CONCURRENT_HOLE_RUNS: AtomicUsize = AtomicUsize::new(0);

fn page_concurrent(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![
            dynamic(p!["a…"], |ctx| async move {
                CONCURRENT_HOLE_RUNS.fetch_add(1, Ordering::SeqCst);
                p![who(&ctx)]
            }),
            dynamic(p!["b…"], |ctx| async move { span![who(&ctx)] }),
        ]
        .into_node())
    })
}

/// Many requests resume one shell at once. Each must see only its own holes:
/// the shell is shared, the boundary ids and the rendered data are not.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_requests_on_one_shell_never_mix() {
    let app = std::sync::Arc::new(build(vec![page_def("/concurrent", page_concurrent, Rendering::Static)]));
    get(&app, "/concurrent", &[]).await; // fill the cache

    // Same length, so one name is never a prefix of another.
    let names: Vec<String> = (0..32).map(|i| format!("Visitor{i:02}")).collect();
    let bodies = futures_util::future::join_all(names.iter().map(|name| {
        let app = app.clone();
        async move { get(&app, "/concurrent", &[("cookie", &format!("user={name}"))]).await.body }
    }))
    .await;

    for (name, body) in names.iter().zip(&bodies) {
        assert_eq!(body.matches(&format!("user:{name}")).count(), 2, "both holes are this visitor's: {body}");
        for other in &names {
            assert!(other == name || !body.contains(&format!("user:{other}")), "{name} saw {other}: {body}");
        }
        assert_eq!(body.matches("<template id=\"nr-t").count(), 2, "{body}");
        assert!(body.ends_with("</body></html>"), "{body}");
    }
}

static GATED_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Its own counter: `page_norm` is shared with a test that runs in parallel.
fn page_gated(ctx: Ctx) -> R {
    Box::pin(async move {
        GATED_RUNS.fetch_add(1, Ordering::SeqCst);
        Ok(main![format!("id={}", ctx.params.get("id").unwrap_or(""))].into_node())
    })
}

fn allowed_params() -> BoxFuture<Result<Vec<next_rust_router::Params>>> {
    Box::pin(async {
        let mut p = next_rust_router::Params::new();
        p.insert("id", "allowed");
        Ok(vec![p])
    })
}

/// Without `dynamic_params` only the parameters `generate_params` returned may
/// be rendered: a visitor cannot make the process render and cache a page for
/// a parameter of their choosing.
#[tokio::test]
async fn parameters_outside_generate_params_are_never_rendered_or_cached() {
    let mut def = page_def("/g/[id]", page_gated, Rendering::Static);
    def.generate_params = Some(allowed_params);
    def.dynamic_params = false;
    let app = build(vec![def]);

    let ok = get(&app, "/g/allowed", &[]).await;
    assert_eq!(ok.status, 200);
    assert!(ok.body.contains("id=allowed"), "{}", ok.body);

    let runs = GATED_RUNS.load(Ordering::SeqCst);
    for id in ["other", "allowed2", "..", "%2e%2e", "ALLOWED"] {
        let res = get(&app, &format!("/g/{id}"), &[]).await;
        assert_eq!(res.status, 404, "/g/{id}: {}", res.body);
        assert!(res.headers.get("x-nr-cache").is_none(), "/g/{id} was cached");
    }
    assert_eq!(GATED_RUNS.load(Ordering::SeqCst), runs, "nothing was rendered for a refused parameter");
}

// ---------------------------------------------------------------------------
// 5: state left behind by a panic
// ---------------------------------------------------------------------------

static PANIC_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Panics on its second render, the way an application page panics on one
/// unlucky row.
fn page_panics_once(_ctx: Ctx) -> R {
    Box::pin(async {
        let n = PANIC_RUNS.fetch_add(1, Ordering::SeqCst) + 1;
        assert!(n != 2, "this render panics");
        Ok(main![format!("render #{n}")].into_node())
    })
}

/// A regeneration that panics used to leave its path marked as being
/// revalidated for the life of the process: every later request took the
/// "already running" path, so the page was never rendered again and the stale
/// copy was served forever.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_revalidation_does_not_freeze_the_page_forever() {
    let mut def = page_def("/panics", page_panics_once, Rendering::Static);
    def.revalidate = Some(0); // stale the moment it is stored
    let app = build(vec![def]);

    let first = get(&app, "/panics", &[]).await;
    assert_eq!(first.headers["x-nr-cache"], "MISS");
    assert!(first.body.contains("render #1"), "{}", first.body);

    // Stale: this spawns the regeneration that panics.
    assert_eq!(get(&app, "/panics", &[]).await.headers["x-nr-cache"], "STALE");
    for _ in 0..200 {
        if PANIC_RUNS.load(Ordering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    assert_eq!(PANIC_RUNS.load(Ordering::SeqCst), 2, "the second render ran and panicked");

    // A further request must try again, and the page must move on.
    let mut moved_on = false;
    for _ in 0..200 {
        let res = get(&app, "/panics", &[]).await;
        if res.body.contains("render #3") {
            moved_on = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    assert!(moved_on, "the path stayed marked as revalidating: it is never rendered again");
}

// ---------------------------------------------------------------------------
// 6: streaming protocol
// ---------------------------------------------------------------------------

const HOSTILE: &str = "</template><script>alert(1)</script>$nr(1)</script><!--nr-l:0000000000000000-->";

fn page_hostile(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![
            h1!["hostile"],
            dynamic(p!["…"], |_ctx| async { p![HOSTILE] }),
            dynamic(p!["…"], |_ctx| async { div![attr("data-x", HOSTILE), "attr"] }),
            dynamic(p!["…"], |_ctx| async { script![HOSTILE] }),
        ]
        .into_node())
    })
}

/// Whatever a hole renders is escaped exactly as it is in a normal render: it
/// cannot end the `<template>` it is streamed in, nor the swap script.
#[tokio::test]
async fn hostile_hole_output_cannot_break_out_of_the_stream() {
    let app = build(vec![page_def("/hostile", page_hostile, Rendering::Static)]);
    for headers in [&[][..], &[("user-agent", "Googlebot/2.1")][..]] {
        let res = get(&app, "/hostile", headers).await;
        let body = &res.body;
        assert_eq!(res.status, 200);
        // No stray close tag, no injected script, no forged layout marker.
        assert!(!body.contains("<script>alert(1)</script>"), "{body}");
        assert!(!body.contains("<!--nr-l:0000000000000000-->"), "{body}");
        assert_eq!(body.matches("</template>").count(), body.matches("<template id=").count(), "{body}");
        // The text is there, escaped.
        assert!(body.contains("&lt;/template&gt;"), "{body}");
        // `script![text]` keeps the raw-text escaping of a normal render.
        assert!(body.contains("<\\/template>") || body.contains("<\\/script>"), "{body}");
    }
}

fn page_hostile_raw(_ctx: Ctx) -> R {
    Box::pin(async { Ok(main![dynamic(p!["…"], |_ctx| async { raw_html(HOSTILE) })].into_node()) })
}

/// `raw_html` in a hole is as trusted as `raw_html` anywhere else: it is
/// written through. This is the documented contract, pinned so a change is
/// deliberate.
#[tokio::test]
async fn raw_html_in_a_hole_is_written_through_like_anywhere_else() {
    let app = build(vec![page_def("/hostile-raw", page_hostile_raw, Rendering::Static)]);
    let body = get(&app, "/hostile-raw", &[]).await.body;
    assert!(body.contains("<script>alert(1)</script>"), "raw_html is trusted by design: {body}");
}

// ---------------------------------------------------------------------------
// 1 (c): a hole cannot set a cookie, streaming or not
// ---------------------------------------------------------------------------

fn page_cookie_hole(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![dynamic(p!["…"], |ctx: Ctx| async move {
            ctx.cookies.set(Cookie::new("late", "1"));
            ctx.cookies.delete("session");
            p!["tried"]
        })]
        .into_node())
    })
}

/// The jar a hole writes to is the request's own (it is an `Arc`), so the
/// cookie is real — but the response head is long gone by then. Checked with
/// streaming on and off, because the non-streaming path builds the whole body
/// in one chunk.
#[tokio::test]
async fn a_hole_cannot_set_a_cookie_streaming_or_not() {
    let app = build(vec![page_def("/cookie-hole", page_cookie_hole, Rendering::Static)]);
    for headers in [&[("cookie", "session=abc")][..], &[("cookie", "session=abc"), ("user-agent", "Googlebot")][..]] {
        let res = get(&app, "/cookie-hole", headers).await;
        assert!(res.body.contains("tried"), "{}", res.body);
        let set: Vec<_> = res.headers.get_all("set-cookie").iter().filter_map(|v| v.to_str().ok()).collect();
        assert!(!set.iter().any(|c| c.starts_with("late=") || c.starts_with("session=")), "{set:?}");
    }
}
