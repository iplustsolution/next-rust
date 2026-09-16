//! Next Rust benchmarks.
//!
//! `cargo run --release -p next-rust-benchmarks [-- --quick]`
//!
//! A small harness instead of criterion keeps the dependency graph small.
//! Each benchmark runs warm-up iterations, then reports the median of several
//! timed batches. Numbers are machine-specific; see docs/benchmarks.md.

use std::hint::black_box;
use std::time::{Duration, Instant};

use bytes::Bytes;
use next_rust_core::{Config, Environment};
use next_rust_router::{Matcher, PatternSegment, ScanOptions, scan};
use next_rust_server::app::{PageBody, PageDef, Rendering, Routes, SegmentDef};
use next_rust_server::{App, BoxFuture, Ctx, Next, Request, Response, Result};
use next_rust_view::*;

struct Bench {
    quick: bool,
}

impl Bench {
    /// Run `f` repeatedly; print time per iteration and throughput.
    fn run(&self, name: &str, mut f: impl FnMut()) {
        let budget = if self.quick { Duration::from_millis(150) } else { Duration::from_millis(800) };
        // Calibrate iterations per batch (~budget/10).
        let start = Instant::now();
        let mut calib = 0u64;
        while start.elapsed() < budget / 10 {
            f();
            calib += 1;
        }
        let batch = calib.max(1);
        let mut samples = Vec::new();
        for _ in 0..9 {
            let t = Instant::now();
            for _ in 0..batch {
                f();
            }
            samples.push(t.elapsed().as_nanos() as f64 / batch as f64);
        }
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = samples[samples.len() / 2];
        println!("{name:<52} {:>12}  {:>14}", fmt_ns(median), format!("{:.0}/s", 1e9 / median));
    }
}

fn fmt_ns(ns: f64) -> String {
    match ns {
        n if n < 1_000.0 => format!("{n:.0} ns"),
        n if n < 1_000_000.0 => format!("{:.2} µs", n / 1e3),
        n => format!("{:.2} ms", n / 1e6),
    }
}

fn synthetic_patterns(n: usize) -> Vec<Vec<PatternSegment>> {
    (0..n)
        .map(|i| match i % 4 {
            0 => vec![PatternSegment::Static(format!("section{}", i / 4)), PatternSegment::Static("list".into())],
            1 => vec![PatternSegment::Static(format!("section{}", i / 4)), PatternSegment::Dynamic("id".into())],
            2 => vec![
                PatternSegment::Static(format!("section{}", i / 4)),
                PatternSegment::Dynamic("id".into()),
                PatternSegment::Static("edit".into()),
            ],
            _ => vec![
                PatternSegment::Static(format!("section{}", i / 4)),
                PatternSegment::Static("docs".into()),
                PatternSegment::CatchAll("rest".into()),
            ],
        })
        .collect()
}

fn page(_ctx: Ctx) -> BoxFuture<Result<Node>> {
    Box::pin(async {
        Ok(div![
            h1!["Benchmark"],
            ul![each(0..50, |i| li![class("item"), a![href(format!("/items/{i}")), format!("Item {i}")]])],
        ]
        .into_node())
    })
}

fn layout(_ctx: Ctx, children: Children, _slots: Slots) -> BoxFuture<Result<Node>> {
    Box::pin(async move { Ok(div![header!["Site"], main![children], footer!["©"]].into_node()) })
}

fn passthrough(req: Request, next: Next) -> BoxFuture<Response> {
    Box::pin(async move { next.run(req).await })
}

fn bench_app(middleware_layers: usize) -> App {
    let seg = SegmentDef { layout: Some(layout), ..Default::default() };
    let mut routes = Routes::default();
    routes.pages.push(PageDef {
        pattern: "/",
        source: "bench",
        body: PageBody::Rust(page),
        segments: vec![seg.clone(), SegmentDef { middleware: None, ..Default::default() }],
        metadata: None,
        rendering: Rendering::Dynamic,
        revalidate: None,
        generate_params: None,
        dynamic_params: true,
        tags: &[],
        intercept_from: None,
    });
    let mut config = Config::default();
    config.logging.requests = false;
    let mut builder = App::new(routes).config(config).environment(Environment::Production);
    for _ in 0..middleware_layers {
        builder = builder.middleware(passthrough);
    }
    builder.build().unwrap()
}

