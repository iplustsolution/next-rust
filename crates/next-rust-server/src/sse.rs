//! Server-Sent Events.

use std::time::Duration;

use bytes::Bytes;
use futures_util::stream::{Stream, StreamExt};

use crate::response::{Body, Response};

/// One SSE message.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<Duration>,
}

impl SseEvent {
    pub fn data(data: impl Into<String>) -> Self {
        SseEvent { data: data.into(), ..Default::default() }
    }

    /// Serialize `value` as JSON data.
    pub fn json<T: serde::Serialize>(value: &T) -> Self {
        SseEvent::data(serde_json::to_string(value).unwrap_or_default())
    }

    pub fn event(mut self, name: impl Into<String>) -> Self {
        self.event = Some(name.into());
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn retry(mut self, d: Duration) -> Self {
        self.retry = Some(d);
        self
    }

    /// Wire format. Newlines in `event`/`id` are stripped so a value can
    /// never inject additional fields.
    pub fn to_wire(&self) -> String {
        let clean = |s: &str| s.replace(['\r', '\n'], "");
        let mut out = String::new();
        if let Some(e) = &self.event {
            out.push_str(&format!("event: {}\n", clean(e)));
        }
        if let Some(id) = &self.id {
            out.push_str(&format!("id: {}\n", clean(id)));
        }
        if let Some(r) = self.retry {
            out.push_str(&format!("retry: {}\n", r.as_millis()));
        }
        for line in self.data.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
            out.push_str("data: ");
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
        out
    }
}

impl Response {
    /// Stream server-sent events. A keep-alive comment is sent every 15
    /// seconds of inactivity; the response ends when `events` ends.
    pub fn sse<S>(events: S) -> Response
    where
        S: Stream<Item = SseEvent> + Send + 'static,
    {
        Response::sse_with_keep_alive(events, Duration::from_secs(15))
    }

    pub fn sse_with_keep_alive<S>(events: S, keep_alive: Duration) -> Response
    where
        S: Stream<Item = SseEvent> + Send + 'static,
    {
        let events = Box::pin(events);
        let stream = futures_util::stream::unfold(Some(events), move |state| async move {
            let mut events = state?;
            tokio::select! {
                next = events.next() => next.map(|ev| (Ok::<Bytes, crate::request::BoxError>(Bytes::from(ev.to_wire())), Some(events))),
                _ = tokio::time::sleep(keep_alive) => Some((Ok(Bytes::from_static(b": keep-alive\n\n")), Some(events))),
            }
        });
        let mut r = Response::new(http::StatusCode::OK, Body::Stream(Box::pin(stream)));
        r.set_header("content-type", "text/event-stream");
        r.set_header("cache-control", "no-cache");
        r.set_header("x-accel-buffering", "no");
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_format() {
        let e = SseEvent::data("line1\nline2").event("up\ndate").id("7");
        assert_eq!(e.to_wire(), "event: update\nid: 7\ndata: line1\ndata: line2\n\n");
    }
}
