//! Numbers, without `core`'s decimal conversion.
//!
//! `"1.5".parse::<f64>()` is correctly rounded, and it pays for that with
//! `core::num::dec2flt`: Lemire's fast path, a fallback big-decimal, and a
//! table of 128-bit powers of five. Measured in this module's wasm, that is
//! about **10 KB of lookup tables and 4 KB of code** — more than twice the
//! whole minified JavaScript runtime this crate replaces, to parse
//! `"120ms"`, `"0.3"` and `"250"`.
//!
//! So the runtime parses its own numbers. [`parse_prefix`] is JavaScript's
//! `parseFloat`: it reads the longest numeric prefix and ignores the rest,
//! which is exactly what `parseFloat(getComputedStyle(…))` does all over the
//! JavaScript runtime.
//!
//! **This is not correctly rounded.** An integer up to 2^53 and a decimal
//! with at most 15 significant digits and a small exponent come out exact;
//! beyond that the answer can be a unit or two in the last place off. Every
//! number the runtime reads is a CSS length, a millisecond count, a progress
//! fraction or a small JSON number, where that is invisible. Nothing here
//! should ever be used for money or for a value that is compared for
//! equality against one a server produced.

use alloc::string::String;

/// Powers of ten that are exactly representable, so one multiplication is
/// exact for the common case.
const POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20,
    1e21, 1e22,
];

/// `value * 10^exp`, exactly when it can be.
fn scale(value: f64, exp: i32) -> f64 {
    if exp == 0 {
        return value;
    }
    if (0..=22).contains(&exp) {
        return value * POW10[exp as usize];
    }
    if (-22..0).contains(&exp) {
        return value / POW10[(-exp) as usize];
    }
    // Far from zero: step in exact chunks rather than looping one power at a
    // time, which would accumulate error.
    let mut out = value;
    let mut left = exp;
    while left > 22 {
        out *= POW10[22];
        left -= 22;
        if !out.is_finite() {
            return out;
        }
    }
    while left < -22 {
        out /= POW10[22];
        left += 22;
        if out == 0.0 {
            return out;
        }
    }
    scale(out, left)
}

/// JavaScript's `parseFloat`: the longest numeric prefix of `text`, and how
/// many bytes it used. Leading whitespace is skipped; anything that cannot
/// continue a number ends it. `None` when there is no number at all.
///
/// `Infinity` is recognised, because `parseFloat` recognises it and a
/// computed style can hold it; `NaN` is not, because `parseFloat` does not.
pub fn parse_prefix(text: &str) -> Option<(f64, usize)> {
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() && (bytes[at] == b' ' || bytes[at].is_ascii_whitespace()) {
        at += 1;
    }
    let negative = match bytes.get(at) {
        Some(b'-') => {
            at += 1;
            true
        }
        Some(b'+') => {
            at += 1;
            false
        }
        _ => false,
    };
    const INFINITY: &str = "Infinity";
    if text.get(at..at + INFINITY.len()) == Some(INFINITY) {
        let value = if negative { f64::NEG_INFINITY } else { f64::INFINITY };
        return Some((value, at + INFINITY.len()));
    }
    // The mantissa as an integer, with how far the decimal point moved.
    let mut mantissa: u64 = 0;
    let mut digits = 0u32;
    let mut exponent = 0i32;
    let mut seen = false;
    while let Some(&byte) = bytes.get(at) {
        if !byte.is_ascii_digit() {
            break;
        }
        seen = true;
        at += 1;
        // Past 19 digits the mantissa would overflow; the rest only shift it.
        if digits < 19 {
            mantissa = mantissa * 10 + u64::from(byte - b'0');
            digits += 1;
        } else {
            exponent += 1;
        }
    }
    if bytes.get(at) == Some(&b'.') {
        let point = at;
        at += 1;
        let mut any = false;
        while let Some(&byte) = bytes.get(at) {
            if !byte.is_ascii_digit() {
                break;
            }
            any = true;
            at += 1;
            if digits < 19 {
                mantissa = mantissa * 10 + u64::from(byte - b'0');
                digits += 1;
                exponent -= 1;
            }
        }
        // `"5."` is a number in JavaScript; `"."` on its own is not.
        if !any && !seen {
            return None;
        }
        seen = seen || any;
        let _ = point;
    }
    if !seen {
        return None;
    }
    let mantissa_end = at;
    // An exponent only counts when digits follow it: `parseFloat("1e")` is 1.
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        let mut probe = at + 1;
        let sign = match bytes.get(probe) {
            Some(b'-') => {
                probe += 1;
                -1
            }
            Some(b'+') => {
                probe += 1;
                1
            }
            _ => 1,
        };
        let start = probe;
        let mut power: i32 = 0;
        while let Some(&byte) = bytes.get(probe) {
            if !byte.is_ascii_digit() {
                break;
            }
            // A wild exponent saturates rather than wrapping into nonsense.
            power = power.saturating_mul(10).saturating_add(i32::from(byte - b'0'));
            probe += 1;
        }
        if probe > start {
            exponent = exponent.saturating_add(sign * power);
            at = probe;
        } else {
            at = mantissa_end;
        }
    }
    let value = scale(mantissa as f64, exponent);
    Some((if negative { -value } else { value }, at))
}

