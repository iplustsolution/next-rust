# Benchmarks

```sh
cargo run --release -p next-rust-benchmarks            # ~30 s
cargo run --release -p next-rust-benchmarks -- --quick
```

The harness (`benchmarks/src/main.rs`) runs warm-up iterations, then reports
the **median** of nine timed batches. It avoids criterion to keep the
dependency graph small, so treat results as indicative, not statistically
rigorous.

## Results

Measured on 2026-09-17: Apple Silicon (aarch64-apple-darwin), Rust 1.98.0,
release profile (`lto = "thin"`), other applications running. These are
**single-machine numbers. No comparison with other frameworks was made, and
none is claimed.**

| benchmark | time / iteration | throughput |
|---|---|---|
| match static route (100 routes) | 137 ns | 7.3 M/s |
| match dynamic route + params (100 routes) | 174 ns | 5.8 M/s |
| match catch-all route (100 routes) | 327 ns | 3.1 M/s |
| match static route (10,000 routes) | 139 ns | 7.2 M/s |
| match dynamic route + params (10,000 routes) | 176 ns | 5.7 M/s |
| match catch-all route (10,000 routes) | 315 ns | 3.2 M/s |
| scan + flatten + validate (2,000 routes, 2,101 dirs) | 69.35 ms | 14/s |
| `render_static`: 1,000-item list | 333 µs | 3.0 k/s |
| `render_static`: escaping-heavy text (10 KB) | 8.68 µs | 115 k/s |
| SSR request through `App::handle` (layout + 50 items) | 19.6 µs | 51 k/s |
| SSR request with 10 middleware layers | 19.6 µs | 51 k/s |
| `App` build (config + matcher, 1 route) | 7.7 µs | 130 k/s |
| HTTP/1.1 keep-alive over loopback, 32 connections | — | 97 k req/s |

## Reading the numbers

- **Route matching doesn't depend on route count.** 100 and 10,000 routes
  cost the same, because the trie's work is proportional to URL segments.
  The static-match figure includes a `format!` allocation for the benchmark
  URL.
- **Discovery** is a build- and dev-time cost only. Production never scans
  the filesystem.
- **Middleware** passthrough layers cost less than the measurement noise at
  this scale (one boxed future and one `Arc` clone per layer).
- **HTTP throughput** runs the load generator in the same process and on the
  same machine as the server, without `Accept-Encoding` (no compression)
  and with request logging disabled. It measures the full hyper + framework stack,
  including rendering a 50-item page per request.

## Not yet measured

- **Memory usage**. Measure externally, e.g.
  `/usr/bin/time -l ./target/release/my-app` on macOS or
  `/usr/bin/time -v` on Linux.
- **Startup time of a real binary**. The `App` build figure excludes process
  start and config discovery.
- **Static generation throughput**. `next-rust build` prints per-page render
  times in `.next-rust/manifest/build.json`.
- **Streaming time-to-first-byte** under load.

Contributions of reproducible benchmark scenarios are welcome.
