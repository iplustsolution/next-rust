use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Basic").title_template("%s · Basic").description("A minimal Next Rust app")
}

pub fn Layout(children: Children) -> impl View {
    div![
        global_css!("globals.css"),
        header![nav![Link!(href = "/", "Home"), " ", Link!(href = "/about", "About"), " ", Link!(href = "/blog/hello", "Blog")]],
        main![children],
        footer!["Built with Next Rust"],
    ]
}
