//! Cron expressions and next-fire computation, in UTC.
//!
//! Accepted forms:
//!
//! * five fields — `minute hour day-of-month month day-of-week`
//! * six fields — `second minute hour day-of-month month day-of-week`
//! * `@hourly`, `@daily` (`@midnight`), `@weekly`, `@monthly`, `@yearly` (`@annually`)
//! * `@every 90s` / `@every 5m` / `@every 2h` / `@every 1d` — aligned to the Unix
//!   epoch, so every instance of a deployment computes the same tick times
//!
//! A field is `*`, `?`, a value, a range `a-b`, a step `*/n`, `a/n` or `a-b/n`,
//! or a comma-separated list of those. Months accept `JAN`–`DEC`, days of the
//! week `SUN`–`SAT` (and `0` or `7` for Sunday). When both day-of-month and
//! day-of-week are restricted a day matches if **either** does, as in Vixie cron.

use std::fmt;
use std::str::FromStr;

/// A parse error, naming the field that is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronError(pub(crate) String);

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cron expression: {}", self.0)
    }
}

impl std::error::Error for CronError {}

/// A parsed cron expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cron {
    source: String,
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Fields(Fields),
    /// `@every`: fires whenever `unix_seconds % n == 0`.
    Every(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Fields {
    seconds: u64,
    minutes: u64,
    hours: u64,
    days: u64,
    months: u64,
    weekdays: u64,
    /// Day-of-month was not `*` / `?`.
    dom_restricted: bool,
    dow_restricted: bool,
}

const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];

/// How far ahead [`Cron::next_after`] searches before giving up (an
/// expression such as `0 0 30 2 *` never fires).
const SEARCH_YEARS: i64 = 9;

/// Longest accepted expression, in bytes. Real ones are a few dozen.
pub const MAX_EXPRESSION_LEN: usize = 256;

/// Times past 9999-12-31T23:59:59Z are out of range: [`Cron::next_after`]
/// answers `None` instead of overflowing.
const MAX_UNIX_SECS: u64 = 253_402_300_799;

impl Cron {
    /// Parse an expression. See the [module documentation](self) for the syntax.
    pub fn parse(expr: &str) -> Result<Cron, CronError> {
        let source = expr.trim().to_owned();
        if source.len() > MAX_EXPRESSION_LEN {
            return Err(CronError(format!("the expression is longer than {MAX_EXPRESSION_LEN} bytes")));
        }
        let kind = if let Some(rest) = source.strip_prefix('@') {
            parse_macro(rest)?
        } else {
            Kind::Fields(parse_fields(&source)?)
        };
        Ok(Cron { source, kind })
    }

    /// The expression as written.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Whether the expression fires at `unix_secs`.
    pub fn matches(&self, unix_secs: u64) -> bool {
        match &self.kind {
            Kind::Every(n) => unix_secs.is_multiple_of(*n),
            Kind::Fields(_) if unix_secs > MAX_UNIX_SECS => false,
            Kind::Fields(f) => {
                let t = Civil::from_unix(unix_secs as i64);
                bit(f.seconds, t.second) && bit(f.minutes, t.minute) && bit(f.hours, t.hour) && f.day_matches(&t)
            }
        }
    }

    /// The first fire time strictly after `unix_secs`, or `None` when the
    /// expression does not fire within the next nine years (or `unix_secs`
    /// is past the year 9999).
    pub fn next_after(&self, unix_secs: u64) -> Option<u64> {
        if unix_secs >= MAX_UNIX_SECS {
            return None;
        }
        let start = unix_secs + 1;
        let next = match &self.kind {
            Kind::Every(n) => start.div_ceil(*n).checked_mul(*n)?,
            Kind::Fields(f) => u64::try_from(f.next_from(i64::try_from(start).ok()?)?).ok()?,
        };
        // Strictly after and in range, whatever the arithmetic above did.
        (next > unix_secs && next <= MAX_UNIX_SECS).then_some(next)
    }

