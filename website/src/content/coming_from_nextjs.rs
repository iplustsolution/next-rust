//! The "coming-from-nextjs" documentation page.

use next_rust::prelude::*;

/// One row: the Next.js concept, the Next Rust equivalent, and where it is documented.
fn row(next_js: &'static str, next_rust: Node, page: (&'static str, &'static str)) -> Node {
    tr![td![next_js], td![next_rust], td![a![href(page.0), page.1]]].into_node()
}

fn head() -> Node {
    thead![tr![th!["Next.js"], th!["Next Rust"], th!["Read more"]]].into_node()
}

const ROUTING: (&str, &str) = ("/docs/routing", "Routing");
const RENDERING: (&str, &str) = ("/docs/rendering", "Rendering & data");
const VIEWS: (&str, &str) = ("/docs/views", "Views & components");
const CACHING: (&str, &str) = ("/docs/caching", "Caching");
const CLIENT: (&str, &str) = ("/docs/client", "Client & navigation");
const STYLING: (&str, &str) = ("/docs/styling-and-assets", "Styling & assets");
const CONFIGURATION: (&str, &str) = ("/docs/configuration", "Configuration");

pub fn content() -> Node {
    fragment![
        div![
            class("callout"),
            p![
                strong!["In plain words."],
                " If you can find your way around a Next.js project, you can find your way around a Next Rust one: the folders, the file names and the way pages nest are the same on purpose. It is like driving a different make of car. The pedals and the wheel are where you expect them; the engine is different. This page is the list of which control is where, and the few that really moved.",
            ],
        ],
        p![
            "Next Rust follows the Next.js App Router model on purpose, so the way you lay out an app carries over: folders are routes, layouts nest, pages render on the server. What changes is the language and ",
            strong!["when mistakes are caught"],
            ": a build script reads ",
            code!["app/"],
            " at compile time, so a wrong filename or export is a build error rather than a 404.",
        ],
        p![
            "New to both? Start with ",
            a![href("/docs/project-structure"), "Project structure"],
            ", which explains every file without assuming Next.js. Two terms used below: an ",
            strong!["extractor"],
            " is a function argument the framework fills in from the request (cookies, URL parameters), and an ",
            strong!["island"],
            " is a small interactive part of a page that is otherwise plain server-rendered HTML.",
        ],
        h2![id("side-by-side"), a![class("anchor"), href("#side-by-side"), "One page, side by side"]],
        p![
            "The same blog-post page in both frameworks. This is the Next.js version, a server component in ",
            code!["app/blog/[slug]/page.tsx"],
            " that loads a post and shows a 404 when it doesn't exist:",
        ],
        pre![code![
            class("language-js"),
            r#"// app/blog/[slug]/page.tsx
import { notFound } from "next/navigation";

export default async function Page({ params }) {
  const { slug } = await params;
  const post = await findPost(slug);
  if (!post) notFound();
  return <article><h1>{post.title}</h1><p>{post.summary}</p></article>;
}"#,
        ],],
        p![
            "And this is the Next Rust version, in ",
            code!["app/blog/[slug]/page.rs"],
            ". Loading is a separate ",
            code!["load"],
            " function in the same file; its result arrives in the page as ",
            code!["Data<Post>"],
            ", and ",
            code![".or_not_found()"],
            " turns a missing post into the 404 page:",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/blog/[slug]/page.rs
use next_rust::prelude::*;

pub async fn load(params: Params) -> Result<Post> {
    let slug = params.get("slug").unwrap_or_default();
    posts::find(slug).await?.or_not_found()
}

pub fn Page(Data(post): Data<Post>) -> impl View {
    article![h1![post.title.clone()], p![post.summary.clone()]]
}"#,
        ],],
        p![
            strong!["Why it matters:"],
            " the split keeps the view a plain function of its data, and lets the build see from the signatures alone whether a page can be prepared in advance or must run per request.",
        ],
        h2![id("files"), a![class("anchor"), href("#files"), "Files and routing"]],
        p!["The special files keep their names, with a ", code![".rs"], " extension and a fixed export name."],
        div![
            class("table-wrap"),
            table![
                head(),
                tbody![
                    row(
                        "app/page.tsx, default export",
                        fragment![code!["app/page.rs"], " exporting ", code!["pub fn Page() -> impl View"]],
                        ROUTING,
                    ),
                    row(
                        "layout.tsx with {children}",
                        fragment![code!["layout.rs"], " exporting ", code!["pub fn Layout(children: Children)"]],
                        ROUTING,
                    ),
                    row("template.tsx", fragment![code!["template.rs"], " exporting ", code!["Template"]], ROUTING),
                    row(
                        "loading.tsx",
                        fragment![code!["loading.rs"], " exporting ", code!["Loading"], " (must not be async)"],
                        RENDERING,
                    ),
                    row(
                        "error.tsx, global-error.tsx",
                        fragment![
                            code!["error.rs"],
                            " exporting ",
                            code!["ErrorBoundary"],
                            ", ",
                            code!["global-error.rs"],
                            " exporting ",
                            code!["GlobalError"],
                            " (both must not be async)",
                        ],
                        RENDERING,
                    ),
                    row(
                        "not-found.tsx, notFound()",
                        fragment![
                            code!["not-found.rs"],
                            " exporting ",
                            code!["NotFound"],
                            "; return ",
                            code!["Err(not_found())"],
                            " or use ",
                            code![".or_not_found()?"],
                        ],
                        RENDERING,
                    ),
                    row(
                        "[id], [...slug], [[...slug]], (group), @slot, (.)intercept",
                        fragment!["The same directory names"],
                        ROUTING,
                    ),
                    row(
                        "route.ts with GET, POST, …",
                        fragment![
                            code!["route.rs"],
                            " with ",
                            code!["pub async fn GET(req: Request)"],
                            ", ",
                            code!["POST"],
                            ", …",
                        ],
                        ("/docs/api-routes", "API routes"),
                    ),
                    row(
                        "middleware.ts (one, at the root)",
                        fragment![
                            code!["middleware.rs"],
                            " exporting ",
                            code!["middleware(req: Request, next: Next)"],
                            "; nested folders can have their own",
                        ],
                        ("/docs/middleware", "Middleware & auth"),
                    ),
                    row(
                        "sitemap.ts, robots.ts",
                        fragment![code!["sitemap.rs"], ", ", code!["robots.rs"], " at the app root"],
                        RENDERING,
                    ),
                ],
            ],
        ],
        h2![id("data"), a![class("anchor"), href("#data"), "Data, rendering and caching"]],
        div![
            class("table-wrap"),
            table![
                head(),
                tbody![
                    row(
                        "async server component that fetches",
                        fragment![
                            code!["pub async fn load() -> Result<T>"],
                            " in the same file, received as ",
                            code!["Data<T>"],
                        ],
                        RENDERING,
                    ),
                    row(
                        "params, searchParams, headers(), cookies()",
                        fragment![
                            "Extractors in the function signature: ",
                            code!["Params"],
                            ", ",
                            code!["Path<T>"],
                            ", ",
                            code!["Query<T>"],
                            ", ",
                            code!["Headers"],
                            ", ",
                            code!["Cookies"],
                        ],
                        RENDERING,
                    ),
                    row(
                        "export const dynamic / revalidate / dynamicParams",
                        fragment![
                            code!["pub const RENDERING"],
                            ", ",
                            code!["REVALIDATE"],
                            ", ",
                            code!["DYNAMIC_PARAMS"],
                            ", ",
                            code!["TAGS"],
                        ],
                        RENDERING,
                    ),
                    row(
                        "generateStaticParams",
                        fragment![code!["pub async fn generate_params() -> Vec<Params>"]],
                        RENDERING,
                    ),
                    row(
                        "export const metadata / generateMetadata",
                        fragment![code!["pub fn metadata() -> Metadata"], " (may take extractors, may be async)"],
                        RENDERING,
                    ),
                    row(
                        "<Suspense fallback>",
                        fragment![code!["suspense(fallback, async { … })"], ", or a ", code!["loading.rs"]],
                        RENDERING,
                    ),
                    row(
                        "redirect(), permanentRedirect()",
                        fragment![
                            "Return ",
                            code!["Err(redirect(\"/login\"))"],
                            " or ",
                            code!["Err(permanent_redirect(..))"],
                        ],
                        RENDERING,
                    ),
                    row(
                        "revalidatePath(), revalidateTag()",
                        fragment![code!["revalidate_path(..).await"], ", ", code!["revalidate_tag(..).await"]],
                        CACHING,
                    ),
                    row(
                        "unstable_cache / \"use cache\"",
                        fragment![code!["cache(key, CacheOptions::revalidate(60).tag(..), || async { … })"]],
                        CACHING,
                    ),
                    row(
                        "\"use server\" functions",
                        fragment![
                            code!["#[server_action]"],
                            " functions, attached to a form with ",
                            code!["action!(path::to::function)"],
                        ],
                        ("/docs/server-actions", "Server actions & forms"),
                    ),
                ],
            ],
        ],
        h2![id("ui"), a![class("anchor"), href("#ui"), "UI and the browser"]],
        div![
            class("table-wrap"),
            table![
                head(),
                tbody![
                    row(
                        "JSX",
                        fragment![
                            "One macro per HTML tag: ",
                            code!["div![class(\"card\"), h1![\"Hi\"]]"],
                            ". Text is escaped for you.",
                        ],
                        VIEWS,
                    ),
                    row(
                        "list.map(), cond && <X/>",
                        fragment![code!["each(list, |item| …)"], ", ", code!["when(cond, || …)"]],
                        VIEWS,
                    ),
                    row(
                        "<Link>, <Image>",
                        fragment![code!["Link!(href = \"/about\", \"About\")"], ", ", code!["Image!(src = …)"]],
                        VIEWS,
                    ),
                    row(
                        "\"use client\" components",
                        fragment![
                            code!["#[client]"],
                            " islands: declarative bindings, or an ES module with ",
                            code!["hydrate(element, props)"],
                        ],
                        CLIENT,
                    ),
                    row(
                        "useRouter()",
                        fragment![code!["window.nextRust.navigate(href)"], ", ", code![".refresh()"], ", …"],
                        CLIENT,
                    ),
                    row(
                        "globals.css, CSS Modules",
                        fragment![
                            code!["global_css!(\"globals.css\")"],
                            ", ",
                            code!["css_module!(\"card.module.css\")"]
                        ],
                        STYLING,
                    ),
                    row(
                        "Tailwind through PostCSS",
                        fragment!["Built in, no Node.js: ", code!["[tailwind] enabled = true"]],
                        ("/docs/tailwind", "Tailwind CSS"),
                    ),
                    row(
                        "next/font/local",
                        fragment![code!["LocalFont::new(\"Inter\", asset!(\"fonts/inter-var.woff2\"))"]],
                        STYLING,
                    ),
                ],
            ],
        ],
        h2![id("tooling"), a![class("anchor"), href("#tooling"), "Tooling and deployment"]],
        div![
            class("table-wrap"),
            table![
                head(),
                tbody![
                    row(
                        "create-next-app",
                        fragment![code!["next-rust new my-app"]],
                        ("/docs/getting-started", "Getting started"),
                    ),
                    row(
                        "next dev / build / start",
                        fragment![
                            code!["next-rust dev"],
                            " / ",
                            code!["build"],
                            " / ",
                            code!["start"],
                            ", plus ",
                            code!["next-rust routes"],
                            " to list every route",
                        ],
                        ("/docs/cli", "CLI"),
                    ),
                    row(
                        "next.config.js (redirects, headers, basePath)",
                        fragment![
                            code!["next-rust.toml"],
                            ": ",
                            code!["[[redirects]]"],
                            ", ",
                            code!["[[headers]]"],
                            ", ",
                            code!["[app] base_path"],
                        ],
                        CONFIGURATION,
                    ),
                    row(
                        ".env files, NEXT_PUBLIC_*",
                        fragment![
                            "The same ",
                            code![".env"],
                            " files; only ",
                            code!["NEXT_RUST_PUBLIC_*"],
                            " variables reach the browser",
                        ],
                        CONFIGURATION,
                    ),
                    row(
                        "Node.js server or standalone output",
                        fragment!["One stripped binary with pages, config and static files inside"],
                        ("/docs/deployment", "Deployment"),
                    ),
                    row(
                        "Tests against a running server",
                        fragment![code!["TestClient"], " inside ", code!["cargo test"], ", no port needed"],
                        ("/docs/testing", "Testing"),
                    ),
                ],
            ],
        ],
        h2![id("differences"), a![class("anchor"), href("#differences"), "Differences that will surprise you"]],
        div![
            class("callout"),
            p![
                strong!["Nothing is discovered at runtime."],
                " Adding or renaming a file under ",
                code!["app/"],
                " needs a rebuild, not a reload. ",
                code!["next-rust dev"],
                " does that for you; a plain ",
                code!["cargo run"],
                " only sees new files when the build script reruns.",
            ],
        ],
        ul![
            li![
                strong!["Export names are fixed and must be public."],
                " ",
                code!["error.rs"],
                " exports ",
                code!["ErrorBoundary"],
                ", not ",
                code!["Error"],
                ". A wrong name fails the build with ",
                a![href("/docs/diagnostics"), "NR0201"],
                ".",
            ],
            li![
                strong!["Data loading is a separate function."],
                " ",
                code!["load"],
                " returns a ",
                code!["Result"],
                " and must live in the same file as the ",
                code!["Page"],
                " that takes ",
                code!["Data<T>"],
                ". A ",
                code!["load"],
                " in another module is never called.",
            ],
            li![
                strong!["There is no React."],
                " Pages are HTML rendered on the server. Interactivity comes from small ",
                code!["#[client]"],
                " islands, and a page without links, islands or action forms ships no JavaScript at all.",
            ],
            li![
                strong!["Files in the app directory are modules of your crate."],
                " Reach your own code with ",
                code!["crate::db::posts()"],
                ", never with the crate's own name.",
            ],
            li![
                strong!["Unknown configuration keys are errors."],
                " A typo in ",
                code!["next-rust.toml"],
                " fails the build and names the key.",
            ],
            li![
                strong!["Static or dynamic is inferred from the signature."],
                " Only ",
                code!["Params"],
                ", ",
                code!["Path"],
                ", ",
                code!["Nonce"],
                " and ",
                code!["Data"],
                " keep a page static; any other extractor makes it render per request. ",
                code!["next-rust routes --layouts"],
                " prints the reason.",
            ],
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes when porting"]],
        ul![
            li![
                strong!["Writing ", code!["pub fn Error"], " in ", code!["error.rs"], "."],
                " The export is ",
                code!["ErrorBoundary"],
                "; in ",
                code!["global-error.rs"],
                " it is ",
                code!["GlobalError"],
                ".",
            ],
            li![
                strong!["An ", code!["async"], " ", code!["loading.rs"], " or ", code!["error.rs"], "."],
                " They render instantly while something else is pending, so they must be synchronous (",
                code!["NR0204"],
                ").",
            ],
            li![
                strong![code!["load"], " returning a bare value, or living in another module."],
                " It must return ",
                code!["Result<T>"],
                " and sit in the same file as the ",
                code!["Page"],
                " that takes ",
                code!["Data<T>"],
                ".",
            ],
            li![
                strong![code!["generate_params"], " with arguments."],
                " Unlike ",
                code!["generateStaticParams"],
                ", it takes none.",
            ],
            li![
                strong!["Rendering ", code!["<html>"], " and ", code!["<body>"], " in the root layout."],
                " The framework writes the document shell; the root layout renders only what goes inside the body. Set ",
                code!["<html>"],
                " attributes through ",
                code!["[app] lang"],
                " or ",
                code!["Metadata::html_attribute"],
                ".",
            ],
            li![
                strong!["Pasting an analytics snippet into a layout."],
                " Analytics and site verification are configuration: ",
                code!["[analytics]"],
                " and ",
                code!["[verification]"],
                " in ",
                code!["next-rust.toml"],
                ". See ",
                a![href("/docs/analytics"), "Analytics & verification"],
                ".",
            ],
            li![
                strong!["Adding fade or hover transitions by hand."],
                " Pages fade into each other and state changes ease by default (",
                code!["[animation]"],
                " in ",
                code!["next-rust.toml"],
                "); your own ",
                code!["transition"],
                " rules still win.",
            ],
            li![
                strong!["A styling bug that only appears in production."],
                " Release builds shorten class names and remove unused CSS. A class name that is assembled at run time, where the build can't see it, needs ",
                code!["[tailwind] keep_classes"],
                " or ",
                code!["[assets] css_safelist"],
                ". Don't turn Tailwind off to fix it.",
            ],
        ],
        p!["Next: ", a![href("/docs/routing"), "Routing"], "."],
    ]
}
