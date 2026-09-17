//! A conservative HTML minifier for hand-written `page.html` files.
//!
//! It removes comments and formatting whitespace without changing how a page
//! renders:
//!
//! * whitespace runs in text collapse to a single space;
//! * whitespace next to block-level tags (where browsers ignore it) is removed;
//! * whitespace between inline content is kept as one space;
//! * `<pre>`, `<textarea>` and `<script>` contents are copied verbatim;
//! * `<style>` contents are minified with [`crate::css::minify`];
//! * tags themselves (and attribute values) are never rewritten.

const BLOCK: &[&str] = &[
    "!doctype",
    "html",
    "head",
    "body",
    "title",
    "meta",
    "link",
    "base",
    "script",
    "style",
    "noscript",
    "div",
    "p",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "nav",
    "main",
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "td",
    "th",
    "caption",
    "colgroup",
    "col",
    "form",
    "fieldset",
    "legend",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "br",
    "pre",
    "blockquote",
    "figure",
    "figcaption",
    "details",
    "summary",
    "dialog",
    "address",
    "option",
    "optgroup",
    "select",
    "iframe",
    "canvas",
    "video",
    "audio",
    "source",
    "track",
    "picture",
    "svg",
    "template",
    "hgroup",
    "menu",
];

const RAW: &[&str] = &["pre", "textarea", "script", "style"];

enum Token {
    /// Full tag text and lowercase name (without `/`).
    Tag(String, String),
    Text(String),
}

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_start_matches('/')
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '>' && *c != '/')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_block(name: &str) -> bool {
    BLOCK.contains(&name)
}

fn find_ci(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    haystack.get(from..)?.to_ascii_lowercase().find(needle).map(|i| i + from)
}

/// Minify an HTML document or fragment.
pub fn minify(input: &str) -> String {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        if rest.starts_with("<!--") {
            i = find_ci(input, "-->", i + 4).map(|e| e + 3).unwrap_or(input.len());
            continue;
        }
        if rest.starts_with('<') && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!') {
            let end = tag_end(input, i);
            let tag = input[i..end].to_owned();
            let name = tag_name(&tag);
            let closing = tag.starts_with("</");
            tokens.push(Token::Tag(tag, name.clone()));
            i = end;
            if !closing && RAW.contains(&name.as_str()) {
                let close = format!("</{name}");
                let content_end = find_ci(input, &close, i).unwrap_or(input.len());
                let content = &input[i..content_end];
                if !content.is_empty() {
                    let body = if name == "style" { crate::css::minify(content) } else { content.to_owned() };
                    tokens.push(Token::Text(format!("\u{0}{body}")));
                }
                i = content_end;
            }
            continue;
        }
        let next = rest.find('<').map(|p| i + p).unwrap_or(input.len());
        let next = if next == i { i + 1 } else { next };
        tokens.push(Token::Text(input[i..next].to_owned()));
        i = next;
    }

    let mut out = String::with_capacity(input.len());
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::Tag(tag, _) => out.push_str(tag),
            Token::Text(text) => {
                if let Some(raw) = text.strip_prefix('\u{0}') {
                    out.push_str(raw);
                    continue;
                }
                let prev_block = idx == 0 || matches!(tokens.get(idx - 1), Some(Token::Tag(_, n)) if is_block(n));
                let next_block = matches!(tokens.get(idx + 1), None | Some(Token::Tag(_, _)))
                    && tokens.get(idx + 1).is_none_or(|t| matches!(t, Token::Tag(_, n) if is_block(n)));
                let mut collapsed = String::with_capacity(text.len());
                let mut space = false;
                for c in text.chars() {
                    if c.is_whitespace() {
                        space = true;
                    } else {
                        if space && !(collapsed.is_empty() && prev_block) {
                            collapsed.push(' ');
                        }
                        space = false;
                        collapsed.push(c);
                    }
                }
                if collapsed.is_empty() {
                    if space && !prev_block && !next_block {
                        out.push(' ');
                    }
                    continue;
                }
                if space && !next_block {
                    collapsed.push(' ');
                }
                out.push_str(&collapsed);
            }
        }
    }
    out
}

/// End index (exclusive) of the tag starting at `start`, honouring quotes.
fn tag_end(input: &str, start: usize) -> usize {
    let bytes = input.as_bytes();
    let mut quote = None;
    let mut i = start + 1;
    while i < bytes.len() {
        match (quote, bytes[i]) {
            (None, b'"' | b'\'') => quote = Some(bytes[i]),
            (Some(q), b) if b == q => quote = None,
            (None, b'>') => return i + 1,
            _ => {}
        }
        i += 1;
    }
    input.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_formatting_and_comments() {
        let html = "<!DOCTYPE html>\n<html>\n  <head>\n    <!-- a comment -->\n    <title> My  page </title>\n  </head>\n  <body>\n    <p>Hello,\n       <b>world</b> and <i>you</i>!</p>\n  </body>\n</html>\n";
        assert_eq!(
            minify(html),
            "<!DOCTYPE html><html><head><title>My page</title></head><body><p>Hello, <b>world</b> and <i>you</i>!</p></body></html>"
        );
    }

    #[test]
    fn keeps_significant_content() {
        let html = "<pre>\n  keep   this\n</pre>\n<textarea>  a\n b</textarea>\n<script>if (a < b) { x = \"  \"; }</script>\n<style>\n  body { color : red; }\n</style>\n<a href=\"/x y\"  title='a > b'>link</a> <span>next</span>";
        assert_eq!(
            minify(html),
            "<pre>\n  keep   this\n</pre><textarea>  a\n b</textarea><script>if (a < b) { x = \"  \"; }</script><style>body{color:red}</style><a href=\"/x y\"  title='a > b'>link</a> <span>next</span>"
        );
    }

    #[test]
    fn plain_text_comparisons_survive() {
        assert_eq!(minify("<p>1 < 2 and 3 > 2</p>"), "<p>1 < 2 and 3 > 2</p>");
    }
}
