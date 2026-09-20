//! The "testing" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Applications are easiest to test with a ",
            strong!["library + binary"],
            " split: the library owns the routes, and tests call it directly without a network socket.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "A test is a small program that uses your site the way a visitor would — open this page, submit that form — and checks the answer, so you do not have to click through everything by hand after each change. Here the tests talk to the app directly inside one process: no server is started, no port is opened, no browser is needed. That makes them fast (milliseconds each) and lets them run anywhere, including CI (the automated checks that run on every push).",
            ],
        ],
        h2![id("project-layout"), a![class("anchor"), href("#project-layout"), "Project layout for tests"]],
        p![
            "A new project has only ",
            code!["src/main.rs"],
            " with ",
            code!["next_rust::app!();"],
            ". That is a program, and a test cannot reach inside a program. Move the routes into a library, which tests can import, and let the program call the library. Three small changes:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["file"], th!["what it is for"]]],
                tbody![
                    tr![
                        td![code!["src/lib.rs"]],
                        td!["the library: generates ", code!["routes()"], " and holds your shared modules. New file"],
                    ],
                    tr![td![code!["src/main.rs"]], td!["the program: one call that serves the library's routes"]],
                    tr![
                        td![code!["Cargo.toml"]],
                        td![
                            code!["[dev-dependencies]"],
                            ": packages used by tests only; they never reach the production binary"
                        ],
                    ],
                    tr![
                        td![code!["tests/"]],
                        td![
                            "a folder in the project root, next to ",
                            code!["src/"],
                            ". Every ",
                            code![".rs"],
                            " file in it is a test suite that ",
                            code!["cargo test"],
                            " finds by itself"
                        ],
                    ],
                ],
            ],
        ],
        p![
            "Create ",
            code!["src/lib.rs"],
            " (",
            code!["my_app"],
            " below is your package name with ",
            code!["-"],
            " written as ",
            code!["_"],
            "):",
        ],
        pre![code![
            class("language-rust"),
            r"// src/lib.rs
pub mod db;
next_rust::routes!();",
        ],],
        p!["Replace the content of ", code!["src/main.rs"], ":"],
        pre![code![
            class("language-rust"),
            r"// src/main.rs
fn main() {
    next_rust::run(my_app::routes());
}",
        ],],
        p![
            "Tests are ",
            code!["async"],
            " and need a runtime to run on. Add Tokio for tests only, in ",
            code!["Cargo.toml"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            r#"# Cargo.toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }"#,
        ],],
        h2![id("testclient"), a![class("anchor"), href("#testclient"), code!["TestClient"]]],
        p![
            code!["TestClient"],
            " is a pretend browser. It sends requests to your routes, remembers cookies between them like a real browser, and hands back the status, headers and HTML. Save this as ",
            code!["tests/pages.rs"],
            " and run ",
            code!["cargo test"],
            " from the project folder:",
        ],
        pre![code![
            class("language-rust"),
            r#"// tests/pages.rs
use next_rust::{http, TestClient};

#[tokio::test]
async fn home_page() {
    let client = TestClient::new(my_app::routes());
    let res = client.get("/").await;
    assert_eq!(res.status, 200);
    assert!(res.text.contains("<h1>Hello World</h1>"));
}

#[tokio::test]
async fn api_and_cookies() {
    let client = TestClient::new(my_app::routes());
    let created = client.post_json("/api/todos", &serde_json::json!({ "title": "Test" })).await;
    assert_eq!(created.status, 201);
    let todo: serde_json::Value = created.json();

    // Cookies set by responses are sent on later requests.
    client.post_form(&client.action_url("app/login/page.rs::login"), "username=ada").await;

    let req = http::Request::delete(format!("/api/todos/{}", todo["id"])).body(Default::default()).unwrap();
    assert_eq!(client.send(req).await.status, 204);
}"#,
        ],],
        p![
            "Each ",
            code!["#[tokio::test]"],
            " function is one test. ",
            code!["assert_eq!"],
            " and ",
            code!["assert!"],
            " state what must be true; if one is not, the test fails and prints the two values. Details:",
        ],
        ul![
            li![
                code!["TestClient::new"],
                " loads your ",
                code!["next-rust.toml"],
                " and uses the ",
                code!["test"],
                " environment, where static pages are cached as in production. ",
                code!["TestClient::production"],
                " enables production error messages. ",
                code!["TestClient::with(App::new(routes()).middleware(..), env)"],
                " tests a custom middleware stack.",
            ],
            li![
                code!["TestResponse"],
                " has ",
                code!["status"],
                ", ",
                code!["headers"],
                ", ",
                code!["text"],
                ", ",
                code!["chunks"],
                " (streamed chunks in order), ",
                code!["header(..)"],
                ", ",
                code!["json::<T>()"],
                " and ",
                code!["cookies()"],
                ".",
            ],
            li![
                "Action URLs: ",
                code!["client.action_url(\"src/actions.rs::create_user\")"],
                " signs a URL for that client, like the pages it loads do. You can also take the URL from a page the client fetched.",
            ],
        ],
        h2![
            id("assertions-for-common-framework-behaviour"),
            a![
                class("anchor"),
                href("#assertions-for-common-framework-behaviour"),
                "Assertions for common framework behaviour",
            ],
        ],
        p![
            "Lines to use inside a test, after ",
            code!["let res = client.get(..).await;"],
            ". They check streaming (the page arrives in pieces, the loading placeholder first), ISR (a static page served from the cache and refreshed on demand) and the complete, non-streamed HTML that search-engine crawlers get:",
        ],
        pre![code![
            class("language-rust"),
            r#"// streaming: fallback in the first chunk, content later
assert!(res.chunks[0].contains("Loading…"));

// ISR
assert_eq!(res.header("x-nr-cache"), Some("HIT"));
client.app().revalidate_tag("posts").await;

// crawlers get non-streamed HTML
let req = http::Request::get("/").header("user-agent", "Googlebot").body(Default::default()).unwrap();"#,
        ],],
        h2![
            id("static-generation-in-tests"),
            a![class("anchor"), href("#static-generation-in-tests"), "Static generation in tests"],
        ],
        p![
            "Static generation is the step of ",
            code!["next-rust build"],
            " that renders every static page ahead of time. Running it inside a test catches a page that fails to render without doing a full production build. ",
            code!["tempdir"],
            " is a temporary folder you create, so the test writes nothing into the project:",
        ],
        pre![code![
            class("language-rust"),
            r#"let mut config = next_rust::Config::discover(env!("CARGO_MANIFEST_DIR")).unwrap();
config.build.output = tempdir.clone();
let app = next_rust::App::new(my_app::routes()).config(config).environment(next_rust::Environment::Production).build().unwrap();
let report = app.export().await.unwrap();"#,
        ],],
        h2![id("end-to-end"), a![class("anchor"), href("#end-to-end"), "End-to-end"]],
        p![
            "An end-to-end test drives a real browser against the real server. It is slower, and the only way to test what happens in the browser: JavaScript, islands, navigation without reloads.",
        ],
        p![
            "For browser tests (client navigation, islands), start the binary and use Playwright or WebDriver against it. ",
            code!["App::serve_listener"],
            " accepts a ",
            code!["TcpListener"],
            " bound to port 0 if you start the server from Rust.",
        ],
        p!["Every example in ", code!["examples/"], " includes a test suite that uses these APIs."],
        h2![id("running-tests"), a![class("anchor"), href("#running-tests"), "Running the tests"]],
        p!["From the project folder:"],
        pre![code![
            class("language-sh"),
            r"cargo test                     # everything
cargo test --test pages        # only tests/pages.rs
cargo test home_page           # only tests whose name contains home_page
cargo test -- --nocapture      # also show what the tests print",
        ]],
        p![
            "A passing run ends with ",
            code!["test result: ok"],
            " and the number of tests; a failing one names the test, the file and line of the assertion, and both values. The first run compiles the project in test mode into ",
            code!["target/"],
            " and is slow; later runs are quick.",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["No src/lib.rs. "],
                code!["my_app::routes()"],
                " is not found because a binary-only crate cannot be imported. Make the library + binary split above.",
            ],
            li![
                strong!["Your own crate name inside app/. "],
                "Files under ",
                code!["app/"],
                " are modules of the crate: reach your code with ",
                code!["crate::db"],
                ", not ",
                code!["my_app::db"],
                ". Tests, which are outside the crate, use ",
                code!["my_app::"],
                ".",
            ],
            li![strong!["Tokio missing from dev-dependencies. "], code!["#[tokio::test]"], " does not resolve.",],
            li![
                strong!["Asserting development error text. "],
                "Production shows a generic message and a digest. Use ",
                code!["TestClient::production"],
                " to test what visitors will see.",
            ],
            li![
                strong!["Expecting a fresh render every time. "],
                "In the ",
                code!["test"],
                " environment static pages are cached as in production; the second request is a cache hit (",
                code!["x-nr-cache: HIT"],
                "). Revalidate the path or tag when a test changes the data behind a static page.",
            ],
            li![
                strong!["Tests that need .env.local. "],
                "That file is skipped in the ",
                code!["test"],
                " environment so that tests behave the same on every machine. Put test values in ",
                code![".env.test"],
                ".",
            ],
        ],
    ]
}
