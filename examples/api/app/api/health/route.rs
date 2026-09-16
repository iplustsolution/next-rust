use next_rust::prelude::*;

pub fn GET() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
