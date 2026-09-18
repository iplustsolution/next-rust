use next_rust::TestClient;

#[tokio::test]
async fn showcase_renders_with_the_component_script() {
    let client = TestClient::new(example_ui::routes());
    let page = client.get("/").await;
    assert_eq!(page.status, 200);
    for needle in ["nr-btn", "nr-select-list", "nr-calendar", "nr-switch", "nr-avatar-group", "nr-shell"] {
        assert!(page.text.contains(needle), "{needle} missing");
    }
    assert!(page.text.contains("/_nr/ui.js?v="), "interactive components load the script");
    assert!(page.text.contains("/_nr/runtime.js?v="), "and the runtime they call actions through");

    let script = client.get("/_nr/ui.js").await;
    assert_eq!(script.status, 200);
    assert_eq!(script.header("content-type"), Some("text/javascript; charset=utf-8"));
    assert!(script.text.contains("nr-ui-js") && script.text.contains("data-nr-press"));
}

#[tokio::test]
async fn pages_get_only_the_css_of_their_components() {
    let client = TestClient::new(example_ui::routes());
    let css_of = |html: &str| {
        html.split("<style data-nr-css=\"nr-ui~")
            .skip(1)
            .map(|s| s.split("</style>").next().unwrap().to_owned())
            .collect::<String>()
    };
    let full = css_of(&client.get("/").await.text);
    let plain = css_of(&TestClient::new(example_ui::routes()).get("/plain").await.text);
    assert!(full.contains(".nr-calendar") && full.contains(".nr-select-list"));
    assert!(plain.contains(".nr-chip-flat"), "the chip on the page");
    assert!(!plain.contains(".nr-calendar") && !plain.contains(".nr-select-list") && !plain.contains(".nr-switch"));
    assert!(plain.len() * 2 < full.len(), "plain {} vs full {}", plain.len(), full.len());
}

#[tokio::test]
async fn the_form_reports_validation_errors_per_field() {
    let client = TestClient::new(example_ui::routes());
    let url = client.action_url("src/actions.rs::sign_up");
    let res =
        client.post_json(&url, &serde_json::json!({ "name": "", "email": "x", "password": "short", "plan": "" })).await;
    assert_eq!(res.status, 422);
    let errors = &res.json::<serde_json::Value>()["errors"];
    for field in ["name", "email", "password", "plan"] {
        assert!(errors[field].is_string(), "{field}");
    }
    // Every field of the form renders the element those errors go into.
    let page = client.get("/").await;
    for field in ["name", "email", "password", "plan"] {
        assert!(page.text.contains(&format!(r#"data-nr-error="{field}""#)), "{field}");
    }
    let ok = client
        .post_json(
            &url,
            &serde_json::json!({ "name": "Ada", "email": "a@b.c", "password": "long enough", "plan": "pro" }),
        )
        .await;
    assert_eq!(ok.json::<serde_json::Value>()["data"], "Welcome, Ada!");
}

#[test]
fn press_from_an_action_reference() {
    use next_rust::ui::*;
    let html = next_rust::render_static(Button![
        on_press = Press::from(next_rust::action!(example_ui::actions::like)).input(&7).no_refresh(),
        "Like"
    ]);
    assert!(html.contains(r#"data-nr-press="action""#) && html.contains(r#"data-nr-press-input="7""#));
}
