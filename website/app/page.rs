use crate::highlight::highlight;
use crate::ui::{BUTTON, BUTTON_PRIMARY, REPO, SYNTAX, VERSION};
use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().absolute_title("Next Rust · The full-stack web framework for Rust")
}

const INSTALL: &str = "cargo install next-rust-cli";
const SPONSOR: &str = "https://github.com/sponsors/iplustsolution";

const PAGE_EXAMPLE: &str = r#"use next_rust::prelude::*;

pub async fn load(params: Params) -> Result<Post> {
    posts::find(params.get("slug").unwrap_or_default())
        .await
        .or_not_found()
}

pub fn Page(Data(post): Data<Post>) -> impl View {
    article![
        h1![post.title],
        time![datetime(&post.date), &post.date],
        p![post.summary],
    ]
}"#;

const WASM_EXAMPLE: &str = r#"#[client(wasm = "AuthSwitch")]
pub fn AuthSwitch(signed_in: bool)
    -> impl View
{
    nav![class("auth"), /* … */]
}"#;

const ACTION_EXAMPLE: &str = r#"#[server_action]
pub async fn signup(input: Signup) -> Result<()> {
    if !input.email.contains('@') {
        return Err(Error::validation([("email", "Invalid")]));
    }
    users::create(input).await?;
    Err(redirect("/welcome"))
}"#;

const PPR_EXAMPLE: &str = r#"dynamic(p!["Loading…"], |ctx| async move {
    let user = ctx.cookies.get("user");
    Ok::<_, Error>(p![greet(user)])
})"#;

const SHIP_EXAMPLE: &str = "next-rust build\nscp .next-rust/my-app server:/srv/my-app/\nssh server /srv/my-app/my-app";

// The logo (public/logo.svg) taken apart into layers, so the gear can turn
// while the "N" stands still. Each layer is its own <svg> inside a <div> that
// carries the animation class: nothing in these strings needs renaming.
const LOGO_ORBIT: &str = r##"<svg viewBox="0 0 512 512" width="100%" height="100%" fill="none" aria-hidden="true"><circle cx="256" cy="256" r="244" stroke="currentColor" stroke-width="1.5" stroke-dasharray="3 11"/><circle cx="256" cy="12" r="6" fill="#f25c1f"/><circle cx="467.3" cy="378" r="4" fill="#f25c1f" opacity="0.7"/><circle cx="44.7" cy="378" r="3" fill="#f25c1f" opacity="0.45"/></svg>"##;
const LOGO_TICKS: &str = r##"<svg viewBox="0 0 512 512" width="100%" height="100%" fill="none" aria-hidden="true"><circle cx="256" cy="256" r="214" stroke="currentColor" stroke-width="10" stroke-dasharray="1.5 9.705"/></svg>"##;
const LOGO_GEAR: &str = r##"<svg viewBox="0 0 512 512" width="100%" height="100%" aria-hidden="true"><defs><linearGradient id="hero-rust" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff9c2a"/><stop offset="0.55" stop-color="#f25c1f"/><stop offset="1" stop-color="#b3260c"/></linearGradient></defs><g fill="url(#hero-rust)"><rect x="238" y="70" width="36" height="72" rx="8"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(30 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(60 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(90 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(120 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(150 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(180 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(210 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(240 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(270 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(300 256 256)"/><rect x="238" y="70" width="36" height="72" rx="8" transform="rotate(330 256 256)"/></g><circle cx="256" cy="256" r="146" fill="none" stroke="url(#hero-rust)" stroke-width="40"/></svg>"##;
const LOGO_CORE: &str = r##"<svg viewBox="0 0 512 512" width="100%" height="100%" role="img" aria-label="Next Rust logo"><defs><linearGradient id="hero-fade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f25c1f"/><stop offset="1" stop-color="#f25c1f" stop-opacity="0"/></linearGradient><clipPath id="hero-inner"><circle cx="256" cy="256" r="126"/></clipPath></defs><circle cx="256" cy="256" r="127" fill="currentColor"/><g clip-path="url(#hero-inner)"><rect x="190" y="186" width="30" height="140" rx="4" fill="#f25c1f"/><polygon points="190,186 222,186 350,360 318,360" fill="url(#hero-fade)"/><rect x="292" y="186" width="30" height="92" rx="4" fill="url(#hero-fade)"/></g></svg>"##;

