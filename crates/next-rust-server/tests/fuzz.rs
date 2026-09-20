//! Adversarial tests of the request pipeline: generated hostile input for
//! the parsers a request reaches before any application code, and socket
//! level checks of the limits the server relies on. Seeds are fixed, so a
//! failure is reproducible, and the whole file runs in a few seconds.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

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

    fn mix(&mut self, pieces: &[&str], max: usize) -> String {
        (0..self.below(max)).map(|_| self.pick(pieces)).collect()
    }
}

const SECRET: &str = "TOP-SECRET-CONTENT";

fn echo(ctx: Ctx) -> BoxFuture<Result<Node>> {
    Box::pin(async move { Ok(p![ctx.params.get("slug").unwrap_or("-").to_owned()].into_node()) })
}

/// `<tmp>/site/public/{ok.txt,.env,sub/file.txt,.git/config}` with
/// `<tmp>/secret.txt` next to the project and (on Unix) a link to it.
fn project(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("nr-fuzz-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let public = base.join("site/public");
    std::fs::create_dir_all(public.join("sub")).unwrap();
    std::fs::create_dir_all(public.join(".git")).unwrap();
    std::fs::create_dir_all(base.join("site/app")).unwrap();
    std::fs::create_dir_all(base.join("site/src")).unwrap();
    std::fs::write(public.join("ok.txt"), "public file").unwrap();
    std::fs::write(public.join("sub/file.txt"), "public file").unwrap();
    for hidden in [public.join(".env"), public.join(".git/config"), public.join("sub/.htpasswd")] {
        std::fs::write(hidden, SECRET).unwrap();
    }
    for private in ["secret.txt", "site/.env", "site/next-rust.toml", "site/app/page.rs", "site/src/main.rs"] {
        std::fs::write(base.join(private), SECRET).unwrap();
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(base.join("secret.txt"), public.join("link.txt")).unwrap();
        std::os::unix::fs::symlink(&base, public.join("up")).unwrap();
        std::os::unix::fs::symlink(public.join(".env"), public.join("env.txt")).unwrap();
    }
    base
}

fn app(root: PathBuf, header_timeout: u64) -> App {
    let routes = Routes {
        pages: vec![PageDef {
            pattern: "/p/[slug]",
            source: "test",
            body: PageBody::Rust(echo),
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
    let mut config = Config { root, ..Config::default() };
    config.server.header_timeout = header_timeout;
    App::new(routes)
        .config(config)
        .environment(Environment::Test)
        .post("/echo", |mut req: Request| async move {
            let form = req.form::<HashMap<String, String>>().await.map(|f| f.len());
            format!("{form:?}")
        })
        .post("/json", |mut req: Request| async move {
            match req.json::<serde_json::Value>().await {
                Ok(_) => "ok".to_owned(),
                Err(e) => format!("refused: {}", e.status()),
            }
        })
        .build()
        .unwrap()
}

const PATH_PIECES: &[&str] = &[
    "/",
    "/",
    "/",
    "..",
    ".",
    "%2e",
    "%2E",
    "%2e%2e",
    "..%2f",
    "%2f",
    "%2F",
    "%5c",
    "\\",
    "%00",
    "%c0%ae",
    "%c0%af",
    "%e0%80%ae",
    "%ef%bc%8e",
    "\u{ff0e}",
    "\u{2024}",
    "\u{2215}",
    "%252e",
    "%25",
    "%",
    "%+f",
    "%zz",
    "ok.txt",
    "sub",
    "file.txt",
    ".env",
    "%2eenv",
    ".git",
    "config",
    "secret.txt",
    "link.txt",
    "up",
    "env.txt",
    "site",
    "public",
    "next-rust.toml",
    "app",
    "page.rs",
    "src",
    "main.rs",
    "CON",
    "NUL",
    "aux.txt",
    "COM1",
    "C:",
    "c%3a",
    "::$DATA",
    "ok.txt.",
    "ok.txt%20",
    "OK.TXT",
    "~",
    ";",
    "?",
    "#",
    "_next-rust",
    "client",
    "assets",
    "image",
    "m",
];

#[tokio::test]
async fn no_generated_path_reads_outside_public_or_a_hidden_file() {
    let base = project("paths");
    let public = std::fs::canonicalize(base.join("site/public")).unwrap();
    let app = app(base.join("site"), 10);
    let mut rng = Rng(0x5EED_1001);
    let mut served = 0;
    for round in 0..6000 {
        let mut path = format!("/{}", rng.mix(PATH_PIECES, 9));
        if round % 5 == 0 {
            path = format!("/_next-rust/{}{path}", rng.pick(&["client", "assets", "m", "image?url="]));
        }
        // The resolver itself, for any string at all.
        if let Some((file, _)) = static_files::resolve_safe(&public, &path).await {
            assert!(file.starts_with(&public), "{path:?} escaped to {file:?}");
            let inside = file.strip_prefix(&public).unwrap();
            assert!(
                inside.components().all(|c| !c.as_os_str().to_string_lossy().starts_with('.')),
                "{path:?} reached the hidden file {file:?}"
            );
        }
        // The whole pipeline, for what a client can put on the request line.
        let Ok(uri) = path.parse::<http::Uri>() else { continue };
        let Ok(req) = http::Request::get(uri).body(Bytes::new()) else { continue };
        let res = app.handle(Request::from_http(req)).await;
        let status = res.status.as_u16();
        let body = res.into_text().await;
        assert!(!body.contains(SECRET), "{path:?} leaked a private file ({status})");
        assert!(status < 500, "{path:?} answered {status}: {body}");
        served += usize::from(body == "public file");
    }
    assert!(served > 0, "the generator reaches real files too");
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn cookies_survive_hostile_headers_and_never_serialise_a_second_attribute() {
    let mut rng = Rng(0x5EED_1002);
    let pieces = &[
        ";",
        "=",
        "\"",
        " ",
        ",",
        "\t",
        "a",
        "session",
        "__Host-",
        "__Secure-",
        "%",
        "%3B",
        "%0d%0a",
        "\\",
        "é",
        "\u{0}",
        "\r\n",
        "Set-Cookie:",
        "Path=/",
        "Domain=evil.example",
        "HttpOnly",
        "==",
        ";;",
        "x=y",
    ];
    for _ in 0..5000 {
        let raw = rng.mix(pieces, 14);
        let mut headers = http::HeaderMap::new();
        if let Ok(value) = http::HeaderValue::from_bytes(raw.as_bytes()) {
            headers.append(http::header::COOKIE, value);
        }
        let jar = Cookies::from_headers(&headers, true);
        for (name, value) in jar.all() {
            assert!(!name.contains(';') && !value.contains(';'), "{raw:?}");
            let _ = (jar.get(&name), jar.get_decoded(&name));
        }

        let (name, value) = (rng.mix(pieces, 4), rng.mix(pieces, 6));
        if let Ok(header) = Cookie::new(name.clone(), value.clone()).path("/").to_header_value() {
            assert!(!header.contains(['\r', '\n', '\0']), "{header:?}");
            assert_eq!(header.matches(';').count(), 1, "only `; Path=/` follows the pair: {header:?}");
            assert!(header.starts_with(&format!("{name}={value}; ")), "{header:?}");
        }
        // Any text survives `Cookie::encoded`, and cannot add an attribute.
        let header = Cookie::encoded("v", &value).to_header_value().expect("encoded values are always valid");
        assert!(!header.contains([';', '\r', '\n', ' ', ',']), "{header:?}");
        let mut back = http::HeaderMap::new();
        back.insert(http::header::COOKIE, header.parse().unwrap());
        assert_eq!(Cookies::from_headers(&back, false).get_decoded("v").as_deref(), Some(value.as_str()));
    }
}

#[test]
fn http_dates_never_panic_and_round_trip() {
    let mut rng = Rng(0x5EED_1003);
    let pieces = &[
        "Sun,",
        "Mon,",
        " ",
        "  ",
        "06",
        "31",
        "00",
        "99",
        "Nov",
        "Feb",
        "Xyz",
        "1994",
        "1969",
        "9999",
        "10000",
        "99999999999999999999",
        "-1",
        "08:49:37",
        "24:00:00",
        "23:59:60",
        "99:99:99",
        "::",
        ":",
        "GMT",
        "UTC",
        "+0000",
        "18446744073709551615:0:0",
        "\u{0}",
        "é",
        "٣",
    ];
    for _ in 0..20_000 {
        let _ = http_date::parse(&rng.mix(pieces, 9));
    }
    for _ in 0..5000 {
        // Up to the year 9999.
        let secs = rng.next() % 253_402_300_800;
        let time = UNIX_EPOCH + Duration::from_secs(secs);
        assert_eq!(http_date::parse(&http_date::format(time)), Some(time), "{secs}");
    }
    // Times a clock can report but a date cannot hold.
    for secs in [u64::MAX / 2, u64::from(u32::MAX) * 1000, 253_402_300_800] {
        if let Some(time) = UNIX_EPOCH.checked_add(Duration::from_secs(secs)) {
            let _ = http_date::format(time);
        }
    }
    let _ = http_date::format(SystemTime::UNIX_EPOCH - Duration::from_secs(1));
}

#[test]
fn accepted_redirect_targets_stay_on_this_origin() {
    let mut rng = Rng(0x5EED_1004);
    let pieces = &[
        "/",
        "/",
        "\\",
        "\t",
        "\n",
        "\r",
        " ",
        "evil.example",
        "@",
        ":",
        "//",
        "/\\",
        "%09",
        "%2f",
        "%5c",
        "http:",
        "https://",
        "javascript:",
        "data:",
        ".",
        "..",
        "?next=",
        "#",
        "\u{0}",
        "\u{85}",
        "\u{2028}",
        "\u{3000}",
        "a",
    ];
    for _ in 0..20_000 {
        let target = rng.mix(pieces, 8);
        if !is_local_path(&target) {
            continue;
        }
        // What a browser makes of it: tabs and newlines go, `\` reads as `/`.
        let seen: String = target.chars().filter(|c| !matches!(c, '\t' | '\n' | '\r')).collect::<String>();
        let seen = seen.trim_matches(|c: char| c <= ' ').replace('\\', "/");
        assert!(seen.starts_with('/') && !seen.starts_with("//"), "{target:?} leaves the origin as {seen:?}");
        // And the header it becomes is one line.
        let res = Response::redirect(&target);
        if let Some(location) = res.header("location") {
            assert!(!location.contains(['\r', '\n']) && location.starts_with('/'), "{location:?}");
        }
    }
}

async fn post(app: &App, uri: &str, content_type: &str, body: Vec<u8>) -> (u16, String, Duration) {
    let req = http::Request::post(uri).header("content-type", content_type).body(Bytes::from(body)).unwrap();
    let mut req = Request::from_http(req);
    req.set_body_limit(2 * 1024 * 1024);
    let start = Instant::now();
    let res = app.handle(req).await;
    (res.status.as_u16(), res.into_text().await, start.elapsed())
}

#[tokio::test]
async fn huge_and_deep_bodies_are_bounded() {
    let base = project("bodies");
    let app = app(base.join("site"), 10);
    let form = "application/x-www-form-urlencoded";

    // As many keys as fit the body limit, all different, then all the same.
    let many: String = (0..150_000).map(|i| format!("k{i}=v&")).collect();
    let (status, text, took) = post(&app, "/echo", form, many.into_bytes()).await;
    assert_eq!((status, text.as_str()), (200, "Ok(150000)"));
    assert!(took < Duration::from_secs(30), "{took:?}");
    let same = "a=1&".repeat(400_000);
    let (status, text, took) = post(&app, "/echo", form, same.into_bytes()).await;
    assert_eq!((status, text.as_str()), (200, "Ok(1)"));
    assert!(took < Duration::from_secs(30), "{took:?}");
    // Nested names are plain keys: nothing recurses.
    let nested = format!("a{}=1", "[b]".repeat(200_000));
    assert_eq!(post(&app, "/echo", form, nested.into_bytes()).await.0, 200);
    // Over the limit: refused before it is parsed.
    let big = vec![b'a'; 2 * 1024 * 1024 + 1];
    assert!(post(&app, "/echo", form, big).await.1.contains("413"), "over the body limit");

    // JSON nested deeper than any document: an error, not a stack overflow.
    for open in ["[", "{\"a\":"] {
        let (status, text, _) =
            post(&app, "/json", "application/json", open.repeat(1_000_000 / open.len()).into_bytes()).await;
        assert_eq!((status, text.as_str()), (200, "refused: 400 Bad Request"));
    }

    // Query strings: `http::Uri` holds at most 64 KB.
    let query: String = (0..6000).map(|i| format!("q{i}=1&")).collect();
    let req = Request::from_http(http::Request::get(format!("/p/x?{query}")).body(Bytes::new()).unwrap());
    let start = Instant::now();
    assert_eq!(req.query::<HashMap<String, String>>().unwrap().len(), 6000);
    assert_eq!(req.query_param("q5999").as_deref(), Some("1"));
    assert!(start.elapsed() < Duration::from_secs(20));
    let _ = std::fs::remove_dir_all(base);
}

#[tokio::test]
async fn pathological_paths_are_matched_in_bounded_time() {
    let base = project("routes");
    let app = app(base.join("site"), 10);
    let mut rng = Rng(0x5EED_1005);
    let pieces = &["/", "//", "p", "%", "%2f", "%00", "%ff", "%e2%80%ae", "a", ".", "..", ";", "*", ":", "[slug]", "+"];
    let start = Instant::now();
    for round in 0..3000 {
        let path = match round {
            0 => format!("/p{}", "/a".repeat(30_000)),
            1 => format!("/p/{}", "%41".repeat(20_000)),
            2 => format!("/{}", "/".repeat(60_000)),
            _ => format!("/p/{}", rng.mix(pieces, 12)),
        };
        let Ok(uri) = path.parse::<http::Uri>() else { continue };
        let res = app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await;
        assert!(res.status.as_u16() < 500, "{path:?}: {}", res.status);
        let text = res.into_text().await;
        assert!(!text.contains("<p><") && !text.contains(SECRET), "{path:?}");
    }
    assert!(start.elapsed() < Duration::from_secs(20), "{:?}", start.elapsed());
    let _ = std::fs::remove_dir_all(base);
}

/// Send `request` to a real server and read until it closes the connection
/// or `wait` passes: `(what arrived, whether the server closed)`.
async fn raw(addr: std::net::SocketAddr, request: Vec<u8>, wait: Duration) -> (String, bool) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    // The server may close while this is still being written.
    let _ = stream.write_all(&request).await;
    let mut got = Vec::new();
    let closed = tokio::time::timeout(wait, async {
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => got.extend_from_slice(&buf[..n]),
            }
        }
    })
    .await
    .is_ok();
    (String::from_utf8_lossy(&got).into_owned(), closed)
}

#[tokio::test]
async fn the_server_limits_headers_and_closes_silent_connections() {
    let base = project("socket");
    let app = app(base.join("site"), 1);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { app.serve_listener(listener).await });
    let wait = Duration::from_secs(6);

    // 10,000 headers, and one header of 1 MB: refused, never buffered whole.
    let mut many = b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\n".to_vec();
    for i in 0..10_000 {
        many.extend_from_slice(format!("x-h{i}: v\r\n").as_bytes());
    }
    many.extend_from_slice(b"\r\n");
    let (head, closed) = raw(addr, many, wait).await;
    assert!(head.starts_with("HTTP/1.1 431") && closed, "{head:?}");
    let mut long = b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\nx-long: ".to_vec();
    long.extend(std::iter::repeat_n(b'a', 1024 * 1024));
    long.extend_from_slice(b"\r\n\r\n");
    let (head, closed) = raw(addr, long, wait).await;
    assert!(head.starts_with("HTTP/1.1 431") && closed, "{head:?}");
    // A 64 KB value is within the limit and reaches the app unharmed.
    let mut value = b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nx-long: ".to_vec();
    value.extend(std::iter::repeat_n(b'a', 64 * 1024));
    value.extend_from_slice(b"\r\n\r\n");
    assert!(raw(addr, value, wait).await.0.starts_with("HTTP/1.1 200"));

    // Request smuggling posture: both lengths, or two different lengths, are refused.
    for smuggle in [
        "POST /echo HTTP/1.1\r\nHost: l\r\nContent-Length: 4\r\nContent-Length: 11\r\n\r\na=1&GET /x HTTP/1.1\r\n\r\n",
        "POST /echo HTTP/1.1\r\nHost: l\r\nContent-Length: 4\r\nTransfer-Encoding: chunked, identity\r\n\r\na=1&",
        "POST /echo HTTP/1.1\r\nHost: l\r\nTransfer-Encoding: xchunked\r\n\r\n0\r\n\r\n",
        "GET /ok.txt HTTP/1.1\r\nHost: l\r\nContent-Length : 0\r\n\r\n",
        "GET /ok.txt HTTP/1.1\r\nHost: l\r\n X-Folded: 1\r\n\r\n",
    ] {
        let (head, closed) = raw(addr, smuggle.as_bytes().to_vec(), wait).await;
        assert!(head.starts_with("HTTP/1.1 400") && closed, "{smuggle:?} -> {head:?}");
        assert_eq!(head.matches("HTTP/1.1 ").count(), 1, "one request, one response: {head:?}");
    }

    // Slowloris: a header that never ends, and a connection that never
    // speaks, are closed once `[server] header_timeout` (1 s here) passes.
    for silent in [b"GET /ok.txt HTTP/1.1\r\nHost: loc".to_vec(), Vec::new()] {
        let start = Instant::now();
        let (head, closed) = raw(addr, silent.clone(), wait).await;
        assert!(closed, "still open after {:?} ({} bytes sent): {head:?}", start.elapsed(), silent.len());
        assert!(!head.starts_with("HTTP/1.1 200"));
    }
    // So is a keep-alive connection that goes quiet after its first request.
    let start = Instant::now();
    let (head, closed) = raw(addr, b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec(), wait).await;
    assert!(head.starts_with("HTTP/1.1 200") && closed, "idle keep-alive open after {:?}", start.elapsed());

    server.abort();
    let _ = std::fs::remove_dir_all(base);
}

/// Release builds serve `public/` from the binary through a second lookup
/// (`embed::find`), which never touches the filesystem. It has to refuse the
/// same paths the disk resolver refuses, so the generated paths go through
/// both.
#[test]
fn the_embedded_lookup_refuses_what_the_disk_resolver_refuses() {
    use next_rust_server::embed::{EmbeddedFile, find};

    // Sorted by path, as the build writes it. The build skips hidden files,
    // so `.env` and `sub/.htpasswd` could not be embedded at all; they are
    // here to show the lookup would not reach them even then.
    static FILES: &[EmbeddedFile] = &[
        EmbeddedFile { path: ".env", bytes: SECRET.as_bytes(), hash: "h0", br: None, gz: None },
        EmbeddedFile { path: "ok.txt", bytes: b"public file", hash: "h1", br: None, gz: None },
        EmbeddedFile { path: "sub/.htpasswd", bytes: SECRET.as_bytes(), hash: "h2", br: None, gz: None },
        EmbeddedFile { path: "sub/file.txt", bytes: b"public file", hash: "h3", br: None, gz: None },
    ];

    let mut rng = Rng(0x5EED_1006);
    let mut reached = 0;
    for _ in 0..6000 {
        let path = format!("/{}", rng.mix(PATH_PIECES, 9));
        if let Some(file) = find(FILES, &path) {
            assert!(!file.path.split('/').any(|s| s.starts_with('.')), "{path:?} reached {}", file.path);
            reached += 1;
        }
    }
    assert!(reached > 0, "the generator reaches real files too");
    for bad in [
        "/.env",
        "/%2Eenv",
        "/%2e%65nv",
        "/sub/.htpasswd",
        "/sub%2F.htpasswd",
        "/../.env",
        "/sub/../.env",
        "/%2e%2e/ok.txt",
        "/ok.txt%00",
        "/ok.txt%00.png",
        "/",
        "//",
        "/sub",
        "/OK.TXT",
    ] {
        assert!(find(FILES, bad).is_none(), "{bad} must not resolve");
    }
    assert!(find(FILES, "/ok.txt").is_some() && find(FILES, "/sub/file.txt").is_some());
    // Empty segments are dropped, exactly as the disk resolver drops them:
    // these name the same file, they do not escape anywhere.
    assert_eq!(find(FILES, "/ok.txt/").map(|f| f.path), Some("ok.txt"));
    assert_eq!(find(FILES, "//ok.txt").map(|f| f.path), Some("ok.txt"));
}
