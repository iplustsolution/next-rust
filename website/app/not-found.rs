use next_rust::prelude::*;
use crate::ui::{ACTIONS, BUTTON, BUTTON_PRIMARY};

pub fn NotFound() -> impl View {
    main![
        id("content"),
        class("mx-auto max-w-[560px] px-6 py-30 text-center"),
        p![class("font-mono text-sm/[normal] font-semibold tracking-[0.2em] text-muted"), "404"],
        h1![class("mt-3.5 mb-4 text-[clamp(30px,5vw,44px)] font-bold tracking-[-0.035em]"), "This page doesn't exist"],
        p![class("text-muted"), "It may have moved when the docs were reorganized. Search usually finds it."],
        div![
            class(ACTIONS),
            a![class(BUTTON_PRIMARY), href("/docs"), "Go to the docs"],
            a![class(BUTTON), href("/docs/search"), "Search"],
        ],
    ]
}
