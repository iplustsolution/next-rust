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

    /// xorshift64*: seeded, so a failure is reproducible.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn text(&mut self) -> String {
            const PIECES: &[&str] = &[
                "\r",
                "\n",
                "\r\n",
                "\n\n",
                "\r\r",
                "\u{2028}",
                "\u{2029}",
                "\u{0}",
                "\u{7f}",
                "\u{feff}",
                "data: x",
                "event: steal",
                "id: 1",
                "retry: 1",
                ": comment",
                ":",
                " ",
                "\t",
                "a",
                "héllo",
                "}",
                "\\n",
            ];
            (0..self.below(8)).map(|_| PIECES[self.below(PIECES.len())]).collect()
        }

        fn maybe(&mut self) -> Option<String> {
            (!self.next().is_multiple_of(3)).then(|| self.text())
        }
    }

    /// Fields an SSE stream may carry, in the order `to_wire` writes them.
    fn fields(wire: &str) -> Result<Vec<(String, String)>, String> {
        let lines: Vec<&str> = wire.split('\n').collect();
        let (body, tail) = lines.split_at(lines.len() - 2);
        if tail != ["", ""] {
            return Err(format!("no blank line at the end of {wire:?}"));
        }
        let mut out = Vec::new();
        for line in body {
            if line.is_empty() {
                return Err(format!("a second event in {wire:?}"));
            }
            let (name, value) = line.split_once(':').ok_or(format!("bare line {line:?}"))?;
            out.push((name.to_owned(), value.strip_prefix(' ').unwrap_or(value).to_owned()));
        }
        Ok(out)
    }

    /// No value of any field can add a field or a second event.
    #[test]
    fn no_generated_event_can_inject_a_field_or_a_second_event() {
        let mut rng = Rng(0x55E0_0BAD_F00D);
        for _ in 0..12_000 {
            let ev = SseEvent {
                event: rng.maybe(),
                data: rng.text(),
                id: rng.maybe(),
                retry: (rng.next().is_multiple_of(3)).then(|| Duration::from_millis(rng.next())),
            };
            let wire = ev.to_wire();
            assert!(wire.ends_with("\n\n"), "{wire:?}");
            let got = fields(&wire).unwrap_or_else(|e| panic!("{e}\n{ev:?}"));
            let count = |name: &str| got.iter().filter(|(n, _)| n == name).count();
            assert_eq!(count("event"), usize::from(ev.event.is_some()), "{ev:?} -> {wire:?}");
            assert_eq!(count("id"), usize::from(ev.id.is_some()), "{ev:?} -> {wire:?}");
            assert_eq!(count("retry"), usize::from(ev.retry.is_some()), "{ev:?} -> {wire:?}");
            assert_eq!(count("data"), got.len() - count("event") - count("id") - count("retry"), "{wire:?}");
            for (name, _) in &got {
                assert!(["event", "id", "retry", "data"].contains(&name.as_str()), "field {name:?} in {wire:?}");
            }
            // The data a reader reassembles is the data that was sent, with
            // every line ending normalised to `\n` (what the spec requires).
            let joined: Vec<&str> = got.iter().filter(|(n, _)| n == "data").map(|(_, v)| v.as_str()).collect();
            let expected = ev.data.replace("\r\n", "\n").replace('\r', "\n");
            assert_eq!(joined.join("\n"), expected, "{ev:?}");
        }
    }

    #[test]
    fn the_separators_a_reader_acts_on_are_the_only_ones_that_split_data() {
        // `\u{2028}`/`\u{2029}` are line separators to a JavaScript parser but
        // not to the SSE framing, so they stay inside one `data:` line.
        let wire = SseEvent::data("a\u{2028}b\u{2029}c").to_wire();
        assert_eq!(wire, "data: a\u{2028}b\u{2029}c\n\n");
        // A NUL is data like any other byte.
        assert_eq!(SseEvent::data("a\u{0}b").to_wire(), "data: a\u{0}b\n\n");
        // CR, LF and CRLF all start a new `data:` line, and never a new event.
        assert_eq!(SseEvent::data("a\rb\nc\r\nd").to_wire(), "data: a\ndata: b\ndata: c\ndata: d\n\n");
        assert_eq!(SseEvent::data("").to_wire(), "data: \n\n");
        assert_eq!(SseEvent::data("\n").to_wire(), "data: \ndata: \n\n");
        // A name is stripped of line endings, so it cannot open a field.
        assert_eq!(SseEvent::data("x").event("a\nid: 9\nb").to_wire(), "event: aid: 9b\ndata: x\n\n");
        assert_eq!(SseEvent::data("x").id("1\r\nretry: 0").to_wire(), "id: 1retry: 0\ndata: x\n\n");
    }

    #[test]
    fn a_very_long_event_costs_its_length_once() {
        let big = "x\n".repeat(50_000);
        let start = std::time::Instant::now();
        let wire = SseEvent::data(big.clone()).event(big.clone()).id(big).to_wire();
        assert!(wire.ends_with("\n\n"));
        assert!(fields(&wire).is_ok());
        assert!(start.elapsed() < std::time::Duration::from_secs(10), "took {:?}", start.elapsed());
    }
}
