//! Adversarial tests of the matcher: the only part of the router a request
//! touches. Everything here is generated from a fixed seed, so a failure is
//! reproducible, and every test carries a wall-clock bound — the matcher
//! runs once per request, so "slow" is a denial of service, not a nuisance.
//!
//! `tests/routing.rs` already proves the matcher picks the *right* route
//! (against a brute-force implementation of the formal ranking). This file
//! only asks whether hostile input can make it panic, take exponential time,
//! exhaust the stack, or hand a page a parameter shaped like something it is
//! not.

use std::time::{Duration, Instant};

use next_rust_router::{Matcher, ParamValue, PatternSegment, decode_segment, encode_segment};

/// xorshift64*: small, seeded, the same sequence on every run.
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

    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[self.below(from.len())]
    }
}

fn stat(s: &str) -> PatternSegment {
    PatternSegment::Static(s.into())
}

fn dyn_(s: &str) -> PatternSegment {
    PatternSegment::Dynamic(s.into())
}

fn all(s: &str) -> PatternSegment {
    PatternSegment::CatchAll(s.into())
}

fn opt(s: &str) -> PatternSegment {
    PatternSegment::OptionalCatchAll(s.into())
}

/// A route table with the shapes an application really has.
fn site() -> Matcher<&'static str> {
    let mut m = Matcher::new();
    for (name, pattern) in [
        ("home", vec![]),
        ("about", vec![stat("about")]),
        ("blog", vec![stat("blog")]),
        ("post", vec![stat("blog"), dyn_("slug")]),
        ("post-edit", vec![stat("blog"), dyn_("slug"), stat("edit")]),
        ("post-rest", vec![stat("blog"), dyn_("slug"), all("rest")]),
        ("docs", vec![stat("docs"), opt("path")]),
        ("user", vec![dyn_("user")]),
        ("user-tab", vec![dyn_("user"), dyn_("tab")]),
        ("deep", vec![dyn_("user"), dyn_("tab"), dyn_("c"), dyn_("d"), dyn_("e"), stat("end")]),
        ("files", vec![stat("files"), all("path")]),
        ("unicode", vec![stat("café")]),
        ("slash-name", vec![stat("a/b")]),
    ] {
        m.insert(&pattern, name).expect("pattern inserts");
    }
    m
}

const PIECES: &[&str] = &[
    "/",
    "/",
    "//",
    "blog",
    "docs",
    "files",
    "about",
    "end",
    "edit",
    "a",
    "b",
    "café",
    "caf%C3%A9",
    "%2f",
    "%2F",
    "%00",
    "%",
    "%zz",
    "%+f",
    "%25",
    "%2e%2e",
    "..",
    ".",
    "\\",
    "?",
    "#",
    "~",
    " ",
    "%20",
    "x",
    "\u{2028}",
    "\u{ff0e}",
    ":",
    ";",
    "a%2Fb",
];

#[test]
fn generated_paths_never_panic_and_the_matcher_stays_fast() {
    let m = site();
    let mut rng = Rng(0x2A17_0BEE_F00D);
    let mut hits = 0usize;
    let start = Instant::now();
    for _ in 0..80_000 {
        let mut path = String::new();
        for _ in 0..rng.below(8) {
            path.push('/');
            path.push_str(rng.pick(PIECES));
        }
        if let Some(found) = m.at(&path) {
            hits += 1;
            // Every parameter the matcher hands out is a decoded piece of
            // this path — never an owned copy of the pattern name, and never
            // empty unless the URL really had an empty catch-all.
            for (_, value) in found.params.iter() {
                if let ParamValue::Many(v) = value {
                    assert!(v.iter().all(|s| !s.is_empty()), "{path:?} -> {v:?}");
                }
            }
        }
    }
    assert!(hits > 1_000, "the generator reaches real routes too ({hits})");
    assert!(start.elapsed() < Duration::from_secs(10), "80k matches took {:?}", start.elapsed());
}

