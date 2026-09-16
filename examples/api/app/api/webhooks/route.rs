use next_rust::prelude::*;

/// Webhook receiver. Real providers (Stripe, GitHub) sign the *raw* body;
/// verify the signature over `raw` with an HMAC crate before parsing.
pub async fn POST(mut req: Request) -> Result<Response> {
    let expected = std::env::var("WEBHOOK_TOKEN").unwrap_or_else(|_| "dev-token".into());
    let provided = req.header("x-webhook-token").unwrap_or_default().to_owned();
    if !next_rust::constant_time_eq(expected.as_bytes(), provided.as_bytes()) {
        return Err(Error::http(401, "invalid webhook token"));
    }
    let raw = req.raw_body().await?;
    Ok(Response::json(&serde_json::json!({ "received_bytes": raw.len() })))
}
