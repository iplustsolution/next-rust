use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Guestbook").title_template("%s | Guestbook")
}

pub fn Layout(children: Children) -> impl View {
    div![header![strong!["Guestbook"]], main![children]]
}
