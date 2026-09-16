use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("HTML pages").title_template("%s | HTML pages")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["HTML pages"]], main![children]]
}
