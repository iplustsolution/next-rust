use std::sync::atomic::Ordering;

use example_partial_prerendering::routes;
use next_rust::{TestClient, http};

fn get(path: &str, user: Option<&str>) -> http::Request<next_rust::bytes::Bytes> {
    let mut req = http::Request::get(path);
    if let Some(user) = user {
        req = req.header("cookie", format!("user={user}"));
    }
    req.body(Default::default()).unwrap()
}

#[tokio::test]
async fn one_shell_for_everyone_and_holes_for_each_visitor() {
    let client = TestClient::production(routes());
    let ada = client.send(get("/", Some("Ada"))).await;
    let guest = client.send(get("/", None)).await;

    // The shell (layout, product, cached reviews, fallbacks) was rendered once…
    assert_eq!(ada.header("x-nr-cache"), Some("MISS"));
    assert_eq!(guest.header("x-nr-cache"), Some("HIT"));
    assert!(guest.text.contains("Shell render #1"), "{}", guest.text);
    // …and is the first thing on the wire, before the slow hole.
    let shell = &guest.chunks[0];
    assert!(shell.contains("<h1>Oak chair</h1>") && shell.contains("Finding things you may like…"), "{shell}");
    assert!(shell.contains("“Sturdy.”") && !shell.contains("Picked for"), "{shell}");

    // The holes are each visitor's own, and the response is never stored.
    assert!(ada.text.contains("Signed in as Ada") && ada.text.contains("Picked for Ada: walnut desk"));
    assert!(guest.text.contains("Not signed in") && guest.text.contains("Picked for new visitors"));
    assert!(!guest.text.contains("Ada"));
    assert!(guest.header("cache-control").unwrap().contains("no-store"));

    // Revalidating the tag renders the shell again; the holes never were cached.
    client.app().revalidate_tag("products").await;
    let res = client.send(get("/", Some("Bob"))).await;
    assert!(res.text.contains("Shell render #2") && res.text.contains("Signed in as Bob"), "{}", res.text);
    assert_eq!(example_partial_prerendering::SHELL_RENDERS.load(Ordering::SeqCst), 2);
}
