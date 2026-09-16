use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Routes").title_template("%s | Routes")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["Routes"]], main![children]]
}
