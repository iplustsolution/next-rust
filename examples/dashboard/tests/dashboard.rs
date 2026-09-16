use next_rust::{TestClient, action_url, http};

fn signed_in() -> http::Request<next_rust::bytes::Bytes> {
    http::Request::get("/dashboard").header("cookie", "user=ada").body(Default::default()).unwrap()
}

#[tokio::test]
async fn redirects_anonymous_users() {
    let client = TestClient::new(example_dashboard::routes());
    let res = client.get("/dashboard/settings").await;
    assert_eq!(res.status, 307);
    assert_eq!(res.header("location"), Some("/login"));
}

#[tokio::test]
async fn streams_loading_state_and_renders_slots() {
    let client = TestClient::new(example_dashboard::routes());
    let res = client.send(signed_in()).await;
    assert_eq!(res.status, 200);
    let shell = &res.chunks[0];
    assert!(shell.contains("Loading dashboard…"), "{shell}");
    assert!(shell.contains("Signed in as ada"));
    assert!(shell.contains("<section class=\"analytics\"><p>1,024 visitors today</p></section>"));
    assert!(shell.contains("Ada deployed"));
    assert!(res.text.contains("Revenue: $12,345"));
    assert!(res.text.contains("<title>Dashboard | Acme Dashboard</title>"));
}

#[tokio::test]
async fn slot_defaults_and_templates() {
    let client = TestClient::new(example_dashboard::routes());
    let req = http::Request::get("/dashboard/settings")
        .header("cookie", "user=ada")
        .header("user-agent", "Googlebot")
        .body(Default::default())
        .unwrap();
    let res = client.send(req).await;
    assert!(res.text.contains("Analytics are shown on the overview page"));
    assert!(res.text.contains("<div class=\"settings-template\"><h1>Settings</h1></div>"));
}

#[tokio::test]
async fn error_boundary_contains_failures() {
    let client = TestClient::production(example_dashboard::routes());
    let req = http::Request::get("/dashboard/reports")
        .header("cookie", "user=ada")
        .header("user-agent", "Googlebot")
        .body(Default::default())
        .unwrap();
    let res = client.send(req).await;
    assert_eq!(res.status, 500);
    assert!(res.text.contains("This section failed to load"));
    assert!(res.text.contains("Signed in as ada"), "the dashboard layout still renders");
    assert!(!res.text.contains("report service unavailable"), "details stay private in production");
}

#[tokio::test]
async fn login_action_with_validation() {
    let client = TestClient::new(example_dashboard::routes());
    let url = action_url("app/(auth)/login/page.rs::login");
    let res = client.post_form(&url, "username=").await;
    assert_eq!(res.status, 303);
    let page = client.get("/login").await;
    assert!(page.text.contains("Please enter a username"), "flash errors are shown once");
    let again = client.get("/login").await;
    assert!(!again.text.contains("Please enter a username"));

    let res = client.post_form(&url, "username=Ada+Lovelace").await;
    assert_eq!((res.status, res.header("location")), (303, Some("/dashboard")));
    let dash = client.get("/dashboard").await;
    assert!(dash.text.contains("Signed in as Ada Lovelace"));
}
