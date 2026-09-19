//! Per-page CSS.
//!
//! An app-wide stylesheet (the Tailwind build, with the hand-written CSS
//! imported into it, or the UI components' styles) holds every rule the
//! whole app uses. A page needs only the rules for the classes it renders,
//! so the renderer splits the stylesheet into pieces once and sends, per
//! document, the pieces that page needs:
//!
//! * a rule whose selectors all start with a class is sent when one of
//!   those classes is on the page, wherever it is in the sheet: Tailwind's
//!   utilities, a component's `@layer components`, or a plain `.card{…}`
//!   from an imported stylesheet;
//! * everything else (layer order, theme variables, the base reset,
//!   element and attribute selectors, `@font-face`) is always sent, once;
//! * `@property` registrations, their `@layer properties` fallbacks and
//!   `@keyframes` are sent once something sent refers to them.
//!
//! Each `<style>` gets the id `<sheet id>~<bitset of pieces>`, so the client
//! runtime can report what it already has and later navigations and
//! streamed chunks send only what is missing. Utilities depend on their
//! order (`px-2` must follow `p-4`), so when new rules are sent, already-sent
//! rules that come after them are sent again to keep the order intact.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::style::Stylesheet;

/// One piece of a split stylesheet.
#[derive(Debug)]
enum Item {
    /// Outside the utilities layer: always sent.
    Text { index: usize, css: String },
    /// A utility rule, sent when one of `owners` is used (always when empty).
    /// `props`: the property families it sets (see [`families`]).
    Rule { index: usize, owners: Vec<String>, css: String, props: Vec<String> },
    /// `@property --x{…}`, a `--x:…;` fallback or `@keyframes x{…}`: sent
    /// when a sent piece refers to `name`.
    Var { index: usize, name: String, css: String },
    /// `@layer utilities{…}` or an at-rule inside it: written only around
    /// pieces that are sent.
    Group { open: String, items: Vec<Item> },
}

#[derive(Debug)]
pub(crate) struct Split {
    items: Vec<Item>,
    len: usize,
    /// For each `Var` piece, the pieces that refer to it.
    refs: Vec<(usize, Vec<usize>)>,
}

/// Pieces of a split stylesheet, as a bit set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Pieces(Vec<u8>);

impl Pieces {
    fn with_len(len: usize) -> Self {
        Pieces(vec![0; len.div_ceil(8)])
    }

    fn insert(&mut self, i: usize) {
        if let Some(b) = self.0.get_mut(i / 8) {
            *b |= 1 << (i % 8);
        }
    }

    fn contains(&self, i: usize) -> bool {
        self.0.get(i / 8).is_some_and(|b| b & (1 << (i % 8)) != 0)
    }

    fn union(&mut self, other: &Pieces) {
        for (a, b) in self.0.iter_mut().zip(&other.0) {
            *a |= b;
        }
    }

    fn is_empty(&self) -> bool {
        self.0.iter().all(|b| *b == 0)
    }

    fn encode(&self) -> String {
        const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut bytes = self.0.as_slice();
        while let [rest @ .., 0] = bytes {
            bytes = rest;
        }
        let mut out = String::with_capacity(bytes.len() * 4 / 3 + 2);
        for chunk in bytes.chunks(3) {
            let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
            for i in 0..=chunk.len() {
                out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
            }
        }
        out
    }

    fn decode(s: &str, len: usize) -> Option<Pieces> {
        let mut bytes = Vec::with_capacity(s.len() * 3 / 4);
        for chunk in s.as_bytes().chunks(4) {
            if chunk.len() == 1 {
                return None;
            }
            let mut n = 0u32;
            for (i, &c) in chunk.iter().enumerate() {
                let v = match c {
                    b'A'..=b'Z' => c - b'A',
                    b'a'..=b'z' => c - b'a' + 26,
                    b'0'..=b'9' => c - b'0' + 52,
                    b'-' => 62,
                    b'_' => 63,
                    _ => return None,
                };
                n |= u32::from(v) << (18 - 6 * i);
            }
            for i in 0..chunk.len() - 1 {
                bytes.push((n >> (16 - 8 * i)) as u8);
            }
        }
        let mut pieces = Pieces::with_len(len);
        for (a, b) in pieces.0.iter_mut().zip(bytes) {
            *a = b;
        }
        Some(pieces)
    }
}

