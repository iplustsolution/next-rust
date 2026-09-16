use next_rust::{TestClient, action_url, http};

#[tokio::test]
async fn form_and_json_submissions() {
    let client = TestClient::new(example_server_actions::routes());
    let url = action_url("src/actions.rs::sign_guestbook");

    let page = client.get("/").await;
    assert!(page.text.contains(&format!("action=\"{url}\" method=\"post\"")), "{}", page.text);

    // Progressive enhancement: invalid form → redirect back with errors.
    let res = client.post_form(&url, "name=&message=hi").await;
    assert_eq!(res.status, 303);
    let page = client.get("/").await;
    assert!(page.text.contains("Please tell us your name"));
    assert!(page.text.contains("Messages need at least 3 characters"));
    assert!(page.text.contains("<textarea name=\"message\">hi</textarea>"), "values are preserved");

    // JSON API used by the client runtime.
    let res = client.post_json(&url, &serde_json::json!({ "name": "Ada", "message": "Hello!" })).await;
    assert_eq!(
        res.json::<serde_json::Value>(),
        serde_json::json!({ "ok": true, "data": { "name": "Ada", "message": "Hello!" } })
    );
    let count = client.post_json(&action_url("src/actions.rs::count_entries"), &()).await;
    assert!(count.json::<serde_json::Value>()["data"].as_u64().unwrap() >= 1);
    assert!(client.get("/").await.text.contains("<li><strong>Ada</strong>: Hello!</li>"));
}

#[tokio::test]
async fn cross_site_posts_are_rejected() {
    let client = TestClient::new(example_server_actions::routes());
    let url = action_url("src/actions.rs::sign_guestbook");
    let req = http::Request::post(url)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("origin", "https://attacker.example")
        .body("name=x&message=spam".into())
        .unwrap();
    assert_eq!(client.send(req).await.status, 403);
}
