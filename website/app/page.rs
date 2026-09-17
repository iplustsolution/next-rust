use next_rust::prelude::*;
use crate::highlight::highlight;
use crate::ui::{ACTIONS, BUTTON, BUTTON_PRIMARY, REPO, SYNTAX, VERSION};

pub fn metadata() -> Metadata {
    Metadata::new().absolute_title("Next Rust · The full-stack web framework for Rust")
}

const INSTALL: &str = "cargo install next-rust-cli";

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

const FEATURES: &[(&str, &str, &str)] = &[
    (
        "Folders are routes",
        "page.rs, layout.rs, loading.rs and error.rs work like the App Router. Groups, dynamic segments, parallel and intercepting routes included.",
        r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>"#,
    ),
    (
        "Streaming by default",
        "Layouts reach the browser first while slow data keeps loading. loading.rs becomes the fallback without extra code.",
        r#"<path d="M4 6h16M4 12h10M4 18h6"/>"#,
    ),
    (
        "Static, dynamic or both",
        "Each route is pre-rendered or server-rendered based on what it reads. Add REVALIDATE and pages refresh in the background.",
        r#"<path d="M21 12a9 9 0 1 1-3-6.7"/><path d="M21 4v5h-5"/>"#,
    ),
    (
        "Server actions",
        "Forms post to typed Rust functions. Validation errors come back to the right field, with CSRF checks built in.",
        r#"<path d="M4 4h16v12H5.2L4 17.2Z"/><path d="M8 9h8M8 12h5"/>"#,
    ),
    (
        "Almost no JavaScript",
        "Pages send zero JavaScript until they use a link or an island. The runtime that handles both is about 4 KB.",
        r#"<path d="m13 2-9 12h8l-1 8 9-12h-8Z"/>"#,
    ),
    (
        "One file in production",
        "next-rust build writes a single stripped binary with your pages, config and public files inside. Copy it and run it.",
        r#"<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M9 9h6v6H9Z"/>"#,
    ),
];

const SECTION: &str = "mx-auto max-w-[1120px] border-t border-line px-6 py-22 max-[640px]:px-4 max-[640px]:py-16";
const SECTION_TITLE: &str = "mx-auto mb-12 max-w-[720px] text-center text-[clamp(28px,4vw,40px)] leading-[1.15] font-bold tracking-[-0.03em] text-balance";
const PANEL: &str = "rounded-xl border border-line bg-code";