    /// The next `count` fire times after `unix_secs`.
    pub fn upcoming(&self, unix_secs: u64, count: usize) -> Vec<u64> {
        let mut out = Vec::with_capacity(count);
        let mut t = unix_secs;
        while out.len() < count {
            match self.next_after(t) {
                Some(n) => {
                    out.push(n);
                    t = n;
                }
                None => break,
            }
        }
        out
    }
}

impl FromStr for Cron {
    type Err = CronError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Cron::parse(s)
    }
}

impl fmt::Display for Cron {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}

fn parse_macro(rest: &str) -> Result<Kind, CronError> {
    let lower = rest.to_ascii_lowercase();
    let fields = match lower.as_str() {
        "hourly" => "0 0 * * * *",
        "daily" | "midnight" => "0 0 0 * * *",
        "weekly" => "0 0 0 * * 0",
        "monthly" => "0 0 0 1 * *",
        "yearly" | "annually" => "0 0 0 1 1 *",
        _ => {
            let Some(arg) = lower.strip_prefix("every") else {
                return Err(CronError(format!("unknown macro `@{rest}`")));
            };
            let secs = parse_duration_secs(arg.trim()).ok_or_else(|| {
                CronError(format!("`@{rest}`: expected a duration such as `30s`, `5m`, `2h` or `1d`"))
            })?;
            if secs == 0 {
                return Err(CronError("`@every` needs a duration of at least one second".to_owned()));
            }
            return Ok(Kind::Every(secs));
        }
    };
    parse_fields(fields).map(Kind::Fields)
}

/// `90s`, `5m`, `2h`, `1d`, or combinations such as `1h30m`.
pub(crate) fn parse_duration_secs(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    let mut total = 0u64;
    let mut digits = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let unit = match c {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86_400,
            _ => return None,
        };
        let n: u64 = digits.parse().ok()?;
        digits.clear();
        total = total.checked_add(n.checked_mul(unit)?)?;
    }
    digits.is_empty().then_some(total)
}

fn parse_fields(source: &str) -> Result<Fields, CronError> {
    let parts: Vec<&str> = source.split_whitespace().collect();
    let (sec, rest) = match parts.len() {
        5 => ("0", &parts[..]),
        6 => (parts[0], &parts[1..]),
        n => {
            return Err(CronError(format!(
                "expected 5 fields (minute hour day month weekday) or 6 (with seconds first), found {n}"
            )));
        }
    };
    let seconds = parse_field(sec, "second", 0, 59, &[])?;
    let minutes = parse_field(rest[0], "minute", 0, 59, &[])?;
    let hours = parse_field(rest[1], "hour", 0, 23, &[])?;
    let days = parse_field(rest[2], "day-of-month", 1, 31, &[])?;
    let months = parse_field(rest[3], "month", 1, 12, &MONTHS)?;
    // 7 is accepted as Sunday and folded onto bit 0.
    let mut weekdays = parse_field(rest[4], "day-of-week", 0, 7, &DAYS)?;
    if weekdays & (1 << 7) != 0 {
        weekdays = (weekdays | 1) & !(1 << 7);
    }
    let unrestricted = |f: &str| f == "*" || f == "?";
    Ok(Fields {
        seconds,
        minutes,
        hours,
        days,
        months,
        weekdays,
        dom_restricted: !unrestricted(rest[2]),
        dow_restricted: !unrestricted(rest[4]),
    })
}

fn parse_field(text: &str, what: &str, min: u32, max: u32, names: &[&str]) -> Result<u64, CronError> {
    let err = |msg: String| CronError(format!("{what} field `{text}`: {msg}"));
    let value = |token: &str| -> Result<u32, CronError> {
        if let Some(i) = names.iter().position(|n| n.eq_ignore_ascii_case(token)) {
            // Month names are 1-based, weekday names 0-based: both equal `min + i`.
            return Ok(min + i as u32);
        }
        let v: u32 = token.parse().map_err(|_| err(format!("`{token}` is not a number")))?;
        if v < min || v > max {
            return Err(err(format!("{v} is outside {min}-{max}")));
        }
        Ok(v)
    };
    let mut bits = 0u64;
    for part in text.split(',') {
        if part.is_empty() {
            return Err(err("empty list item".to_owned()));
        }
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => {
                let step: u32 = s.parse().map_err(|_| err(format!("step `{s}` is not a number")))?;
                if step == 0 {
                    return Err(err("step must be at least 1".to_owned()));
                }
                (r, Some(step))
            }
            None => (part, None),
        };
        let (lo, hi) = if range == "*" || range == "?" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            let (a, b) = (value(a)?, value(b)?);
            if a > b {
                return Err(err(format!("range {a}-{b} is backwards")));
            }
            (a, b)
        } else {
            let v = value(range)?;
            // `5/15` means "from 5 to the end, every 15".
            (v, if step.is_some() { max } else { v })
        };
        let mut v = lo;
        while v <= hi {
            bits |= 1 << v;
            // A step can be as large as `u32::MAX`.
            match v.checked_add(step.unwrap_or(1)) {
                Some(next) => v = next,
                None => break,
            }
        }
    }
    Ok(bits)
}

