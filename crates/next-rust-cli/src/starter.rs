//! The starter site created by `next-rust new`: a responsive landing page,
//! an About page explaining the framework, a styled 404 page, a shared header
//! and footer, icons and the Next Rust logo as favicon.
//!
//! Every file is ordinary application code the user owns and can change.

/// `src/components.rs`
pub const COMPONENTS_RS: &str = r####"//! Shared building blocks for the starter site: icons, header and footer.
//! Pages import them with `use crate::components::*;`.

#![allow(non_snake_case)]

use next_rust::prelude::*;

pub const REPO: &str = "https://github.com/iplustsolution/next-rust";

/// A small inline SVG icon that inherits the current text colour.
pub fn icon(name: &str) -> Node {
    let shape = match name {
        "folder" => {
            r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#
        }
        "bolt" => r#"<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>"#,
        "layers" => r#"<path d="m12 3 9 5-9 5-9-5z"/><path d="m3 13 9 5 9-5"/>"#,
        "shield" => {
            r#"<path d="M12 3 5 6v6c0 4.5 3 7.5 7 9 4-1.5 7-4.5 7-9V6z"/><path d="m9 12 2 2 4-4"/>"#
        }
        "code" => r#"<path d="m8 8-4 4 4 4"/><path d="m16 8 4 4-4 4"/><path d="m14 4-4 16"/>"#,
        "sparkle" => {
            r#"<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/><path d="M19 17l.7 1.8 1.8.7-1.8.7L19 22l-.7-1.8-1.8-.7 1.8-.7z"/>"#
        }
        "server" => {
            r#"<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>"#
        }
        "arrow" => r#"<path d="M5 12h14M13 6l6 6-6 6"/>"#,
        "branch" => {
            r#"<circle cx="6" cy="6" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="8" r="2"/><path d="M6 8v8M18 10c0 4-6 3-10 6"/>"#
        }
        "book" => r#"<path d="M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z"/><path d="M4 19V5"/>"#,
        "terminal" => {
            r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 9 3 3-3 3M12 15h5"/>"#
        }
        "menu" => r#"<path d="M4 7h16M4 12h16M4 17h16"/>"#,
        "home" => r#"<path d="m3 11 9-7 9 7"/><path d="M5 10v10h14V10"/>"#,
        "globe" => {
            r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3.5 3 14.5 0 18M12 3c-3 3.5-3 14.5 0 18"/>"#
        }
        "heart" => {
            r#"<path d="M12 20s-7-4.5-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.5-7 10-7 10z"/>"#
        }
        "check" => r#"<path d="m5 12 5 5 9-10"/>"#,
        "compass" => r#"<circle cx="12" cy="12" r="9"/><path d="m15.5 8.5-2 5-5 2 2-5z"/>"#,
        "gauge" => r#"<path d="M4 16a8 8 0 1 1 16 0"/><path d="m12 16 4-5"/>"#,
        _ => r#"<circle cx="12" cy="12" r="9"/>"#,
    };
    raw_html(format!(
        r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{shape}</svg>"#
    ))
}

fn brand() -> impl View {
    Link!(
        href = "/",
        class = "brand",
        img![src("/favicon.svg"), alt(""), width(34), height(34)],
        span!["Next ", span![class("text-gradient"), "Rust"]]
    )
}

pub fn SiteHeader() -> impl View {
    let links = || {
        fragment![
            Link!(href = "/", "Home"),
            Link!(href = "/about", "About"),
            a![href("/api/hello"), "API"],
            a![href(REPO), target("_blank"), rel("noopener"), "GitHub"],
        ]
    };
    header![
        class("site-header"),
        div![
            class("container nav"),
            brand(),
            nav![class("nav-links"), aria("label", "Main"), links()],
            a![
                class("btn btn-primary btn-sm nav-cta"),
                href(REPO),
                target("_blank"),
                rel("noopener"),
                icon("branch"),
                "Star on GitHub"
            ],
            details![
                class("mobile-menu"),
                summary![aria("label", "Menu"), icon("menu")],
                nav![class("mobile-panel"), aria("label", "Mobile"), links()],
            ],
        ],
    ]
}

