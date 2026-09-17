use next_rust::prelude::*;
use crate::highlight::highlight;
use crate::ui::{self, Area, REPO, VERSION};

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
        "Pages send zero JavaScript until they use a link or an island. The runtime that handles both is about 3 KB.",
        r#"<path d="m13 2-9 12h8l-1 8 9-12h-8Z"/>"#,
    ),
    (
        "One file in production",
        "next-rust build writes a single stripped binary with your pages, config and public files inside. Copy it and run it.",
        r#"<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M9 9h6v6H9Z"/>"#,
    ),
];

pub fn Page() -> impl View {
    fragment![
        ui::header(Area::Home, ""),
        main![
            id("content"),
            class("home"),
            section![
                class("hero"),
                a![
                    class("pill"),
                    href("/docs/status"),
                    span![class("pill-dot")],
                    format!("v{VERSION}"),
                    span![class("pill-sep")],
                    "See what's ready",
                ],
                h1!["The full-stack framework for ", span![class("accent"), "Rust"], "."],
                p![
                    class("hero-copy"),
                    "Filesystem routing, streaming server rendering and server actions, compiled into one small binary. If you know the Next.js App Router, you already know how it works."
                ],
                div![
                    class("hero-actions"),
                    a![class("button primary"), href("/docs/getting-started"), "Get started"],
                    a![class("button"), href("/docs"), "Read the docs"],
                ],
                div![
                    class("install"),
                    span![class("prompt"), "$"],
                    code![INSTALL],
                ],
            ],
            section![
                class("showcase"),
                div![
                    class("tree"),
                    p![class("tree-title"), "my-app"],
                    ul![
                        li![span![class("dir"), "app/"]],
                        li![class("indent"), "layout.rs"],
                        li![class("indent"), "page.rs"],
                        li![class("indent"), span![class("dir"), "blog/[slug]/"]],
                        li![class("indent-2 current"), "page.rs", span![class("route"), "/blog/:slug"]],
                        li![class("indent-2"), "loading.rs"],
                        li![class("indent"), span![class("dir"), "api/hello/"]],
                        li![class("indent-2"), "route.rs", span![class("route"), "GET /api/hello"]],
                        li![span![class("dir"), "public/"]],
                        li!["next-rust.toml"],
                    ],
                ],
                div![
                    class("window"),
                    div![class("window-bar"), span![], span![], span![], p!["app/blog/[slug]/page.rs"]],
                    pre![code![raw_html(highlight("rust", PAGE_EXAMPLE))]],
                ],
            ],
            section![
                class("features"),
                h2![class("section-title"), "Everything a web app needs, nothing it doesn't"],
                div![
                    class("feature-grid"),
                    each(FEATURES, |(title, text, icon)| div![
                        class("feature"),
                        raw_html(format!(
                            r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{icon}</svg>"#
                        )),
                        h3![*title],
                        p![*text],
                    ]),
                ],
            ],
            section![
                class("steps"),
                h2![class("section-title"), "From an empty folder to production"],
                ol![
                    li![
                        span![class("step-num"), "1"],
                        h3!["Create"],
                        pre![code![raw_html(highlight("sh", "next-rust new my-app"))]],
                        p!["A working app with a layout, a page and a styled hero."],
                    ],
                    li![
                        span![class("step-num"), "2"],
                        h3!["Develop"],
                        pre![code![raw_html(highlight("sh", "next-rust dev"))]],
                        p!["Live reload, an error overlay, and new files filled with starter code."],
                    ],
                    li![
                        span![class("step-num"), "3"],
                        h3!["Ship"],
                        pre![code![raw_html(highlight("sh", "next-rust build"))]],
                        p!["One stripped binary, about 2 MB for a new app. Nothing else to upload."],
                    ],
                ],
            ],
            section![
                class("cta"),
                h2!["Start building"],
                p!["Install the CLI and have a server running in a couple of minutes."],
                div![
                    class("hero-actions"),
                    a![class("button primary"), href("/docs/getting-started"), "Installation guide"],
                    a![class("button"), href(REPO), target("_blank"), rel("noopener"), "Star on GitHub"],
                ],
            ],
        ],
        ui::footer(),
    ]
}
