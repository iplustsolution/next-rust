//! The "testing" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Applications are easiest to test with a ",
            strong!["library + binary"],
            " split: the library owns the routes, and tests call it directly without a network socket.",
        ],
        pre![code![
            class("language-rust"),
            r"// src/lib.rs
pub mod db;
next_rust::routes!();",
        ],],
        pre![code![
            class("language-rust"),
            r"// src/main.rs
fn main() {
    next_rust::run(my_app::routes());
}",
        ],],
        pre![code![
            class("language-toml"),
            r#"# Cargo.toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }"#,
        ],],
        h2![id("testclient"), a![class("anchor"), href("#testclient"), code!["TestClient"]]],
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
    client.post_form("/_nr/action/…", "username=ada").await;

    let req = http::Request::delete(format!("/api/todos/{}", todo["id"])).body(Default::default()).unwrap();
    assert_eq!(client.send(req).await.status, 204);
}"#,
        ],],
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
                code!["next_rust::action_url(\"src/actions.rs::create_user\")"],
                ", or ",
                code!["action!(create_user).url()"],
                " from the crate.",
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
        pre![code![
            class("language-rust"),
            r#"let mut config = next_rust::Config::discover(env!("CARGO_MANIFEST_DIR")).unwrap();
config.build.output = tempdir.clone();
let app = next_rust::App::new(my_app::routes()).config(config).environment(next_rust::Environment::Production).build().unwrap();
let report = app.export().await.unwrap();"#,
        ],],
        h2![id("end-to-end"), a![class("anchor"), href("#end-to-end"), "End-to-end"]],
        p![
            "For browser tests (client navigation, islands), start the binary and use Playwright or WebDriver against it. ",
            code!["App::serve_listener"],
            " accepts a ",
            code!["TcpListener"],
            " bound to port 0 if you start the server from Rust.",
        ],
        p!["Every example in ", code!["examples/"], " includes a test suite that uses these APIs."],
    ]
}
