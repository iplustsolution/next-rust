//! Short names for release builds: `data-*` attributes, CSS custom
//! properties, ids, `@keyframes` and `@layer` names.
//!
//! Release builds give the names a page uses to talk to its own stylesheets
//! and scripts short, random replacements (`data-auth-scope` → `data-k`,
//! `--color-primary` → `--b3`). The build decides which names are safe to
//! change, rewrites the scripts it compiles into the binary and hands the
//! table to the app; the server then writes the short names into every HTML
//! response as it leaves: attributes, ids and the stylesheets in the page.
//! Rendering, caching and per-page CSS all work with the names as written.
//!
//! This module holds everything both sides share, so a name is always
//! rewritten by the same rules: the table lookup ([`find`]), the CSS pass
//! ([`rename_css`]), the script passes ([`rename_tokens`], [`rename_js_text`])
//! and the attribute rules ([`attr_name`], [`attr_value`]).
//!
//! ## Keys
//!
//! One table holds every kind of name; the key says which kind it is:
//!
//! | key | value | meaning |
//! | --- | --- | --- |
//! | `data-auth-scope` | `data-k` | an attribute (an empty value leaves the attribute out) |
//! | `--color-primary` | `--b3` | a custom property |
//! | `#newsletter-email` | `q` | an id |
//! | `#auth-*` | `w-` | every id with that prefix; the rest of the id is kept |
//! | `#nr-field-+` | `z-` | the same, for a prefix followed by a counter only |
//! | `@k:fade-up` | `c` | a `@keyframes` name |
//! | `@l:components` | `d` | a `@layer` name |
//!
//! Short names only make pages smaller and harder to read. They are not a
//! security boundary: whatever a browser needs, a person can read too.

use std::borrow::Cow;

/// Names the client runtime reads or sets. The runtime is served as
/// written, so a build never renames these, nor anything under `data-nr-`
/// (a test in the server crate keeps the list complete).
pub const RUNTIME_NAMES: &[&str] =
    &["--nr-load", "--nr-load-delay", "--nr-load-speed", "--nr-loader", "--nr-page-fade", "data-module", "data-props"];

/// The short form of `key` in `table` (sorted by key). Ids (`#…`) also match
/// the longest family entry, whose value replaces the prefix: `#auth-*` is
/// every id starting with `auth-`, `#nr-field-+` every id made of `nr-field-`
/// and a counter (digits and dashes).
pub fn find<K: AsRef<str>, V: AsRef<str>>(table: &[(K, V)], key: &str) -> Option<String> {
    if let Ok(i) = table.binary_search_by(|(k, _)| k.as_ref().cmp(key)) {
        return Some(table[i].1.as_ref().to_owned());
    }
    let id = key.strip_prefix('#')?;
    let from = table.partition_point(|(k, _)| k.as_ref() < "#");
    let mut best: Option<(&str, &str)> = None;
    for (k, v) in &table[from..] {
        let Some(pattern) = k.as_ref().strip_prefix('#') else { break };
        let (prefix, counter) = match (pattern.strip_suffix('*'), pattern.strip_suffix('+')) {
            (Some(prefix), _) => (prefix, false),
            (_, Some(prefix)) => (prefix, true),
            _ => continue,
        };
        let matches = id.len() > prefix.len()
            && id.starts_with(prefix)
            && (!counter || id[prefix.len()..].bytes().all(|b| b.is_ascii_digit() || b == b'-'));
        if matches && best.is_none_or(|(p, _)| prefix.len() > p.len()) {
            best = Some((prefix, v.as_ref()));
        }
    }
    best.map(|(prefix, short)| format!("{short}{}", &id[prefix.len()..]))
}

fn is_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// End of the run of name characters starting at `from`.
fn name_end(b: &[u8], from: usize) -> usize {
    from + b[from..].iter().take_while(|c| is_name_char(**c)).count()
}