pub fn Page() -> impl View {
    let tree_item = |indent: &'static str, text: &'static str| li![class(indent), text];
    let tree_dir = |indent: &'static str, text: &'static str| li![class(indent), span![class("text-fg"), text]];
    let route = |text: &'static str| span![class("ml-2.5 text-[11.5px] text-muted"), text];
    fragment![
        main![
            id("content"),
            class("overflow-hidden"),
            section![
                class("relative isolate mx-auto max-w-[880px] px-6 pt-26 pb-18 text-center max-[640px]:px-5 max-[640px]:pt-18 max-[640px]:pb-14"),
                class("before:pointer-events-none before:absolute before:inset-x-[-40%] before:-top-30 before:-z-10 before:h-[520px] before:bg-[radial-gradient(closest-side,var(--color-accent-soft),transparent)] before:content-['']"),
                a![
                    class("inline-flex items-center gap-2.5 rounded-full border border-line-strong bg-soft py-[5px] pr-3.5 pl-2.5 text-[13px] font-medium text-fg-soft transition-colors hover:border-muted"),
                    href("/docs/status"),
                    span![class("size-[7px] rounded-full bg-accent ring-3 ring-accent/15")],
                    format!("v{VERSION}"),
                    span![class("h-3 w-px bg-line-strong")],
                    "See what's ready",
                ],
                h1![
                    class("mt-7 text-[clamp(40px,7vw,72px)] leading-[1.05] font-bold tracking-[-0.035em] text-balance"),
                    "The full-stack framework for ",
                    span![class("text-accent-fg"), "Rust"],
                    "."
                ],
                p![
                    class("mx-auto mt-6 max-w-[640px] text-[clamp(17px,2.2vw,19px)] leading-[1.65] text-muted"),
                    "Filesystem routing, streaming server rendering and server actions, compiled into one small binary. If you know the Next.js App Router, you already know how it works."
                ],
                div![
                    class(ACTIONS),
                    a![class(BUTTON_PRIMARY), href("/docs/getting-started"), "Get started"],
                    a![class(BUTTON), href("/docs"), "Read the docs"],
                ],
                div![
                    class(PANEL),
                    class("mx-auto mt-9 flex w-fit max-w-full items-center gap-3 overflow-x-auto px-[18px] py-3 text-[13.5px] whitespace-nowrap max-[640px]:w-full"),
                    span![class("font-mono text-accent-fg select-none"), "$"],
                    code![class("font-mono text-fg-soft"), INSTALL],
                ],
            ],
            section![
                class("mx-auto grid max-w-[1040px] grid-cols-[280px_minmax(0,1fr)] gap-4 px-6 pb-24 max-[960px]:grid-cols-1 max-[640px]:px-4 max-[640px]:pb-18"),
                div![
                    class(PANEL),
                    class("px-5 py-[18px] font-mono text-[13.5px]/[1.9]"),
                    p![class("mb-1.5 text-xs/[1.9] tracking-[0.08em] text-muted uppercase"), "my-app"],
                    ul![
                        class("text-fg-soft"),
                        tree_dir("", "app/"),
                        tree_item("pl-[18px]", "layout.rs"),
                        tree_item("pl-[18px]", "page.rs"),
                        tree_dir("pl-[18px]", "blog/[slug]/"),
                        li![class("-mx-2.5 rounded-md bg-accent/10 pr-2.5 pl-[46px] text-accent-fg"), "page.rs", route("/blog/:slug")],
                        tree_item("pl-9", "loading.rs"),
                        tree_dir("pl-[18px]", "api/hello/"),
                        li![class("pl-9"), "route.rs", route("GET /api/hello")],
                        tree_dir("", "public/"),
                        tree_item("", "next-rust.toml"),
                    ],
                ],
                div![
                    class(PANEL),
                    class("min-w-0 overflow-hidden"),
                    div![
                        class("flex h-[42px] items-center gap-[7px] border-b border-line px-4"),
                        span![class("size-[11px] rounded-full bg-line-strong")],
                        span![class("size-[11px] rounded-full bg-line-strong")],
                        span![class("size-[11px] rounded-full bg-line-strong")],
                        p![class("ml-2.5 font-mono text-[12.5px] text-muted"), "app/blog/[slug]/page.rs"],
                    ],
                    pre![
                        class("overflow-x-auto px-[22px] py-5 font-mono text-[13.5px]/[1.75]"),
                        class(SYNTAX),
                        code![raw_html(highlight("rust", PAGE_EXAMPLE))]
                    ],
                ],
            ],
            section![
                class(SECTION),
                h2![class(SECTION_TITLE), "Everything a web app needs, nothing it doesn't"],
                div![
                    class("grid grid-cols-3 gap-px overflow-hidden rounded-2xl border border-line bg-line max-[960px]:grid-cols-2 max-[640px]:grid-cols-1"),
                    each(FEATURES, |(title, text, icon)| div![
                        class("bg-bg px-7 py-[30px]"),
                        span![
                            class("block text-accent-fg"),
                            raw_html(format!(
                                r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{icon}</svg>"#
                            ))
                        ],
                        h3![class("mt-[18px] mb-2 text-[17px] font-[650] tracking-[-0.015em]"), *title],
                        p![class("text-[15px]/[1.6] text-muted"), *text],
                    ]),
                ],
            ],
            section![
                class(SECTION),
                h2![class(SECTION_TITLE), "From an empty folder to production"],
                ol![
                    class("grid grid-cols-3 gap-5 max-[960px]:grid-cols-2 max-[640px]:grid-cols-1"),
                    each(
                        [
                            ("1", "Create", "next-rust new my-app", "A working app with a layout, a page and a styled hero."),
                            ("2", "Develop", "next-rust dev", "Live reload, an error overlay, and new files filled with starter code."),
                            ("3", "Ship", "next-rust build", "One stripped binary, about 1.2 MB for a new app. Nothing else to upload."),
                        ],
                        |(num, title, command, text)| li![
                            class("rounded-2xl border border-line bg-soft p-[26px]"),
                            span![
                                class("grid size-7 place-items-center rounded-full border border-line-strong font-mono text-[13px] font-semibold text-muted"),
                                num
                            ],
                            h3![class("mt-4 mb-3 text-lg tracking-[-0.015em]"), title],
                            pre![
                                class(PANEL),
                                class("overflow-x-auto rounded-[9px] px-3.5 py-[11px] font-mono text-[13.5px]"),
                                class(SYNTAX),
                                code![raw_html(highlight("sh", command))]
                            ],
                            p![class("mt-3.5 text-[15px] text-muted"), text],
                        ]
                    ),
                ],
            ],
            section![
                class("mx-auto mb-24 max-w-[1120px] rounded-[20px] border border-line bg-soft bg-[radial-gradient(60%_120%_at_50%_0%,var(--color-accent-soft),transparent_70%)] px-6 py-18 text-center max-[1136px]:mx-4 max-[640px]:mb-18 max-[640px]:px-5 max-[640px]:py-14"),
                h2![class("text-[clamp(28px,4vw,40px)] font-bold tracking-[-0.035em]"), "Start building"],
                p![class("mt-3 text-[17px] text-muted"), "Install the CLI and have a server running in a couple of minutes."],
                div![
                    class(ACTIONS),
                    a![class(BUTTON_PRIMARY), href("/docs/getting-started"), "Installation guide"],
                    a![class(BUTTON), href(REPO), target("_blank"), rel("noopener"), "Star on GitHub"],
                ],
            ],
        ],
    ]
}
