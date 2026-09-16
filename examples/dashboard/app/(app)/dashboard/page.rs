use std::time::Duration;

use next_rust::prelude::*;

pub async fn Page() -> impl View {
    // Simulate a slow query; loading.rs is streamed first.
    next_rust::tokio::time::sleep(Duration::from_millis(40)).await;
    div![h1!["Overview"], p!["Revenue: $12,345"]]
}
