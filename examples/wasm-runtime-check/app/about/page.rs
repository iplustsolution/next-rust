use next_rust::prelude::*;

pub const RENDERING: Rendering = Rendering::Dynamic;

pub fn Page() -> impl View {
    div![
        h1![id("title"), "About"],
        p![id("where"), "about"],
        crate::module_island::Badge("about".into()),
        p![id("anchor-target"), style("margin-top:1500px"), "anchor target"],
        Link!(href = "/about#anchor-target", id("same-page-hash"), "Jump to the anchor"),
    ]
}
