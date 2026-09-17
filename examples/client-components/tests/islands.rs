use next_rust::TestClient;

#[tokio::test]
async fn islands_render_on_the_server() {
    let client = TestClient::new(example_client_components::routes());
    let res = client.get("/").await;
    assert!(
        res.text.contains(
            r#"<nr-island data-component="Counter" data-props="{&quot;count&quot;:3}"><div class="counter">"#
        ),
        "{}",
        res.text
    );
    assert!(res.text.contains(r#"<output data-nr-text="count">3</output>"#));
    assert!(res.text.contains(r#"<script type="module" src="/_nr/runtime.js?v="#));
}
