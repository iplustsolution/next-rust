//! The "benchmarks" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "A benchmark runs one operation many times and measures how long it takes. The numbers on this page come from the harness in the framework's own repository; nothing here is an estimate.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "This page answers \"how much time does the framework itself add to a request?\" The answer is measured in millionths of a second. For a real site that is rarely the number that matters: a database query or a call to another service usually takes a thousand times longer. Read these figures as proof that the framework is not the bottleneck, not as a promise about your site's speed.",
            ],
        ],
        h2![id("units"), a![class("anchor"), href("#units"), "The units"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["unit"], th!["is"], th!["for scale"]]],
                tbody![
                    tr![
                        td!["ms (millisecond)"],
                        td!["a thousandth of a second"],
                        td!["a blink of an eye is about 100 ms"]
                    ],
                    tr![td!["µs (microsecond)"], td!["a millionth of a second"], td!["1,000 µs = 1 ms"]],
                    tr![td!["ns (nanosecond)"], td!["a billionth of a second"], td!["1,000 ns = 1 µs"]],
                    tr![
                        td!["k/s, M/s"],
                        td!["thousand / million operations per second"],
                        td!["the inverse of the time per iteration"]
                    ],
                ],
            ],
        ],
        h2![id("running"), a![class("anchor"), href("#running"), "Running them yourself"]],
        p![
            "The harness is part of the framework's repository, not of an application. Clone ",
            code!["https://github.com/iplustsolution/next-rust"],
            " and run this from its root folder. ",
            code!["--release"],
            " matters: an unoptimized build is many times slower and says nothing.",
        ],
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
        p![
            "Each row is one operation. \"Match a route\" is finding which page answers a URL; \"scan + flatten + validate\" is reading the ",
            code!["app/"],
            " folder at build time; ",
            code!["render_static"],
            " is turning a view into HTML; an \"SSR request\" is the whole path from request to rendered response, without the network.",
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
        h2![id("your-own-app"), a![class("anchor"), href("#your-own-app"), "Measuring your own app"]],
        p![
            "The numbers that matter are those of your site on your server. Build the production binary first (",
            code!["next-rust build"],
            ", then ",
            code!["next-rust start"],
            "); a development build is unoptimized and reads files from disk, so measuring it is the most common mistake. Then:",
        ],
        ul![
            li![
                code!["next-rust analyze"],
                " reports the size of the binary and the render time of every pre-rendered page (from ",
                code!["target/next-rust/build.json"],
                ").",
            ],
            li![
                "Point a load generator of your choice at the running binary from a second machine; on the same machine the generator competes with the server for the CPU.",
            ],
            li![
                "Compare like with like: the HTTP figure above was taken without compression and without request logging.",
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
