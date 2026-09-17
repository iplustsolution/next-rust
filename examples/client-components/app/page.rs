use next_rust::prelude::*;

use crate::components::{Counter, Disclosure};

pub fn Page() -> impl View {
    div![
        h1!["Client components"],
        p!["The page is server-rendered HTML; only the islands below are interactive."],
        Counter(3),
        Disclosure(false, "More".into()),
        Link!(href = "/about", "Client-side navigation to /about"),
    ]
}
