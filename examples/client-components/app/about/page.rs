use next_rust::prelude::*;

pub fn Page() -> impl View {
    div![h1!["About"], Link!(href = "/", "Back")]
}
