//! Float arithmetic `core` does not have, and JavaScript's number semantics.
//!
//! Two separate problems live here, and both bite island code immediately.
//!
//! The first is that most of `f64`'s inherent methods live in `std`, not
//! `core`. An island is `no_std`, so `abs`, `round`, `floor`, `ceil`, `trunc`,
//! `fract`, `rem_euclid`, `sqrt` and `powi` **do not exist** — code using them
//! does not compile, which is a confusing first encounter because they look
//! like language primitives. (`min`, `max`, `clamp`, `is_finite` and `is_nan`
//! *are* in `core`; use those directly.) Linking `libm` for a handful of call
//! sites would cost more than the island. The versions below use only `%` and comparison, which
//! `core` does provide, and are exact for every value a browser hands an
//! island: byte counts, prices, percentages, pixel offsets, millisecond
//! deltas.
//!
//! The second is that **Rust and JavaScript disagree about numbers in both
//! directions**, and an island that round-trips a value through the DOM needs
//! JavaScript's answer, not Rust's:
//!
//! * `str::parse::<f64>()` accepts `inf`, `infinity` and `NaN`, all of which
//!   `Number()` turns into NaN; and it rejects `0x10`, `0b101` and `0o17`,
//!   which `Number()` reads as 16, 5 and 15.
//! * `{}` on an `f64` prints all 309 digits of a large number, where
//!   `String(n)` switches to exponent notation past 1e21 and below 1e-6.
//!
//! Getting either wrong is a rendering difference nobody notices until a
//! specific value appears. [`finite_number`] and [`number_to_string`] are the
//! JavaScript rules, ported from a differential test against V8.
//!
//! # Never format or parse an `f64` with the standard library here
//!
//! This is the largest avoidable cost in an island and nothing about it looks
//! expensive. A single `str::parse::<f64>()` links `core::num`'s exact-decimal
//! path and its tables: **13 KB brotli**, measured. `{}` or `{:.1}` on an
//! `f64` costs about the same through `core::fmt::float`. One island crate
//! went from 44,656 to 31,399 bytes brotli — a third of it — by replacing
//! four `parse()` calls and two format strings.
//!
//! So [`finite_number`] parses by hand, and [`number_to_string`] formats
//! integers by hand and hands anything with a fractional part to the browser.
//! Integer formatting is a different and much smaller path, which is why
//! `to_string()` on an `i64` is fine.

/// `Math.floor`.
///
/// `x % 1.0` is the remainder with the sign of `x` and is exact, so `x` minus
/// it is the truncation; for a negative `x` with a fractional part, floor is
/// one below that.
pub fn floor(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let rest = x % 1.0;
    if rest < 0.0 { x - rest - 1.0 } else { x - rest }
}

/// `Math.ceil`.
pub fn ceil(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let rest = x % 1.0;
    if rest > 0.0 { x - rest + 1.0 } else { x - rest }
}

/// Toward zero — `Math.trunc`.
pub fn trunc(x: f64) -> f64 {
    if !x.is_finite() { x } else { x - x % 1.0 }
}

/// The fractional part, with the sign of `x`.
pub fn fract(x: f64) -> f64 {
    if !x.is_finite() { f64::NAN } else { x % 1.0 }
}

/// `Math.round` — half **up**, toward positive infinity.
///
/// Deliberately not `f64::round`'s "half away from zero": the two disagree on
/// negatives (`Math.round(-0.5)` is `-0`, `(-0.5f64).round()` is `-1`), and
/// this exists to match the browser.
pub fn round(x: f64) -> f64 {
    floor(x + 0.5)
}

pub fn abs(x: f64) -> f64 {
    if x < 0.0 { -x } else { x }
}

/// The remainder with the sign of the divisor, as `f64::rem_euclid` gives it.
pub fn rem_euclid(x: f64, m: f64) -> f64 {
    let r = x % m;
    if r < 0.0 { r + abs(m) } else { r }
}

/// `Number.isInteger`, for a value already known to be finite.
pub fn is_integer(x: f64) -> bool {
    x.is_finite() && x % 1.0 == 0.0
}

/// `Number(text)`, as far as a finite result goes.
///
/// `None` is JavaScript's `NaN`. Empty and all-whitespace are `0`, the way
/// `Number("")` is. Non-decimal literals take no sign, because `Number("-0x10")`
/// is NaN rather than -16.
pub fn finite_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    if let Some((radix, digits)) = radix_of(trimmed) {
        if digits.is_empty() {
            return None;
        }
        let mut value: f64 = 0.0;
        for byte in digits.bytes() {
            let digit = digit_of(byte, radix)?;
            value = value * f64::from(radix) + f64::from(digit);
        }
        return Some(value);
    }
    // Rust's parser takes `inf`, `infinity` and `NaN`; `Number()` does not,
    // and neither do we.
    if trimmed.bytes().any(|b| b.is_ascii_alphabetic() && !matches!(b, b'e' | b'E')) {
        return None;
    }
    crate::text::parse_f64(trimmed).filter(|v| v.is_finite())
}

/// `0x`, `0b` and `0o` literals, which `Number()` accepts and Rust does not.
fn radix_of(text: &str) -> Option<(u32, &str)> {
    let rest = text.strip_prefix('0')?;
    let mut chars = rest.chars();
    let radix = match chars.next()? {
        'x' | 'X' => 16,
        'b' | 'B' => 2,
        'o' | 'O' => 8,
        _ => return None,
    };
    Some((radix, &rest[1..]))
}

