//! HTML escaping and URL sanitation.

use std::borrow::Cow;

/// Escape text content (`&`, `<`, `>`).
pub fn escape_text(s: &str) -> Cow<'_, str> {
    escape(s, false)
}

/// Escape attribute values (`&`, `<`, `>`, `"`, `'`).
pub fn escape_attr(s: &str) -> Cow<'_, str> {
    escape(s, true)
}

fn escape(s: &str, attr: bool) -> Cow<'_, str> {
    let needs = s.bytes().any(|b| matches!(b, b'&' | b'<' | b'>') || (attr && matches!(b, b'"' | b'\'')));
    if !needs {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\'' if attr => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Escape text placed inside `<script>` or `<style>`: those elements are not
/// entity-decoded by browsers, so the only dangerous sequence is a closing
/// tag (and `<!--`, which changes script parsing state).
pub(crate) fn escape_raw_text(s: &str) -> Cow<'_, str> {
    if !s.contains("</") && !s.contains("<!--") {
        return Cow::Borrowed(s);
    }
    Cow::Owned(s.replace("</", "<\\/").replace("<!--", "<\\!--"))
}

/// Attributes whose values are URLs and must not carry script.
pub(crate) fn is_url_attr(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "href"
            | "src"
            | "action"
            | "formaction"
            | "poster"
            | "cite"
            | "background"
            | "xlink:href"
            | "manifest"
            | "data"
    )
}

/// Whether a URL is safe to place in a URL attribute.
///
/// Rejects `javascript:`, `vbscript:` and `data:` URLs (except
/// `data:image/*` which cannot execute script in an `<img>`). Control
/// characters and whitespace that browsers strip before parsing the scheme
/// are ignored during the check, defeating `java\tscript:` tricks.
pub fn is_safe_url(url: &str) -> bool {
    let cleaned: String = url
        .chars()
        .filter(|c| !c.is_ascii_control() && !c.is_whitespace())
        .take(32)
        .collect::<String>()
        .to_ascii_lowercase();
    if cleaned.starts_with("javascript:") || cleaned.starts_with("vbscript:") {
        return false;
    }
    if cleaned.starts_with("data:") {
        return cleaned.starts_with("data:image/") && !cleaned.starts_with("data:image/svg");
    }
    true
}

/// Attribute names must not contain characters that would break out of the
/// attribute syntax.
pub(crate) fn is_valid_attr_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| !c.is_control() && !c.is_whitespace() && !matches!(c, '"' | '\'' | '>' | '/' | '=' | '<' | '`'))
}

pub(crate) fn is_valid_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(escape_text("<a href='x'>&</a>"), "&lt;a href='x'&gt;&amp;&lt;/a&gt;");
        assert_eq!(escape_attr("\"'<>&"), "&quot;&#39;&lt;&gt;&amp;");
        assert!(matches!(escape_text("plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn urls() {
        assert!(is_safe_url("/about"));
        assert!(is_safe_url("https://example.com/?q=javascript:"));
        assert!(is_safe_url("data:image/png;base64,AAA"));
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url(" JaVa\tScRiPt:alert(1)"));
        assert!(!is_safe_url("\u{1}javascript:alert(1)"));
        assert!(!is_safe_url("data:text/html,<script>"));
        assert!(!is_safe_url("data:image/svg+xml,<svg onload=alert(1)>"));
        assert!(!is_safe_url("vbscript:x"));
    }

    #[test]
    fn raw_text() {
        assert_eq!(escape_raw_text("a</script><script>"), "a<\\/script><script>");
    }

    #[test]
    fn names() {
        assert!(is_valid_attr_name("data-x"));
        assert!(is_valid_attr_name("aria-label"));
        assert!(!is_valid_attr_name("x onload=alert(1)"));
        assert!(!is_valid_attr_name("a\"b"));
        assert!(is_valid_tag_name("my-element"));
        assert!(!is_valid_tag_name("script><img"));
    }
}
