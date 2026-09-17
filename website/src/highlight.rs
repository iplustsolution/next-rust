//! Syntax highlighting for code blocks, done in Rust while the page renders,
//! so readers download highlighted HTML and no JavaScript.
//!
//! This is a small tokenizer tuned for the languages in the docs (Rust, TOML,
//! shell, HTML, CSS, JavaScript), not a full parser.

use next_rust::prelude::escape_text;

/// Decode the entities an HTML escaper produces.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&")
}

/// Highlight every `<pre><code class="language-…">` block in `html` and wrap
/// it in a frame that shows the language.
pub fn code_blocks(html: &str) -> String {
    const OPEN: &str = "<pre><code";
    const CLOSE: &str = "</code></pre>";
    let mut out = String::with_capacity(html.len() * 2);
    let mut rest = html;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + OPEN.len()..];
        let (Some(tag_end), Some(end)) = (after.find('>'), after.find(CLOSE)) else {
            out.push_str(&rest[start..]);
            return out;
        };
        let lang = after[..tag_end].split("language-").nth(1).map(|l| l.trim_end_matches('"')).unwrap_or("text");
        let source = unescape(&after[tag_end + 1..end]);
        let label = match lang {
            "rust" => "Rust",
            "toml" => "TOML",
            "sh" | "bash" => "Terminal",
            "html" => "HTML",
            "css" => "CSS",
            "js" => "JavaScript",
            "ini" => "systemd",
            "caddyfile" => "Caddyfile",
            _ => "",
        };
        out.push_str("<div class=\"code\">");
        if !label.is_empty() {
            out.push_str("<div class=\"code-bar\"><span>");
            out.push_str(label);
            out.push_str("</span></div>");
        }
        out.push_str("<pre><code>");
        out.push_str(&highlight(lang, source.trim_end_matches('\n')));
        out.push_str("</code></pre></div>");
        rest = &after[end + CLOSE.len()..];
    }
    out.push_str(rest);
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Rust,
    Toml,
    Shell,
    Html,
    Css,
    Js,
    Plain,
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];
const JS_KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "const",
    "else",
    "export",
    "false",
    "for",
    "from",
    "function",
    "if",
    "import",
    "let",
    "new",
    "null",
    "of",
    "return",
    "true",
    "try",
    "catch",
    "typeof",
    "undefined",
    "var",
    "while",
];
const SHELL_COMMANDS: &[&str] = &[
    "cargo",
    "next-rust",
    "curl",
    "cd",
    "docker",
    "ssh",
    "scp",
    "git",
    "export",
    "sudo",
    "rustup",
    "npm",
    "sh",
    "echo",
];

