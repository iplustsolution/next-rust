//! The "project-structure" documentation page.
//!
//! Every file and folder of a Next Rust project, explained twice: once in
//! everyday words, once for the person who has to edit it.

use next_rust::prelude::*;

/// Text with `backticks` turned into `<code>`.
fn rich(text: &str) -> Node {
    text.split('`')
        .enumerate()
        .map(|(i, part)| if i % 2 == 1 { code![part.to_owned()].into_node() } else { part.to_owned().into_node() })
        .collect::<Vec<_>>()
        .into_node()
}

/// A highlighted code block. The class names are literals so the highlighter finds them.
fn code_block(lang: &str, source: &'static str) -> Node {
    match lang {
        "rust" => pre![code![class("language-rust"), source]].into_node(),
        "toml" => pre![code![class("language-toml"), source]].into_node(),
        "sh" => pre![code![class("language-sh"), source]].into_node(),
        "html" => pre![code![class("language-html"), source]].into_node(),
        _ => pre![code![class("language-text"), source]].into_node(),
    }
}

/// One file or folder of the project.
struct Entry {
    id: &'static str,
    name: &'static str,
    /// The everyday analogy, one sentence.
    plain: &'static str,
    does: &'static str,
    touch: &'static str,
    missing: &'static str,
    /// The sentence before the example: what the code does and where it goes.
    intro: &'static str,
    lang: &'static str,
    example: &'static str,
}

fn entry(e: &Entry) -> Node {
    fragment![
        h3![id(e.id), a![class("anchor"), href(format!("#{}", e.id)), code![e.name]]],
        div![
            class("feature-list"),
            div![strong!["In plain words"], span![rich(e.plain)]],
            div![strong!["What it does"], span![rich(e.does)]],
            div![strong!["When you'd touch it"], span![rich(e.touch)]],
            div![strong!["If it's missing"], span![rich(e.missing)]],
        ],
        p![rich(e.intro)],
        code_block(e.lang, e.example),
    ]
    .into_node()
}

/// One row of the "I want to…" table.
fn want(goal: &'static str, open: &'static str) -> Node {
    tr![td![rich(goal)], td![rich(open)]].into_node()
}