/// Split form of `sheet`, parsed on first use.
pub(crate) fn split(sheet: &'static Stylesheet) -> &'static Split {
    let mut cache = SPLITS.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    // Keyed by address: stylesheets are statics, and ids may repeat in tests.
    cache
        .entry(sheet as *const Stylesheet as usize)
        .or_insert_with(|| (sheet, Box::leak(Box::new(Split::parse(sheet.css)))))
        .1
}

type SplitCache = Mutex<HashMap<usize, (&'static Stylesheet, &'static Split)>>;

/// Every per-class stylesheet rendered so far, split.
static SPLITS: OnceLock<SplitCache> = OnceLock::new();

impl Split {
    fn parse(css: &str) -> Split {
        let mut len = 0;
        let items = parse_rules(css, &mut len, true);
        let refs = references(&items);
        Split { items, len, refs }
    }

    /// The bit set encoded in a known `<style>` id for `sheet`, if it is one.
    pub(crate) fn known(&self, sheet: &Stylesheet, ids: &HashSet<String>) -> Pieces {
        let mut known = Pieces::with_len(self.len);
        let prefix = format!("{}~", sheet.id);
        for id in ids {
            if id == sheet.id {
                // The whole sheet (an older page, or pruning was off).
                (0..self.len).for_each(|i| known.insert(i));
            } else if let Some(bits) = id.strip_prefix(&prefix).and_then(|b| Pieces::decode(b, self.len)) {
                known.union(&bits);
            }
        }
        known
    }

    /// Pieces needed for `used` classes that are not in `sent`, plus sent
    /// rules that must be repeated to keep the order.
    pub(crate) fn delta(&self, used: &dyn Fn(&str) -> bool, sent: &Pieces) -> Pieces {
        let mut wanted = Pieces::with_len(self.len);
        visit(&self.items, &mut |item| match item {
            Item::Text { index, .. } => wanted.insert(*index),
            Item::Rule { index, owners, .. } if owners.is_empty() || owners.iter().any(|o| used(o)) => {
                wanted.insert(*index)
            }
            _ => {}
        });
        for (var, by) in &self.refs {
            if by.iter().any(|p| wanted.contains(*p) || sent.contains(*p)) {
                wanted.insert(*var);
            }
        }
        self.missing(&wanted, sent)
    }

    /// Pieces of `wanted` not in `sent`, plus the sent rules that must be
    /// repeated after them. Invariant kept for the browser: for any two rules
    /// it has that set the same property, the last copies are in the
    /// stylesheet's own order. So a sent rule is repeated only when it
    /// follows (in stylesheet order) a rule being sent that it could conflict with.
    fn missing(&self, wanted: &Pieces, sent: &Pieces) -> Pieces {
        let mut needed = Pieces::with_len(self.len);
        // Property families of the rules sent in this piece, so far.
        let mut sending: HashSet<&str> = HashSet::new();
        visit(&self.items, &mut |item| match item {
            Item::Text { index, .. } | Item::Var { index, .. } if wanted.contains(*index) && !sent.contains(*index) => {
                needed.insert(*index)
            }
            Item::Rule { index, props, .. } => {
                let new = wanted.contains(*index) && !sent.contains(*index);
                let conflicts =
                    || sending.contains("*") || props.iter().any(|p| p == "*" || sending.contains(p.as_str()));
                if new || (sent.contains(*index) && !sending.is_empty() && conflicts()) {
                    needed.insert(*index);
                    sending.extend(props.iter().map(String::as_str));
                }
            }
            _ => {}
        });
        needed
    }

    /// `<style>` element with the given pieces, or nothing when empty.
    pub(crate) fn style(&self, sheet: &Stylesheet, pieces: &Pieces) -> String {
        if pieces.is_empty() {
            return String::new();
        }
        let mut css = String::new();
        write_items(&self.items, pieces, &mut css);
        format!(
            "<style data-nr-css=\"{}~{}\">{}</style>",
            sheet.id,
            pieces.encode(),
            crate::escape::escape_raw_text(&css)
        )
    }

    #[cfg(test)]
    fn empty(&self) -> Pieces {
        Pieces::with_len(self.len)
    }

    pub(crate) fn record(&self, sent: &mut Pieces, pieces: &Pieces) {
        sent.union(pieces);
    }
}

/// Rewrite the per-class `<style>` elements of a rendered document (a cached
/// static page) for a browser that already has the style ids `known`: only
/// what it is missing is kept.
/// Every per-class stylesheet rendered so far (component libraries' too) is
/// considered, besides `sheets`.
pub fn restyle_document(html: &str, sheets: &[&'static Stylesheet], known: &[&str]) -> String {
    let known: HashSet<String> = known.iter().map(|s| (*s).to_owned()).collect();
    let mut all: Vec<&'static Stylesheet> = sheets.iter().map(|s| crate::style::resolve(s)).collect();
    if let Some(cache) = SPLITS.get() {
        for (sheet, _) in cache.lock().unwrap_or_else(|e| e.into_inner()).values() {
            if !all.iter().any(|s| std::ptr::eq(*s, *sheet)) {
                all.push(sheet);
            }
        }
    }
    let mut out = html.to_owned();
    for sheet in all.into_iter().filter(|s| s.per_class.is_some()) {
        let open = format!("<style data-nr-css=\"{}~", sheet.id);
        let Some(at) = out.find(&open) else { continue };
        let bits_start = at + open.len();
        let Some(bits_len) = out[bits_start..].find('"') else { continue };
        let Some(close) = out[at..].find("</style>").map(|c| at + c + "</style>".len()) else { continue };
        let split = split(sheet);
        let Some(page) = Pieces::decode(&out[bits_start..bits_start + bits_len], split.len) else { continue };
        let sent = split.known(sheet, &known);
        let style = split.style(sheet, &split.missing(&page, &sent));
        out.replace_range(at..close, &style);
    }
    out
}

fn visit<'a>(items: &'a [Item], f: &mut dyn FnMut(&'a Item)) {
    for item in items {
        match item {
            Item::Group { items, .. } => visit(items, f),
            other => f(other),
        }
    }
}

/// Write the selected pieces; returns whether anything was written.
fn write_items(items: &[Item], pieces: &Pieces, out: &mut String) -> bool {
    let start = out.len();
    for item in items {
        match item {
            Item::Text { index, css } | Item::Rule { index, css, .. } | Item::Var { index, css, .. } => {
                if pieces.contains(*index) {
                    out.push_str(css);
                }
            }
            Item::Group { open, items } => {
                let at = out.len();
                out.push_str(open);
                if write_items(items, pieces, out) {
                    out.push('}');
                } else {
                    out.truncate(at);
                }
            }
        }
    }
    out.len() > start
}

/// `@layer properties{@supports …{*,…{--tw-x:initial;…}}}`: each custom
/// property fallback becomes a `Var` piece.
fn parse_fallbacks(css: &str, len: &mut usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut rest = css;
    while !rest.trim().is_empty() {
        let (prelude, block, after) = next_block(rest);
        match block {
            Some(body) => {
                items.push(Item::Group { open: format!("{}{{", prelude.trim()), items: parse_fallbacks(body, len) })
            }
            None => {
                let decl = prelude.trim().trim_end_matches(';');
                let name = decl.split_once(':').map(|(n, _)| n.trim()).filter(|n| n.starts_with("--"));
                let css = format!("{decl};");
                items.push(match name {
                    Some(name) => Item::Var { index: *len, name: name.to_owned(), css },
                    None => Item::Text { index: *len, css },
                });
                *len += 1;
            }
        }
        rest = after;
    }
    items
}

/// For each `Var`, the other pieces whose CSS mentions its name.
fn references(items: &[Item]) -> Vec<(usize, Vec<usize>)> {
    let mut vars = Vec::new();
    let mut others = Vec::new();
    visit(items, &mut |item| match item {
        Item::Var { index, name, .. } => vars.push((*index, name.as_str())),
        Item::Text { index, css } | Item::Rule { index, css, .. } => others.push((*index, css.as_str())),
        Item::Group { .. } => {}
    });
    vars.into_iter()
        .map(|(var, name)| (var, others.iter().filter(|(_, css)| mentions(css, name)).map(|(i, _)| *i).collect()))
        .collect()
}

/// Whether `css` contains `name` as a whole identifier.
fn mentions(css: &str, name: &str) -> bool {
    let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
    css.match_indices(name)
        .any(|(at, _)| !ident(css[..at].chars().next_back()) && !ident(css[at + name.len()..].chars().next()))
}

/// Leading `/* … */` comments of a prelude, and the rest.
fn split_comments(prelude: &str) -> (&str, &str) {
    let mut end = 0;
    loop {
        let rest = &prelude[end..];
        let trimmed = rest.trim_start();
        match trimmed.strip_prefix("/*").and_then(|c| c.find("*/")) {
            Some(close) => end += rest.len() - trimmed.len() + 2 + close + 2,
            None => return prelude.split_at(end),
        }
    }
}

/// Every rule of `css` as pieces. A rule whose selectors all start with a
/// class is owned by those classes and sent when one of them is on the
/// page; anything else (element and attribute selectors, `:root`,
/// `@font-face`, statements such as `@import`) is always sent. Grouping
/// at-rules (`@media`, `@supports`, `@layer`, `@container`) are parsed
/// inside; `@property`, `@keyframes` and `@layer properties` become
/// registrations sent when something refers to them. Only a leading
/// comment of the whole sheet (`top`), such as a license, is kept.
fn parse_rules(css: &str, len: &mut usize, top: bool) -> Vec<Item> {
    let mut items = Vec::new();
    let mut rest = css;
    while !rest.trim().is_empty() {
        let (prelude, block, after) = next_block(rest);
        let (comment, prelude) = split_comments(prelude);
        if top && !comment.trim().is_empty() {
            items.push(Item::Text { index: *len, css: comment.trim().to_owned() });
            *len += 1;
        }
        let head = normalize(prelude);
        match block {
            Some(body) if head == "@layer properties" => {
                items.push(Item::Group { open: format!("{}{{", prelude.trim()), items: parse_fallbacks(body, len) });
            }
            Some(body)
                if head.starts_with("@property --")
                    || head.starts_with("@keyframes ")
                    || head.starts_with("@-webkit-keyframes ") =>
            {
                let name = head.split_once(' ').map(|(_, n)| n.trim().to_owned()).unwrap_or_default();
                items.push(Item::Var { index: *len, name, css: format!("{}{{{}}}", prelude.trim(), body) });
                *len += 1;
            }
            Some(body) if head.starts_with('@') && !head.starts_with("@font-face") && !head.starts_with("@page") => {
                items.push(Item::Group { open: format!("{}{{", prelude.trim()), items: parse_rules(body, len, false) });
            }
            Some(body) => {
                items.push(Item::Rule {
                    index: *len,
                    owners: owner_classes(head.as_str()),
                    css: format!("{}{{{}}}", prelude.trim(), body),
                    props: families(body),
                });
                *len += 1;
            }
            None => {
                // A statement (`@import …;`, `@layer a,b;`, `@apply …;`): keep it.
                let css = prelude.trim();
                if !css.is_empty() {
                    items.push(Item::Rule {
                        index: *len,
                        owners: Vec::new(),
                        css: css.to_owned(),
                        props: vec!["*".into()],
                    });
                    *len += 1;
                }
            }
        }
        rest = after;
    }
    items
}

/// Split off the next `prelude{body}` (or `statement;`) from `css`.
/// Returns (prelude, body, rest). Escapes and strings are respected.
fn next_block(css: &str) -> (&str, Option<&str>, &str) {
    let bytes = css.as_bytes();
    let mut i = 0;
    let mut quote = None;
    let mut open = None;
    let mut depth = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        match (quote, b) {
            (_, b'\\') => i += 1,
            (None, b'/') if bytes.get(i + 1) == Some(&b'*') => {
                i += css[i + 2..].find("*/").map_or(bytes.len(), |end| end + 3);
                continue;
            }
            (Some(q), _) if b == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(b),
            (None, b'{') => {
                if depth == 0 {
                    open = Some(i);
                }
                depth += 1;
            }
            (None, b'}') => {
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(o) = open
                {
                    return (&css[..o], Some(&css[o + 1..i]), &css[i + 1..]);
                }
            }
            (None, b';') if depth == 0 => return (&css[..=i], None, &css[i + 1..]),
            _ => {}
        }
        i += 1;
    }
    (css, None, "")
}

/// Owners of a selector list: the first class of each selector. Empty (the
/// rule is always needed) when one of them has no class.
fn owner_classes(selectors: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    let mut chars = selectors.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&selectors[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&selectors[start..]);
    parts.into_iter().map(owner_class).collect::<Option<Vec<_>>>().unwrap_or_default()
}

/// The class a utility selector starts with, unescaped (`.sm\:p-4:hover` → `sm:p-4`).
fn owner_class(selector: &str) -> Option<String> {
    let bytes = selector.as_bytes();
    let dot = (0..bytes.len()).find(|&i| bytes[i] == b'.' && (i == 0 || bytes[i - 1] != b'\\'))?;
    let mut name = String::new();
    let mut chars = selector[dot + 1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => name.push(chars.next()?),
            c if c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii() => name.push(c),
            _ => break,
        }
    }
    (!name.is_empty()).then_some(name)
}

