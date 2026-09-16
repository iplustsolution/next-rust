use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("SSR").title_template("%s | SSR")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["SSR"]], main![children]]
}
