//! Regression tests for findings of the second security audit.

use bytes::Bytes;
use futures_util::FutureExt;
use next_rust_core::{Config, Environment};
use next_rust_server::actions::ActionRef;
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

fn photo(ctx: Ctx) -> R {
    Box::pin(async move { Ok(p![format!("photo {}", ctx.params.get("id").unwrap_or(""))].into_node()) })
}
fn photo_modal(ctx: Ctx) -> R {
    Box::pin(async move { Ok(p![format!("modal {}", ctx.params.get("id").unwrap_or(""))].into_node()) })
}
fn home(_ctx: Ctx) -> R {
    Box::pin(async { Ok(fragment![h1!["Home"], Link!(href = "/photo/1", "Photo")]) })
}

/// A directory of its own for every app a test builds: tests in one binary
/// run in parallel, so a path keyed only by the process id is shared.
fn temp_root(label: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("nr-security-{label}-{}-{n}", std::process::id()))
}

fn page(pattern: &'static str, body: PageBody, intercept_from: Option<&'static str>) -> PageDef {
    PageDef {
        pattern,
        source: "test",
        body,
        segments: vec![SegmentDef::default()],
        metadata: None,
        // What `[rendering] default = "static"` makes of every page.
        rendering: Rendering::Static,
        revalidate: None,
        generate_params: None,
        dynamic_params: true,
        tags: &[],
        intercept_from,
    }
}

fn app(configure: impl FnOnce(&mut Config)) -> App {
    let routes = Routes {
        pages: vec![
            page("/", PageBody::Rust(home), None),
            page("/photo/[id]", PageBody::Rust(photo), None),
            page("/photo/[id]", PageBody::Rust(photo_modal), Some("/feed")),
        ],
        ..Default::default()
    };
    let mut config = Config { root: temp_root("app"), ..Config::default() };
    configure(&mut config);
    App::new(routes).config(config).environment(Environment::Test).build().unwrap()
}

async fn send(app: &App, uri: &str, headers: &[(&str, &str)]) -> (http::HeaderMap, String) {
    let mut req = http::Request::get(uri);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let res = app.handle(Request::from_http(req.body(Bytes::new()).unwrap())).await;
    (res.headers.clone(), res.into_text().await)
}

/// A static intercepting route and the static page at the same URL used one
/// page-cache entry: whoever asked first decided what everyone else got.
#[tokio::test]
async fn an_intercepting_route_cannot_poison_the_page_cache() {
    let app = app(|_| {});
    let nav = [("x-nr-nav", "1"), ("x-nr-from", "/feed")];
    // The attacker goes first, before the page is cached.
    let (headers, body) = send(&app, "/photo/1", &nav).await;
    assert!(body.contains("modal 1"), "{body}");
    assert!(!headers.contains_key("x-nr-cache"), "intercepted renders are per request");
    for _ in 0..2 {
        let (_, body) = send(&app, "/photo/1", &[]).await;
        assert!(body.contains("photo 1") && !body.contains("modal"), "{body}");
    }
    // And the cached page is not served in place of the modal either.
    let (_, body) = send(&app, "/photo/1", &nav).await;
    assert!(body.contains("modal 1"), "{body}");
}

/// With inline scripts, a client navigation gets a document without the
/// scripts the browser reported. A shared cache that ignores `Vary` (several
/// CDNs do) would hand that to the next full page load.
#[tokio::test]
async fn navigation_responses_with_inline_scripts_are_never_shared() {
    let app = app(|c| c.build.inline_assets = true);
    let (headers, body) = send(&app, "/", &[]).await;
    assert!(body.contains("data-nr-js=\"rt-"), "the document carries the runtime: {body}");
    assert!(headers["cache-control"].to_str().unwrap().contains("public"));
    let (headers, body) = send(&app, "/", &[("x-nr-nav", "1")]).await;
    assert!(!body.contains("data-nr-js=\"rt-"), "{body}");
    let cache_control = headers["cache-control"].to_str().unwrap();
    assert!(cache_control.contains("no-store") && cache_control.contains("private"), "{cache_control}");
}

