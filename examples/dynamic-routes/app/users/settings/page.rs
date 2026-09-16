use next_rust::prelude::*;

pub fn Page() -> impl View {
    h1!["User settings (static route wins over [id])"]
}
