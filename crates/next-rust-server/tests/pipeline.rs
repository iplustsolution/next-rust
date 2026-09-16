//! End-to-end tests of the request pipeline with hand-written route
//! definitions (the same structures the code generator emits).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;
use http::Method;
use next_rust_cache::MemoryStore;
use next_rust_core::{Config, Environment};
use next_rust_router::Params;
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

fn root_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move { Ok(div![id("root"), header!["Site"], main![children]].into_node()) })
}
fn root_metadata(_ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async { Ok(Metadata::new().title("Acme").title_template("%s | Acme").description("root desc")) })
}
fn root_not_found(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["Nothing here"].into_node()) })
}
fn home(_ctx: Ctx) -> R {
    Box::pin(async { Ok(fragment![h1!["Home"], Link!(href = "/about", "About")]) })
}
fn blog_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move { Ok(section![class("blog"), children].into_node()) })
}
fn blog_not_found(_ctx: Ctx) -> R {
    Box::pin(async { Ok(p!["No such post"].into_node()) })
}
fn blog_error(_ctx: Ctx, info: ErrorInfo) -> Node {
    p![class("err"), format!("Blog error {}", info.status)].into_node()
}
fn post(ctx: Ctx) -> R {
    Box::pin(async move {
        let params = Params::from_context(&ctx)?;
        match params.get("slug") {
            Some("missing") => Err(not_found()),
            Some("broken") => Err(Error::msg("database exploded")),
            Some("moved") => Err(redirect("/blog/new-home")),
            Some(slug) => {
                let cookies = Cookies::from_context(&ctx)?;
                cookies.set(Cookie::new("last_post", slug.to_owned()));
                Ok(article![h2![slug.to_owned()]].into_node())
            }
            None => Err(not_found()),
        }
    })
}
fn post_metadata(ctx: Ctx) -> BoxFuture<Result<Metadata>> {
    Box::pin(async move { Ok(Metadata::new().title(ctx.params.get("slug").unwrap_or("?").to_owned())) })
}
fn crash(_ctx: Ctx) -> R {
    Box::pin(async { Err(Error::msg("secret internal detail")) })
}
fn loading(_ctx: Ctx) -> Node {
    p!["Loading dashboard…"].into_node()
}
fn dashboard(_ctx: Ctx) -> R {
    Box::pin(async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        Ok(h1!["Dashboard ready"].into_node())
    })
}
fn dash_layout(_ctx: Ctx, children: Children, mut slots: Slots) -> R {
    Box::pin(async move { Ok(div![aside![slots.take("stats")], children].into_node()) })
}
fn stats_slot(_ctx: Ctx) -> R {
    Box::pin(async { Ok(span!["42 users"].into_node()) })
}
fn dashboard_mw(req: Request, next: Next) -> BoxFuture<Response> {
    Box::pin(async move {
        if req.header("authorization").is_none() && req.path() != "/dashboard" {
            return Response::text("unauthorized").with_status(401);
        }
        next.run(req).await.with_header("x-dashboard", "1")
    })
}
fn root_mw(mut req: Request, next: Next) -> BoxFuture<Response> {
    Box::pin(async move {
        if req.path() == "/old-home" {
            req.set_path("/").unwrap();
        }
        next.run(req).await.with_header("x-root-mw", "1")
    })
}

static STATIC_RENDERS: AtomicUsize = AtomicUsize::new(0);
fn static_page(_ctx: Ctx) -> R {
    Box::pin(async {
        let n = STATIC_RENDERS.fetch_add(1, Ordering::SeqCst);
        Ok(p![format!("render #{n}")].into_node())
    })
}
fn static_needs_cookies(ctx: Ctx) -> R {
    Box::pin(async move {
        let _c = Cookies::from_context(&ctx)?;
        Ok(p!["x"].into_node())
    })
}
fn photo(ctx: Ctx) -> R {
    Box::pin(
        async move { Ok(div![class("full"), format!("photo {}", ctx.params.get("id").unwrap_or(""))].into_node()) },
    )
}
fn photo_modal(ctx: Ctx) -> R {
    Box::pin(
        async move { Ok(div![class("modal"), format!("modal {}", ctx.params.get("id").unwrap_or(""))].into_node()) },
    )
}

