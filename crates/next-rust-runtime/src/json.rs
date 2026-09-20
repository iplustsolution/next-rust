//! Just enough JSON for the four small shapes the runtime sees.
//!
//! The runtime reads four kinds of body: an action's `{ok, data, errors,
//! redirect}`, a failed request's `{error|message}`, the answer to a posted
//! form, and an island's props. A parser crate would bring a tree, a number
//! tower and an error type along for that — `serde_json` measured at
//! twenty-three times the size of a whole island — so nothing here builds a
//! value. An [`Object`] *is* the text, and asking for a member walks it: a
//! member nobody asks about costs nothing, and the usual question (`body.ok`)
//! stops at the first one.
//!
//! Every byte that reaches this module came off the network, so no function
//! here may panic. Malformed input — a string that is never closed, `\u` with
//! two digits, a lone surrogate, a brace inside a value — answers `None`, or
//! keeps the text as it stands, and never indexes past the end.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A JSON object, scanned lazily: members are found by walking the text, so
/// nothing is allocated for members nobody asks about.
pub struct Object<'a> {
    /// The object as it was written, trimmed, starting at its `{`.
    text: &'a str,
}

impl<'a> Object<'a> {
    /// The text as an object, or `None` when it is not one.
    pub fn new(text: &'a str) -> Option<Self> {
        let text = text.trim();
        (text.as_bytes().first() == Some(&b'{')).then_some(Self { text })
    }

    /// The member's raw JSON text, exactly as it was written.
    pub fn raw(&self, key: &str) -> Option<&'a str> {
        // The first member with this name wins, which is what a reader that
        // stops as soon as it has an answer does.
        self.walk().find(|(name, _)| same_key(name, key)).map(|(_, value)| value)
    }

    /// A string member, unescaped. A member that is not a string is `None`.
    pub fn str(&self, key: &str) -> Option<String> {
        self.raw(key).and_then(unquote)
    }

    /// `true`, else false — which is what `if (body.ok)` asks.
    pub fn bool(&self, key: &str) -> bool {
        self.raw(key) == Some("true")
    }

    pub fn num(&self, key: &str) -> Option<f64> {
        self.raw(key).and_then(crate::num::number)
    }

    /// Every member, as `(key, raw JSON text)`, in the order they were written.
    pub fn members(&self) -> Vec<(String, &'a str)> {
        self.walk().map(|(key, value)| (unescape(key), value)).collect()
    }

    /// Every member whose value is a string, as `(key, value)`. This is what
    /// `Object.entries(body.errors)` needs.
    pub fn entries(&self) -> Vec<(String, String)> {
        self.walk().filter_map(|(key, value)| Some((unescape(key), unquote(value)?))).collect()
    }

    fn walk(&self) -> Members<'a> {
        Members { text: self.text, at: 1 }
    }
}

/// A JSON string, quotes included.
pub fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            // Everything else below a space has no short form of its own.
            c if (c as u32) < 0x20 => {
                out.push_str("\\u00");
                out.push(hex(c as u32 >> 4));
                out.push(hex(c as u32 & 0xf));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A raw value as the text the source puts on screen: `value == null ||
/// typeof value === "object" ? "" : String(value)`. A string comes out
/// unescaped and without its quotes; a number or a boolean as it was written.
pub fn text(raw: &str) -> String {
    let raw = raw.trim();
    if raw == "null" || is_object(raw) {
        return String::new();
    }
    unquote(raw).unwrap_or_else(|| raw.to_string())
}

/// Whether the text is a JSON object or array (`typeof data === "object"`).
pub fn is_object(text: &str) -> bool {
    matches!(text.trim().as_bytes().first(), Some(b'{') | Some(b'['))
}

/// Whether `JSON.parse` would take this text — the question every `try {
/// JSON.parse(text) } catch {}` in the source is really asking. The insides of
/// an object or an array are taken on trust: nothing downstream reads a member
/// that was never written properly anyway.
pub fn parses(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && value_end(text.as_bytes(), 0) == Some(text.len())
}

// ---- walking ----------------------------------------------------------------

/// The members of an object, as `(key as written, value as written)`. It stops
/// at the closing brace, at the end of the text, or at anything it cannot
/// make sense of — a body that was cut in half yields what it did carry.
struct Members<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Iterator for Members<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let b = self.text.as_bytes();
        let mut i = skip_ws(b, self.at);
        if b.get(i) == Some(&b',') {
            i = skip_ws(b, i + 1);
        }
        if b.get(i)? != &b'"' {
            return None; // `}`, the end, or something that is not a member
        }
        let key_end = string_end(b, i)?;
        let key = self.text.get(i + 1..key_end - 1)?;
        let mut j = skip_ws(b, key_end);
        if b.get(j)? != &b':' {
            return None;
        }
        j = skip_ws(b, j + 1);
        let end = value_end(b, j)?;
        self.at = end;
        Some((key, self.text.get(j..end)?))
    }
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while matches!(b.get(i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        i += 1;
    }
    i
}

