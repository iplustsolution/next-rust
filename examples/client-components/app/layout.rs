use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Islands").title_template("%s | Islands")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["Islands"]], main![children]]
}
