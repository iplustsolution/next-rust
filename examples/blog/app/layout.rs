use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new()
        .title("The Rust Blog")
        .title_template("%s — The Rust Blog")
        .description("Articles about building web apps with Next Rust")
        .open_graph(OpenGraph { site_name: Some("The Rust Blog".into()), kind: Some("website".into()), ..Default::default() })
}

pub fn Layout(children: Children) -> impl View {
    div![
        header![nav![Link!(href = "/", "Home"), " · ", Link!(href = "/blog", "Archive")]],
        main![children],
        footer![small!["© 2026 The Rust Blog"]],
    ]
}