/// `strengthFor` → `data-strength-for`: the attribute behind a `dataset` key.
pub fn dataset_attr(key: &str) -> String {
    let mut out = String::from("data-");
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `data-strength-for` → `strengthFor`: the `dataset` key of an attribute.
pub fn dataset_key(attr: &str) -> String {
    let name = attr.strip_prefix("data-").unwrap_or(attr);
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for c in name.chars() {
        if c == '-' {
            upper = true;
        } else if upper && c.is_ascii_lowercase() {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            if upper {
                out.push('-');
            }
            out.push(c);
            upper = false;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Text: selectors, declarations and HTML inside script strings
// ---------------------------------------------------------------------------

/// Rename the names in a piece of text that is known to be data, not code:
/// the contents of a script's string literal. Whole tokens only:
/// `data-name` (`[data-open]`, `setAttribute("data-open", …)`), `--name`
/// (`var(--x)`, `setProperty("--x", …)`) and `#id` (`querySelector("#menu")`).
pub fn rename_tokens<'a>(text: &'a str, f: &dyn Fn(&str) -> Option<String>) -> Cow<'a, str> {
    let b = text.as_bytes();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut i = 0;
    while i < b.len() {
        let boundary = i == 0 || !is_name_char(b[i - 1]);
        let (start, end, key_start) = match b[i] {
            b'd' if boundary && b[i..].starts_with(b"data-") => (i, name_end(b, i), i),
            b'-' if boundary && b[i..].starts_with(b"--") && b.get(i + 2).is_some_and(|c| is_name_char(*c)) => {
                (i, name_end(b, i), i)
            }
            // Not `&#39;`. `a#menu` and `/page#menu` both name the id.
            b'#' if (i == 0 || b[i - 1] != b'&')
                && b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_') =>
            {
                (i + 1, name_end(b, i + 1), i)
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let key = &text[key_start..end];
        // `data-nr-class-` is the start of a family of names, not a name.
        if !key.ends_with('-')
            && let Some(short) = f(key).filter(|s| !s.is_empty())
        {
            let out = out.get_or_insert_with(|| String::with_capacity(text.len()));
            out.push_str(&text[copied..start]);
            out.push_str(&short);
            copied = end;
        }
        i = end.max(i + 1);
    }
    match out {
        Some(mut out) => {
            out.push_str(&text[copied..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(text),
    }
}

/// Rename the names of a whole script without parsing it: every token
/// [`rename_tokens`] knows, and `.dataset.key` accesses. For scripts whose
/// contents are known (the framework's own); the build parses the scripts
/// of a project instead and rewrites their string literals only.
pub fn rename_js_text(js: &str, f: &dyn Fn(&str) -> Option<String>) -> String {
    let renamed = rename_tokens(js, f);
    let text = renamed.as_ref();
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut from = 0;
    while let Some(at) = text[from..].find("dataset.").map(|at| from + at) {
        let start = at + "dataset.".len();
        let end = start + b[start..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == b'_').count();
        from = end.max(start);
        if at > 0 && is_name_char(b[at - 1]) {
            continue;
        }
        if let Some(short) = f(&dataset_attr(&text[start..end])) {
            out.push_str(&text[copied..start]);
            out.push_str(&dataset_key(&short));
            copied = end;
        }
    }
    out.push_str(&text[copied..]);
    out
}

// ---------------------------------------------------------------------------
// CSS
// ---------------------------------------------------------------------------

/// Rename the names of a stylesheet: custom properties wherever they are
/// declared or used, `[data-*]` selectors and `attr(data-*)`, `#id`
/// selectors and `url(#id)`, `@keyframes` names with the `animation`
/// declarations that use them, and `@layer` names. Strings and comments are
/// left alone. `f` gets a key (see the module documentation) and returns the
/// short name, or `None` to keep the name; a build collects the names of a
/// stylesheet by recording what it is asked.
pub fn rename_css(css: &str, f: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(css.len());
    css_block(css, f, &mut out);
    out
}

/// Rename the names in a `style` attribute: a list of declarations.
pub fn rename_css_declarations<'a>(style: &'a str, f: &dyn Fn(&str) -> Option<String>) -> Cow<'a, str> {
    if !style.contains("--") && !style.contains("url(") && !style.contains("attr(") && !style.contains("animation") {
        return Cow::Borrowed(style);
    }
    let out = rename_css(style, f);
    if out == style { Cow::Borrowed(style) } else { Cow::Owned(out) }
}

/// Index after the string starting at `at` (a quote).
fn skip_css_string(b: &[u8], at: usize) -> usize {
    let quote = b[at];
    let mut i = at + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            c if c == quote => return i + 1,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// Index after the comment starting at `at` (`/*`).
fn skip_css_comment(text: &str, at: usize) -> usize {
    text[at + 2..].find("*/").map_or(text.len(), |end| at + 2 + end + 2)
}

/// The statements of a block: declarations end at `;`, rules at their `}`.
fn css_block(text: &str, f: &dyn Fn(&str) -> Option<String>, out: &mut String) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // Where this statement ends: `;`, `{` or the end of the block.
        let mut j = i;
        let mut parens = 0usize;
        let mut open = None;
        while j < b.len() {
            match b[j] {
                b'"' | b'\'' => {
                    j = skip_css_string(b, j);
                    continue;
                }
                b'/' if b.get(j + 1) == Some(&b'*') => {
                    j = skip_css_comment(text, j);
                    continue;
                }
                b'\\' => j += 1,
                b'(' => parens += 1,
                b')' => parens = parens.saturating_sub(1),
                b';' if parens == 0 => break,
                b'{' if parens == 0 => {
                    open = Some(j);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let j = j.min(b.len());
        match open {
            None => {
                let end = (j + 1).min(b.len());
                css_declaration(&text[i..end], f, out);
                i = end;
            }
            Some(open) => {
                let close = css_matching_brace(text, open);
                css_prelude(&text[i..open], f, out);
                out.push('{');
                css_block(&text[open + 1..close.min(b.len())], f, out);
                if close < b.len() {
                    out.push('}');
                }
                i = close + 1;
            }
        }
    }
}

fn css_matching_brace(text: &str, open: usize) -> usize {
    let b = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_css_string(b, i);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = skip_css_comment(text, i);
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

/// What comes before a `{`: an at-rule header or a selector.
fn css_prelude(prelude: &str, f: &dyn Fn(&str) -> Option<String>, out: &mut String) {
    let trimmed = prelude.trim_start();
    let lead = prelude.len() - trimmed.len();
    // Comments before a rule stay as they are.
    if trimmed.starts_with("/*") {
        let end = skip_css_comment(trimmed, 0);
        out.push_str(&prelude[..lead + end]);
        return css_prelude(&prelude[lead + end..], f, out);
    }
    let Some(at) = trimmed.strip_prefix('@') else {
        return css_selector(prelude, f, out);
    };
    let name_len = at.bytes().take_while(|c| is_name_char(*c)).count();
    let (name, rest) = at.split_at(name_len);
    out.push_str(&prelude[..lead + 1 + name_len]);
    match name.trim_start_matches("-webkit-").trim_start_matches("-moz-") {
        "keyframes" => {
            let ident = rest.trim();
            match f(&format!("@k:{ident}")) {
                Some(short) if !ident.is_empty() => out.push_str(&rest.replacen(ident, &short, 1)),
                _ => out.push_str(rest),
            }
        }
        "layer" => css_layer_names(rest, f, out),
        _ => css_value(rest, false, f, out),
    }
}

/// `a, b.c` after `@layer`.
fn css_layer_names(list: &str, f: &dyn Fn(&str) -> Option<String>, out: &mut String) {
    let b = list.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if is_name_char(b[i]) {
            let end = name_end(b, i);
            let name = &list[i..end];
            out.push_str(&f(&format!("@l:{name}")).unwrap_or_else(|| name.to_owned()));
            i = end;
        } else {
            // Whatever is between the names is copied as it stands. It has
            // to be copied a character at a time: a layer name may be
            // followed by a letter that takes more than one byte, and a byte
            // of it on its own is not a character.
            let rest = &list[i..];
            let width = rest.chars().next().map_or(1, char::len_utf8);
            out.push_str(&rest[..width]);
            i += width;
        }
    }
}

/// A selector: `[data-*]` and `#id`.
fn css_selector(selector: &str, f: &dyn Fn(&str) -> Option<String>, out: &mut String) {
    let b = selector.as_bytes();
    let mut copied = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_css_string(b, i);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = skip_css_comment(selector, i);
                continue;
            }
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' => {
                let start = i + 1 + b[i + 1..].iter().take_while(|c| c.is_ascii_whitespace()).count();
                if b[start..].starts_with(b"data-") {
                    let end = name_end(b, start);
                    if let Some(short) = f(&selector[start..end]).filter(|s| !s.is_empty()) {
                        out.push_str(&selector[copied..start]);
                        out.push_str(&short);
                        copied = end;
                    }
                    i = end;
                    continue;
                }
            }
            b'#' if b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b'-') => {
                let end = name_end(b, i + 1);
                // An escaped id (`#a\:b`) keeps its name.
                if b.get(end) != Some(&b'\\')
                    && let Some(short) = f(&selector[i..end])
                {
                    out.push_str(&selector[copied..i + 1]);
                    out.push_str(&short);
                    copied = end;
                }
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out.push_str(&selector[copied..]);
}

/// `property: value;`, or an at-statement (`@layer a, b;`).
fn css_declaration(text: &str, f: &dyn Fn(&str) -> Option<String>, out: &mut String) {
    let trimmed = text.trim_start();
    let lead = text.len() - trimmed.len();
    if trimmed.starts_with("/*") {
        let end = skip_css_comment(trimmed, 0);
        out.push_str(&text[..lead + end]);
        return css_declaration(&text[lead + end..], f, out);
    }
    if let Some(at) = trimmed.strip_prefix('@') {
        let name_len = at.bytes().take_while(|c| is_name_char(*c)).count();
        out.push_str(&text[..lead + 1 + name_len]);
        return match &at[..name_len] {
            "layer" => css_layer_names(&at[name_len..], f, out),
            _ => out.push_str(&at[name_len..]),
        };
    }
    let Some(colon) = trimmed.find(':') else {
        out.push_str(text);
        return;
    };
    let property = trimmed[..colon].trim_end();
    let custom = property.starts_with("--");
    out.push_str(&text[..lead]);
    match if custom { f(property) } else { None } {
        Some(short) => {
            out.push_str(&short);
            out.push_str(&trimmed[property.len()..colon]);
        }
        None => out.push_str(&trimmed[..colon]),
    }
    let value = &trimmed[colon..];
    let lower = property.to_ascii_lowercase();
    let animation =
        matches!(lower.trim_start_matches("-webkit-").trim_start_matches("-moz-"), "animation" | "animation-name")
            || (custom && has_time(value));
    css_value(value, animation, f, out);
}

/// Whether a value holds a time (`1s`, `.25s`, `150ms`): a custom property
/// holding a whole `animation` does (`--animate-spin: spin 1s linear infinite`).
fn has_time(value: &str) -> bool {
    let b = value.as_bytes();
    (0..b.len()).any(|i| {
        b[i].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_alphabetic()) && {
            let rest = &b[i + 1..];
            let digits = rest.iter().take_while(|c| c.is_ascii_digit() || **c == b'.').count();
            let unit = &rest[digits..];
            let len = if unit.starts_with(b"ms") {
                2
            } else if unit.starts_with(b"s") {
                1
            } else {
                0
            };
            len > 0 && !unit.get(len).is_some_and(|c| is_name_char(*c))
        }
    })
}

/// A value: `--x`, `attr(data-x)`, `url(#id)` and, where an animation is
/// named, keyframes names.
fn css_value(value: &str, animation: bool, f: &dyn Fn(&str) -> Option<String>, out: &mut String) {
    let b = value.as_bytes();
    let mut copied = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                i = skip_css_string(b, i);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = skip_css_comment(value, i);
                continue;
            }
            b'\\' => {
                i += 2;
                continue;
            }
            c if is_name_char(c) && (i == 0 || !is_name_char(b[i - 1])) => {
                let end = name_end(b, i);
                let token = &value[i..end];
                let call = b.get(end) == Some(&b'(');
                if call && token.eq_ignore_ascii_case("url") {
                    // `url(#id)`; any other URL is skipped whole.
                    let close = value[end..].find(')').map_or(b.len(), |c| end + c);
                    let inner = value[end + 1..close].trim().trim_matches(['"', '\'']);
                    if let Some(id) = inner.strip_prefix('#')
                        && let Some(short) = f(inner)
                    {
                        let at = end + 1 + value[end + 1..close].find(id).unwrap_or(0);
                        out.push_str(&value[copied..at]);
                        out.push_str(&short);
                        copied = at + id.len();
                    }
                    i = close;
                    continue;
                }
                if call && token.eq_ignore_ascii_case("attr") {
                    let start = end + 1 + b[end + 1..].iter().take_while(|c| c.is_ascii_whitespace()).count();
                    if b[start..].starts_with(b"data-") {
                        let name_end = name_end(b, start);
                        if let Some(short) = f(&value[start..name_end]).filter(|s| !s.is_empty()) {
                            out.push_str(&value[copied..start]);
                            out.push_str(&short);
                            copied = name_end;
                        }
                        i = name_end;
                        continue;
                    }
                }
                let short = if token.starts_with("--") && token.len() > 2 {
                    f(token)
                } else if animation && !call && !token.starts_with(|c: char| c.is_ascii_digit()) {
                    f(&format!("@k:{token}"))
                } else {
                    None
                };
                if let Some(short) = short {
                    out.push_str(&value[copied..i]);
                    out.push_str(&short);
                    copied = end;
                }
                i = end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out.push_str(&value[copied..]);
}

// ---------------------------------------------------------------------------
// Attributes
// ---------------------------------------------------------------------------

/// What happens to an attribute's name.
#[derive(Debug, PartialEq, Eq)]
pub enum AttrName<'a> {
    Keep,
    /// The build found nothing that reads the attribute: it is left out.
    Drop,
    Renamed(Cow<'a, str>),
}

/// The name an attribute is written with.
pub fn attr_name<'a>(name: &'a str, f: &dyn Fn(&str) -> Option<String>) -> AttrName<'a> {
    if !name.starts_with("data-") {
        return AttrName::Keep;
    }
    match f(name) {
        None => AttrName::Keep,
        Some(short) if short.is_empty() => AttrName::Drop,
        Some(short) => AttrName::Renamed(Cow::Owned(short)),
    }
}

/// Attributes holding one id.
const ID_REF: &[&str] = &[
    "id",
    "for",
    "form",
    "list",
    "popovertarget",
    "commandfor",
    "anchor",
    "aria-activedescendant",
    "aria-details",
    "aria-errormessage",
];

/// Attributes holding a space-separated list of ids.
const ID_REFS: &[&str] =
    &["aria-controls", "aria-describedby", "aria-labelledby", "aria-owns", "aria-flowto", "headers", "itemref"];

/// SVG attributes that point at an element with `url(#id)`.
const URL_REF: &[&str] = &["fill", "stroke", "filter", "mask", "clip-path", "marker-start", "marker-mid", "marker-end"];

fn ids<'a>(value: &'a str, all: bool, f: &dyn Fn(&str) -> Option<String>) -> Cow<'a, str> {
    let mut mapped = Vec::new();
    let mut changed = false;
    for id in value.split_ascii_whitespace() {
        match f(&format!("#{id}")) {
            Some(short) => {
                changed = true;
                mapped.push(short);
            }
            None if all => return Cow::Borrowed(value),
            None => mapped.push(id.to_owned()),
        }
    }
    if changed { Cow::Owned(mapped.join(" ")) } else { Cow::Borrowed(value) }
}

