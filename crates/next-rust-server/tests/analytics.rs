//! `[verification]` and `[analytics]` end to end: every kind of document gets
//! the tags exactly once, client navigations get none, excluded and opted-out
//! pages stay clean, and the `strict` CSP allows what was configured.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;
use next_rust_cache::MemoryStore;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

const LOADER: &str = "https://www.googletagmanager.com/gtag/js?id=G-TEST1234";
const NOSCRIPT: &str = "<noscript><iframe src=\"https://www.googletagmanager.com/ns.html?id=GTM-TEST12\"";
const OPT_OUT: &str = "addEventListener(\"nr:navigate\",function(){location.reload()})";

fn root_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move { Ok(div![id("root"), nav![a![href("/"), "Home"]], children].into_node()) })
}
fn root_metadata(_ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async { Ok(Metadata::new().title("Site")) })
}
fn home(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["Home"].into_node()) })
}
fn about(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["About"].into_node()) })
}
fn panel(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["Panel"].into_node()) })
}
fn private_metadata(_ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async { Ok(Metadata::new().analytics(false)) })
}
fn hidden_metadata(_ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async { Ok(Metadata::new().robots("noindex, nofollow")) })
}
fn crash(_ctx: Ctx) -> R {
    Box::pin(async { Err(Error::msg("boom")) })
}
fn loading(_ctx: Ctx) -> Node {
    p!["loading…"].into_node()
}
fn slow(_ctx: Ctx) -> R {
    Box::pin(async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        Ok(h1!["Slow"].into_node())
    })
}
fn with_hole(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(main![
            h1!["Product"],
            dynamic(p!["…"], |ctx| async move { p![format!("hello {}", ctx.cookies.get("user").unwrap_or_default())] })
        ]
        .into_node())
    })
}