/// JavaScript's `parseFloat`, without the length.
pub fn parse(text: &str) -> Option<f64> {
    parse_prefix(text).map(|(value, _)| value)
}

/// A whole string as a number, the way `Number(text)` does: surrounding
/// whitespace is allowed, trailing rubbish is not, and `""` is zero.
pub fn number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    match parse_prefix(trimmed) {
        Some((value, used)) if used == trimmed.len() => Some(value),
        _ => None,
    }
}

/// An unsigned integer as decimal, without `core::fmt`.
pub fn u32_to_string(mut value: u32) -> String {
    let mut digits = [0u8; 10];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let mut out = String::with_capacity(digits.len() - at);
    for &digit in &digits[at..] {
        out.push(digit as char);
    }
    out
}

/// A number as decimal with at most `places` digits after the point and no
/// trailing zeros, without `core::fmt`'s float formatting.
///
/// This is what `--nr-load` and the loading bar's saved progress are written
/// with. JavaScript would print `8.000000000000007%`; this prints `8%`.
pub fn to_string(value: f64, places: u32) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }
    let negative = value < 0.0;
    let mut value = if negative { -value } else { value };
    if value > 1e18 {
        // Nothing the runtime formats is ever this large; keep it finite
        // rather than spinning.
        value = 1e18;
    }
    let scale = POW10[places.min(22) as usize];
    let scaled = value * scale;
    // Round half away from zero, as `toFixed` does.
    let rounded = (scaled + 0.5) as u64;
    let whole = rounded / (scale as u64);
    let mut fraction = rounded % (scale as u64);

    let mut out = String::new();
    if negative && (whole != 0 || fraction != 0) {
        out.push('-');
    }
    out.push_str(&u64_to_string(whole));
    if fraction == 0 {
        return out;
    }
    // The fraction is printed with leading zeros to `places`, then trimmed.
    let mut digits = [b'0'; 22];
    let mut at = places.min(22) as usize;
    while fraction > 0 && at > 0 {
        at -= 1;
        digits[at] = b'0' + (fraction % 10) as u8;
        fraction /= 10;
    }
    let text = &digits[..places.min(22) as usize];
    let end = text.iter().rposition(|&d| d != b'0').map_or(0, |i| i + 1);
    if end == 0 {
        return out;
    }
    out.push('.');
    for &digit in &text[..end] {
        out.push(digit as char);
    }
    out
}

