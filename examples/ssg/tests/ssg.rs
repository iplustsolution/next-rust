use next_rust::TestClient;

#[tokio::test]
async fn cached_until_revalidated() {
    let client = TestClient::production(example_ssg::routes());
    let first = client.get("/").await;
    assert_eq!(first.header("x-nr-cache"), Some("MISS"));
    assert_eq!(first.header("cache-control"), Some("public, max-age=0, s-maxage=60, stale-while-revalidate"));
    assert!(first.text.contains("Stars: 4200"));
    assert_eq!(client.get("/").await.header("x-nr-cache"), Some("HIT"));

    assert!(client.app().revalidate_tag("stats").await >= 1);
    assert_eq!(client.get("/").await.header("x-nr-cache"), Some("MISS"));
}

#[tokio::test]
async fn static_pages_are_prerendered_in_memory() {
    let client = TestClient::production(example_ssg::routes());
    let report = client.app().export().await.unwrap();
    let paths: Vec<&str> = report.pages.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, vec!["/", "/about"]);
    assert!(report.pages.iter().all(|p| p.bytes > 0));
    assert_eq!(client.get("/about").await.header("x-nr-cache"), Some("MISS"), "export only checks rendering");

    let client = TestClient::production(example_ssg::routes());
    client.app().prerender().await.unwrap();
    let about = client.get("/about").await;
    assert_eq!(about.header("x-nr-cache"), Some("HIT"), "prerender fills the page cache");
    assert!(about.text.contains("About (static, never revalidated)"));
}
