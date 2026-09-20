//! Path patterns for `[[redirects]]` and `[[headers]]` configuration rules.
//!
//! Syntax: `/blog/:slug` captures one segment, `/docs/:path*` captures the
//! rest (zero or more segments). Destinations may reference captures.

fn capture(pattern: &str, path: &str) -> Option<Vec<(String, String)>> {
    let pat: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut caps = Vec::new();
    for (i, p) in pat.iter().enumerate() {
        if let Some(name) = p.strip_prefix(':') {
            if let Some(name) = name.strip_suffix('*') {
                caps.push((name.to_owned(), segs.get(i..).map(|s| s.join("/")).unwrap_or_default()));
                return Some(caps);
            }
            caps.push((name.to_owned(), (*segs.get(i)?).to_owned()));
        } else if segs.get(i) != Some(p) {
            return None;
        }
    }
    (segs.len() == pat.len()).then_some(caps)
}

pub(crate) fn matches(pattern: &str, path: &str) -> bool {
    capture(pattern, path).is_some()
}

/// Apply a redirect rule; returns the destination when `path` matches.
pub(crate) fn apply(source: &str, destination: &str, path: &str) -> Option<String> {
    let caps = capture(source, path)?;
    let mut dest = destination.to_owned();
    // Replace longer names first so `:slug` does not clobber `:slugs`.
    let mut sorted = caps;
    sorted.sort_by_key(|c| std::cmp::Reverse(c.0.len()));
    for (name, value) in sorted {
        let value = encode_capture(&value);
        dest = dest.replace(&format!(":{name}*"), &value).replace(&format!(":{name}"), &value);
    }
    Some(dest)
}

