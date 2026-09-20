use next_rust::prelude::*;

pub const RENDERING: Rendering = Rendering::Static;

pub fn metadata() -> Metadata {
    Metadata::new().title("Icons").description(
        "Search every Lucide icon that ships with Next Rust and copy it as a Rust function call or as SVG.",
    )
}

pub fn Page() -> impl View {
    crate::icon_gallery::page()
}