fn bit(set: u64, value: u32) -> bool {
    set & (1 << value) != 0
}

impl Fields {
    fn day_matches(&self, t: &Civil) -> bool {
        if !bit(self.months, t.month) {
            return false;
        }
        let dom = bit(self.days, t.day);
        let dow = bit(self.weekdays, t.weekday());
        match (self.dom_restricted, self.dow_restricted) {
            (true, true) => dom || dow,
            _ => dom && dow,
        }
    }

    /// First matching time at or after `start`.
    fn next_from(&self, start: i64) -> Option<i64> {
        let limit = start + SEARCH_YEARS * 366 * 86_400;
        let mut day = start.div_euclid(86_400);
        let mut from_sec = start.rem_euclid(86_400) as u32;
        while day * 86_400 <= limit {
            let civil = Civil::from_days(day);
            if !bit(self.months, civil.month) {
                // Skip to the first day of the next month.
                let (y, m) = if civil.month == 12 { (civil.year + 1, 1) } else { (civil.year, civil.month + 1) };
                day = days_from_civil(y, m, 1);
                from_sec = 0;
                continue;
            }
            if self.day_matches(&civil)
                && let Some(s) = self.next_in_day(from_sec)
            {
                return Some(day * 86_400 + i64::from(s));
            }
            day += 1;
            from_sec = 0;
        }
        None
    }

    /// First matching second-of-day at or after `from`.
    fn next_in_day(&self, from: u32) -> Option<u32> {
        let (h0, m0, s0) = (from / 3600, from / 60 % 60, from % 60);
        for h in h0..24 {
            if !bit(self.hours, h) {
                continue;
            }
            let m_start = if h == h0 { m0 } else { 0 };
            for m in m_start..60 {
                if !bit(self.minutes, m) {
                    continue;
                }
                let s_start = if h == h0 && m == m0 { s0 } else { 0 };
                if let Some(s) = (s_start..60).find(|s| bit(self.seconds, *s)) {
                    return Some(h * 3600 + m * 60 + s);
                }
            }
        }
        None
    }
}

/// A UTC calendar time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Civil {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    days: i64,
}

impl Civil {
    pub(crate) fn from_unix(secs: i64) -> Civil {
        let mut c = Civil::from_days(secs.div_euclid(86_400));
        let rem = secs.rem_euclid(86_400) as u32;
        c.hour = rem / 3600;
        c.minute = rem / 60 % 60;
        c.second = rem % 60;
        c
    }

    /// Howard Hinnant's `civil_from_days`.
    fn from_days(days: i64) -> Civil {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Civil { year, month, day, hour: 0, minute: 0, second: 0, days }
    }

