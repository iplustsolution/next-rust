use next_rust::prelude::*;

/// The declarative island runtime: no custom JavaScript, every binding is a
/// `data-nr-*` attribute the browser runtime reads.
#[client]
pub fn Counter(count: i32) -> impl View {
    div![
        id("counter"),
        button![id("count-down"), on("click", "decrement:count"), aria("label", "Decrease"), "−"],
        output![id("count"), data("nr-text", "count"), count],
        button![id("count-up"), on("click", "increment:count"), aria("label", "Increase"), "+"],
        input![id("count-bind"), data("nr-bind", "count"), r#type("number")],
        span![id("count-class"), data("nr-class-big", "count"), "class target"],
    ]
}

/// Every other operation of the declarative runtime: `set:`, `prevent`,
/// `navigate:`, `action:`, nested paths and a step argument.
#[client]
pub fn Ops(open: bool, user: Who, n: i32, action_url: String) -> impl View {
    // The props are the island's initial state; the markup below reads them
    // through `data-nr-text`, so the arguments themselves are only rendered
    // where a server-rendered value has to match what hydration will show.
    div![
        id("ops"),
        data("nr-initial", format!("{} {} {}", open, user.name, n)),
        button![id("op-set"), on("click", "set:user.name=\"Ada\"; set:open=true"), "set"],
        button![id("op-set-num"), on("click", "set:n=41"), "set number"],
        button![id("op-dec"), on("click", "decrement:n,5"), "-5"],
        button![id("op-inc"), on("click", "increment:n,3"), "+3"],
        a![id("op-prevent"), href("/about"), on("click", "prevent; set:open=false"), "prevent"],
        button![id("op-navigate"), on("click", "navigate:/section/a"), "navigate"],
        button![id("op-action"), on("click", format!("action:{action_url}->echoed")), "action"],
        button![id("op-action-bad"), on("click", "action:/nope"), "bad action"],
        span![id("op-name"), data("nr-text", "user.name")],
        span![id("op-n"), data("nr-text", "n")],
        span![id("op-open"), data("nr-text", "open")],
        span![id("op-echoed"), data("nr-text", "echoed")],
        span![id("op-error"), data("nr-text", "error")],
    ]
}

#[derive(serde::Serialize, Clone)]
pub struct Who {
    pub name: String,
}

/// `data-nr-show` and `toggle:`.
#[client]
pub fn Disclosure(open: bool, label: String) -> impl View {
    div![
        id("disclosure"),
        button![id("disclosure-toggle"), on("click", "toggle:open"), aria("expanded", open.to_string()), label],
        p![id("disclosure-body"), data("nr-show", "open"), hidden(!open), "Hidden details, revealed on the client."],
    ]
}
