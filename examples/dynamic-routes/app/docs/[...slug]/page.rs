use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct DocParams {
    slug: Vec<String>,
}

pub fn Page(Path(p): Path<DocParams>) -> impl View {
    div![h1!["Docs"], ol![each(p.slug, |s| li![s])]]
}