/// The value an attribute is written with: ids and references to them
/// (`for`, `aria-controls`, `href="#id"`, `url(#id)`, ...), the custom
/// properties of `style` and of any value that reads one (`fill="var(--x)"`),
/// and `data-*` values that are entirely ids the build renames
/// (`data-target="auth-password"`).
pub fn attr_value<'a>(name: &str, value: &'a str, f: &dyn Fn(&str) -> Option<String>) -> Cow<'a, str> {
    if value.is_empty() {
        return Cow::Borrowed(value);
    }
    if ID_REF.contains(&name) || ID_REFS.contains(&name) {
        return ids(value, false, f);
    }
    match name {
        "style" => rename_css_declarations(value, f),
        // A presentation attribute can read a custom property:
        // `font-family="var(--mono), monospace"`, `fill="var(--brand)"`.
        // SVG has dozens of them and HTML keeps adding some, so this looks
        // at the value rather than keeping a list of attribute names.
        _ if value.contains("var(--") => {
            let mut out = String::new();
            css_value(value, false, f, &mut out);
            if out == value { Cow::Borrowed(value) } else { Cow::Owned(out) }
        }
        "href" | "xlink:href" => {
            // Links into this site only: `#part`, `/page#part`.
            let local = value.starts_with('#') || (value.starts_with('/') && !value.starts_with("//"));
            match value.split_once('#') {
                Some((page, id)) if local && !id.is_empty() => match f(&format!("#{id}")) {
                    Some(short) => Cow::Owned(format!("{page}#{short}")),
                    None => Cow::Borrowed(value),
                },
                _ => Cow::Borrowed(value),
            }
        }
        _ if URL_REF.contains(&name) && value.starts_with("url(") => {
            let mut out = String::new();
            css_value(value, false, f, &mut out);
            if out == value { Cow::Borrowed(value) } else { Cow::Owned(out) }
        }
        _ if name.starts_with("data-") => ids(value, true, f),
        _ => Cow::Borrowed(value),
    }
}