const APP_FILES: &[Entry] = &[
    Entry {
        id: "page-rs",
        name: "page.rs",
        plain: "If `layout.rs` is the picture frame, `page.rs` is the picture: the actual content a visitor came to see at one address.",
        does: "Makes its folder a page with a URL. It must export a public function named exactly `Page` that returns the HTML, written with the view macros. `app/page.rs` is the home page, `app/about/page.rs` is `/about`.",
        touch: "Every time you change what a page says or shows, and every time you add a new page: new folder, new `page.rs`.",
        missing: "The folder has no address of its own. It can still hold layouts and sub-folders, but visiting its URL shows the 404 page.",
        intro: "This file creates the page `/about`. Save it as `app/about/page.rs`:",
        lang: "rust",
        example: r#"// app/about/page.rs  →  /about
use next_rust::prelude::*;

pub fn Page() -> impl View {
    main![h1!["About us"], p!["We build fast things."]]
}"#,
    },
    Entry {
        id: "layout-rs",
        name: "layout.rs",
        plain: "The picture frame. The header, the menu and the footer that stay the same while the picture inside changes.",
        does: "Wraps the page in its folder and every page in the folders below it. It exports `Layout` and receives the inner page as `children`, which it must place somewhere. Layouts nest: a page is wrapped by every `layout.rs` between it and `app/`. The framework writes `<html>`, `<head>` and `<body>` itself, so the root layout only renders what goes inside the body.",
        touch: "To change something that appears on many pages at once: navigation, footer, a sidebar for one section, the site title and description (`metadata`).",
        missing: "Nothing breaks. Pages are shown without a shared wrapper. Most apps want at least `app/layout.rs`.",
        intro: "This root layout puts a header above every page and a footer below it. It goes in `app/layout.rs`:",
        lang: "rust",
        example: r#"// app/layout.rs
use next_rust::prelude::*;

pub fn Layout(children: Children) -> impl View {
    div![
        header![nav![Link!(href = "/", "Home"), Link!(href = "/about", "About")]],
        main![children],              // the current page appears here
        footer!["© 2026 My Company"],
    ]
}"#,
    },
    Entry {
        id: "template-rs",
        name: "template.rs",
        plain: "A frame that is taken down and hung up again for every visit, where a layout stays on the wall.",
        does: "Works like `layout.rs` (it exports `Template` and receives `children`) and is rendered inside the layout of the same folder. The difference shows when a visitor clicks a link: layouts stay on screen untouched, templates are created again.",
        touch: "Rarely. Use it when something must start fresh on every navigation, such as an entrance animation or a wrapper whose state should reset.",
        missing: "Nothing changes. Most projects never need one.",
        intro: "This template wraps every page of the folder in a container that is rebuilt on each navigation. It goes next to a `layout.rs`, for example in `app/shop/template.rs`:",
        lang: "rust",
        example: r#"// app/shop/template.rs
use next_rust::prelude::*;

pub fn Template(children: Children) -> impl View {
    div![class("page-enter"), children]
}"#,
    },
    Entry {
        id: "loading-rs",
        name: "loading.rs",
        plain: "The \"your order is being prepared\" sign a restaurant shows while the kitchen is still cooking.",
        does: "Shown immediately while the page below it is still fetching its data. The server sends this placeholder first and then streams the real content into its place (streaming = sending a page in pieces as they become ready). It exports `Loading`, and it must be a plain `fn`, not `async`: it has to be ready instantly.",
        touch: "When a page loads data that can be slow (a database query, another API) and you want visitors to see something right away.",
        missing: "The browser waits until the whole page is ready before it shows anything. Fine for fast pages.",
        intro: "This placeholder is shown while any page under `/dashboard` loads. It goes in `app/dashboard/loading.rs`:",
        lang: "rust",
        example: r#"// app/dashboard/loading.rs
use next_rust::prelude::*;

pub fn Loading() -> impl View {
    p![aria("busy", "true"), "Loading the dashboard…"]
}"#,
    },
    Entry {
        id: "error-rs",
        name: "error.rs",
        plain: "The safety net under the trapeze: when a page falls, visitors land on a friendly message, not on a blank screen.",
        does: "Catches errors from the pages and layouts below its folder and shows this view in their place, while the surrounding layout stays intact. It exports `ErrorBoundary` (not `Error`), receives an `ErrorInfo` with `status`, `message` and `digest`, and must not be `async`. It does not catch errors of the `layout.rs` in its own folder; the next `error.rs` further up does.",
        touch: "When you want your own wording and design for \"something went wrong\" in a section of the site.",
        missing: "The error travels up to the nearest `error.rs` above, then to `global-error.rs`, and finally to a plain built-in error page that shows a reference code (and the details, in development only).",
        intro: "This error view replaces a failed page under `/dashboard` and shows the reference code to quote to support. It goes in `app/dashboard/error.rs`:",
        lang: "rust",
        example: r#"// app/dashboard/error.rs
use next_rust::prelude::*;

pub fn ErrorBoundary(info: ErrorInfo) -> impl View {
    div![
        role("alert"),
        h2!["Something went wrong"],
        p![format!("Reference: {}", info.digest)],
    ]
}"#,
    },
    Entry {
        id: "not-found-rs",
        name: "not-found.rs",
        plain: "The \"no such address\" sign a visitor sees after following a broken link.",
        does: "The 404 page. It is shown for URLs that match nothing, and whenever your code says \"this does not exist\" by returning `not_found()`, for example for a blog post that was deleted. It exports `NotFound`. A `not-found.rs` inside a folder handles `not_found()` raised below that folder.",
        touch: "To give the 404 page your own design, a search box or a link back home. A new project already has one.",
        missing: "A plain built-in page that says \"404: This page could not be found.\"",
        intro: "This 404 page offers a way back to the home page. It goes in `app/not-found.rs`:",
        lang: "rust",
        example: r#"// app/not-found.rs
use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    main![h1!["404"], p!["There's nothing at this address."], Link!(href = "/", "Back to home")]
}"#,
    },
    Entry {
        id: "global-error-rs",
        name: "global-error.rs",
        plain: "The fire exit: the last thing that still works when everything else, even the outer frame, has failed.",
        does: "The last-resort error page. It is used when an error reaches the top without being caught by any `error.rs`, including errors of the root layout itself. It exports `GlobalError`, takes an `ErrorInfo`, must not be `async`, and is only allowed directly in `app/`.",
        touch: "Once, to make even the worst-case error page look like your site.",
        missing: "A plain built-in page: \"Something went wrong\" with a reference code in production, and the full error message in development.",
        intro: "This page is shown when nothing else could handle an error. It goes in `app/global-error.rs` and nowhere else:",
        lang: "rust",
        example: r#"// app/global-error.rs
use next_rust::prelude::*;

pub fn GlobalError(info: ErrorInfo) -> impl View {
    main![h1!["We'll be right back"], p![format!("Error reference: {}", info.digest)]]
}"#,
    },
    Entry {
        id: "route-rs",
        name: "route.rs",
        plain: "A service hatch for machines. Pages are for people to read; a `route.rs` hands raw data to apps, scripts and other servers.",
        does: "An API endpoint (a URL that answers with data such as JSON, not with a web page). You export one function per HTTP method you support: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`. Each takes nothing or one `Request`. `HEAD` and `OPTIONS` are answered for you. A folder can hold a `route.rs` or a `page.rs`, not both for the same URL (that is build error `NR0104`).",
        touch: "When a mobile app, a webhook, a script or your own JavaScript needs data from the server. A new project has one example: `app/api/hello/route.rs`.",
        missing: "There is simply no API at that URL. Pages don't need one.",
        intro: "This endpoint answers `GET /api/hello?name=Ada` with `{\"hello\":\"Ada\"}`. It is the file a new project ships in `app/api/hello/route.rs`:",
        lang: "rust",
        example: r#"// app/api/hello/route.rs  →  /api/hello
use next_rust::prelude::*;

pub async fn GET(req: Request) -> Response {
    let name = req.query_param("name").unwrap_or_else(|| "world".into());
    Response::json(&serde_json::json!({ "hello": name }))
}"#,
    },
    Entry {
        id: "middleware-rs",
        name: "middleware.rs",
        plain: "The security guard at the door: every visitor passes the guard before reaching any room, and the guard may send them elsewhere.",
        does: "Code that runs before the page or API does. `app/middleware.rs` sees every request, before the URL is even matched, so it can redirect or rewrite it. A `middleware.rs` in a sub-folder guards only that folder and what is below it; outer guards run first. It exports `middleware(req: Request, next: Next)` and either answers itself or calls `next.run(req).await` to let the request through.",
        touch: "For rules that apply to many pages at once: \"must be signed in to see `/admin`\", logging, extra headers, rate limits.",
        missing: "Requests go straight to the page. Nothing else changes.",
        intro: "This guard sends visitors without a session cookie to the login page and lets everyone else through. Saved as `app/admin/middleware.rs`, it protects `/admin` and every page below it:",
        lang: "rust",
        example: r#"// app/admin/middleware.rs
use next_rust::prelude::*;

pub async fn middleware(req: Request, next: Next) -> Response {
    if req.cookies().get("session").is_none() {
        return Response::redirect("/login");
    }
    next.run(req).await
}"#,
    },
    Entry {
        id: "metadata-rs",
        name: "metadata.rs",
        plain: "The label on the outside of the box: the title in the browser tab and the description search engines and link previews show.",
        does: "Sets the document metadata (title, description, icons, social preview) for its folder. It exports `metadata`. You can also export the same `metadata` function from `page.rs` or `layout.rs`; a separate `metadata.rs` wins for its folder. Values merge from the outside in, and the innermost one wins.",
        touch: "To change how a page is named in the browser tab, in Google results or when shared in a chat.",
        missing: "The page inherits the metadata of the layouts above it.",
        intro: "This sets the tab title and the search-result description of `/pricing`. It goes in `app/pricing/metadata.rs`, or as the same function inside `app/pricing/page.rs`:",
        lang: "rust",
        example: r#"// app/pricing/metadata.rs
use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Pricing").description("Plans and pricing")
}"#,
    },
    Entry {
        id: "sitemap-and-robots",
        name: "sitemap.rs, robots.rs",
        plain: "The note you leave for search engines: a list of your pages, and which doors they may open.",
        does: "`app/sitemap.rs` generates `/sitemap.xml` and `app/robots.rs` generates `/robots.txt`. Both are only allowed directly in `app/`. A new project ships a fixed `public/robots.txt` that allows everything; routes win over files in `public/`, so adding `robots.rs` replaces it.",
        touch: "Before launch, when you care how search engines crawl the site, or when the list of pages comes from a database.",
        missing: "No `/sitemap.xml`; `/robots.txt` is whatever file you keep in `public/`.",
        intro: "These two functions publish a sitemap with two URLs and a robots file that points to it. The first goes in `app/sitemap.rs`, the second in `app/robots.rs`:",
        lang: "rust",
        example: r#"// app/sitemap.rs  →  /sitemap.xml
pub async fn sitemap() -> Sitemap {
    Sitemap::new().url("https://acme.dev/").url("https://acme.dev/pricing")
}

// app/robots.rs  →  /robots.txt
pub fn robots() -> Robots {
    Robots::allow_all().sitemap("https://acme.dev/sitemap.xml")
}"#,
    },
    Entry {
        id: "page-html",
        name: "page.html",
        plain: "A ready-printed poster: you hang it up as it is, no programming involved.",
        does: "A page written in plain HTML, for content nobody needs to compute. A fragment (no `<html>` tag) is wrapped in your layouts like any page; a complete document is served exactly as written, without layouts. A folder with both `page.rs` and `page.html` is build error `NR0101`, unless `[app] html_precedence` says which one wins.",
        touch: "For legal texts, a landing page from a designer, or any page a non-programmer maintains.",
        missing: "Nothing. It is an alternative to `page.rs`, not an addition.",
        intro: "This HTML fragment becomes the page `/terms`, inside your normal layout. It goes in `app/terms/page.html`:",
        lang: "html",
        example: r#"<!-- app/terms/page.html  →  /terms -->
<h1>Terms of service</h1>
<p>Plain HTML, no Rust needed.</p>"#,
    },
];