fn main() {
    let quick = std::env::args().any(|a| a == "--quick");
    let b = Bench { quick };
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    next_rust_server::log::configure(false, "error");

    println!("{:<52} {:>12}  {:>14}", "benchmark", "time/iter", "throughput");
    println!("{}", "-".repeat(82));

    // Route matching
    for n in [100, 10_000] {
        let mut m = Matcher::new();
        for (i, p) in synthetic_patterns(n).into_iter().enumerate() {
            m.insert(&p, i).unwrap();
        }
        let last = n / 4 - 1;
        b.run(&format!("match static route ({n} routes)"), || {
            black_box(m.at(black_box(&format!("/section{last}/list"))));
        });
        b.run(&format!("match dynamic route + params ({n} routes)"), || {
            black_box(m.at(black_box("/section7/12345/edit")));
        });
        b.run(&format!("match catch-all route ({n} routes)"), || {
            black_box(m.at(black_box("/section3/docs/a/b/c/d")));
        });
    }

    // Route discovery
    let dir = std::env::temp_dir().join(format!("nr-bench-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for s in 0..100 {
        for r in 0..10 {
            let d = dir.join(format!("s{s}/r{r}/[id]"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("page.rs"), "").unwrap();
            std::fs::write(dir.join(format!("s{s}/r{r}/page.rs")), "").unwrap();
        }
        std::fs::write(dir.join(format!("s{s}/layout.rs")), "").unwrap();
    }
    b.run("scan + flatten + validate (2,000 routes, 2,101 dirs)", || {
        black_box(scan(&dir, &ScanOptions::default()));
    });
    let _ = std::fs::remove_dir_all(&dir);

    // Rendering
    b.run("render_static: 1,000-item list", || {
        let view = ul![each(0..1000, |i| li![class("row"), span![format!("Row {i}")], a![href("/x"), "link"]])];
        black_box(render_static(view));
    });
    b.run("render_static: escaping-heavy text (10 KB)", || {
        let text = "<b>&\"quoted\"</b> ".repeat(500);
        black_box(render_static(p![text]));
    });

    // Full pipeline (in-process, no network)
    let app0 = bench_app(0);
    b.run("SSR request through App::handle (layout + 50 items)", || {
        rt.block_on(async {
            let res = app0.handle(Request::from_http(http::Request::get("/").body(Bytes::new()).unwrap())).await;
            black_box(res.into_bytes().await.unwrap());
        })
    });
    let app10 = bench_app(10);
    b.run("SSR request with 10 middleware layers", || {
        rt.block_on(async {
            let res = app10.handle(Request::from_http(http::Request::get("/").body(Bytes::new()).unwrap())).await;
            black_box(res.into_bytes().await.unwrap());
        })
    });

    // Startup
    b.run("App build (config + matcher, 1 route)", || {
        black_box(bench_app(0));
    });

    // HTTP throughput over loopback TCP (keep-alive, sequential per connection).
    let app = bench_app(0);
    let (addr, server) = rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        (addr, tokio::spawn(app.serve_listener(listener)))
    });
    let connections = 32;
    let duration = if quick { Duration::from_millis(500) } else { Duration::from_secs(3) };
    let total = rt.block_on(async move {
        let deadline = Instant::now() + duration;
        let mut tasks = Vec::new();
        for _ in 0..connections {
            tasks.push(tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
                s.set_nodelay(true).unwrap();
                let mut buf = vec![0u8; 64 * 1024];
                let mut n = 0u64;
                while Instant::now() < deadline {
                    s.write_all(b"GET / HTTP/1.1\r\nHost: bench\r\n\r\n").await.unwrap();
                    // Read until the chunked terminator.
                    let mut acc = Vec::new();
                    loop {
                        let r = s.read(&mut buf).await.unwrap();
                        acc.extend_from_slice(&buf[..r]);
                        if acc.ends_with(b"0\r\n\r\n") {
                            break;
                        }
                    }
                    n += 1;
                }
                n
            }));
        }
        let mut total = 0;
        for t in tasks {
            total += t.await.unwrap();
        }
        total
    });
    println!(
        "{:<52} {:>12}  {:>14}",
        format!("HTTP/1.1 keep-alive, {connections} connections"),
        format!("{} reqs", total),
        format!("{:.0}/s", total as f64 / duration.as_secs_f64())
    );
    server.abort();
}
