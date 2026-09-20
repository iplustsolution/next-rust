use next_rust::prelude::*;

pub const RENDERING: Rendering = Rendering::Dynamic;

#[derive(serde::Deserialize, Default)]
pub struct Search {
    q: Option<String>,
}

const FRUIT: &[&str] = &["apple", "apricot", "banana", "blueberry", "cherry", "cranberry", "damson"];

/// A live search: `data-nr-live` sends the form as the visitor types, drops
/// the request of the keystroke before, replaces history, and keeps focus and
/// the caret. `data-nr-target` narrows the swap to one element, which the
/// runtime morphs instead of replacing.
pub fn Page(Query(q): Query<Search>) -> impl View {
    let needle = q.q.clone().unwrap_or_default();
    let hits: Vec<&&str> =
        FRUIT.iter().filter(|f| needle.is_empty() || f.contains(needle.to_lowercase().as_str())).collect();
    div![
        h1![id("title"), "Live"],
        p![id("where"), "live"],
        form![
            id("live"),
            method("get"),
            action("/live"),
            data("nr-live", "120"),
            data("nr-target", "#results"),
            input![id("q"), name("q"), value(needle.clone()), autocomplete("off")],
        ],
        ul![
            id("results"),
            each(hits, |f| li![data("nr-key", f.to_owned()), (*f).to_owned()])
        ],
        p![id("q-echo"), needle],
        p![id("renders"), crate::render_counter::next_layout().to_string()],
    ]
}
