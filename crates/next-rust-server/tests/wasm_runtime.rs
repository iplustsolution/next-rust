//! `[client] runtime = "wasm"`: a page serves the shim instead of the
//! JavaScript runtime, the module behind it is served as `application/wasm`,
//! and `"js"` (the default) changes nothing.
//!
//! The point of these is the *switch*. What the two runtimes do is the same
//! behaviour by construction — `crates/next-rust-runtime` is the port of
//! `RUNTIME_JS` — and the port is covered by that crate's own tests and by
//! the browser suite. What is easy to get wrong, and invisible until someone
//! measures a page, is whether the choice reaches the markup at all.

use bytes::Bytes;
use next_rust_core::config::ClientRuntime;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// A page with an internal link, which is what makes a page load the runtime.
fn page(_ctx: Ctx) -> R {
    Box::pin(async { Ok(div![a![href("/other"), "Other"]].into_node()) })
}

fn build(runtime: ClientRuntime) -> App {
    let routes = Routes {
        pages: vec![PageDef {
            pattern: "/",
            source: "test",
            body: PageBody::Rust(page),
            segments: vec![SegmentDef::default()],
            metadata: None,
            rendering: Rendering::Dynamic,
            revalidate: None,
            generate_params: None,
            dynamic_params: true,
            tags: &[],
            intercept_from: None,
        }],
        ..Default::default()
    };
    let root = std::env::temp_dir().join(format!("nr-wasm-runtime-{}-{runtime:?}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut config = Config { root, ..Config::default() };
    config.client.runtime = runtime;
    App::new(routes).config(config).environment(Environment::Production).build().unwrap()
}

async fn get(app: &App, uri: &str) -> Response {
    app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await
}

/// The switch has to reach the page. Without `data-nr-shim` on the tag a page
/// looks identical whichever runtime it serves, which is exactly how the
/// wiring stayed unfinished once already.
#[tokio::test]
async fn a_wasm_page_serves_the_shim_and_marks_it() {
    let app = build(ClientRuntime::Wasm);
    let html = get(&app, "/").await.into_text().await;
    assert!(html.contains("data-nr-shim"), "the runtime script is not marked as the shim: {html}");
    assert!(html.contains("src=\"/_next-rust/runtime.js?v="), "{html}");

    // Same URL as always: what changes is what comes back.
    let res = get(&app, "/_next-rust/runtime.js").await;
    assert_eq!(res.status, 200);
    let js = res.into_text().await;
    assert!(js.contains("WebAssembly.instantiate"), "the shim was not served: {js}");
    assert!(!js.contains("x-nr-nav"), "the JavaScript runtime was served instead");

    // The module, which a browser refuses to stream-compile without this type.
    let res = get(&app, "/_next-rust/runtime.wasm").await;
    assert_eq!(res.status, 200);
    assert_eq!(res.header("content-type").unwrap(), "application/wasm");
    let bytes = res.into_bytes().await.unwrap();
    assert!(bytes.starts_with(b"\0asm\x01\0\0\0"), "not a WebAssembly module");
    assert!(bytes.len() > 1024, "the module is a stub: {} bytes", bytes.len());
}

/// The default. An application that has not asked for any of this must not be
/// able to tell that it exists.
#[tokio::test]
async fn the_javascript_runtime_is_still_the_default() {
    assert_eq!(Config::default().client.runtime, ClientRuntime::Js);
    let app = build(ClientRuntime::Js);
    let html = get(&app, "/").await.into_text().await;
    assert!(html.contains("src=\"/_next-rust/runtime.js?v="), "{html}");
    assert!(!html.contains("data-nr-shim"), "a JavaScript page must not claim to carry a shim: {html}");

    let js = get(&app, "/_next-rust/runtime.js").await.into_text().await;
    assert!(js.contains("x-nr-nav"), "the JavaScript runtime was not served: {js}");
    assert!(!js.contains("WebAssembly.instantiate"), "the shim was served instead");

    // And the module is not reachable at all, so a stale page cannot fetch it.
    assert_eq!(get(&app, "/_next-rust/runtime.wasm").await.status, 404);
}

/// The two runtimes are versioned separately: a shim that did not change still
/// has to be re-fetched when the module behind it did, or a browser keeps a
/// shim that indexes an opcode table the new module has moved on from.
#[tokio::test]
async fn the_version_covers_the_module_as_well_as_the_shim() {
    let wasm = get(&build(ClientRuntime::Wasm), "/").await.into_text().await;
    let js = get(&build(ClientRuntime::Js), "/").await.into_text().await;
    let version = |html: &str| {
        let at = html.find("runtime.js?v=").expect("no runtime script") + "runtime.js?v=".len();
        html[at..].split('"').next().unwrap().to_owned()
    };
    assert_ne!(version(&wasm), version(&js), "both runtimes are served under the same version");
    assert!(!version(&wasm).is_empty() && !version(&js).is_empty());
}