/// Percent-encode the characters a capture must never introduce into the
/// destination, because the destination becomes a `Location` header.
///
/// A capture is a piece of the request path, so it is attacker-controlled.
/// `\` is the dangerous one: browsers read it as `/`, so a rule with
/// `source = "/go/:to"` and `destination = "/:to"` would turn a request for
/// `/go/\evil.example` into `Location: /\evil.example`, which every major
/// browser resolves as `//evil.example` — an open redirect out of a rule that
/// only ever meant to move a path. `?` and `#` would likewise cut the rest of
/// the destination off, and controls/space would change how the header is
/// read. `/` is kept: a `:path*` capture is several segments by definition,
/// and `%` is kept so an already-encoded path segment is not double-encoded.
fn encode_capture(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' | '?' | '#' => out.push_str(&format!("%{:02X}", c as u32)),
            c if (c as u32) <= 0x20 || c as u32 == 0x7f => out.push_str(&format!("%{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert_eq!(apply("/old/:slug", "/new/:slug", "/old/hello").as_deref(), Some("/new/hello"));
        assert_eq!(apply("/docs/:path*", "/guide/:path*", "/docs/a/b").as_deref(), Some("/guide/a/b"));
        assert_eq!(apply("/docs/:path*", "/guide", "/docs").as_deref(), Some("/guide"));
        assert!(apply("/old/:slug", "/new", "/old/a/b").is_none());
        assert!(matches("/api/:x*", "/api/v1/users"));
        assert!(!matches("/about", "/contact"));
    }

    /// xorshift64*: seeded, so a failure is reproducible.
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

    /// The one case that mattered: a capture taken from the request path,
    /// substituted into a destination that becomes a `Location` header.
    /// Red before the fix — `Location: /\evil.example` is `//evil.example`
    /// in Chrome, Firefox and Safari.
    #[test]
    fn a_capture_cannot_turn_a_local_destination_into_another_origin() {
        assert_eq!(apply("/go/:to", "/:to", "/go/\\evil.example").as_deref(), Some("/%5Cevil.example"));
        assert_eq!(apply("/go/:to*", "/:to*", "/go/\\\\evil.example/x").as_deref(), Some("/%5C%5Cevil.example/x"));
        // `?` and `#` would cut off the rest of the configured destination.
        assert_eq!(apply("/go/:to", "/read/:to/end", "/go/a?b").as_deref(), Some("/read/a%3Fb/end"));
        assert_eq!(apply("/go/:to", "/read/:to/end", "/go/a#b").as_deref(), Some("/read/a%23b/end"));
        // Ordinary captures, including already-encoded ones, are untouched.
        assert_eq!(apply("/go/:to", "/:to", "/go/a%20b").as_deref(), Some("/a%20b"));
        assert_eq!(apply("/go/:to", "/:to", "/go/héllo").as_deref(), Some("/héllo"));
        assert_eq!(apply("/o/:s", "https://cdn.example/:s", "/o/x").as_deref(), Some("https://cdn.example/x"));
    }

    const PATH_PIECES: &[&str] = &[
        "/",
        "/",
        "//",
        "\\",
        "\\\\",
        "..",
        ".",
        "%2e%2e",
        "%2f",
        "%5c",
        "%00",
        "evil.example",
        "a",
        "go",
        "docs",
        ":to",
        ":to*",
        "\t",
        "\r\n",
        " ",
        "\u{7f}",
        "\u{1}",
        "http:",
        "javascript:",
        "x?y",
        "x#y",
        "%",
        "héllo",
        "",
        "@",
        "localhost",
        "?",
        "#",
    ];

    const RULES: &[(&str, &str)] = &[
        ("/go/:to", "/:to"),
        ("/go/:to", "/read/:to"),
        ("/go/:to*", "/:to*"),
        ("/:a/:b", "/:b/:a"),
        ("/docs/:path*", "/guide/:path*"),
        ("/go/:to", ":to"),
        ("/go/:to", "/go?next=:to"),
        ("/go/:to*", "https://cdn.example/:to*"),
        ("/:a", "/:a/"),
        ("/go/:to", "/:to:to"),
    ];

    /// Generated hostile paths against generated rules: whenever the
    /// configured destination is a path on this site, so is the result.
    #[test]
    fn generated_paths_never_leave_a_local_destination_local() {
        let mut rng = Rng(0x0D15_EA5E_C0DE);
        let mut matched = 0usize;
        for _ in 0..40_000 {
            let mut path = String::new();
            for _ in 0..rng.below(6) {
                path.push('/');
                path.push_str(rng.pick(PATH_PIECES));
            }
            let (source, destination) = RULES[rng.below(RULES.len())];
            let Some(dest) = apply(source, destination, &path) else { continue };
            matched += 1;
            if destination.starts_with('/') && !destination.starts_with("//") {
                assert!(
                    crate::response::is_local_path(&dest),
                    "rule {source:?} -> {destination:?} on {path:?} produced {dest:?}"
                );
            }
            // And never a header break.
            assert!(!dest.contains('\r') && !dest.contains('\n'), "{path:?} -> {dest:?}");
        }
        assert!(matched > 500, "the generator produces matching paths too ({matched})");
    }

    #[test]
    fn matching_is_linear_in_the_path_even_for_pathological_input() {
        let long = format!("/go/{}", "a/".repeat(50_000));
        let start = std::time::Instant::now();
        for _ in 0..20 {
            let _ = apply("/go/:to*", "/:to*", &long);
            let _ = apply("/go/:a/:b/:c/:d/:e", "/:e/:d/:c/:b/:a", &long);
            let _ = matches(&"/:x*".repeat(2_000), &long);
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(5), "took {:?}", start.elapsed());
        // A destination that names a capture the source never defines keeps
        // the placeholder rather than panicking.
        assert_eq!(apply("/a", "/:missing", "/a").as_deref(), Some("/:missing"));
        assert_eq!(apply("", "", "").as_deref(), Some(""));
        assert_eq!(apply("/:*", "/:*", "/x").as_deref(), Some("/x"));
    }
}