    /// 0 = Sunday.
    fn weekday(&self) -> u32 {
        (self.days + 4).rem_euclid(7) as u32
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-09-19T12:30:00Z`.
pub(crate) fn format_utc(unix_secs: u64) -> String {
    let c = Civil::from_unix(unix_secs.min(MAX_UNIX_SECS) as i64);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", c.year, c.month, c.day, c.hour, c.minute, c.second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(y: i64, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> u64 {
        (days_from_civil(y, mo, d) * 86_400 + i64::from(h * 3600 + mi * 60 + s)) as u64
    }

    #[test]
    fn civil_round_trip() {
        assert_eq!(ts(1970, 1, 1, 0, 0, 0), 0);
        assert_eq!(ts(2000, 3, 1, 0, 0, 0), 951_868_800);
        let c = Civil::from_unix(1_790_000_000);
        assert_eq!(ts(c.year, c.month, c.day, c.hour, c.minute, c.second), 1_790_000_000);
        assert_eq!(format_utc(951_868_800), "2000-03-01T00:00:00Z");
        // 1970-01-01 was a Thursday.
        assert_eq!(Civil::from_unix(0).weekday(), 4);
        assert_eq!(Civil::from_unix(ts(2026, 9, 19, 0, 0, 0) as i64).weekday(), 6);
    }

    #[test]
    fn five_and_six_fields() {
        let five = Cron::parse("*/5 * * * *").unwrap();
        let six = Cron::parse("0 */5 * * * *").unwrap();
        let t = ts(2026, 9, 19, 12, 1, 30);
        assert_eq!(five.next_after(t), Some(ts(2026, 9, 19, 12, 5, 0)));
        assert_eq!(six.next_after(t), Some(ts(2026, 9, 19, 12, 5, 0)));
        // Strictly after.
        assert_eq!(six.next_after(ts(2026, 9, 19, 12, 5, 0)), Some(ts(2026, 9, 19, 12, 10, 0)));
    }

    #[test]
    fn every_second_and_lists() {
        let c = Cron::parse("* * * * * *").unwrap();
        assert_eq!(c.next_after(100), Some(101));
        let c = Cron::parse("15,45 30 9-17/4 * * *").unwrap();
        let t = ts(2026, 1, 1, 9, 30, 15);
        assert_eq!(
            c.upcoming(t, 3),
            vec![ts(2026, 1, 1, 9, 30, 45), ts(2026, 1, 1, 13, 30, 15), ts(2026, 1, 1, 13, 30, 45)]
        );
        assert_eq!(c.next_after(ts(2026, 1, 1, 17, 30, 45)), Some(ts(2026, 1, 2, 9, 30, 15)));
    }

    #[test]
    fn rollovers() {
        let c = Cron::parse("0 0 1 1 *").unwrap();
        assert_eq!(c.next_after(ts(2026, 1, 1, 0, 0, 0)), Some(ts(2027, 1, 1, 0, 0, 0)));
        let c = Cron::parse("59 23 31 * *").unwrap();
        assert_eq!(c.next_after(ts(2026, 1, 31, 23, 59, 0)), Some(ts(2026, 3, 31, 23, 59, 0)));
        // Leap day.
        let c = Cron::parse("0 12 29 2 *").unwrap();
        assert_eq!(c.next_after(ts(2026, 1, 1, 0, 0, 0)), Some(ts(2028, 2, 29, 12, 0, 0)));
        assert_eq!(c.next_after(ts(2028, 2, 29, 12, 0, 0)), Some(ts(2032, 2, 29, 12, 0, 0)));
        // Never fires.
        assert_eq!(Cron::parse("0 0 30 2 *").unwrap().next_after(0), None);
    }

    #[test]
    fn weekdays_and_names() {
        // 2026-09-19 is a Saturday.
        let c = Cron::parse("0 9 * * MON-FRI").unwrap();
        assert_eq!(c.next_after(ts(2026, 9, 19, 0, 0, 0)), Some(ts(2026, 9, 21, 9, 0, 0)));
        let sunday = Cron::parse("0 0 * * 7").unwrap();
        assert_eq!(sunday, Cron { source: "0 0 * * 7".into(), ..Cron::parse("0 0 * * 0").unwrap() });
        assert_eq!(sunday.next_after(ts(2026, 9, 19, 0, 0, 0)), Some(ts(2026, 9, 20, 0, 0, 0)));
        let c = Cron::parse("0 0 1 jan,JUL ?").unwrap();
        assert_eq!(c.next_after(ts(2026, 2, 1, 0, 0, 0)), Some(ts(2026, 7, 1, 0, 0, 0)));
    }

    #[test]
    fn day_of_month_or_day_of_week() {
        // The 13th, or any Friday.
        let c = Cron::parse("0 0 13 * FRI").unwrap();
        let t = ts(2026, 9, 1, 0, 0, 0);
        assert_eq!(
            c.upcoming(t, 4),
            vec![ts(2026, 9, 4, 0, 0, 0), ts(2026, 9, 11, 0, 0, 0), ts(2026, 9, 13, 0, 0, 0), ts(2026, 9, 18, 0, 0, 0)]
        );
    }

    #[test]
    fn macros() {
        assert_eq!(Cron::parse("@hourly").unwrap().next_after(ts(2026, 1, 1, 0, 30, 0)), Some(ts(2026, 1, 1, 1, 0, 0)));
        assert_eq!(Cron::parse("@daily").unwrap().next_after(ts(2026, 1, 1, 0, 30, 0)), Some(ts(2026, 1, 2, 0, 0, 0)));
        assert_eq!(
            Cron::parse("@weekly").unwrap().next_after(ts(2026, 9, 19, 0, 0, 0)),
            Some(ts(2026, 9, 20, 0, 0, 0))
        );
        assert_eq!(Cron::parse("@monthly").unwrap().next_after(ts(2026, 1, 1, 0, 0, 0)), Some(ts(2026, 2, 1, 0, 0, 0)));
        assert_eq!(Cron::parse("@yearly").unwrap().next_after(ts(2026, 1, 1, 0, 0, 0)), Some(ts(2027, 1, 1, 0, 0, 0)));
        let every = Cron::parse("@every 1h30m").unwrap();
        assert_eq!(every.next_after(5400 * 10), Some(5400 * 11));
        assert_eq!(every.next_after(5400 * 10 + 1), Some(5400 * 11));
        assert!(every.matches(5400 * 3) && !every.matches(5400 * 3 + 1));
    }

    #[test]
    fn matches_agrees_with_next_after() {
        let c = Cron::parse("30 */7 3,4 * * *").unwrap();
        let mut t = ts(2026, 5, 5, 0, 0, 0);
        for _ in 0..20 {
            let n = c.next_after(t).unwrap();
            assert!(c.matches(n));
            assert!((t + 1..n).step_by(17).all(|s| !c.matches(s)));
            t = n;
        }
    }

    #[test]
    fn errors() {
        for bad in [
            "",
            "* * * *",
            "* * * * * * *",
            "60 * * * *",
            "* 24 * * *",
            "* * 0 * *",
            "* * * 13 *",
            "* * * * 8",
            "*/0 * * * *",
            "5-1 * * * *",
            "a * * * *",
            "1,,2 * * * *",
            "@sometimes",
            "@every",
            "@every 0s",
            "@every 5x",
        ] {
            assert!(Cron::parse(bad).is_err(), "`{bad}` should not parse");
        }
        let e = Cron::parse("* 24 * * *").unwrap_err().to_string();
        assert!(e.contains("hour") && e.contains("24"), "{e}");
    }

    #[test]
    fn hostile_steps_and_lengths() {
        // A step near `u32::MAX` used to overflow the loop counter.
        let c = Cron::parse("5/4294967295 * * * *").unwrap();
        assert_eq!(c, Cron { source: "5/4294967295 * * * *".into(), ..Cron::parse("5 * * * *").unwrap() });
        let c = Cron::parse("*/4294967295 0-23/4294967295 * * *").unwrap();
        assert_eq!(c.next_after(ts(2026, 1, 1, 0, 0, 0)), Some(ts(2026, 1, 2, 0, 0, 0)));
        assert!(Cron::parse("*/4294967296 * * * *").is_err());
        assert!(Cron::parse("*/-1 * * * *").is_err());
        // Length cap.
        let long = format!("{} * * * *", vec!["1"; 400].join(","));
        assert!(Cron::parse(&long).unwrap_err().to_string().contains("longer"));
        assert!(Cron::parse(&format!("@every {}s", "9".repeat(300))).is_err());
        assert!(Cron::parse("@every 99999999999999999999s").is_err());
        assert!(Cron::parse("@every 18446744073709551615d").is_err());
    }

    #[test]
    fn extreme_times_do_not_overflow() {
        let huge = Cron::parse("@every 213503982334601d").unwrap();
        for c in [Cron::parse("* * * * * *").unwrap(), Cron::parse("@every 7s").unwrap(), huge] {
            for t in [u64::MAX, u64::MAX - 1, i64::MAX as u64, i64::MAX as u64 - 1, MAX_UNIX_SECS, 1 << 62] {
                assert_eq!(c.next_after(t), None, "{c} after {t}");
                let _ = c.matches(t);
                assert!(c.upcoming(t, 3).is_empty());
            }
            if let Some(n) = c.next_after(MAX_UNIX_SECS - 1) {
                assert_eq!(n, MAX_UNIX_SECS);
            }
        }
        assert_eq!(format_utc(u64::MAX), "9999-12-31T23:59:59Z");
    }

    /// xorshift64*: deterministic, no dependency.
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
        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[self.below(items.len())]
        }
    }

    fn fuzz_expression(rng: &mut Rng) -> String {
        const NUMBERS: [&str; 16] = [
            "0",
            "1",
            "5",
            "7",
            "12",
            "23",
            "24",
            "29",
            "30",
            "31",
            "59",
            "60",
            "4294967295",
            "4294967296",
            "18446744073709551616",
            "-1",
        ];
        const ATOMS: [&str; 12] = ["*", "?", "JAN", "dec", "SUN", "sat", "MON-FRI", "", " ", "\t", "x", "\u{e9}"];
        const GLUE: [&str; 6] = ["/", "-", ",", "*/", "-/", ",,"];
        const WHOLE: [&str; 10] = [
            "@hourly",
            "@daily",
            "@weekly",
            "@monthly",
            "@yearly",
            "@every ",
            "@every",
            "@",
            "@EVERY 1H",
            "@every 1h30m",
        ];
        const UNITS: [&str; 6] = ["s", "m", "h", "d", "x", ""];
        let mut out = String::new();
        match rng.below(8) {
            0 => {
                out.push_str(rng.pick(&WHOLE));
                for _ in 0..rng.below(4) {
                    out.push_str(rng.pick(&NUMBERS));
                    out.push_str(rng.pick(&UNITS));
                }
            }
            1 => {
                // Raw bytes, including multi-byte and control characters.
                for _ in 0..rng.below(40) {
                    out.push(char::from_u32(rng.next() as u32 % 0x250).unwrap_or('*'));
                }
            }
            _ => {
                let fields = [4, 5, 5, 5, 6, 6, 6, 7][rng.below(8)];
                for i in 0..fields {
                    if i > 0 {
                        out.push(' ');
                    }
                    for _ in 0..=rng.below(4) {
                        match rng.below(3) {
                            0 => out.push_str(rng.pick(&NUMBERS)),
                            1 => out.push_str(rng.pick(&ATOMS)),
                            _ => out.push_str(rng.pick(&GLUE)),
                        }
                    }
                }
            }
        }
        out
    }

    #[test]
    fn fuzz_parser_never_panics_and_always_moves_forward() {
        let started = std::time::Instant::now();
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let nows =
            [0, 59, ts(2026, 9, 19, 23, 59, 59), ts(2028, 2, 29, 0, 0, 0), MAX_UNIX_SECS - 1, MAX_UNIX_SECS, u64::MAX];
        let mut parsed = 0;
        for _ in 0..6_000 {
            let expr = fuzz_expression(&mut rng);
            let Ok(cron) = Cron::parse(&expr) else { continue };
            parsed += 1;
            assert_eq!(cron.as_str(), expr.trim());
            for now in nows.into_iter().chain([rng.next() % (MAX_UNIX_SECS / 4), rng.next()]) {
                match cron.next_after(now) {
                    Some(next) => {
                        assert!(next > now, "`{expr}`: {next} is not after {now}");
                        assert!(next <= MAX_UNIX_SECS, "`{expr}`: {next} is out of range");
                        assert!(cron.matches(next), "`{expr}` does not match its own tick {next}");
                    }
                    None => assert!(cron.upcoming(now, 2).is_empty()),
                }
            }
        }
        assert!(parsed > 200, "the generator should produce valid expressions too ({parsed})");
        assert!(started.elapsed() < std::time::Duration::from_secs(30), "took {:?}", started.elapsed());
    }
}
