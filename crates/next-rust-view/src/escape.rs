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
/// are ignored during the check, defeating `java\tscript:` tricks. A leading
/// byte order mark goes with them: no URL parser in use today reads
/// `\u{feff}javascript:` as a scheme, and this way none has to.
pub fn is_safe_url(url: &str) -> bool {
    let cleaned: String = url
        .chars()
        .filter(|c| !c.is_ascii_control() && !c.is_whitespace() && *c != '\u{feff}')
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
    }

    const URL_PIECES: &[&str] = &[
        "j",
        "a",
        "v",
        "s",
        "c",
        "r",
        "i",
        "p",
        "t",
        ":",
        "J",
        "A",
        "V",
        "S",
        "C",
        "R",
        "I",
        "P",
        "T",
        "\t",
        "\n",
        "\r",
        " ",
        "\u{0}",
        "\u{b}",
        "\u{1f}",
        "\u{7f}",
        "\u{feff}",
        "\u{a0}",
        "\u{2028}",
        "&#58;",
        "&#x3a;",
        "%3a",
        "data",
        "image",
        "png",
        "svg",
        "+xml",
        ";base64,",
        "/",
        "vb",
        "blob",
        "filesystem",
        "http",
        "s",
        "//",
        "\\",
        ".",
        "#",
        "?",
        "e",
        "l",
        "b",
        "o",
        "f",
        "y",
        "m",
        "x",
    ];

    /// The schemes that must never survive into a URL attribute, spelled the
    /// ways a browser still accepts.
    #[test]
    fn script_schemes_are_refused_however_they_are_spelled() {
        for bad in [
            "javascript:alert(1)",
            "JAVASCRIPT:alert(1)",
            "jAvAsCrIpT:alert(1)",
            "java\u{0}script:alert(1)",
            "java\tscript:alert(1)",
            "java\nscript:alert(1)",
            "java\rscript:alert(1)",
            "\u{1}\u{2}\u{3}javascript:alert(1)",
            "   javascript:alert(1)",
            "\u{feff}javascript:alert(1)",
            "vbscript:msgbox(1)",
            "VBScript:x",
            "data:text/html,<script>alert(1)</script>",
            "data:text/html;base64,PHNjcmlwdD4=",
            "DATA:TEXT/HTML,x",
            "data:image/svg+xml,<svg onload=alert(1)>",
            "data:image/svg+xml;base64,PHN2Zz4=",
            "data:image/SVG+xml,x",
            "data:,x",
            "data:application/javascript,alert(1)",
        ] {
            assert!(!is_safe_url(bad), "{bad:?} must not be a safe URL");
        }
    }

    /// And the ones that are ordinary URLs, which must keep working.
    #[test]
    fn everything_that_is_not_script_stays_usable() {
        for ok in [
            "/about",
            "",
            "#anchor",
            "?q=javascript:alert(1)",
            "https://example.com/?next=javascript:alert(1)",
            "mailto:a@example.com",
            "tel:+1",
            "data:image/png;base64,iVBORw0KGgo=",
            "data:image/webp;base64,UklGR",
            "data:image/gif;base64,R0lGOD",
            // Not script either. `blob:` and `filesystem:` need a URL a
            // script already created, and `\`/`//` are relative URLs that
            // point at another origin — a redirect question, not an XSS one.
            "blob:https://example.com/9d2c",
            "filesystem:https://example.com/temporary/x",
            "\\evil.example",
            "//evil.example",
            "\\\\evil.example\\share",
        ] {
            assert!(is_safe_url(ok), "{ok:?} must stay usable");
        }
    }

    /// Generated URLs: whatever `is_safe_url` accepts, a browser stripping
    /// the characters it ignores must not read as a script scheme.
    #[test]
    fn no_generated_url_slips_a_script_scheme_past_the_check() {
        let mut rng = Rng(0x5AFE_0DD0_1234);
        for _ in 0..60_000 {
            let url: String = (0..rng.below(14)).map(|_| URL_PIECES[rng.below(URL_PIECES.len())]).collect();
            if !is_safe_url(&url) {
                continue;
            }
            // What a browser does before it looks for the scheme: drop the
            // ASCII control characters and whitespace it ignores, then compare
            // case-insensitively.
            let seen: String =
                url.chars().filter(|c| !c.is_ascii_control() && !c.is_whitespace()).collect::<String>().to_lowercase();
            for scheme in ["javascript:", "vbscript:"] {
                assert!(!seen.starts_with(scheme), "{url:?} reads as {scheme}");
            }
            if seen.starts_with("data:") {
                assert!(seen.starts_with("data:image/"), "{url:?} is a non-image data URL");
                assert!(!seen.starts_with("data:image/svg"), "{url:?} is an SVG data URL");
            }
        }
    }

    /// Escaping is total: whatever goes in, what comes out has no character
    /// that can end an attribute, a tag or an element.
    #[test]
    fn generated_text_never_keeps_a_character_that_ends_markup() {
        let mut rng = Rng(0xE5CA_9E00_0001);
        const PIECES: &[&str] =
            &["<", ">", "&", "\"", "'", "`", "=", "/", " ", "\n", "\u{0}", "&amp;", "&#39;", "</script>", "é", "a"];
        for _ in 0..40_000 {
            let text: String = (0..rng.below(10)).map(|_| PIECES[rng.below(PIECES.len())]).collect();
            let attr = escape_attr(&text);
            assert!(!attr.contains(['<', '>', '"', '\'']), "{text:?} -> {attr:?}");
            let body = escape_text(&text);
            assert!(!body.contains(['<', '>']), "{text:?} -> {body:?}");
            // Every `&` that survives starts an entity this function wrote.
            for tail in attr.split('&').skip(1) {
                assert!(
                    ["amp;", "lt;", "gt;", "quot;", "#39;"].iter().any(|e| tail.starts_with(e)),
                    "{text:?} -> {attr:?}"
                );
            }
            // Raw text (`<script>`, `<style>`) only has to survive a closing
            // tag and a comment opener.
            let raw = escape_raw_text(&text);
            assert!(!raw.contains("</") && !raw.contains("<!--"), "{text:?} -> {raw:?}");
            // Names that would break out of the attribute syntax are refused.
            if is_valid_attr_name(&text) {
                assert!(!text.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '>' | '<' | '=' | '/')));
            }
            if is_valid_tag_name(&text) {
                assert!(text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':'));
            }
        }
    }

    /// A very long value costs time proportional to its length, once.
    #[test]
    fn escaping_stays_linear() {
        let big = "<&\"'>".repeat(200_000);
        let start = std::time::Instant::now();
        assert_eq!(escape_attr(&big).len(), big.len() / 5 * "&lt;&amp;&quot;&#39;&gt;".len());
        assert!(is_safe_url(&big));
        assert!(start.elapsed() < std::time::Duration::from_secs(5), "took {:?}", start.elapsed());
    }
}