// ---------------------------------------------------------------------------
// Compact markup
// ---------------------------------------------------------------------------

/// Whether an escaped attribute value can be written without quotes.
pub fn unquoted(escaped: &str) -> bool {
    !escaped.is_empty()
        && !escaped.bytes().any(|b| b.is_ascii_whitespace() || matches!(b, b'"' | b'\'' | b'`' | b'=' | b'<' | b'>'))
}

/// Whether an attribute only states the default, in a way nothing can
/// observe differently: `type="text"` stays (stylesheets select on it).
pub fn is_default_attr(tag: &str, name: &str, value: &str) -> bool {
    match (tag, name) {
        ("script", "type") => value.eq_ignore_ascii_case("text/javascript"),
        ("style" | "link", "type") => value.eq_ignore_ascii_case("text/css"),
        ("form", "method") => value.eq_ignore_ascii_case("get"),
        _ => false,
    }
}

const P_CLOSERS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "details",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "search",
    "section",
    "ul",
];
// `<table>` belongs in that list everywhere but a document in quirks mode,
// where a table nests inside an open paragraph instead of ending it. A
// response cannot tell which mode it will be read in, so `</p>` stays.

/// Parents a `<p>` may end with: their end tags close an open paragraph.
/// The standard makes this every element but a handful; the list is kept
/// to the ones that hold flow content in a document that is written the
/// way HTML asks, because the rest (a `<p>` inside a `<span>`, say) is
/// markup a browser puts back together in ways the end tag can change.
const P_PARENTS: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "dd",
    "dt",
    "details",
    "div",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "header",
    "li",
    "main",
    "nav",
    "section",
    "td",
    "th",
];

