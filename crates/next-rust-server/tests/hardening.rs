//! Production hardening: what a release build is allowed to say about itself.
//!
//! Two questions: does an error page in `Environment::Production` ever carry
//! an internal detail (a source path, a panic message, a file name, the text
//! of an internal error), and are the development-only endpoints really off
//! outside development.

use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use http::Method;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// Strings that must never reach a client in production. `PROJECT_MARK` stands
/// in for the absolute source path a backtrace or a `Debug` impl would print.
const SECRET_DETAIL: &str = "connection string postgres://user:hunter2@db.internal/app";
const PANIC_MARK: &str = "kaboom-in-a-page";

fn root_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move { Ok(div![id("root"), main![children]].into_node()) })
}

fn root_not_found(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["Nothing here"].into_node()) })
}

fn ok_page(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1!["Fine"].into_node()) })
}

/// A loader that fails with an internal error carrying a secret.
fn failing_loader(_ctx: Ctx) -> R {
    Box::pin(async { Err(Error::msg(format!("failed to reach the database: {SECRET_DETAIL}"))) })
}

/// A loader whose error has a source chain, which `detailed_message` walks.
fn chained_loader(_ctx: Ctx) -> R {
    Box::pin(async {
        #[derive(Debug)]
        struct Outer(std::io::Error);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("loading users failed")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        Err(Error::from(Outer(std::io::Error::other(SECRET_DETAIL))))
    })
}

fn panicking_page(_ctx: Ctx) -> R {
    Box::pin(async { panic!("{PANIC_MARK}") })
}

/// Static rendering of a page that reads request data: the failure mode that
/// `next-rust build` reports, reached here through a request.
fn static_needs_cookies(ctx: Ctx) -> R {
    Box::pin(async move {
        let cookies = Cookies::from_context(&ctx)?;
        Ok(p![cookies.get("user").unwrap_or_default()].into_node())
    })
}

/// A statically rendered page whose loader simply fails.
fn static_fails(_ctx: Ctx) -> R {
    Box::pin(async { Err(Error::msg(format!("prerender failed: {SECRET_DETAIL}"))) })
}

fn api_boom(_req: Request) -> BoxFuture<Response> {
    Box::pin(async { Error::msg(format!("upstream refused: {SECRET_DETAIL}")).into_response() })
}

#[derive(serde::Deserialize)]
struct Filter {
    #[allow(dead_code)]
    page: u32,
}

fn query_page(ctx: Ctx) -> R {
    Box::pin(async move {
        let Query(filter) = Query::<Filter>::from_context(&ctx)?;
        Ok(p![filter.page.to_string()].into_node())
    })
}

/// An `error.rs` boundary that prints everything it is given, so a leak into
/// `ErrorInfo` shows up in the page rather than being swallowed.
fn loud_boundary(_ctx: Ctx, info: ErrorInfo) -> Node {
    section![class("err"), p![format!("status={} digest={} message={}", info.status, info.digest, info.message)]]
        .into_node()
}

