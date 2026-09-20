//! The "introduction" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        div![
            class("callout"),
            p![
                strong!["In plain words."],
                " Next Rust is a kit for building websites. You describe each page in a small text file, you put that file in a folder, and the folder's name becomes the page's address: a file in ",
                code!["app/about/"],
                " is the page ",
                code!["/about"],
                ". When you are done, the kit packs the whole site into one program file. Copy that file to a computer on the internet, start it, and the site is live. Nothing else has to be installed there.",
            ],
        ],
        p![
            "Next Rust is a full-stack web framework for Rust that works the way Next.js does. Folders in ",
            code!["app/"],
            " become routes, layouts nest, pages render on the server and stream to the browser, and the whole app ships as one native binary.",
        ],
        p![
            "If you've built with the Next.js App Router, most of this will feel familiar. The difference is that your pages, data loading, API routes and server actions are plain Rust functions, checked by the compiler before anything runs. ",
            a![href("/docs/coming-from-nextjs"), "Coming from Next.js"],
            " maps one onto the other.",
        ],
        p![
            "A few words used on every page of these docs. A ",
            strong!["route"],
            " is an address your site answers, such as ",
            code!["/about"],
            ". ",
            strong!["Server rendering"],
            " means the finished HTML of a page is produced on the server, the program that answers the browser, so visitors and search engines receive a complete page and not an empty shell that scripts fill in later. ",
            strong!["Streaming"],
            " means a page is sent in pieces as they become ready, so the first part shows while slow data is still loading. A ",
            strong!["binary"],
            " is a program as one runnable file. An ",
            strong!["island"],
            " is a small interactive part of an otherwise static page, and ",
            strong!["hydration"],
            " is the moment a script in the browser takes over that part and makes it respond to clicks.",
        ],
        p![
            strong!["Why it matters:"],
            " most web stacks need a language runtime, a bundler and a folder of files on the server, and find mistakes when a visitor hits them. Here the compiler checks every page, data loader and form handler before anything runs, and the result is a single file.",
        ],
        p![
            strong!["In a hurry?"],
            " ",
            a![href("/docs/getting-started"), "Getting started"],
            " goes from an empty folder to a running app in about five minutes.",
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
                    "Pages ship no JavaScript unless they use links or islands. The client runtime is about 4 KB gzipped.",
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
        p![
            "A page is a function that returns a view (a description of HTML). Put it in a folder and it has a URL. This file, saved as ",
            code!["app/about/page.rs"],
            ", creates the page ",
            code!["/about"],
            " with a heading, a paragraph and a link, and sets the title shown in the browser tab:",
        ],
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
            " function named ",
            code!["load"],
            " in the same file as the page, and its result is passed in already typed. This pair, in a file such as ",
            code!["app/blog/page.rs"],
            ", fetches the posts first and then renders them as a list:",
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
        p![
            "Not sure which file does what? ",
            a![href("/docs/project-structure"), "Project structure"],
            " explains every file and folder of a project, in everyday words first.",
        ],
        h2![id("where-to-go-next"), a![class("anchor"), href("#where-to-go-next"), "Where to go next"]],
        div![
            class("card-grid"),
            a![
                class("card"),
                href("/docs/getting-started"),
                strong!["Getting started"],
                span!["Install the CLI, create an app and build the production binary in about five minutes."],
            ],
            a![
                class("card"),
                href("/docs/project-structure"),
                strong!["Project structure"],
                span!["What every file and folder is for, in plain words, and which one to open for a given change."],
            ],
            a![
                class("card"),
                href("/docs/coming-from-nextjs"),
                strong!["Coming from Next.js"],
                span!["Every App Router concept next to its Next Rust equivalent."],
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
            a![
                class("card"),
                href("/docs/server-actions"),
                strong!["Server actions & forms"],
                span!["Forms that call typed Rust functions, with validation and redirects."],
            ],
            a![
                class("card"),
                href("/components"),
                strong!["Component gallery"],
                span!["Every built-in UI component, rendered live: buttons, fields, dialogs, tables, charts."],
            ],
            a![
                class("card"),
                href("/icons"),
                strong!["Icons"],
                span!["Browse and search the built-in icon set, and copy the Rust name of an icon."],
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
