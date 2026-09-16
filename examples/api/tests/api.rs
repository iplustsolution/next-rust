use next_rust::{TestClient, http};

#[tokio::test]
async fn crud() {
    let client = TestClient::new(example_api::routes());
    let created = client.post_json("/api/todos", &serde_json::json!({ "title": "Write docs" })).await;
    assert_eq!(created.status, 201);
    let id = created.json::<serde_json::Value>()["id"].as_u64().unwrap();

    let res = client.get(&format!("/api/todos/{id}")).await;
    assert_eq!(res.json::<serde_json::Value>()["title"], "Write docs");

    let req = http::Request::patch(format!("/api/todos/{id}"))
        .header("content-type", "application/json")
        .body(r#"{"done":true}"#.into())
        .unwrap();
    assert_eq!(client.send(req).await.json::<serde_json::Value>()["done"], true);

    let req = http::Request::delete(format!("/api/todos/{id}")).body(Default::default()).unwrap();
    assert_eq!(client.send(req).await.status, 204);
    assert_eq!(client.get(&format!("/api/todos/{id}")).await.status, 404);
    assert_eq!(client.get("/api/todos/abc").await.status, 400);
}

#[tokio::test]
async fn validation_and_methods() {
    let client = TestClient::new(example_api::routes());
    let res = client.post_json("/api/todos", &serde_json::json!({ "title": " " })).await;
    assert_eq!(res.status, 422);
    let req = http::Request::put("/api/todos").body(Default::default()).unwrap();
    let res = client.send(req).await;
    assert_eq!(res.status, 405);
    assert_eq!(res.header("allow"), Some("GET, POST, HEAD"));
    assert_eq!(client.get("/api/health").await.text, r#"{"status":"ok"}"#);
}

#[tokio::test]
async fn webhooks_use_the_raw_body() {
    let client = TestClient::new(example_api::routes());
    let req = http::Request::post("/api/webhooks")
        .header("x-webhook-token", "dev-token")
        .body("{\"event\":\"push\"}".into())
        .unwrap();
    assert_eq!(client.send(req).await.text, r#"{"received_bytes":16}"#);
    let req = http::Request::post("/api/webhooks").header("x-webhook-token", "wrong").body("x".into()).unwrap();
    assert_eq!(client.send(req).await.status, 401);
}

#[tokio::test]
async fn server_sent_events() {
    let client = TestClient::new(example_api::routes());
    let res = client.get("/api/events").await;
    assert_eq!(res.header("content-type"), Some("text/event-stream"));
    assert_eq!(res.text, "event: tick\ndata: 1\n\nevent: tick\ndata: 2\n\nevent: tick\ndata: 3\n\n");
}
