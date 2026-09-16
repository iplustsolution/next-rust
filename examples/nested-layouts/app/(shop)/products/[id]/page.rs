use next_rust::prelude::*;

pub fn metadata(params: Params) -> Metadata {
    Metadata::new().title(format!("Product {}", params.get("id").unwrap_or_default()))
}

pub fn Page(params: Params) -> impl View {
    h1![format!("Product {}", params.get("id").unwrap_or_default())]
}
