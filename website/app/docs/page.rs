use next_rust::prelude::*;
use crate::{docs, ui};

pub fn metadata() -> Metadata {
    Metadata::new().title("Introduction").description(docs::find(docs::INTRODUCTION).map(|d| d.description).unwrap_or(""))
}

pub fn Page() -> Result<impl View> {
    let doc = docs::find(docs::INTRODUCTION).or_not_found()?;
    Ok(ui::doc_page(doc))
}