/// Whether a `<tag>` start tag closes a paragraph that is open before it.
pub fn closes_p(tag: &str) -> bool {
    P_CLOSERS.contains(&tag) || matches!(tag, "dd" | "dt" | "li" | "form")
}

/// Whether the end tag of `tag` may be left out without changing the
/// document a browser builds. `tag` is the innermost element still open;
/// `next` is the element that starts right after its end tag, or `None`
/// when an end tag follows instead, which makes `parent` (the element
/// `tag` is in) the one that ends it. Stricter than the HTML standard: the
/// parent has to be the usual one, so an `<li>` that ends inside a custom
/// element keeps its end tag.
pub fn end_tag_optional(tag: &str, next: Option<&str>, parent: &str) -> bool {
    match next {
        Some(next) => match tag {
            "li" => next == "li",
            "dt" | "dd" => matches!(next, "dt" | "dd"),
            "option" => matches!(next, "option" | "optgroup"),
            // An `<optgroup>` only closes an open one inside a `<select>`;
            // anywhere else a browser puts the second one inside the first.
            "optgroup" => next == "optgroup" && parent == "select",
            "tr" => next == "tr",
            "td" | "th" => matches!(next, "td" | "th"),
            "thead" | "tbody" => matches!(next, "tbody" | "tfoot"),
            "p" => closes_p(next),
            _ => false,
        },
        None => match tag {
            "li" => matches!(parent, "ul" | "ol" | "menu"),
            "dd" => parent == "dl",
            "option" => matches!(parent, "select" | "datalist" | "optgroup"),
            "optgroup" => parent == "select",
            "tr" => matches!(parent, "tbody" | "thead" | "tfoot" | "table"),
            "td" | "th" => parent == "tr",
            "tbody" | "tfoot" => parent == "table",
            "p" => P_PARENTS.contains(&parent),
            _ => false,
        },
    }
}

