use next_rust::prelude::*;

pub fn Page() -> impl View {
    h1!["About (static, never revalidated)"]
}
