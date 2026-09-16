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
        dest = dest.replace(&format!(":{name}*"), &value).replace(&format!(":{name}"), &value);
    }
    Some(dest)
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
}
