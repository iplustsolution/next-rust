use next_rust::prelude::*;

pub fn Page() -> impl View {
    div![
        h1!["Hello World"],
        p!["Welcome to Next Rust."],
    ]
}
