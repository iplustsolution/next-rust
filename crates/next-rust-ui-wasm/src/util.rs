//! Small shared helpers: numbers the way a browser writes them, the icons
//! the kit builds at runtime, and the date arithmetic the calendar needs.

use alloc::borrow::ToOwned;
use alloc::string::String;

use next_rust_wasm::num::round;

/// Decimal places a CSS length is written to. A third of a device pixel is
/// already below what any display can show.
const PLACES: i64 = 1_000;

/// The largest magnitude written as a plain integer. Past this an `f64` has
/// no fractional part left anyway, and the `i64` arithmetic below would
/// overflow.
const HUGE: f64 = 9.0e15;

/// Append a signed integer.
///
/// This is `itoa` by hand because `write!`/`to_string` on an integer pulls
/// `core::fmt` in, and `core::fmt` is the single largest thing a module this
/// size can accidentally link — thousands of bytes to turn a number into
/// three characters.
pub fn push_int(out: &mut String, value: i64) {
    if value < 0 {
        out.push('-');
    }
    let mut digits = [0u8; 20];
    let mut at = 0;
    let mut left = value.unsigned_abs();
    loop {
        digits[at] = b'0' + (left % 10) as u8;
        left /= 10;
        at += 1;
        if left == 0 {
            break;
        }
    }
    while at > 0 {
        at -= 1;
        out.push(char::from(digits[at]));
    }
}

/// Append an integer left-padded with zeros to `width`.
pub fn push_padded(out: &mut String, value: i64, width: usize) {
    let mut digits = String::new();
    push_int(&mut digits, value.abs());
    if value < 0 {
        out.push('-');
    }
    for _ in digits.len()..width {
        out.push('0');
    }
    out.push_str(&digits);
}

/// A signed integer as a string.
pub fn int(value: i64) -> String {
    let mut out = String::new();
    push_int(&mut out, value);
    out
}

/// A number the way JavaScript stringifies it: no trailing zeros, no
/// exponent in the ranges the layout uses. `12.0` is `"12"`, not `"12.0"`,
/// because the value goes straight into a CSS length.
///
/// Done with integer arithmetic rather than `{}` on an `f64`: Rust's float
/// formatter is exact, which costs about 12 KB of Grisu and bignum code in
/// the module. Three decimals of a CSS pixel does not need any of it.
pub fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    if value >= HUGE || value <= -HUGE {
        // Nothing the layout produces reaches this, and an `f64` this large
        // is an integer already.
        return int(value as i64);
    }
    let scaled = round(value * PLACES as f64) as i64;
    let mut out = String::new();
    if scaled < 0 {
        out.push('-');
    }
    let whole = scaled.abs() / PLACES;
    let mut frac = scaled.abs() % PLACES;
    push_int(&mut out, whole);
    if frac == 0 {
        return out;
    }
    out.push('.');
    // Leading zeros of the fraction, then the digits with the trailing
    // zeros dropped: `0.5` is `.5`, not `.500`.
    let mut width = 3;
    while frac % 10 == 0 {
        frac /= 10;
        width -= 1;
    }
    push_padded(&mut out, frac, width);
    out
}

/// A number with exactly `places` decimals, the way `toFixed` writes one.
pub fn fixed(value: f64, places: u32) -> String {
    let scale = 10i64.pow(places);
    let scaled = round(value * scale as f64) as i64;
    let mut out = String::new();
    if scaled < 0 {
        out.push('-');
    }
    push_int(&mut out, scaled.abs() / scale);
    if places > 0 {
        out.push('.');
        push_padded(&mut out, scaled.abs() % scale, places as usize);
    }
    out
}

/// A CSS pixel length.
pub fn px(value: f64) -> String {
    let mut out = number(value);
    out.push_str("px");
    out
}

/// A CSS percentage.
pub fn percent(value: f64) -> String {
    let mut out = number(value);
    out.push('%');
    out
}

/// `parseFloat`: the leading number of a string, `0` when there is none.
///
/// Computed styles come back as `"16px"` or `"normal"`, a `min`/`max` as
/// `"10"`, and the carousel's step as `"-1"`; all of them end up in
/// arithmetic. Done by hand for the same reason as [`number`] — `str::parse`
/// for an `f64` is another four kilobytes of exact-decimal machinery, and
/// these are two-digit numbers.
pub fn parse_number(text: &str) -> f64 {
    let bytes = text.trim_start().as_bytes();
    let mut at = 0;
    let negative = match bytes.first() {
        Some(b'-') => {
            at = 1;
            true
        }
        Some(b'+') => {
            at = 1;
            false
        }
        _ => false,
    };
    let mut digits = 0i64;
    let mut seen = false;
    let mut scale = 0i32;
    while let Some(&b) = bytes.get(at) {
        if b.is_ascii_digit() {
            seen = true;
            // Digits past an `i64` change nothing a layout can see; keep the
            // magnitude rather than wrapping.
            if digits < i64::MAX / 16 {
                digits = digits * 10 + i64::from(b - b'0');
            } else {
                scale += 1;
            }
            at += 1;
        } else if b == b'.' && scale == 0 && !bytes[at..].is_empty() {
            at += 1;
            let start = at;
            while let Some(&b) = bytes.get(at) {
                if !b.is_ascii_digit() {
                    break;
                }
                seen = true;
                if digits < i64::MAX / 16 {
                    digits = digits * 10 + i64::from(b - b'0');
                    scale -= 1;
                }
                at += 1;
            }
            if at == start {
                break;
            }
            break;
        } else {
            break;
        }
    }
    if !seen {
        return 0.0;
    }
    let mut value = digits as f64;
    let mut left = scale;
    while left > 0 {
        value *= 10.0;
        left -= 1;
    }
    while left < 0 {
        value /= 10.0;
        left += 1;
    }
    if negative { -value } else { value }
}

