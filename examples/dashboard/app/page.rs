use next_rust::prelude::*;

pub fn Page() -> impl View {
    div![h1!["Welcome"], Link!(href = "/dashboard", "Open the dashboard")]
}
