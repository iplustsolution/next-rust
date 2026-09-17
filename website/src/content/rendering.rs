//! The "rendering" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        h2![id("rendering-modes"), a![class("anchor"), href("#rendering-modes"), "Rendering modes"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["mode"], th!["when HTML is produced"], th!["set with"]]],
                tbody![
                    tr![
                        td![code!["static"]],
                        td!["at build time (", code!["next-rust build"], "), or on first request, then cached",],
                        td![code!["pub const RENDERING: Rendering = Rendering::Static;"]],
                    ],
                    tr![
                        td![code!["dynamic"]],
                        td!["on every request"],
                        td![code!["pub const RENDERING: Rendering = Rendering::Dynamic;"]],
                    ],
                    tr![td![code!["auto"], " (default)"], td!["decided by the build"], td!["nothing"]],
                ],
            ],
        ],
        p!["With ", code!["auto"], ", a page is ", strong!["static"], " unless one of these is true:"],
        ul![
            li![
                "it, a layout or template above it, a slot page, or a ",
                code!["metadata"],
                "/",
                code!["load"],
                " function on its path takes a request-bound argument (",
                code!["Cookies"],
                ", ",
                code!["Headers"],
                ", ",
                code!["Query"],
                ", ",
                code!["Extension"],
                ", ",
                code!["Auth"],
                ", ",
                code!["FormState"],
                ", …, see ",
                a![href("/docs/routing#pages"), "routing"],
                ");",
            ],
            li!["it has a dynamic segment and no ", code!["generate_params"], ";"],
            li!["it is an intercepting route."],
        ],
        p![
            code!["next-rust routes --layouts"],
            " and ",
            code!["next-rust build"],
            " print the reason a route is dynamic. Change the project default with ",
            code!["[rendering] default"],
            ".",
        ],
        p![
            "If a static page reads request data anyway (for example, one forced to ",
            code!["static"],
            " that takes ",
            code!["Cookies"],
            "), rendering fails with a clear error instead of silently leaking one user's data into a shared cache.",
        ],
        p![
            "In development (",
            code!["next-rust dev"],
            ") every page renders per request, so changes show up immediately.",
        ],
        h2![id("server-side-rendering"), a![class("anchor"), href("#server-side-rendering"), "Server-side rendering"],],
        p![
            "Dynamic pages render on each request. The response is ",
            code!["text/html"],
            ", marked ",
            code!["private, no-store"],
            ", with security headers and a per-request CSP nonce. Page functions can be ",
            code!["async"],
            " and await databases or APIs directly:",
        ],
        pre![code![
            class("language-rust"),
            r#"pub async fn Page(Extension(db): Extension<Db>, Path(p): Path<P>) -> Result<impl View> {
    let order = db.order(p.id).await?.or_not_found()?;
    Ok(article![h1![format!("Order {}", order.id)], OrderLines(&order)])
}"#,
        ],],
        h3![
            id("load-and-datat"),
            a![class("anchor"), href("#load-and-datat"), code!["load"], " and ", code!["Data<T>"]],
        ],
        p!["To keep data access separate from markup, export ", code!["load"], " and take ", code!["Data<T>"], ":",],
        pre![code![
            class("language-rust"),
            r"pub async fn load(Path(p): Path<P>) -> Result<Vec<Post>> {
    posts::by_author(&p.author).await
}

pub fn Page(Data(posts): Data<Vec<Post>>) -> impl View {
    ul![each(posts, |p| li![p.title])]
}",
        ],],
        p![
            "The type connection is checked at compile time. The generated code passes ",
            code!["load"],
            "'s result straight into ",
            code!["Page"],
            ". Layouts can use ",
            code!["load"],
            " the same way.",
        ],
        h2![
            id("static-generation-ssg"),
            a![class("anchor"), href("#static-generation-ssg"), "Static generation (SSG)"],
        ],
        p![
            code!["next-rust build"],
            " renders every static page once to check that it succeeds. The production binary renders them again into its in-memory page cache when it starts, before most visitors arrive, and serves them from there. Nothing is written to disk.",
        ],
        h3![id("generateparams"), a![class("anchor"), href("#generateparams"), code!["generate_params"]]],
        p!["A dynamic route is static when it lists its parameters:"],
        pre![code![
            class("language-rust"),
            r#"// app/blog/[slug]/page.rs
pub async fn generate_params() -> Vec<Params> {
    posts::all().await.iter().map(|p| Params::new().with("slug", &p.slug)).collect()
}

/// false: slugs not in the list render the 404 page.
/// true (default): they are rendered on first request, then cached.
pub const DYNAMIC_PARAMS: bool = false;"#,
        ],],
        p![
            code!["generate_params"],
            " may return ",
            code!["Result<Vec<Params>>"],
            ". Returning an error fails the build.",
        ],
        h2![
            id("incremental-static-regeneration-isr"),
            a![class("anchor"), href("#incremental-static-regeneration-isr"), "Incremental static regeneration (ISR)",],
        ],
        pre![code![
            class("language-rust"),
            r#"pub const REVALIDATE: u64 = 60;           // seconds
pub const TAGS: &[&str] = &["products"];  // for revalidate_tag"#,
        ],],
        ul![
            li!["Before the interval passes, the cached HTML is served (", code!["x-nr-cache: HIT"], ")."],
            li![
                "After it, the stale page is still served (",
                code!["STALE"],
                ") while one background task re-renders it. Concurrent requests don't start more renders.",
            ],
            li!["A failed regeneration keeps serving the stale page and logs the error."],
        ],
        p!["On-demand revalidation from an API route, server action or job:"],
        pre![code![
            class("language-rust"),
            r#"next_rust::revalidate_path("/blog/hello").await;  // one URL
next_rust::revalidate_tag("products").await;      // every page and data entry with the tag"#,
        ],],
        p![
            "Responses carry ",
            code!["cache-control: public, max-age=0, s-maxage=<revalidate>, stale-while-revalidate"],
            ", so a CDN can cache them too. Set a project-wide default with ",
            code!["[rendering] revalidate = 300"],
            ".",
        ],
        p!["Where cached pages are stored is pluggable. See ", a![href("/docs/caching"), "caching"], "."],
        h2![id("streaming"), a![class("anchor"), href("#streaming"), "Streaming"]],
        p![
            "A slow page doesn't have to block the whole response. With streaming enabled (the default), the server sends the ",
            strong!["shell"],
            " (layouts, fallbacks, everything that doesn't wait) as soon as it's ready, then streams each suspended part when it resolves.",
        ],
        h3![id("loadingrs"), a![class("anchor"), href("#loadingrs"), code!["loading.rs"]]],
        pre![code![
            class("language-rust"),
            r#"// app/dashboard/loading.rs
pub fn Loading() -> impl View { p![aria("busy", "true"), "Loading dashboard…"] }"#,
        ],],
        p![
            "It wraps everything below the dashboard layout in a suspense boundary. The layout and the fallback arrive immediately, and the page follows.",
        ],
        h3![id("suspense"), a![class("anchor"), href("#suspense"), code!["suspense"]]],
        p!["Suspend any async component:"],
        pre![code![
            class("language-rust"),
            r#"async fn Weather() -> impl View { let w = api::weather().await; p![w.summary] }

pub fn Page() -> impl View {
    div![
        h1!["Today"],
        suspense(p!["Loading weather…"], Weather()),
        suspense(p!["Loading news…"], News()),
    ]
}"#,
        ],],
        p![
            "Boundaries resolve concurrently and are flushed ",
            strong!["in completion order"],
            ". Each resolution is a ",
            code!["<template>"],
            " chunk plus a ~60-byte script call that moves it into place. The script is inlined once, only on pages that actually suspend, and carries the CSP nonce.",
        ],
        p!["Streaming facts:"],
        ul![
            li![
                "Streaming applies to ",
                strong!["dynamic"],
                " rendering. Static pages are rendered completely at build or revalidation time.",
            ],
            li![
                "Crawlers (user agents containing ",
                code!["bot"],
                ", ",
                code!["spider"],
                ", ",
                code!["crawler"],
                ", …) get fully rendered HTML in one response.",
            ],
            li![
                "Without JavaScript, fallbacks stay visible. Avoid suspense for content that must work without JavaScript.",
            ],
            li![
                "The status code and headers are sent with the shell. A ",
                code!["not_found()"],
                " or error thrown inside streamed content renders the nearest boundary in place. A redirect becomes a client-side ",
                code!["location.replace"],
                ".",
            ],
            li!["Disable streaming with ", code!["[rendering] streaming = false"], "."],
        ],
        h2![id("errors"), a![class("anchor"), href("#errors"), "Errors"]],
        pre![code![
            class("language-rust"),
            r"pub async fn Page(Path(p): Path<P>) -> Result<impl View> {
    let order = db::order(p.id).await?;        // any std::error::Error works with `?`
    Ok(OrderView(order))
}",
        ],],
        ul![
            li![
                code!["error.rs"],
                " catches errors from the pages and layouts ",
                strong!["below"],
                " its segment, including the page in the same directory, but not errors from its own segment's ",
                code!["layout.rs"],
                ". The layouts above it still render.",
            ],
            li![
                code!["ErrorInfo { status, message, digest }"],
                ": ",
                code!["message"],
                " is the full error in development and a generic message in production. ",
                code!["digest"],
                " is written to the server log so you can correlate reports.",
            ],
            li![
                "Without a boundary, ",
                code!["global-error.rs"],
                " or the built-in error page renders with status 500. In development the built-in page shows the error and the source file.",
            ],
            li!["Errors never crash the process. Each request is isolated."],
        ],
        h2![id("not-found"), a![class("anchor"), href("#not-found"), "Not found"]],
        pre![code![
            class("language-rust"),
            r"return Err(not_found());                         // or:
let user = db::user(id).await?.or_not_found()?;  // Option → 404",
        ],],
        p![
            code!["not_found()"],
            " renders the nearest ",
            code!["not-found.rs"],
            " inside the layouts above it, with status 404. Unmatched URLs render the root ",
            code!["app/not-found.rs"],
            ", or a built-in page. Requests that don't accept HTML, or that fall under the API prefix, get a plain ",
            code!["404 Not Found"],
            ".",
        ],
        h2![id("redirects"), a![class("anchor"), href("#redirects"), "Redirects"]],
        pre![code![
            class("language-rust"),
            r#"return Err(redirect("/login"));             // 307
return Err(permanent_redirect("/new-url")); // 308"#,
        ],],
        p![
            "These work in pages, layouts, ",
            code!["metadata"],
            ", ",
            code!["load"],
            " and server actions. They're also available as ",
            code!["Response::redirect(..)"],
            " in middleware and API routes, and through configuration:",
        ],
        pre![code![
            class("language-toml"),
            r#"[[redirects]]
source = "/old-blog/:slug"
destination = "/blog/:slug"
permanent = true"#,
        ],],
        p![
            "Trailing slashes are normalized with a 308. ",
            code!["/about/"],
            " redirects to ",
            code!["/about"],
            " unless you set ",
            code!["[app] trailing_slash = true"],
            ".",
        ],
        h2![id("metadata"), a![class("anchor"), href("#metadata"), "Metadata"]],
        p![
            "Export ",
            code!["metadata"],
            " from ",
            code!["layout.rs"],
            ", ",
            code!["page.rs"],
            " or ",
            code!["metadata.rs"],
            ":",
        ],
        pre![code![
            class("language-rust"),
            r##"pub fn metadata() -> Metadata {
    Metadata::new()
        .title("Acme")
        .title_template("%s · Acme")          // applied to titles below this segment
        .description("Tools for builders")
        .keywords(["tools", "rust"])
        .canonical("https://acme.dev/")
        .theme_color("#111827")
        .icon("/favicon.ico")
        .manifest("/site.webmanifest")
        .open_graph(OpenGraph {
            site_name: Some("Acme".into()),
            images: vec![OgImage { url: "https://acme.dev/og.png".into(), width: Some(1200), height: Some(630), ..Default::default() }],
            ..Default::default()
        })
        .twitter(Twitter { card: Some("summary_large_image".into()), ..Default::default() })
}"##,
        ],],
        p![
            "Metadata is merged from the root layout down to the page. Every field set by a child overrides the parent. A title set below a ",
            code!["title_template"],
            " is formatted with it. ",
            code!["absolute_title(..)"],
            " ignores templates. Metadata functions can be ",
            code!["async"],
            ", take extractors, and return ",
            code!["Result"],
            ". ",
            code!["not_found()"],
            " from metadata renders the 404 page.",
        ],
        p![
            "All values are HTML-escaped. ",
            code!["canonical"],
            " and link URLs with a ",
            code!["javascript:"],
            " scheme are neutralized.",
        ],
        h3![id("generated-seo-files"), a![class("anchor"), href("#generated-seo-files"), "Generated SEO files"],],
        pre![code![
            class("language-rust"),
            r#"// app/sitemap.rs
pub async fn sitemap() -> Sitemap {
    Sitemap::new().url("https://acme.dev/").url("https://acme.dev/pricing")
}

// app/robots.rs
pub fn robots() -> Robots {
    Robots::allow_all().sitemap("https://acme.dev/sitemap.xml")
}"#,
        ],],
        p![
            "Static files like ",
            code!["public/robots.txt"],
            ", ",
            code!["public/favicon.ico"],
            " and ",
            code!["public/og.png"],
            " work too.",
        ],
    ]
}