const FOLDER_NAMES: &[Entry] = &[
    Entry {
        id: "dynamic-segment",
        name: "[slug]",
        plain: "A form letter with a blank: one template, and the part of the address in the blank decides whose letter it is.",
        does: "A folder name in square brackets matches any single piece of the URL and hands it to your page as a parameter. `app/blog/[slug]/page.rs` serves `/blog/hello`, `/blog/news` and every other `/blog/…`; the page reads the value with `params.get(\"slug\")`. The name inside the brackets is yours to choose, as long as it is a valid identifier (letters, digits, `_`).",
        touch: "Whenever many pages share one design and differ only in data: blog posts, products, user profiles.",
        missing: "You would need one folder per post. Exact folder names still win over a `[slug]` folder, so `/blog/about` can have its own page.",
        intro: "This one file serves every blog post URL and prints the part of the address that was matched. It goes in `app/blog/[slug]/page.rs`:",
        lang: "rust",
        example: r#"// app/blog/[slug]/page.rs  →  /blog/hello, /blog/news, …
use next_rust::prelude::*;

pub fn Page(params: Params) -> impl View {
    h1![format!("Post: {}", params.get("slug").unwrap_or_default())]
}"#,
    },
    Entry {
        id: "catch-all-segment",
        name: "[...path]",
        plain: "A catch-all mailbox: whatever is written after this point in the address, however long, ends up here.",
        does: "Three dots inside the brackets match one or more URL pieces. `app/docs/[...path]/page.rs` serves `/docs/a`, `/docs/a/b` and `/docs/a/b/c`; the page gets all pieces with `params.get_all(\"path\")`. Double brackets, `[[...path]]`, also match zero pieces, so the same page serves `/docs` itself. A catch-all must be the last folder of a route (`NR0105`).",
        touch: "For deep, free-form URLs: documentation trees, file browsers, a CMS where editors invent paths.",
        missing: "Only URLs with a fixed number of pieces can be matched.",
        intro: "This page serves every URL below `/docs` and shows the pieces it received. It goes in `app/docs/[...path]/page.rs`:",
        lang: "rust",
        example: r#"// app/docs/[...path]/page.rs  →  /docs/a, /docs/a/b, …
use next_rust::prelude::*;

pub fn Page(params: Params) -> impl View {
    let parts = params.get_all("path").unwrap_or_default();
    h1![parts.join(" / ")]
}"#,
    },
    Entry {
        id: "route-group",
        name: "(group)",
        plain: "A binder on the shelf: it keeps related papers together, but the binder's label is not part of any paper's title.",
        does: "A folder name in parentheses organises files without showing up in the URL. `app/(marketing)/pricing/page.rs` is `/pricing`, not `/marketing/pricing`. A group can have its own `layout.rs`, so marketing pages and shop pages can look different while living at the same URL level.",
        touch: "When the `app/` folder gets crowded, or when two sets of pages at the same URL level need different layouts.",
        missing: "Nothing. Groups are optional tidiness. Two groups that produce the same URL are build error `NR0102`.",
        intro: "This tree gives the marketing pages one layout and the shop pages another, and no URL contains a group name:",
        lang: "text",
        example: r"app/
├── (marketing)/
│   ├── layout.rs            layout for marketing pages only
│   └── pricing/page.rs      → /pricing
└── (shop)/
    ├── layout.rs            layout for shop pages only
    └── cart/page.rs         → /cart",
    },
    Entry {
        id: "parallel-slot",
        name: "@slot",
        plain: "A dashboard with several screens on one wall: each screen shows its own programme, and the wall decides where each one hangs.",
        does: "A folder starting with `@` is a named region that the layout next to it places on the page. The layout takes `Slots` and calls `slots.take(\"analytics\")` for `app/dashboard/@analytics/`. Slots never create URLs. Inside a slot, `page.rs` is the content for the matching URL and `default.rs` (which also exports `Page`) is the fallback for every other URL of that section.",
        touch: "For pages made of independent panels: a dashboard with a feed and a chart, a mail app with a list and a reading pane.",
        missing: "The layout only has `children`. A slot without a matching page and without `default.rs` renders nothing.",
        intro: "This layout shows the current page in the middle and the `@analytics` slot in a side column. It goes in `app/dashboard/layout.rs`, next to the folder `app/dashboard/@analytics/`:",
        lang: "rust",
        example: r#"// app/dashboard/layout.rs
use next_rust::prelude::*;

pub fn Layout(children: Children, mut slots: Slots) -> impl View {
    div![main![children], aside![slots.take("analytics")]]
}"#,
    },
    Entry {
        id: "private-folder",
        name: "_private",
        plain: "The staff-only room: it is inside the building, but it is not on the visitor map.",
        does: "A folder whose name starts with `_` (or `.`) is never turned into a URL, whatever it contains. Use it for shared building blocks that live next to the pages using them. Other `.rs` files with non-special names are ignored by routing as well, so helper modules are safe anywhere.",
        touch: "When you want components close to the pages that use them, without risking a URL by accident.",
        missing: "Nothing. Shared code can also live in `src/`.",
        intro: "In this tree only `page.rs` becomes a URL; the `_components` folder is invisible to visitors:",
        lang: "text",
        example: r"app/blog/
├── page.rs                  → /blog
└── _components/
    └── post_card.rs         a helper, never a URL",
    },
];