/// Property families a rule body sets, for deciding which rules can
/// override each other: `padding-inline` → `padding`, `-webkit-mask` →
/// `mask`; custom properties by full name. `*` when nothing was recognized.
fn families(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = body;
    while !rest.trim().is_empty() {
        let (prelude, block, after) = next_block(rest);
        match block {
            // A nested rule or at-rule: its declarations count too.
            Some(inner) => {
                for f in families(inner) {
                    if !out.contains(&f) {
                        out.push(f);
                    }
                }
            }
            None => {
                for decl in prelude.split(';') {
                    let Some((name, _)) = decl.split_once(':') else { continue };
                    let name = name.trim().to_ascii_lowercase();
                    let family = if name.starts_with("--") {
                        name
                    } else {
                        let bare =
                            ["-webkit-", "-moz-", "-ms-"].iter().find_map(|v| name.strip_prefix(v)).unwrap_or(&name);
                        match bare {
                            // Shorthands whose longhands are named differently.
                            "all" => "*".to_owned(),
                            "row-gap" | "column-gap" => "gap".to_owned(),
                            "column-count" | "column-width" => "columns".to_owned(),
                            _ => match bare.split('-').next().unwrap_or(bare) {
                                "top" | "right" | "bottom" | "left" => "inset".to_owned(),
                                "align" | "justify" => "place".to_owned(),
                                first => first.to_owned(),
                            },
                        }
                    };
                    if !family.is_empty() && !out.contains(&family) {
                        out.push(family);
                    }
                }
            }
        }
        rest = after;
    }
    if out.is_empty() {
        out.push("*".into());
    }
    out
}

