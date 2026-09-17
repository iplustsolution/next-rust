use next_rust::prelude::*;

/// Interactive counter using the declarative island runtime (no custom JS).
#[client]
pub fn Counter(count: i32) -> impl View {
    div![
        class("counter"),
        button![on("click", "decrement:count"), aria("label", "Decrease"), "−"],
        output![data("nr-text", "count"), count],
        button![on("click", "increment:count"), aria("label", "Increase"), "+"],
    ]
}

/// Show/hide content.
#[client]
pub fn Disclosure(open: bool, label: String) -> impl View {
    div![
        button![on("click", "toggle:open"), aria("expanded", open.to_string()), label],
        p![data("nr-show", "open"), hidden(!open), "Hidden details, revealed on the client."],
    ]
}