fn page(pattern: &'static str, body: PageBody, segments: Vec<SegmentDef>) -> PageDef {
    PageDef {
        pattern,
        source: "app/secret-source-file.rs",
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

fn boundary_seg() -> SegmentDef {
    SegmentDef { error: Some(loud_boundary), ..Default::default() }
}

fn root_seg() -> SegmentDef {
    SegmentDef { layout: Some(root_layout), not_found: Some(root_not_found), ..Default::default() }
}

struct TestApp {
    app: App,
    dir: std::path::PathBuf,
}

impl TestApp {
    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// `App::build` stores the environment in a process-global flag (`is_dev`),
/// which `IntoResponse for Error` consults. Two apps of different
/// environments in one process therefore share it, so every test that builds
/// one takes this lock and keeps its app alive while it asserts.
static ENV_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

fn build(env: Environment) -> TestApp {
    build_with(env, |_| {})
}

fn build_with(env: Environment, tweak: impl FnOnce(&mut Config)) -> TestApp {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir =
        std::env::temp_dir().join(format!("nr-hardening-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("public")).unwrap();
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("public/robots.txt"), "User-agent: *").unwrap();
    std::fs::write(dir.join(".env"), format!("DATABASE_URL={SECRET_DETAIL}")).unwrap();
    std::fs::write(dir.join("next-rust.toml"), "[security]\nheaders = true\n").unwrap();
    std::fs::write(dir.join("app/page.rs"), format!("// {SECRET_DETAIL}")).unwrap();
    std::fs::write(dir.join("src/main.rs"), format!("// {SECRET_DETAIL}")).unwrap();

    let mut static_bad = page("/static-bad", PageBody::Rust(static_needs_cookies), vec![root_seg()]);
    static_bad.rendering = Rendering::Static;
    let mut static_broken = page("/static-broken", PageBody::Rust(static_fails), vec![root_seg()]);
    static_broken.rendering = Rendering::Static;

    let routes = Routes {
        pages: vec![
            page("/", PageBody::Rust(ok_page), vec![root_seg()]),
            page("/broken", PageBody::Rust(failing_loader), vec![root_seg(), SegmentDef::default()]),
            page("/chained", PageBody::Rust(chained_loader), vec![root_seg(), SegmentDef::default()]),
            page("/panics", PageBody::Rust(panicking_page), vec![root_seg(), SegmentDef::default()]),
            page("/search", PageBody::Rust(query_page), vec![root_seg(), SegmentDef::default()]),
            page("/caught", PageBody::Rust(failing_loader), vec![root_seg(), boundary_seg()]),
            page("/caught-query", PageBody::Rust(query_page), vec![root_seg(), boundary_seg()]),
            static_bad,
            static_broken,
        ],
        apis: vec![ApiDef {
            pattern: "/api/boom",
            source: "app/api/boom/route.rs",
            handlers: vec![(Method::GET, api_boom)],
            middleware: vec![],
        }],
        root: root_seg(),
        ..Default::default()
    };

    let mut config = Config { root: dir.clone(), ..Config::default() };
    tweak(&mut config);
    let app = App::new(routes)
        .config(config)
        .environment(env)
        .post("/api/json", |mut req: Request| async move {
            let body: serde_json::Value = req.json().await?;
            Ok::<_, Error>(Json(body))
        })
        .build()
        .unwrap();
    TestApp { app, dir }
}

async fn get(app: &App, uri: &str) -> (u16, String) {
    let req = http::Request::get(uri).body(Bytes::new()).unwrap();
    let res = app.handle(Request::from_http(req)).await;
    (res.status.as_u16(), res.into_text().await)
}

/// Everything a production answer must never contain.
fn assert_clean(what: &str, body: &str, dir: &std::path::Path) {
    let leaks = [
        (SECRET_DETAIL, "the internal error message"),
        (PANIC_MARK, "the panic message"),
        ("panicked", "the word `panicked`"),
        ("hunter2", "a credential from the internal message"),
        ("secret-source-file", "the route's source file"),
        ("app/api/boom/route.rs", "the API route's source file"),
        (env!("CARGO_MANIFEST_DIR"), "an absolute source path"),
        ("stack backtrace", "a backtrace"),
    ];
    for (needle, why) in leaks {
        assert!(!body.contains(needle), "{what}: leaked {why} ({needle:?})\n----\n{body}\n----");
    }
    assert!(!body.contains(&dir.display().to_string()), "{what}: leaked the project path\n----\n{body}\n----");
    // No `.rs` file name of any kind.
    for word in body.split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '_' || c == '/' || c == '-')) {
        assert!(!word.ends_with(".rs"), "{what}: leaked the file name {word:?}\n----\n{body}\n----");
    }
}

#[tokio::test]
async fn production_error_pages_carry_no_internal_detail() {
    let _env = ENV_LOCK.lock().await;
    let TestApp { app, dir } = build(Environment::Production);

    // A loader that fails with an internal error.
    let (status, body) = get(&app, "/broken").await;
    assert_eq!(status, 500);
    assert_clean("a failing loader", &body, &dir);
    assert!(body.contains("Something went wrong"), "the public message is missing:\n{body}");
    // The only identifier it offers is the digest, which is also in the log.
    assert!(body.contains("Error reference: "), "no error reference to correlate with the log:\n{body}");

    // The same with a source chain, which the detailed message walks.
    let (status, body) = get(&app, "/chained").await;
    assert_eq!(status, 500);
    assert_clean("a failing loader with a source chain", &body, &dir);

    // A 500 from an API route.
    let (status, body) = get(&app, "/api/boom").await;
    assert_eq!(status, 500);
    assert_clean("an API route", &body, &dir);

    // A statically rendered page that reads request data, and one whose
    // loader simply fails: both go through the static-failure path.
    for path in ["/static-bad", "/static-broken"] {
        let (status, body) = get(&app, path).await;
        assert_eq!(status, 500, "{path}");
        assert_clean(path, &body, &dir);
    }

    // A body that is not JSON.
    let req = http::Request::post("/api/json")
        .header("content-type", "application/json")
        .body(Bytes::from_static(b"{\"password\": "))
        .unwrap();
    let res = app.handle(Request::from_http(req)).await;
    let status = res.status.as_u16();
    let body = res.into_text().await;
    assert_eq!(status, 400);
    assert_clean("a bad JSON body", &body, &dir);
    // What it *does* say: serde's own message, verbatim, including the byte
    // position it stopped at. `Error::http` messages are public by design, so
    // this survives into production. Pinned here so a change is noticed.
    assert_eq!(body, "invalid JSON body: EOF while parsing a value at line 1 column 13", "{body}");

    // A query string that does not deserialise. Without an `error.rs`
    // boundary every unhandled error is a 500 (`global_error_page` raises the
    // status), so the status here is 500 rather than the 400 the extractor
    // chose; what matters for this review is the body.
    let (status, body) = get(&app, "/search?page=not-a-number").await;
    assert_eq!(status, 500, "{body}");
    assert_clean("a bad query string", &body, &dir);

    // The same failures reaching an `error.rs` boundary, which is handed
    // `ErrorInfo` and prints all of it.
    let (status, body) = get(&app, "/caught").await;
    assert_eq!(status, 500);
    assert_clean("an error boundary", &body, &dir);
    assert!(body.contains("message=Internal Server Error"), "the boundary saw an internal message:\n{body}");

    // An `Error::http` message reaches the boundary (and the page) verbatim,
    // in production too: serde's text, with the field name it choked on.
    let (status, body) = get(&app, "/caught-query?page=not-a-number").await;
    assert_eq!(status, 400, "a caught error keeps the status the extractor chose");
    assert_clean("a caught query error", &body, &dir);
    assert!(
        body.contains("invalid query string: invalid digit found in string"),
        "the query-string message changed shape:\n{body}"
    );
    TestApp::cleanup(&dir);
}

/// What the *development* build says, so the production assertions above are
/// known to be testing a difference rather than a page that never says
/// anything. Anything this test finds in development and the one above does
/// not find in production is a detail production withholds.
#[tokio::test]
async fn development_does_show_the_detail_production_withholds() {
    let _env = ENV_LOCK.lock().await;
    let TestApp { app, dir } = build(Environment::Development);
    let (status, body) = get(&app, "/broken").await;
    assert_eq!(status, 500);
    assert!(body.contains(SECRET_DETAIL), "development hid the detail, so the production test proves nothing:\n{body}");
    let (status, body) = get(&app, "/static-broken").await;
    assert_eq!(status, 500);
    assert!(body.contains(SECRET_DETAIL), "{body}");
    TestApp::cleanup(&dir);
}

/// A panic inside a page is caught by the server (not by `App::handle`), so
/// this one goes over a socket.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_page_answers_500_with_nothing_in_it() {
    let _env = ENV_LOCK.lock().await;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let TestApp { app, dir } = build(Environment::Production);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { app.serve_listener(listener).await });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"GET /panics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut got = Vec::new();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), stream.read_to_end(&mut got)).await;
    let answer = String::from_utf8_lossy(&got).into_owned();
    server.abort();

    assert!(answer.starts_with("HTTP/1.1 500"), "{answer}");
    assert_clean("a panicking page", &answer, &dir);
    // The security headers are still applied to the fallback answer.
    let lower = answer.to_ascii_lowercase();
    assert!(lower.contains("x-content-type-options: nosniff"), "{answer}");
    // The built-in server names neither itself nor the framework in a header.
    for banner in ["\r\nserver:", "\r\nx-powered-by:", "\r\nx-aspnet", "\r\nvia:"] {
        assert!(!lower.contains(banner), "the answer carries {banner:?}:\n{answer}");
    }
    TestApp::cleanup(&dir);
}

