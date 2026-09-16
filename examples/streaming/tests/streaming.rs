use next_rust::TestClient;

#[tokio::test]
async fn shell_first_then_boundaries_in_completion_order() {
    let client = TestClient::new(example_streaming::routes());
    let res = client.get("/").await;
    assert!(res.chunks.len() >= 3, "{:?}", res.chunks);
    assert!(res.chunks[0].contains("Loading weather…") && res.chunks[0].contains("Loading headlines…"));
    let news = res.text.find("Rust 2.0 announced").unwrap();
    let weather = res.text.find("Sunny, 24°C").unwrap();
    assert!(news < weather, "faster boundaries are flushed first");
}

#[tokio::test]
async fn loading_rs_becomes_a_boundary() {
    let client = TestClient::new(example_streaming::routes());
    let res = client.get("/slow").await;
    assert!(res.chunks[0].contains("Preparing report…"));
    assert!(res.text.contains("<h1>Report ready</h1>"));
}
