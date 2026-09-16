use next_rust::prelude::*;

pub fn Page(params: Params) -> impl View {
    let filters = params.get_all("filters").unwrap_or_default();
    if filters.is_empty() {
        h1!["All products".to_owned()]
    } else {
        h1![format!("Filtered by {}", filters.join(" > "))]
    }
}
