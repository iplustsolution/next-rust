use next_rust::prelude::*;

use crate::posts;

/// Refresh pre-rendered posts at most once an hour.
pub const REVALIDATE: u64 = 3600;
/// `revalidate_tag("posts")` invalidates every post.
pub const TAGS: &[&str] = &["posts"];
/// Unknown slugs are 404s instead of being rendered on demand.
pub const DYNAMIC_PARAMS: bool = false;

pub async fn generate_params() -> Vec<Params> {
    posts::POSTS.iter().map(|p| Params::new().with("slug", p.slug)).collect()
}

pub fn metadata(params: Params) -> Result<Metadata> {
    let post = posts::find(params.get("slug").unwrap_or_default()).or_not_found()?;
    Ok(Metadata::new().title(post.title).description(post.summary).open_graph(OpenGraph {
        kind: Some("article".into()),
        ..Default::default()
    }))
}

pub fn Page(params: Params) -> Result<impl View> {
    let post = posts::find(params.get("slug").unwrap_or_default()).or_not_found()?;
    Ok(article![
        h1![post.title],
        p![time![datetime(post.date), post.date]],
        each(post.paragraphs, |text| p![*text]),
    ])
}
