//! The "benchmarks" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        pre![code![
            class("language-sh"),
            r"cargo run --release -p next-rust-benchmarks            # ~30 s
cargo run --release -p next-rust-benchmarks -- --quick",
        ],],
        p![
            "The harness (",
            code!["benchmarks/src/main.rs"],
            ") runs warm-up iterations, then reports the ",
            strong!["median"],
            " of nine timed batches. It avoids criterion to keep the dependency graph small, so treat results as indicative, not statistically rigorous.",
        ],
        h2![id("results"), a![class("anchor"), href("#results"), "Results"]],
        p![
            "Measured on 2026-09-17: Apple Silicon (aarch64-apple-darwin), Rust 1.98.0, release profile (",
            code!["lto = \"thin\""],
            "), other applications running. These are ",
            strong!["single-machine numbers. No comparison with other frameworks was made, and none is claimed.",],
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["benchmark"], th!["time / iteration"], th!["throughput"]]],
                tbody![
                    tr![td!["match static route (100 routes)"], td!["137 ns"], td!["7.3 M/s"]],
                    tr![td!["match dynamic route + params (100 routes)"], td!["174 ns"], td!["5.8 M/s"]],
                    tr![td!["match catch-all route (100 routes)"], td!["327 ns"], td!["3.1 M/s"]],
                    tr![td!["match static route (10,000 routes)"], td!["139 ns"], td!["7.2 M/s"]],
                    tr![td!["match dynamic route + params (10,000 routes)"], td!["176 ns"], td!["5.7 M/s"]],
                    tr![td!["match catch-all route (10,000 routes)"], td!["315 ns"], td!["3.2 M/s"]],
                    tr![td!["scan + flatten + validate (2,000 routes, 2,101 dirs)"], td!["69.35 ms"], td!["14/s"],],
                    tr![td![code!["render_static"], ": 1,000-item list"], td!["333 µs"], td!["3.0 k/s"]],
                    tr![td![code!["render_static"], ": escaping-heavy text (10 KB)"], td!["8.68 µs"], td!["115 k/s"],],
                    tr![
                        td!["SSR request through ", code!["App::handle"], " (layout + 50 items)"],
                        td!["19.6 µs"],
                        td!["51 k/s"],
                    ],
                    tr![td!["SSR request with 10 middleware layers"], td!["19.6 µs"], td!["51 k/s"]],
                    tr![td![code!["App"], " build (config + matcher, 1 route)"], td!["7.7 µs"], td!["130 k/s"],],
                    tr![td!["HTTP/1.1 keep-alive over loopback, 32 connections"], td!["—"], td!["97 k req/s"]],
                ],
            ],
        ],
        h2![id("reading-the-numbers"), a![class("anchor"), href("#reading-the-numbers"), "Reading the numbers"],],
        ul![
            li![
                strong!["Route matching doesn't depend on route count."],
                " 100 and 10,000 routes cost the same, because the trie's work is proportional to URL segments. The static-match figure includes a ",
                code!["format!"],
                " allocation for the benchmark URL.",
            ],
            li![strong!["Discovery"], " is a build- and dev-time cost only. Production never scans the filesystem.",],
            li![
                strong!["Middleware"],
                " passthrough layers cost less than the measurement noise at this scale (one boxed future and one ",
                code!["Arc"],
                " clone per layer).",
            ],
            li![
                strong!["HTTP throughput"],
                " runs the load generator in the same process and on the same machine as the server, without ",
                code!["Accept-Encoding"],
                " (no compression) and with request logging disabled. It measures the full hyper + framework stack, including rendering a 50-item page per request.",
            ],
        ],
        h2![id("not-yet-measured"), a![class("anchor"), href("#not-yet-measured"), "Not yet measured"]],
        ul![
            li![
                strong!["Memory usage"],
                ". Measure externally, e.g. ",
                code!["/usr/bin/time -l ./target/release/my-app"],
                " on macOS or ",
                code!["/usr/bin/time -v"],
                " on Linux.",
            ],
            li![
                strong!["Startup time of a real binary"],
                ". The ",
                code!["App"],
                " build figure excludes process start and config discovery.",
            ],
            li![
                strong!["Static generation throughput"],
                ". ",
                code!["next-rust build"],
                " prints per-page render times in ",
                code!["target/next-rust/build.json"],
                ".",
            ],
            li![strong!["Streaming time-to-first-byte"], " under load."],
        ],
        p!["Contributions of reproducible benchmark scenarios are welcome."],
    ]
}
