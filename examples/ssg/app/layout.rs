use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("SSG").title_template("%s | SSG")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["SSG"]], main![children]]
}
