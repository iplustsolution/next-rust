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
    // Header and footer are shared by every page, so client navigations never
    // replace them: the logo is requested once per visit.
    div![
        class("min-h-dvh bg-bg font-sans text-base/[1.6] text-fg antialiased selection:bg-accent-soft selection:text-fg"),
        a![
            class("absolute -top-12 left-3 z-[100] rounded-lg bg-accent px-3.5 py-2 font-semibold text-on-accent focus:top-3"),
            href("#content"),
            "Skip to content"
        ],
        crate::ui::header(),
        children,
        crate::ui::footer(),
    ]
}