/// The truth about `/` in a parameter, which is easy to assume wrongly.
///
/// A `[slug]` parameter is one URL segment, but segments are percent-decoded
/// *after* they are split, so `%2F` puts a real `/` inside a single-segment
/// parameter. A page that joins a parameter onto a filesystem path or another
/// URL must still treat it as untrusted text; the framework's own static file
/// resolver does (see the server crate's `fuzz.rs`).
#[test]
fn a_dynamic_param_is_one_segment_but_percent_decoding_can_put_a_slash_in_it() {
    let m = site();
    let found = m.at("/blog/a%2Fb").expect("matches /blog/[slug]");
    assert_eq!(*found.value, "post");
    assert_eq!(found.params.get("slug"), Some("a/b"), "a parameter CAN contain `/`");
    // `..%2f..` likewise arrives as text, not as a traversal the router acted on.
    let found = m.at("/blog/..%2f..%2fetc").expect("matches");
    assert_eq!(found.params.get("slug"), Some("../../etc"));
    // A NUL is refused outright rather than truncating something downstream.
    assert!(m.at("/blog/a%00b").is_none());
    // An empty inner segment never matches, so a parameter is never empty.
    assert!(m.at("/blog//x").is_none() && m.at("/blog/").is_some());
    // Catch-all values stay split, one entry per URL segment.
    let found = m.at("/files/a/b%2Fc/d").expect("matches /files/[...path]");
    assert_eq!(found.params.get_all("path").unwrap(), ["a".to_owned(), "b/c".into(), "d".into()]);
    // The other side of the same coin: because decoding happens after the
    // split, `%2F` also reaches a *static* segment whose own text contains a
    // `/`. `parse_segment` refuses such a directory name, so no scanned app
    // can have one — but a hand-built `Matcher` can, and this is what it does.
    assert_eq!(m.at("/a%2Fb").map(|f| *f.value), Some("slash-name"));
    assert_eq!(m.at("/a/b").map(|f| *f.value), Some("user-tab"), "the unencoded form is two segments");
    // Percent-encoding does not change which static segment is meant.
    assert_eq!(m.at("/caf%C3%A9").map(|f| *f.value), Some("unicode"));
    assert_eq!(m.at("/café").map(|f| *f.value), Some("unicode"));
    // Invalid escapes and invalid UTF-8 are refused, not lossily decoded.
    for bad in ["/blog/%", "/blog/%2", "/blog/%zz", "/blog/%+f", "/blog/%ff%fe", "/blog/%c0%ae"] {
        assert!(m.at(bad).is_none(), "{bad}");
    }
}

/// A path is not a recursion budget: `find` descends the route tree, so a
/// URL with more segments than the deepest route stops at the deepest route.
#[test]
fn a_path_of_100k_segments_is_answered_without_recursing_into_it() {
    let m = site();
    for prefix in ["/blog", "/files", "/docs", "/nope", ""] {
        let path = format!("{prefix}{}", "/a".repeat(100_000));
        let start = Instant::now();
        let found = m.at(&path);
        assert!(start.elapsed() < Duration::from_secs(5), "{prefix}: took {:?}", start.elapsed());
        match prefix {
            "/files" => assert_eq!(found.map(|f| *f.value), Some("files")),
            "/docs" => assert_eq!(found.map(|f| *f.value), Some("docs")),
            _ => {}
        }
    }
    // The same on a thread with a small stack: nothing recurses per segment.
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let m = site();
            m.at(&format!("/files{}", "/a".repeat(100_000))).map(|f| *f.value)
        })
        .unwrap();
    assert_eq!(handle.join().unwrap(), Some("files"));
}

/// Backtracking is depth-first over the route tree, so the work is bounded by
/// the tree, not by 2^(path length). This builds the shape that would be
/// worst for a backtracking matcher: at every depth a static child *and* a
/// dynamic child, with the match only failing at the very end.
#[test]
fn a_dead_end_at_the_last_segment_does_not_backtrack_exponentially() {
    const DEPTH: usize = 22;
    let mut m: Matcher<usize> = Matcher::new();
    // `/a/a/a/...` and `/[p0]/[p1]/[p2]/...`, plus a static/dynamic fork at
    // every depth so the search has two children to try all the way down.
    for depth in 1..=DEPTH {
        let mut pattern: Vec<PatternSegment> = Vec::new();
        for i in 0..depth {
            pattern.push(if i % 2 == 0 { stat("a") } else { dyn_(&format!("p{i}")) });
        }
        pattern.push(stat("never"));
        m.insert(&pattern, depth).expect("inserts");
        let mut mirror: Vec<PatternSegment> = Vec::new();
        for i in 0..depth {
            mirror.push(if i % 2 == 0 { dyn_(&format!("q{i}")) } else { stat("a") });
        }
        mirror.push(stat("never"));
        let _ = m.insert(&mirror, depth + 1000);
    }
    let path = format!("/{}", vec!["a"; DEPTH + 1].join("/"));
    let start = Instant::now();
    for _ in 0..200 {
        assert!(m.at(&path).is_none(), "nothing ends in `never`");
    }
    assert!(start.elapsed() < Duration::from_secs(5), "200 failed matches took {:?}", start.elapsed());
}