/// tungstenite's own limit is 64 MiB a message: every connected client could
/// make the server buffer that much. `ws::upgrade` allows 1 MiB.
#[cfg(feature = "websocket")]
#[tokio::test]
async fn websocket_messages_over_the_limit_close_the_connection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let routes = Routes::default();
    let config = Config { root: temp_root("ws"), ..Config::default() };
    let app = App::new(routes)
        .config(config)
        .environment(Environment::Test)
        .ws("/ws", move |mut socket| {
            let tx = tx.clone();
            async move {
                while let Some(message) = socket.recv().await {
                    let _ = tx.send(match message {
                        Ok(m) => format!("ok {}", m.len()),
                        Err(e) => format!("err {e}"),
                    });
                }
            }
        })
        .build()
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { app.serve_listener(listener).await });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let handshake = "GET /ws HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                     Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n";
    stream.write_all(handshake.as_bytes()).await.unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    assert!(String::from_utf8_lossy(&buf[..n]).starts_with("HTTP/1.1 101"));

    // A masked text frame of 5 bytes, then the header of one that claims 8 MiB.
    let frame = |len: u64| {
        let mut f = vec![0x81u8];
        if len < 126 {
            f.push(0x80 | len as u8);
        } else {
            f.push(0x80 | 127);
            f.extend_from_slice(&len.to_be_bytes());
        }
        f.extend_from_slice(&[0, 0, 0, 0]);
        f
    };
    let mut small = frame(5);
    small.extend_from_slice(b"hello");
    stream.write_all(&small).await.unwrap();
    stream.write_all(&frame(8 * 1024 * 1024)).await.unwrap();
    let wait = std::time::Duration::from_secs(5);
    assert_eq!(tokio::time::timeout(wait, rx.recv()).await.unwrap().as_deref(), Some("ok 5"));
    let refused = tokio::time::timeout(wait, rx.recv()).await.unwrap().unwrap();
    assert!(refused.starts_with("err"), "refused from the frame header, before any of it is buffered: {refused}");
    server.abort();
}

/// Every HTML response has its action markers replaced with a token bound to
/// this visitor's cookie, and the visitor gets that cookie. A complete body
/// is then marked `private, no-store` — a streamed one was not, so an
/// application that streams a cacheable HTML page with a form handed a
/// shared cache one visitor's token and binding cookie.
#[tokio::test]
async fn a_streamed_page_with_an_action_is_never_stored_by_a_shared_cache() {
    fn streamed(html: String, cache_control: &str) -> Response {
        let chunks = vec![Ok::<_, next_rust_server::request::BoxError>(Bytes::from(html))];
        let stream: next_rust_server::response::ByteStream = Box::pin(futures_util::stream::iter(chunks));
        Response::new(http::StatusCode::OK, Body::Stream(stream))
            .with_content_type("text/html; charset=utf-8")
            .with_cache_control(cache_control)
    }
    fn action_handler(_req: Request) -> BoxFuture<Response> {
        Box::pin(async { Response::text("ran") })
    }
    const ID: &str = "tests::streamed";

    let page = render_static(form![ActionRef::new(ID), button!["Go"]]);
    assert!(page.contains("/_next-rust/action/nr~"), "the marker is in the body: {page}");
    let app = |actions: Vec<ActionDef>| {
        let config = Config { root: temp_root("seal"), ..Config::default() };
        let page = page.clone();
        App::new(Routes { actions, ..Default::default() })
            .config(config)
            .environment(Environment::Test)
            .get("/streamed", move |_req: Request| {
                let page = page.clone();
                async move { streamed(page, "public, max-age=600") }
            })
            .build()
            .unwrap()
    };

    let with_action = app(vec![ActionDef { id: ID, handler: action_handler }]);
    let res = with_action.handle(Request::from_http(http::Request::get("/streamed").body(Bytes::new()).unwrap())).await;
    let headers = res.headers.clone();
    let cache_control = headers["cache-control"].to_str().unwrap();
    assert_eq!(
        cache_control, "private, no-cache, no-store, max-age=0, must-revalidate",
        "the header every response that must not be kept uses"
    );
    let cookie = headers["set-cookie"].to_str().unwrap().to_owned();
    assert!(cookie.contains("next_rust_bind"), "the binding cookie: {cookie}");
    let body = res.into_text().await;
    assert!(!body.contains("nr~"), "the marker became a token: {body}");

    // Only HTML: a streamed download or an event stream keeps its own.
    let other = App::new(Routes::default())
        .config(Config { root: temp_root("seal2"), ..Config::default() })
        .environment(Environment::Test)
        .get("/data", |_req: Request| async {
            let chunks = vec![Ok::<_, next_rust_server::request::BoxError>(Bytes::from_static(b"1,2,3"))];
            let stream: next_rust_server::response::ByteStream = Box::pin(futures_util::stream::iter(chunks));
            Response::new(http::StatusCode::OK, Body::Stream(stream))
                .with_content_type("text/csv")
                .with_cache_control("public, max-age=600")
        })
        .build()
        .unwrap();
    let res = other.handle(Request::from_http(http::Request::get("/data").body(Bytes::new()).unwrap())).await;
    assert_eq!(res.header("cache-control"), Some("public, max-age=600"));
}

