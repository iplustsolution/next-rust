//! Strings read out of the DOM.
//!
//! A [`Str`] owns its bytes in linear memory. Getting one costs an allocation
//! and a copy, which is why the API only hands them back where a string is
//! genuinely what you asked for — comparisons like [`crate::El::class_has`] or
//! `Event::key_is` are answered on the browser side instead, and never
//! allocate.

use alloc::string::String;
use core::ops::Deref;

/// An owned UTF-8 string that came from the browser.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Str(String);

impl Str {
    pub(crate) fn new(s: String) -> Self {
        Self(s)
    }

    /// An empty string, allocating nothing.
    pub const EMPTY: Self = Self(String::new());

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the string equals a literal. Cheaper to read than `*s == *"x"`
    /// and the shape islands actually want.
    pub fn eq_str(&self, other: &str) -> bool {
        self.0 == other
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.0.contains(needle)
    }

    pub fn trim(&self) -> &str {
        self.0.trim()
    }

    /// The integer this string spells, if it spells one exactly.
    ///
    /// Hand-written rather than `str::parse`, because `parse` drags in the
    /// error types and their formatting, which is measurable in a module this
    /// small. Leading `+`/`-`, ASCII digits, nothing else; overflow is `None`
    /// rather than a wrap.
    pub fn parse_i64(&self) -> Option<i64> {
        parse_i64(self.0.trim())
    }

    /// The number this string spells. Decimal notation only — no exponents,
    /// no hex, no infinities — because that is what a DOM value ever holds and
    /// a full float parser is kilobytes.
    pub fn parse_f64(&self) -> Option<f64> {
        parse_f64(self.0.trim())
    }

    /// Split on a byte, for the flat `\t`/`\n` encodings the host uses where
    /// JSON would otherwise be needed.
    pub fn split(&self, sep: char) -> impl Iterator<Item = &str> {
        self.0.split(sep)
    }
}

impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Str> for String {
    fn from(s: Str) -> String {
        s.0
    }
}

impl From<&str> for Str {
    fn from(s: &str) -> Str {
        Str(String::from(s))
    }
}

impl core::fmt::Debug for Str {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

pub(crate) fn parse_i64(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let (negative, digits) = match bytes.first()? {
        b'-' => (true, &bytes[1..]),
        b'+' => (false, &bytes[1..]),
        _ => (false, bytes),
    };
    if digits.is_empty() {
        return None;
    }
    let mut value: i64 = 0;
    for &byte in digits {
        let digit = byte.checked_sub(b'0').filter(|d| *d < 10)?;
        value = value.checked_mul(10)?.checked_add(i64::from(digit))?;
    }
    if negative { Some(-value) } else { Some(value) }
}

pub(crate) fn parse_f64(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    let (negative, rest) = match bytes.first()? {
        b'-' => (true, &bytes[1..]),
        b'+' => (false, &bytes[1..]),
        _ => (false, bytes),
    };
    let mut whole: f64 = 0.0;
    let mut seen = false;
    let mut at = 0;
    while at < rest.len() && rest[at].is_ascii_digit() {
        whole = whole * 10.0 + f64::from(rest[at] - b'0');
        seen = true;
        at += 1;
    }
    if at < rest.len() && rest[at] == b'.' {
        at += 1;
        let mut scale = 0.1;
        while at < rest.len() && rest[at].is_ascii_digit() {
            whole += f64::from(rest[at] - b'0') * scale;
            scale *= 0.1;
            seen = true;
            at += 1;
        }
    }
    if !seen || at != rest.len() {
        return None;
    }
    Some(if negative { -whole } else { whole })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn integers_parse_or_refuse() {
        assert_eq!(parse_i64("0"), Some(0));
        assert_eq!(parse_i64("42"), Some(42));
        assert_eq!(parse_i64("-42"), Some(-42));
        assert_eq!(parse_i64("+7"), Some(7));
        assert_eq!(parse_i64(&i64::MAX.to_string()), Some(i64::MAX));
        for bad in ["", "-", "+", " 1", "1 ", "1.0", "1e3", "abc", "1a", "99999999999999999999"] {
            assert_eq!(parse_i64(bad), None, "{bad:?}");
        }
        // i64::MIN cannot be reached by negating a positive that fits, which
        // is a documented edge rather than a silent wrap.
        assert_eq!(parse_i64("-9223372036854775808"), None);
    }

    #[test]
    fn decimals_parse_or_refuse() {
        assert_eq!(parse_f64("0"), Some(0.0));
        assert_eq!(parse_f64("12"), Some(12.0));
        assert_eq!(parse_f64("-0.5"), Some(-0.5));
        assert_eq!(parse_f64("3."), Some(3.0));
        assert!(parse_f64("0.1").is_some_and(|v| (v - 0.1).abs() < 1e-12));
        for bad in ["", ".", "-", "1e3", "abc", "1.2.3", "NaN", "Infinity"] {
            assert_eq!(parse_f64(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn str_is_a_thin_wrapper() {
        let s = Str::from("hello");
        assert_eq!(&*s, "hello");
        assert!(s.starts_with("he"));
        assert!(s.eq_str("hello"));
        assert!(!s.is_empty());
        assert_eq!(Str::EMPTY.len(), 0);
    }
}
