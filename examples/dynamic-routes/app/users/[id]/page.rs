use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct UserParams {
    id: u64,
}

/// `/users/abc` does not parse as `u64` and renders the 404 page.
pub fn Page(Path(p): Path<UserParams>) -> impl View {
    h1![format!("User #{}", p.id)]
}
