use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Streaming").title_template("%s | Streaming")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["Streaming"]], main![children]]
}