pub fn SiteFooter() -> impl View {
    let col = |title: &'static str, items: Vec<Node>| {
        div![
            class("footer-col"),
            h4![title],
            ul![each(items, |item| li![item])]
        ]
    };
    footer![
        class("site-footer"),
        div![
            class("container footer-grid"),
            div![
                class("footer-brand"),
                brand(),
                p![
                    "The Rust-native full-stack web framework. Folders become routes, Rust becomes HTML, one binary goes to production."
                ],
            ],
            col(
                "Your app",
                vec![
                    Link!(href = "/", "Home").into_node(),
                    Link!(href = "/about", "About").into_node(),
                    a![href("/api/hello"), "API route"].into_node(),
                ]
            ),
            col(
                "Framework",
                vec![
                    a![href(format!("{REPO}#readme")), "Documentation"].into_node(),
                    a![href(format!("{REPO}/tree/main/examples")), "Examples"].into_node(),
                    a![href(format!("{REPO}/blob/main/CHANGELOG.md")), "Changelog"].into_node(),
                ]
            ),
            col(
                "Community",
                vec![
                    a![href(REPO), "GitHub"].into_node(),
                    a![
                        href(format!("{REPO}/blob/main/CONTRIBUTING.md")),
                        "Contribute"
                    ]
                    .into_node(),
                    a![href(format!("{REPO}/issues")), "Report an issue"].into_node(),
                ]
            ),
        ],
        div![
            class("container footer-bottom"),
            p![
                "Built with ",
                strong!["Next Rust"],
                ". Edit ",
                code!["app/layout.rs"],
                " to make this footer yours."
            ],
            p![
                class("made-by"),
                "Crafted with ",
                span![class("heart"), icon("heart")],
                " by ",
                a![
                    href("https://www.iplust.in/"),
                    target("_blank"),
                    rel("noopener"),
                    "I Plus T Solution"
                ],
            ],
        ],
    ]
}
"####;

/// `app/layout.rs (the app name is substituted)`
pub const LAYOUT_RS: &str = r####"use next_rust::prelude::*;

use crate::components::{SiteFooter, SiteHeader};

pub fn metadata() -> Metadata {
    Metadata::new()
        .title("__APP_NAME__")
        .title_template("%s · __APP_NAME__")
        .description("A Next Rust application: file-based routing, server rendering and a single native binary.")
        .icon("/favicon.svg")
        .theme_color("#0b0b10")
        .open_graph(OpenGraph { site_name: Some("__APP_NAME__".into()), kind: Some("website".into()), ..Default::default() })
}

pub fn Layout(children: Children) -> impl View {
    div![
        class("page"),
        global_css!("globals.css"),
        a![class("skip-link"), href("#content"), "Skip to content"],
        SiteHeader(),
        main![id("content"), children],
        SiteFooter(),
    ]
}
"####;

/// `app/page.rs`
pub const PAGE_RS: &str = r####"use next_rust::prelude::*;

use crate::components::{REPO, icon};

/// A highlighted code token.
fn t(kind: &'static str, text: &'static str) -> Node {
    span![class(kind), text].into_node()
}

fn code_window() -> impl View {
    div![
        class("window"),
        div![class("window-bar"), span![class("dot")], span![class("dot")], span![class("dot")], span![class("window-title"), "app/page.rs"]],
        pre![code![
            t("kw", "use"), " next_rust::prelude::*;\n\n",
            t("kw", "pub fn"), " ", t("fn", "Page"), "() -> ", t("kw", "impl"), " ", t("ty", "View"), " {\n",
            "    ", t("mac", "div!"), "[\n",
            "        ", t("mac", "h1!"), "[", t("str", "\"Hello, world\""), "],\n",
            "        ", t("mac", "p!"), "[", t("str", "\"Rendered on the server.\""), "],\n",
            "    ]\n",
            "}",
        ]],
        div![class("window-foot"), span![class("pill"), icon("check"), "GET /"], span!["200 OK · HTML streamed · 0 KB JavaScript"]],
    ]
}

fn feature(icon_name: &str, title: &'static str, text: &'static str) -> impl View {
    article![class("card feature"), div![class("feature-icon"), icon(icon_name)], h3![title], p![text]]
}

fn step(number: &'static str, title: &'static str, text: &'static str, snippet: &'static str) -> impl View {
    div![class("card step"), span![class("step-number"), number], h3![title], p![text], code![class("snippet"), snippet]]
}

