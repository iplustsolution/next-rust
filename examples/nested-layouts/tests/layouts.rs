use next_rust::TestClient;
use next_rust::testing::{layout_keys, strip_layout_markers};

#[tokio::test]
async fn layouts_nest_in_order() {
    let client = TestClient::new(example_nested_layouts::routes());
    let res = client.get("/products/7").await;
    assert!(strip_layout_markers(&res.text).contains(
        "<div id=\"root-layout\"><div id=\"shop-group-layout\"><div id=\"products-layout\"><div id=\"product-layout\" data-product=\"7\"><div class=\"product-template\"><h1>Product 7</h1></div></div></div></div></div>"
    ), "{}", res.text);
    assert!(res.text.contains("<title>Product 7 · Products · Shop</title>"));
    let list = client.get("/products").await;
    assert!(list.text.contains("<title>Products · Shop</title>"));
}

#[tokio::test]
async fn navigation_keeps_shared_layouts() {
    let client = TestClient::new(example_nested_layouts::routes());
    let page = client.get("/products/7").await;
    let keys = layout_keys(&page.text);
    assert_eq!(keys.len(), 4, "root, (shop), products and product layouts");

    // Another product: the product layout reads the id, so it renders again;
    // root, shop and products layouts are neither rendered nor sent.
    let next = client.navigate("/products/8", &page.text).await;
    assert_eq!(next.header("x-nr-partial"), Some(keys[2].as_str()));
    let html = strip_layout_markers(&next.text);
    assert!(html.contains("<div id=\"product-layout\" data-product=\"8\">"), "{html}");
    assert!(!html.contains("products-layout") && !html.contains("root-layout"), "{html}");
    assert!(next.text.contains("<title>Product 8 · Products · Shop</title>"));

    // Up to the list: everything inside the products layout.
    let list = client.navigate("/products", &page.text).await;
    assert_eq!(list.header("x-nr-partial"), Some(keys[2].as_str()));
    assert!(!list.text.contains("shop-group-layout"));
}