fn users_get(req: Request) -> BoxFuture<Response> {
    Box::pin(async move { Response::json(&serde_json::json!({ "id": req.param("id") })) })
}
fn users_post(mut req: Request) -> BoxFuture<Response> {
    Box::pin(async move {
        #[derive(serde::Deserialize)]
        struct Body {
            name: String,
        }
        match req.json::<Body>().await {
            Ok(b) => Response::json(&serde_json::json!({ "created": b.name })).with_status(201),
            Err(e) => e.into_response(),
        }
    })
}

#[derive(serde::Deserialize)]
struct NewUser {
    name: String,
}
async fn create_user(input: NewUser) -> Result<String> {
    if input.name.trim().is_empty() {
        return Err(Error::validation([("name", "Name is required")]));
    }
    if input.name == "admin" {
        return Err(redirect("/login"));
    }
    Ok(format!("hello {}", input.name))
}
fn create_user_handler(req: Request) -> BoxFuture<Response> {
    Box::pin(__private::run_action(req, create_user))
}
const CREATE_USER_ID: &str = "tests::create_user";

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
        metadata: Some(root_metadata),
        not_found: Some(root_not_found),
        ..Default::default()
    }
}

struct TestApp {
    app: App,
    dir: std::path::PathBuf,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn build(env: Environment) -> TestApp {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir =
        std::env::temp_dir().join(format!("nr-pipeline-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(dir.join("public/images")).unwrap();
    std::fs::write(dir.join("public/robots.txt"), "User-agent: *").unwrap();
    std::fs::write(dir.join("public/images/a.svg"), "<svg/>").unwrap();
    std::fs::write(dir.join("secret.txt"), "secret").unwrap();

    let blog = SegmentDef {
        layout: Some(blog_layout),
        not_found: Some(blog_not_found),
        error: Some(blog_error),
        ..Default::default()
    };
    let dash = SegmentDef {
        layout: Some(dash_layout),
        loading: Some(loading),
        middleware: Some(dashboard_mw),
        slots: vec![SlotDef { name: "stats", render: Some(stats_slot) }],
        ..Default::default()
    };
    let mut post_page =
        page("/blog/[slug]", PageBody::Rust(post), vec![root_seg(), blog.clone(), SegmentDef::default()]);
    post_page.metadata = Some(post_metadata);
    let mut static_p = page("/static", PageBody::Rust(static_page), vec![root_seg(), SegmentDef::default()]);
    static_p.rendering = Rendering::Static;
    static_p.revalidate = Some(3600);
    let mut static_bad =
        page("/static-bad", PageBody::Rust(static_needs_cookies), vec![root_seg(), SegmentDef::default()]);
    static_bad.rendering = Rendering::Static;
    let mut modal = page("/photo/[id]", PageBody::Rust(photo_modal), vec![root_seg()]);
    modal.intercept_from = Some("/feed");

    let routes = Routes {
        pages: vec![
            page("/", PageBody::Rust(home), vec![root_seg()]),
            post_page,
            page("/crash", PageBody::Rust(crash), vec![root_seg(), SegmentDef::default()]),
            page("/dashboard", PageBody::Rust(dashboard), vec![root_seg(), dash.clone()]),
            page("/dashboard/settings", PageBody::Rust(home), vec![root_seg(), dash, SegmentDef::default()]),
            page("/legacy", PageBody::Html("<p>legacy <b>html</b></p>"), vec![root_seg(), SegmentDef::default()]),
            page(
                "/full",
                PageBody::Html("<!DOCTYPE html><html><body>full doc</body></html>"),
                vec![root_seg(), SegmentDef::default()],
            ),
            static_p,
            static_bad,
            page("/photo/[id]", PageBody::Rust(photo), vec![root_seg(), SegmentDef::default()]),
            modal,
        ],
        apis: vec![ApiDef {
            pattern: "/api/users/[id]",
            source: "test",
            handlers: vec![(Method::GET, users_get), (Method::POST, users_post)],
            middleware: vec![],
        }],
        actions: vec![ActionDef { id: CREATE_USER_ID, handler: create_user_handler }],
        root: root_seg(),
        middleware: Some(root_mw),
        ..Default::default()
    };

    let mut config = Config::from_toml_str(
        r#"
        [server]
        body_limit = 64
        [[redirects]]
        source = "/old-blog/:slug"
        destination = "/blog/:slug"
        permanent = true
        [[headers]]
        source = "/api/:path*"
        headers = { "x-api" = "yes" }
        [security]
        csp = "script-src 'nonce-{nonce}'"
        "#,
    )
    .unwrap();
    config.root = dir.clone();
    let app = App::new(routes)
        .config(config)
        .environment(env)
        .page_store(MemoryStore::new(100))
        .middleware(|req: Request, next: Next| async move { next.run(req).await.with_header("x-global", "1") })
        .get("/health", |_req: Request| async { "ok" })
        .build()
        .unwrap();
    TestApp { app, dir }
}

async fn send(app: &App, req: http::Request<Bytes>) -> (u16, http::HeaderMap, String) {
    let res = app.handle(Request::from_http(req)).await;
    let status = res.status.as_u16();
    let headers = res.headers.clone();
    (status, headers, res.into_text().await)
}

async fn get(app: &App, uri: &str) -> (u16, http::HeaderMap, String) {
    send(app, http::Request::get(uri).body(Bytes::new()).unwrap()).await
}

#[tokio::test]
async fn renders_page_with_layout_metadata_and_security_headers() {
    let t = build(Environment::Test);
    let (status, headers, body) = get(&t.app, "/").await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "text/html; charset=utf-8");
    assert!(body.starts_with("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">"), "{body}");
    assert!(body.contains("<title>Acme</title>"));
    assert!(body.contains("<div id=\"root\"><header>Site</header><main><h1>Home</h1><a href=\"/about\" data-nr-link=\"\">About</a></main></div>"));
    // A Link! pulls in the client runtime with the CSP nonce.
    let csp = headers["content-security-policy"].to_str().unwrap();
    let nonce = csp.trim_start_matches("script-src 'nonce-").trim_end_matches('\'');
    assert!(body.contains(&"<script type=\"module\" src=\"/_nr/runtime.js?v=".to_string()));
    assert!(body.contains(&format!("nonce=\"{nonce}\"")));
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-global"], "1");
    assert_eq!(headers["x-root-mw"], "1");
}

#[tokio::test]
async fn dynamic_params_metadata_and_cookies() {
    let t = build(Environment::Test);
    let (status, headers, body) = get(&t.app, "/blog/hello-world").await;
    assert_eq!(status, 200);
    assert!(body.contains("<title>hello-world | Acme</title>"), "{body}");
    assert!(body.contains("<section class=\"blog\"><article><h2>hello-world</h2></article></section>"));
    assert_eq!(headers["set-cookie"], "last_post=hello-world; Path=/; HttpOnly; SameSite=Lax");
}

#[tokio::test]
async fn not_found_boundaries() {
    let t = build(Environment::Test);
    let (status, _, body) = get(&t.app, "/blog/missing").await;
    assert_eq!(status, 404);
    assert!(body.contains("<section class=\"blog\"><p>No such post</p></section>"), "{body}");

    let (status, _, body) = get(&t.app, "/does/not/exist").await;
    assert_eq!(status, 404);
    assert!(body.contains("<main><h1>Nothing here</h1></main>"));
    assert!(body.contains("<title>404: Not Found</title>"));

    let res = t
        .app
        .handle(Request::from_http(
            http::Request::get("/nope").header("accept", "application/json").body(Bytes::new()).unwrap(),
        ))
        .await;
    assert_eq!(res.status, 404);
    assert_eq!(res.into_text().await, "Not Found");
}

#[tokio::test]
async fn error_boundaries_and_global_errors() {
    let t = build(Environment::Production);
    let (status, _, body) = get(&t.app, "/blog/broken").await;
    assert_eq!(status, 500);
    assert!(body.contains("<section class=\"blog\"><p class=\"err\">Blog error 500</p></section>"), "{body}");
    assert!(body.contains("<header>Site</header>"), "layouts above the boundary still render");

    let (status, _, body) = get(&t.app, "/crash").await;
    assert_eq!(status, 500);
    assert!(body.contains("Something went wrong"));
    assert!(!body.contains("secret internal detail"), "production hides details");

    let t = build(Environment::Development);
    let (_, _, body) = get(&t.app, "/crash").await;
    assert!(body.contains("secret internal detail"), "development shows details");
}

#[tokio::test]
async fn redirects_from_pages_config_and_trailing_slash() {
    let t = build(Environment::Test);
    let (status, headers, _) = get(&t.app, "/blog/moved").await;
    assert_eq!(status, 307);
    assert_eq!(headers["location"], "/blog/new-home");

    let (status, headers, _) = get(&t.app, "/old-blog/rust?x=1").await;
    assert_eq!(status, 308);
    assert_eq!(headers["location"], "/blog/rust?x=1");

    let (status, headers, _) = get(&t.app, "/blog/hello/").await;
    assert_eq!(status, 308);
    assert_eq!(headers["location"], "/blog/hello");

    // root middleware rewrite
    let (status, _, body) = get(&t.app, "/old-home").await;
    assert_eq!(status, 200);
    assert!(body.contains("<h1>Home</h1>"));
}

#[tokio::test]
async fn streaming_with_loading_boundary_and_slots() {
    let t = build(Environment::Test);
    let res = t.app.handle(Request::get("/dashboard")).await;
    assert_eq!(res.status, 200);
    assert_eq!(res.header("x-dashboard"), Some("1"));
    let Body::Stream(mut stream) = res.body else { panic!("expected stream") };
    use futures_util::StreamExt;
    let first = String::from_utf8(stream.next().await.unwrap().unwrap().to_vec()).unwrap();
    assert!(
        first.contains("<aside><span>42 users</span></aside><nr-b id=\"nr-b1\"><p>Loading dashboard…</p></nr-b>"),
        "{first}"
    );
    assert!(!first.contains("Dashboard ready"));
    let mut rest = String::new();
    while let Some(chunk) = stream.next().await {
        rest.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
    }
    assert!(rest.contains("<template id=\"nr-t1\"><h1>Dashboard ready</h1></template>"));
    assert!(rest.ends_with("</body></html>"));

    // Bots get the resolved page in one piece.
    let res = t
        .app
        .handle(Request::from_http(
            http::Request::get("/dashboard").header("user-agent", "Googlebot/2.1").body(Bytes::new()).unwrap(),
        ))
        .await;
    let body = res.into_text().await;
    assert!(body.contains("<h1>Dashboard ready</h1>") && !body.contains("nr-b1"));
}

#[tokio::test]
async fn nested_middleware_scopes() {
    let t = build(Environment::Test);
    let (status, _, _) = get(&t.app, "/dashboard/settings").await;
    assert_eq!(status, 401);
    let (status, headers, _) = get(&t.app, "/").await;
    assert_eq!(status, 200);
    assert!(!headers.contains_key("x-dashboard"));
}

#[tokio::test]
async fn html_pages() {
    let t = build(Environment::Test);
    let (_, _, body) = get(&t.app, "/legacy").await;
    assert!(body.contains("<main><p>legacy <b>html</b></p></main>"), "fragments are wrapped in layouts");
    let (_, _, body) = get(&t.app, "/full").await;
    assert_eq!(body, "<!DOCTYPE html><html><body>full doc</body></html>");
}

#[tokio::test]
async fn api_routes() {
    let t = build(Environment::Test);
    let (status, headers, body) = get(&t.app, "/api/users/7").await;
    assert_eq!((status, body.as_str()), (200, r#"{"id":"7"}"#));
    assert_eq!(headers["x-api"], "yes");

    let req = http::Request::post("/api/users/7")
        .header("content-type", "application/json")
        .body(Bytes::from(r#"{"name":"Ada"}"#))
        .unwrap();
    let (status, _, body) = send(&t.app, req).await;
    assert_eq!((status, body.as_str()), (201, r#"{"created":"Ada"}"#));

    let req = http::Request::post("/api/users/7").body(Bytes::from("x".repeat(100))).unwrap();
    assert_eq!(send(&t.app, req).await.0, 413, "body limit");

    let req = http::Request::delete("/api/users/7").body(Bytes::new()).unwrap();
    let (status, headers, _) = send(&t.app, req).await;
    assert_eq!(status, 405);
    assert_eq!(headers["allow"], "GET, POST, HEAD");

    let req = http::Request::head("/api/users/7").body(Bytes::new()).unwrap();
    let (status, _, body) = send(&t.app, req).await;
    assert_eq!((status, body.as_str()), (200, ""));

    let req = http::Request::post("/").body(Bytes::new()).unwrap();
    assert_eq!(send(&t.app, req).await.0, 405, "pages only accept GET/HEAD");

    assert_eq!(get(&t.app, "/health").await.2, "ok");
}

#[tokio::test]
async fn public_files() {
    let t = build(Environment::Test);
    let (status, headers, body) = get(&t.app, "/robots.txt").await;
    assert_eq!((status, body.as_str()), (200, "User-agent: *"));
    assert!(headers.contains_key("etag"));
    let etag = headers["etag"].to_str().unwrap().to_owned();
    let req = http::Request::get("/robots.txt").header("if-none-match", etag).body(Bytes::new()).unwrap();
    assert_eq!(send(&t.app, req).await.0, 304);
    assert_eq!(get(&t.app, "/images/a.svg").await.1["content-type"], "image/svg+xml");
    let req = http::Request::get("/robots.txt").header("range", "bytes=0-3").body(Bytes::new()).unwrap();
    let (status, _, body) = send(&t.app, req).await;
    assert_eq!((status, body.as_str()), (206, "User"));
    for bad in ["/../secret.txt", "/%2e%2e/secret.txt", "/images/..%2f..%2fsecret.txt"] {
        let (status, _, body) = get(&t.app, bad).await;
        assert_ne!(status, 200, "{bad}");
        assert!(!body.contains("secret"), "{bad}");
    }
}

#[tokio::test]
async fn server_actions() {
    let t = build(Environment::Test);
    let url = action_url(CREATE_USER_ID);

    let json = |body: &str, origin: Option<&str>| {
        let mut b =
            http::Request::post(url.as_str()).header("host", "example.com").header("content-type", "application/json");
        if let Some(o) = origin {
            b = b.header("origin", o);
        }
        b.body(Bytes::from(body.to_owned())).unwrap()
    };
    let (status, _, body) = send(&t.app, json(r#"{"name":"Ada"}"#, Some("https://example.com"))).await;
    assert_eq!((status, body.as_str()), (200, r#"{"data":"hello Ada","ok":true}"#));

    let (status, _, body) = send(&t.app, json(r#"{"name":" "}"#, None)).await;
    assert_eq!(status, 422);
    assert_eq!(body, r#"{"errors":{"name":"Name is required"},"ok":false}"#);

    let (status, _, body) = send(&t.app, json(r#"{"name":"admin"}"#, None)).await;
    assert_eq!((status, body.as_str()), (200, r#"{"ok":false,"redirect":"/login"}"#));

    let (status, _, _) = send(&t.app, json(r#"{"name":"Ada"}"#, Some("https://evil.example"))).await;
    assert_eq!(status, 403, "cross-origin actions are rejected");

    let req = http::Request::post(url.as_str())
        .header("host", "example.com")
        .header("sec-fetch-site", "cross-site")
        .header("content-type", "application/json")
        .body(Bytes::from(r#"{"name":"Ada"}"#))
        .unwrap();
    assert_eq!(send(&t.app, req).await.0, 403);

    // Progressive enhancement: plain HTML form post.
    let form = |body: &str| {
        http::Request::post(url.as_str())
            .header("host", "example.com")
            .header("referer", "http://example.com/signup?step=1")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Bytes::from(body.to_owned()))
            .unwrap()
    };
    let (status, headers, _) = send(&t.app, form("name=Ada")).await;
    assert_eq!(status, 303);
    assert_eq!(headers["location"], "/signup?step=1");
    let (status, headers, _) = send(&t.app, form("name=&password=hunter2")).await;
    assert_eq!(status, 303);
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(cookie.starts_with("nr_flash="));
    assert!(!cookie.contains("hunter2"), "passwords are never echoed back");
    let (status, headers, _) = send(&t.app, form("name=Ada&_redirect=//evil.com")).await;
    assert_eq!((status, headers["location"].to_str().unwrap()), (303, "/signup?step=1"), "open redirects are ignored");

    let req = http::Request::get(url.as_str()).body(Bytes::new()).unwrap();
    assert_eq!(send(&t.app, req).await.0, 405);
    let req = http::Request::post("/_nr/action/unknown").body(Bytes::new()).unwrap();
    assert_eq!(send(&t.app, req).await.0, 404);
}

#[tokio::test]
async fn incremental_static_regeneration() {
    let t = build(Environment::Production);
    let before = STATIC_RENDERS.load(Ordering::SeqCst);
    let (status, headers, body1) = get(&t.app, "/static").await;
    assert_eq!(status, 200);
    assert_eq!(headers["x-nr-cache"], "MISS");
    assert_eq!(headers["cache-control"], "public, max-age=0, s-maxage=3600, stale-while-revalidate");
    let (_, headers, body2) = get(&t.app, "/static").await;
    assert_eq!(headers["x-nr-cache"], "HIT");
    assert_eq!(body1.replace(|c: char| c.is_ascii_hexdigit(), ""), body2.replace(|c: char| c.is_ascii_hexdigit(), ""));
    assert_eq!(STATIC_RENDERS.load(Ordering::SeqCst), before + 1);

    assert!(t.app.revalidate_path("/static/").await);
    let (_, headers, _) = get(&t.app, "/static").await;
    assert_eq!(headers["x-nr-cache"], "MISS");

    // Request data in a static page is a build error, not silently dynamic.
    let (status, _, body) = get(&t.app, "/static-bad").await;
    assert_eq!(status, 500);
    assert!(!body.contains("Cookies"), "details are not leaked in production");
}

#[tokio::test]
async fn intercepting_routes_on_soft_navigation() {
    let t = build(Environment::Test);
    let (_, _, body) = get(&t.app, "/photo/1").await;
    assert!(body.contains("photo 1") && !body.contains("modal"));
    let req =
        http::Request::get("/photo/1").header("x-nr-nav", "1").header("x-nr-from", "/feed").body(Bytes::new()).unwrap();
    let (_, _, body) = send(&t.app, req).await;
    assert!(body.contains("modal 1"), "{body}");
    let req = http::Request::get("/photo/1")
        .header("x-nr-nav", "1")
        .header("x-nr-from", "/other")
        .body(Bytes::new())
        .unwrap();
    assert!(send(&t.app, req).await.2.contains("photo 1"));
}

#[tokio::test]
async fn framework_endpoints() {
    let t = build(Environment::Test);
    let (status, headers, body) = get(&t.app, "/_nr/runtime.js").await;
    assert_eq!(status, 200);
    assert!(headers["content-type"].to_str().unwrap().starts_with("text/javascript"));
    assert!(body.contains("window.nextRust"));
    let (status, _, _) = get(&t.app, "/_nr/dev/events").await;
    assert_ne!(status, 200, "dev endpoints are disabled outside development");
    let (status, headers, _) = get(&t.app, "/_nr/image?url=/images/a.svg&w=640").await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "image/svg+xml");
    assert_eq!(get(&t.app, "/_nr/image?url=https://evil.com/x.png&w=640").await.0, 400);
    assert_eq!(get(&t.app, "/_nr/image?url=/../secret.txt&w=640").await.0, 404);
}

#[tokio::test]
async fn real_server_gzip_and_keep_alive() {
    use std::io::Read;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let t = build(Environment::Test);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = t.app.clone();
    let server = tokio::spawn(async move { app.serve_listener(listener).await });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(
            b"GET /blog/compressed HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n",
        )
        .await
        .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_ascii_lowercase();
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert!(head.contains("content-encoding: gzip"));
    assert!(head.contains("transfer-encoding: chunked"));
    // De-chunk.
    let mut body = &raw[split + 4..];
    let mut gz = Vec::new();
    loop {
        let line_end = body.windows(2).position(|w| w == b"\r\n").unwrap();
        let size = usize::from_str_radix(std::str::from_utf8(&body[..line_end]).unwrap().trim(), 16).unwrap();
        body = &body[line_end + 2..];
        if size == 0 {
            break;
        }
        gz.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
    let mut html = String::new();
    flate2::read::GzDecoder::new(&gz[..]).read_to_string(&mut html).unwrap();
    assert!(html.contains("<h2>compressed</h2>"));
    server.abort();
}