const OUTSIDE_APP: &[Entry] = &[
    Entry {
        id: "public",
        name: "public/",
        plain: "The shop counter: whatever you lay on it is handed to visitors exactly as it is.",
        does: "Files in `public/` are served from the top of the site: `public/favicon.svg` is `/favicon.svg`, `public/robots.txt` is `/robots.txt`. No processing, no renaming. If a page or API route has the same address as a file, the route wins. In a production build the files are packed inside the binary, so there is no folder to upload.",
        touch: "To add or replace a logo, the favicon, images, a PDF, or a verification file some service asks you to host.",
        missing: "Nothing breaks; the site just has no static files.",
        intro: "An image placed at `public/team.jpg` is available at `/team.jpg`. This line, in any page, shows it:",
        lang: "rust",
        example: r#"img![src("/team.jpg"), alt("The team in front of the office")]"#,
    },
    Entry {
        id: "src-main-rs",
        name: "src/main.rs",
        plain: "The ignition key. It is tiny, and its only job is to start the engine that everything else is part of.",
        does: "The entry point of the program. In a new project it is one line, `next_rust::app!();`, which pulls in the route table that `build.rs` generated and starts the web server.",
        touch: "Almost never. Replace the one-liner with your own `main` when the server needs setup before it starts: a database connection, global middleware, background jobs.",
        missing: "There is no program to build: `cargo` reports that the package has no target.",
        intro: "This is the entire file a new project gets as `src/main.rs`:",
        lang: "rust",
        example: r"// src/main.rs
next_rust::app!();",
    },
    Entry {
        id: "src-lib-rs",
        name: "src/lib.rs",
        plain: "A second door into the same building, used by the inspectors (your automated tests) and not by visitors.",
        does: "Optional, and not created by `next-rust new`. When it exists, it holds `next_rust::routes!();` and declares your modules, which makes the whole app usable as a library. Tests can then load pages without starting a server, and `src/main.rs` shrinks to a `main` that runs those routes.",
        touch: "When you start writing tests with `TestClient`, or when `src/` grows several modules.",
        missing: "Nothing, until you want tests: they need the routes to be reachable from a library.",
        intro: "With this split, `src/lib.rs` owns the routes and the modules, and `src/main.rs` only starts them (`my_app` is your package name with `-` written as `_`):",
        lang: "rust",
        example: r"// src/lib.rs
pub mod actions;
next_rust::routes!();

// src/main.rs
fn main() {
    next_rust::run(my_app::routes());
}",
    },
    Entry {
        id: "src-actions-rs",
        name: "src/actions.rs and other modules",
        plain: "The back office: visitors never walk in here, but this is where forms get processed and records get filed.",
        does: "Everything in `src/` is ordinary Rust that your pages call: database code, helpers, shared components, and server actions. A server action is a function marked `#[server_action]` that a form can submit to. The build finds actions anywhere in `src/` and `app/`; `actions.rs` is a convention, not a required name. Declare the module once (`mod actions;` in `src/main.rs`, or `pub mod actions;` in `src/lib.rs`), and reach it from pages as `crate::actions::…`, never through your package's own name.",
        touch: "Whenever a form or button has to change something on the server, or when code is shared by several pages.",
        missing: "Nothing. A site of pure pages needs no `src/` code beyond `main.rs`.",
        intro: "This action receives the fields of a sign-up form, rejects a bad email with a message for that field, and sends the visitor on to `/welcome`. It goes in `src/actions.rs`; a form uses it with `action!(crate::actions::signup)`:",
        lang: "rust",
        example: r#"// src/actions.rs
use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct Signup { email: String }

#[server_action]
pub async fn signup(input: Signup) -> Result<()> {
    if !input.email.contains('@') {
        return Err(Error::validation([("email", "Enter a valid email address")]));
    }
    Err(redirect("/welcome"))
}"#,
    },
    Entry {
        id: "build-rs",
        name: "build.rs",
        plain: "The inspector who walks through the building before it opens and writes the directory board in the lobby.",
        does: "A small program Cargo runs before compiling yours. Its one line, `next_rust_build::generate()`, reads the `app/` folder, checks every special file (names, exports, conflicting URLs) and writes the route table as Rust code. This is why mistakes show up as build errors with an `NR` code and not as broken pages later.",
        touch: "Almost never. Only to register build plugins or configure Tailwind from Rust, with `Generator::new()…run()` in place of `generate()`.",
        missing: "No route table is generated, so the project does not compile.",
        intro: "This is the entire build script of a new project, in `build.rs` at the project root:",
        lang: "rust",
        example: r"// build.rs
fn main() {
    next_rust_build::generate();
}",
    },
    Entry {
        id: "next-rust-toml",
        name: "next-rust.toml",
        plain: "The settings panel of the site: switches and dials, no programming.",
        does: "Project-wide configuration: the port, the Tailwind theme (brand colors), redirects, extra headers, security options, analytics and site verification. Every setting is optional and has a default. Unknown keys are errors, so a typo is reported by name and never silently ignored. A production binary carries its configuration inside it.",
        touch: "To change the port, brand colors, add a redirect from an old URL, turn on analytics or tighten security headers.",
        missing: "Every default applies: `app/` for routes, `public/` for files, port 3000, Tailwind off.",
        intro: "These lines, in `next-rust.toml` at the project root, move the server to port 8080 and send visitors of `/old` to `/new`:",
        lang: "toml",
        example: r#"# next-rust.toml
[server]
port = 8080

[[redirects]]
source = "/old"
destination = "/new"
permanent = true"#,
    },
    Entry {
        id: "cargo-toml",
        name: "Cargo.toml",
        plain: "The ingredients list of the project: its name, its version and every library it is cooked with.",
        does: "The standard manifest of every Rust project, read by Cargo (Rust's build tool and package manager). A new project lists `next-rust` (the framework), `next-rust-build` (used by `build.rs` only), `serde` and `serde_json`, turns on warnings for unused code, and sets a size-optimised release profile. `Cargo.lock`, which appears next to it after the first build, records the exact versions that were used; commit it.",
        touch: "To add a library (a database driver, an HTTP client), or to rename the project. For the size and panic settings of `next-rust build`, edit `[build]` in `next-rust.toml`: the build command overrides `[profile.release]`.",
        missing: "It is not a Rust project any more; no `cargo` or `next-rust` command works.",
        intro: "These are the dependency sections of a new project's `Cargo.toml` (the version numbers follow the CLI you installed):",
        lang: "toml",
        example: r#"# Cargo.toml
[dependencies]
next-rust = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[build-dependencies]
next-rust-build = "0.1""#,
    },
    Entry {
        id: "env-files",
        name: ".env, .env.example",
        plain: "The key cabinet: passwords and addresses the program needs, kept out of the code and out of the shop window.",
        does: "Text files of `NAME=value` lines, loaded when the app starts. `.env.example` is a template without real secrets, meant to be committed and copied. Files are read in this order, later ones winning: `.env`, `.env.<environment>`, `.env.local`, `.env.<environment>.local`; real environment variables of the machine always win. Only variables starting with `NEXT_RUST_PUBLIC_` are ever sent to the browser.",
        touch: "To store a database URL, an API key or any value that differs between your laptop and the server.",
        missing: "Nothing, as long as your code does not expect a variable. On servers, real environment variables usually replace these files.",
        intro: "This is the `.env.example` of a new project. Copy it to `.env.local`, which the generated `.gitignore` already keeps out of Git, and fill in real values:",
        lang: "text",
        example: r"# Server-only secrets (never sent to the browser)
DATABASE_URL=postgres://localhost/app

# Values safe for client code must use the public prefix
NEXT_RUST_PUBLIC_SITE_NAME=My App",
    },
    Entry {
        id: "target",
        name: "target/",
        plain: "The workshop floor: sawdust, half-finished parts and jigs. Necessary while building, never part of the delivery.",
        does: "Cargo's build cache. It appears after the first build, can grow to several gigabytes, and makes every later build fast. `next-rust build` also writes two small reports to `target/next-rust/` that `next-rust analyze` reads.",
        touch: "Never by hand. Delete it (or run `cargo clean`) to free disk space; the next build recreates it, slowly.",
        missing: "The next build starts from zero and takes as long as the very first one.",
        intro: "It is already listed in the `.gitignore` of a new project, because it must not be committed:",
        lang: "text",
        example: r"/target
/.next-rust
.env*.local",
    },
    Entry {
        id: "dot-next-rust",
        name: ".next-rust/",
        plain: "The shipping box: the finished product, packed and ready to leave the workshop.",
        does: "The output folder. `next-rust build` puts one executable file here, named after your project, with the pages, the configuration and the `public/` files inside it. That single file is the whole deployment. `next-rust dev` keeps a small status file in the same folder. Change the location with `[build] output`.",
        touch: "You copy the binary out of it when you deploy. `next-rust clean` deletes the folder.",
        missing: "`next-rust start` refuses to run and asks you to build first.",
        intro: "These commands build the binary, show it, and run it the way a server would:",
        lang: "sh",
        example: r"next-rust build
ls .next-rust/          # my-app
./.next-rust/my-app     # serves http://localhost:3000",
    },
];

