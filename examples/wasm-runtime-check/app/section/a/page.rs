use next_rust::prelude::*;

pub const RENDERING: Rendering = Rendering::Dynamic;

pub fn Page() -> impl View {
    div![h3![id("title"), "A"], p![id("where"), "section-a"]]
}