pub fn Page() -> impl View {
    fragment![
        section![
            class("hero"),
            div![
                class("container hero-grid"),
                div![
                    class("hero-copy"),
                    span![class("badge"), icon("sparkle"), "Next Rust v0.1 · 100% Rust"],
                    h1!["Build for the web ", br![], span![class("text-gradient"), "at the speed of Rust."]],
                    p![
                        class("lead"),
                        "Your app is running. Folders become routes, Rust functions become HTML, and everything ships as one fast, native binary. No Node.js. No bundler. No surprises."
                    ],
                    div![
                        class("actions"),
                        Link!(href = "/about", class = "btn btn-primary", "Explore the project", icon("arrow")),
                        a![class("btn btn-ghost"), href(format!("{REPO}#readme")), target("_blank"), rel("noopener"), icon("book"), "Read the docs"],
                    ],
                    div![class("install"), span![class("prompt"), "$"], code!["next-rust routes"], span![class("hint"), "see every route in this app"]],
                ],
                code_window(),
            ],
        ],
        section![
            class("stats"),
            div![
                class("container"),
                div![class("stats-grid"), each(
                    [("100%", "Rust, end to end"), ("1", "native binary to deploy"), ("0 KB", "JavaScript by default"), ("∞", "nested layouts")],
                    |(value, label)| div![class("stat"), strong![class("text-gradient"), value], span![label]],
                )],
            ],
        ],
        section![
            class("section"),
            div![
                class("container"),
                div![class("section-head"), span![class("eyebrow"), "Why Next Rust"], h2!["Everything you need, nothing you don't."], p!["A modern full-stack toolkit that feels familiar on day one and stays fast in production."]],
                div![
                    class("grid-3"),
                    feature("folder", "File-based routing", "Create app/blog/[slug]/page.rs and /blog/:slug exists. Layouts, groups and catch-all routes included."),
                    feature("bolt", "Streaming SSR", "Send the page shell instantly and stream slow parts as they finish, with loading.rs boundaries."),
                    feature("layers", "Static & incremental", "Pages that don't read the request are pre-rendered at build time and refreshed on a schedule."),
                    feature("server", "API routes & actions", "Put route.rs next to your pages, or call #[server_action] functions straight from forms."),
                    feature("shield", "Secure by default", "Escaped HTML, safe cookies, CSRF checks, CSP nonces and traversal-proof static files."),
                    feature("gauge", "Type-checked end to end", "If a page asks for data the framework can't provide, it simply doesn't compile."),
                ],
            ],
        ],
        section![
            class("section section-alt"),
            div![
                class("container"),
                div![class("section-head"), span![class("eyebrow"), "How it works"], h2!["From folder to production in three steps."]],
                div![
                    class("grid-3"),
                    step("01", "Create a folder", "Every folder inside app/ is a URL segment.", "app/pricing/"),
                    step("02", "Write a page", "Export a Page function that returns your view.", "pub fn Page() -> impl View"),
                    step("03", "Ship one binary", "Build once, pre-render what you can, deploy anywhere.", "next-rust build"),
                ],
            ],
        ],
        section![
            class("section"),
            div![
                class("container split"),
                div![
                    class("section-head left"),
                    span![class("eyebrow"), "Your project"],
                    h2!["Here's what was created for you."],
                    p!["Open any of these files, change something and save. The dev server rebuilds and your browser reloads on its own."],
                    ul![
                        class("checklist"),
                        li![icon("check"), "Edit ", code!["app/page.rs"], " to change this page"],
                        li![icon("check"), "Add ", code!["app/contact/page.rs"], " to create /contact"],
                        li![icon("check"), "Style everything in ", code!["app/globals.css"]],
                    ],
                ],
                div![
                    class("window tree"),
                    div![class("window-bar"), span![class("dot")], span![class("dot")], span![class("dot")], span![class("window-title"), "project"]],
                    pre![code![
                        t("dir", "app/"), "\n",
                        "├── ", t("fn", "layout.rs"), "          ", t("cm", "header, footer, metadata"), "\n",
                        "├── ", t("fn", "page.rs"), "            ", t("cm", "/  (you are here)"), "\n",
                        "├── ", t("fn", "not-found.rs"), "       ", t("cm", "the 404 page"), "\n",
                        "├── ", t("fn", "globals.css"), "        ", t("cm", "theme & styles"), "\n",
                        "├── ", t("dir", "about/"), t("fn", "page.rs"), "      ", t("cm", "/about"), "\n",
                        "└── ", t("dir", "api/hello/"), t("fn", "route.rs"), " ", t("cm", "GET /api/hello"), "\n",
                        t("dir", "src/"), t("fn", "components.rs"), "      ", t("cm", "icons, header, footer"), "\n",
                        t("dir", "public/"), t("fn", "favicon.svg"), "     ", t("cm", "your logo"),
                    ]],
                ],
            ],
        ],
        section![
            class("cta"),
            div![class("container"), div![
                class("cta-card"),
                h2!["Ready to make it yours?"],
                p!["Start with app/page.rs. Everything on this page is plain Rust you can change or delete."],
                div![
                    class("actions center"),
                    Link!(href = "/about", class = "btn btn-primary", "Learn how it works", icon("arrow")),
                    a![class("btn btn-ghost"), href(REPO), target("_blank"), rel("noopener"), icon("branch"), "View on GitHub"],
                ],
            ]],
        ],
    ]
}
"####;

/// `app/about/page.rs`
pub const ABOUT_RS: &str = r####"use next_rust::prelude::*;

use crate::components::{REPO, icon};

