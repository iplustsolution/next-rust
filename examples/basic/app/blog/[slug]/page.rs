use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct Params {
    slug: String,
}

pub async fn generate_params() -> Vec<next_rust::Params> {
    ["hello", "rust"].into_iter().map(|s| next_rust::Params::new().with("slug", s)).collect()
}

pub fn metadata(Path(p): Path<Params>) -> Metadata {
    Metadata::new().title(p.slug)
}

pub async fn Page(Path(p): Path<Params>) -> Result<impl View> {
    if p.slug == "missing" {
        return Err(not_found());
    }
    Ok(article![h1![format!("Post: {}", p.slug)], p!["Rendered from a dynamic route."]])
}
