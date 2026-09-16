use std::time::Duration;

use next_rust::futures::StreamExt;
use next_rust::prelude::*;

/// Server-sent events: three ticks, then the stream ends.
pub async fn GET() -> Response {
    let events = next_rust::futures::stream::iter(1..=3).then(|i| async move {
        next_rust::tokio::time::sleep(Duration::from_millis(5)).await;
        SseEvent::data(i.to_string()).event("tick")
    });
    Response::sse(events)
}
