//! Adversarial tests of the parsers a request reaches after routing: the
//! query string, form bodies, the typed extractors pages declare, and the
//! one place where a *path* becomes a `Location` header (configured
//! `[[redirects]]` rules).
//!
//! `tests/fuzz.rs` covers the pipeline in front of these — static paths,
//! cookies, dates, body limits and the socket. Seeds are fixed, so a failure
//! is reproducible, and every test carries a wall-clock bound.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use bytes::Bytes;
use next_rust_core::config::RedirectRule;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;
use serde::Deserialize;

/// xorshift64*: small, seeded, the same sequence on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[self.below(from.len())]
    }
}

const QUERY_PIECES: &[&str] = &[
    "a",
    "b",
    "id",
    "next",
    "=",
    "&",
    ";",
    "+",
    "%20",
    "%00",
    "%ff",
    "%fe%ff",
    "%c0%ae",
    "%",
    "%2",
    "%zz",
    "%25",
    "a[b]",
    "[",
    "]",
    "[0]",
    ".",
    "..",
    "-",
    "_",
    "self",
    "crate",
    "fn",
    "0",
    "1",
    "-1",
    "99999999999999999999",
    "1e400",
    "NaN",
    "true",
    "on",
    "one",
    "héllo",
    "\u{2028}",
    "#",
    "?",
    "/",
    "\\",
];

/// A page that reports what the extractors gave it, so the same generated
/// request can be pushed through `Query<T>` and `Path<T>` as well.
fn probe(ctx: Ctx) -> BoxFuture<Result<Node>> {
    Box::pin(async move {
        #[derive(Deserialize, Debug)]
        struct Rest {
            rest: Vec<String>,
        }
        let Query(q): Query<QueryMap> = ctx.extract()?;
        let typed = ctx.extract::<Query<BTreeMap<String, String>>>().map(|Query(m)| m.len());
        let path = ctx.extract::<Path<Rest>>().map(|Path(p)| p.rest.len());
        Ok(p![format!("q={} typed={typed:?} path={path:?}", q.0.len())].into_node())
    })
}

fn app() -> App {
    let routes = Routes {
        pages: vec![PageDef {
            pattern: "/q/[...rest]",
            source: "test",
            body: PageBody::Rust(probe),
            segments: vec![SegmentDef::default()],
            metadata: None,
            rendering: Rendering::Dynamic,
            revalidate: None,
            generate_params: None,
            dynamic_params: true,
            tags: &[],
            intercept_from: None,
        }],
        ..Default::default()
    };
    let mut config = Config { root: std::env::temp_dir().join("nr-parsers"), ..Config::default() };
    // Rules an owner might plausibly write, including the one that used to
    // hand the visitor's own path straight to `Location`.
    config.redirects = [
        ("/go/:to", "/:to", false),
        ("/out/:to*", "/:to*", true),
        ("/moved/:slug", "/new/:slug", false),
        ("/away/:to", "https://cdn.example/:to", false),
    ]
    .into_iter()
    .map(|(source, destination, permanent)| RedirectRule {
        source: source.into(),
        destination: destination.into(),
        permanent,
    })
    .collect();
    App::new(routes)
        .config(config)
        .environment(Environment::Test)
        .get("/qp", |req: Request| async move {
            let one = req.query_param("a");
            let all = req.query::<QueryMap>().map(|q| q.0.len());
            format!("{one:?} {all:?}")
        })
        .post("/form", |mut req: Request| async move {
            match req.form::<HashMap<String, String>>().await {
                Ok(f) => format!("ok {}", f.len()),
                Err(e) => format!("refused {}", e.status()),
            }
        })
        .build()
        .unwrap()
}

async fn get(app: &App, target: &str) -> (u16, String) {
    let Ok(req) = http::Request::get(target).body(Bytes::new()) else { return (0, String::new()) };
    let res = app.handle(Request::from_http(req)).await;
    let status = res.status.as_u16();
    (status, res.into_text().await)
}

