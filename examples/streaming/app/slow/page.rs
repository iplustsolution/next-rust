use std::time::Duration;

use next_rust::prelude::*;

/// Streaming applies to request-time rendering; static pages are pre-rendered whole.
pub const RENDERING: Rendering = Rendering::Dynamic;

pub async fn Page() -> impl View {
    next_rust::tokio::time::sleep(Duration::from_millis(30)).await;
    h1!["Report ready"]
}
