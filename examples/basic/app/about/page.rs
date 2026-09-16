use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("About")
}

pub fn Page() -> impl View {
    let styles = css_module!("about.module.css");
    section![class(styles.card), h1!["About"], p![class(styles.muted), "This page is rendered statically at build time."]]
}
