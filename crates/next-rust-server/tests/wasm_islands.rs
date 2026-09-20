//! `[build] wasm_islands`: a page with a WebAssembly island serves the shared
//! shim, the island's glue module and the module itself, and relaxes the CSP
//! by exactly the one keyword the browser needs to compile it.
//!
//! Hermetic on purpose: the files the build would generate are written by
//! hand, so nothing here needs the `wasm32-unknown-unknown` target, a nested
//! cargo build or the network. What the build puts *into* those files is
//! covered by the unit tests in `next-rust-build`.

use bytes::Bytes;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// The shortest valid WebAssembly module: the magic and version, no sections.
const EMPTY_MODULE: &[u8] = b"\0asm\x01\0\0\0";

const LOADER: &str = "export async function h(n,e,p){await WebAssembly.instantiateStreaming(fetch(u),i)}";
const GLUE: &str = "import{h}from\"../__nrw.js\";export const hydrate=(e,p)=>h(\"AuthSwitch\",e,p);";

fn page(_ctx: Ctx) -> R {
    Box::pin(async {
        let island = Element::new("nr-island")
            .with(attr("data-component", "AuthSwitch"))
            .with(attr("data-props", "{}"))
            .with(attr("data-module", "/_next-rust/client/__nrw/AuthSwitch.js"))
            .with("tabs");
        Ok(div![island].into_node())
    })
}

/// A project whose `client/` already holds what `wasm_islands` generates.
fn build(wasm_islands: bool) -> App {
    let root = std::env::temp_dir().join(format!("nr-wasm-islands-{}-{wasm_islands}", std::process::id()));
    let dir = root.join("client").join("__nrw");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(root.join("client").join("__nrw.js"), LOADER).unwrap();
    std::fs::write(dir.join("AuthSwitch.js"), GLUE).unwrap();
    std::fs::write(dir.join("islands.wasm"), EMPTY_MODULE).unwrap();

    let routes = Routes {
        pages: vec![PageDef {
            pattern: "/signin",
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
    let mut config = Config { root, ..Config::default() };
    config.build.wasm_islands = wasm_islands;
    config.security.headers = true;
    config.security.csp = Some("strict".into());
    App::new(routes).config(config).environment(Environment::Production).build().unwrap()
}

async fn get(app: &App, uri: &str) -> Response {
    app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await
}

#[tokio::test]
async fn a_page_with_a_wasm_island_serves_the_module_and_the_shim() {
    let app = build(true);

    // The island points the runtime at its glue module, which is what the
    // existing `data-module` hydration path already knows how to import.
    let html = get(&app, "/signin").await.into_text().await;
    assert!(html.contains("data-module=\"/_next-rust/client/__nrw/AuthSwitch.js\""), "{html}");
    assert!(html.contains("src=\"/_next-rust/runtime.js?v="), "the island pulls the runtime in: {html}");

    // The glue module: one line that binds this island's name to the shim.
    let res = get(&app, "/_next-rust/client/__nrw/AuthSwitch.js").await;
    assert_eq!(res.status, 200);
    assert!(res.header("content-type").is_some_and(|c| c.contains("javascript")), "{:?}", res.header("content-type"));
    let glue = res.into_text().await;
    assert!(glue.contains("hydrate") && glue.contains("AuthSwitch") && glue.contains("__nrw.js"), "{glue}");

    // The shim every island shares.
    let res = get(&app, "/_next-rust/client/__nrw.js").await;
    assert_eq!(res.status, 200);
    assert!(res.into_text().await.contains("WebAssembly"));

    // The module itself, as `application/wasm` — without that content type a
    // browser refuses to stream-compile it.
    let res = get(&app, "/_next-rust/client/__nrw/islands.wasm").await;
    assert_eq!(res.status, 200);
    assert_eq!(res.header("content-type").unwrap(), "application/wasm");
    assert_eq!(&res.into_bytes().await.unwrap()[..], EMPTY_MODULE);
}

#[tokio::test]
async fn the_csp_allows_compiling_a_module_only_when_islands_are_on() {
    let on = get(&build(true), "/signin").await;
    let csp = on.header("content-security-policy").unwrap().to_owned();
    assert!(csp.contains("'wasm-unsafe-eval'"), "{csp}");
    assert!(csp.contains("script-src ") && csp.contains("object-src 'none'"), "still the strict policy: {csp}");

    // A project without wasm islands keeps the tighter policy.
    let off = get(&build(false), "/signin").await;
    let csp = off.header("content-security-policy").unwrap();
    assert!(!csp.contains("wasm-unsafe-eval"), "{csp}");
}