/// Highlight one code block, returning escaped HTML.
pub fn highlight(lang: &str, code: &str) -> String {
    let lang = match lang {
        "rust" | "rs" => Lang::Rust,
        "toml" | "ini" => Lang::Toml,
        "sh" | "bash" | "shell" | "caddyfile" => Lang::Shell,
        "html" => Lang::Html,
        "css" => Lang::Css,
        "js" | "javascript" | "json" => Lang::Js,
        _ => Lang::Plain,
    };
    if lang == Lang::Plain {
        return escape_text(code).into_owned();
    }
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::with_capacity(code.len() * 2);
    let mut i = 0;
    let mut line_start = true;
    while i < chars.len() {
        let c = chars[i];
        let rest = |n: usize| chars[i..].iter().take(n).collect::<String>();

        // Comments.
        let line_comment = match lang {
            Lang::Rust | Lang::Js | Lang::Css => rest(2) == "//" && lang != Lang::Css,
            Lang::Toml | Lang::Shell => c == '#' && (lang == Lang::Toml || line_start || chars[i - 1].is_whitespace()),
            _ => false,
        };
        if line_comment {
            let end = chars[i..].iter().position(|&c| c == '\n').map(|p| i + p).unwrap_or(chars.len());
            push(&mut out, "c", &chars[i..end]);
            i = end;
            continue;
        }
        if matches!(lang, Lang::Rust | Lang::Js | Lang::Css) && rest(2) == "/*" {
            let end = find(&chars, i + 2, "*/").map(|p| p + 2).unwrap_or(chars.len());
            push(&mut out, "c", &chars[i..end]);
            i = end;
            continue;
        }
        if lang == Lang::Html && rest(4) == "<!--" {
            let end = find(&chars, i + 4, "-->").map(|p| p + 3).unwrap_or(chars.len());
            push(&mut out, "c", &chars[i..end]);
            i = end;
            continue;
        }

        // Strings.
        if c == '"'
            || (c == '\'' && matches!(lang, Lang::Js | Lang::Shell | Lang::Css | Lang::Toml))
            || (c == '`' && lang == Lang::Js)
        {
            let end = string_end(&chars, i, c);
            push(&mut out, "s", &chars[i..end]);
            i = end;
            line_start = false;
            continue;
        }
        // Rust raw strings r#"…"#.
        if lang == Lang::Rust && c == 'r' && matches!(chars.get(i + 1), Some('#') | Some('"')) {
            let hashes = chars[i + 1..].iter().take_while(|&&c| c == '#').count();
            if chars.get(i + 1 + hashes) == Some(&'"') {
                let close: String = std::iter::once('"').chain(std::iter::repeat_n('#', hashes)).collect();
                let end = find(&chars, i + 2 + hashes, &close).map(|p| p + close.len()).unwrap_or(chars.len());
                push(&mut out, "s", &chars[i..end]);
                i = end;
                continue;
            }
        }
        // Rust chars and lifetimes.
        if lang == Lang::Rust && c == '\'' {
            if chars.get(i + 2) == Some(&'\'') || (chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'')) {
                let end = if chars.get(i + 1) == Some(&'\\') { i + 4 } else { i + 3 };
                push(&mut out, "s", &chars[i..end]);
                i = end;
                continue;
            }
            let end = i + 1 + chars[i + 1..].iter().take_while(|c| c.is_alphanumeric() || **c == '_').count();
            push(&mut out, "k", &chars[i..end]);
            i = end;
            continue;
        }
        // Rust attributes.
        if lang == Lang::Rust && c == '#' && matches!(chars.get(i + 1), Some('[') | Some('!')) {
            let end = chars[i..].iter().position(|&c| c == ']').map(|p| i + p + 1).unwrap_or(chars.len());
            push(&mut out, "a", &chars[i..end]);
            i = end;
            continue;
        }
        // TOML tables.
        if lang == Lang::Toml && line_start && c == '[' {
            let end = chars[i..].iter().position(|&c| c == '\n').map(|p| i + p).unwrap_or(chars.len());
            push(&mut out, "t", &chars[i..end]);
            i = end;
            continue;
        }
        // HTML tags and attributes.
        if lang == Lang::Html && c == '<' {
            let name_end =
                i + 1 + chars[i + 1..].iter().take_while(|c| c.is_alphanumeric() || **c == '/' || **c == '!').count();
            push(&mut out, "t", &chars[i..name_end]);
            i = name_end;
            while i < chars.len() && chars[i] != '>' {
                if chars[i] == '"' {
                    let end = string_end(&chars, i, '"');
                    push(&mut out, "s", &chars[i..end]);
                    i = end;
                } else if chars[i].is_alphabetic() {
                    let end = i + chars[i..].iter().take_while(|c| c.is_alphanumeric() || **c == '-').count();
                    push(&mut out, "a", &chars[i..end]);
                    i = end;
                } else {
                    push(&mut out, "", &chars[i..i + 1]);
                    i += 1;
                }
            }
            if i < chars.len() {
                push(&mut out, "t", &chars[i..i + 1]);
                i += 1;
            }
            continue;
        }
        // Numbers.
        if c.is_ascii_digit() && (i == 0 || !is_ident(chars[i - 1])) {
            let end =
                i + chars[i..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == '_' || **c == '.').count();
            push(&mut out, "n", &chars[i..end]);
            i = end;
            line_start = false;
            continue;
        }
        // Words.
        if is_ident_start(c) || (lang == Lang::Css && (c == '-' || c == '.')) || (lang == Lang::Shell && c == '-') {
            let end =
                i + chars[i..].iter().take_while(|&&c| is_ident(c) || (lang != Lang::Rust && c == '-')).count().max(1);
            let word: String = chars[i..end].iter().collect();
            let next = chars.get(end).copied();
            let class = match lang {
                Lang::Rust if RUST_KEYWORDS.contains(&word.as_str()) => "k",
                Lang::Rust if next == Some('!') => "m",
                Lang::Rust if next == Some('(') => "f",
                Lang::Rust if word.starts_with(char::is_uppercase) => "y",
                Lang::Js if JS_KEYWORDS.contains(&word.as_str()) => "k",
                Lang::Js if next == Some('(') => "f",
                Lang::Toml if line_start => "p",
                Lang::Toml if word == "true" || word == "false" => "k",
                Lang::Shell if line_start && SHELL_COMMANDS.contains(&word.as_str()) => "f",
                Lang::Shell if word.starts_with('-') => "a",
                Lang::Css if next == Some(':') && !line_start => "p",
                Lang::Css if word.starts_with('.') || word.starts_with("--") => "y",
                _ => "",
            };
            // `!` belongs to the macro name.
            let end = if class == "m" { end + 1 } else { end };
            push(&mut out, class, &chars[i..end]);
            i = end;
            line_start = false;
            continue;
        }
        if lang == Lang::Shell && c == '$' && line_start {
            push(&mut out, "c", &chars[i..i + 1]);
            i += 1;
            continue;
        }
        if c == '\n' {
            line_start = true;
        } else if !c.is_whitespace() {
            line_start = false;
        }
        push(&mut out, "", &chars[i..i + 1]);
        i += 1;
    }
    out
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn find(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (from..chars.len().saturating_sub(needle.len() - 1)).find(|&i| chars[i..].starts_with(&needle))
}