const HEART: &str = r#"<svg viewBox="0 0 24 24" width="1.0625rem" height="1.0625rem" fill="currentColor" aria-hidden="true"><path d="M2 9.5a5.5 5.5 0 0 1 9.591-3.676.56.56 0 0 0 .818 0A5.49 5.49 0 0 1 22 9.5c0 2.29-1.5 4-3 5.5l-5.492 5.313a2 2 0 0 1-3 .019L5 15c-1.5-1.5-3-3.2-3-5.5"/></svg>"#;
const STAR: &str = r#"<svg viewBox="0 0 24 24" width="1.0625rem" height="1.0625rem" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/></svg>"#;

/// The hero ends on a hairline that fades out towards both edges, so the
/// ambient light is not cut off by a hard full-width rule.
const SOFT_RULE: &str = "after:absolute after:inset-x-0 after:bottom-0 after:h-px after:bg-[linear-gradient(to_right,transparent_0%,var(--color-line-strong)_24%,var(--color-line-strong)_76%,transparent_100%)] after:content-['']";
const WRAP: &str = "mx-auto max-w-[80rem] px-6 max-[640px]:px-4";
const LABEL: &str = "font-mono text-[0.75rem]/[1.6] tracking-[0.14em] text-accent-fg uppercase";
const SECTION_TITLE: &str =
    "mt-3 max-w-[44rem] text-[clamp(1.75rem,3.6vw,2.625rem)] leading-[1.1] font-bold tracking-[-0.035em] text-balance";
const PANEL: &str = "rounded-xl border border-line bg-code";
const CODE: &str = "overflow-x-auto font-mono text-[0.8125rem]/[1.7] text-fg-soft";

/// One cell of the feature grid. `span` sets its width at each breakpoint.
fn cell(
    span: &'static str,
    label: &'static str,
    title: &'static str,
    text: &'static str,
    extra: impl View,
) -> impl View {
    div![
        class("group relative flex min-w-0 flex-col bg-bg p-7 transition-colors hover:bg-soft max-[640px]:p-5"),
        class(span),
        p![
            class("font-mono text-[0.6875rem]/[1.6] tracking-[0.14em] text-muted uppercase group-hover:text-accent-fg"),
            label
        ],
        h3![class("mt-3 text-[1.125rem]/[1.3] font-[650] tracking-[-0.02em]"), title],
        p![class("mt-2 text-[0.9375rem]/[1.6] text-muted"), text],
        extra,
    ]
}

fn snippet(lang: &'static str, source: &'static str) -> impl View {
    pre![class(PANEL), class(CODE), class(SYNTAX), class("mt-5 px-4 py-3.5"), code![raw_html(highlight(lang, source))]]
}

/// Big mono figures inside a cell.
fn figures(items: &'static [(&'static str, &'static str)]) -> impl View {
    dl![
        class("mt-auto flex flex-wrap gap-x-6 gap-y-3 pt-6"),
        each(items, |(value, unit)| div![
            dt![class("font-mono text-[1.375rem]/[1.1] font-semibold tracking-[-0.04em] text-fg"), *value],
            dd![class("mt-1 font-mono text-[0.6875rem] tracking-[0.06em] text-muted uppercase"), *unit],
        ]),
    ]
}

fn animated_logo() -> impl View {
    let layer = "absolute inset-0";
    div![
        class("relative mx-auto aspect-square w-full max-w-[32rem] max-[960px]:max-w-[15rem]"),
        div![class(
            "absolute -inset-[12%] rounded-full bg-[radial-gradient(closest-side,rgb(242_92_31/0.34),transparent)] motion-safe:animate-glow-pulse"
        ),],
        div![class(layer), class("text-line-strong motion-safe:animate-orbit-spin"), raw_html(LOGO_ORBIT)],
        div![class(layer), class("text-muted/60 motion-safe:animate-ticks-spin"), raw_html(LOGO_TICKS)],
        div![
            class(layer),
            class("drop-shadow-[0_1.25rem_2.5rem_rgb(179_38_12/0.28)] motion-safe:animate-gear-spin"),
            raw_html(LOGO_GEAR)
        ],
        div![class(layer), class("text-bg"), raw_html(LOGO_CORE)],
    ]
}