/// Wrap icon path data in the kit's `<svg>`, exactly as the stylesheet
/// expects it. Only module constants are ever passed in.
pub fn svg(body: &str) -> String {
    const OPEN: &str = "<svg class=\"nr-icon\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" \
                        stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" \
                        aria-hidden=\"true\">";
    const CLOSE: &str = "</svg>";
    let mut out = String::with_capacity(OPEN.len() + body.len() + CLOSE.len());
    out.push_str(OPEN);
    out.push_str(body);
    out.push_str(CLOSE);
    out
}

/// `‹`.
pub const CHEVRON_LEFT: &str = "<path d=\"m15 18-6-6 6-6\"/>";
/// `›`.
pub const CHEVRON_RIGHT: &str = "<path d=\"m9 18 6-6-6-6\"/>";
/// `⌄`.
pub const CHEVRON_DOWN: &str = "<path d=\"m6 9 6 6 6-6\"/>";
/// `✕`.
pub const CROSS: &str = "<path d=\"M18 6 6 18\"/><path d=\"m6 6 12 12\"/>";

// ---------------------------------------------------------------- dates

/// A local calendar date. The kit never needs a time, and doing the
/// arithmetic here rather than in the host keeps the host's date surface to
/// "today" and `Intl` formatting.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Date {
    /// Year.
    pub year: i32,
    /// Month, `0` for January, as `Date.getMonth()` counts.
    pub month: i32,
    /// Day of the month, from `1`.
    pub day: i32,
}

/// Days in a month, honouring leap years.
pub fn days_in(year: i32, month: i32) -> i32 {
    let (year, month) = normalise(year, month);
    match month {
        0 | 2 | 4 | 6 | 7 | 9 | 11 => 31,
        3 | 5 | 8 | 10 => 30,
        _ if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        _ => 28,
    }
}

/// Carry a month outside `0..12` into the year.
fn normalise(year: i32, month: i32) -> (i32, i32) {
    (year + month.div_euclid(12), month.rem_euclid(12))
}

impl Date {
    /// A date, carrying an out-of-range month into the year the way
    /// `new Date(y, m, d)` does.
    pub fn new(year: i32, month: i32, day: i32) -> Date {
        let (year, month) = normalise(year, month);
        Date { year, month, day }
    }

    /// `YYYY-MM-DD`, the value a `<input type=date>` carries.
    pub fn iso(self) -> String {
        let mut out = String::with_capacity(10);
        push_padded(&mut out, i64::from(self.year), 4);
        out.push('-');
        push_padded(&mut out, i64::from(self.month) + 1, 2);
        out.push('-');
        push_padded(&mut out, i64::from(self.day), 2);
        out
    }

