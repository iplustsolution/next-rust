# Testing

Applications are easiest to test with a **library + binary** split: the
library owns the routes, and tests call it directly without a network socket.

```rust
// src/lib.rs
pub mod db;
next_rust::routes!();
```

```rust
// src/main.rs
fn main() {
    next_rust::run(my_app::routes());
}
```

```toml
# Cargo.toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## `TestClient`

```rust
// tests/pages.rs
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
}
```

- `TestClient::new` loads your `next-rust.toml` and uses the `test`
  environment, where static pages are cached as in production.
  `TestClient::production` enables production error messages.
  `TestClient::with(App::new(routes()).middleware(..), env)` tests a custom
  middleware stack.
- `TestResponse` has `status`, `headers`, `text`, `chunks` (streamed chunks
  in order), `header(..)`, `json::<T>()` and `cookies()`.
- Action URLs: `next_rust::action_url("src/actions.rs::create_user")`, or
  `action!(create_user).url()` from the crate.

## Assertions for common framework behaviour

```rust
// streaming: fallback in the first chunk, content later
assert!(res.chunks[0].contains("Loading…"));

// ISR
assert_eq!(res.header("x-nr-cache"), Some("HIT"));
client.app().revalidate_tag("posts").await;

// crawlers get non-streamed HTML
let req = http::Request::get("/").header("user-agent", "Googlebot").body(Default::default()).unwrap();
```

## Static generation in tests

```rust
let mut config = next_rust::Config::discover(env!("CARGO_MANIFEST_DIR")).unwrap();
config.build.output = tempdir.clone();
let app = next_rust::App::new(my_app::routes()).config(config).environment(next_rust::Environment::Production).build().unwrap();
let report = app.export().await.unwrap();
```

## End-to-end

For browser tests (client navigation, islands), start the binary and use
Playwright or WebDriver against it. `App::serve_listener` accepts a
`TcpListener` bound to port 0 if you start the server from Rust.

Every example in `examples/` includes a test suite that uses these APIs.