fn digit_of(byte: u8, radix: u32) -> Option<u32> {
    let value = match byte {
        b'0'..=b'9' => u32::from(byte - b'0'),
        b'a'..=b'f' => u32::from(byte - b'a') + 10,
        b'A'..=b'F' => u32::from(byte - b'A') + 10,
        _ => return None,
    };
    if value < radix { Some(value) } else { None }
}

/// `String(n)` for a finite number.
///
/// The integer fast path is bounded by 2^53, **not** by JavaScript's 1e21
/// exponent threshold. Bounding it at 1e21 looks right and is wrong: the
/// `as i64` cast saturates, so `1e20` would print as `9223372036854775807`.
/// That bug was found by a differential run against V8, not by reading.
pub fn number_to_string(value: f64) -> alloc::string::String {
    use alloc::string::ToString;
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if value == 0.0 {
        return "0".into();
    }
    if is_integer(value) && abs(value) < 9_007_199_254_740_992.0 {
        return (value as i64).to_string();
    }
    // Anything with a fractional part goes to the browser. Formatting an
    // `f64` here instead would link `core::fmt`'s float path, and that is not
    // a rounding error in the budget: it was measured at 13 KB brotli, larger
    // than most islands. The browser's `String(n)` is also, by definition,
    // exactly the semantics being matched.
    let (lo, hi) = crate::host::f(value);
    crate::host::take(crate::host::call(crate::host::op::NUM_TO_STR, lo, hi, 0, 0, 0)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_matches_the_browser_not_the_standard_library() {
        assert_eq!(round(0.5), 1.0);
        assert_eq!(round(2.5), 3.0);
        // Half up, not half away from zero: this is the disagreement.
        assert_eq!(round(-0.5), 0.0);
        assert_eq!(round(-1.5), -1.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(-0.6), -1.0);
        assert_eq!(round(1.0e15 + 0.5), 1.0e15 + 1.0);
        // And agrees with the standard library everywhere the rules coincide.
        for n in 0..2000 {
            let x = f64::from(n) / 7.0;
            assert_eq!(round(x), x.round(), "{x}");
        }
    }

    #[test]
    fn the_core_replacements_agree_with_the_standard_library() {
        for n in -2000..2000 {
            let x = f64::from(n) / 7.0;
            assert_eq!(floor(x), x.floor(), "floor {x}");
            assert_eq!(ceil(x), x.ceil(), "ceil {x}");
            assert_eq!(trunc(x), x.trunc(), "trunc {x}");
            assert_eq!(abs(x), x.abs(), "abs {x}");
            assert!((fract(x) - x.fract()).abs() < 1e-12, "fract {x}");
            assert!((rem_euclid(x, 3.0) - x.rem_euclid(3.0)).abs() < 1e-12, "rem_euclid {x}");
        }
        for x in [f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(floor(x), x);
            assert_eq!(ceil(x), x);
            assert_eq!(trunc(x), x);
        }
        assert!(fract(f64::NAN).is_nan());
    }

    #[test]
    fn finite_number_accepts_the_non_decimal_literals_number_does() {
        assert_eq!(finite_number("0x10"), Some(16.0));
        assert_eq!(finite_number("0b101"), Some(5.0));
        assert_eq!(finite_number("0o17"), Some(15.0));
        assert_eq!(finite_number("0X1f"), Some(31.0));
        // A sign in front of one is NaN, not a negative.
        assert_eq!(finite_number("-0x10"), None);
        // And the ordinary cases still work.
        assert_eq!(finite_number(""), Some(0.0));
        assert_eq!(finite_number("   "), Some(0.0));
        assert_eq!(finite_number(" 42 "), Some(42.0));
        assert_eq!(finite_number("-1.5"), Some(-1.5));
    }

    #[test]
    fn finite_number_rejects_what_number_rejects() {
        for bad in ["inf", "infinity", "Infinity", "NaN", "nan", "abc", "1abc", "0x", "0xg"] {
            assert_eq!(finite_number(bad), None, "{bad:?}");
        }
    }

    /// The cases answered without a host call. A value with a fractional part
    /// is the browser's `String(n)`, which is why there is no case for one
    /// here: off the browser it answers empty, and on it, it is correct by
    /// construction rather than by our arithmetic.
    #[test]
    fn number_to_string_handles_the_integer_cases_itself() {
        assert_eq!(number_to_string(0.0), "0");
        assert_eq!(number_to_string(42.0), "42");
        assert_eq!(number_to_string(-7.0), "-7");
        assert_eq!(number_to_string(9_007_199_254_740_991.0), "9007199254740991");
        assert_eq!(number_to_string(f64::NAN), "NaN");
        assert_eq!(number_to_string(f64::INFINITY), "Infinity");
        assert_eq!(number_to_string(f64::NEG_INFINITY), "-Infinity");
    }

    /// The bug a differential run against V8 caught: an integer fast path
    /// bounded by 1e21 rather than 2^53 saturates the `as i64` cast and
    /// prints `1e20` as `9223372036854775807`.
    ///
    /// The bound is 2^53 for that reason, so `1e20` — an integer, but past
    /// what an `i64` cast can be trusted with — goes to the browser like any
    /// other value the fast path cannot answer. What this asserts is the
    /// thing that must never come back: the saturated value.
    #[test]
    fn large_integers_do_not_saturate() {
        assert_ne!(number_to_string(1e20), "9223372036854775807");
        assert_eq!(number_to_string(9_007_199_254_740_991.0), "9007199254740991");
        assert_ne!(number_to_string(9_007_199_254_740_993.0), "9223372036854775807");
    }
}
