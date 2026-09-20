//! Just enough JSON to read one flat options object.
//!
//! `nextRust.toast("Saved")` and `nextRust.toast({ title, message, color,
//! duration })` reach the module as the event detail, serialised by the shim.
//! That is the only JSON the kit reads, and it is always a string or a flat
//! object of strings and numbers — so this is fifty lines rather than a
//! parser crate, which on `wasm32` would cost more than the whole toaster.
//!
//! Anything it does not understand reads as absent: a malformed detail
//! produces a toast with no title rather than no toast.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;

/// A parsed options object.
#[derive(Default, Debug, PartialEq)]
pub struct Options {
    /// The whole detail, when it was a bare string.
    pub text: Option<String>,
    /// Key/value pairs, in the order they were written.
    pub fields: Vec<(String, String)>,
}

impl Options {
    /// The value of a key, or the bare string under `message`.
    pub fn get(&self, key: &str) -> Option<&str> {
        if let Some(text) = &self.text {
            return (key == "message").then_some(text.as_str());
        }
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The value of a key as a number.
    ///
    /// Read with the kit's own `parseFloat` rather than `str::parse`: the
    /// standard library's float parser is exact, and exactness costs about
    /// four kilobytes of the module for a value that is a toast's duration
    /// in whole milliseconds.
    pub fn number(&self, key: &str) -> Option<f64> {
        let text = self.get(key)?;
        let value = crate::util::parse_number(text);
        // `parse_number` reads nothing as zero, so an unparseable value has
        // to be told apart from a real one.
        (value != 0.0 || text.trim_start().starts_with(['0', '.', '-', '+'])).then_some(value)
    }
}

/// Read a flat JSON value: a string, or an object of strings and numbers.
pub fn parse(text: &str) -> Options {
    let bytes = text.as_bytes();
    let mut at = skip_space(bytes, 0);
    match bytes.get(at) {
        Some(b'"') => {
            let (value, _) = string(bytes, at);
            Options { text: Some(value), fields: Vec::new() }
        }
        Some(b'{') => {
            at += 1;
            let mut fields = Vec::new();
            loop {
                at = skip_space(bytes, at);
                match bytes.get(at) {
                    Some(b',') => at += 1,
                    Some(b'"') => {
                        let (key, next) = string(bytes, at);
                        at = skip_space(bytes, next);
                        if bytes.get(at) != Some(&b':') {
                            break;
                        }
                        at = skip_space(bytes, at + 1);
                        let (value, next) = value(bytes, at);
                        at = next;
                        if let Some(value) = value {
                            fields.push((key, value));
                        }
                    }
                    _ => break,
                }
            }
            Options { text: None, fields }
        }
        _ => Options::default(),
    }
}

/// Skip whitespace.
fn skip_space(bytes: &[u8], mut at: usize) -> usize {
    while matches!(bytes.get(at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        at += 1;
    }
    at
}

/// Read a quoted string starting at `at`, and where it ended.
fn string(bytes: &[u8], at: usize) -> (String, usize) {
    let mut out = String::new();
    let mut i = at + 1;
    while let Some(&b) = bytes.get(i) {
        match b {
            b'"' => return (out, i + 1),
            b'\\' => {
                i += 1;
                match bytes.get(i) {
                    Some(b'n') => out.push('\n'),
                    Some(b't') => out.push('\t'),
                    Some(b'r') => out.push('\r'),
                    Some(b'b') => out.push('\u{8}'),
                    Some(b'f') => out.push('\u{c}'),
                    Some(b'u') => {
                        let hex = bytes.get(i + 1..i + 5).and_then(|h| core::str::from_utf8(h).ok());
                        let code = hex.and_then(|h| u32::from_str_radix(h, 16).ok()).unwrap_or(0xfffd);
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        i += 4;
                    }
                    Some(&other) => out.push(char::from(other)),
                    None => break,
                }
                i += 1;
            }
            _ => {
                // The bytes came out of a browser's `JSON.stringify`, so they
                // are valid UTF-8; copy the whole sequence.
                let len = utf8_len(b);
                if let Some(chunk) = bytes.get(i..i + len).and_then(|c| core::str::from_utf8(c).ok()) {
                    out.push_str(chunk);
                }
                i += len;
            }
        }
    }
    (out, i)
}

/// Bytes in a UTF-8 sequence starting with this one.
fn utf8_len(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// Read one value, as text. `null`, `true`, `false`, numbers and strings are
/// values; an array or a nested object is skipped and reads as absent.
fn value(bytes: &[u8], at: usize) -> (Option<String>, usize) {
    match bytes.get(at) {
        Some(b'"') => {
            let (text, next) = string(bytes, at);
            (Some(text), next)
        }
        Some(b'{') | Some(b'[') => (None, skip_nested(bytes, at)),
        Some(_) => {
            let end = bytes[at..]
                .iter()
                .position(|b| matches!(b, b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t'))
                .map_or(bytes.len(), |n| at + n);
            let text = core::str::from_utf8(&bytes[at..end]).unwrap_or_default();
            (if text == "null" { None } else { Some(text.to_owned()) }, end)
        }
        None => (None, at),
    }
}

/// Skip a nested object or array.
fn skip_nested(bytes: &[u8], at: usize) -> usize {
    let mut depth = 0i32;
    let mut i = at;
    let mut in_string = false;
    while let Some(&b) = bytes.get(i) {
        if in_string {
            match b {
                b'\\' => i += 1,
                b'"' => in_string = false,
                _ => {}
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_string_is_the_message() {
        let o = parse("\"Saved\"");
        assert_eq!(o.get("message"), Some("Saved"));
        assert_eq!(o.get("title"), None);
    }

    #[test]
    fn an_object_reads_its_fields() {
        let o = parse(r#"{"title":"Done","message":"Saved","color":"success","duration":4000}"#);
        assert_eq!(o.get("title"), Some("Done"));
        assert_eq!(o.get("message"), Some("Saved"));
        assert_eq!(o.get("color"), Some("success"));
        assert_eq!(o.number("duration"), Some(4000.0));
    }

    #[test]
    fn escapes_and_unicode_survive() {
        let o = parse(r#"{"message":"a\"b\ncé—"}"#);
        assert_eq!(o.get("message"), Some("a\"b\nc\u{e9}\u{2014}"));
        assert_eq!(parse(r#"{"message":"héllo"}"#).get("message"), Some("héllo"));
    }

    #[test]
    fn null_and_nesting_read_as_absent() {
        let o = parse(r#"{"title":null,"extra":{"a":1},"list":[1,2],"message":"x"}"#);
        assert_eq!(o.get("title"), None);
        assert_eq!(o.get("extra"), None);
        assert_eq!(o.get("list"), None);
        assert_eq!(o.get("message"), Some("x"), "the keys after a nested value are still read");
    }

    #[test]
    fn nothing_understandable_is_no_options_rather_than_a_panic() {
        for text in ["", "null", "0", "{", r#"{"a"#, r#"{"a":"#, "[1,2]", "{oops}", "\"unterminated"] {
            let _ = parse(text);
        }
        assert_eq!(parse("null"), Options::default());
        assert_eq!(parse("").get("message"), None);
    }
}
