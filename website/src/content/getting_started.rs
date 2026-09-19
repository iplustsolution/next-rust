//! The "getting-started" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        h2![id("requirements"), a![class("anchor"), href("#requirements"), "Requirements"]],
        ul![
            li!["Rust ", strong!["1.89"], " or newer (", code!["rustup update stable"], ")"],
            li!["macOS, Linux or Windows"],
        ],
        p!["Next Rust does not need Node.js."],
        h2![id("install-the-cli"), a![class("anchor"), href("#install-the-cli"), "Install the CLI"]],
        pre![code![
            class("language-sh"),
            r"cargo install --git https://github.com/iplustsolution/next-rust next-rust-cli",
        ],],
        p![
            "This installs the ",
            code!["next-rust"],
            " binary. Don't have Rust yet? See the ",
            a![
                href("https://github.com/iplustsolution/next-rust/blob/main/README.md#getting-started"),
                target("_blank"),
                rel("noopener"),
                "installation steps in the README",
            ],
            ".",
        ],
        h2![id("create-a-project"), a![class("anchor"), href("#create-a-project"), "Create a project"]],
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
        p![
            "New projects depend on the GitHub repository. Update the project and the CLI with ",
            code!["next-rust upgrade"],
            "; see ",
            a![href("/docs/cli#upgrade"), "the CLI reference"],
            ". To develop against a local clone instead, use ",
            code!["next-rust new my-app --framework-path /path/to/next-rust"],
            ".",
        ],
        h2![id("what-was-generated"), a![class("anchor"), href("#what-was-generated"), "What was generated"]],
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
        h2![id("your-first-page"), a![class("anchor"), href("#your-first-page"), "Your first page"]],
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
        p!["A dynamic route:"],
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
        p!["An API endpoint:"],
        pre![code![
            class("language-rust"),
            r#"// app/api/time/route.rs
use next_rust::prelude::*;

pub async fn GET() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "unix": std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }))
}"#,
        ],],
        h2![id("inspect-routes"), a![class("anchor"), href("#inspect-routes"), "Inspect routes"]],
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
        h2![
            id("build-and-run-for-production"),
            a![class("anchor"), href("#build-and-run-for-production"), "Build and run for production"],
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
        pre![code![
            class("language-toml"),
            r#"# Cargo.toml
[dependencies]
next-rust = "0.0"

[build-dependencies]
next-rust-build = "0.0""#,
        ],],
        pre![code![
            class("language-rust"),
            r"// build.rs
fn main() { next_rust_build::generate(); }",
        ],],
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
        p!["Next: ", a![href("/docs/routing"), "Routing"], "."],
    ]
}