/// The one that mattered: a `[[redirects]]` rule is the only configuration
/// in which a piece of the request *path* is written into a `Location`
/// header. Red before the fix — `/go/\evil.example` produced
/// `Location: /\evil.example`, which Chrome, Firefox and Safari all resolve
/// as `//evil.example`.
#[tokio::test]
async fn a_configured_redirect_rule_cannot_send_a_visitor_to_another_origin() {
    let app = app();
    let location = |target: &'static str| {
        let app = app.clone();
        async move {
            let req = http::Request::get(target).body(Bytes::new()).expect("valid target");
            app.handle(Request::from_http(req)).await.header("location").map(str::to_owned)
        }
    };
    assert_eq!(location("/go/\\evil.example").await.as_deref(), Some("/%5Cevil.example"));
    assert_eq!(location("/out/\\/evil.example").await.as_deref(), Some("/%5C/evil.example"));
    assert_eq!(location("/go/%5Cevil.example").await.as_deref(), Some("/%5Cevil.example"));
    // The rules that are supposed to work still do, query string included.
    assert_eq!(location("/moved/hello?x=1").await.as_deref(), Some("/new/hello?x=1"));
    assert_eq!(location("/away/logo.png").await.as_deref(), Some("https://cdn.example/logo.png"));
    assert_eq!(location("/out/a/b/c").await.as_deref(), Some("/a/b/c"));
}

#[tokio::test]
async fn no_generated_request_target_produces_a_location_off_this_site() {
    let app = app();
    let mut rng = Rng(0x10C1_0BAD_5EED);
    let mut redirected = 0usize;
    let start = Instant::now();
    for _ in 0..8_000 {
        let mut target = String::from("/");
        target.push_str(rng.pick(&["go", "out", "moved", "away", "q", "x"]));
        for _ in 0..rng.below(4) {
            target.push('/');
            target.push_str(rng.pick(QUERY_PIECES));
        }
        if rng.next().is_multiple_of(3) {
            target.push('?');
            for _ in 0..rng.below(5) {
                target.push_str(rng.pick(QUERY_PIECES));
            }
        }
        let Ok(req) = http::Request::get(&target).body(Bytes::new()) else { continue };
        let res = app.handle(Request::from_http(req)).await;
        assert!(res.status.as_u16() < 500, "{target:?} -> {}", res.status);
        if let Some(location) = res.header("location") {
            redirected += 1;
            // The invariant that matters for a `Location`: either it is the
            // absolute URL the rule configured, or it stays on this origin —
            // which means one leading `/`, and never `//` or `/\\` (browsers
            // read a backslash in a path as a slash, so `/\\host` is `//host`).
            let off_site = location.starts_with("https://cdn.example/");
            let local = location.starts_with('/')
                && !location.starts_with("//")
                && !location.starts_with("/\\")
                && !location.contains(['\r', '\n']);
            assert!(off_site || local, "{target:?} -> {location:?}");
        }
    }
    assert!(redirected > 200, "the generator triggers redirect rules ({redirected})");
    assert!(start.elapsed() < Duration::from_secs(60), "took {:?}", start.elapsed());
}

/// Nesting, repetition and bad escapes in a query string: an answer or a
/// 400, in time proportional to the input.
#[tokio::test]
async fn generated_query_strings_are_parsed_or_refused_in_bounded_time() {
    let app = app();
    let mut rng = Rng(0x0DDB_A11A_0001);
    let mut parsed = 0usize;
    let start = Instant::now();
    for _ in 0..6_000 {
        let mut query = String::new();
        for _ in 0..rng.below(12) {
            query.push_str(rng.pick(QUERY_PIECES));
        }
        let target = format!("/qp?{query}");
        let Ok(req) = http::Request::get(&target).body(Bytes::new()) else { continue };
        let res = app.handle(Request::from_http(req)).await;
        assert_eq!(res.status.as_u16(), 200, "{query:?}");
        parsed += usize::from(!res.into_text().await.contains("Err"));
    }
    assert!(parsed > 500, "the generator produces parseable query strings ({parsed})");
    assert!(start.elapsed() < Duration::from_secs(60), "took {:?}", start.elapsed());
}

