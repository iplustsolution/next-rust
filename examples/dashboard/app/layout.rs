use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Acme Dashboard").title_template("%s | Acme Dashboard")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["Acme Dashboard"]], main![children]]
}