/// The index just past the string that starts at `i`, whose first byte is a
/// quote. A backslash always takes the byte after it with it, so a `"` that
/// was escaped never ends the string.
fn string_end(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    while let Some(&c) = b.get(j) {
        match c {
            b'\\' => j += 2,
            b'"' => return Some(j + 1),
            _ => j += 1,
        }
    }
    None
}

/// The index just past the value that starts at `i`.
fn value_end(b: &[u8], i: usize) -> Option<usize> {
    match *b.get(i)? {
        b'"' => string_end(b, i),
        b'{' | b'[' => {
            // Nested: count the brackets, and skip whole strings so that a `}`
            // or a `,` written inside one is never mistaken for structure.
            let mut depth = 0usize;
            let mut j = i;
            while let Some(&c) = b.get(j) {
                match c {
                    b'"' => {
                        j = string_end(b, j)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            None
        }
        _ => {
            // A number or a literal: it runs to the next delimiter.
            let mut j = i;
            while !matches!(b.get(j), None | Some(b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')) {
                j += 1;
            }
            let token = core::str::from_utf8(b.get(i..j)?).ok()?;
            let known = matches!(token, "true" | "false" | "null") || crate::num::number(token).is_some();
            // `j > i`: `Number("")` is zero, but a value that took no bytes at
            // all is not a value, and would leave a walk where it started.
            (j > i && known).then_some(j)
        }
    }
}

// ---- strings ------------------------------------------------------------------

/// A raw value as a `String`, or `None` when it is not a JSON string.
fn unquote(raw: &str) -> Option<String> {
    let b = raw.as_bytes();
    let closed = b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"';
    closed.then(|| unescape(raw.get(1..raw.len() - 1).unwrap_or("")))
}

/// Whether a key as written names `want`. Most keys carry no escape at all,
/// and those are compared without allocating anything.
fn same_key(written: &str, want: &str) -> bool {
    if written.as_bytes().contains(&b'\\') { unescape(written) == want } else { written == want }
}

/// The inside of a JSON string, with its escapes resolved. Anything the
/// grammar does not allow is kept as it was written rather than dropped: the
/// text is going on screen, not through a validator.
fn unescape(inner: &str) -> String {
    let b = inner.as_bytes();
    if !b.contains(&b'\\') {
        return inner.to_string();
    }
    let mut out = String::with_capacity(inner.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            // A run of plain text, copied whole: the backslash is ASCII, so
            // this slice always starts and ends on a character boundary.
            let start = i;
            while i < b.len() && b[i] != b'\\' {
                i += 1;
            }
            out.push_str(inner.get(start..i).unwrap_or(""));
            continue;
        }
        match b.get(i + 1) {
            Some(b'"') => out.push('"'),
            Some(b'\\') => out.push('\\'),
            Some(b'/') => out.push('/'),
            Some(b'b') => out.push('\u{8}'),
            Some(b'f') => out.push('\u{c}'),
            Some(b'n') => out.push('\n'),
            Some(b'r') => out.push('\r'),
            Some(b't') => out.push('\t'),
            Some(b'u') => {
                i += unicode(b, i, &mut out);
                continue;
            }
            // Not an escape the grammar knows, or a backslash at the very end.
            Some(&c) => out.push(c as char),
            None => {}
        }
        i += 2;
    }
    out
}

/// One `\uXXXX`, and the low half of a surrogate pair when one follows.
/// Returns how many bytes it consumed.
fn unicode(b: &[u8], i: usize, out: &mut String) -> usize {
    let Some(hi) = hex4(b, i + 2) else {
        // Not four hex digits: the text stays as it stands.
        out.push_str("\\u");
        return 2;
    };
    if !(0xd800..0xdc00).contains(&hi) {
        out.push(char::from_u32(hi).unwrap_or('\u{fffd}'));
        return 6;
    }
    // The high half of a pair; the escape that follows carries the low half.
    if b.get(i + 6) == Some(&b'\\')
        && b.get(i + 7) == Some(&b'u')
        && let Some(lo) = hex4(b, i + 8).filter(|lo| (0xdc00..0xe000).contains(lo))
    {
        let c = 0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00);
        out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
        return 12;
    }
    // A half on its own is not a character.
    out.push('\u{fffd}');
    6
}

fn hex4(b: &[u8], i: usize) -> Option<u32> {
    let mut value = 0u32;
    for k in 0..4 {
        value = value * 16 + (*b.get(i + k)? as char).to_digit(16)?;
    }
    Some(value)
}

fn hex(nibble: u32) -> char {
    char::from_digit(nibble, 16).unwrap_or('0')
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn reads_the_shapes_the_runtime_sees() {
        let body = r#"{"ok":true,"data":{"id":7},"redirect":null,"count":2.5}"#;
        let o = Object::new(body).unwrap();
        assert!(o.bool("ok"));
        assert!(!o.bool("redirect"));
        assert!(!o.bool("missing"));
        assert_eq!(o.raw("data"), Some(r#"{"id":7}"#));
        assert_eq!(o.raw("redirect"), Some("null"));
        assert_eq!(o.num("count"), Some(2.5));
        assert_eq!(o.num("data"), None);
        assert_eq!(o.str("data"), None);
    }

    #[test]
    fn nesting_and_structure_inside_strings() {
        let body = r#"{"a":{"b":[1,{"c":"}"}],"d":"x"},"e":"a,b:c{}[]","f":1}"#;
        let o = Object::new(body).unwrap();
        assert_eq!(o.raw("a"), Some(r#"{"b":[1,{"c":"}"}],"d":"x"}"#));
        assert_eq!(o.str("e"), Some("a,b:c{}[]".to_string()));
        assert_eq!(o.num("f"), Some(1.0));
    }

    #[test]
    fn escapes_come_out_as_text() {
        let o = Object::new(r#"{"m":"a\"b\\c\/d\be\ff\ng\rh\ti"}"#).unwrap();
        assert_eq!(o.str("m"), Some("a\"b\\c/d\u{8}e\u{c}f\ng\rh\ti".to_string()));
    }

    #[test]
    fn unicode_escapes_and_surrogate_pairs() {
        let o =
            Object::new(r#"{"a":"\u0041\u00e9","b":"\ud83d\ude00","c":"\ud800","d":"\u00","e":"\ud83dx"}"#).unwrap();
        assert_eq!(o.str("a"), Some("Aé".to_string()));
        assert_eq!(o.str("b"), Some("😀".to_string()));
        assert_eq!(o.str("c"), Some("\u{fffd}".to_string()));
        assert_eq!(o.str("d"), Some("\\u00".to_string()));
        assert_eq!(o.str("e"), Some("\u{fffd}x".to_string()));
    }

    #[test]
    fn keys_may_be_escaped_too() {
        let o = Object::new(r#"{"a\"b":1,"c\u0041":"x"}"#).unwrap();
        assert_eq!(o.num("a\"b"), Some(1.0));
        assert_eq!(o.str("cA"), Some("x".to_string()));
    }

    #[test]
    fn entries_takes_the_string_members() {
        let o = Object::new(r#"{"email":"Required","age":12,"name":"Too \"short\"","deep":{"x":"y"}}"#).unwrap();
        assert_eq!(
            o.entries(),
            vec![("email".to_string(), "Required".to_string()), ("name".to_string(), "Too \"short\"".to_string())]
        );
    }

    #[test]
    fn members_keeps_every_value_as_written() {
        let o = Object::new(r#"{"a":1,"b":[1,2],"c":"x"}"#).unwrap();
        assert_eq!(o.members(), vec![("a".to_string(), "1"), ("b".to_string(), "[1,2]"), ("c".to_string(), r#""x""#)]);
    }

    #[test]
    fn a_hostile_body_answers_nothing_rather_than_panicking() {
        for text in [
            "",
            "   ",
            "null",
            "[1,2]",
            "\"just a string\"",
            "{",
            "{\"a\"",
            "{\"a\":",
            "{\"a\":\"unclosed",
            "{\"a\":{\"b\":1",
            "{\"a\":tru}",
            "{a:1}",
            "{\"a\":1,,}",
            "{\"\\ud83d\":1}",
            "{\"a\":\"é\u{1f600}\"}",
            "not json at all",
        ] {
            let found = Object::new(text).map(|o| (o.bool("a"), o.raw("a"), o.str("a"), o.num("a"), o.entries()));
            // Whatever it answers, it answers: the point is that it returns.
            let _ = found;
        }
        assert_eq!(Object::new("[1,2]").map(|_| ()), None);
        assert_eq!(Object::new("not json").map(|_| ()), None);
        assert_eq!(Object::new("{\"a\":\"unclosed").unwrap().raw("a"), None);
        assert_eq!(Object::new("{\"a\":tru}").unwrap().raw("a"), None);
    }

    #[test]
    fn quoting_is_the_inverse_of_reading() {
        assert_eq!(quote("plain"), "\"plain\"");
        assert_eq!(quote("a\"b\\c\nd\te\rf"), r#""a\"b\\c\nd\te\rf""#);
        assert_eq!(quote("\u{1}\u{8}\u{c}"), r#""\u0001\b\f""#);
        assert_eq!(quote("é😀"), "\"é😀\"");
        let round = alloc::format!("{{\"m\":{}}}", quote("a\"b\nc\u{1}d"));
        assert_eq!(Object::new(&round).unwrap().str("m"), Some("a\"b\nc\u{1}d".to_string()));
    }

    #[test]
    fn a_value_as_the_text_it_shows_as() {
        assert_eq!(text(r#""a \"quoted\" word""#), "a \"quoted\" word");
        assert_eq!(text("12"), "12");
        assert_eq!(text("true"), "true");
        assert_eq!(text("null"), "");
        assert_eq!(text(r#"{"a":1}"#), "");
        assert_eq!(text("[1,2]"), "");
        assert_eq!(text(""), "");
    }

    #[test]
    fn object_or_array() {
        assert!(is_object("{}"));
        assert!(is_object("  [1] "));
        assert!(!is_object("\"x\""));
        assert!(!is_object("null"));
        assert!(!is_object(""));
    }

    #[test]
    fn parses_matches_what_json_parse_would_take() {
        for good in ["{}", "[]", " {\"a\":1} ", "null", "true", "-1.5e3", "\"text\"", "[1,2,3]"] {
            assert!(parses(good), "{good} should parse");
        }
        for bad in ["", "   ", "{\"a\":1} trailing", "{", "\"unclosed", "tru", "a b"] {
            assert!(!parses(bad), "{bad} should not parse");
        }
    }
}
