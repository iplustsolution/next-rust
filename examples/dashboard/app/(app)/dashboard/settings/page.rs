use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Settings")
}

pub fn Page() -> impl View {
    h1!["Settings"]
}