/// What the query parser does with the shapes people assume it understands.
#[tokio::test]
async fn nesting_repetition_and_separators_are_flat_text() {
    let one = |q: &str| {
        let uri = format!("/x?{q}");
        Request::get(&uri).query::<QueryMap>().map(|m| m.0)
    };
    // `a[b][c]` is one key called `a[b][c]`: there is no nested decoding, so
    // there is no depth for a generator to explode.
    assert_eq!(one("a[b][c][d]=1").unwrap(), [("a[b][c][d]".to_owned(), "1".to_owned())]);
    let deep = format!("a{}=1", "[x]".repeat(15_000));
    let start = Instant::now();
    let parsed = Request::get(&format!("/x?{deep}")).query::<QueryMap>().unwrap().0;
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].0.len(), 1 + 3 * 15_000);
    assert!(start.elapsed() < Duration::from_secs(5), "took {:?}", start.elapsed());
    // A repeated key keeps every value, in order; `query_param` takes the first.
    let many = (0..5_000).map(|i| format!("a={i}")).collect::<Vec<_>>().join("&");
    let req = Request::get(&format!("/x?{many}"));
    let values = req.query::<QueryMap>().unwrap().0;
    assert_eq!(values.len(), 5_000);
    assert_eq!(req.query_param("a").as_deref(), Some("0"));
    // `;` is not a separator: it is text inside a value.
    assert_eq!(one("a=1;b=2").unwrap(), [("a".to_owned(), "1;b=2".to_owned())]);
    // `+` is a space, `%20` is a space, `%2B` is a plus.
    assert_eq!(one("a=x+y").unwrap()[0].1, "x y");
    assert_eq!(one("a=x%20y").unwrap()[0].1, "x y");
    assert_eq!(one("a=x%2By").unwrap()[0].1, "x+y");
    // A key with no `=` is a key with an empty value; a bare `&` is nothing.
    assert_eq!(one("a").unwrap(), [("a".to_owned(), String::new())]);
    assert_eq!(one("&&&").unwrap(), []);
    assert_eq!(one("").unwrap(), []);
    // `%00` is kept as data — it is the *path* that refuses NUL, not a value.
    assert_eq!(one("a=%00").unwrap()[0].1, "\u{0}");
    // An escape that is not an escape is kept as the text it is.
    for kept in ["a=%", "a=%2", "a=%zz"] {
        assert!(one(kept).is_ok(), "{kept} was refused");
    }
    assert_eq!(one("a=%").unwrap()[0].1, "%");
    assert_eq!(one("a=%zz").unwrap()[0].1, "%zz");
    // Worth stating: bytes that are not UTF-8 are *replaced*, not refused —
    // `form_urlencoded` decodes lossily, as browsers do. So a query value can
    // arrive as U+FFFD and two different requests can produce the same value.
    // (The *path* is stricter: `decode_segment` refuses these outright, which
    // `next-rust-router/tests/matching.rs` pins down.)
    assert_eq!(one("a=%ff").unwrap()[0].1, "\u{fffd}");
    assert_eq!(one("a=%c0%ae").unwrap()[0].1, "\u{fffd}\u{fffd}");
    assert_eq!(one("a=%ed%a0%80").unwrap()[0].1, "\u{fffd}\u{fffd}\u{fffd}");
    assert_eq!(one("a=%ff").unwrap()[0].1, one("a=%fe").unwrap()[0].1, "different bytes, one value");
    // A single huge value costs its length, once. (An `http::Uri` holds at
    // most 64 KiB, so this is as large as a request line can make it.)
    let big = "v".repeat(60_000);
    let start = Instant::now();
    assert_eq!(Request::get(&format!("/x?a={big}")).query_param("a").as_deref(), Some(big.as_str()));
    assert!(start.elapsed() < Duration::from_secs(5), "took {:?}", start.elapsed());
}

