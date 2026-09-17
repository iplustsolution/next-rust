//! The "introduction" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Next Rust is a full-stack web framework for Rust that works the way Next.js does. Folders in ",
            code!["app/"],
            " become routes, layouts nest, pages render on the server and stream to the browser, and the whole app ships as one native binary.",
        ],
        p![
            "If you've built with the Next.js App Router, most of this will feel familiar. The difference is that your pages, data loading, API routes and server actions are plain Rust functions, checked by the compiler before anything runs.",
        ],
        h2![id("what-you-get"), a![class("anchor"), href("#what-you-get"), "What you get"]],
        div![
            class("feature-list"),
            div![
                strong!["Filesystem routing"],
                span![
                    code!["page.rs"],
                    ", ",
                    code!["layout.rs"],
                    ", ",
                    code!["loading.rs"],
                    ", ",
                    code!["error.rs"],
                    ", route groups, dynamic and catch-all segments, parallel and intercepting routes.",
                ],
            ],
            div![
                strong!["Rendering on your terms"],
                span![
                    "Server rendering with streaming, static generation, and incremental regeneration. The mode is inferred from your code.",
                ],
            ],
            div![
                strong!["Server actions"],
                span![
                    "Forms that post to typed Rust functions, with validation errors, CSRF protection and progressive enhancement.",
                ],
            ],
            div![
                strong!["Very little JavaScript"],
                span![
                    "Pages ship no JavaScript unless they use links or islands. The client runtime is about 3 KB gzipped.",
                ],
            ],
            div![
                strong!["One file to deploy"],
                span![
                    code!["next-rust build"],
                    " writes a single stripped binary that carries your pages, config and static files.",
                ],
            ],
            div![
                strong!["Errors at build time"],
                span![
                    "Wrong exports, duplicate routes and bad extractor types fail the build with a clear diagnostic.",
                ],
            ],
        ],
        h2![id("a-first-look"), a![class("anchor"), href("#a-first-look"), "A first look"]],
        p!["A page is a function that returns a view. Put it in a folder and it has a URL:"],
        pre![code![
            class("language-rust"),
            r#"// app/about/page.rs  →  /about
use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("About")
}

pub fn Page() -> impl View {
    main![
        h1!["About us"],
        p!["We build fast things."],
        a![href("/"), "Back home"],
    ]
}"#,
        ],],
        p![
            "Data loading is an ",
            code!["async"],
            " function next to the page, and its result is passed in already typed:",
        ],
        pre![code![
            class("language-rust"),
            r"pub async fn load() -> Result<Vec<Post>> {
    db::posts().await
}

pub fn Page(Data(posts): Data<Vec<Post>>) -> impl View {
    ul![each(posts, |post| li![post.title])]
}",
        ],],
        h2![id("where-to-go-next"), a![class("anchor"), href("#where-to-go-next"), "Where to go next"]],
        div![
            class("card-grid"),
            a![
                class("card"),
                href("/docs/getting-started"),
                strong!["Getting started"],
                span!["Install the CLI and create your first app in a couple of minutes."],
            ],
            a![
                class("card"),
                href("/docs/routing"),
                strong!["Routing"],
                span!["Special files, layouts, dynamic segments and how routes are ranked."],
            ],
            a![
                class("card"),
                href("/docs/rendering"),
                strong!["Rendering & data"],
                span!["SSR, static generation, ISR, streaming and metadata."],
            ],
            a![
                class("card"),
                href("/docs/deployment"),
                strong!["Deployment"],
                span!["Ship the single binary to a VPS, Docker or any container platform."],
            ],
        ],
        div![
            class("callout"),
            p![
                "Next Rust is young. ",
                a![href("/docs/status"), "Status & roadmap"],
                " lists what is finished, what has known limits, and what isn't built yet.",
            ],
        ],
    ]
}