    /// Parse `YYYY-MM-DD`, rejecting a day the month does not have — which
    /// is what `new Date(y, m, d).getMonth() === m` checks in the script.
    pub fn parse(text: &str) -> Option<Date> {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        if !bytes.iter().enumerate().all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit()) {
            return None;
        }
        let digits =
            |from: usize, to: usize| -> i32 { bytes[from..to].iter().fold(0, |sum, b| sum * 10 + i32::from(b - b'0')) };
        let (year, month, day) = (digits(0, 4), digits(5, 7), digits(8, 10));
        if !(1..=12).contains(&month) || day < 1 || day > days_in(year, month - 1) {
            return None;
        }
        Some(Date { year, month: month - 1, day })
    }

    /// Days since the epoch, for ordering and for day arithmetic.
    pub fn ordinal(self) -> i32 {
        let (y, m) = (self.year, self.month);
        // Howard Hinnant's civil-from-days, inverted.
        let year = y - i32::from(m < 2);
        let era = if year >= 0 { year } else { year - 399 } / 400;
        let yoe = year - era * 400;
        let mp = (m + 10) % 12;
        let doy = (153 * mp + 2) / 5 + self.day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date that many days since the epoch.
    pub fn from_ordinal(days: i32) -> Date {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let year = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 2 } else { mp - 10 };
        Date { year: year + i32::from(month < 2), month, day }
    }

    /// This date `days` later.
    pub fn add_days(self, days: i32) -> Date {
        Date::from_ordinal(self.ordinal() + days)
    }

    /// This date `months` later, clamped to the shorter month the way
    /// the script's `addMonths` does (31 January plus a month is 28/29
    /// February, not 2/3 March).
    pub fn add_months(self, months: i32) -> Date {
        let (year, month) = normalise(self.year, self.month + months);
        Date { year, month, day: self.day.min(days_in(year, month)) }
    }

    /// The first of this month.
    pub fn month_start(self) -> Date {
        Date { day: 1, ..self }
    }

    /// The last day of this month.
    pub fn month_end(self) -> Date {
        Date { day: days_in(self.year, self.month), ..self }
    }

    /// Day of the week, `0` for Sunday, as `Date.getDay()` counts.
    pub fn weekday(self) -> i32 {
        (self.ordinal() + 4).rem_euclid(7)
    }

    /// How far into the week this date is, for a week starting on
    /// `first_day` (`0` Sunday, `1` Monday).
    pub fn offset_in_week(self, first_day: i32) -> i32 {
        (self.weekday() - first_day).rem_euclid(7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_the_way_a_browser_writes_them() {
        assert_eq!(number(12.0), "12");
        assert_eq!(number(-0.0), "0");
        assert_eq!(number(12.5), "12.5");
        assert_eq!(number(12.3456), "12.346");
        assert_eq!(number(f64::NAN), "0");
        assert_eq!(px(8.0), "8px");
        assert_eq!(percent(33.5), "33.5%");
    }

    #[test]
    fn a_length_reads_as_its_number() {
        assert_eq!(parse_number("16px"), 16.0);
        assert_eq!(parse_number(" 1.5rem"), 1.5);
        assert_eq!(parse_number("-4px"), -4.0);
        assert_eq!(parse_number("normal"), 0.0, "`column-gap: normal` is no gap");
        assert_eq!(parse_number(""), 0.0);
        assert_eq!(parse_number("0"), 0.0);
    }

    #[test]
    fn iso_dates_round_trip() {
        let d = Date::new(2026, 8, 20);
        assert_eq!(d.iso(), "2026-09-20");
        assert_eq!(Date::parse("2026-09-20"), Some(d));
        assert_eq!(Date::parse("2026-09-20").map(Date::iso).as_deref(), Some("2026-09-20"));
    }

    #[test]
    fn a_day_a_month_does_not_have_is_not_a_date() {
        assert_eq!(Date::parse("2026-02-30"), None);
        assert_eq!(Date::parse("2026-13-01"), None);
        assert_eq!(Date::parse("2026-00-10"), None);
        assert_eq!(Date::parse("not-a-date"), None);
        assert_eq!(Date::parse("2026-2-1"), None);
        assert_eq!(Date::parse(""), None);
        assert!(Date::parse("2024-02-29").is_some(), "2024 is a leap year");
        assert_eq!(Date::parse("2100-02-29"), None, "2100 is not");
    }

    #[test]
    fn days_carry_across_months_and_years() {
        assert_eq!(Date::new(2026, 0, 31).add_days(1), Date::new(2026, 1, 1));
        assert_eq!(Date::new(2026, 11, 31).add_days(1), Date::new(2027, 0, 1));
        assert_eq!(Date::new(2026, 0, 1).add_days(-1), Date::new(2025, 11, 31));
        for offset in [-400, -31, -1, 0, 1, 45, 400] {
            let d = Date::new(2026, 5, 15).add_days(offset);
            assert_eq!(d.add_days(-offset), Date::new(2026, 5, 15), "{offset}");
        }
    }

    #[test]
    fn months_clamp_to_the_shorter_month() {
        assert_eq!(Date::new(2026, 0, 31).add_months(1), Date::new(2026, 1, 28));
        assert_eq!(Date::new(2024, 0, 31).add_months(1), Date::new(2024, 1, 29));
        assert_eq!(Date::new(2026, 11, 15).add_months(1), Date::new(2027, 0, 15));
        assert_eq!(Date::new(2026, 0, 15).add_months(-1), Date::new(2025, 11, 15));
        assert_eq!(Date::new(2026, 0, 15).add_months(-13), Date::new(2024, 11, 15));
    }

    #[test]
    fn weekdays_match_the_calendar() {
        // 2026-09-20 is a Sunday; 1970-01-01 was a Thursday.
        assert_eq!(Date::new(2026, 8, 20).weekday(), 0);
        assert_eq!(Date::new(1970, 0, 1).weekday(), 4);
        assert_eq!(Date::new(2026, 8, 20).offset_in_week(1), 6, "a week starting on Monday ends on Sunday");
        assert_eq!(Date::new(2026, 8, 20).offset_in_week(0), 0);
    }

    #[test]
    fn month_edges_know_their_length() {
        assert_eq!(Date::new(2026, 1, 15).month_end().day, 28);
        assert_eq!(Date::new(2024, 1, 15).month_end().day, 29);
        assert_eq!(Date::new(2026, 8, 15).month_end().day, 30);
        assert_eq!(Date::new(2026, 8, 15).month_start().day, 1);
    }
}
