use std::time::Duration;

use next_rust::prelude::*;

use crate::components::Counter;

pub const RENDERING: Rendering = Rendering::Dynamic;

/// Streamed in after the shell, carrying an island: hydration has to reach
/// content that arrived later.
async fn late() -> impl View {
    next_rust::tokio::time::sleep(Duration::from_millis(120)).await;
    div![id("late"), p![id("late-text"), "arrived late"], Counter(7)]
}

pub fn Page() -> impl View {
    div![
        h1![id("title"), "Slow"],
        p![id("where"), "slow"],
        suspense(p![id("late-fallback"), "Loading…"], late()),
    ]
}
