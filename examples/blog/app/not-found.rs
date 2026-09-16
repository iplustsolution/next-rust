use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    div![h1!["Page not found"], Link!(href = "/", "Back to the blog")]
}
