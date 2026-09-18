use next_rust::{TestClient, http};

/// The action URL of the first form in `html`.
fn form_action(html: &str) -> String {
    html.split("<form")
        .nth(1)
        .and_then(|f| f.split("action=\"").nth(1))
        .and_then(|a| a.split('"').next())
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn form_and_json_submissions() {
    let client = TestClient::new(example_server_actions::routes());
    let page = client.get("/").await;
    let url = form_action(&page.text);
    assert!(url.starts_with("/_nr/action/"), "{}", page.text);
    assert!(page.text.contains(&format!("action=\"{url}\" method=\"post\"")), "{}", page.text);
    assert!(page.header("cache-control").unwrap().contains("no-store"), "personal pages are never cached");
    assert!(!page.text.contains("/_nr/action/nr~"), "every marker is replaced, island props included");
    assert!(page.cookies().iter().any(|c| c.starts_with("nr_bind=") && c.contains("HttpOnly")));
    assert_ne!(url, form_action(&client.get("/").await.text), "each page view gets its own URL");

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
    let count = client.post_json(&client.action_url("src/actions.rs::count_entries"), &()).await;
    assert!(count.json::<serde_json::Value>()["data"].as_u64().unwrap() >= 1);
    assert!(client.get("/").await.text.contains("<li><strong>Ada</strong>: Hello!</li>"));
}

#[tokio::test]
async fn cross_site_posts_are_rejected() {
    let client = TestClient::new(example_server_actions::routes());
    let url = client.action_url("src/actions.rs::sign_guestbook");
    let req = http::Request::post(url)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("origin", "https://attacker.example")
        .body("name=x&message=spam".into())
        .unwrap();
    assert_eq!(client.send(req).await.status, 403);
}

#[tokio::test]
async fn copied_action_urls_do_not_work_elsewhere() {
    let victim = TestClient::new(example_server_actions::routes());
    let url = form_action(&victim.get("/").await.text);
    let attacker = TestClient::new(example_server_actions::routes());
    attacker.get("/").await;
    let res = attacker.post_form(&url, "name=Mallory&message=spam").await;
    assert_eq!(res.status, 303, "sent back to the page");
    assert!(attacker.get("/").await.text.contains("This page is out of date"));
    assert!(!victim.get("/").await.text.contains("Mallory"), "the action did not run");
}
