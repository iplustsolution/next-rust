use next_rust::prelude::*;

use crate::actions::{self, ENTRIES};

#[client]
fn EntryCounter(count: usize, url: String) -> impl View {
    p![
        "Entries: ",
        strong![data("nr-text", "count"), count],
        " ",
        button![on("click", format!("action:{url}->count")), "Refresh"],
    ]
}

pub fn Page(form: FormState) -> impl View {
    let entries = ENTRIES.lock().unwrap().clone();
    div![
        h1!["Guestbook"],
        form![
            action!(actions::sign_guestbook),
            label!["Name ", input![name("name"), value(form.value("name"))]],
            small![data("nr-error", "name"), form.error("name").unwrap_or_default().to_owned()],
            label!["Message ", textarea![name("message"), form.value("message").to_owned()]],
            small![data("nr-error", "message"), form.error("message").unwrap_or_default().to_owned()],
            p![data("nr-error", "_form"), form.message.clone().unwrap_or_default()],
            button![r#type("submit"), "Sign"],
        ],
        EntryCounter(entries.len(), action!(actions::count_entries).url()),
        ul![each(entries, |e| li![strong![e.name], ": ", e.message])],
    ]
}