fn normalize(prelude: &str) -> String {
    prelude.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSS: &str = "@layer theme,base,components,utilities;@layer theme{:root{--c:red}}@layer base{*{margin:0}}\
@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}.hover\\:underline{&:hover{@media (hover:hover){text-decoration:underline}}}\
@media (width>=40rem){.sm\\:p-10{padding:2.5rem}.sm\\:w-\\[1\\.5rem\\]{width:1.5rem}}\
:where(.space-y-2>:not(:last-child)){margin-block:.5rem}}@property --tw-x{syntax:\"*\";inherits:false}@keyframes spin{to{rotate:1turn}}";

    static SHEET: Stylesheet = Stylesheet { id: "tw", css: CSS, per_class: Some(&[]), scripts: &[] };

    fn css_of(style: &str) -> &str {
        &style[style.find('>').unwrap() + 1..style.rfind("</style>").unwrap()]
    }

    #[test]
    fn owners() {
        assert_eq!(owner_class(".p-4").as_deref(), Some("p-4"));
        assert_eq!(owner_class(".sm\\:w-\\[1\\.5rem\\]").as_deref(), Some("sm:w-[1.5rem]"));
        assert_eq!(owner_class(":where(.space-y-2>:not(:last-child))").as_deref(), Some("space-y-2"));
        assert_eq!(owner_class(".dark\\:x:where(.dark,.dark *)").as_deref(), Some("dark:x"));
        assert_eq!(owner_class("input"), None);
        assert_eq!(owner_classes(".a:hover,.b\\,c:where(.x,.y)"), ["a", "b,c"]);
        assert!(owner_classes(".a,input").is_empty(), "a part without a class: always needed");
    }

    #[test]
    fn sends_only_used_rules_and_everything_outside_utilities() {
        let split = Split::parse(CSS);
        let used = |c: &str| ["px-2", "sm:p-10"].contains(&c);
        let pieces = split.delta(&used, &split.empty());
        let css = split.style(&SHEET, &pieces);
        assert_eq!(
            css_of(&css),
            "@layer theme,base,components,utilities;@layer theme{:root{--c:red}}@layer base{*{margin:0}}\
@layer utilities{.px-2{padding-inline:.5rem}@media (width>=40rem){.sm\\:p-10{padding:2.5rem}}}",
            "`@property` and `@keyframes` nothing refers to are left out"
        );
        // Every class used: the original stylesheet, minus the registrations
        // nothing refers to.
        let all = split.delta(&|_| true, &split.empty());
        let (rules, _) = CSS.split_once("@property").unwrap();
        assert_eq!(css_of(&split.style(&SHEET, &all)), rules);
    }

    #[test]
    fn property_families() {
        assert_eq!(families("padding-inline:.5rem"), ["padding"]);
        assert_eq!(families("--tw-shadow:0 0;box-shadow:var(--tw-shadow)"), ["--tw-shadow", "box"]);
        assert_eq!(families("&:hover{@media (hover:hover){-webkit-text-decoration:underline}}"), ["text"]);
        assert_eq!(families(""), ["*"]);
        assert_eq!(families("top:0;align-items:center;row-gap:1rem"), ["inset", "place", "gap"]);
    }

    #[test]
    fn unrelated_rules_are_not_repeated() {
        // `.p-4` is new; `.px-2` (same family) is repeated, `.mt-2` isn't.
        let css = "@layer utilities{.p-4{padding:1rem}.mt-2{margin-top:.5rem}.px-2{padding-inline:.5rem}}";
        let split = Split::parse(css);
        let mut sent = split.empty();
        let first = split.delta(&|c| c == "px-2" || c == "mt-2", &sent);
        split.record(&mut sent, &first);
        let next = split.delta(&|c| c == "p-4", &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &next)),
            "@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}}"
        );
    }

    #[test]
    fn registrations_follow_the_rules_that_use_them() {
        let css = "/*! notice */@layer properties{@supports (x:y){*,:before{--tw-a:initial;--tw-b:0}}}\
@layer utilities{.a{--tw-a:1px;width:var(--tw-a)}.b{height:var(--tw-b)}.c{color:red}}\
@property --tw-a{syntax:\"*\";inherits:false}@property --tw-b{syntax:\"*\";inherits:false}@keyframes spin{to{rotate:1turn}}";
        let split = Split::parse(css);
        let mut sent = split.empty();
        let first = split.delta(&|c| c == "a" || c == "c", &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &first)),
            "/*! notice */@layer properties{@supports (x:y){*,:before{--tw-a:initial;}}}\
@layer utilities{.a{--tw-a:1px;width:var(--tw-a)}.c{color:red}}@property --tw-a{syntax:\"*\";inherits:false}"
        );
        split.record(&mut sent, &first);
        let next = split.delta(&|c| c == "b", &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &next)),
            "@layer properties{@supports (x:y){*,:before{--tw-b:0;}}}@layer utilities{.b{height:var(--tw-b)}}\
@property --tw-b{syntax:\"*\";inherits:false}"
        );
        assert!(mentions("a{animation:spin 1s}", "spin") && !mentions("a{animation:spinner}", "spin"));
    }

    #[test]
    fn later_chunks_send_what_is_missing_in_order() {
        let split = Split::parse(CSS);
        let mut sent = split.empty();
        let first = split.delta(&|c| c == "px-2", &sent);
        split.record(&mut sent, &first);
        // `p-4` comes before `px-2`: `px-2` is repeated after it so it still wins.
        let next = split.delta(&|c| c == "p-4" || c == "px-2", &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &next)),
            "@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}}"
        );
        split.record(&mut sent, &next);
        assert!(split.delta(&|c| c == "p-4", &sent).is_empty(), "nothing new");
    }

    #[test]
    fn cached_documents_are_restyled_for_the_browser() {
        let split = Split::parse(CSS);
        let page = split.delta(&|c| c == "p-4" || c == "px-2", &split.empty());
        let html = format!("<head>{}</head>", split.style(&SHEET, &page));
        let had = split.delta(&|c| c == "px-2", &split.empty());
        let had_id = split.style(&SHEET, &had).split('"').nth(1).unwrap().to_owned();
        let out = restyle_document(&html, &[&SHEET], &[&had_id]);
        assert_eq!(
            css_of(&out.replace("<head>", "").replace("</head>", "")),
            "@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}}"
        );
        let all_id = split.style(&SHEET, &page).split('"').nth(1).unwrap().to_owned();
        assert_eq!(restyle_document(&html, &[&SHEET], &[&all_id]), "<head></head>", "nothing missing");
    }

    #[test]
    fn ids_round_trip_through_the_client() {
        let split = Split::parse(CSS);
        let pieces = split.delta(&|c| c == "sm:w-[1.5rem]" || c == "hover:underline", &split.empty());
        let style = split.style(&SHEET, &pieces);
        let id = style.split('"').nth(1).unwrap().to_owned();
        let known = split.known(&SHEET, &HashSet::from([id]));
        assert_eq!(known, pieces);
        let whole = split.known(&SHEET, &HashSet::from(["tw".to_owned()]));
        assert!((0..split.len).all(|i| whole.contains(i)), "the whole sheet");
        assert_eq!(split.known(&SHEET, &HashSet::from(["tw~!!".to_owned()])), split.empty(), "garbage ignored");
    }

    #[test]
    fn plain_rules_are_split_by_their_classes() {
        // Hand-written CSS imported into the build: class rules go out per
        // page; everything without a class owner is always sent.
        let css = "@import url(x.css);:root{--c:red}h1{margin:0}.card{padding:1rem}.card .title{font-weight:700}\
@media (width>=40rem){.card{padding:2rem}.wide{width:100%}}@font-face{font-family:F;src:url(f.woff2)}\
.spin{animation:spin 1s}@keyframes spin{to{rotate:1turn}}[data-theme=dark]{--c:blue}";
        let split = Split::parse(css);
        let used = |c: &str| c == "card";
        let pieces = split.delta(&used, &split.empty());
        assert_eq!(
            css_of(&split.style(&SHEET, &pieces)),
            "@import url(x.css);:root{--c:red}h1{margin:0}.card{padding:1rem}.card .title{font-weight:700}\
@media (width>=40rem){.card{padding:2rem}}@font-face{font-family:F;src:url(f.woff2)}[data-theme=dark]{--c:blue}"
        );
        let spin = split.delta(&|c| c == "spin", &split.empty());
        let out = split.style(&SHEET, &spin);
        assert!(
            out.contains(".spin{animation:spin 1s}@keyframes spin{to{rotate:1turn}}") && !out.contains(".card"),
            "{out}"
        );
        let all = split.delta(&|_| true, &split.empty());
        assert_eq!(css_of(&split.style(&SHEET, &all)), css, "all pieces: the sheet byte for byte");
    }
}
