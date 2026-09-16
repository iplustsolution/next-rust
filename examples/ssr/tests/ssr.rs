use next_rust::{TestClient, http};

#[tokio::test]
async fn renders_request_data_and_escapes_it() {
    let client = TestClient::new(example_ssr::routes());
    let req =
        http::Request::get("/?name=%3Cscript%3E").header("user-agent", "curl/8").body(Default::default()).unwrap();
    let res = client.send(req).await;
    assert!(res.text.contains("<h1>Hello, &lt;script&gt;</h1>"));
    assert!(res.text.contains("<p class=\"agent\">curl/8</p>"));
    assert_eq!(res.header("cache-control"), Some("private, no-cache, no-store, max-age=0, must-revalidate"));
}

#[tokio::test]
async fn cookies_persist_between_requests() {
    let client = TestClient::new(example_ssr::routes());
    assert!(client.get("/").await.text.contains("Visit #1"));
    let second = client.get("/").await;
    assert!(second.text.contains("Visit #2"));
    assert_eq!(second.header("x-visits"), Some("2"));
}