/// The error body of an API route (and of a server action) follows the
/// `Environment` of the app that answers, not the last app built in the
/// process. A process usually serves one app, but tests and embeddings build
/// several, and a production app must not become verbose because a
/// development app exists somewhere else.
#[tokio::test]
async fn api_error_bodies_follow_the_app_that_answers_not_the_process() {
    let _env = ENV_LOCK.lock().await;
    let production = build(Environment::Production);
    let (status, body) = get(&production.app, "/api/boom").await;
    assert_eq!(status, 500);
    assert!(!body.contains(SECRET_DETAIL), "a lone production app leaked: {body}");

    // A development app in the same process, built after it and answering
    // in between, changes nothing for the production one.
    let development = build(Environment::Development);
    let (status, dev_body) = get(&development.app, "/api/boom").await;
    assert_eq!(status, 500);
    assert!(dev_body.contains(SECRET_DETAIL), "development still explains itself: {dev_body}");

    let (status, body) = get(&production.app, "/api/boom").await;
    assert_eq!(status, 500);
    assert!(!body.contains(SECRET_DETAIL), "the production app followed the development app: {body}");

    // And the two answering at the same time keep their own environments.
    let (a, b) = tokio::join!(get(&production.app, "/api/boom"), get(&development.app, "/api/boom"));
    assert!(!a.1.contains(SECRET_DETAIL), "{}", a.1);
    assert!(b.1.contains(SECRET_DETAIL), "{}", b.1);
    TestApp::cleanup(&production.dir);
    TestApp::cleanup(&development.dir);
}

