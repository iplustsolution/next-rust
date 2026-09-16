use next_rust::{Environment, TestClient, http};

fn client() -> TestClient {
    TestClient::with(example_middleware::app(), Environment::Test)
}

#[tokio::test]
async fn global_and_file_middleware() {
    let client = client();
    let res = client.get("/home").await;
    assert_eq!(res.status, 200, "rewritten by app/middleware.rs");
    assert_eq!(res.header("x-powered-by"), Some("next-rust"));
    assert_eq!(res.header("x-request-id").map(str::len), Some(32));
    assert!(res.text.contains("Session views: 1"));
    assert!(client.get("/").await.text.contains("Session views: 2"), "sessions persist via cookie");
}

#[tokio::test]
async fn nested_auth_middleware() {
    let client = client();
    assert_eq!(client.get("/admin").await.status, 401);
    let req =
        http::Request::get("/admin").header("authorization", "Bearer admin-token").body(Default::default()).unwrap();
    assert!(client.send(req).await.text.contains("<h1>Admin: admin</h1>"));
}

#[tokio::test]
async fn cors_preflight() {
    let client = client();
    let req = http::Request::options("/")
        .header("origin", "https://app.example.com")
        .header("access-control-request-method", "POST")
        .body(Default::default())
        .unwrap();
    let res = client.send(req).await;
    assert_eq!(res.status, 204);
    assert_eq!(res.header("access-control-allow-origin"), Some("https://app.example.com"));
    assert_eq!(res.header("access-control-allow-credentials"), Some("true"));
}
