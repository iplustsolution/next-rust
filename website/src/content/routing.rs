//! The "routing" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "The routing directory (",
            code!["app/"],
            " by default, see ",
            a![href("/docs/configuration#app"), "configuration"],
            ") is scanned recursively at build time, and again whenever files change during ",
            code!["next-rust dev"],
            ". The result is compiled into an in-memory trie. Requests never touch the filesystem for routing.",
        ],
        h2![id("special-files"), a![class("anchor"), href("#special-files"), "Special files"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["file"], th!["purpose"], th!["required export"]]],
                tbody![
                    tr![
                        td![code!["page.rs"]],
                        td!["makes the directory a route"],
                        td![code!["pub fn Page(..) -> impl View"]],
                    ],
                    tr![td![code!["page.html"]], td!["static HTML page (fragment or full document)"], td!["—"],],
                    tr![
                        td![code!["layout.rs"]],
                        td!["wraps the segment and everything below"],
                        td![code!["pub fn Layout(children: Children, ..)"]],
                    ],
                    tr![
                        td![code!["template.rs"]],
                        td!["like a layout, rendered inside it"],
                        td![code!["pub fn Template(children: Children, ..)"]],
                    ],
                    tr![
                        td![code!["loading.rs"]],
                        td!["streamed fallback for the segment below"],
                        td![code!["pub fn Loading(..) -> impl View"], " (sync)"],
                    ],
                    tr![
                        td![code!["error.rs"]],
                        td!["error boundary for the segment below"],
                        td![code!["pub fn ErrorBoundary(info: ErrorInfo, ..)"], " (sync)"],
                    ],
                    tr![
                        td![code!["not-found.rs"]],
                        td!["UI for ", code!["not_found()"], " below this segment"],
                        td![code!["pub fn NotFound(..)"]],
                    ],
                    tr![
                        td![code!["global-error.rs"]],
                        td!["last-resort error UI (root only)"],
                        td![code!["pub fn GlobalError(info: ErrorInfo)"]],
                    ],
                    tr![
                        td![code!["route.rs"]],
                        td!["HTTP handlers"],
                        td![
                            code!["GET"],
                            ", ",
                            code!["POST"],
                            ", ",
                            code!["PUT"],
                            ", ",
                            code!["PATCH"],
                            ", ",
                            code!["DELETE"],
                            ", ",
                            code!["OPTIONS"],
                            ", ",
                            code!["HEAD"],
                        ],
                    ],
                    tr![
                        td![code!["middleware.rs"]],
                        td!["middleware for this segment and below"],
                        td![code!["pub async fn middleware(req: Request, next: Next) -> Response"]],
                    ],
                    tr![
                        td![code!["metadata.rs"]],
                        td!["metadata for this segment"],
                        td![code!["pub fn metadata(..) -> Metadata"]],
                    ],
                    tr![td![code!["default.rs"]], td!["fallback for a parallel slot"], td![code!["pub fn Page(..)"]],],
                    tr![
                        td![code!["sitemap.rs"]],
                        td!["generates ", code!["/sitemap.xml"], " (root only)"],
                        td![code!["pub fn sitemap() -> Sitemap"]],
                    ],
                    tr![
                        td![code!["robots.rs"]],
                        td!["generates ", code!["/robots.txt"], " (root only)"],
                        td![code!["pub fn robots() -> Robots"]],
                    ],
                ],
            ],
        ],
        p![
            "Any of these functions may be ",
            code!["async"],
            " (except the synchronous ones marked above) and may return ",
            code!["Result<..>"],
            ". Other ",
            code![".rs"],
            " files in the app directory are ignored. Put shared code in ",
            code!["src/"],
            " and import it with ",
            code!["crate::…"],
            ".",
        ],
        p![
            "Directories starting with ",
            code!["_"],
            " (for example ",
            code!["_components"],
            ") and ",
            code!["."],
            " are private: they never become routes.",
        ],
        h2![id("pages"), a![class("anchor"), href("#pages"), "Pages"]],
        pre![code![
            class("language-text"),
            r"app/page.rs                 → /
app/about/page.rs           → /about
app/blog/page.rs            → /blog
app/blog/[slug]/page.rs     → /blog/:slug",
        ],],
        p![
            "Page functions declare the data they need as ",
            strong!["arguments"],
            ". The generated code supplies each one:",
        ],
        pre![code![
            class("language-rust"),
            r"pub async fn Page(
    Path(p): Path<PostParams>,   // typed route parameters
    Query(q): Query,             // query string (map) or Query<T> (typed)
    cookies: Cookies,            // read/set cookies
    headers: Headers,            // request headers
    Extension(user): Extension<User>, // values inserted by middleware
) -> Result<impl View> { … }",
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["argument"], th!["provides"], th!["makes the route dynamic"]]],
                tbody![
                    tr![td![code!["Params"]], td!["raw route params"], td!["no"]],
                    tr![
                        td![code!["Path<T>"]],
                        td!["params deserialized into ", code!["T"], " (parse failure → 404)"],
                        td!["no"],
                    ],
                    tr![
                        td![code!["Data<T>"]],
                        td!["the result of this file's ", code!["load"], " function"],
                        td!["no"],
                    ],
                    tr![td![code!["Nonce"]], td!["CSP nonce for your own inline scripts"], td!["no"]],
                    tr![td![code!["Query"], " / ", code!["Query<T>"]], td!["query string"], td!["yes"]],
                    tr![td![code!["Cookies"]], td!["cookie jar (get/set/delete)"], td!["yes"]],
                    tr![td![code!["Headers"]], td!["request headers"], td!["yes"]],
                    tr![td![code!["RequestInfo"]], td!["method, URI, client address"], td!["yes"]],
                    tr![
                        td![code!["Extension<T>"], ", ", code!["Option<Extension<T>>"]],
                        td!["request extensions"],
                        td!["yes"],
                    ],
                    tr![td![code!["Auth<T>"]], td!["the authenticated user, if any"], td!["yes"]],
                    tr![td![code!["FormState"]], td!["errors/values of the last form submission"], td!["yes"]],
                    tr![td![code!["CsrfToken"]], td!["CSRF token + hidden field"], td!["yes"]],
                    tr![td![code!["ResponseHeaders"]], td!["set response headers"], td!["yes"]],
                    tr![td![code!["Ctx"]], td!["the entire request context"], td!["yes"]],
                ],
            ],
        ],
        p![
            "Unknown argument types are compile errors in the generated code: a misspelled extractor fails the build instead of failing at runtime.",
        ],
        h2![id("html-pages"), a![class("anchor"), href("#html-pages"), "HTML pages"]],
        p![code!["page.html"], " is a first-class route:"],
        ul![
            li![
                "A ",
                strong!["fragment"],
                " (no ",
                code!["<html>"],
                " or ",
                code!["<!doctype>"],
                ") is wrapped in the layouts, like a Rust page.",
            ],
            li!["A ", strong!["full document"], " is served as-is, bypassing layouts."],
        ],
        p![
            "A directory can't contain both ",
            code!["page.rs"],
            " and ",
            code!["page.html"],
            ". That's error ",
            code!["NR0101"],
            ", unless you set ",
            code!["[app] html_precedence = \"rs\""],
            " or ",
            code!["\"html\""],
            ".",
        ],
        h2![id("layouts"), a![class("anchor"), href("#layouts"), "Layouts"]],
        pre![code![
            class("language-text"),
            r"app/
├── layout.rs                 RootLayout
└── dashboard/
    ├── layout.rs               DashboardLayout
    └── settings/
        ├── layout.rs             SettingsLayout
        └── page.rs                 SettingsPage",
        ],],
        p!["Layouts receive their rendered children:"],
        pre![code![
            class("language-rust"),
            r"pub fn Layout(children: Children) -> impl View {
    div![Header(), main![children], Footer()]
}",
        ],],
        p![
            "There's no nesting limit. Layouts can take any extractor, for example ",
            code!["Layout(children: Children, cookies: Cookies)"],
            ", and can export ",
            code!["metadata"],
            ". The framework renders the ",
            code!["<html>"],
            ", ",
            code!["<head>"],
            " and ",
            code!["<body>"],
            " elements, so the root layout renders only body content. Set the document language with ",
            code!["[app] lang"],
            ".",
        ],
        p![
            code!["template.rs"],
            " works like ",
            code!["layout.rs"],
            " and is rendered inside the layout of the same segment. On the server the two behave the same. The distinction exists so client-side navigation can re-create templates while preserving layouts.",
        ],
        h3![
            id("layouts-stay-on-screen"),
            a![class("anchor"), href("#layouts-stay-on-screen"), "Layouts stay on screen"]
        ],
        p![
            "When a link moves between two pages that share layouts, only what changed is rendered and sent. For ",
            code!["/dashboard/settings"],
            " → ",
            code!["/dashboard/billing"],
            ", the server does not run the root and dashboard layouts again: it renders the inside of ",
            code!["DashboardLayout"],
            " and the browser swaps just that part. The shared layouts keep their DOM, so scroll positions, open menus, focus and form inputs in them survive the navigation. A refresh or a first visit still renders the whole page.",
        ],
        p![
            "A layout is kept only when its output can't depend on the request. The build checks what each layout (and its ",
            code!["load"],
            ") takes:"
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["layout arguments"], th!["on navigation"]]],
                tbody![
                    tr![
                        td![code!["Children"], ", ", code!["Slots"], ", ", code!["Data"], ", ", code!["Nonce"]],
                        td!["kept"]
                    ],
                    tr![
                        td![code!["Params"], ", ", code!["Path<T>"]],
                        td!["kept while every route parameter keeps its value"]
                    ],
                    tr![
                        td![
                            code!["Cookies"],
                            ", ",
                            code!["Headers"],
                            ", ",
                            code!["Query"],
                            ", ",
                            code!["Auth"],
                            ", ",
                            code!["RequestInfo"],
                            ", ",
                            code!["Ctx"],
                            ", …"
                        ],
                        td!["rendered again, with everything below it"],
                    ],
                ],
            ],
        ],
        p![
            "A ",
            code!["template.rs"],
            " and parallel slots (",
            code!["@slot"],
            ") are rendered on every navigation, and so is everything below them. Static pages are cut out of their cached HTML, so shared layouts aren't sent either. After a deploy the browser's layouts no longer match the new build and the next navigation renders the whole page.",
        ],
        p![
            "A navigation link in a shared layout can't highlight the current page on the server any more, since the layout isn't rendered again. Mark it and the framework keeps it up to date:"
        ],
        pre![code![
            class("language-rust"),
            r#"nav![
    a![href("/dashboard/settings"), active_class("active"), "Settings"],
    // Also active on every page below /dashboard:
    a![href("/dashboard"), active_class_prefix("active"), "Dashboard"],
]"#,
        ],],
        p![
            "The class and ",
            code!["aria-current=\"page\""],
            " are applied on the server for the first render and by the client runtime after every navigation. A link inside an open ",
            code!["<details>"],
            " menu closes it when it navigates.",
        ],
        h2![id("route-groups"), a![class("anchor"), href("#route-groups"), "Route groups"]],
        p!["Parenthesized directories organize files without affecting URLs:"],
        pre![code![
            class("language-text"),
            r"app/(marketing)/pricing/page.rs     → /pricing
app/(marketing)/layout.rs           layout for marketing pages only
app/(shop)/cart/page.rs             → /cart",
        ],],
        p![
            "Groups take part in the layout hierarchy. If two groups resolve to the same URL, that's error ",
            code!["NR0102"],
            ".",
        ],
        h2![id("dynamic-segments"), a![class("anchor"), href("#dynamic-segments"), "Dynamic segments"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["directory"], th!["matches"], th!["param value"]]],
                tbody![
                    tr![td![code!["[id]"]], td!["exactly one segment"], td![code!["\"42\""]]],
                    tr![td![code!["[...slug]"]], td!["one or more segments"], td![code!["[\"a\", \"b\", \"c\"]"]],],
                    tr![
                        td![code!["[[...slug]]"]],
                        td!["zero or more segments"],
                        td![code!["[]"], " for the parent URL"],
                    ],
                ],
            ],
        ],
        pre![code![
            class("language-rust"),
            r#"// app/docs/[...slug]/page.rs
#[derive(serde::Deserialize)]
pub struct P { slug: Vec<String> }

pub fn Page(Path(p): Path<P>) -> impl View { h1![p.slug.join(" / ")] }"#,
        ],],
        p![
            "Parameter names must be identifiers. Values are percent-decoded (",
            code!["%20"],
            " → space), and an encoded slash (",
            code!["%2F"],
            ") stays inside the segment.",
        ],
        h2![id("route-ranking"), a![class("anchor"), href("#route-ranking"), "Route ranking"]],
        p!["When several routes match a URL, the most specific one wins. Formally, each pattern segment has a weight:",],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["segment"], th!["weight"]]],
                tbody![
                    tr![td!["static (", code!["settings"], ")"], td!["3"]],
                    tr![td!["dynamic (", code!["[id]"], ")"], td!["2"]],
                    tr![td!["catch-all (", code!["[...path]"], ")"], td!["1"]],
                    tr![td!["optional catch-all (", code!["[[...path]]"], ")"], td!["0"]],
                ],
            ],
        ],
        p![
            "A route's ",
            strong!["rank"],
            " is its sequence of weights. Among matching routes, the one with the lexicographically greatest rank wins, compared position by position.",
        ],
        pre![code![
            class("language-text"),
            r"/users/settings      rank [3,3]   wins for /users/settings
/users/[id]          rank [3,2]   wins for /users/42
/users/[...path]     rank [3,1]   wins for /users/42/posts/7",
        ],],
        p![
            "The matcher implements this without comparing ranks at runtime. The trie tries static, then dynamic, then catch-all children, and backtracks when a branch fails. So ",
            code!["/users/settings/posts"],
            " still reaches ",
            code!["/users/[id]/posts"],
            " when there's no ",
            code!["/users/settings/posts"],
            " page. A randomized test in ",
            code!["crates/next-rust-router/tests/routing.rs"],
            " checks the matcher against a brute-force implementation of the definition above.",
        ],
        p![
            "Matching cost doesn't depend on the number of routes. It's proportional to the number of URL segments, plus backtracking on shared prefixes. See ",
            a![href("/docs/benchmarks"), "benchmarks"],
            ".",
        ],
        h2![
            id("conflicts-detected-at-build-time"),
            a![class("anchor"), href("#conflicts-detected-at-build-time"), "Conflicts detected at build time"],
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["code"], th!["problem"]]],
                tbody![
                    tr![td!["NR0101"], td![code!["page.rs"], " and ", code!["page.html"], " in one directory"],],
                    tr![td!["NR0102"], td!["two routes resolve to the same URL (often through groups)"]],
                    tr![
                        td!["NR0103"],
                        td![
                            "different parameter names at the same position (",
                            code!["[id]"],
                            " vs ",
                            code!["[userId]"],
                            ")",
                        ],
                    ],
                    tr![td!["NR0104"], td!["a page and a ", code!["route.rs"], " for the same URL"]],
                    tr![td!["NR0105"], td!["a catch-all that is not the last segment"]],
                    tr![td!["NR0106"], td!["an optional catch-all overlapping a page at its parent URL"]],
                    tr![td!["NR0107"], td!["the same parameter name twice in one route"]],
                    tr![
                        td!["NR0108"],
                        td!["invalid segment names (", code!["user[id]"], ", ", code!["[a-b]"], ", ", code!["()"], ")",],
                    ],
                    tr![td!["NR0116"], td![code!["[...x]"], " and ", code!["[[...x]]"], " side by side"]],
                ],
            ],
        ],
        p!["All diagnostic codes are listed in ", a![href("/docs/diagnostics"), "Diagnostics"], ". Example output:",],
        pre![code![
            class("language-text"),
            r"error[NR0103]: Conflicting dynamic segment names
  --> /app/users/[id]/page.rs
  --> /app/users/[userId]/edit/page.rs

  The dynamic segment at `/users/[]` is named differently in different routes: `id`, `userId`.
  Routes sharing a URL position must use the same parameter name.

  help: rename the directories so they use a single name",
        ],],
        h2![id("api-routes"), a![class("anchor"), href("#api-routes"), "API routes"]],
        p![
            "A ",
            code!["route.rs"],
            " anywhere in the tree handles HTTP methods for its URL. See ",
            a![href("/docs/api-routes"), "API routes"],
            ". You can also keep API routes in a separate directory:",
        ],
        pre![code![
            class("language-toml"),
            r#"[api]
directory = "api"   # api/users/route.rs → /api/users
prefix = "/api""#,
        ],],
        h2![
            id("parallel-routes-slot"),
            a![class("anchor"), href("#parallel-routes-slot"), "Parallel routes (", code!["@slot"], ")"],
        ],
        p!["A layout can render several independent sections, each backed by its own subtree:"],
        pre![code![
            class("language-text"),
            r"app/dashboard/
├── layout.rs
├── page.rs                    /dashboard (children)
├── settings/page.rs           /dashboard/settings (children)
├── @analytics/
│   ├── page.rs                slot content for /dashboard
│   └── default.rs             slot content for other dashboard URLs
└── @activity/
    └── default.rs",
        ],],
        pre![code![
            class("language-rust"),
            r#"pub fn Layout(children: Children, mut slots: Slots) -> impl View {
    div![main![children], aside![slots.take("analytics")], aside![slots.take("activity")]]
}"#,
        ],],
        p![
            "For each URL, a slot renders the page at the matching path inside the slot directory. If there isn't one, it renders the slot's ",
            code!["default.rs"],
            ", or nothing. Slots never create URLs of their own.",
        ],
        h2![id("intercepting-routes"), a![class("anchor"), href("#intercepting-routes"), "Intercepting routes"],],
        p![
            "An intercepting route renders a different page for a URL when the user navigates to it ",
            strong!["client-side"],
            " from a given part of the app. The typical example is showing a photo in a modal from a feed, while a direct visit shows the full page.",
        ],
        pre![code![
            class("language-text"),
            r"app/feed/page.rs
app/feed/(.)photo/[id]/page.rs   intercepts /feed/photo/:id when navigating from /feed
app/photo/[id]/page.rs           the regular page",
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["marker"], th!["target resolves relative to"]]],
                tbody![
                    tr![td![code!["(.)segment"]], td!["the directory containing the marker"]],
                    tr![td![code!["(..)segment"]], td!["one URL segment up (", code!["(..)(..)"], " for two)"],],
                    tr![td![code!["(...)segment"]], td!["the app root"]],
                ],
            ],
        ],
        p![
            "How it works: the client runtime sends ",
            code!["x-nr-nav: 1"],
            " and ",
            code!["x-nr-from: <current path>"],
            " with navigation requests. The server uses the intercepting page when the target URL matches and ",
            code!["x-nr-from"],
            " falls under the interceptor's directory. Direct loads, reloads and requests without the runtime always get the regular page.",
        ],
        p![
            "Limitations: interception works at page level. It isn't combined with parallel slots, and nesting intercepts is an error (",
            code!["NR0115"],
            ").",
        ],
        h2![id("middleware-placement"), a![class("anchor"), href("#middleware-placement"), "Middleware placement"],],
        p![
            code!["app/middleware.rs"],
            " runs for every request ",
            strong!["before routing"],
            ", so it can rewrite paths. Nested ",
            code!["middleware.rs"],
            " files run for their subtree, outermost first. See ",
            a![href("/docs/middleware"), "Middleware & auth"],
            ".",
        ],
    ]
}