pub fn metadata() -> Metadata {
    Metadata::new().title("About").description("What Next Rust is, how a project is organized, and the commands you'll use every day.")
}

fn convention(file: &'static str, what: &'static str) -> impl View {
    div![class("row"), code![file], span![what]]
}

fn command(cmd: &'static str, what: &'static str) -> impl View {
    div![class("card command"), div![class("feature-icon small"), icon("terminal")], div![code![cmd], p![what]]]
}

pub fn Page() -> impl View {
    fragment![
        section![
            class("page-hero"),
            div![
                class("container narrow center"),
                span![class("badge"), icon("book"), "About this project"],
                h1!["A web framework that ", span![class("text-gradient"), "thinks in Rust."]],
                p![
                    class("lead"),
                    "Next Rust brings file-based routing, nested layouts, server rendering and static generation to Rust. Your whole site, from pages to APIs, compiles into a single server binary."
                ],
            ],
        ],
        section![
            class("section"),
            div![
                class("container grid-3"),
                article![class("card feature"), div![class("feature-icon"), icon("folder")], h3!["The filesystem is the router"], p!["Folders map to URLs. page.rs renders a page, layout.rs wraps everything below it, and route.rs answers API requests."]],
                article![class("card feature"), div![class("feature-icon"), icon("code")], h3!["Views are Rust code"], p!["div![], h1![] and friends build HTML with automatic escaping. Components are ordinary functions."]],
                article![class("card feature"), div![class("feature-icon"), icon("globe")], h3!["The server decides"], p!["Pages render on the server. Static pages are pre-built, dynamic pages stream, and JavaScript is only sent when you ask for it."]],
            ],
        ],
        section![
            class("section section-alt"),
            div![
                class("container split"),
                div![
                    class("section-head left"),
                    span![class("eyebrow"), "Conventions"],
                    h2!["Special files"],
                    p!["Drop these into any folder under app/. Nothing needs to be registered."],
                ],
                div![
                    class("card table"),
                    convention("page.rs", "A page, and the URL for its folder"),
                    convention("layout.rs", "Wraps the folder and everything below"),
                    convention("loading.rs", "Shown while the page streams in"),
                    convention("error.rs", "Catches errors from pages below"),
                    convention("not-found.rs", "Shown for missing pages"),
                    convention("route.rs", "HTTP handlers: GET, POST, …"),
                    convention("middleware.rs", "Runs before requests below it"),
                    convention("[id]/", "A dynamic URL segment"),
                    convention("(group)/", "Organize folders without changing URLs"),
                ],
            ],
        ],
        section![
            class("section"),
            div![
                class("container"),
                div![class("section-head"), span![class("eyebrow"), "Commands"], h2!["The CLI you'll use every day"]],
                div![
                    class("grid-2"),
                    command("next-rust dev", "Start the dev server with live reload"),
                    command("next-rust routes", "List routes and how each one renders"),
                    command("next-rust generate page /x", "Scaffold a new page"),
                    command("next-rust build", "Compile and pre-render for production"),
                    command("next-rust start", "Run the production build"),
                    command("next-rust upgrade", "Update to the latest Next Rust"),
                ],
            ],
        ],
        section![
            class("cta"),
            div![class("container"), div![
                class("cta-card"),
                h2!["Go deeper"],
                p!["Guides for routing, rendering, data, middleware, server actions, deployment and more."],
                div![
                    class("actions center"),
                    a![class("btn btn-primary"), href(format!("{REPO}#readme")), target("_blank"), rel("noopener"), icon("book"), "Read the documentation"],
                    Link!(href = "/", class = "btn btn-ghost", icon("home"), "Back home"),
                ],
            ]],
        ],
    ]
}
"####;

/// `app/not-found.rs`
pub const NOT_FOUND_RS: &str = r####"use next_rust::prelude::*;

use crate::components::icon;

pub fn NotFound() -> impl View {
    section![
        class("not-found"),
        div![
            class("container narrow center"),
            div![class("lost-icon"), icon("compass")],
            p![class("code-404 text-gradient"), "404"],
            h1!["This page wandered off."],
            p![class("lead"), "The page you're looking for doesn't exist or has moved. Let's get you back on track."],
            div![
                class("actions center"),
                Link!(href = "/", class = "btn btn-primary", icon("home"), "Take me home"),
                Link!(href = "/about", class = "btn btn-ghost", icon("book"), "About this project"),
            ],
            p![class("hint"), "Tip: create ", code!["app/<path>/page.rs"], " and this URL will exist."],
        ],
    ]
}
"####;

