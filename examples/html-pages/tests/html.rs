use next_rust::TestClient;

#[tokio::test]
async fn html_fragments_and_documents() {
    let client = TestClient::new(example_html_pages::routes());
    let legacy = client.get("/legacy").await;
    assert!(legacy.text.contains("<main><h1>Legacy page</h1>"), "{}", legacy.text);
    let landing = client.get("/landing").await;
    assert!(landing.text.starts_with("<!DOCTYPE html>"));
    assert!(landing.text.contains("<h1>Standalone landing page</h1>"));
    assert!(!landing.text.contains("<strong>HTML pages</strong>"), "full documents bypass layouts");
}
