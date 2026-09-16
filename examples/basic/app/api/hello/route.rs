use next_rust::prelude::*;

pub async fn GET(req: Request) -> Response {
    let name = req.query_param("name").unwrap_or_else(|| "world".into());
    Response::json(&serde_json::json!({ "hello": name }))
}