/// `app/globals.css`
pub const GLOBALS_CSS: &str = r####"/* Theme ------------------------------------------------------------------ */
:root {
  --bg: #0a0a0f;
  --bg-soft: #111118;
  --surface: rgba(255, 255, 255, 0.035);
  --surface-strong: rgba(255, 255, 255, 0.06);
  --border: rgba(255, 255, 255, 0.09);
  --text: #f2f2f5;
  --muted: #a1a1b0;
  --accent: #ff7a2f;
  --accent-2: #ffb35c;
  --accent-3: #e8430f;
  --glow: rgba(255, 122, 47, 0.18);
  --radius: 18px;
  --shadow: 0 20px 60px rgba(0, 0, 0, 0.45);
  --mono: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
  color-scheme: dark;
}

@media (prefers-color-scheme: light) {
  :root {
    --bg: #fbfaf8;
    --bg-soft: #f3f1ee;
    --surface: rgba(20, 20, 30, 0.03);
    --surface-strong: rgba(20, 20, 30, 0.055);
    --border: rgba(20, 20, 30, 0.1);
    --text: #16161d;
    --muted: #5d5d6c;
    --glow: rgba(255, 122, 47, 0.14);
    --shadow: 0 20px 50px rgba(30, 20, 10, 0.12);
    color-scheme: light;
  }
}

/* Base ------------------------------------------------------------------- */
*, *::before, *::after { box-sizing: border-box; }

html { scroll-behavior: smooth; }

body {
  margin: 0;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  font-size: 16px;
  line-height: 1.65;
  color: var(--text);
  background:
    radial-gradient(900px 500px at 85% -10%, var(--glow), transparent 70%),
    radial-gradient(700px 400px at -10% 10%, rgba(255, 179, 92, 0.08), transparent 70%),
    var(--bg);
  -webkit-font-smoothing: antialiased;
}

a { color: inherit; text-decoration: none; }
img { display: block; max-width: 100%; }
h1, h2, h3, h4 { line-height: 1.15; letter-spacing: -0.02em; margin: 0; }
p { margin: 0; }
code { font-family: var(--mono); font-size: 0.9em; }
p code, li code {
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.1em 0.4em;
}

.icon { width: 1.15em; height: 1.15em; flex: none; }

.container { width: 100%; max-width: 1180px; margin: 0 auto; padding: 0 24px; }
.narrow { max-width: 780px; }
.center { text-align: center; margin-left: auto; margin-right: auto; }

.text-gradient {
  background: linear-gradient(100deg, var(--accent-2), var(--accent) 45%, var(--accent-3));
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.skip-link {
  position: absolute;
  left: -999px;
  top: 12px;
  z-index: 100;
  padding: 8px 14px;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
}
.skip-link:focus { left: 12px; }

:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; border-radius: 6px; }

/* Buttons ---------------------------------------------------------------- */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.55em;
  padding: 0.8em 1.35em;
  border-radius: 999px;
  font-weight: 600;
  font-size: 0.97rem;
  border: 1px solid transparent;
  transition: transform 0.2s ease, box-shadow 0.2s ease, background 0.2s ease, border-color 0.2s ease;
  white-space: nowrap;
}
.btn:hover { transform: translateY(-2px); }
.btn-primary {
  color: #fff;
  background: linear-gradient(135deg, var(--accent-2), var(--accent) 50%, var(--accent-3));
  box-shadow: 0 10px 30px rgba(255, 122, 47, 0.35);
}
.btn-primary:hover { box-shadow: 0 14px 40px rgba(255, 122, 47, 0.5); }
.btn-ghost { background: var(--surface); border-color: var(--border); }
.btn-ghost:hover { background: var(--surface-strong); border-color: var(--accent); }
.btn-sm { padding: 0.55em 1em; font-size: 0.88rem; }

/* Header ----------------------------------------------------------------- */
.site-header {
  position: sticky;
  top: 0;
  z-index: 50;
  backdrop-filter: saturate(180%) blur(14px);
  -webkit-backdrop-filter: saturate(180%) blur(14px);
  background: color-mix(in srgb, var(--bg) 72%, transparent);
  border-bottom: 1px solid var(--border);
}
.nav { display: flex; align-items: center; gap: 28px; height: 70px; }
.brand { display: inline-flex; align-items: center; gap: 10px; font-weight: 800; font-size: 1.15rem; letter-spacing: -0.02em; }
.brand img { border-radius: 9px; }
.nav-links { display: flex; gap: 6px; margin-left: auto; }
.nav-links a { padding: 8px 14px; border-radius: 999px; color: var(--muted); font-weight: 500; transition: color 0.2s, background 0.2s; }
.nav-links a:hover { color: var(--text); background: var(--surface-strong); }

