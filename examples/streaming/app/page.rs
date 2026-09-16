use std::time::Duration;

use next_rust::prelude::*;

/// Streaming applies to request-time rendering; static pages are pre-rendered whole.
pub const RENDERING: Rendering = Rendering::Dynamic;

async fn weather() -> impl View {
    next_rust::tokio::time::sleep(Duration::from_millis(60)).await;
    p![class("weather"), "Sunny, 24°C"]
}

async fn headlines() -> impl View {
    next_rust::tokio::time::sleep(Duration::from_millis(10)).await;
    ul![class("news"), li!["Rust 2.0 announced (not really)"]]
}

pub fn Page() -> impl View {
    div![
        h1!["Streaming SSR"],
        suspense(p!["Loading weather…"], weather()),
        suspense(p!["Loading headlines…"], headlines()),
    ]
}
