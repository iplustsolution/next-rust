use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    fragment![
        main![
            id("content"),
            class("not-found"),
            p![class("code-404"), "404"],
            h1!["This page doesn't exist"],
            p!["It may have moved when the docs were reorganized. Search usually finds it."],
            div![
                class("hero-actions"),
                a![class("button primary"), href("/docs"), "Go to the docs"],
                a![class("button"), href("/docs/search"), "Search"],
            ],
        ],
    ]
}