pub fn Page() -> impl View {
    let tree_item = |indent: &'static str, text: &'static str| li![class(indent), text];
    let tree_dir = |indent: &'static str, text: &'static str| li![class(indent), span![class("text-fg"), text]];
    let route = |text: &'static str| span![class("ml-2.5 text-[0.71875rem] text-muted"), text];
    let wide = "col-span-4 max-[960px]:col-span-2 max-[640px]:col-span-1";
    let half = "col-span-3 max-[960px]:col-span-1";
    let third = "col-span-2 max-[960px]:col-span-1";
    let third_wide = "col-span-2 max-[640px]:col-span-1";
    fragment![main![
        id("content"),
        class("overflow-x-clip"),
        // 1. Hero
        section![
            class("relative"),
            class(SOFT_RULE),
            div![
                class(WRAP),
                class(
                    "grid grid-cols-[minmax(0,1.08fr)_minmax(0,0.92fr)] items-center gap-x-16 gap-y-12 py-20 min-[961px]:min-h-[min(calc(100dvh-4rem),54rem)] max-[960px]:grid-cols-1 max-[960px]:py-14 max-[640px]:py-11"
                ),
                div![
                    class("min-w-0"),
                    a![
                        class(
                            "inline-flex items-center gap-2.5 rounded-full border border-line-strong bg-bg py-[0.3125rem] pr-3.5 pl-2.5 font-mono text-[0.75rem] text-fg-soft transition-colors hover:border-muted"
                        ),
                        href("/docs/status"),
                        span![class("size-[0.4375rem] rounded-full bg-accent ring-3 ring-accent/15")],
                        format!("v{VERSION}"),
                        span![class("h-3 w-px bg-line-strong")],
                        "see what's ready",
                    ],
                    h1![
                        class(
                            "mt-7 text-[clamp(2.5rem,5.6vw,4.75rem)] leading-[1.02] font-bold tracking-[-0.045em] text-balance"
                        ),
                        "The most advanced full-stack framework in ",
                        span![class("text-accent-fg"), "Rust"],
                        "."
                    ],
                    p![
                        class("mt-6 max-w-[36rem] text-[clamp(1.0625rem,1.6vw,1.1875rem)] leading-[1.65] text-muted"),
                        "App Router routing, streaming SSR, partial prerendering, typed server actions and WebAssembly islands, checked at compile time and shipped as one small binary. If you know Next.js, you already know how it works."
                    ],
                    div![
                        class("mt-9 flex flex-wrap gap-3"),
                        a![class(BUTTON_PRIMARY), href("/docs/getting-started"), "Get started"],
                        a![class(BUTTON), href("/docs"), "Read the docs"],
                    ],
                    div![
                        class(PANEL),
                        class(
                            "mt-6 flex w-fit max-w-full items-center gap-3 overflow-x-auto px-4 py-2.5 text-[0.84375rem] whitespace-nowrap max-[640px]:w-full"
                        ),
                        span![class("font-mono text-accent-fg select-none"), "$"],
                        code![class("font-mono text-fg-soft"), INSTALL],
                    ],
                ],
                div![class("min-w-0"), animated_logo()],
            ],
        ],
        // 2. Folder tree → page.rs
        section![
            class(WRAP),
            class("py-22 max-[640px]:py-14"),
            p![class(LABEL), "01 / Routing"],
            h2![class(SECTION_TITLE), "A folder is a route. A function is a page."],
            div![
                class("mt-10 grid grid-cols-[18rem_minmax(0,1fr)] gap-4 max-[960px]:grid-cols-1"),
                div![
                    class(PANEL),
                    class("px-5 py-[1.125rem] font-mono text-[0.84375rem]/[1.9]"),
                    p![class("mb-1.5 text-xs/[1.9] tracking-[0.08em] text-muted uppercase"), "my-app"],
                    ul![
                        class("text-fg-soft"),
                        tree_dir("", "app/"),
                        tree_item("pl-[1.125rem]", "layout.rs"),
                        tree_item("pl-[1.125rem]", "page.rs"),
                        tree_dir("pl-[1.125rem]", "blog/[slug]/"),
                        li![
                            class("-mx-2.5 rounded-md bg-accent/10 pr-2.5 pl-[2.875rem] text-accent-fg"),
                            "page.rs",
                            route("/blog/:slug")
                        ],
                        tree_item("pl-9", "loading.rs"),
                        tree_dir("pl-[1.125rem]", "api/hello/"),
                        li![class("pl-9"), "route.rs", route("GET /api/hello")],
                        tree_dir("", "public/"),
                        tree_item("", "next-rust.toml"),
                    ],
                ],
                div![
                    class(PANEL),
                    class("min-w-0 overflow-hidden"),
                    div![
                        class("flex h-[2.625rem] items-center gap-[0.4375rem] border-b border-line px-4"),
                        span![class("size-[0.6875rem] rounded-full bg-line-strong")],
                        span![class("size-[0.6875rem] rounded-full bg-line-strong")],
                        span![class("size-[0.6875rem] rounded-full bg-line-strong")],
                        p![class("ml-2.5 font-mono text-[0.78125rem] text-muted"), "app/blog/[slug]/page.rs"],
                    ],
                    pre![
                        class("overflow-x-auto px-[1.375rem] py-5 font-mono text-[0.84375rem]/[1.75]"),
                        class(SYNTAX),
                        code![raw_html(highlight("rust", PAGE_EXAMPLE))]
                    ],
                ],
            ],
        ],
        // 3. Features
        section![
            class(WRAP),
            class("pb-22 max-[640px]:pb-14"),
            p![class(LABEL), "02 / What's inside"],
            h2![class(SECTION_TITLE), "Everything the App Router does, and what only a compiler can."],
            div![
                class(
                    "mt-10 grid grid-cols-6 gap-px overflow-hidden rounded-2xl border border-line bg-line max-[960px]:grid-cols-2 max-[640px]:grid-cols-1"
                ),
                div![
                    class("relative isolate flex min-w-0 flex-col overflow-hidden bg-bg p-8 max-[640px]:p-5"),
                    class(wide),
                    div![class(
                        "pointer-events-none absolute -top-24 -right-24 -z-10 size-[22rem] rounded-full bg-[radial-gradient(closest-side,var(--color-accent-soft),transparent)]"
                    )],
                    p![class(LABEL), "Deploy"],
                    h3![
                        class("mt-3 text-[clamp(1.625rem,3vw,2.25rem)] leading-[1.1] font-bold tracking-[-0.035em]"),
                        "One binary. That's the deploy."
                    ],
                    p![
                        class("mt-3 max-w-[34rem] text-[0.9375rem]/[1.6] text-muted"),
                        "next-rust build writes a single stripped executable with your pages, config and public files embedded. No Node.js, no bundler, no node_modules on the server. Copy it and run it."
                    ],
                    p![class(LABEL), class("mt-7"), "Inside the file"],
                    ul![
                        class("mt-3 flex flex-wrap gap-2 font-mono text-[0.78125rem] text-fg-soft"),
                        each(
                            [
                                "app/ pages and layouts",
                                "next-rust.toml",
                                "public/",
                                "client runtime",
                                "the server itself"
                            ],
                            |item| li![class("rounded-md border border-line bg-soft px-2.5 py-1"), item]
                        ),
                    ],
                    div![
                        class(
                            "mt-auto grid grid-cols-[minmax(0,1fr)_auto] items-end gap-6 pt-8 max-[640px]:grid-cols-1"
                        ),
                        pre![
                            class(PANEL),
                            class(CODE),
                            class(SYNTAX),
                            class("px-4 py-3.5"),
                            code![raw_html(highlight("sh", SHIP_EXAMPLE))]
                        ],
                        div![
                            p![
                                class(
                                    "font-mono text-[clamp(2.5rem,5vw,3.75rem)] leading-none font-semibold tracking-[-0.06em] text-accent-fg"
                                ),
                                "1.2 MB"
                            ],
                            p![
                                class("mt-2 font-mono text-[0.6875rem] tracking-[0.06em] text-muted uppercase"),
                                "a new app, size-optimized"
                            ],
                        ],
                    ],
                ],
                div![
                    class("relative flex min-w-0 flex-col bg-soft p-8 max-[640px]:p-5"),
                    class("col-span-2 max-[640px]:col-span-1"),
                    p![class(LABEL), "WebAssembly"],
                    h3![
                        class("mt-3 text-[clamp(1.375rem,2.2vw,1.75rem)] leading-[1.15] font-bold tracking-[-0.03em]"),
                        "Islands written in Rust."
                    ],
                    p![
                        class("mt-3 text-[0.9375rem]/[1.6] text-muted"),
                        "Turn on wasm_islands and the islands/ crate compiles to one WebAssembly module for the whole site, behind a small JavaScript shim. Or point an island at any ES module, wasm-bindgen output included."
                    ],
                    snippet("rust", WASM_EXAMPLE),
                    p![
                        class("mt-4 font-mono text-[0.75rem]/[1.6] text-muted"),
                        "about 1.6 KB brotli for a measured island: 865 B module, 720 B shim"
                    ],
                ],
                cell(
                    third,
                    "Routing",
                    "App Router parity",
                    "Nested layouts, (groups), [dynamic] and [...catch-all] segments, parallel @slots and intercepting routes. loading.rs and error.rs work the way you expect.",
                    ()
                ),
                cell(
                    third,
                    "Rendering",
                    "Streaming and partial prerendering",
                    "Layouts reach the browser first. A static shell is served from cache while dynamic(..) holes render per request and stream in after it.",
                    snippet("rust", PPR_EXAMPLE)
                ),
                cell(
                    third,
                    "Caching",
                    "Static, dynamic or ISR, per route",
                    "Each route is pre-rendered or server-rendered based on what it reads. REVALIDATE refreshes it in the background; revalidate_tag(\"posts\") does it on demand.",
                    ()
                ),
                cell(
                    half,
                    "Server actions",
                    "Typed forms that work without JavaScript",
                    "A form posts to a Rust function. Validation errors come back to the right field, with or without the client runtime, and cross-origin requests are rejected.",
                    snippet("rust", ACTION_EXAMPLE)
                ),
                cell(
                    half,
                    "Compile time",
                    "A wrong route is a build error",
                    "The route table is generated by the build script, so mistakes never reach a browser. Every diagnostic has a code and a help line.",
                    ul![
                        class(PANEL),
                        class("mt-5 divide-y divide-line font-mono text-[0.78125rem]/[1.6]"),
                        each(
                            [
                                ("NR0102", "two routes resolve to the same URL"),
                                ("NR0105", "catch-all segment is not the last segment"),
                                ("NR0201", "a special file lacks its required pub fn"),
                                ("NR0204", "loading.rs and error.rs must not be async"),
                            ],
                            |(code, text)| li![
                                class("flex gap-4 px-4 py-2.5"),
                                span![class("shrink-0 text-accent-fg"), code],
                                span![class("text-muted"), text],
                            ]
                        ),
                    ]
                ),
                cell(
                    third,
                    "Client",
                    "Zero JavaScript by default",
                    "A page sends no script until it uses client navigation, a form or an island. Then it is one runtime of about 4 KB, safe under a strict CSP.",
                    figures(&[("0", "JS by default"), ("~4 KB", "runtime")])
                ),
                cell(
                    third,
                    "Styling",
                    "Tailwind CSS v4 built in",
                    "No Node.js, no config file. Release builds shorten class names, keyframes and custom properties, and each page gets only the rules it renders.",
                    ()
                ),
                cell(
                    third,
                    "UI",
                    "1,800+ Lucide icons and a component kit",
                    "icons::ArrowRight() and the rest as inline SVG, with unused icons left out of the binary. Dialogs, date pickers, charts and forms that are zero-JS first.",
                    ()
                ),
                cell(
                    third,
                    "Security",
                    "Safe unless you say otherwise",
                    "Output is escaped, inline scripts carry a per-request CSP nonce, server actions pass CSRF checks, and sessions use 256-bit ids from the OS.",
                    ()
                ),
                cell(
                    third,
                    "Backend",
                    "Jobs, cron and the rest",
                    "Durable queues with retries and a dead-letter list, cron schedules, middleware at any level, sessions, rate limiting, SSE and WebSockets.",
                    ()
                ),
                cell(
                    third_wide,
                    "Measured",
                    "Fast where it counts",
                    "Route matching costs the same at 100 and 10,000 routes. One machine, no comparisons claimed.",
                    figures(&[("137 ns", "route match"), ("19.6 µs", "SSR request"), ("97k req/s", "loopback")])
                ),
            ],
            p![
                class("mt-4 font-mono text-[0.75rem]/[1.6] text-muted"),
                "Numbers from ",
                a![
                    class("underline decoration-line-strong underline-offset-3 hover:text-fg"),
                    href("/docs/benchmarks"),
                    "the benchmarks page"
                ],
                " and ",
                a![
                    class("underline decoration-line-strong underline-offset-3 hover:text-fg"),
                    href("/docs/deployment"),
                    "deployment"
                ],
                ". Apple Silicon, release profile.",
            ],
        ],
        // 4. new → dev → build
        section![
            class("border-y border-line bg-soft"),
            div![
                class(WRAP),
                class("py-16 max-[640px]:py-12"),
                p![class(LABEL), "03 / Workflow"],
                ol![
                    class(
                        "mt-7 grid grid-cols-3 gap-px overflow-hidden rounded-xl border border-line bg-line max-[860px]:grid-cols-1"
                    ),
                    each(
                        [
                            ("new", "next-rust new my-app", "A working app with a layout, a page and Tailwind."),
                            ("dev", "next-rust dev", "Live reload, an error overlay, starter code for new files."),
                            ("build", "next-rust build", "Pre-renders static pages and writes one stripped binary."),
                        ],
                        |(step, command, text)| li![
                            class("min-w-0 bg-bg p-6"),
                            p![class("font-mono text-[0.6875rem] tracking-[0.14em] text-muted uppercase"), step],
                            pre![
                                class(CODE),
                                class(SYNTAX),
                                class("mt-3 text-[0.9375rem]/[1.6]"),
                                code![
                                    span![class("mr-2.5 text-accent-fg select-none"), "$"],
                                    raw_html(highlight("sh", command))
                                ]
                            ],
                            p![class("mt-2.5 text-[0.9375rem]/[1.55] text-muted"), text],
                        ]
                    ),
                ],
            ],
        ],
        // 5. Open source and sponsorship
        section![
            class(WRAP),
            class("py-22 max-[640px]:py-14"),
            div![
                class(
                    "relative isolate grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-12 gap-y-8 overflow-hidden rounded-2xl border border-line-strong bg-bg p-12 max-[960px]:grid-cols-1 max-[640px]:p-6"
                ),
                div![class(
                    "pointer-events-none absolute -top-1/2 right-0 -z-10 aspect-square w-[34rem] max-w-[90%] translate-x-1/4 bg-[radial-gradient(closest-side,var(--color-glow-1)_0%,var(--color-glow-2)_30%,var(--color-glow-3)_55%,var(--color-glow-4)_76%,transparent_100%)]"
                )],
                div![
                    p![class(LABEL), "Open source"],
                    h2![
                        class(
                            "mt-3 max-w-[38rem] text-[clamp(1.625rem,3.2vw,2.375rem)] leading-[1.12] font-bold tracking-[-0.035em] text-balance"
                        ),
                        "Free, and funded by the people who use it."
                    ],
                    p![
                        class("mt-4 max-w-[36rem] text-[1rem]/[1.65] text-muted"),
                        "Next Rust is licensed under MIT or Apache-2.0 and has no paid tier. Sponsorship pays for the time spent on releases, documentation and issues."
                    ],
                ],
                div![
                    class("flex flex-wrap gap-3"),
                    a![
                        class(BUTTON_PRIMARY),
                        href(SPONSOR),
                        target("_blank"),
                        rel("noopener"),
                        raw_html(HEART),
                        "Become a sponsor"
                    ],
                    a![class(BUTTON), href(REPO), target("_blank"), rel("noopener"), raw_html(STAR), "Star on GitHub"],
                ],
            ],
        ],
    ]]
}