/// The framework signs every document with its exact version by default. This
/// is deliberate (`[build] signature`), but it is a version disclosure: the
/// test pins both halves so a change to either is noticed.
#[tokio::test]
async fn the_generator_meta_names_the_exact_framework_version_unless_turned_off() {
    let _env = ENV_LOCK.lock().await;
    let signed = build(Environment::Production);
    let (_, body) = get(&signed.app, "/").await;
    let stamp = format!("<meta name=\"generator\" content=\"Next Rust {}\">", env!("CARGO_PKG_VERSION"));
    assert!(body.contains(&stamp), "the signature changed shape:\n{body}");
    // Error pages carry it too, so an unauthenticated 404 or 500 tells a
    // scanner the exact version.
    let (status, body) = get(&signed.app, "/broken").await;
    assert_eq!(status, 500);
    assert!(body.contains(&stamp), "an error page dropped the signature");
    let (status, body) = get(&signed.app, "/no-such-page").await;
    assert_eq!(status, 404);
    assert!(body.contains(&stamp), "a 404 dropped the signature");
    TestApp::cleanup(&signed.dir);

    // `[build] signature = false` removes it everywhere.
    let quiet = build_with(Environment::Production, |config| config.build.signature = false);
    for path in ["/", "/broken", "/no-such-page"] {
        let (_, body) = get(&quiet.app, path).await;
        assert!(!body.contains("name=\"generator\""), "{path} still carries a generator meta:\n{body}");
        assert!(!body.contains(env!("CARGO_PKG_VERSION")), "{path} still names the version");
    }
    TestApp::cleanup(&quiet.dir);
}

/// The development endpoints must not answer in production, including the SSE
/// stream (which reads a status file from the build directory) and the ping.
#[tokio::test]
async fn development_endpoints_are_off_in_production() {
    let _env = ENV_LOCK.lock().await;
    for env in [Environment::Production, Environment::Test] {
        let TestApp { app, dir } = build(env);
        for path in [
            "/_next-rust/dev/events",
            "/_next-rust/dev/ping",
            "/_next-rust/dev/status.json",
            "/_next-rust/dev/",
            "/_next-rust/dev",
            "/_next-rust/dev/events?x=1",
            "/_next-rust/dev/../dev/events",
        ] {
            let Ok(req) = http::Request::get(path).body(Bytes::new()) else { continue };
            let res =
                tokio::time::timeout(std::time::Duration::from_secs(5), app.handle(Request::from_http(req))).await;
            let res = res.unwrap_or_else(|_| panic!("{env:?} {path}: the endpoint answered and did not finish"));
            let status = res.status.as_u16();
            let content_type = res.header("content-type").unwrap_or_default().to_owned();
            let body = res.into_text().await;
            assert!(status == 404 || (300..400).contains(&status), "{env:?} {path} answered {status}: {body}");
            assert!(!content_type.contains("event-stream"), "{env:?} {path} opened an SSE stream");
            assert_clean(path, &body, &dir);
        }
        TestApp::cleanup(&dir);
    }
    // In development they do answer, so the assertions above mean something.
    let TestApp { app, dir } = build(Environment::Development);
    let req = http::Request::get("/_next-rust/dev/ping").body(Bytes::new()).unwrap();
    let res = app.handle(Request::from_http(req)).await;
    assert_eq!(res.status.as_u16(), 200, "the dev endpoints are gone entirely, not just gated");
    TestApp::cleanup(&dir);
}

