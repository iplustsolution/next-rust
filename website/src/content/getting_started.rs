//! The "getting-started" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    // New projects from crates.io depend on `major.minor` of the CLI's version.
    let requirement = env!("CARGO_PKG_VERSION").splitn(3, '.').take(2).collect::<Vec<_>>().join(".");
    fragment![
        div![
            class("callout"),
            p![
                strong!["In plain words."],
                " You will install one tool, ask it to create a starter website, and watch that site in your browser while you change it. At the end you will turn the site into a single program file that can run on any server. You type a handful of commands into a terminal (the text window where you run commands); everything else is editing text files.",
            ],
        ],
        p![
            "This page takes you from nothing to a running app and a production binary. It is six short steps, and the first build is the only slow one:",
        ],
        ol![
            li![a![href("#install-the-cli"), "Install the CLI"]],
            li![a![href("#create-a-project"), "Create a project and run it"]],
            li![a![href("#what-was-generated"), "Look at what was generated"]],
            li![a![href("#your-first-page"), "Add a page, a dynamic route and an API endpoint"]],
            li![a![href("#inspect-routes"), "Check your routes"]],
            li![a![href("#build-and-run-for-production"), "Build the production binary"]],
        ],
        p![
            "Already know Next.js? Keep ",
            a![href("/docs/coming-from-nextjs"), "Coming from Next.js"],
            " open in another tab.",
        ],
        h2![id("requirements"), a![class("anchor"), href("#requirements"), "Before you start"]],
        ul![
            li!["Rust ", strong!["1.89"], " or newer (", code!["rustup update stable"], ")"],
            li!["macOS, Linux or Windows"],
        ],
        p![
            "Next Rust does not need Node.js. Two terms used below: the ",
            strong!["CLI"],
            " (command-line interface) is the ",
            code!["next-rust"],
            " command you type in a terminal, and ",
            strong!["Cargo"],
            " is Rust's own build tool and package manager, installed together with Rust.",
        ],
        h2![id("install-the-cli"), a![class("anchor"), href("#install-the-cli"), "1. Install the CLI"]],
        p![
            "Run this in a terminal, in any folder. It downloads the CLI from crates.io (Rust's package registry) and compiles it, which takes a few minutes the first time:",
        ],
        pre![code![class("language-sh"), r"cargo install next-rust-cli",],],
        p!["To use the newest code from the main branch instead, install from GitHub:"],
        pre![code![
            class("language-sh"),
            r"cargo install --git https://github.com/iplustsolution/next-rust next-rust-cli",
        ],],
        p![
            "Either command installs the ",
            code!["next-rust"],
            " binary; check it with ",
            code!["next-rust --version"],
            ". Don't have Rust yet? See the ",
            a![
                href("https://github.com/iplustsolution/next-rust/blob/main/README.md#getting-started"),
                target("_blank"),
                rel("noopener"),
                "installation steps in the README",
            ],
            ".",
        ],
        h2![id("create-a-project"), a![class("anchor"), href("#create-a-project"), "2. Create a project and run it"]],
        p![
            "These three commands create a folder ",
            code!["my-app"],
            " with a starter site in it, move into that folder, and start the development server (a local copy of the site that rebuilds itself whenever you save a file):",
        ],
        pre![code![
            class("language-sh"),
            r"next-rust new my-app
cd my-app
next-rust dev",
        ],],
        p![
            "Open ",
            a![href("http://localhost:3000"), target("_blank"), rel("noopener"), "http://localhost:3000"],
            ". Edit ",
            code!["app/page.rs"],
            " and save: the dev server rebuilds and the browser reloads.",
        ],
        p![
            "Adding a page is just creating a file. While ",
            code!["next-rust dev"],
            " runs, create an empty ",
            code!["app/about/page.rs"],
            " and it's filled with a starter page, so ",
            code!["/about"],
            " works immediately. The same works for ",
            code!["layout.rs"],
            ", ",
            code!["route.rs"],
            ", ",
            code!["loading.rs"],
            " and the other special files. In VS Code, type ",
            code!["nrpage"],
            ", ",
            code!["nrroute"],
            " or ",
            code!["nrlayout"],
            " for snippets.",
        ],
        div![
            class("callout"),
            p![
                strong!["New files need a rebuild."],
                " The route table is generated at compile time, so a file added or renamed under ",
                code!["app/"],
                " only becomes a route after a build. ",
                code!["next-rust dev"],
                " does this for you; a plain ",
                code!["cargo run"],
                " does not notice new files unless the build script reruns.",
            ],
        ],
        p![
            "New projects depend on the framework from crates.io when this version is published there, and on the GitHub repository otherwise (or always, with ",
            code!["next-rust new my-app --git"],
            "). Update the project and the CLI with ",
            code!["next-rust upgrade"],
            "; see ",
            a![href("/docs/cli#upgrade"), "the CLI reference"],
            ". To develop against a local clone instead, use ",
            code!["next-rust new my-app --framework-path /path/to/next-rust"],
            ".",
        ],
        h2![id("what-was-generated"), a![class("anchor"), href("#what-was-generated"), "3. What was generated"]],
        p![
            "This is the content of the ",
            code!["my-app"],
            " folder, with a note on each file. It is a picture of the folder, not something to type. ",
            a![href("/docs/project-structure"), "Project structure"],
            " explains every one of these files in plain words: what it does, when you would open it, and what happens without it.",
        ],
        pre![code![
            class("language-text"),
            r"my-app/
├── Cargo.toml          # depends on next-rust, build-depends on next-rust-build
├── build.rs            # fn main() { next_rust_build::generate(); }
├── next-rust.toml      # configuration, including the Tailwind CSS theme
├── src/main.rs         # next_rust::app!();
├── public/
│   ├── favicon.svg     # the Next Rust logo (replace with yours)
│   └── robots.txt
├── .env.example
└── app/
    ├── layout.rs       # root layout and metadata
    ├── page.rs         # / — a single hero to replace with your own
    ├── not-found.rs    # 404 page
    └── api/hello/route.rs  # GET /api/hello",
        ],],
        p![
            "There are no CSS files: the pages are styled with ",
            a![href("/docs/tailwind"), "Tailwind CSS"],
            " classes, and the brand colors and custom classes are in the ",
            code!["[tailwind]"],
            " section of ",
            code!["next-rust.toml"],
            ". The first build downloads the Tailwind engine once for your machine. Prefer plain CSS? Create the project with ",
            code!["next-rust new my-app --no-tailwind"],
            ".",
        ],
        p!["Three pieces connect your files to the framework:"],
        ol![
            li![
                strong![code!["build.rs"]],
                " runs ",
                code!["next_rust_build::generate()"],
                ". It scans ",
                code!["app/"],
                ", validates the routes, reads the signatures of your special files, and writes Rust code to ",
                code!["$OUT_DIR/next_rust_routes.rs"],
                ".",
            ],
            li![
                strong![code!["src/main.rs"]],
                " calls ",
                code!["next_rust::app!()"],
                ". This includes the generated code and defines ",
                code!["main"],
                ".",
            ],
            li![
                strong!["Special files"],
                " like ",
                code!["page.rs"],
                " and ",
                code!["layout.rs"],
                " are ordinary Rust modules. You never register them anywhere.",
            ],
        ],
        h2![id("your-first-page"), a![class("anchor"), href("#your-first-page"), "4. Your first page"]],
        p![
            "A page is a file named ",
            code!["page.rs"],
            " in a folder under ",
            code!["app/"],
            "; the folder's path is the page's address. Create the file ",
            code!["app/products/page.rs"],
            " with this content. It sets the browser-tab title and renders a heading with a list of two products:",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/products/page.rs
use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Products")
}

pub fn Page() -> impl View {
    section![
        h1!["Products"],
        ul![each(["Keyboard", "Mouse"], |name| li![name])],
    ]
}"#,
        ],],
        p!["That's the whole route: ", code!["/products"], " now exists."],
        div![
            class("callout"),
            p![
                strong!["Files in the app directory are modules of your crate."],
                " Reach your own code with ",
                code!["crate::"],
                " (for example ",
                code!["crate::db::posts()"],
                "), never with the crate's own name. Export names such as ",
                code!["Page"],
                " and ",
                code!["Layout"],
                " are fixed and must be ",
                code!["pub"],
                "; a wrong name fails the build with ",
                a![href("/docs/diagnostics"), "NR0201"],
                ".",
            ],
        ],
        p![
            "A dynamic route is one file that serves many addresses. A folder name in square brackets matches any value, so this file, saved as ",
            code!["app/products/[id]/page.rs"],
            ", answers ",
            code!["/products/1"],
            ", ",
            code!["/products/42"],
            " and so on. It reads the number from the URL and shows it; a value that is not a number gives a 404:",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/products/[id]/page.rs
use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct P { id: u64 }

pub fn Page(Path(p): Path<P>) -> impl View {
    h1![format!("Product #{}", p.id)]
}"#,
        ],],
        p![
            "An API endpoint is a URL that answers with data for programs, not with a page for people. A file named ",
            code!["route.rs"],
            " makes one. This file, saved as ",
            code!["app/api/time/route.rs"],
            ", answers ",
            code!["GET /api/time"],
            " with the current time as JSON:",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/api/time/route.rs
use next_rust::prelude::*;

pub async fn GET() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "unix": std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }))
}"#,
        ],],
        h2![id("inspect-routes"), a![class("anchor"), href("#inspect-routes"), "5. Inspect routes"]],
        p![
            "Run this command in the project folder to see every address the app answers, which file serves it, which layouts wrap it, and whether the page is prepared once in advance (static) or rendered again for every visitor (dynamic). The lines after the first show the output:",
        ],
        pre![code![
            class("language-sh"),
            r"$ next-rust routes --layouts
METHOD  PATH            RENDER      SOURCE
GET     /               ○ static    app/page.rs
          └ app/layout.rs
GET     /products/:id   λ dynamic   app/products/[id]/page.rs
          └ app/layout.rs
          dynamic because: dynamic segment without generate_params
GET     /api/time       api         app/api/time/route.rs",
        ],],
        p![
            strong!["Why it matters:"],
            " this is the quickest way to confirm the framework read your folder names the way you meant them, before you open a browser.",
        ],
        h2![
            id("build-and-run-for-production"),
            a![class("anchor"), href("#build-and-run-for-production"), "6. Build and run for production"],
        ],
        p![
            "Run these two commands in the project folder. The first compiles the optimized program and prepares every static page; the second starts that program the way a server would:",
        ],
        pre![code![
            class("language-sh"),
            r"next-rust build    # one self-contained, stripped binary → .next-rust/my-app
next-rust start    # run it",
        ],],
        p![
            code!["next-rust build"],
            " prints which pages were pre-rendered (",
            code!["○"],
            "), which render per request (",
            code!["λ"],
            ") and which are API routes (",
            code!["ƒ"],
            "). The binary carries your pages, configuration and ",
            code!["public/"],
            " files, so you can copy ",
            code![".next-rust/my-app"],
            " to a server and run it with nothing else.",
        ],
        h2![
            id("adding-to-an-existing-cargo-project"),
            a![class("anchor"), href("#adding-to-an-existing-cargo-project"), "Adding to an existing Cargo project",],
        ],
        p![
            "An existing Rust project needs three small changes. First, add the framework and its build-time companion to ",
            code!["Cargo.toml"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            format!(
                "# Cargo.toml\n[dependencies]\nnext-rust = \"{requirement}\"\n\n[build-dependencies]\nnext-rust-build = \"{requirement}\""
            ),
        ],],
        p![
            "Second, create ",
            code!["build.rs"],
            " next to ",
            code!["Cargo.toml"],
            ". Cargo runs it before compiling; it reads ",
            code!["app/"],
            " and generates the route table:",
        ],
        pre![code![
            class("language-rust"),
            r"// build.rs
fn main() { next_rust_build::generate(); }",
        ],],
        p![
            "Third, make ",
            code!["src/main.rs"],
            " include the generated routes and start the server. This one line is the whole file:",
        ],
        pre![code![
            class("language-rust"),
            r"// src/main.rs
next_rust::app!();",
        ],],
        p![
            "Then create ",
            code!["app/page.rs"],
            ". Use ",
            code!["next_rust::routes!()"],
            " instead of ",
            code!["app!()"],
            " when you want your own ",
            code!["main"],
            " (custom middleware, background jobs, a library + binary split for testing). See ",
            a![href("/docs/testing"), "Testing"],
            ".",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["The new page gives a 404."],
                " The file must be named exactly ",
                code!["page.rs"],
                ", sit in a folder under ",
                code!["app/"],
                ", and the app must have been rebuilt. ",
                code!["next-rust dev"],
                " rebuilds on its own; ",
                code!["next-rust routes"],
                " shows whether the route exists.",
            ],
            li![
                strong!["The build fails with ", code!["NR0201"], "."],
                " The function has the wrong name or is not ",
                code!["pub"],
                ". A page exports ",
                code!["Page"],
                ", a layout ",
                code!["Layout"],
                ", a 404 page ",
                code!["NotFound"],
                " and an error boundary ",
                code!["ErrorBoundary"],
                ".",
            ],
            li![
                strong!["Your own module is \"not found\" from a page."],
                " Inside ",
                code!["app/"],
                ", write ",
                code!["crate::db::posts()"],
                ", never ",
                code!["my_app::db::posts()"],
                ".",
            ],
            li![
                strong!["A typo in ", code!["next-rust.toml"], "."],
                " Unknown keys stop the build and the message names the key. ",
                a![href("/docs/configuration"), "Configuration"],
                " lists the real ones.",
            ],
            li![
                strong!["Editing ", code!["[profile.release]"], " to change the production build."],
                " ",
                code!["next-rust build"],
                " applies its own size settings; change them under ",
                code!["[build]"],
                " in ",
                code!["next-rust.toml"],
                ".",
            ],
            li![
                strong!["Something outside your code looks wrong."],
                " ",
                code!["next-rust doctor"],
                " checks the Rust version, the configuration, the port and whether ",
                code![".env"],
                " is ignored by Git.",
            ],
        ],
        p![
            "Next: ",
            a![href("/docs/project-structure"), "Project structure"],
            " for a tour of every file, or ",
            a![href("/docs/routing"), "Routing"],
            " for the rules of the ",
            code!["app/"],
            " folder.",
        ],
    ]
}
