//! Formal route ranking.
//!
//! Every pattern segment has a weight:
//!
//! | segment         | example        | weight |
//! |-----------------|----------------|--------|
//! | static          | `settings`     | 3      |
//! | dynamic         | `[id]`         | 2      |
//! | catch-all       | `[...path]`    | 1      |
//! | optional c.-a.  | `[[...path]]`  | 0      |
//!
//! A route's *rank* is the sequence of its segment weights. Ranks are
//! compared **lexicographically, position by position, higher first**. When
//! one rank is a prefix of the other, the shorter pattern sorts first (it is
//! only reachable for shorter URLs, so the two never compete for the same
//! URL unless an optional catch-all is involved, which validation forbids).
//!
//! **Theorem (implemented by [`crate::Matcher`]).** For any URL path, the
//! matcher returns the matching route with the greatest rank. It does so
//! without comparing ranks at runtime: the trie tries children in weight order
//! (static, dynamic, catch-all, optional catch-all) and backtracks on failure,
//! so the first complete match found by the depth-first search is the
//! lexicographically greatest one. This property is verified by an
//! exhaustive randomized test against a brute-force implementation.
//!
//! Consequences:
//!
//! * `/users/settings` beats `/users/[id]` for `/users/settings`.
//! * `/users/[id]` beats `/users/[...path]` for `/users/42`.
//! * `/users/[...path]` still matches `/users/42/posts`.
//! * `/users/settings/[tab]` does **not** block `/users/[id]/edit` for
//!   `/users/settings/edit` if no static route matches the rest: backtracking
//!   falls through to the dynamic branch.

use std::cmp::Ordering;

use crate::segment::{PatternSegment, RoutePattern};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SegmentRank {
    OptionalCatchAll = 0,
    CatchAll = 1,
    Dynamic = 2,
    Static = 3,
}

pub fn segment_rank(seg: &PatternSegment) -> SegmentRank {
    match seg {
        PatternSegment::Static(_) => SegmentRank::Static,
        PatternSegment::Dynamic(_) => SegmentRank::Dynamic,
        PatternSegment::CatchAll(_) => SegmentRank::CatchAll,
        PatternSegment::OptionalCatchAll(_) => SegmentRank::OptionalCatchAll,
    }
}

pub fn rank_of(pattern: &RoutePattern) -> Vec<SegmentRank> {
    pattern.segments().iter().map(segment_rank).collect()
}

/// Ordering used for listings: higher precedence first, then shorter
/// patterns, then alphabetical static text for stable output.
pub fn compare_patterns(a: &RoutePattern, b: &RoutePattern) -> Ordering {
    for (x, y) in a.segments().iter().zip(b.segments()) {
        let ord = segment_rank(y).cmp(&segment_rank(x));
        if ord != Ordering::Equal {
            return ord;
        }
        if let (PatternSegment::Static(sx), PatternSegment::Static(sy)) = (x, y) {
            let ord = sx.cmp(sy);
            if ord != Ordering::Equal {
                return ord;
            }
        }
    }
    a.segments().len().cmp(&b.segments().len())
}

/// Precedence comparison for routes that match the *same* URL:
/// `Greater` means `a` wins.
pub fn precedence(a: &RoutePattern, b: &RoutePattern) -> Ordering {
    rank_of(a).cmp(&rank_of(b))
}
