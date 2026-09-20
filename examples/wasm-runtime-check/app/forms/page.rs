use next_rust::prelude::*;

use crate::actions::{self, ENTRIES};

pub const RENDERING: Rendering = Rendering::Dynamic;

pub fn Page(form: FormState, Query(q): Query<Search>) -> impl View {
    let entries = ENTRIES.lock().unwrap().clone();
    div![
        h1![id("title"), "Forms"],
        p![id("where"), "forms"],
        // A server-action form: validation errors land in the `data-nr-error`
        // slots, success redirects through `_redirect`.
        form![
            id("sign"),
            action!(actions::sign),
            novalidate(true),
            label!["Name ", input![id("f-name"), name("name"), value(form.value("name"))]],
            small![id("e-name"), data("nr-error", "name"), form.error("name").unwrap_or_default().to_owned()],
            label!["Message ", textarea![id("f-message"), name("message"), form.value("message").to_owned()]],
            small![
                id("e-message"),
                data("nr-error", "message"),
                form.error("message").unwrap_or_default().to_owned()
            ],
            p![id("e-form"), data("nr-error", "_form"), form.message.clone().unwrap_or_default()],
            input![r#type("hidden"), name("_redirect"), value("/about")],
            button![id("sign-submit"), r#type("submit"), "Sign"],
        ],
        // The same action, staying on the page and writing into a result slot.
        form![
            id("stay"),
            action!(actions::echo),
            novalidate(true),
            data("nr-stay", ""),
            data("nr-reset", ""),
            input![id("s-text"), name("text"), value("")],
            output![id("s-result"), data("nr-result", "")],
            button![id("stay-submit"), r#type("submit"), "Echo"],
        ],
        // A plain GET form: enhanced into a client navigation to the URL it builds.
        form![
            id("search"),
            method("get"),
            action("/forms"),
            input![id("q"), name("q"), value(q.q.clone().unwrap_or_default())],
            button![id("search-submit"), r#type("submit"), "Search"],
        ],
        p![id("q-echo"), q.q.unwrap_or_default()],
        // A POST form that goes to a normal route, not an action.
        form![
            id("post"),
            method("post"),
            action("/api/echo"),
            input![id("p-text"), name("text"), value("hello")],
            button![id("post-submit"), r#type("submit"), "Post"],
        ],
        p![id("entries-count"), entries.len().to_string()],
        ul![id("entries"), each(entries, |e| li![strong![e.name], ": ", e.message])],
    ]
}

#[derive(serde::Deserialize, Default)]
pub struct Search {
    q: Option<String>,
}
