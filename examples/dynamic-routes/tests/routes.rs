use next_rust::TestClient;

async fn h1(client: &TestClient, path: &str) -> (u16, String) {
    let res = client.get(path).await;
    let start = res.text.find("<h1>").map(|i| i + 4).unwrap_or(0);
    let end = res.text[start..].find("</h1>").map(|i| i + start).unwrap_or(start);
    (res.status, res.text[start..end].to_owned())
}

#[tokio::test]
async fn precedence_and_params() {
    let client = TestClient::new(example_dynamic_routes::routes());
    assert_eq!(h1(&client, "/users/settings").await.1, "User settings (static route wins over [id])");
    assert_eq!(h1(&client, "/users/42").await.1, "User #42");
    assert_eq!(client.get("/users/abc").await.status, 404);
    let docs = client.get("/docs/guide/routing/dynamic").await;
    assert!(docs.text.contains("<ol><li>guide</li><li>routing</li><li>dynamic</li></ol>"));
    assert_eq!(client.get("/docs").await.status, 404, "catch-all needs at least one segment");
    assert_eq!(h1(&client, "/shop").await.1, "All products");
    assert_eq!(h1(&client, "/shop/shoes/red").await.1, "Filtered by shoes &gt; red");
}
