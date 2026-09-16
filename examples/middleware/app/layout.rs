use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Middleware").title_template("%s | Middleware")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["Middleware"]], main![children]]
}
