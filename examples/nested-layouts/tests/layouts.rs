use next_rust::TestClient;

#[tokio::test]
async fn layouts_nest_in_order() {
    let client = TestClient::new(example_nested_layouts::routes());
    let res = client.get("/products/7").await;
    assert!(res.text.contains(
        "<div id=\"root-layout\"><div id=\"shop-group-layout\"><div id=\"products-layout\"><div id=\"product-layout\" data-product=\"7\"><div class=\"product-template\"><h1>Product 7</h1></div></div></div></div></div>"
    ), "{}", res.text);
    assert!(res.text.contains("<title>Product 7 · Products · Shop</title>"));
    let list = client.get("/products").await;
    assert!(list.text.contains("<title>Products · Shop</title>"));
}
