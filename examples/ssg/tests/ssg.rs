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
async fn export_writes_static_files() {
    let dir = std::env::temp_dir().join(format!("nr-ssg-export-{}", std::process::id()));
    let mut config = next_rust::Config::discover(env!("CARGO_MANIFEST_DIR")).unwrap();
    config.build.output = dir.clone();
    let app = next_rust::App::new(example_ssg::routes())
        .config(config)
        .environment(next_rust::Environment::Production)
        .build()
        .unwrap();
    let report = app.export().await.unwrap();
    let paths: Vec<&str> = report.pages.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, vec!["/", "/about"]);
    let html = std::fs::read_to_string(dir.join("static/about/index.html")).unwrap();
    assert!(html.contains("About (static, never revalidated)"));
    assert!(!html.contains("nonce="), "static HTML files carry no nonce placeholders");
    std::fs::remove_dir_all(dir).unwrap();
}
