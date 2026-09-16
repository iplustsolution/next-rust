use next_rust::TestClient;

#[tokio::test]
async fn pages_are_static_and_cached() {
    let client = TestClient::production(example_blog::routes());
    let res = client.get("/blog/hello-next-rust").await;
    assert_eq!(res.status, 200);
    assert_eq!(res.header("x-nr-cache"), Some("MISS"));
    assert!(res.text.contains("<title>Hello, Next Rust — The Rust Blog</title>"), "{}", res.text);
    assert!(res.text.contains("<meta property=\"og:type\" content=\"article\">"));
    let again = client.get("/blog/hello-next-rust").await;
    assert_eq!(again.header("x-nr-cache"), Some("HIT"));
}

#[tokio::test]
async fn unknown_posts_are_not_found() {
    let client = TestClient::production(example_blog::routes());
    let res = client.get("/blog/nope").await;
    assert_eq!(res.status, 404);
}

#[tokio::test]
async fn seo_files() {
    let client = TestClient::new(example_blog::routes());
    let sitemap = client.get("/sitemap.xml").await;
    assert_eq!(sitemap.header("content-type"), Some("application/xml"));
    assert!(sitemap.text.contains("<loc>https://blog.example.com/blog/static-generation</loc>"));
    let robots = client.get("/robots.txt").await;
    assert!(robots.text.contains("Sitemap: https://blog.example.com/sitemap.xml"));
}

#[tokio::test]
async fn index_lists_posts() {
    let client = TestClient::new(example_blog::routes());
    let res = client.get("/").await;
    assert!(res.text.contains("<a href=\"/blog/static-generation\" data-nr-link=\"\">Static generation</a>"));
}
