//! A small, dependency-free CSS transformer.
//!
//! It is intentionally *not* a full CSS parser. It understands just enough
//! structure (strings, comments, blocks, at-rules, selector vs. declaration
//! context, nesting) to perform two safe transformations:
//!
//! * **Minification** – remove comments, collapse whitespace and drop
//!   redundant semicolons without ever touching string contents.
//! * **Scoping** (CSS modules) – rename class selectors `.card` to
//!   `.card_<hash>` so styles cannot leak between modules. Class names in
//!   declaration values (for example `url(a.png)`) are never rewritten.
//!   Use `:global(.name)` to opt a selector out of scoping.

use std::collections::BTreeMap;

use crate::hash::content_hash;

/// Options for [`transform`].
#[derive(Debug, Clone, Default)]
pub struct TransformOptions {
    /// Remove comments and superfluous whitespace.
    pub minify: bool,
    /// When `Some(scope)`, rename class selectors using `scope` as salt.
    pub scope: Option<String>,
}

/// Result of [`transform`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransformOutput {
    pub css: String,
    /// Original class name → scoped class name (empty when not scoping).
    pub classes: BTreeMap<String, String>,
}

/// Minify a stylesheet.
pub fn minify(css: &str) -> String {
    transform(css, &TransformOptions { minify: true, scope: None }).css
}

/// Scope all class selectors of a stylesheet (CSS module semantics).
///
/// `salt` should uniquely identify the module, e.g. its source path.
pub fn scope(css: &str, salt: &str, minify: bool) -> TransformOutput {
    transform(css, &TransformOptions { minify, scope: Some(salt.to_owned()) })
}

/// Derive the scoped name for `class` in the module identified by `salt`.
pub fn scoped_class_name(class: &str, salt: &str) -> String {
    let hash = content_hash(format!("{salt}\u{0}{class}").as_bytes());
    format!("{class}_{}", &hash[..8])
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Block {
    /// Block whose statements are rules (top level, `@media`, `@supports`...).
    Rules,
    /// Block whose rules must not be scoped (e.g. `@keyframes`).
    Keyframes,
    /// Declaration block (may contain nested rules).
    Decls,
}

const RULE_AT_RULES: &[&str] = &["media", "supports", "layer", "container", "document", "scope", "starting-style"];

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || b >= 0x80 || b == b'\\'
}

