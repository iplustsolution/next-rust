use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new()
        .title("Next Rust")
        .title_template("%s · Next Rust")
        .description("A full-stack web framework for Rust with filesystem routing, streaming server rendering and single-binary deploys.")
        .icon("/logo.svg")
        .theme_color("#0b0b0d")
}

pub fn Layout(children: Children) -> impl View {
    fragment![
        global_css!("globals.css"),
        a![class("skip-link"), href("#content"), "Skip to content"],
        children
    ]
}