#[tokio::test]
async fn form_bodies_are_answered_or_refused_but_never_fatal() {
    let app = app();
    let mut rng = Rng(0x000F_0A11_0BAD_0002);
    let post = |body: Vec<u8>| {
        let app = app.clone();
        async move {
            let req = http::Request::post("/form")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Bytes::from(body))
                .unwrap();
            let res = app.handle(Request::from_http(req)).await;
            (res.status.as_u16(), res.into_text().await)
        }
    };
    let start = Instant::now();
    let mut accepted = 0usize;
    for _ in 0..3_000 {
        let mut body = String::new();
        for _ in 0..rng.below(14) {
            body.push_str(rng.pick(QUERY_PIECES));
        }
        let (status, text) = post(body.clone().into_bytes()).await;
        assert_eq!(status, 200, "{body:?} -> {text}");
        accepted += usize::from(text.starts_with("ok"));
    }
    assert!(accepted > 300, "the generator produces parseable forms ({accepted})");
    // Raw bytes that are not UTF-8 are replaced, not refused (see the query
    // test above for what that means).
    let (status, text) = post(vec![b'a', b'=', 0xff, 0xfe]).await;
    assert_eq!((status, text.as_str()), (200, "ok 1"));
    // A body that is not a form at all is refused with a 400, not a panic.
    let (status, text) = post(vec![0xff; 64]).await;
    assert_eq!(status, 200);
    assert!(text.starts_with("ok") || text.starts_with("refused"), "{text}");
    // Very many keys and one very large value stay linear.
    let many: String = (0..50_000).map(|i| format!("k{i}=v&")).collect();
    let (status, text) = post(many.into_bytes()).await;
    assert_eq!((status, text.as_str()), (200, "ok 50000"));
    let big = format!("a={}", "v".repeat(1_000_000));
    let (status, text) = post(big.into_bytes()).await;
    assert_eq!((status, text.as_str()), (200, "ok 1"));
    assert!(start.elapsed() < Duration::from_secs(60), "took {:?}", start.elapsed());
}

/// The typed extractors a page declares, driven by the same generated input.
#[tokio::test]
async fn typed_extractors_answer_every_generated_request() {
    let app = app();
    let mut rng = Rng(0xE217_AC70_0003);
    let start = Instant::now();
    for _ in 0..4_000 {
        let mut target = String::from("/q");
        for _ in 0..1 + rng.below(4) {
            target.push('/');
            target.push_str(rng.pick(QUERY_PIECES));
        }
        target.push('?');
        for _ in 0..rng.below(8) {
            target.push_str(rng.pick(QUERY_PIECES));
        }
        let (status, body) = get(&app, &target).await;
        if status == 0 {
            continue; // not a request target a client can send
        }
        assert!(matches!(status, 200 | 307 | 308 | 400 | 404), "{target:?} -> {status}: {body}");
        if status == 200 {
            assert!(body.contains("q=") && body.contains("path="), "{target:?} -> {body}");
        }
    }
    assert!(start.elapsed() < Duration::from_secs(60), "took {:?}", start.elapsed());
    // `Path<T>` is the catch-all, so it is a sequence however deep the URL is.
    let (status, body) = get(&app, "/q/a/b/c").await;
    assert_eq!(status, 200);
    assert!(body.contains("path=Ok(3)"), "{body}");
    // A page that asks for `Query` outside a request is a usage error, not a
    // panic: the static render path reports it.
    let ctx =
        RequestContext::for_static("/q/a", next_rust_router::Params::new(), std::sync::Arc::new(Config::default()));
    assert!(ctx.extract::<Query<QueryMap>>().is_err());
}