fn is_ident_char(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

/// Run the transformer.
pub fn transform(input: &str, opts: &TransformOptions) -> TransformOutput {
    let src = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(src.len());
    let mut classes = BTreeMap::new();
    let mut stack: Vec<Block> = Vec::new();
    // Text of the current statement prelude (used to classify at-rules).
    let mut prelude: Vec<u8> = Vec::new();
    // Whether the current statement (inside a declaration block) is a nested rule.
    let mut stmt_start = true;
    let mut nested_selector = false;
    let mut pending_space = false;
    // Depth of an active `:global(` wrapper: parentheses depth at which it closes.
    let mut global_depth: Option<usize> = None;
    let mut paren_depth = 0usize;
    let mut i = 0usize;

    let top = |stack: &Vec<Block>| *stack.last().unwrap_or(&Block::Rules);

    macro_rules! flush_space {
        ($next:expr) => {{
            if pending_space {
                pending_space = false;
                let prev = out.last().copied();
                let next: u8 = $next;
                let sel = selector_context(&stack, nested_selector) && !prelude.starts_with(b"@");
                let drop = prev.is_none()
                    || matches!(prev, Some(b'{' | b'}' | b';' | b',' | b'>' | b'('))
                    || (prev == Some(b':') && (!sel || paren_depth > 0))
                    || matches!(next, b'{' | b'}' | b';' | b',' | b'>' | b')')
                    || (next == b':' && (!sel || paren_depth > 0));
                if !drop {
                    out.push(b' ');
                }
            }
        }};
    }

    fn selector_context(stack: &[Block], nested_selector: bool) -> bool {
        match stack.last() {
            None | Some(Block::Rules) => true,
            Some(Block::Keyframes) => false,
            Some(Block::Decls) => nested_selector,
        }
    }

    while i < src.len() {
        let b = src[i];

        // Comments.
        if b == b'/' && src.get(i + 1) == Some(&b'*') {
            let end = find(src, i + 2, b"*/").map(|e| e + 2).unwrap_or(src.len());
            if opts.minify {
                pending_space = true;
            } else {
                out.extend_from_slice(&src[i..end]);
            }
            i = end;
            continue;
        }

        // Whitespace.
        if b.is_ascii_whitespace() {
            let mut j = i;
            while j < src.len() && src[j].is_ascii_whitespace() {
                j += 1;
            }
            if opts.minify {
                pending_space = true;
            } else {
                out.extend_from_slice(&src[i..j]);
            }
            if !prelude.is_empty() {
                prelude.push(b' ');
            }
            i = j;
            continue;
        }

        if opts.minify {
            flush_space!(b);
        }

        // Strings are copied verbatim.
        if b == b'"' || b == b'\'' {
            let mut j = i + 1;
            while j < src.len() && src[j] != b {
                if src[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            let end = (j + 1).min(src.len());
            out.extend_from_slice(&src[i..end]);
            prelude.extend_from_slice(&src[i..end]);
            stmt_start = false;
            i = end;
            continue;
        }

        // url(...) contents are copied verbatim.
        if (b == b'u' || b == b'U') && src.len() >= i + 4 && src[i..i + 4].eq_ignore_ascii_case(b"url(") {
            let end = find(src, i + 4, b")").map(|e| e + 1).unwrap_or(src.len());
            out.extend_from_slice(&src[i..end]);
            stmt_start = false;
            i = end;
            continue;
        }

        if top(&stack) == Block::Decls && stmt_start {
            // Decide whether this statement is a nested rule or a declaration.
            nested_selector = matches!(b, b'.' | b'&' | b'#' | b'[' | b'>' | b'+' | b'~' | b'*')
                || (b == b':' && src.get(i + 1).is_some_and(|c| *c == b':' || c.is_ascii_alphabetic()) && {
                    // `:hover {` nested pseudo-class vs `color: red`: a
                    // declaration never *starts* with a colon.
                    true
                });
            stmt_start = false;
        }
        let in_selector = selector_context(&stack, nested_selector);

        match b {
            b'{' => {
                let kind = match top(&stack) {
                    Block::Rules | Block::Keyframes => classify_prelude(&prelude),
                    Block::Decls => {
                        if prelude.first() == Some(&b'@') {
                            classify_prelude(&prelude)
                        } else {
                            Block::Decls
                        }
                    }
                };
                stack.push(kind);
                out.push(b'{');
                prelude.clear();
                stmt_start = true;
                nested_selector = false;
                i += 1;
            }
            b'}' => {
                if opts.minify && out.last() == Some(&b';') {
                    out.pop();
                }
                stack.pop();
                out.push(b'}');
                prelude.clear();
                stmt_start = true;
                nested_selector = false;
                i += 1;
            }
            b';' => {
                out.push(b';');
                prelude.clear();
                stmt_start = true;
                nested_selector = false;
                i += 1;
            }
            b'(' => {
                paren_depth += 1;
                out.push(b'(');
                prelude.push(b'(');
                i += 1;
            }
            b')' => {
                if global_depth == Some(paren_depth) {
                    // Closing paren of `:global(` is dropped.
                    global_depth = None;
                } else {
                    out.push(b')');
                }
                paren_depth = paren_depth.saturating_sub(1);
                prelude.push(b')');
                i += 1;
            }
            b':' if in_selector && opts.scope.is_some() && src[i..].starts_with(b":global(") => {
                paren_depth += 1;
                global_depth = Some(paren_depth);
                i += ":global(".len();
            }
            b'.' if in_selector
                && opts.scope.is_some()
                && global_depth.is_none()
                && src.get(i + 1).is_some_and(|c| is_ident_start(*c))
                && !prelude.starts_with(b"@") =>
            {
                let start = i + 1;
                let mut j = start;
                while j < src.len() && is_ident_char(src[j]) {
                    if src[j] == b'\\' {
                        j += 1;
                    }
                    j += 1;
                }
                let j = j.min(src.len());
                let name = String::from_utf8_lossy(&src[start..j]).into_owned();
                let salt = opts.scope.as_deref().unwrap_or_default();
                let scoped = classes.entry(name.clone()).or_insert_with(|| scoped_class_name(&name, salt)).clone();
                out.push(b'.');
                out.extend_from_slice(scoped.as_bytes());
                prelude.push(b'.');
                prelude.extend_from_slice(&src[start..j]);
                i = j;
            }
            _ => {
                out.push(b);
                prelude.push(b);
                i += 1;
            }
        }
    }

    // `:global(.a)` references inside selectors are recorded as-is so the
    // mapping only contains scoped names. Output is valid UTF-8 because we
    // only ever split at ASCII bytes.
    let css = String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    TransformOutput { css: css.trim().to_owned(), classes }
}

// ---------------------------------------------------------------------------
// Unused rule removal
// ---------------------------------------------------------------------------

/// Remove rules that can never match because a class or id they require is
/// not used anywhere. `used(name)` reports whether a class or id name (without
/// `.` or `#`) appears in the project.
///
/// The input must be minified (see [`minify`]). The removal is conservative:
///
/// * a selector is dead only when a class or id *outside any parentheses* is
///   unused, so `:not(.x)`, `:is(.a, .b)` and `:where(…)` never cause removal;
/// * in a selector list (`.a, .b`) only the dead selectors are dropped;
/// * element, attribute and pseudo selectors, `:root`, declarations,
///   `@font-face`, `@import`, `@property` and other at-rules are kept;
/// * `@media`, `@supports`, `@layer` and `@container` blocks are pruned
///   inside and dropped when empty;
/// * `@keyframes` are kept only when their name still appears in the output.
pub fn prune(css: &str, used: &dyn Fn(&str) -> bool) -> String {
    let pruned = prune_block(css, used);
    remove_unused_keyframes(&pruned)
}

/// Split minified CSS into top-level statements: `prelude{body}` or `text;`.
fn statements(css: &str) -> Vec<(&str, Option<&str>)> {
    let b = css.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let mut paren = 0usize;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => i = skip_string(b, i),
            b'\\' => i += 2,
            b'(' => {
                paren += 1;
                i += 1;
            }
            b')' => {
                paren = paren.saturating_sub(1);
                i += 1;
            }
            b';' if paren == 0 => {
                out.push((&css[start..i], None));
                i += 1;
                start = i;
            }
            b'{' => {
                let body_start = i + 1;
                let end = matching_brace(b, i);
                out.push((&css[start..i], Some(&css[body_start..end.min(css.len())])));
                i = end + 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < css.len() && !css[start..].trim().is_empty() {
        out.push((&css[start..], None));
    }
    out
}

fn skip_string(b: &[u8], at: usize) -> usize {
    let quote = b[at];
    let mut j = at + 1;
    while j < b.len() && b[j] != quote {
        if b[j] == b'\\' {
            j += 1;
        }
        j += 1;
    }
    (j + 1).min(b.len())
}

/// Index of the `}` matching the `{` at `open`.
fn matching_brace(b: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_string(b, i);
                continue;
            }
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// Prune the statements of a block: the top level, an at-rule's body, or a
/// style rule's body (declarations and nested rules).
fn prune_block(css: &str, used: &dyn Fn(&str) -> bool) -> String {
    let mut out = String::with_capacity(css.len());
    for (prelude, body) in statements(css) {
        let Some(body) = body else {
            // Declarations and statement at-rules (`@import`, `@charset`).
            if !prelude.is_empty() {
                if !out.is_empty() && !out.ends_with('{') && !out.ends_with('}') && !out.ends_with(';') {
                    out.push(';');
                }
                out.push_str(prelude);
                if prelude.starts_with('@') {
                    out.push(';');
                }
            }
            continue;
        };
        if let Some(at) = prelude.strip_prefix('@') {
            let name = at.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).next().unwrap_or("");
            let name = name.trim_start_matches("-webkit-").trim_start_matches("-moz-").to_ascii_lowercase();
            if RULE_AT_RULES.contains(&name.as_str()) {
                let inner = prune_block(body, used);
                if !inner.is_empty() {
                    push_rule(&mut out, prelude, &inner);
                }
            } else {
                // @font-face, @keyframes, @page, @property, …: kept as they are.
                push_rule(&mut out, prelude, body);
            }
            continue;
        }
        let live: Vec<&str> =
            split_top_level(prelude, b',').into_iter().filter(|s| selector_is_live(s, used)).collect();
        if live.is_empty() {
            continue;
        }
        let inner = prune_block(body, used);
        push_rule(&mut out, &live.join(","), &inner);
    }
    out
}

fn push_rule(out: &mut String, prelude: &str, body: &str) {
    if !out.is_empty() && !out.ends_with('{') && !out.ends_with('}') && !out.ends_with(';') {
        out.push(';');
    }
    out.push_str(prelude);
    out.push('{');
    out.push_str(body);
    out.push('}');
}

/// Split at `sep` outside parentheses, brackets and strings.
fn split_top_level(text: &str, sep: u8) -> Vec<&str> {
    let b = text.as_bytes();
    let mut parts = Vec::new();
    let (mut depth, mut start, mut i) = (0usize, 0usize, 0usize);
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_string(b, i);
                continue;
            }
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            c if c == sep && depth == 0 => {
                parts.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(&text[start..]);
    parts
}

/// A selector is live unless a class or id outside parentheses is unused.
fn selector_is_live(selector: &str, used: &dyn Fn(&str) -> bool) -> bool {
    let b = selector.as_bytes();
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_string(b, i);
                continue;
            }
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b'.' | b'#' if depth == 0 && b.get(i + 1).is_some_and(|c| is_ident_start(*c)) => {
                let start = i + 1;
                let mut j = start;
                let mut escaped = false;
                while j < b.len() && is_ident_char(b[j]) {
                    if b[j] == b'\\' {
                        escaped = true;
                        j += 1;
                    }
                    j += 1;
                }
                let name = &selector[start..j.min(b.len())];
                // Escaped names (`.md\:flex`) can't be matched against source
                // identifiers reliably: keep them.
                if !escaped && !used(name) {
                    return false;
                }
                i = j;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    true
}

/// Drop `@keyframes name{…}` whose name no longer appears elsewhere.
fn remove_unused_keyframes(css: &str) -> String {
    let stmts = statements(css);
    let mut names = Vec::new();
    for (prelude, body) in &stmts {
        if body.is_some()
            && let Some(name) = keyframes_name(prelude)
        {
            names.push(name);
        }
    }
    if names.is_empty() {
        return css.to_owned();
    }
    let mut out = String::with_capacity(css.len());
    for (prelude, body) in &stmts {
        if let (Some(body), Some(name)) = (body, keyframes_name(prelude)) {
            let elsewhere = css.replace(&format!("{prelude}{{{body}}}"), "");
            let referenced = elsewhere.match_indices(name).any(|(at, _)| {
                let before = elsewhere[..at].chars().next_back();
                let after = elsewhere[at + name.len()..].chars().next();
                !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    && !after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            });
            if !referenced {
                continue;
            }
        }
        match body {
            Some(body) => push_rule(&mut out, prelude, body),
            None => {
                if !out.is_empty() && !out.ends_with('}') && !out.ends_with(';') {
                    out.push(';');
                }
                out.push_str(prelude);
                if prelude.starts_with('@') {
                    out.push(';');
                }
            }
        }
    }
    out
}

fn keyframes_name(prelude: &str) -> Option<&str> {
    let rest = prelude.strip_prefix('@')?;
    let rest = rest.trim_start_matches("-webkit-").trim_start_matches("-moz-");
    let name = rest.strip_prefix("keyframes")?.trim();
    (!name.is_empty()).then_some(name.trim_matches(|c| c == '"' || c == '\''))
}

/// Class names in the selectors of a minified stylesheet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassSelectors {
    /// Every class name that appears in a selector (unescaped).
    pub all: std::collections::BTreeSet<String>,
    /// Classes that come first in at least one selector, like `mt-4` in
    /// `.mt-4` or `.hover\:underline:hover`, and `space-y-2` in
    /// `:where(.space-y-2>:not(:last-child))`. For generated utility CSS these
    /// are the utilities; classes only used as context (`.group` in
    /// `.group-hover\:x:is(:where(.group):hover *)`) are not included.
    pub leading: std::collections::BTreeSet<String>,
}

/// Collect the class names used in the selectors of a minified stylesheet.
pub fn class_selectors(css: &str) -> ClassSelectors {
    let mut found = ClassSelectors::default();
    for_each_selector(css, &mut |prelude| {
        for selector in split_top_level(prelude, b',') {
            let mut first = true;
            scan_classes(selector, &mut |_, name| {
                if first {
                    found.leading.insert(name.clone());
                    first = false;
                }
                found.all.insert(name);
            });
        }
    });
    found
}

/// Rename class selectors in a minified stylesheet. `rename` receives each
/// class name (unescaped) and returns the new name, or `None` to keep it.
/// New names are written as they are, so they must be plain identifiers
/// (`[a-z][a-z0-9]*`). Declarations, `@keyframes` and everything else are
/// copied byte for byte.
pub fn rename_classes(css: &str, rename: &dyn Fn(&str) -> Option<String>) -> String {
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for_each_selector(css, &mut |prelude| {
        let base = prelude.as_ptr() as usize - css.as_ptr() as usize;
        scan_classes(prelude, &mut |range, name| {
            if let Some(new) = rename(&name) {
                edits.push((base + range.start, base + range.end, new));
            }
        });
    });
    let mut out = String::with_capacity(css.len());
    let mut at = 0;
    for (start, end, new) in edits {
        out.push_str(&css[at..start]);
        out.push_str(&new);
        at = end;
    }
    out.push_str(&css[at..]);
    out
}

/// Calls `f` with the prelude of every style rule, including rules nested in
/// at-rules and in other rules. `@keyframes` bodies are skipped.
fn for_each_selector<'a>(css: &'a str, f: &mut dyn FnMut(&'a str)) {
    for (prelude, body) in statements(css) {
        let Some(body) = body else { continue };
        if let Some(at) = prelude.strip_prefix('@') {
            if keyframes_name(prelude).is_none() && !at.starts_with("font-face") && !at.starts_with("property") {
                for_each_selector(body, f);
            }
            continue;
        }
        f(prelude);
        for_each_selector(body, f);
    }
}

/// Finds the class selectors (`.name`) in a selector, outside strings and
/// attribute selectors, and calls `f` with the byte range of the name (after
/// the dot) and the unescaped name.
fn scan_classes(selector: &str, f: &mut dyn FnMut(std::ops::Range<usize>, String)) {
    let b = selector.as_bytes();
    let mut i = 0;
    let mut brackets = 0usize;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_string(b, i);
                continue;
            }
            b'\\' => i += 1,
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b'.' if brackets == 0 && b.get(i + 1).is_some_and(|c| is_ident_start(*c)) => {
                let start = i + 1;
                let (end, name) = read_ident(selector, start);
                if !name.is_empty() {
                    f(start..end, name);
                }
                i = end.max(start);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
}

/// Reads a CSS identifier starting at `start`, resolving escapes
/// (`\:` → `:`, `\32 ` → `2`). Returns the end offset and the name.
fn read_ident(text: &str, start: usize) -> (usize, String) {
    let b = text.as_bytes();
    let mut name = String::new();
    let mut i = start;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' {
            let hex = b[i + 1..].iter().take(6).take_while(|h| h.is_ascii_hexdigit()).count();
            if hex > 0 {
                let code = u32::from_str_radix(&text[i + 1..i + 1 + hex], 16).unwrap_or(0xFFFD);
                name.push(char::from_u32(code).filter(|&c| c != '\0').unwrap_or('\u{FFFD}'));
                i += 1 + hex;
                if b.get(i).is_some_and(|w| *w == b' ' || *w == b'\t' || *w == b'\n') {
                    i += 1;
                }
            } else if let Some(ch) = text.get(i + 1..).and_then(|rest| rest.chars().next()) {
                name.push(ch);
                i += 1 + ch.len_utf8();
            } else {
                i += 1;
            }
        } else if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' {
            name.push(c as char);
            i += 1;
        } else if c >= 0x80 {
            let ch = text[i..].chars().next().expect("char boundary");
            name.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    (i, name)
}

fn classify_prelude(prelude: &[u8]) -> Block {
    let text = String::from_utf8_lossy(prelude);
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('@') {
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect::<String>()
            .to_ascii_lowercase();
        let name = name.trim_start_matches("-webkit-").trim_start_matches("-moz-");
        if RULE_AT_RULES.contains(&name) {
            Block::Rules
        } else if name == "keyframes" {
            Block::Keyframes
        } else {
            Block::Decls
        }
    } else {
        Block::Decls
    }
}

fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minifies() {
        let css = "/* c */\nbody {\n  color : red;\n  margin: 0 auto;\n}\n\na > b ,  c { x: y; }";
        assert_eq!(minify(css), "body{color:red;margin:0 auto}a>b,c{x:y}");
    }

    #[test]
    fn keeps_strings_and_calc() {
        let css = r#"a::after { content: " { ; } "; width: calc(1px + 2px); }"#;
        assert_eq!(minify(css), r#"a::after{content:" { ; } ";width:calc(1px + 2px)}"#);
    }

    #[test]
    fn keeps_descendant_pseudo_space() {
        assert_eq!(minify("a :hover { x: y }"), "a :hover{x:y}");
        assert_eq!(
            minify("@media screen and (min-width: 1px) { a { b: c } }"),
            "@media screen and (min-width:1px){a{b:c}}"
        );
    }

    #[test]
    fn scopes_classes_in_selectors_only() {
        let out = scope(".card .title:hover, .card-body { background: url(x.png); width: .5em }", "m", true);
        let card = out.classes.get("card").unwrap();
        let title = out.classes.get("title").unwrap();
        let body = out.classes.get("card-body").unwrap();
        assert_eq!(out.css, format!(".{card} .{title}:hover,.{body}{{background:url(x.png);width:.5em}}"));
        assert_eq!(out.classes.len(), 3);
    }

    #[test]
    fn scopes_inside_media_but_not_font_face() {
        let out = scope("@media (max-width: 10px) { .a { x: y } } @font-face { src: url(a.woff2) }", "m", true);
        let a = out.classes.get("a").unwrap();
        assert_eq!(out.css, format!("@media (max-width:10px){{.{a}{{x:y}}}}@font-face{{src:url(a.woff2)}}"));
    }

    #[test]
    fn global_escape_hatch() {
        let out = scope(":global(.dark) .btn { color: white }", "m", true);
        let btn = out.classes.get("btn").unwrap();
        assert_eq!(out.css, format!(".dark .{btn}{{color:white}}"));
        assert!(!out.classes.contains_key("dark"));
    }

    #[test]
    fn nested_rules() {
        let out = scope(".a { color: red; .b { color: blue } &:hover { x: y } }", "m", true);
        let a = out.classes.get("a").unwrap();
        let b = out.classes.get("b").unwrap();
        assert_eq!(out.css, format!(".{a}{{color:red;.{b}{{color:blue}}&:hover{{x:y}}}}"));
    }

    #[test]
    fn keyframes_untouched() {
        let out = scope("@keyframes spin { from { a: b } 50.5% { a: c } }", "m", true);
        assert!(out.classes.is_empty());
        assert_eq!(out.css, "@keyframes spin{from{a:b}50.5%{a:c}}");
    }

    fn pruned(css: &str, used: &[&str]) -> String {
        prune(&minify(css), &|name| used.contains(&name))
    }

    #[test]
    fn prune_removes_rules_for_unused_classes() {
        let css = r#"
            :root { --brand: #f26b2a }
            body { margin: 0 }
            .btn { color: red }
            .unused { color: blue }
            .card .title, .ghost .x { font-weight: 700 }
            #app { display: grid }
            #nope { display: none }
            a[href^="http"]:hover { text-decoration: underline }
        "#;
        assert_eq!(
            pruned(css, &["btn", "card", "title", "app"]),
            r#":root{--brand:#f26b2a}body{margin:0}.btn{color:red}.card .title{font-weight:700}#app{display:grid}a[href^="http"]:hover{text-decoration:underline}"#
        );
    }

    #[test]
    fn prune_is_conservative_inside_functional_pseudo_classes() {
        let css = ".a:not(.unused) { x: y } :is(.b, .c) { x: z } .md\\:flex { display: flex }";
        assert_eq!(pruned(css, &["a"]), ".a:not(.unused){x:y}:is(.b,.c){x:z}.md\\:flex{display:flex}");
    }

    #[test]
    fn prune_media_nesting_and_keyframes() {
        let css = r#"
            @media (min-width: 40rem) { .used { a: b } .gone { a: c } }
            @supports (display: grid) { .gone { a: d } }
            @font-face { font-family: X; src: url(x.woff2) }
            .spin { animation: spin 1s linear infinite }
            .fade { animation: fade 1s }
            @keyframes spin { to { rotate: 1turn } }
            @keyframes fade { from { opacity: 0 } }
            .card { color: red; .inner { color: blue } .gone { color: green } &:hover { color: pink } }
        "#;
        assert_eq!(
            pruned(css, &["used", "spin", "card", "inner"]),
            "@media (min-width:40rem){.used{a:b}}@font-face{font-family:X;src:url(x.woff2)}.spin{animation:spin 1s linear infinite}@keyframes spin{to{rotate:1turn}}.card{color:red;.inner{color:blue}&:hover{color:pink}}"
        );
    }

    #[test]
    fn prune_keeps_everything_that_is_used() {
        let css = "a{b:c}.x{y:z}@media print{.x{d:e}}@import url(a.css);";
        let min = minify(css);
        assert_eq!(prune(&min, &|_| true), min);
    }

    #[test]
    fn deterministic_names() {
        assert_eq!(scoped_class_name("x", "salt"), scoped_class_name("x", "salt"));
        assert_ne!(scoped_class_name("x", "a"), scoped_class_name("x", "b"));
    }

    #[test]
    fn collects_leading_and_context_classes() {
        let css = r".mt-4{margin:0}.hover\:underline{&:hover{@media (hover:hover){text-decoration:underline}}}@layer utilities{:where(.space-y-2\.5>:not(:last-child)){margin:0}.group-open\:block:is(:where(.group):is([open]) *){display:block}.\[\&_\.tk\]\:text-red .tk{color:red}.\32 xl\:grid{display:grid}}@keyframes spin{0%{x:y}50.5%{x:z}}[class~=not-prose]{a:b}";
        let found = class_selectors(css);
        let leading: Vec<_> = found.leading.iter().map(String::as_str).collect();
        assert_eq!(
            leading,
            ["2xl:grid", "[&_.tk]:text-red", "group-open:block", "hover:underline", "mt-4", "space-y-2.5"]
        );
        assert!(found.all.contains("group") && found.all.contains("tk"));
        assert!(!found.all.contains("not-prose"));
    }

    #[test]
    fn renames_classes_in_selectors_only() {
        let css = r".mt-4{margin:0;background:url(a.mt-4.png)}.hover\:underline{&:hover{text-decoration:underline}}@media (width>=40rem){.\32 xl\:grid,.mt-4>.x{display:grid}}:where(.space-y-2\.5>:not(:last-child)){content:'.mt-4'}@keyframes k{12.5%{opacity:0}}";
        let renamed = rename_classes(css, &|name| match name {
            "mt-4" => Some("a".into()),
            "hover:underline" => Some("b".into()),
            "2xl:grid" => Some("c".into()),
            "space-y-2.5" => Some("d".into()),
            _ => None,
        });
        assert_eq!(
            renamed,
            ".a{margin:0;background:url(a.mt-4.png)}.b{&:hover{text-decoration:underline}}@media (width>=40rem){.c,.a>.x{display:grid}}:where(.d>:not(:last-child)){content:'.mt-4'}@keyframes k{12.5%{opacity:0}}"
        );
    }
}