fn string_end(chars: &[char], start: usize, quote: char) -> usize {
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            c if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    chars.len()
}

fn push(out: &mut String, class: &str, text: &[char]) {
    let text: String = text.iter().collect();
    if class.is_empty() {
        out.push_str(&escape_text(&text));
    } else {
        out.push_str("<span class=\"t");
        out.push_str(class);
        out.push_str("\">");
        out.push_str(&escape_text(&text));
        out.push_str("</span>");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_tokens() {
        let html = highlight("rust", "pub fn Page() -> impl View { h1![\"<hi>\"] } // done");
        assert!(html.contains("<span class=\"tk\">pub</span>"), "{html}");
        assert!(html.contains("<span class=\"tf\">Page</span>"), "{html}");
        assert!(html.contains("<span class=\"ty\">View</span>"), "{html}");
        assert!(html.contains("<span class=\"tm\">h1!</span>"), "{html}");
        assert!(html.contains("<span class=\"ts\">\"&lt;hi&gt;\"</span>"), "{html}");
        assert!(html.contains("<span class=\"tc\">// done</span>"), "{html}");
    }

    #[test]
    fn blocks_are_unescaped_once_and_framed() {
        let out = code_blocks(
            "<p>a</p><pre><code class=\"language-toml\">[server]\nport = 3000 # web\n</code></pre><p>b</p>",
        );
        assert!(out.starts_with("<p>a</p><div class=\"code\"><div class=\"code-bar\"><span>TOML</span>"), "{out}");
        assert!(out.contains("<span class=\"tt\">[server]</span>"), "{out}");
        assert!(out.contains("<span class=\"tp\">port</span>"), "{out}");
        assert!(out.contains("<span class=\"tn\">3000</span>"), "{out}");
        assert!(out.ends_with("</code></pre></div><p>b</p>"), "{out}");
        let text = code_blocks("<pre><code>a &amp;&amp; b</code></pre>");
        assert!(text.contains("a &amp;&amp; b"), "{text}");
    }

    #[test]
    fn every_docs_page_highlights_without_losing_text() {
        for doc in crate::docs::all() {
            let out = code_blocks(doc.html());
            let visible = |html: &str| crate::docs::text_of(html).chars().filter(|c| !c.is_whitespace()).count();
            assert_eq!(visible(&out), visible(doc.html()) + visible(&labels(&out)), "{}", doc.slug);
        }
    }

    fn labels(html: &str) -> String {
        html.split("<div class=\"code-bar\"><span>").skip(1).map(|p| p.split('<').next().unwrap_or("")).collect()
    }
}
