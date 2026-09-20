use std::time::Duration;

use next_rust::prelude::*;

/// Three server-sent events, for `window.nextRust.stream`.
pub async fn GET() -> Response {
    let events = next_rust::futures::stream::unfold(0u32, |n| async move {
        if n >= 3 {
            return None;
        }
        next_rust::tokio::time::sleep(Duration::from_millis(30)).await;
        Some((SseEvent::data(format!("tick {n}")).event("tick").id(n.to_string()), n + 1))
    });
    Response::sse(events)
}
