//! The "rendering" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "Rendering is turning your Rust code into the HTML a browser can show. The only real question is when \
             that happens. A restaurant can cook a dish ahead and keep it ready (static: fastest, the same for \
             everyone), cook it when it is ordered (dynamic: always fresh, can be personal), or keep the dish ready \
             and add the garnish at the table (partial). Next Rust picks the fastest option that is still correct \
             for each page, and this page explains how it decides and how to overrule it."
        ),
        p!["Words used on this page:"],
        ul![
            li![
                strong!["SSR"],
                " (server-side rendering): the server builds the finished HTML and sends it, instead of sending an \
                 empty page that JavaScript fills in. Every Next Rust page is server-rendered; the modes below only \
                 differ in when.",
            ],
            li![
                strong!["SSG"],
                " (static site generation): rendering a page ahead of time, once, and serving that copy to everybody.",
            ],
            li![
                strong!["ISR"],
                " (incremental static regeneration): a static page that is rendered again in the background every so \
                 many seconds, so it is fast and never very old.",
            ],
            li![
                strong!["Streaming"],
                ": sending the page in pieces as they become ready, so the visitor sees the frame at once and slow \
                 parts fill in.",
            ],
            li![
                strong!["Request-bound"],
                ": anything that belongs to one visitor's request, such as cookies, headers, the query string (the ",
                code!["?page=2"],
                " part of a URL) or the signed-in user.",
            ],
            li![
                strong!["Dynamic segment"],
                ": a folder in square brackets, such as ",
                code!["app/blog/[slug]/"],
                ", that matches many URLs.",
            ],
            li![strong!["CDN"], ": servers around the world that keep copies of your pages close to visitors.",],
        ],
        h2![id("rendering-modes"), a![class("anchor"), href("#rendering-modes"), "Rendering modes"]],
        p![
            "Most projects never set a mode: ",
            code!["auto"],
            " makes every page static that can be, and dynamic otherwise. To overrule it for one page, write the \
             constant from the last column at the top level of that page's ",
            code!["page.rs"],
            ".",
        ],
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
                    tr![
                        td![code!["partial"]],
                        td![
                            "a static shell as above; the ",
                            a![href("#partial-prerendering"), code!["dynamic"], " holes"],
                            " in it on every request",
                        ],
                        td![
                            code!["pub const RENDERING: Rendering = Rendering::Partial;"],
                            ", or just a hole in a static page",
                        ],
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
            "The reasoning behind the first rule: a page that reads a visitor's cookie may look different for each \
             visitor, so one stored copy cannot be right for everyone. ",
            code!["Params"],
            ", ",
            code!["Path"],
            " and ",
            code!["Nonce"],
            " are the only arguments that do not depend on the visitor.",
        ],
        p![
            code!["next-rust routes --layouts"],
            " and ",
            code!["next-rust build"],
            " print the reason a route is dynamic. Change the project default with ",
            code!["[rendering] default"],
            " in ",
            code!["next-rust.toml"],
            ", the project's settings file in its root folder.",
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
            " and await databases or APIs directly. (A CSP nonce is a one-time password the server puts on its own \
             inline scripts so the browser can refuse injected ones; see ",
            a![href("/docs/security"), "Security"],
            ".) This page, for example ",
            code!["app/orders/[id]/page.rs"],
            ", takes the database handle that middleware attached to the request, looks up the order from the URL, \
             and answers 404 when it does not exist:",
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
        p![
            "To keep data access separate from markup, export ",
            code!["load"],
            " and take ",
            code!["Data<T>"],
            ". ",
            code!["load"],
            " fetches, ",
            code!["Page"],
            " draws. Both functions go in the same ",
            code!["page.rs"],
            ", here ",
            code!["app/authors/[author]/page.rs"],
            ":",
        ],
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
        div![
            class("callout"),
            p![
                strong!["Two rules for ", code!["load"], "."],
                " It must return a ",
                code!["Result"],
                " (a bare value does not compile), and it must be in the same file as the ",
                code!["Page"],
                " or ",
                code!["Layout"],
                " that takes ",
                code!["Data<T>"],
                ". A ",
                code!["load"],
                " in another module is never called.",
            ],
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
        p![
            "A folder like ",
            code!["app/blog/[slug]/"],
            " stands for many pages, and the build cannot guess which ones exist. ",
            code!["generate_params"],
            " tells it: return one ",
            code!["Params"],
            " per page to prepare. Without it the route is rendered per request. A dynamic route is static when it \
             lists its parameters, in its own ",
            code!["page.rs"],
            ":",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/blog/[slug]/page.rs
pub async fn generate_params() -> Vec<Params> {
    posts::all().await.iter().map(|p| Params::new().with("slug", p.slug.as_str())).collect()
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
        p![
            "A purely static page is as old as the last deploy. ISR gives it a shelf life: after that many seconds \
             the next visit triggers a fresh render in the background, and nobody waits for it. Add these two \
             constants to the top level of a static page's ",
            code!["page.rs"],
            ". The first sets the shelf life; the second labels the page so it can be refreshed on demand:",
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
        p!["On-demand revalidation from an API route, server action or job: call one of these right after the data \
             changes (a post was edited, a price updated) and the next visitor gets a new page without waiting for \
             the interval:",],
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
        h2![id("partial-prerendering"), a![class("anchor"), href("#partial-prerendering"), "Partial prerendering"],],
        p![
            "Partial prerendering (PPR) is the garnish-at-the-table mode. Most pages are static except for a corner: \
             the cart count, the signed-in name, a price for this visitor's region. A ",
            code!["dynamic"],
            " hole keeps that corner out of an otherwise static page, so the page stays cached and the \
             corner stays personal. In this product page (",
            code!["app/products/[slug]/page.rs"],
            ") the product is fetched once for everybody, and only the cart line runs for each visitor. The first \
             argument of ",
            code!["dynamic"],
            " is the fallback, shown until the real content arrives:",
        ],
        pre![code![
            class("language-rust"),
            r#"pub const REVALIDATE: u64 = 3600;
pub const TAGS: &[&str] = &["products"];

pub async fn Page(Path(p): Path<P>) -> Result<impl View> {
    let product = db::product(&p.slug).await?;             // once, for the shell
    Ok(article![
        h1![product.name.clone()],
        dynamic(p!["Checking your cart…"], move |ctx| {    // on every request
            let sku = product.sku.clone();                 // captured from the static render
            async move {
                let cart = cart::load(ctx.cookies.get("session")).await?;
                Ok::<_, Error>(p![format!("{} in your cart", cart.count(&sku))])
            }
        }),
    ])
}"#,
        ],],
        ul![
            li![
                strong!["The shell"],
                " is everything outside the holes, with each hole's fallback in place. It is rendered at \
                 build time or on the first request and cached by exactly the rules of a static page: ",
                code!["REVALIDATE"],
                ", ",
                code!["TAGS"],
                ", ",
                code!["revalidate_path"],
                "/",
                code!["revalidate_tag"],
                ", stale-while-revalidate, ",
                code!["generate_params"],
                ".",
            ],
            li![
                strong!["A request"],
                " is answered with the shell's bytes at once. The holes then run concurrently, with the \
                 real request, and each one is streamed into the same response when it is done, through the \
                 same ",
                code!["<template>"],
                " swap as ",
                a![href("#suspense"), code!["suspense"]],
                ". A page with a 300 ms hole has the time to first byte of a static page.",
            ],
            li![
                strong!["A hole"],
                " gets the request's context: ",
                code!["ctx.cookies"],
                ", ",
                code!["ctx.headers"],
                ", ",
                code!["ctx.uri"],
                ", ",
                code!["ctx.params"],
                ", and any extractor through ",
                code!["ctx.extract::<Query<Filter>>()?"],
                " or ",
                code!["ctx.extract::<Auth<User>>()?"],
                ". It returns a view or a ",
                code!["Result"],
                " of one. Whatever the closure captures comes from the static render and is shared by \
                 every visitor.",
            ],
            li![
                "Holes work in pages, layouts, templates and components. A hole in a layout is not run \
                 again by a client-side navigation that keeps the layout on screen. On a ",
                code!["dynamic"],
                " page, and in development, a hole simply behaves like ",
                code!["suspense"],
                ".",
            ],
        ],
        p![
            "There is nothing to switch on. A static page that renders a hole is served this way, wherever \
             the hole comes from; ",
            code!["next-rust routes"],
            " and ",
            code!["next-rust build"],
            " list such pages as ",
            code!["◐ partial"],
            ". Writing ",
            code!["Rendering::Partial"],
            " states the intent, and makes the build fail (",
            code!["NR0212"],
            ") when the page, a layout above it, ",
            code!["load"],
            " or ",
            code!["metadata"],
            " takes a request-bound argument, instead of quietly rendering the page per request.",
        ],
        h3![id("partial-guarantees"), a![class("anchor"), href("#partial-guarantees"), "What is guaranteed"]],
        ul![
            li![
                strong!["No visitor data in the shell."],
                " The shell is rendered without a request. Reading cookies, headers or the query outside a \
                 hole fails the render instead of caching one visitor's data for everyone.",
            ],
            li![
                strong!["Never stored downstream."],
                " A response with holes is sent ",
                code!["cache-control: private, no-store"],
                " with ",
                code!["x-nr-prerender: partial"],
                "; ",
                code!["x-nr-cache"],
                " reports the shell (",
                code!["HIT"],
                ", ",
                code!["MISS"],
                ", ",
                code!["STALE"],
                "). The shell is cached on the server only. A static page without holes keeps its public, CDN-friendly headers.",
            ],
            li![
                strong!["Errors stay in their hole."],
                " An error (or a panic) in a hole renders the page's nearest ",
                code!["error.rs"],
                " in place of it, ",
                code!["not_found()"],
                " the nearest ",
                code!["not-found.rs"],
                ", a redirect becomes ",
                code!["location.replace"],
                ". The other holes and the rest of the stream are not affected. For an error view of its own: ",
                code!["Dynamic::new(fallback).on_error(|info| p![info.message]).render(|ctx| ..)"],
                ".",
            ],
            li![
                strong!["The head is sent before holes run."],
                " A hole cannot set a cookie, a response header or the status; the server logs a warning \
                 when one tries. Do that in middleware, a server action or a route handler. (This includes ",
                code!["CsrfToken"],
                " creating its cookie: have it set before the form is shown, for example by the ",
                code!["csrf()"],
                " middleware.)",
            ],
        ],
        h3![
            id("partial-without-javascript"),
            a![class("anchor"), href("#partial-without-javascript"), "Without JavaScript",],
        ],
        p![
            "Moving a streamed hole into place takes the same inline script as suspense (about 300 bytes, \
             sent once with the shell, with the CSP nonce); HTML has no declarative way to fill an earlier \
             part of the document. Without JavaScript the fallback stays, so write fallbacks that make sense \
             on their own. Where the holes must be in the HTML itself, they are: crawlers, and everyone when ",
            code!["[rendering] streaming = false"],
            ", get one complete document with every hole rendered in place. The shell still comes from the \
             cache; only the time to first byte becomes that of the slowest hole.",
        ],
        h3![id("cached-fragments"), a![class("anchor"), href("#cached-fragments"), "Cached fragments"]],
        p![
            code!["cached"],
            " caches the HTML of a part of a page under a key, with a lifetime and tags of its own. Use it inside \
             any async page, layout or hole. Here the list of reviews for one product is rendered at most once every \
             300 seconds, however many visitors ask:",
        ],
        pre![code![
            class("language-rust"),
            r#"use next_rust::CacheOptions;

let reviews = cached(&format!("reviews:{sku}"), CacheOptions::revalidate(300).tag("reviews"), || async {
    let reviews = db::reviews(&sku).await?;
    Ok::<_, Error>(ul![each(reviews, |r| li![r.text])])
})
.await?;"#,
        ],],
        ul![
            li![
                strong!["In a hole"],
                ", the expensive, shared part of per-request content is rendered once: the hole reads the \
                 visitor's cookie, the fragment inside it comes from the cache.",
            ],
            li![
                strong!["In a shell"],
                " (or any static page), the page is cached under the fragment's tags as well and for no \
                 longer than the fragment lives. ",
                code!["revalidate_tag(\"reviews\")"],
                " renders the fragment and the shells around it again, and the other fragments of those \
                 shells are reused.",
            ],
            li![
                "A fragment remembers the stylesheets, utility classes and client features it uses, so a \
                 page built from cached fragments gets the same CSS and scripts as one rendered fresh. ",
                code!["invalidate_fragment(key)"],
                " drops one entry. Errors are not cached.",
            ],
            li!["The key must name everything the view depends on. Never cache a visitor's markup under a \
                 shared key. A hole inside a fragment shows its fallback: put the hole around the fragment.",],
        ],
        div![
            class("callout"),
            p![
                strong!["Compared with Next.js."],
                " Holes need no ",
                code!["Suspense"],
                " plus request-API detection: a hole is an explicit closure, so what is per-request is \
                 visible in the code and checked at build time. They take captured values from the static \
                 render, work in layouts, are skipped for layouts a navigation keeps, render in place for \
                 crawlers, and their shells can be assembled from independently revalidated fragments.",
            ],
        ],
        h2![id("streaming"), a![class("anchor"), href("#streaming"), "Streaming"]],
        p![
            "A slow page doesn't have to block the whole response. With streaming enabled (the default), the server sends the ",
            strong!["shell"],
            " (layouts, fallbacks, everything that doesn't wait) as soon as it's ready, then streams each suspended part when it resolves.",
        ],
        h3![id("loadingrs"), a![class("anchor"), href("#loadingrs"), code!["loading.rs"]]],
        p![
            "A file named ",
            code!["loading.rs"],
            " in a route folder is the placeholder shown while the page in that folder (and below) is still being \
             prepared: a spinner, a grey skeleton, a line of text. It is optional; without it the visitor simply \
             waits for the whole page. It must export a plain, non-async ",
            code!["Loading"],
            " function, because it has to be ready instantly (an ",
            code!["async fn"],
            " is build error ",
            code!["NR0204"],
            "). This one covers ",
            code!["/dashboard"],
            ":",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/dashboard/loading.rs
pub fn Loading() -> impl View { p![aria("busy", "true"), "Loading dashboard…"] }"#,
        ],],
        p![
            "It wraps everything below the dashboard layout in a suspense boundary. The layout and the fallback arrive immediately, and the page follows.",
        ],
        h3![id("suspense"), a![class("anchor"), href("#suspense"), code!["suspense"]]],
        p![
            code!["suspense"],
            " does the same for one piece of a page instead of the whole page: give it a fallback and a slow async \
             component. In this page the heading is sent at once, and weather and news each appear when their own \
             request finishes, whichever is first:",
        ],
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
                " rendering and to the ",
                a![href("#partial-prerendering"), "holes"],
                " of a static page. The static part of a page is rendered completely at build or revalidation time.",
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
        p![
            "Things fail: a database is down, a record is malformed. A page returns ",
            code!["Result"],
            " and uses ",
            code!["?"],
            " to pass failures up; the framework then shows an error view instead of a broken page. A file named ",
            code!["error.rs"],
            " in a route folder is that view for the folder and everything below it. It exports a non-async ",
            code!["pub fn ErrorBoundary(info: ErrorInfo)"],
            " (note the name: not ",
            code!["Error"],
            "). ",
            code!["app/global-error.rs"],
            ", exporting ",
            code!["GlobalError"],
            ", is the last resort for the whole app. Without either, a built-in error page is used.",
        ],
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
        p![
            "A file named ",
            code!["not-found.rs"],
            " (exporting ",
            code!["NotFound"],
            ") is the \"this page does not exist\" view, status 404. ",
            code!["app/not-found.rs"],
            " covers the whole site; one deeper in the tree covers its folder. Your code asks for it from a page, ",
            code!["load"],
            " or an action with either of these lines:",
        ],
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
        p![
            "A redirect tells the browser \"what you want is at another address\". 307 is temporary (guests sent to \
             the login page); 308 is permanent (a page moved for good, and search engines should update their \
             links). From a page, ",
            code!["load"],
            " or an action, return one as an error:",
        ],
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
            " in middleware and API routes, and through configuration. This block goes in ",
            code!["next-rust.toml"],
            " and needs no code at all; repeat it for every moved URL:",
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
        p!["Metadata is what a page says about itself to browsers, search engines and chat apps without showing it \
             in the page: the title in the tab, the description under a search result, the picture and headline of \
             a shared link (Open Graph), the icon. Without it a page has no title and shared links look bare.",],
        p![
            "Export ",
            code!["metadata"],
            " from ",
            code!["layout.rs"],
            ", ",
            code!["page.rs"],
            " or ",
            code!["metadata.rs"],
            " (a file next to a ",
            code!["page.rs"],
            " or ",
            code!["layout.rs"],
            " that holds only this function; when both exist, ",
            code!["metadata.rs"],
            " wins for that folder). A root layout, ",
            code!["app/layout.rs"],
            ", typically sets the site-wide values like this:",
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
        .theme_color_for("(prefers-color-scheme: light)", "#fafaf7")   // per color scheme
        .icon("/favicon.ico")
        .icon_sized("/favicon-32x32.png", "32x32", "image/png")
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
        h3![id("html-attributes"), a![class("anchor"), href("#html-attributes"), "Attributes on ", code!["<html>"]]],
        p![
            "A theme that lives on the root element (",
            code!["class=\"dark\""],
            ", ",
            code!["data-theme"],
            ") should be in the HTML the server sends, so the page never flashes the wrong colors and works without JavaScript. The root layout owns the content of the page, not the ",
            code!["<html>"],
            " tag itself, so attributes for that tag are set through metadata:",
        ],
        pre![code![
            class("language-rust"),
            r##"// app/layout.rs
pub fn metadata() -> Metadata {
    Metadata::new()
        .html_attribute("data-theme", "ocean")
        .html_attribute("class", "dark")
}"##,
        ],],
        p![
            "Children override a name their parents set. ",
            code!["lang"],
            " replaces ",
            code!["[app] lang"],
            " for that page. Values are escaped, invalid names and event handlers (",
            code!["on*"],
            ") are never rendered, and ",
            code!["class"],
            " gets the same short names as every other class in release builds, so its rules are sent with the page. The element is written with the document: client-side navigations keep the attributes the page has, including any a script changed since, such as a theme toggle.",
        ],
        h3![id("generated-seo-files"), a![class("anchor"), href("#generated-seo-files"), "Generated SEO files"],],
        p![
            "Search engines look for two files at fixed addresses: ",
            code!["/sitemap.xml"],
            ", the list of pages you want indexed, and ",
            code!["/robots.txt"],
            ", the house rules for crawlers. Two optional files directly in ",
            code!["app/"],
            " generate them from code, so the list can come from your database. Without them (and without files of \
             the same name in public/) the URLs answer 404, which is allowed but leaves crawlers to find pages on \
             their own:",
        ],
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
        h3![id("feeds"), a![class("anchor"), href("#feeds"), "Feeds"]],
        p![
            code!["Feed"],
            " renders the same entries as RSS 2.0, Atom 1.0 or JSON Feed 1.1 — each format wants a \
             different date format and a different idea of escaping, which it handles. Serve one per \
             route:",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/feed.xml/route.rs
use next_rust::prelude::*;

pub async fn GET() -> Response {
    let mut feed = Feed::new("Acme", "https://acme.dev", "https://acme.dev/feed.xml")
        .description("Notes from the team")
        .language("en");
    for post in posts::all() {
        feed = feed.entry(
            FeedEntry::new(post.url(), post.title)
                .summary(post.summary)
                .content_html(post.body_html())   // full text, not a teaser
                .published(post.date)             // YYYY-MM-DD or RFC 3339
                .tag(post.topic),
        );
    }
    Response::xml(feed.to_rss())                  // or .to_atom() / .to_json()
}"#,
        ],],
        p!["Tell readers and browsers the feed exists from your root layout, and it is discoverable \
             everywhere:",],
        pre![code![
            class("language-rust"),
            r#"Metadata::new().feed("Acme · RSS", "https://acme.dev/feed.xml", "application/rss+xml")"#,
        ],],
        div![
            class("callout"),
            p![
                code!["Metadata::link(rel, href)"],
                " adds any other ",
                code!["<link>"],
                " you need (",
                code!["preconnect"],
                ", ",
                code!["me"],
                ", ",
                code!["alternate"],
                "). Keep site-wide links in the root layout: client navigation swaps titles, meta tags \
                 and canonicals, and leaves the rest of the head alone.",
            ],
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        p!["Three terminal commands answer \"which mode did my page get, and is the cache used?\":"],
        pre![code![
            class("language-sh"),
            r"next-rust routes --layouts     # every route: static, dynamic or partial, and the reason when dynamic
next-rust build                # renders every static page once; a page that fails here fails the build
next-rust start                # then, in another terminal:
curl -sI http://localhost:3000/blog/hello | grep -i -E 'x-nr-cache|x-nr-prerender|cache-control'",
        ],],
        p![
            code!["x-nr-cache: HIT"],
            " means the stored copy was served, ",
            code!["STALE"],
            " that it was served while a new one is being made, ",
            code!["MISS"],
            " that it was rendered just now. Check this against the production build: in ",
            code!["next-rust dev"],
            " every page renders per request on purpose.",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["A page you expected to be static is dynamic."],
                " Usually a layout above it takes ",
                code!["Cookies"],
                " or ",
                code!["Auth"],
                " (for a \"signed in as\" corner). Move that corner into a ",
                code!["dynamic"],
                " hole and the pages below become static again.",
            ],
            li![
                strong![code!["load"], " returning a bare value, or living in another file."],
                " It must return ",
                code!["Result"],
                " and sit in the same file as the ",
                code!["Page"],
                " that takes ",
                code!["Data<T>"],
                ".",
            ],
            li![
                strong!["An ", code!["async fn Loading"], " or ", code!["async fn ErrorBoundary"], "."],
                " Both must be synchronous (",
                code!["NR0204"],
                "), and the error file exports ",
                code!["ErrorBoundary"],
                ", not ",
                code!["Error"],
                " (",
                code!["NR0201"],
                ").",
            ],
            li![
                strong![code!["generate_params"], " with arguments."],
                " It takes none; it runs at build time, when there is no request.",
            ],
            li![
                strong!["Non-literal constants."],
                " ",
                code!["RENDERING"],
                ", ",
                code!["REVALIDATE"],
                ", ",
                code!["DYNAMIC_PARAMS"],
                " and ",
                code!["TAGS"],
                " are read by the build script from the source text, so write literal values, not expressions or \
                 other constants.",
            ],
            li![
                strong!["Per-segment 404 from ", code!["metadata"], "."],
                " ",
                code!["not_found()"],
                " returned from ",
                code!["metadata"],
                " renders the app-root ",
                code!["not-found.rs"],
                ". For a folder's own 404 page, raise it from ",
                code!["load"],
                " or ",
                code!["Page"],
                ".",
            ],
            li![
                strong!["Setting a cookie inside a hole or streamed content."],
                " The response head has already been sent. Do it in middleware, an action or a route handler.",
            ],
            li![
                strong!["It worked in dev and fails in the build."],
                " Static pages are first rendered without a request during ",
                code!["next-rust build"],
                ". A page that needs a database at that moment needs the database reachable at build time, or must \
                 be dynamic.",
            ],
        ],
    ]
}
