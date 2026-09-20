use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new()
        .title("Component gallery")
        .description("Every built-in Next Rust UI component, rendered live with the code that makes it.")
}

pub fn Page() -> impl View {
    crate::gallery::page()
}
