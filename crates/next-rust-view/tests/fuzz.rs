//! Property tests for the renderer's escaping: whatever a string holds, it
//! cannot leave the place it was written to. Deterministic (fixed seeds) and
//! fast, so they run with the rest of the suite.

use next_rust_view::*;

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
}

/// Pieces that matter to an HTML parser, mixed with arbitrary characters.
const HOSTILE: &[&str] = &[
    "<",
    ">",
    "\"",
    "'",
    "&",
    "/",
    "=",
    "`",
    " ",
    "\t",
    "\n",
    "\0",
    "\\",
    "<script>",
    "</script>",
    "</SCRIPT >",
    "</style>",
    "</title>",
    "</textarea>",
    "<!--",
    "-->",
    "--!>",
    "<![CDATA[",
    "]]>",
    "&lt;",
    "&#60;",
    "&quot",
    "javascript:",
    "JaVa\tScRiPt:",
    "data:text/html,",
    "onerror=",
    " onload=alert(1) ",
    "<img src=x onerror=alert(1)>",
    "<svg/onload=alert(1)>",
    "\u{2028}",
    "\u{2029}",
    "\u{feff}",
    "\u{202e}",
    "é",
    "𝒳",
    "\u{fffd}",
    "%3C",
    "\r",
    "<nr-island data-module=\"/_next-rust/client/x.js\">",
    "<script src=\"/_next-rust/runtime.js\">",
    "$nr(1)",
    "<template id=\"nr-t1\">",
    "<!--nr-l:0000000000000000-->",
];

fn hostile(rng: &mut Rng) -> String {
    let mut s = String::new();
    for _ in 0..rng.below(12) {
        if rng.below(4) == 0 {
            s.push(char::from_u32(rng.below(0x3000) as u32).unwrap_or('x'));
        } else {
            s.push_str(HOSTILE[rng.below(HOSTILE.len())]);
        }
    }
    s
}

/// Undo `escape_text`/`escape_attr`.
fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

#[test]
fn text_and_attribute_values_never_leave_their_context() {
    let mut rng = Rng(0x5EED_0001);
    for _ in 0..4000 {
        let (text, value) = (hostile(&mut rng), hostile(&mut rng));
        let html = render_static(div![title(value.clone()), data("x", value.clone()), text.clone()]);
        let rest = html.strip_prefix("<div title=\"").unwrap_or_else(|| panic!("{html:?}"));
        let (title, rest) = rest.split_once("\" data-x=\"").unwrap_or_else(|| panic!("{html:?}"));
        let (data, rest) = rest.split_once("\">").unwrap_or_else(|| panic!("{html:?}"));
        let body = rest.strip_suffix("</div>").unwrap_or_else(|| panic!("{html:?}"));
        for written in [title, data] {
            assert!(!written.contains(['"', '\'', '<', '>']), "attribute breakout: {value:?} -> {html:?}");
            assert_eq!(unescape(written), value, "attribute values round-trip");
        }
        assert!(!body.contains(['<', '>']), "markup from text: {text:?} -> {html:?}");
        assert_eq!(unescape(body), text, "text round-trips");
    }
}

#[test]
fn script_and_style_content_cannot_close_its_element() {
    let mut rng = Rng(0x5EED_0002);
    for _ in 0..4000 {
        let code = hostile(&mut rng);
        for (html, close) in
            [(render_static(script![code.clone()]), "</script>"), (render_static(style![code.clone()]), "</style>")]
        {
            let open = html.find('>').unwrap() + 1;
            let inner = html[open..].strip_suffix(close).unwrap_or_else(|| panic!("{html:?}"));
            let lower = inner.to_ascii_lowercase();
            assert!(!lower.contains("</script") && !lower.contains("</style"), "{code:?} -> {html:?}");
            assert!(!inner.contains("<!--"), "script data escape state: {code:?} -> {html:?}");
        }
    }
}

#[test]
fn hostile_attribute_names_and_urls_are_dropped() {
    let mut rng = Rng(0x5EED_0003);
    for _ in 0..4000 {
        let (name, value) = (hostile(&mut rng), hostile(&mut rng));
        let html = render_static(a![attr(name.clone(), "v"), href(value.clone()), src(value.clone()), "x"]);
        // One element, whatever the name was: no second tag, no handler.
        assert_eq!(html.matches('<').count(), 2, "{name:?} -> {html:?}");
        assert_eq!(html.matches('>').count(), 2, "{name:?} -> {html:?}");
        let tag = html[..html.find('>').unwrap()].strip_prefix("<a").unwrap();
        // Outside quoted values nothing but names, `=` and spaces remains.
        let mut outside = String::new();
        let mut quoted = false;
        for c in tag.chars() {
            match c {
                '"' => quoted = !quoted,
                c if !quoted => outside.push(c),
                _ => {}
            }
        }
        assert!(!quoted, "unbalanced quotes: {html:?}");
        for word in outside.split([' ', '=']).filter(|w| !w.is_empty()) {
            assert!(!word.to_ascii_lowercase().starts_with("on"), "event handler from {name:?}: {html:?}");
            assert!(
                word.chars().all(|c| !c.is_whitespace() && !c.is_control() && !"\"'<>/=`".contains(c)),
                "{name:?} -> {html:?}"
            );
        }
        let cleaned: String = value.chars().filter(|c| !c.is_ascii_control() && !c.is_whitespace()).collect();
        if cleaned.to_ascii_lowercase().starts_with("javascript:") {
            assert!(html.contains(" href=\"#\" src=\"#\">"), "{value:?} -> {html:?}");
        }
    }
}

#[test]
fn metadata_is_escaped() {
    let mut rng = Rng(0x5EED_0004);
    for _ in 0..1500 {
        let s = hostile(&mut rng);
        let head = Metadata::new().title(s.clone()).description(s.clone()).render_head();
        // <title> is RCDATA: only its own end tag closes it, and text is escaped.
        assert_eq!(head.to_ascii_lowercase().matches("</title").count(), 1, "{s:?} -> {head:?}");
        assert!(!head.to_ascii_lowercase().contains("<script"), "{s:?} -> {head:?}");
        assert!(!head.contains("<img") && !head.contains("<svg"), "{s:?} -> {head:?}");
    }
}
