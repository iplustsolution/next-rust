use next_rust::prelude::*;
use crate::{docs, ui};

/// Every page is known at build time; anything else is a 404.
pub const DYNAMIC_PARAMS: bool = false;

pub async fn generate_params() -> Vec<Params> {
    docs::all().filter(|d| d.slug != docs::INTRODUCTION).map(|d| Params::new().with("slug", d.slug)).collect()
}

pub fn metadata(params: Params) -> Result<Metadata> {
    let doc = docs::find(params.get("slug").unwrap_or_default()).or_not_found()?;
    Ok(Metadata::new().title(doc.title).description(doc.description))
}

pub fn Page(params: Params) -> Result<impl View> {
    let doc = docs::find(params.get("slug").unwrap_or_default())
        .filter(|d| d.slug != docs::INTRODUCTION)
        .or_not_found()?;
    Ok(ui::doc_page(doc))
}
