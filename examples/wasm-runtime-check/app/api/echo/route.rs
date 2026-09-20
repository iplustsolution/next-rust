use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct Body {
    text: String,
}

/// A normal route a form can post to: the answer is JSON, not a page, so the
/// runtime has to hand it back to the form.
pub async fn POST(mut req: Request) -> Result<Json<serde_json::Value>> {
    let body: Body = req.form().await?;
    if body.text.is_empty() {
        return Err(Error::validation([("text", "Say something")]));
    }
    Ok(Json(serde_json::json!({ "text": body.text })))
}

pub async fn GET() -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
