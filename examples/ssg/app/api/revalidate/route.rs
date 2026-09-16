use next_rust::prelude::*;

/// On-demand revalidation, e.g. called by a CMS webhook.
pub async fn POST(req: Request) -> Response {
    let tag = req.query_param("tag").unwrap_or_else(|| "stats".into());
    next_rust::revalidate_tag(&tag).await;
    Response::json(&serde_json::json!({ "revalidated": tag }))
}
