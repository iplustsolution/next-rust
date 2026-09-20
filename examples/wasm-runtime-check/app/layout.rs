use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new().title("Wasm runtime check").title_template("%s | Wasm runtime check")
}

/// The root layout. `#root-render` carries a number that only changes when
/// this function runs again, so a client navigation that reuses the layout is
/// distinguishable from one that re-renders it.
pub fn Layout(children: Children) -> impl View {
    div![
        id("root"),
        header![
            id("site-header"),
            strong!["Wasm runtime check"],
            span![id("root-render"), crate::render_counter::next_layout().to_string()],
            nav![
                id("nav"),
                Link!(href = "/", id("nav-home"), active_class("on"), "Home"),
                " ",
                Link!(href = "/about", id("nav-about"), active_class("on"), "About"),
                " ",
                Link!(href = "/section/a", id("nav-section-a"), active_class_prefix("on"), "Section A"),
                " ",
                Link!(href = "/section/b", id("nav-section-b"), "Section B"),
                " ",
                Link!(href = "/forms", id("nav-forms"), "Forms"),
                " ",
                Link!(href = "/slow", id("nav-slow"), "Slow"),
                " ",
                a![href("/about"), id("plain-about"), "Plain about"],
                " ",
                a![href("/about"), id("reload-about"), data("nr-reload", ""), "Reload about"],
                " ",
                a![href("https://example.com/"), id("external"), "External"],
            ],
        ],
        main![id("content"), children],
    ]
}
