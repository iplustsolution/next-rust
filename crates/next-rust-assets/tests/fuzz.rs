//! The CSS and HTML passes run on application sources at build time and on
//! component styles at startup: malformed input must come out as text, never
//! as a panic or a hang. Seeds are fixed, so a failure is reproducible.

use std::time::{Duration, Instant};

use next_rust_assets::{css, html};

/// xorshift64*: small, seeded, the same sequence on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn mix(&mut self, pieces: &[&str], max: usize) -> String {
        let n = (self.next() % max as u64) as usize;
        (0..n).map(|_| pieces[(self.next() % pieces.len() as u64) as usize]).collect()
    }
}

const CSS: &[&str] = &[
    "{",
    "}",
    "}}",
    "{{",
    "(",
    ")",
    "[",
    "]",
    ";",
    ":",
    ",",
    ".",
    "#",
    "@",
    "\\",
    "\\.",
    "\\\n",
    "\"",
    "'",
    "/*",
    "*/",
    "/",
    "*",
    " ",
    "\n",
    "\t",
    "\0",
    "é",
    "𝒳",
    "\u{feff}",
    "a",
    ".btn",
    ".md\\:flex",
    ".w-1\\/2",
    ":hover",
    "::before",
    ":not(",
    ":is(",
    ":where(",
    "@media",
    "@media (min-width:1px)",
    "@supports",
    "@layer",
    "@layer a,b;",
    "@keyframes",
    "@font-face",
    "@import",
    "@charset \"utf-8\";",
    "@container",
    "@property",
    "--x",
    "var(--x",
    "url(",
    "url(\"",
    "calc(",
    "color:red",
    "!important",
    "from",
    "to",
    "50%",
    "&",
    ">",
    "+",
    "~",
    "||",
    "<!--",
    "-->",
    "</style>",
    "expression(",
    "\\",
    "\\1F600 ",
    "\\ffffff",
    "u+0-7f",
];

const HTML: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "<!--",
    "-->",
    "--!>",
    "<!DOCTYPE html>",
    "<p>",
    "</p>",
    "<pre>",
    "</pre>",
    "<textarea>",
    "</textarea>",
    "<script>",
    "</script>",
    "</SCRIPT >",
    "<style>",
    "</style>",
    "<title>",
    "\"",
    "'",
    "=",
    " ",
    "\n",
    "\t",
    "\0",
    "a",
    "class=",
    "data-x=\"",
    "&amp;",
    "&",
    "é",
    "𝒳",
    "<![CDATA[",
    "]]>",
    "<svg>",
    "<a href='",
    "<br/>",
];

#[test]
fn css_passes_never_panic_or_hang() {
    let mut rng = Rng(0x5EED_2001);
    let start = Instant::now();
    for _ in 0..4000 {
        let input = rng.mix(CSS, 40);
        let _ = css::minify(&input);
        let _ = css::scope(&input, "salt", true);
        let _ = css::scope(&input, "salt", false);
        let _ = css::class_selectors(&input);
        let _ = css::prune(&input, &|class| class.len() % 2 == 0);
        let _ = css::prune(&input, &|_| false);
        let renamed = css::rename_classes(&input, &|class| Some(format!("x{}", class.len())));
        let _ = css::minify(&renamed);
    }
    // Long runs, and nesting far deeper than a stylesheet has. (`prune`
    // recurses once per nested block: tens of thousands of unclosed blocks
    // would overflow the stack, which only the project's own CSS could do,
    // at build time.)
    for deep in ["@media x{".repeat(200), "a{".repeat(200), "(".repeat(200_000), "/*".repeat(100_000)] {
        let _ = css::minify(&deep);
        let _ = css::prune(&deep, &|_| true);
        let _ = css::rename_classes(&deep, &|_| None);
        let _ = css::class_selectors(&deep);
    }
    assert!(start.elapsed() < Duration::from_secs(60), "{:?}", start.elapsed());
}

#[test]
fn html_passes_never_panic_and_inline_scripts_stay_closed() {
    let mut rng = Rng(0x5EED_2002);
    for _ in 0..6000 {
        let input = rng.mix(HTML, 30);
        let _ = html::minify(&input);
        // Whatever a script holds, written inline it cannot end its element
        // or enter the "script data escaped" state.
        let code = html::escape_inline_script(&input).to_ascii_lowercase();
        assert!(!code.contains("</script") && !code.contains("<!--"), "{input:?} -> {code:?}");
    }
    let _ = html::minify(&"<div>".repeat(100_000));
    let _ = html::minify(&"<!--".repeat(100_000));
}