.mobile-menu { display: none; margin-left: auto; position: relative; }
.mobile-menu summary {
  list-style: none;
  cursor: pointer;
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface);
}
.mobile-menu summary::-webkit-details-marker { display: none; }
.mobile-menu .icon { width: 22px; height: 22px; }
.mobile-panel {
  position: absolute;
  right: 0;
  top: 52px;
  min-width: 210px;
  display: grid;
  padding: 8px;
  border-radius: 16px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  box-shadow: var(--shadow);
}
.mobile-panel a { padding: 10px 14px; border-radius: 10px; }
.mobile-panel a:hover { background: var(--surface-strong); }

/* Hero ------------------------------------------------------------------- */
.hero { position: relative; padding: 96px 0 72px; overflow: hidden; }
.hero::before {
  content: "";
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(var(--border) 1px, transparent 1px),
    linear-gradient(90deg, var(--border) 1px, transparent 1px);
  background-size: 56px 56px;
  mask-image: radial-gradient(ellipse at 50% 20%, #000 20%, transparent 70%);
  -webkit-mask-image: radial-gradient(ellipse at 50% 20%, #000 20%, transparent 70%);
  pointer-events: none;
}
.hero-grid { position: relative; display: grid; grid-template-columns: 1.05fr 0.95fr; gap: 56px; align-items: center; }
.badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  border-radius: 999px;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--accent-2);
  background: rgba(255, 122, 47, 0.1);
  border: 1px solid rgba(255, 122, 47, 0.3);
}
.hero h1 { margin: 22px 0 20px; font-size: clamp(2.5rem, 5.6vw, 4.4rem); font-weight: 800; }
.lead { font-size: clamp(1.02rem, 1.6vw, 1.2rem); color: var(--muted); max-width: 40em; }
.center .lead { margin: 0 auto; }
.actions { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 32px; }
.actions.center { justify-content: center; }

.install {
  display: inline-flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 28px;
  padding: 10px 16px;
  border-radius: 12px;
  background: var(--surface);
  border: 1px dashed var(--border);
  font-family: var(--mono);
  font-size: 0.9rem;
}
.install .prompt { color: var(--accent); }
.install .hint { color: var(--muted); font-family: inherit; font-size: 0.8rem; }