/// `public/` may itself be a symbolic link (a deployment that mounts assets
/// elsewhere). Files inside the link's target are still served, and nothing
/// outside it is, however the request spells the path.
#[cfg(unix)]
#[tokio::test]
async fn a_symlinked_public_directory_stays_confined_to_its_target() {
    static N: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "nr-hardening-link-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&base);
    let real = base.join("elsewhere/assets");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::create_dir_all(base.join("site")).unwrap();
    std::fs::write(real.join("ok.txt"), "public file").unwrap();
    std::fs::write(real.join(".env"), SECRET_DETAIL).unwrap();
    std::fs::write(base.join("elsewhere/private.txt"), SECRET_DETAIL).unwrap();
    std::fs::write(base.join("outside.txt"), SECRET_DETAIL).unwrap();
    // `site/public` is a link to `elsewhere/assets`.
    std::os::unix::fs::symlink(&real, base.join("site/public")).unwrap();

    let config = Config { root: base.join("site"), ..Config::default() };
    let app = App::new(Routes { root: root_seg(), ..Default::default() })
        .config(config)
        .environment(Environment::Production)
        .build()
        .unwrap();

    let (status, body) = get(&app, "/ok.txt").await;
    assert_eq!((status, body.as_str()), (200, "public file"), "a linked public directory is not served");

    for path in [
        "/.env",
        "/%2Eenv",
        "/../private.txt",
        "/..%2Fprivate.txt",
        "/%2e%2e/private.txt",
        "/../../outside.txt",
        "/..%5Cprivate.txt",
        "/./../private.txt",
    ] {
        let Ok(req) = http::Request::get(path).body(Bytes::new()) else { continue };
        let res = app.handle(Request::from_http(req)).await;
        let status = res.status.as_u16();
        let body = res.into_text().await;
        assert!(!body.contains(SECRET_DETAIL), "{path} escaped the link's target ({status})");
        assert!(status < 500, "{path} answered {status}");
    }
    let _ = std::fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// Default security headers
// ---------------------------------------------------------------------------

/// What `[security] headers = true` actually puts on a response, what it does
/// not, and who can overrule it.
#[tokio::test]
async fn the_default_security_headers_are_what_the_code_claims() {
    let _env = ENV_LOCK.lock().await;
    let TestApp { app, dir } = build_with(Environment::Production, |config| {
        config.security.csp = Some("strict".to_owned());
    });
    let req = http::Request::get("/").body(Bytes::new()).unwrap();
    let res = app.handle(Request::from_http(req)).await;
    let header = |name: &str| res.header(name).map(str::to_owned);

    assert_eq!(header("x-content-type-options").as_deref(), Some("nosniff"));
    assert_eq!(header("x-frame-options").as_deref(), Some("SAMEORIGIN"));
    assert_eq!(header("referrer-policy").as_deref(), Some("strict-origin-when-cross-origin"));
    assert_eq!(header("cross-origin-opener-policy").as_deref(), Some("same-origin"));
    assert_eq!(header("permissions-policy").as_deref(), Some("camera=(), microphone=(), geolocation=()"));

    // HSTS is off unless `[security] hsts_max_age` is set.
    assert_eq!(header("strict-transport-security"), None, "HSTS is on by default");

    // Cross-origin isolation headers are NOT sent.
    assert_eq!(header("cross-origin-resource-policy"), None);
    assert_eq!(header("cross-origin-embedder-policy"), None);

    // The strict CSP: a nonce and `strict-dynamic` for scripts, no
    // `unsafe-inline` there; `style-src` does allow inline styles.
    let csp = header("content-security-policy").expect("a CSP");
    let script_src = csp.split("; ").find(|part| part.starts_with("script-src")).expect("script-src");
    assert!(script_src.contains("'strict-dynamic'"), "{script_src}");
    assert!(!script_src.contains("unsafe-inline"), "{script_src}");
    assert!(!script_src.contains("unsafe-eval"), "{script_src}");
    assert!(csp.contains("object-src 'none'"), "{csp}");
    assert!(csp.contains("base-uri 'self'"), "{csp}");
    assert!(csp.contains("form-action 'self'"), "{csp}");
    let style_src = csp.split("; ").find(|part| part.starts_with("style-src")).expect("style-src");
    assert!(style_src.contains("'unsafe-inline'"), "style-src no longer allows inline styles: {style_src}");

    // `frame-ancestors 'self'` and `X-Frame-Options: SAMEORIGIN` agree.
    assert!(csp.contains("frame-ancestors 'self'"), "{csp}");

    // `{nonce}` is substituted, and the value is the one the document uses.
    let nonce =
        csp.split("'nonce-").nth(1).and_then(|rest| rest.split('\'').next()).expect("a nonce in the CSP").to_owned();
    assert!(!nonce.is_empty() && nonce != "{nonce}", "the placeholder survived: {csp}");
    assert!(nonce.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'), "{nonce}");
    let body = res.into_text().await;
    assert!(!body.contains("{nonce}"), "the placeholder reached the document");
    for tag in body.match_indices("<script").map(|(i, _)| &body[i..body[i..].find('>').map_or(body.len(), |e| i + e)]) {
        assert!(tag.contains(&format!("nonce=\"{nonce}\"")), "a script without the request's nonce: {tag}");
    }

    // Two requests get two nonces.
    let req = http::Request::get("/").body(Bytes::new()).unwrap();
    let second = app.handle(Request::from_http(req)).await;
    let other = second.header("content-security-policy").unwrap_or_default().to_owned();
    assert!(!other.contains(&nonce), "the nonce is reused between requests");
    TestApp::cleanup(&dir);
}

/// An application `[[headers]]` rule is applied before the defaults, and the
/// defaults only fill in what is missing: a rule therefore *replaces* a
/// security header. Deliberate (a site may need `X-Frame-Options: DENY` or a
/// CSP of its own), and worth knowing.
#[tokio::test]
async fn an_application_header_rule_overrides_a_security_header() {
    let _env = ENV_LOCK.lock().await;
    let TestApp { app, dir } = build_with(Environment::Production, |config| {
        config.security.csp = Some("strict".to_owned());
        config.headers = vec![next_rust_core::config::HeaderRule {
            source: "/".to_owned(),
            headers: [
                ("x-frame-options".to_owned(), "ALLOWALL".to_owned()),
                ("content-security-policy".to_owned(), "default-src *".to_owned()),
                ("referrer-policy".to_owned(), "unsafe-url".to_owned()),
            ]
            .into_iter()
            .collect(),
        }];
    });
    let req = http::Request::get("/").body(Bytes::new()).unwrap();
    let res = app.handle(Request::from_http(req)).await;
    assert_eq!(res.header("x-frame-options"), Some("ALLOWALL"), "a rule could not replace the default");
    assert_eq!(res.header("content-security-policy"), Some("default-src *"));
    assert_eq!(res.header("referrer-policy"), Some("unsafe-url"));
    // What the rule does not name keeps its default.
    assert_eq!(res.header("x-content-type-options"), Some("nosniff"));
    TestApp::cleanup(&dir);
}

/// Middleware runs inside the pipeline, so `set_header` there is overwritten
/// by the defaults rather than the other way round — unless it sets a header
/// the defaults also set, which it then keeps.
#[tokio::test]
async fn middleware_can_also_set_a_security_header() {
    let _env = ENV_LOCK.lock().await;
    static N: AtomicUsize = AtomicUsize::new(9000);
    let dir = std::env::temp_dir().join(format!(
        "nr-hardening-mw-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("public")).unwrap();
    let config = Config { root: dir.clone(), ..Config::default() };
    let app = App::new(Routes {
        pages: vec![page("/", PageBody::Rust(ok_page), vec![root_seg()])],
        root: root_seg(),
        ..Default::default()
    })
    .config(config)
    .environment(Environment::Production)
    .middleware(|req: Request, next: Next| async move {
        next.run(req).await.with_header("x-frame-options", "DENY").with_header("referrer-policy", "no-referrer")
    })
    .build()
    .unwrap();
    let req = http::Request::get("/").body(Bytes::new()).unwrap();
    let res = app.handle(Request::from_http(req)).await;
    assert_eq!(res.header("x-frame-options"), Some("DENY"), "middleware could not tighten the default");
    assert_eq!(res.header("referrer-policy"), Some("no-referrer"));
    assert_eq!(res.header("x-content-type-options"), Some("nosniff"));
    TestApp::cleanup(&dir);
}