pub fn content() -> Node {
    fragment![
        div![
            class("callout"),
            p![
                strong!["In plain words."],
                " A Next Rust project is a folder of small text files, and each file has one job. The ",
                code!["app/"],
                " folder is the map of your website: every folder inside it is an address, and the files in a folder say what visitors see there. Everything outside ",
                code!["app/"],
                " is either a setting, a storage shelf, or machinery you rarely open. This page walks through all of it, one file at a time.",
            ],
        ],
        p![
            "Each entry has the same four boxes: what the file is in everyday words, what it does technically, when you would open it, and what happens when it isn't there. A short example follows. Terms that may be new: a ",
            strong!["route"],
            " is an address of your site, such as ",
            code!["/about"],
            "; the ",
            strong!["server"],
            " is the program that answers when a browser asks for an address; a ",
            strong!["binary"],
            " is that program as one finished, runnable file; to ",
            strong!["build"],
            " is to turn your text files into that binary.",
        ],
        p![
            strong!["Why it matters:"],
            " the file names below are a contract. The framework reads them when you build, so a misspelled name or a missing ",
            code!["pub"],
            " is reported immediately, with an error code and a suggested fix, and never turns into a page that is mysteriously missing later.",
        ],
        h2![id("the-tree"), a![class("anchor"), href("#the-tree"), "A new project at a glance"]],
        p![
            "This is everything ",
            code!["next-rust new my-app"],
            " creates, with one line on what each file is for. It is a picture of the folder, not something to type:",
        ],
        pre![code![
            class("language-text"),
            r"my-app/
├── app/                       YOUR WEBSITE: folders are addresses, files are what visitors see
│   ├── layout.rs              the frame around every page: title, shared header and footer
│   ├── page.rs                the home page  →  /
│   ├── not-found.rs           the 404 page for addresses that don't exist
│   └── api/
│       └── hello/
│           └── route.rs       a data endpoint for programs  →  /api/hello
├── public/                    files handed out as they are
│   ├── favicon.svg            the small icon in the browser tab  →  /favicon.svg
│   └── robots.txt             tells search engines they may read the site  →  /robots.txt
├── src/
│   └── main.rs                one line that starts the server
├── build.rs                   one line that reads app/ and checks it before every build
├── next-rust.toml             settings: port, colors, redirects, analytics
├── Cargo.toml                 the project's name and the libraries it uses
├── .env.example               a template for secrets such as a database address
├── .gitignore                 what Git must not store: target/, .next-rust/, .env*.local
├── README.md                  the three commands you need, for the next person
└── .vscode/                   editor snippets and settings (three files, optional)",
        ],],
        p![
            "With ",
            code!["--no-tailwind"],
            " there is one more file, ",
            code!["app/globals.css"],
            ". The default project has no CSS file at all: it is styled with ",
            a![href("/docs/tailwind"), "Tailwind CSS"],
            " classes, and the theme lives in ",
            code!["next-rust.toml"],
            ".",
        ],
        p!["Three more things appear on their own once you start working. None of them is written by hand:"],
        pre![code![
            class("language-text"),
            r"my-app/
├── Cargo.lock                 the exact library versions used (created by the first build; commit it)
├── target/                    Cargo's build cache (large, safe to delete, never committed)
└── .next-rust/                the output: after `next-rust build`, the one file you deploy",
        ],],
        div![
            class("callout"),
            p![
                strong!["The one rule to remember."],
                " Inside ",
                code!["app/"],
                ", only a fixed list of file names means something: ",
                code!["page.rs"],
                ", ",
                code!["layout.rs"],
                ", ",
                code!["route.rs"],
                " and the others below. Each must export a function with a fixed name (",
                code!["Page"],
                ", ",
                code!["Layout"],
                ", …) marked ",
                code!["pub"],
                ". Any other ",
                code![".rs"],
                " file is ignored by routing, and adding or renaming a file needs a rebuild, which ",
                code!["next-rust dev"],
                " does for you.",
            ],
        ],
        h2![id("app"), a![class("anchor"), href("#app"), "The ", code!["app/"], " folder"]],
        div![
            class("feature-list"),
            div![
                strong!["In plain words"],
                span![
                    "The floor plan of a building. Every folder is a room with an address on its door, and the files inside a room decide what a visitor finds there."
                ],
            ],
            div![
                strong!["What it does"],
                span![rich(
                    "The build reads this folder and turns it into the list of URLs your site answers. `app/page.rs` is `/`, `app/blog/page.rs` is `/blog`. Requests never search the disk: the list is compiled into the program."
                )],
            ],
            div![
                strong!["When you'd touch it"],
                span!["Constantly. Adding a page, an API endpoint or a section of the site all happen here."],
            ],
            div![
                strong!["If it's missing"],
                span![rich(
                    "The build stops with `NR0001` (app directory not found). To keep routes elsewhere, set `[app] directory` in `next-rust.toml`."
                )],
            ],
        ],
        p!["Folder paths map to URLs like this. The tree is an illustration of the rule, not a file to create:"],
        pre![code![
            class("language-text"),
            r"app/page.rs                     → /
app/about/page.rs               → /about
app/blog/page.rs                → /blog
app/blog/[slug]/page.rs         → /blog/anything
app/api/hello/route.rs          → /api/hello   (data, not a page)",
        ],],
        p![
            "You don't have to create these files by hand. While ",
            code!["next-rust dev"],
            " runs, an empty file with a special name is filled with a working starter, and ",
            code!["next-rust generate page /about"],
            " does the same from the terminal. ",
            a![href("/docs/routing"), "Routing"],
            " is the full reference for this folder.",
        ],
        h2![id("special-files"), a![class("anchor"), href("#special-files"), "Special files inside ", code!["app/"]]],
        p![
            "These are all the file names the framework recognises. A folder uses only the ones it needs; most folders contain just a ",
            code!["page.rs"],
            ".",
        ],
        APP_FILES.iter().map(entry).collect::<Vec<_>>(),
        h2![id("folder-names"), a![class("anchor"), href("#folder-names"), "Special folder names"]],
        p![
            "A normal folder name is simply part of the URL: ",
            code!["app/about/"],
            " is ",
            code!["/about"],
            ". Brackets, parentheses, ",
            code!["@"],
            " and ",
            code!["_"],
            " give a folder a special meaning.",
        ],
        FOLDER_NAMES.iter().map(entry).collect::<Vec<_>>(),
        p![
            "There is one more, rarer marker: a folder such as ",
            code!["(.)photo"],
            " is an intercepting route, which shows a different page for a URL when the visitor arrives by clicking a link inside the app (the classic use is a photo that opens in a popup from a feed, and as a full page when opened directly). See ",
            a![href("/docs/routing#intercepting-routes"), "Routing"],
            ".",
        ],
        h2![id("outside-app"), a![class("anchor"), href("#outside-app"), "Everything outside ", code!["app/"]]],
        p![
            "The rest of the project is static files, your own supporting code, settings, and folders the tools create.",
        ],
        OUTSIDE_APP.iter().map(entry).collect::<Vec<_>>(),
        h3![id("housekeeping"), a![class("anchor"), href("#housekeeping"), "Housekeeping files"]],
        p!["Three generated files have nothing to do with the framework at run time. All can be edited or deleted:"],
        ul![
            li![code![".gitignore"], " tells Git which files not to store: the two output folders and local secrets.",],
            li![
                code!["README.md"],
                " is a short note for the next person: what the project is and the three commands to run it.",
            ],
            li![
                code![".vscode/"],
                " holds snippets and settings for the VS Code editor (type ",
                code!["nrpage"],
                " in an empty file to get a page). Recreate it any time with ",
                code!["next-rust editor"],
                ".",
            ],
        ],
        h3![id("folders-you-may-add"), a![class("anchor"), href("#folders-you-may-add"), "Folders you may add later"]],
        p![
            "A new project doesn't have these, and many never need them. The framework recognises them at the project root:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["folder"], th!["in plain words"], th!["what it holds"]]],
                tbody![
                    tr![
                        td![code!["assets/"]],
                        td!["a shelf like ", code!["public/"], ", with a tamper-proof label on every item"],
                        td![rich(
                            "Files referenced with `asset!(\"fonts/inter.woff2\")`. They are served under a URL that contains a fingerprint of the content, so browsers can cache them forever and still get a new file when it changes."
                        )],
                    ],
                    tr![
                        td![code!["client/"]],
                        td!["the few scripts that run in the visitor's browser"],
                        td![rich(
                            "JavaScript modules for interactive islands (`#[client(script = \"chart.js\")]`), served from `/_next-rust/client/`."
                        )],
                    ],
                    tr![
                        td![code!["islands/"]],
                        td!["browser code written in Rust"],
                        td![rich(
                            "A separate small crate compiled to WebAssembly, used only with `[build] wasm_islands = true`."
                        )],
                    ],
                    tr![
                        td![code!["tests/"]],
                        td!["the inspectors' checklists"],
                        td![rich(
                            "Ordinary Rust tests that load pages through `TestClient`, without starting a server."
                        )],
                    ],
                ],
            ],
        ],
        p![
            "See ",
            a![href("/docs/styling-and-assets"), "Styling & assets"],
            ", ",
            a![href("/docs/client"), "Client & navigation"],
            " and ",
            a![href("/docs/testing"), "Testing"],
            ".",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["A wrong export name."],
                " ",
                code!["error.rs"],
                " exports ",
                code!["ErrorBoundary"],
                ", not ",
                code!["Error"],
                "; ",
                code!["not-found.rs"],
                " exports ",
                code!["NotFound"],
                ". A wrong name or a missing ",
                code!["pub"],
                " is build error ",
                code!["NR0201"],
                ".",
            ],
            li![
                strong![code!["async"], " where it isn't allowed."],
                " ",
                code!["Loading"],
                ", ",
                code!["ErrorBoundary"],
                " and ",
                code!["GlobalError"],
                " must be plain functions (",
                code!["NR0204"],
                "). Pages, layouts and ",
                code!["load"],
                " may be async.",
            ],
            li![
                strong!["Expecting a new file to appear without a rebuild."],
                " The route list is made at build time. ",
                code!["next-rust dev"],
                " rebuilds for you; a plain ",
                code!["cargo run"],
                " may not notice a new file.",
            ],
            li![
                strong!["Using the package name inside ", code!["app/"], "."],
                " Files under ",
                code!["app/"],
                " are part of your own crate. Write ",
                code!["crate::db::posts()"],
                ", not ",
                code!["my_app::db::posts()"],
                ".",
            ],
            li![
                strong!["Inventing a configuration key."],
                " Unknown keys in ",
                code!["next-rust.toml"],
                " stop the build. Check ",
                a![href("/docs/configuration"), "Configuration"],
                " for the real names.",
            ],
            li![
                strong!["Editing generated code or output folders."],
                " Nothing in ",
                code!["target/"],
                " or ",
                code![".next-rust/"],
                " is yours to edit; both are rebuilt from your source files.",
            ],
            li![
                strong!["Committing secrets."],
                " The generated ",
                code![".gitignore"],
                " covers ",
                code![".env*.local"],
                ", not a plain ",
                code![".env"],
                ". Keep secrets in ",
                code![".env.local"],
                " or add ",
                code![".env"],
                " to ",
                code![".gitignore"],
                "; ",
                code!["next-rust doctor"],
                " checks this for you.",
            ],
        ],
        h2![id("i-want-to"), a![class("anchor"), href("#i-want-to"), "I want to… → open this file"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["I want to…"], th!["open or create"]]],
                tbody![
                    want("change the text on the home page", "`app/page.rs`"),
                    want("add a new page at `/about`", "create `app/about/page.rs`"),
                    want("change the header, menu or footer of every page", "`app/layout.rs`"),
                    want("give one section its own sidebar", "a `layout.rs` in that section's folder"),
                    want(
                        "change the title in the browser tab or the description in search results",
                        "the `metadata` function in `app/layout.rs` (whole site) or in a page (one page)"
                    ),
                    want("redesign the \"page not found\" screen", "`app/not-found.rs`"),
                    want(
                        "show a friendly message when a page crashes",
                        "an `error.rs` in that section, and `app/global-error.rs`"
                    ),
                    want("show something while a slow page loads", "a `loading.rs` next to the page"),
                    want("make one page design serve every blog post", "`app/blog/[slug]/page.rs`"),
                    want(
                        "offer data to a mobile app or a script",
                        "a `route.rs`, for example `app/api/orders/route.rs`"
                    ),
                    want("process a form", "a `#[server_action]` function in `src/actions.rs`"),
                    want("require a login for a section", "a `middleware.rs` in that section's folder"),
                    want("replace the logo or favicon, add an image or a PDF", "the `public/` folder"),
                    want("change the brand colors", "`[tailwind.theme]` in `next-rust.toml`"),
                    want("change the port", "`[server] port` in `next-rust.toml`"),
                    want("redirect an old address to a new one", "`[[redirects]]` in `next-rust.toml`"),
                    want(
                        "add Google Analytics or verify the site with a search engine",
                        "`[analytics]` and `[verification]` in `next-rust.toml`"
                    ),
                    want("store a database password or an API key", "`.env.local` (start from `.env.example`)"),
                    want("use another Rust library", "`[dependencies]` in `Cargo.toml`"),
                    want("connect to a database before the server starts", "`src/main.rs`, with your own `main`"),
                    want("see every address the site answers", "nothing: run `next-rust routes`"),
                    want("put the site on a server", "run `next-rust build`, then copy the file in `.next-rust/`"),
                ],
            ],
        ],
        h2![id("where-next"), a![class("anchor"), href("#where-next"), "Where to go next"]],
        div![
            class("card-grid"),
            a![
                class("card"),
                href("/docs/routing"),
                strong!["Routing"],
                span!["The full rules of the app folder: ranking, conflicts, slots and interception."],
            ],
            a![
                class("card"),
                href("/docs/views"),
                strong!["Views & components"],
                span!["How to write the HTML inside a page or layout."],
            ],
            a![
                class("card"),
                href("/docs/rendering"),
                strong!["Rendering & data"],
                span!["Loading data, loading and error states, redirects and metadata."],
            ],
            a![
                class("card"),
                href("/docs/configuration"),
                strong!["Configuration"],
                span!["Every key of next-rust.toml and how .env files are read."],
            ],
        ],
    ]
}
