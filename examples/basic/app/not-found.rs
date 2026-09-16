use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    div![h1!["404"], p!["We couldn't find that page."], Link!(href = "/", "Go home")]
}