/// Numbers in SVG path data (`d`, `points`) rounded to `decimals`. Only
/// numbers with more decimals change, and only their decimals: `01.23456`
/// (two arc flags and a number) stays readable as what it was.
pub fn trim_numbers(data: &str, decimals: u8) -> Cow<'_, str> {
    let keep = decimals as usize;
    let b = data.as_bytes();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'.' || !b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            i += 1;
            continue;
        }
        let start = i + 1;
        let end = start + b[start..].iter().take_while(|c| c.is_ascii_digit()).count();
        i = end;
        // An exponent changes what the digits mean.
        if end - start <= keep || matches!(b.get(end), Some(b'e' | b'E')) {
            continue;
        }
        let mut digits: Vec<u8> = b[start..start + keep].to_vec();
        if b[start + keep] >= b'5' {
            // Round up; a carry into the whole part leaves the number alone.
            let mut at = digits.len();
            loop {
                if at == 0 {
                    break;
                }
                at -= 1;
                if digits[at] == b'9' {
                    digits[at] = b'0';
                } else {
                    digits[at] += 1;
                    break;
                }
            }
            if digits.iter().all(|d| *d == b'0') {
                continue;
            }
        }
        while digits.last() == Some(&b'0') {
            digits.pop();
        }
        let whole = start > 1 && b[start - 2].is_ascii_digit();
        let out = out.get_or_insert_with(|| String::with_capacity(data.len()));
        if digits.is_empty() {
            // `1.0004` → `1`, `.0004` → `0`; never glued to a following `.5`.
            out.push_str(&data[copied..start - 1]);
            if !whole {
                out.push('0');
            }
            if b.get(end) == Some(&b'.') {
                out.push(' ');
            }
        } else {
            out.push_str(&data[copied..start]);
            out.push_str(std::str::from_utf8(&digits).expect("digits"));
        }
        copied = end;
    }
    match out {
        Some(mut out) => {
            out.push_str(&data[copied..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &[(&str, &str)] = &[
        ("#auth-*", "w-"),
        ("#menu", "q"),
        ("#nr-field-+", "z-"),
        ("--brand", "--a"),
        ("--mx", "--b"),
        ("@k:fade-up", "c"),
        ("@l:components", "d"),
        ("@l:theme", "e"),
        ("data-component", ""),
        ("data-open", "data-k"),
        ("data-strength-for", "data-m"),
    ];

    fn f(key: &str) -> Option<String> {
        find(TABLE, key)
    }

    #[test]
    fn finds_names_and_id_prefixes() {
        assert_eq!(f("data-open").as_deref(), Some("data-k"));
        assert_eq!(f("#menu").as_deref(), Some("q"));
        assert_eq!(f("#auth-full_name").as_deref(), Some("w-full_name"));
        // The prefix alone is not an id of the family.
        assert_eq!(f("#auth-"), None);
        assert_eq!(f("#menus"), None);
        // A counter family: `nr-field-3`, not `nr-field-wrap`.
        assert_eq!(f("#nr-field-3").as_deref(), Some("z-3"));
        assert_eq!(f("#nr-field-0-12").as_deref(), Some("z-0-12"));
        assert_eq!(f("#nr-field-wrap"), None);
        assert_eq!(f("data-opened"), None);
    }

    #[test]
    fn dataset_keys_round_trip() {
        assert_eq!(dataset_attr("strengthFor"), "data-strength-for");
        assert_eq!(dataset_key("data-strength-for"), "strengthFor");
        assert_eq!(dataset_key("data-k"), "k");
        assert_eq!(dataset_attr("nrUi"), "data-nr-ui");
    }

    #[test]
    fn renames_a_stylesheet() {
        let css = r#"@layer theme,base,components;@layer theme{:root,[data-theme=x]{--brand:#fff;--mx:0;--other:var(--brand)}}@property --brand{syntax:"*";inherits:false}@layer components{.card[data-open]:hover,#menu>a,[ data-open="--brand" ]{color:var(--brand,var(--mx));content:attr(data-open) "--brand #menu";fill:url(#menu);animation:fade-up .3s var(--ease) both}.x{--anim:fade-up 1s;--name:fade-up;animation-name:fade-up,spin}}@keyframes fade-up{from{opacity:0;--mx:1}to{opacity:1}}@media (min-width:40rem){#menu{color:#abc}}/* --brand #menu */"#;
        let expected = r#"@layer e,base,d;@layer e{:root,[data-theme=x]{--a:#fff;--b:0;--other:var(--a)}}@property --a{syntax:"*";inherits:false}@layer d{.card[data-k]:hover,#q>a,[ data-k="--brand" ]{color:var(--a,var(--b));content:attr(data-k) "--brand #menu";fill:url(#q);animation:c .3s var(--ease) both}.x{--anim:c 1s;--name:fade-up;animation-name:c,spin}}@keyframes c{from{opacity:0;--b:1}to{opacity:1}}@media (min-width:40rem){#q{color:#abc}}/* --brand #menu */"#;
        assert_eq!(rename_css(css, &f), expected);
        // Nothing to rename: the same text.
        assert_eq!(rename_css(css, &|_| None), css);
        // Readable CSS keeps its formatting.
        let readable = ".a {\n  color: var( --brand );\n  animation: 1s fade-up;\n}\n";
        assert_eq!(rename_css(readable, &f), ".a {\n  color: var( --a );\n  animation: 1s c;\n}\n");
    }

    #[test]
    fn collects_the_names_a_stylesheet_asks_for() {
        let asked = std::cell::RefCell::new(Vec::new());
        rename_css("@layer a{#x[data-y]{--z:1;animation:k 1s}}", &|key| {
            asked.borrow_mut().push(key.to_owned());
            None
        });
        assert_eq!(asked.into_inner(), ["@l:a", "#x", "data-y", "--z", "@k:k"]);
    }

    #[test]
    fn urls_and_colors_are_not_ids() {
        let css = ".a{background:url(data:image/svg+xml;base64,--brand#menu) #menu;color:#menu}";
        // `#menu` in a value is a color, whatever it looks like.
        assert_eq!(rename_css(css, &f), css);
    }

    #[test]
    fn style_attributes() {
        assert_eq!(rename_css_declarations("--mx:50%;color:var(--brand)", &f), "--b:50%;color:var(--a)");
        assert!(matches!(rename_css_declarations("color:red", &f), Cow::Borrowed(_)));
        assert!(matches!(rename_css_declarations("--unknown:1", &f), Cow::Borrowed(_)));
    }

    #[test]
    fn script_strings_are_renamed_by_whole_tokens() {
        let text = r##"[data-open],[data-opened] .x-data-open #menu, a#menu &#menu; var(--brand) x--brand "data-nr-class-" #auth-email"##;
        assert_eq!(
            rename_tokens(text, &f),
            r##"[data-k],[data-opened] .x-data-open #q, a#q &#menu; var(--a) x--brand "data-nr-class-" #w-email"##
        );
        assert!(matches!(rename_tokens("nothing here", &f), Cow::Borrowed(_)));
        // A dropped attribute keeps its name in scripts.
        assert_eq!(rename_tokens("[data-component]", &f), "[data-component]");
    }

    #[test]
    fn whole_scripts_rename_dataset_accesses() {
        let js = r#"e.dataset.open="";t.dataset.strengthFor;x.mydataset.open;e.dataset.other;e.querySelector("[data-open]")"#;
        assert_eq!(
            rename_js_text(js, &f),
            r#"e.dataset.k="";t.dataset.m;x.mydataset.open;e.dataset.other;e.querySelector("[data-k]")"#
        );
    }

    #[test]
    fn attributes() {
        assert_eq!(attr_name("data-open", &f), AttrName::Renamed(Cow::Borrowed("data-k")));
        assert_eq!(attr_name("data-component", &f), AttrName::Drop);
        assert_eq!(attr_name("data-other", &f), AttrName::Keep);
        assert_eq!(attr_name("class", &f), AttrName::Keep);

        assert_eq!(attr_value("id", "menu", &f), "q");
        assert_eq!(attr_value("for", "auth-email", &f), "w-email");
        assert_eq!(attr_value("aria-labelledby", "auth-role-label other  menu", &f), "w-role-label other q");
        assert_eq!(attr_value("href", "#menu", &f), "#q");
        assert_eq!(attr_value("href", "/pricing#menu", &f), "/pricing#q");
        assert_eq!(attr_value("href", "https://example.com/#menu", &f), "https://example.com/#menu");
        assert_eq!(attr_value("href", "//example.com/#menu", &f), "//example.com/#menu");
        assert_eq!(attr_value("fill", "url(#menu)", &f), "url(#q)");
        assert_eq!(attr_value("fill", "#menu", &f), "#menu");
        // A presentation attribute reading a custom property, whatever it is
        // called: SVG `font-family`, `fill`, `stroke-width`, `offset`, ...
        assert_eq!(attr_value("font-family", "var(--brand), monospace", &f), "var(--a), monospace");
        assert_eq!(attr_value("stroke", "var(--brand, var(--mx))", &f), "var(--a, var(--b))");
        assert_eq!(attr_value("offset", "calc(var(--mx) * 2)", &f), "calc(var(--b) * 2)");
        assert_eq!(attr_value("data-x", "var(--brand)", &f), "var(--a)");
        assert_eq!(attr_value("alt", "var(--brand)", &f), "var(--a)");
        // Not a custom property: text that merely looks like one.
        assert!(matches!(attr_value("title", "var(--unknown)", &f), Cow::Borrowed(_)));
        assert_eq!(attr_value("style", "--mx:1", &f), "--b:1");
        // A `data-*` value that is an id the build renames, and only that.
        assert_eq!(attr_value("data-strength-for", "auth-password", &f), "w-password");
        assert_eq!(attr_value("data-tab", "menu email", &f), "menu email");
        assert_eq!(attr_value("value", "menu", &f), "menu");
        assert_eq!(attr_value("name", "menu", &f), "menu");
    }

    #[test]
    fn compact_markup_rules() {
        assert!(unquoted("email") && unquoted("/a/b?x&amp;y") && unquoted("a&quot;b"));
        assert!(!unquoted("") && !unquoted("a b") && !unquoted("a=b") && !unquoted("a'b") && !unquoted("a`b"));
        assert!(is_default_attr("form", "method", "GET") && is_default_attr("script", "type", "text/javascript"));
        assert!(!is_default_attr("input", "type", "text") && !is_default_attr("script", "type", "module"));
        assert!(end_tag_optional("li", Some("li"), "") && end_tag_optional("li", None, "ul"));
        assert!(!end_tag_optional("li", None, "nr-b") && !end_tag_optional("td", None, "template"));
        assert!(!end_tag_optional("li", Some("script"), "ul"));
        assert!(end_tag_optional("option", Some("option"), "") && end_tag_optional("td", None, "tr"));
        assert!(end_tag_optional("p", Some("div"), "") && end_tag_optional("p", None, "div"));
        assert!(!end_tag_optional("p", Some("span"), "") && !end_tag_optional("p", None, "a"));
        assert!(!end_tag_optional("p", None, "nr-island") && !end_tag_optional("div", None, "div"));
        assert!(!end_tag_optional("dt", None, "dl") && end_tag_optional("thead", Some("tbody"), ""));
        // A description term holds flow content, so a paragraph may end with it.
        assert!(end_tag_optional("p", None, "dt") && !end_tag_optional("p", None, "option"));
        assert!(!end_tag_optional("optgroup", Some("option"), "") && !end_tag_optional("thead", None, "table"));
        // Only a `<select>` makes one `<optgroup>` end another.
        assert!(end_tag_optional("optgroup", Some("optgroup"), "select"));
        assert!(!end_tag_optional("optgroup", Some("optgroup"), "x-widget"));
        // A paragraph ends before anything that would close it anyway.
        assert!(closes_p("div") && closes_p("li") && !closes_p("span"));
    }

    #[test]
    fn path_numbers_lose_only_extra_decimals() {
        assert_eq!(trim_numbers("M1.23456 2.5L.98765-1.00049", 3), "M1.235 2.5L.988-1");
        assert_eq!(trim_numbers("M1.2 3.456", 3), "M1.2 3.456");
        assert!(matches!(trim_numbers("M1.2 3.456", 3), Cow::Borrowed(_)));
        // A carry into the whole part, an exponent: left alone.
        assert_eq!(trim_numbers("M1.9996 2.5e-7 1.23456e3", 3), "M1.9996 2.5e-7 1.23456e3");
        // Arc flags written against a number keep their digits.
        assert_eq!(trim_numbers("a1 1 0 01.23456.5", 2), "a1 1 0 01.23.5");
        // `1.0004.5` is two numbers: they stay two.
        assert_eq!(trim_numbers("1.0004.5 .0004.5", 3), "1 .5 0 .5");
    }
}