fn u64_to_string(mut value: u64) -> String {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let mut out = String::with_capacity(digits.len() - at);
    for &digit in &digits[at..] {
        out.push(digit as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_prefix_is_javascripts_parsefloat() {
        // The shapes the runtime actually reads out of computed styles.
        assert_eq!(parse("120ms"), Some(120.0));
        assert_eq!(parse("0.3"), Some(0.3));
        assert_eq!(parse(".5s"), Some(0.5));
        assert_eq!(parse("  200  "), Some(200.0));
        assert_eq!(parse("-1.25em"), Some(-1.25));
        assert_eq!(parse("+7"), Some(7.0));
        assert_eq!(parse("1e3x"), Some(1000.0));
        assert_eq!(parse("1e-3"), Some(0.001));
        assert_eq!(parse("5."), Some(5.0));
        // Nothing to read.
        assert_eq!(parse("none"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("  "), None);
        assert_eq!(parse("."), None);
        assert_eq!(parse("e5"), None);
        assert_eq!(parse("-"), None);
        // `parseFloat` knows this one word, and a computed style can hold it.
        assert_eq!(parse("Infinity"), Some(f64::INFINITY));
        assert_eq!(parse("-Infinity"), Some(f64::NEG_INFINITY));
        assert_eq!(parse_prefix("Infinity!"), Some((f64::INFINITY, 8)));
        assert_eq!(parse("Inf"), None);
        assert_eq!(parse("NaN"), None);
        // `parseFloat("1e")` is 1: the exponent is only part of the number
        // when digits follow it.
        assert_eq!(parse_prefix("1e"), Some((1.0, 1)));
        assert_eq!(parse_prefix("1e+"), Some((1.0, 1)));
        // How much was consumed, which `parseFloat` callers rely on.
        assert_eq!(parse_prefix("120ms"), Some((120.0, 3)));
        assert_eq!(parse_prefix(" 1.5rem"), Some((1.5, 4)));
    }

    /// Everything the runtime parses has to come out bit-for-bit identical to
    /// what `str::parse` would have produced, or the port changes behaviour.
    #[test]
    fn small_decimals_are_exact() {
        for text in [
            "0",
            "1",
            "250",
            "400",
            "30000",
            "0.95",
            "0.435",
            "0.5",
            "1.5",
            "2.25",
            "600",
            "6000",
            "100",
            "16",
            "1e3",
            "1e-3",
            "1.0e2",
            "0.001",
            "9007199254740992",
            "123456789.125",
        ] {
            let ours = parse(text).expect(text);
            let theirs: f64 = text.parse().expect(text);
            assert_eq!(ours.to_bits(), theirs.to_bits(), "{text}: {ours} != {theirs}");
        }
    }

    /// Where it is not exact, it must still be within a couple of ulp.
    #[test]
    fn long_decimals_stay_close() {
        for text in ["3.141592653589793", "2.718281828459045", "0.1234567890123456789", "1.7976931348623157e300"] {
            let ours = parse(text).expect(text);
            let theirs: f64 = text.parse().expect(text);
            let error = ((ours - theirs) / theirs).abs();
            assert!(error < 1e-15, "{text}: {ours} vs {theirs} (relative error {error:e})");
        }
    }

    #[test]
    fn number_rejects_trailing_rubbish() {
        assert_eq!(number("250"), Some(250.0));
        assert_eq!(number(" 250 "), Some(250.0));
        assert_eq!(number(""), Some(0.0));
        assert_eq!(number("   "), Some(0.0));
        assert_eq!(number("250ms"), None);
        assert_eq!(number("abc"), None);
    }

    #[test]
    fn integers_and_fractions_print_without_core_fmt() {
        assert_eq!(u32_to_string(0), "0");
        assert_eq!(u32_to_string(7), "7");
        assert_eq!(u32_to_string(4_294_967_295), "4294967295");
        assert_eq!(to_string(0.0, 6), "0");
        assert_eq!(to_string(1.0, 6), "1");
        assert_eq!(to_string(0.5, 6), "0.5");
        assert_eq!(to_string(12.25, 6), "12.25");
        assert_eq!(to_string(100.0, 6), "100");
        // The case the loading bar hits: 0.08 * 100 is not exactly 8.
        assert_eq!(to_string(0.08 * 100.0, 6), "8");
        assert_eq!(to_string(-1.5, 6), "-1.5");
        // Rounding away, not truncating.
        assert_eq!(to_string(0.9999999, 6), "1");
        assert_eq!(to_string(1.0 / 3.0, 4), "0.3333");
    }
}
