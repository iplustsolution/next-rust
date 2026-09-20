use next_rust::prelude::*;

/// A nested layout. A navigation between `/section/a` and `/section/b` must
/// reuse it: `#section-render` keeps the number it was first rendered with.
pub fn Layout(children: Children) -> impl View {
    section![
        id("section"),
        h2![id("section-title"), "Section"],
        span![id("section-render"), crate::render_counter::next_layout().to_string()],
        div![id("section-body"), children],
    ]
}