/// A panic in application code answers 500 and is contained: the shared
/// state, the caches and the mutexes around them stay usable, and the next
/// request is served normally. (`catch_unwind` ignores lock poisoning, so
/// this pins that no invariant depends on it.)
#[tokio::test]
async fn a_panic_in_one_request_does_not_poison_the_next() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone)]
    struct Counter(Arc<AtomicUsize>);
    let counter = Counter(Arc::new(AtomicUsize::new(0)));

    fn counting(ctx: Ctx) -> BoxFuture<Result<Node>> {
        Box::pin(async move {
            let Extension(Counter(n)) = Extension::<Counter>::from_context(&ctx)?;
            Ok(p![format!("n={}", n.fetch_add(1, Ordering::SeqCst))].into_node())
        })
    }
    fn exploding(_ctx: Ctx) -> BoxFuture<Result<Node>> {
        Box::pin(async { panic!("a page panicked while holding nothing in particular") })
    }

    let routes = Routes {
        pages: vec![
            PageDef { rendering: Rendering::Dynamic, ..page("/count", PageBody::Rust(counting), None) },
            PageDef { rendering: Rendering::Dynamic, ..page("/boom", PageBody::Rust(exploding), None) },
        ],
        ..Default::default()
    };
    let app = App::new(routes)
        .config(Config { root: temp_root("panic"), ..Config::default() })
        .environment(Environment::Test)
        .state(counter.clone())
        .post("/panic-route", |_req: Request| async {
            panic!("a route panicked");
            #[allow(unreachable_code)]
            Response::text("unreachable")
        })
        .build()
        .unwrap();

    let get = |uri: &'static str| {
        let app = app.clone();
        async move { app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await }
    };
    let first = get("/count").await;
    let (status, body) = (first.status.as_u16(), first.into_text().await);
    assert!(body.contains("n=0"), "{status}: {body}");
    // A panicking page and a panicking programmatic route.
    let res = std::panic::AssertUnwindSafe(get("/boom")).catch_unwind().await;
    assert!(res.is_ok(), "the panic did not escape the request");
    assert_eq!(res.unwrap().status.as_u16(), 500);
    let post = http::Request::post("/panic-route").body(Bytes::new()).unwrap();
    let res = std::panic::AssertUnwindSafe(app.handle(Request::from_http(post))).catch_unwind().await;
    assert_eq!(res.map(|r| r.status.as_u16()).unwrap_or(0), 500);
    // Shared state, the page store and the cookie jar still work.
    assert!(get("/count").await.into_text().await.contains("n=1"));
    assert_eq!(counter.0.load(Ordering::SeqCst), 2);
    assert_eq!(get("/nope").await.status.as_u16(), 404);
}

/// A static page is cached with a placeholder where the CSP nonce goes, and
/// every request substitutes its own. The placeholder was a fixed literal
/// replaced anywhere in the document, so a page that rendered that literal
/// as ordinary text — from a CMS field, a slug, a file name — was handed the
/// request's live nonce.
#[tokio::test]
async fn page_text_can_never_pick_up_the_csp_nonce() {
    fn echoes_the_placeholder(ctx: Ctx) -> BoxFuture<Result<Node>> {
        Box::pin(async move {
            let text = ctx.params.get("slug").unwrap_or_default().replace('-', "");
            Ok(div![p![id("a"), "nr-nonce-placeholder-5f3a9c"], p![id("b"), text]].into_node())
        })
    }
    let routes = Routes {
        pages: vec![PageDef {
            rendering: Rendering::Static,
            ..page("/echo/[slug]", PageBody::Rust(echoes_the_placeholder), None)
        }],
        ..Default::default()
    };
    let mut config = Config { root: temp_root("nonce"), ..Config::default() };
    config.security.csp = Some("script-src 'nonce-{nonce}'".into());
    let app = App::new(routes).config(config).environment(Environment::Test).build().unwrap();

    // The second request is the interesting one: the first fills the cache.
    for uri in ["/echo/x", "/echo/nr-nonce-placeholder-5f3a9c"] {
        let res = app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await;
        let csp = res.headers["content-security-policy"].to_str().unwrap().to_owned();
        let nonce = csp.split("'nonce-").nth(1).and_then(|s| s.split('\'').next()).unwrap().to_owned();
        assert_eq!(nonce.len(), 32, "{csp}");
        let body = res.into_text().await;
        assert!(!body.contains(&format!("<p id=\"a\">{nonce}</p>")), "the page text read the nonce: {body}");
        assert!(!body.contains(&format!("<p id=\"b\">{nonce}</p>")), "a route parameter read the nonce: {body}");
    }
}

/// Round two: `data-module` can be derived from application data, so the
/// runtime the browser gets — readable in development, minified in production
/// — must refuse anything that is not a same-origin path instead of handing it
/// to `import()`, which would run another origin's code inside the page.
#[tokio::test]
async fn the_served_runtime_refuses_foreign_island_modules() {
    let dev = App::new(Routes { pages: vec![page("/", PageBody::Rust(home), None)], ..Default::default() })
        .config(Config { root: temp_root("runtime"), ..Config::default() })
        .environment(Environment::Development)
        .build()
        .unwrap();
    let prod = app(|_| {});
    for (name, served) in [("development", &dev), ("production", &prod)] {
        let (_, js) = send(served, "/_next-rust/runtime.js", &[]).await;
        for needle in ["island module refused", "location.origin", r"/^\/[^/\\]/", "Object.hasOwn"] {
            assert!(js.contains(needle), "the {name} runtime lost the island module guard `{needle}`");
        }
    }
}
