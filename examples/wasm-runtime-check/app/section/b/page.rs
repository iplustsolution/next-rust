use next_rust::prelude::*;

use crate::components::Counter;

pub const RENDERING: Rendering = Rendering::Dynamic;

pub fn Page() -> impl View {
    div![
        h3![id("title"), "B"],
        p![id("where"), "section-b"],
        // An island that only exists after a client navigation.
        Counter(10),
    ]
}
