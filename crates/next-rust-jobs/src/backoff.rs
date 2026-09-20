//! Retry delays.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::util::{duration_ms, random_unit};

/// How long to wait before retry *n*: `base × factorⁿ⁻¹`, capped at `max`,
/// then randomised by `jitter`.
///
/// The default is 5 s, 10 s, 20 s … up to 1 h, each ±20 %.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Backoff {
    pub base_ms: u64,
    pub max_ms: u64,
    pub factor: f64,
    /// `0.2` spreads each delay uniformly over ±20 %; `0.0` turns jitter off.
    pub jitter: f64,
}

impl Default for Backoff {
    fn default() -> Self {
        Backoff { base_ms: 5_000, max_ms: 3_600_000, factor: 2.0, jitter: 0.2 }
    }
}

impl Backoff {
    /// Exponential backoff starting at `base`, doubling up to `max`, ±20 % jitter.
    pub fn exponential(base: Duration, max: Duration) -> Self {
        Backoff { base_ms: duration_ms(base), max_ms: duration_ms(max), ..Backoff::default() }
    }

    /// The same delay before every retry, without jitter.
    pub fn fixed(delay: Duration) -> Self {
        let ms = duration_ms(delay);
        Backoff { base_ms: ms, max_ms: ms, factor: 1.0, jitter: 0.0 }
    }

    pub fn factor(mut self, factor: f64) -> Self {
        self.factor = if factor.is_finite() { factor.max(1.0) } else { 2.0 };
        self
    }

    /// Clamped to `0.0..=1.0`.
    pub fn jitter(mut self, jitter: f64) -> Self {
        self.jitter = if jitter.is_finite() { jitter.clamp(0.0, 1.0) } else { 0.0 };
        self
    }

    /// The delay before the retry that follows failed attempt `attempt`
    /// (1-based), without jitter.
    pub fn base_delay(&self, attempt: u32) -> Duration {
        let exp = attempt.saturating_sub(1).min(64) as i32;
        let raw = self.base_ms as f64 * self.factor.max(1.0).powi(exp);
        let capped = if raw.is_finite() { raw.min(self.max_ms as f64) } else { self.max_ms as f64 };
        Duration::from_millis(capped.max(0.0) as u64)
    }

    /// [`base_delay`](Self::base_delay) with jitter applied; never above `max`
    /// unless `max` is below `base`.
    pub fn delay(&self, attempt: u32) -> Duration {
        self.delay_with(attempt, random_unit())
    }

    /// `unit` is a uniform sample in `[0, 1)`; exposed for deterministic tests.
    pub fn delay_with(&self, attempt: u32, unit: f64) -> Duration {
        let base = self.base_delay(attempt).as_millis() as f64;
        // A record read back from a backend may carry anything, NaN included.
        let jitter = if self.jitter.is_finite() { self.jitter.clamp(0.0, 1.0) } else { 0.0 };
        let spread = base * jitter * (unit.clamp(0.0, 1.0) * 2.0 - 1.0);
        Duration::from_millis((base + spread).max(0.0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_until_the_cap() {
        let b = Backoff::exponential(Duration::from_secs(1), Duration::from_secs(10)).jitter(0.0);
        let secs: Vec<u64> = (1..=6).map(|a| b.delay(a).as_secs()).collect();
        assert_eq!(secs, [1, 2, 4, 8, 10, 10]);
        // Attempt 0 is treated like attempt 1; huge attempts do not overflow.
        assert_eq!(b.base_delay(0), Duration::from_secs(1));
        assert_eq!(b.base_delay(u32::MAX), Duration::from_secs(10));
    }

    #[test]
    fn jitter_stays_in_range() {
        let b = Backoff::exponential(Duration::from_secs(10), Duration::from_secs(1000)).jitter(0.5);
        assert_eq!(b.delay_with(1, 0.0), Duration::from_secs(5));
        assert_eq!(b.delay_with(1, 0.5), Duration::from_secs(10));
        assert_eq!(b.delay_with(1, 1.0), Duration::from_secs(15));
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..200 {
            let d = b.delay(3).as_millis() as u64;
            assert!((20_000..=60_000).contains(&d), "{d}");
            seen.insert(d);
        }
        assert!(seen.len() > 20, "jitter should vary");
    }

    #[test]
    fn fixed_and_custom_factor() {
        let f = Backoff::fixed(Duration::from_millis(250));
        assert!((1..5).all(|a| f.delay(a) == Duration::from_millis(250)));
        let b = Backoff::exponential(Duration::from_secs(1), Duration::from_secs(100)).factor(3.0).jitter(0.0);
        assert_eq!(b.delay(3), Duration::from_secs(9));
        // A factor below 1 would shrink delays; it is clamped.
        assert_eq!(Backoff::default().factor(0.1).jitter(0.0).delay(4), Duration::from_secs(5));
        assert_eq!(Backoff::default().jitter(7.0).jitter, 1.0);
        // Fields are public and deserialised: nonsense must not collapse the delay to zero.
        let odd = Backoff { base_ms: 1000, max_ms: 1000, factor: f64::NAN, jitter: f64::NAN };
        assert_eq!(odd.delay(3), Duration::from_secs(1));
        let odd = Backoff { base_ms: u64::MAX, max_ms: u64::MAX, factor: f64::INFINITY, jitter: 1.0 };
        let _ = odd.delay(u32::MAX);
    }
}