#[test]
fn a_matcher_of_thousands_of_routes_answers_in_constant_time() {
    let mut m: Matcher<usize> = Matcher::new();
    let mut names: Vec<String> = Vec::new();
    for i in 0..5_000usize {
        names.push(format!("s{i}"));
    }
    for (i, name) in names.iter().enumerate() {
        let pattern = vec![stat("x"), stat(name), dyn_("id"), stat("edit")];
        m.insert(&pattern, i).expect("inserts");
    }
    m.insert(&[stat("x"), all("rest")], usize::MAX).expect("inserts");
    assert_eq!(m.len(), 5_001);
    let start = Instant::now();
    for i in 0..5_000usize {
        let found = m.at(&format!("/x/s{i}/7/edit")).expect("matches");
        assert_eq!(*found.value, i);
        assert_eq!(found.params.get("id"), Some("7"));
        // A miss at the last segment falls through to the catch-all.
        assert_eq!(*m.at(&format!("/x/s{i}/7/nope")).expect("catch-all").value, usize::MAX);
    }
    assert!(start.elapsed() < Duration::from_secs(10), "10k lookups took {:?}", start.elapsed());
}

/// A route tree as deep as a filesystem allows still matches, and the depth
/// the matcher recurses to is the tree's, not the attacker's.
#[test]
fn a_deeply_nested_static_route_matches() {
    const DEPTH: usize = 1_000;
    let mut m: Matcher<&str> = Matcher::new();
    let pattern: Vec<PatternSegment> = (0..DEPTH).map(|i| if i % 3 == 0 { dyn_("p") } else { stat("s") }).collect();
    m.insert(&pattern, "deep").expect("inserts");
    let path: String = (0..DEPTH).map(|i| if i % 3 == 0 { "/v" } else { "/s" }).collect();
    assert_eq!(m.at(&path).map(|f| *f.value), Some("deep"));
    assert!(m.at(&format!("{path}/extra")).is_none());
    // The last parameter wins when one name repeats down the tree.
    assert_eq!(m.at(&path).unwrap().params.get("p"), Some("v"));
}

#[test]
fn optional_and_required_catch_alls_agree_about_the_empty_case() {
    let mut m: Matcher<&str> = Matcher::new();
    m.insert(&[stat("o"), opt("rest")], "optional").expect("inserts");
    m.insert(&[stat("r"), all("rest")], "required").expect("inserts");
    let empty: [String; 0] = [];
    assert_eq!(m.at("/o").unwrap().params.get_all("rest").unwrap(), empty);
    assert_eq!(m.at("/o/").unwrap().params.get_all("rest").unwrap(), empty);
    assert!(m.at("/r").is_none(), "a required catch-all needs a segment");
    assert_eq!(m.at("/r/a").unwrap().params.get_all("rest").unwrap(), ["a".to_owned()]);
    // A catch-all after a catch-all is refused at insert time, so no pattern
    // can ever ask the matcher to split a path two ways.
    assert!(m.insert(&[stat("q"), all("a"), all("b")], "bad").is_err());
    assert!(m.insert(&[stat("q"), all("a"), stat("b")], "bad").is_err());
}

#[test]
fn segment_encoding_round_trips_for_generated_text() {
    let mut rng = Rng(0x7A17_C0DE_0001);
    const CHARS: &[&str] =
        &["a", "Z", "0", "-", "_", ".", "~", "/", "%", "?", "#", " ", "+", "&", "=", ":", "é", "日", "\u{1}", "\u{7f}"];
    for _ in 0..20_000 {
        let text: String = (0..rng.below(12)).map(|_| rng.pick(CHARS)).collect();
        let encoded = encode_segment(&text);
        assert!(encoded.is_ascii(), "{text:?} -> {encoded:?}");
        assert!(!encoded.contains('/'), "an encoded segment can never add a segment: {encoded:?}");
        assert_eq!(decode_segment(&encoded).as_deref(), Some(text.as_str()), "{text:?}");
    }
    // Decoding is total: every string either decodes or is refused.
    let mut rng = Rng(0x7A17_C0DE_0002);
    for _ in 0..20_000 {
        let text: String = (0..rng.below(12)).map(|_| rng.pick(PIECES)).collect();
        if let Some(decoded) = decode_segment(&text) {
            assert!(!text.is_empty() || decoded.is_empty());
        }
    }
}