fn page(pattern: &'static str, body: PageBody, segments: Vec<SegmentDef>) -> PageDef {
    PageDef {
        pattern,
        source: "test",
        body,
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

fn root_seg() -> SegmentDef {
    SegmentDef {
        layout: Some(root_layout),
        layout_id: "root",
        layout_reusable: true,
        metadata: Some(root_metadata),
        ..Default::default()
    }
}

const CONFIG: &str = r#"
[verification]
google = "g-proof"
bing = "B1NG"
google_file = "google1a2b3c.html"
bing_file = "ABCDEF0123"
ads_txt = ["google.com, pub-1, DIRECT, f08c47fec0942fa0"]
assetlinks = '[{"relation": ["delegate_permission/common.handle_all_urls"]}]'

[analytics]
google_analytics = "G-TEST1234"
google_tag_manager = "GTM-TEST12"
exclude_paths = ["/student/*", "/login"]
skip_noindex = true

[security]
csp = "strict"
"#;

struct TestApp {
    app: App,
    dir: std::path::PathBuf,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn build(env: Environment, config: &str) -> TestApp {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir =
        std::env::temp_dir().join(format!("nr-analytics-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(dir.join("public")).unwrap();

    let leaf = SegmentDef::default;
    let mut static_about = page("/static", PageBody::Rust(about), vec![root_seg(), leaf()]);
    static_about.rendering = Rendering::Static;
    let mut static_private = page("/static-private", PageBody::Rust(about), vec![root_seg(), leaf()]);
    static_private.rendering = Rendering::Static;
    static_private.metadata = Some(private_metadata);
    let mut ppr = page("/product", PageBody::Rust(with_hole), vec![root_seg(), leaf()]);
    ppr.rendering = Rendering::Static;
    let mut private = page("/private", PageBody::Rust(about), vec![root_seg(), leaf()]);
    private.metadata = Some(private_metadata);
    let mut hidden = page("/hidden", PageBody::Rust(about), vec![root_seg(), leaf()]);
    hidden.metadata = Some(hidden_metadata);
    let streamed = SegmentDef { loading: Some(loading), ..Default::default() };

    let routes = Routes {
        pages: vec![
            page("/", PageBody::Rust(home), vec![root_seg()]),
            page("/about", PageBody::Rust(about), vec![root_seg(), leaf()]),
            page("/student/home", PageBody::Rust(panel), vec![root_seg(), leaf()]),
            page("/student/marks", PageBody::Rust(panel), vec![root_seg(), leaf()]),
            page("/login", PageBody::Rust(panel), vec![root_seg(), leaf()]),
            page("/crash", PageBody::Rust(crash), vec![root_seg(), leaf()]),
            page("/slow", PageBody::Rust(slow), vec![root_seg(), streamed]),
            page(
                "/full",
                PageBody::Html(
                    "<!DOCTYPE html><html><head><title>f</title></head><body class=\"x\">full doc</body></html>",
                ),
                vec![root_seg(), leaf()],
            ),
            private,
            hidden,
            static_about,
            static_private,
            ppr,
        ],
        root: root_seg(),
        ..Default::default()
    };
    let mut config = Config::from_toml_str(config).unwrap();
    config.root = dir.clone();
    let app = App::new(routes).config(config).environment(env).page_store(MemoryStore::new(100)).build().unwrap();
    TestApp { app, dir }
}

struct Res {
    status: u16,
    headers: http::HeaderMap,
    body: String,
}

impl Res {
    fn header(&self, name: &str) -> &str {
        self.headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or("")
    }
}

async fn send(app: &App, req: http::Request<Bytes>) -> Res {
    let res = app.handle(Request::from_http(req)).await;
    let (status, headers) = (res.status.as_u16(), res.headers.clone());
    Res { status, headers, body: res.into_text().await }
}

async fn get(app: &App, uri: &str) -> Res {
    send(app, http::Request::get(uri).body(Bytes::new()).unwrap()).await
}

/// A client-side navigation to `uri` from the page at `from`.
async fn navigate(app: &App, from: &str, uri: &str) -> Res {
    let req = http::Request::get(uri).header("x-nr-nav", "1").header("x-nr-from", from).body(Bytes::new()).unwrap();
    send(app, req).await
}

/// The document has the tags, once, where they belong.
fn assert_tagged(res: &Res, what: &str) {
    let html = &res.body;
    assert_eq!(html.matches(LOADER).count(), 1, "{what}: the loader, once\n{html}");
    assert_eq!(html.matches(NOSCRIPT).count(), 1, "{what}: the <noscript> frame, once\n{html}");
    assert_eq!(html.matches("google-site-verification").count(), 1, "{what}: verification, once");
    let head_end = html.find("</head>").expect("head");
    assert!(html.find(LOADER).unwrap() < head_end, "{what}: the script is in <head>");
    assert!(html.contains(&format!("<body>{NOSCRIPT}")), "{what}: the frame comes right after <body>\n{html}");
    assert!(!html.contains(OPT_OUT));
    // The inline script carries the nonce the CSP header allows.
    let csp = res.header("content-security-policy");
    let nonce = csp.split("'nonce-").nth(1).and_then(|r| r.split('\'').next()).expect("a nonce in the CSP");
    assert!(html.contains(&format!("<script nonce=\"{nonce}\">!function(w,d){{")), "{what}: nonce {nonce}\n{html}");
}

fn assert_clean(res: &Res, what: &str) {
    assert!(!res.body.contains("googletagmanager.com"), "{what}: no analytics\n{}", res.body);
    assert!(!res.body.contains("!function(w,d)"), "{what}: no analytics script");
}

#[tokio::test]
async fn every_kind_of_document_gets_the_tags_once() {
    let t = build(Environment::Production, CONFIG);
    let home = get(&t.app, "/").await;
    assert_tagged(&home, "dynamic page");
    assert!(home.body.contains("<meta name=\"google-site-verification\" content=\"g-proof\">"));
    assert!(home.body.contains("<meta name=\"msvalidate.01\" content=\"B1NG\">"));

    assert_tagged(&get(&t.app, "/slow").await, "streamed page");
    let miss = get(&t.app, "/static").await;
    assert_eq!(miss.header("x-nr-cache"), "MISS");
    assert_tagged(&miss, "static page, first render");
    let hit = get(&t.app, "/static").await;
    assert_eq!(hit.header("x-nr-cache"), "HIT");
    assert_tagged(&hit, "static page, cached");
    assert_ne!(
        miss.header("content-security-policy"),
        hit.header("content-security-policy"),
        "every response has its own nonce"
    );

    for _ in 0..2 {
        let ppr = get(&t.app, "/product").await;
        assert_eq!(ppr.header("x-nr-prerender"), "partial");
        assert!(ppr.body.contains("hello"), "the hole was filled");
        assert_tagged(&ppr, "partially prerendered page");
    }

    let missing = get(&t.app, "/nope").await;
    assert_eq!(missing.status, 404);
    let crash = get(&t.app, "/crash").await;
    assert_eq!(crash.status, 500);
    // Both say `noindex`, and this app sets `skip_noindex`.
    for (res, what) in [(&missing, "404"), (&crash, "error page")] {
        assert_clean(res, what);
        assert_eq!(res.body.matches("google-site-verification").count(), 1, "{what}: verification stays");
        assert_eq!(res.body.matches(OPT_OUT).count(), 1, "{what}");
    }

    let full = get(&t.app, "/full").await;
    assert_eq!(full.body.matches(LOADER).count(), 1);
    assert!(full.body.contains(&format!("<body class=\"x\">{NOSCRIPT}")), "{}", full.body);
    assert!(full.body.contains("google-site-verification"));
}

#[tokio::test]
async fn error_pages_are_measured_unless_noindex_is_skipped() {
    let t = build(Environment::Production, &CONFIG.replace("skip_noindex = true", ""));
    let missing = get(&t.app, "/nope").await;
    assert_eq!(missing.status, 404);
    assert_tagged(&missing, "404");
    let crash = get(&t.app, "/crash").await;
    assert_eq!(crash.status, 500);
    assert_tagged(&crash, "error page");
    assert_tagged(&get(&t.app, "/hidden").await, "noindex page");
}

#[tokio::test]
async fn development_gets_verification_but_no_analytics() {
    let t = build(Environment::Development, CONFIG);
    let home = get(&t.app, "/").await;
    assert_clean(&home, "development");
    assert!(home.body.contains("<meta name=\"google-site-verification\" content=\"g-proof\">"));
    assert!(!home.header("content-security-policy").contains("googletagmanager"), "nothing to allow");
    // No analytics, so no boundaries either.
    let nav = navigate(&t.app, "/", "/student/home").await;
    assert_eq!(nav.status, 200);

    let t = build(Environment::Development, &CONFIG.replace("skip_noindex = true", "enabled_in_dev = true"));
    let home = get(&t.app, "/").await;
    assert_eq!(home.body.matches(LOADER).count(), 1, "enabled_in_dev");
    assert!(home.header("content-security-policy").contains("https://www.googletagmanager.com"));
}

#[tokio::test]
async fn excluded_and_opted_out_pages_stay_clean() {
    let t = build(Environment::Production, CONFIG);
    for path in ["/student/home", "/login"] {
        let res = get(&t.app, path).await;
        assert_eq!(res.status, 200);
        assert_clean(&res, path);
        assert!(!res.body.contains(OPT_OUT), "an excluded path is recognized by the server");
        assert!(res.body.contains("google-site-verification"));
    }
    for path in ["/private", "/hidden", "/static-private", "/static-private"] {
        let res = get(&t.app, path).await;
        assert_eq!(res.status, 200);
        assert_clean(&res, path);
        assert_eq!(res.body.matches(OPT_OUT).count(), 1, "{path}: reloads when the visitor moves on");
    }
}

#[tokio::test]
async fn client_navigations_carry_no_tags() {
    let t = build(Environment::Production, CONFIG);
    for path in ["/about", "/slow", "/static", "/static", "/product", "/product"] {
        let nav = navigate(&t.app, "/", path).await;
        assert_ne!(nav.status, 204, "{path}");
        assert_clean(&nav, path);
        assert!(nav.body.contains("About") || nav.body.contains("Slow") || nav.body.contains("Product"));
        assert!(nav.header("cache-control").contains("no-store"), "{path}: {}", nav.header("cache-control"));
    }
    // The cached copies still have them for the next full load.
    assert_tagged(&get(&t.app, "/static").await, "static page after a navigation");
    assert_tagged(&get(&t.app, "/product").await, "ppr page after a navigation");
}

#[tokio::test]
async fn crossing_the_analytics_boundary_is_a_full_page_load() {
    let t = build(Environment::Production, CONFIG);
    // Not HTML: the client runtime answers that with `location.href = url`.
    for (from, to) in [
        ("/", "/student/home"),
        ("/student/home", "/"),
        ("/student/marks", "/static"),
        ("/about", "/login"),
        ("/student/home", "/nope"),
        ("/", "/private"),
        ("/", "/static-private"),
        ("/", "/static-private"),
        ("/", "/hidden"),
        // The 404 page says `noindex`, and this app sets `skip_noindex`.
        ("/", "/nope"),
    ] {
        let nav = navigate(&t.app, from, to).await;
        assert_eq!(nav.status, 204, "{from} → {to}");
        assert_eq!(nav.header("x-nr-reload"), "1");
        assert!(!nav.header("content-type").contains("text/html"));
        assert!(nav.header("cache-control").contains("no-store"));
        assert_eq!(nav.body, "");
    }
    // Inside either side navigations stay client-side.
    for (from, to) in [("/student/home", "/student/marks"), ("/", "/about"), ("/student/marks", "/login")] {
        let nav = navigate(&t.app, from, to).await;
        assert_eq!(nav.status, 200, "{from} → {to}");
        assert_clean(&nav, to);
    }
}

#[tokio::test]
async fn strict_csp_allows_the_configured_tags() {
    let t = build(Environment::Production, CONFIG);
    let res = get(&t.app, "/student/home").await;
    let csp = res.header("content-security-policy");
    for needle in [
        "script-src 'self' 'nonce-",
        "'strict-dynamic' https://www.googletagmanager.com;",
        "connect-src 'self' https://www.googletagmanager.com https://*.google-analytics.com https://*.google.com https://www.google.com;",
        "img-src 'self' data: blob: https://www.googletagmanager.com https://*.google-analytics.com;",
        "frame-src 'self' https://www.googletagmanager.com",
    ] {
        assert!(csp.contains(needle), "missing `{needle}` in `{csp}`");
    }
    assert!(!csp.contains("{nonce}") && !csp.contains("facebook") && !csp.contains("clarity"));
}

#[tokio::test]
async fn verification_files_are_served_from_the_root() {
    for env in [Environment::Development, Environment::Production] {
        let t = build(env, CONFIG);
        let google = get(&t.app, "/google1a2b3c.html").await;
        assert_eq!(google.status, 200);
        assert_eq!(google.body, "google-site-verification: google1a2b3c.html");
        assert!(google.header("content-type").starts_with("text/html"));
        let bing = get(&t.app, "/BingSiteAuth.xml").await;
        assert_eq!(bing.body, "<?xml version=\"1.0\"?>\n<users>\n\t<user>ABCDEF0123</user>\n</users>");
        assert!(bing.header("content-type").contains("xml"));
        let ads = get(&t.app, "/ads.txt").await;
        assert_eq!(ads.body, "google.com, pub-1, DIRECT, f08c47fec0942fa0\n");
        assert!(ads.header("content-type").starts_with("text/plain"));
        let links = get(&t.app, "/.well-known/assetlinks.json").await;
        assert_eq!(links.status, 200, "public/ never serves dot-directories; the config does");
        assert_eq!(links.header("content-type"), "application/json");
        assert!(links.body.starts_with("[{\"relation\""));
        assert_eq!(get(&t.app, "/.well-known/apple-app-site-association").await.status, 404);
        assert_eq!(get(&t.app, "/app-ads.txt").await.status, 404);
        assert_eq!(get(&t.app, "/google-other.html").await.status, 404);
    }
}

#[tokio::test]
async fn an_app_without_the_sections_is_untouched() {
    let t = build(Environment::Production, "[security]\ncsp = \"strict\"\n");
    let home = get(&t.app, "/").await;
    assert_clean(&home, "no config");
    assert!(!home.body.contains("verification") && !home.body.contains("<noscript>"));
    assert_eq!(
        home.header("content-security-policy").replace(|c: char| c.is_ascii_hexdigit(), ""),
        STRICT_CSP.replace("{nonce}", "").replace(|c: char| c.is_ascii_hexdigit(), "")
    );
    assert_eq!(navigate(&t.app, "/", "/student/home").await.status, 200);
}
