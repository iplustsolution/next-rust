//! Clock, identifiers and randomness.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    system_time_ms(SystemTime::now())
}

pub(crate) fn system_time_ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub(crate) fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// `text` cut to at most `max` bytes on a character boundary, with a marker
/// when something was dropped.
pub(crate) fn truncate(text: &str, max: usize) -> std::borrow::Cow<'_, str> {
    const MARK: &str = "… (truncated)";
    if text.len() <= max {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut end = max.saturating_sub(MARK.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    std::borrow::Cow::Owned(format!("{}{MARK}", &text[..end]))
}

pub(crate) fn random_u64() -> u64 {
    let mut buf = [0u8; 8];
    if getrandom::fill(&mut buf).is_err() {
        // No OS randomness: fall back to the clock. Only jitter and id
        // uniqueness depend on this, never security.
        let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
        return t ^ COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    }
    u64::from_le_bytes(buf)
}

/// A uniform value in `[0, 1)`.
pub(crate) fn random_unit() -> f64 {
    (random_u64() >> 11) as f64 / (1u64 << 53) as f64
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique id that sorts by creation time: 11 hex digits of milliseconds,
/// 4 of a process-wide counter, 12 random.
pub(crate) fn new_id() -> String {
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFFF;
    format!("{:011x}{:04x}{:012x}", now_ms(), seq, random_u64() & 0xFFFF_FFFF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_ordered() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 27);
        assert!(a[..11] <= b[..11]);
        let u = random_unit();
        assert!((0.0..1.0).contains(&u));
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        assert_eq!(truncate("short", 64), "short");
        let long = "é".repeat(100);
        for max in 0..70 {
            let cut = truncate(&long, max);
            assert!(cut.len() <= max.max("… (truncated)".len()), "{max}: {}", cut.len());
            assert!(cut.ends_with("(truncated)"));
        }
        assert_eq!(truncate(&"x".repeat(5000), 4096).len(), 4096);
    }
}
