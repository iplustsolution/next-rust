//! What the pages of this example promise the browser drive: the markup the
//! WebAssembly runtime reads has to be there before any of it can be checked
//! in a browser.

use example_wasm_runtime_check::routes;
use next_rust::TestClient;

#[tokio::test]
async fn the_home_page_carries_the_islands_and_the_runtime() {
    let client = TestClient::new(routes());
    let res = client.get("/").await;
    assert_eq!(res.status, 200);
    // The declarative island and its bindings.
    assert!(res.text.contains("nr-island"), "no island on the page");
    assert!(res.text.contains("data-nr-text"));
    assert!(res.text.contains("data-nr-on-click"));
    // An internal link, which is what makes the page load a runtime at all.
    assert!(res.text.contains("/_next-rust/runtime.js"));
}

#[tokio::test]
async fn a_client_navigation_reuses_the_layout() {
    let client = TestClient::new(routes());
    let full = client.get("/section/a").await;
    assert!(full.text.contains("section-render"));
    // `navigate` sends the partial-rendering headers; the answer is only what
    // goes inside the deepest shared layout, so the layout is not in it.
    let partial = client.navigate("/section/b", &full.text).await;
    assert_eq!(partial.status, 200);
    assert!(partial.text.contains("section-b"), "the page's own content is missing");
}

#[tokio::test]
async fn the_action_form_validates_and_the_module_island_declares_its_module() {
    let client = TestClient::new(routes());
    let forms = client.get("/forms").await;
    assert!(forms.text.contains("data-nr-error=\"name\""));
    assert!(forms.text.contains("data-nr-action"));
    assert!(forms.text.contains("name=\"_redirect\""));
    let about = client.get("/about").await;
    assert!(about.text.contains("data-module=\"/_next-rust/client/badge.js\""), "{}", about.text);
}

#[tokio::test]
async fn the_live_page_marks_its_form_and_its_target() {
    let client = TestClient::new(routes());
    let live = client.get("/live?q=ber").await;
    assert!(live.text.contains("data-nr-live"));
    assert!(live.text.contains("data-nr-target"));
    assert!(live.text.contains("blueberry") && !live.text.contains("apple"));
}

#[tokio::test]
async fn the_api_routes_answer() {
    let client = TestClient::new(routes());
    assert_eq!(client.get("/api/echo").await.status, 200);
    let posted = client.post_form("/api/echo", "text=hello").await;
    assert_eq!(posted.status, 200);
    assert!(posted.text.contains("hello"));
    assert_eq!(client.post_form("/api/echo", "text=").await.status, 422);
}
