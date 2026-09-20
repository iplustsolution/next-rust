use next_rust::prelude::*;

use crate::components::{Counter, Disclosure, Ops, Who};

pub const RENDERING: Rendering = Rendering::Dynamic;

pub fn Page() -> impl View {
    div![
        h1![id("title"), "Home"],
        p![id("where"), "home"],
        Counter(3),
        Disclosure(false, "More".into()),
        Ops(false, Who { name: "nobody".into() }, 1, action!(crate::actions::echo_text).url()),
        // Tall enough that the window can be scrolled, for scroll restoration.
        div![id("filler"), style("height:2000px"), ""],
        p![id("bottom"), "bottom of home"],
    ]
}