/* Code windows ----------------------------------------------------------- */
.window {
  border-radius: var(--radius);
  background: linear-gradient(180deg, #16161f, #0e0e14);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: var(--shadow), 0 0 0 1px rgba(255, 122, 47, 0.06), 0 0 80px var(--glow);
  overflow: hidden;
  color: #e7e7ee;
}
.window-bar { display: flex; align-items: center; gap: 8px; padding: 14px 18px; border-bottom: 1px solid rgba(255, 255, 255, 0.07); }
.dot { width: 12px; height: 12px; border-radius: 50%; background: #ff5f57; }
.dot:nth-child(2) { background: #febc2e; }
.dot:nth-child(3) { background: #28c840; }
.window-title { margin-left: 10px; font-family: var(--mono); font-size: 0.82rem; color: #9a9aa8; }
.window pre { margin: 0; padding: 22px 24px; overflow-x: auto; font-size: 0.92rem; line-height: 1.75; }
.window-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  padding: 12px 18px;
  font-size: 0.82rem;
  color: #9a9aa8;
  border-top: 1px solid rgba(255, 255, 255, 0.07);
  background: rgba(255, 255, 255, 0.02);
}
.pill { display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px; border-radius: 999px; background: rgba(40, 200, 64, 0.14); color: #5ee27a; font-weight: 600; }
.kw { color: #ff8a4c; }
.fn { color: #ffd08a; }
.ty { color: #7dd3fc; }
.mac { color: #c4b5fd; }
.str { color: #86efac; }
.cm { color: #6b6b7b; }
.dir { color: #7dd3fc; }

/* Stats ------------------------------------------------------------------ */
.stats { padding: 8px 0 40px; }
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  overflow: hidden;
}
.stat { display: grid; gap: 2px; padding: 26px 24px; text-align: center; }
.stat + .stat { border-left: 1px solid var(--border); }
.stat strong { font-size: 2.1rem; font-weight: 800; letter-spacing: -0.03em; }
.stat span { color: var(--muted); font-size: 0.92rem; }

/* Sections --------------------------------------------------------------- */
.section { padding: 88px 0; }
.section-alt { background: linear-gradient(180deg, transparent, var(--surface) 30%, var(--surface) 70%, transparent); }
.section-head { text-align: center; max-width: 720px; margin: 0 auto 48px; }
.section-head.left { text-align: left; margin: 0; }
.section-head h2 { font-size: clamp(1.9rem, 3.6vw, 2.8rem); font-weight: 800; margin: 12px 0 14px; }
.section-head p { color: var(--muted); font-size: 1.08rem; }
.eyebrow { font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.14em; color: var(--accent); }

.grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; }
.grid-2 { display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px; }
.split { display: grid; grid-template-columns: 0.9fr 1.1fr; gap: 56px; align-items: center; }

.card {
  position: relative;
  padding: 28px;
  border-radius: var(--radius);
  background: var(--surface);
  border: 1px solid var(--border);
  transition: transform 0.25s ease, border-color 0.25s ease, box-shadow 0.25s ease;
}
.card:hover { transform: translateY(-4px); border-color: rgba(255, 122, 47, 0.45); box-shadow: 0 18px 50px var(--glow); }
.card h3 { font-size: 1.15rem; margin: 18px 0 8px; }
.card p { color: var(--muted); font-size: 0.97rem; }

.feature-icon {
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border-radius: 14px;
  color: #fff;
  background: linear-gradient(135deg, var(--accent-2), var(--accent-3));
  box-shadow: 0 8px 24px rgba(255, 122, 47, 0.35);
}
.feature-icon .icon { width: 24px; height: 24px; }
.feature-icon.small { width: 40px; height: 40px; border-radius: 12px; }
.feature-icon.small .icon { width: 20px; height: 20px; }

.step-number { font-family: var(--mono); font-size: 0.85rem; font-weight: 700; color: var(--accent); }
.step h3 { margin-top: 10px; }
.snippet {
  display: inline-block;
  margin-top: 18px;
  padding: 6px 12px;
  border-radius: 8px;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  font-size: 0.85rem;
}

.checklist { list-style: none; padding: 0; margin: 28px 0 0; display: grid; gap: 14px; }
.checklist li { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.checklist .icon { color: #28c840; }

.table { padding: 8px; display: grid; }
.row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 14px 18px; border-radius: 12px; }
.row + .row { border-top: 1px solid var(--border); }
.row code { color: var(--accent-2); font-weight: 600; }
.row span { color: var(--muted); text-align: right; font-size: 0.95rem; }
.table:hover { transform: none; }

.command { display: flex; gap: 16px; align-items: flex-start; padding: 22px; }
.command code { font-weight: 700; color: var(--text); }
.command p { margin-top: 4px; }

/* Page heroes ------------------------------------------------------------ */
.page-hero { padding: 96px 0 24px; }
.page-hero h1 { font-size: clamp(2.3rem, 5vw, 3.8rem); font-weight: 800; margin: 20px 0 18px; }

/* Call to action --------------------------------------------------------- */
.cta { padding: 40px 0 100px; }
.cta-card {
  position: relative;
  text-align: center;
  padding: 64px 32px;
  border-radius: 28px;
  border: 1px solid rgba(255, 122, 47, 0.3);
  background:
    radial-gradient(600px 240px at 50% 0%, rgba(255, 122, 47, 0.22), transparent 70%),
    var(--surface);
  overflow: hidden;
}
.cta-card h2 { font-size: clamp(1.8rem, 3.4vw, 2.6rem); font-weight: 800; margin-bottom: 12px; }
.cta-card p { color: var(--muted); max-width: 560px; margin: 0 auto; }

/* 404 ------------------------------------------------------------------- */
.not-found { padding: 110px 0 120px; }
.lost-icon {
  display: inline-grid;
  place-items: center;
  width: 76px;
  height: 76px;
  border-radius: 22px;
  color: var(--accent);
  background: rgba(255, 122, 47, 0.1);
  border: 1px solid rgba(255, 122, 47, 0.3);
  animation: float 4s ease-in-out infinite;
}
.lost-icon .icon { width: 38px; height: 38px; }
.code-404 { font-size: clamp(6rem, 18vw, 11rem); font-weight: 900; line-height: 1; letter-spacing: -0.06em; margin-top: 18px; }
.not-found h1 { font-size: clamp(1.8rem, 4vw, 2.6rem); margin: 8px 0 16px; }
.not-found .hint { margin-top: 36px; color: var(--muted); font-size: 0.9rem; }

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-8px); }
}

/* Footer ----------------------------------------------------------------- */
.site-footer { border-top: 1px solid var(--border); background: var(--bg-soft); padding-top: 64px; }
.footer-grid { display: grid; grid-template-columns: 1.6fr repeat(3, 1fr); gap: 40px; padding-bottom: 48px; }
.footer-brand p { color: var(--muted); margin-top: 16px; max-width: 340px; font-size: 0.95rem; }
.footer-col h4 { font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.12em; color: var(--muted); margin-bottom: 16px; }
.footer-col ul { list-style: none; padding: 0; margin: 0; display: grid; gap: 10px; }
.footer-col a { color: var(--text); opacity: 0.85; transition: color 0.2s, opacity 0.2s; }
.footer-col a:hover { color: var(--accent); opacity: 1; }
.footer-bottom {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 12px;
  padding-top: 24px;
  padding-bottom: 28px;
  border-top: 1px solid var(--border);
  color: var(--muted);
  font-size: 0.9rem;
}
.made-by { display: inline-flex; align-items: center; gap: 4px; flex-wrap: wrap; }
.made-by a { color: var(--text); font-weight: 600; }
.made-by a:hover { color: var(--accent); }
.heart { color: #ff4d6d; display: inline-flex; }

/* Motion ----------------------------------------------------------------- */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation: none !important; transition: none !important; scroll-behavior: auto !important; }
}

/* Responsive ------------------------------------------------------------- */
@media (max-width: 1000px) {
  .hero-grid, .split { grid-template-columns: 1fr; gap: 44px; }
  .grid-3 { grid-template-columns: repeat(2, 1fr); }
  .footer-grid { grid-template-columns: 1fr 1fr 1fr; }
  .footer-brand { grid-column: 1 / -1; }
}

@media (max-width: 760px) {
  .nav-links, .nav-cta { display: none; }
  .mobile-menu { display: block; }
  .hero { padding: 64px 0 48px; }
  .section { padding: 64px 0; }
  .grid-3, .grid-2 { grid-template-columns: 1fr; }
  .stats-grid { grid-template-columns: repeat(2, 1fr); }
  .stat:nth-child(3) { border-left: 0; }
  .stat:nth-child(n + 3) { border-top: 1px solid var(--border); }
  .footer-grid { grid-template-columns: 1fr 1fr; }
  .row { flex-direction: column; align-items: flex-start; gap: 4px; }
  .row span { text-align: left; }
  .actions .btn { flex: 1 1 auto; justify-content: center; }
}

@media (max-width: 420px) {
  .container { padding: 0 18px; }
  .footer-grid { grid-template-columns: 1fr; }
  .window pre { font-size: 0.8rem; padding: 18px; }
}
"####;

/// `public/favicon.svg (the Next Rust logo)`
pub const FAVICON_SVG: &str = r####"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" role="img" aria-label="Next Rust logo">
  <title>Next Rust</title>
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#e5e5ec"/>
      <stop offset="1" stop-color="#e0e0ef"/>
    </linearGradient>
    <linearGradient id="rust" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ff9c2a"/>
      <stop offset="0.55" stop-color="#f25c1f"/>
      <stop offset="1" stop-color="#b3260c"/>
    </linearGradient>
    <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#f25c1f"/>
      <stop offset="1" stop-color="#f25c1f" stop-opacity="0"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#f25c1f" stop-opacity="0.35"/>
      <stop offset="1" stop-color="#f25c1f" stop-opacity="0"/>
    </radialGradient>
    <clipPath id="inner">
      <circle cx="256" cy="256" r="126"/>
    </clipPath>
  </defs>

  <rect width="512" height="512" rx="112" fill="url(#bg)"/>
  <circle cx="256" cy="256" r="230" fill="url(#glow)"/>

  <!-- gear: the Rust half -->
  <g fill="url(#rust)">
    <g id="tooth"><rect x="238" y="70" width="36" height="72" rx="8"/></g>
    <use href="#tooth" transform="rotate(30 256 256)"/>
    <use href="#tooth" transform="rotate(60 256 256)"/>
    <use href="#tooth" transform="rotate(90 256 256)"/>
    <use href="#tooth" transform="rotate(120 256 256)"/>
    <use href="#tooth" transform="rotate(150 256 256)"/>
    <use href="#tooth" transform="rotate(180 256 256)"/>
    <use href="#tooth" transform="rotate(210 256 256)"/>
    <use href="#tooth" transform="rotate(240 256 256)"/>
    <use href="#tooth" transform="rotate(270 256 256)"/>
    <use href="#tooth" transform="rotate(300 256 256)"/>
    <use href="#tooth" transform="rotate(330 256 256)"/>
  </g>
  <circle cx="256" cy="256" r="146" fill="none" stroke="url(#rust)" stroke-width="40"/>
  <circle cx="256" cy="256" r="126" fill="#f6f6f9"/>

  <!-- the "N": the Next half, its stroke racing out of the gear -->
  <g clip-path="url(#inner)">
    <rect x="190" y="186" width="30" height="140" rx="4" fill="#f25c1f"/>
    <polygon points="190,186 222,186 350,360 318,360" fill="url(#fade)"/>
    <rect x="292" y="186" width="30" height="92" rx="4" fill="url(#fade)"/>
  </g>
</svg>
"####;

pub const MAIN_RS: &str = "mod components;\n\nnext_rust::app!();\n";

pub fn layout_rs(app_name: &str) -> String {
    LAYOUT_RS.replace("__APP_NAME__", app_name)
}
