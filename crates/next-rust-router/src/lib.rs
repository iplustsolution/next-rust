//! Filesystem routing for Next Rust.
//!
//! The pipeline is:
//!
//! ```text
//! directory ──scan──▶ RouteNode tree ──flatten──▶ Vec<Route> ──validate──▶ diagnostics
//!                                                     │
//!                                                     └──▶ Matcher (trie, used per request)
//! ```
//!
//! Scanning happens at build time and when files change during development;
//! request handling only ever touches the in-memory [`Matcher`].

#![forbid(unsafe_code)]

mod flatten;
pub mod manifest;
mod matcher;
mod params;
pub mod rank;
mod scan;
mod segment;
mod tree;
mod validate;

pub use flatten::{InterceptTarget, Route, RouteKind, SegmentEntry, SlotEntry, flatten};
pub use matcher::{InsertError, Match, Matcher};
pub use params::{ParamValue, Params};
pub use rank::{SegmentRank, compare_patterns, rank_of};
pub use scan::{SPECIAL_FILES, ScanOptions, ScanOutput, scan, scan_project};
pub use segment::{InterceptLevel, PatternSegment, RoutePattern, SegmentKind, parse_segment};
pub use tree::{RouteNode, SpecialFiles};
pub use validate::validate;

/// Percent-decode a single URL path segment. Returns `None` for malformed
/// escapes or invalid UTF-8.
pub fn decode_segment(seg: &str) -> Option<String> {
    if !seg.contains('%') {
        return Some(seg.to_owned());
    }
    let bytes = seg.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let s = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(s, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Percent-encode a path segment for use in a URL.
pub fn encode_segment(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for b in seg.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
