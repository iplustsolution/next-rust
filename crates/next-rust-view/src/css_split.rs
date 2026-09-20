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
//! * a rule that names an element, an id or an attribute (`details`,
//!   `#menu`, `input[type=checkbox]`, `[data-theme="x"]`) is sent when the
//!   page renders that element, id or attribute, or loads a script that
//!   names it (see [`Page`]);
//! * a custom property (`--x: …` in `:root`, `.dark`, a theme block) is sent
//!   when something sent reads it, directly or through another custom
//!   property, an inline `style`, an inline script or a script the page
//!   loads; a block left without declarations is not written at all;
//! * `@property` registrations, their `@layer properties` fallbacks,
//!   `@keyframes` (named by an `animation` or a custom property) and
//!   `@font-face` (its family named by a `font`/`font-family` or a custom
//!   property) are sent once something sent refers to them;
//! * what is left (layer order, selectors that need nothing such as `*` or
//!   `::selection`) is always sent, once.
//!
//! Each `<style>` gets the id `<sheet id>~<bitset of pieces>`, so the client
//! runtime can report what it already has and later navigations and
//! streamed chunks send only what is missing. Utilities depend on their
//! order (`px-2` must follow `p-4`), so when new rules are sent, already-sent
//! rules that come after them are sent again to keep the order intact.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::style::Stylesheet;

/// What one selector needs on the page before it can match.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Need {
    /// The class the selector starts with.
    Class(String),
    /// An element (`details`), lower case.
    Tag(String),
    /// `#id`.
    Id(String),
    /// `[name]`, or `[name="value"]` for the attributes whose values pages
    /// record (see [`tracks_value`]).
    Attr(String, Option<String>),
}

/// The needs of each selector of a rule; empty when a selector needs nothing
/// (the rule is always sent).
type Alts = Vec<Vec<Need>>;

/// What a [`Item::Var`] piece defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarKind {
    /// `@property --x{…}` or a `--x:…;` fallback in `@layer properties`:
    /// sent when a sent piece names `--x` at all.
    Registration,
    /// A `--x:…;` declaration of a rule: sent when a sent piece reads `--x`.
    Prop,
    /// `@keyframes x{…}`: sent when an `animation` or a custom property names it.
    Keyframes,
    /// `@font-face{…}`: sent when a `font`, `font-family` or custom property
    /// names its family.
    FontFace,
}

/// One piece of a split stylesheet.
#[derive(Debug)]
enum Item {
    /// A comment kept at the top, `@page`: always sent.
    Text { index: usize, css: String },
    /// A rule (or the plain declarations of one), sent when one of `alts` is
    /// met (always when empty). `props`: the property families it sets (see
    /// [`families`]).
    Rule { index: usize, alts: Alts, css: String, props: Vec<String> },
    /// Something sent only when a sent piece refers to `name` (and one of
    /// `alts` is met). `props` is set for declarations, which override each
    /// other like rules do.
    Var { index: usize, kind: VarKind, name: String, keys: Vec<String>, alts: Alts, css: String, props: Vec<String> },
    /// No CSS: a bit in the `<style>` id that says the page had `need`. A
    /// client navigation renders only the region that changes, so what the
    /// rest of the document has comes back this way, with the ids the
    /// browser reports.
    Fact { index: usize, need: Need },
    /// An at-rule (`@layer utilities{…}`) or a rule split into declarations
    /// (`decls`): written only around pieces that are sent.
    Group { open: String, decls: bool, items: Vec<Item> },
}

#[derive(Debug)]
pub(crate) struct Split {
    items: Vec<Item>,
    len: usize,
    /// For each piece, the `Var` pieces it refers to.
    uses: Vec<Vec<usize>>,
    /// Every `var(--x)` this sheet reads. A document sends several
    /// stylesheets, and one may define what another reads (an app's
    /// `:root{--nr-fg:var(--fg)}` for the UI components' sheet), so each
    /// sheet keeps what the others read.
    reads: HashSet<String>,
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

    pub(crate) fn union(&mut self, other: &Pieces) {
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

/// Names that scripts of the framework itself set or read: attributes,
/// elements, ids and custom properties that may appear without being in the
/// markup. `always` applies to every page, `ui` to pages that render a UI
/// component (or may: client navigations).
#[derive(Debug, Default)]
struct DynamicNames {
    always: HashSet<String>,
    ui: HashSet<String>,
}

static DYNAMIC: OnceLock<DynamicNames> = OnceLock::new();

/// Tell the per-page stylesheets which names the framework's own scripts use
/// (`runtime`: on every page; `ui`: on pages with UI components), so rules for
/// an attribute, element or custom property those scripts set stay on the
/// pages that load them. Set once at startup; later calls are ignored.
pub fn set_framework_scripts(runtime: &str, ui: &str) {
    let _ = DYNAMIC.get_or_init(|| {
        let mut names = DynamicNames::default();
        script_words(runtime, &mut |w| {
            names.always.insert(w.to_owned());
        });
        script_words(ui, &mut |w| {
            names.ui.insert(w.to_owned());
        });
        names
    });
}

/// The names each script of the project uses, from the stylesheet that lists
/// them (the Tailwind build): stylesheets without a list of their own
/// (`global_css!`, the UI components') are sent by it.
static SCRIPT_NAMES: OnceLock<ScriptNames> = OnceLock::new();

type ScriptNames = &'static [(&'static str, &'static [&'static str])];

/// Set once at startup, from the app's stylesheets; later calls are ignored.
pub fn set_script_names(sheets: &[&'static Stylesheet]) {
    if let Some(sheet) = sheets.iter().find(|s| !s.scripts.is_empty()) {
        let _ = SCRIPT_NAMES.set(sheet.scripts);
    }
}

/// The names the scripts of a page use, for `sheet`. `None`: nothing lists
/// them, so a page that loads a script may get anything from it.
pub(crate) fn script_names(sheet: &Stylesheet) -> Option<ScriptNames> {
    if sheet.scripts.is_empty() { SCRIPT_NAMES.get().copied() } else { Some(sheet.scripts) }
}

/// A fact about the page, stored next to its classes: a class never contains
/// a space, so these cannot collide with one.
fn fact(kind: char, name: &str) -> String {
    let mut key = String::with_capacity(name.len() + 2);
    key.push(' ');
    key.push(kind);
    key.push_str(name);
    key
}

fn note(into: &mut HashSet<String>, key: String) {
    if !into.contains(&key) {
        into.insert(key);
    }
}

/// Attributes whose values pages record, so `[type="checkbox"]` or
/// `[data-theme="dark"]` is sent only with a page that has that value.
/// Other attributes (`href`, `title`, …) are matched by name only.
fn tracks_value(name: &str) -> bool {
    name.starts_with("data-") || matches!(name, "type" | "role" | "dir" | "lang" | "rel" | "target" | "method" | "wrap")
}

/// SVG presentation attributes whose value is a CSS value: it may name a
/// font family, an animation or a custom property.
fn is_css_attr(name: &str) -> bool {
    matches!(
        name,
        "font-family"
            | "font"
            | "font-size"
            | "font-weight"
            | "fill"
            | "stroke"
            | "filter"
            | "mask"
            | "clip-path"
            | "color"
            | "stop-color"
            | "flood-color"
            | "lighting-color"
            | "marker-start"
            | "marker-mid"
            | "marker-end"
            | "offset-path"
    )
}

/// Attributes that change without any script the page is known to load: the
/// browser sets `open`, and ARIA states and the framework's own `data-nr-*`
/// attributes are set by the client runtime.
fn volatile_attr(name: &str) -> bool {
    name == "open" || name.starts_with("aria-") || name.starts_with("data-nr-") || name.starts_with("nr-")
}

/// Record an attribute as written to the HTML.
pub(crate) fn note_attr(name: &str, value: Option<&str>, into: &mut HashSet<String>) {
    let name = name.to_ascii_lowercase();
    if name == "class" {
        return;
    }
    if let Some(value) = value {
        match name.as_str() {
            "id" => note(into, fact('i', value)),
            // `style="--fill:.4; animation: pulse 1s"` names custom
            // properties, keyframes and fonts, and so does an SVG
            // presentation attribute (`font-family="var(--mono), monospace"`).
            _ if name == "style" || is_css_attr(&name) || value.contains("var(--") => {
                css_words(value, &mut |w| note(into, fact('w', w)))
            }
            _ => {}
        }
        if tracks_value(&name) && value.len() <= 64 {
            note(into, fact('v', &format!("{name}={value}")));
        }
    }
    note(into, fact('a', &name));
}

/// Record an element as written to the HTML.
pub(crate) fn note_tag(tag: &str, into: &mut HashSet<String>) {
    note(into, fact('t', &tag.to_ascii_lowercase()));
}

/// Record the names in an inline `<script>` or `<style>`.
pub(crate) fn note_text(text: &str, into: &mut HashSet<String>) {
    // Props of an island can be large; the scripts that matter are small.
    let text = if text.len() > 65_536 { &text[..floor_boundary(text, 65_536)] } else { text };
    script_words(text, &mut |w| note(into, fact('w', w)));
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Record the elements, ids, attributes, inline styles and inline scripts of
/// raw HTML (`raw_html`, icons, the document head).
pub(crate) fn note_html(html: &str, into: &mut HashSet<String>) {
    let bytes = html.as_bytes();
    let mut i = 0;
    while let Some(lt) = html[i..].find('<').map(|p| i + p) {
        i = lt + 1;
        if !bytes.get(i).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        let name_end =
            html[i..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).map_or(bytes.len(), |p| i + p);
        let tag = html[i..name_end].to_ascii_lowercase();
        note_tag(&tag, into);
        i = name_end;
        // Attributes, up to the `>` that closes the tag.
        loop {
            while bytes.get(i).is_some_and(|b| b.is_ascii_whitespace() || *b == b'/') {
                i += 1;
            }
            match bytes.get(i) {
                None => return,
                Some(b'>') => {
                    i += 1;
                    break;
                }
                _ => {}
            }
            let start = i;
            while bytes.get(i).is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b'=' | b'>' | b'/')) {
                i += 1;
            }
            if i == start {
                i += 1;
                continue;
            }
            let name = &html[start..i];
            let mut value = None;
            if bytes.get(i) == Some(&b'=') {
                i += 1;
                match bytes.get(i).copied() {
                    Some(q @ (b'"' | b'\'')) => {
                        let end = html[i + 1..].find(q as char).map_or(bytes.len(), |p| i + 1 + p);
                        value = Some(&html[i + 1..end]);
                        i = (end + 1).min(bytes.len());
                    }
                    _ => {
                        let end = html[i..]
                            .find(|c: char| c.is_ascii_whitespace() || c == '>')
                            .map_or(bytes.len(), |p| i + p);
                        value = Some(&html[i..end]);
                        i = end;
                    }
                }
            }
            note_attr(name, value.or(Some("")), into);
        }
        if tag == "script" || tag == "style" {
            let close = if tag == "script" { "</script" } else { "</style" };
            let end = html[i..].to_ascii_lowercase().find(close).map_or(bytes.len(), |p| i + p);
            note_text(&html[i..end], into);
            i = end;
        }
    }
}

/// The names in CSS text: `--fill`, `pulse`, `Inter`.
fn css_words(text: &str, f: &mut dyn FnMut(&str)) {
    for word in text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')) {
        if !word.is_empty() && word.len() <= 64 {
            f(word);
        }
    }
}

/// The names a script may set or read: every word, `data-foo-bar` for
/// `dataset.fooBar`, and `<tag` for an element it creates
/// (`createElement("dialog")`, `"<dialog>"` in a string; `<*` when the
/// element is not a literal).
fn script_words(text: &str, f: &mut dyn FnMut(&str)) {
    css_words(text, f);
    for (at, _) in text.match_indices("dataset.") {
        let rest = &text[at + "dataset.".len()..];
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')).unwrap_or(rest.len());
        if end == 0 || end > 64 {
            continue;
        }
        let mut name = String::from("data-");
        for c in rest[..end].chars() {
            if c.is_ascii_uppercase() {
                name.push('-');
            }
            name.push(c.to_ascii_lowercase());
        }
        f(&name);
    }
    let tag_end = |s: &str| s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).unwrap_or(s.len());
    for (at, _) in text.match_indices('<') {
        let rest = &text[at + 1..];
        let end = tag_end(rest);
        if end > 0 && end <= 64 && rest.as_bytes()[0].is_ascii_alphabetic() {
            f(&format!("<{}", rest[..end].to_ascii_lowercase()));
        }
    }
    for (at, m) in text.match_indices("createElement") {
        // `createElement("x")`, `createElementNS(ns, "x")`.
        let rest = &text[at + m.len()..];
        let args = rest.strip_prefix("NS").unwrap_or(rest).trim_start();
        let Some(args) = args.strip_prefix('(') else { continue };
        let args = &args[..args.find(')').unwrap_or(args.len())];
        let last = args.rsplit(',').next().unwrap_or(args).trim();
        let literal = last.strip_prefix(['"', '\'', '`']).filter(|l| l.len() > 1 && l.ends_with(['"', '\'', '`']));
        match literal {
            Some(l) if tag_end(&l[..l.len() - 1]) == l.len() - 1 => {
                f(&format!("<{}", l[..l.len() - 1].to_ascii_lowercase()))
            }
            _ => f("<*"),
        }
    }
}

/// What is known about the page being sent.
pub(crate) struct Page<'a> {
    /// Classes rendered so far, and the facts recorded with them
    /// ([`note_tag`], [`note_attr`], [`note_text`], [`note_html`]).
    facts: &'a HashSet<String>,
    /// Names that count everywhere: `keep_classes`, the safelists, and what
    /// the scripts this page loads name (classes, attributes, elements, ids,
    /// custom properties).
    keep: HashSet<&'a str>,
    /// The page loads scripts nothing is known about: any name may be theirs.
    unknown_scripts: bool,
    /// What the browser's document had before (see [`Item::Fact`]).
    before: HashSet<&'a Need>,
    dynamic: Option<&'static DynamicNames>,
    ui: bool,
}

impl<'a> Page<'a> {
    pub(crate) fn new(facts: &'a HashSet<String>, keep: impl IntoIterator<Item = &'a str>) -> Self {
        let ui = facts.contains(" adata-nr-ui");
        Page {
            facts,
            keep: keep.into_iter().collect(),
            unknown_scripts: false,
            before: HashSet::new(),
            dynamic: DYNAMIC.get(),
            ui,
        }
    }

    /// The page loads scripts whose names are not listed anywhere.
    pub(crate) fn with_unknown_scripts(mut self, unknown: bool) -> Self {
        self.unknown_scripts = unknown;
        self
    }

    fn class(&self, class: &str) -> bool {
        self.facts.contains(class) || self.keep.contains(class)
    }

    /// A name some script of the page may set, or that is kept everywhere.
    fn word(&self, word: &str) -> bool {
        self.unknown_scripts
            || self.keep.contains(word)
            || self.dynamic.is_some_and(|d| d.always.contains(word) || (self.ui && d.ui.contains(word)))
            || self.facts.contains(&fact('w', word))
    }

    fn has(&self, need: &Need) -> bool {
        match need {
            Need::Class(c) => self.class(c),
            // An element a script creates: `<tag`, or `<*` for any.
            Need::Tag(t) => self.facts.contains(&fact('t', t)) || self.word(&format!("<{t}")) || self.word("<*"),
            Need::Id(i) => self.facts.contains(&fact('i', i)) || self.word(i),
            Need::Attr(name, value) => {
                // `!name` (`[assets] css_static_attributes`): only what the
                // server renders counts, whatever scripts do.
                let fixed = self.keep.contains(format!("!{name}").as_str());
                if !fixed && (volatile_attr(name) || self.word(name)) {
                    return true;
                }
                match value {
                    Some(v) => self.facts.contains(&fact('v', &format!("{name}={v}"))),
                    None => self.facts.contains(&fact('a', name)),
                }
            }
        }
    }

    fn meets(&self, alts: &Alts) -> bool {
        alts.is_empty() || alts.iter().any(|needs| needs.iter().all(|n| self.before.contains(n) || self.has(n)))
    }

    /// Whether an inline style, an inline script, a loaded script or a
    /// safelist names what a `Var` piece defines.
    fn names(&self, name: &str, keys: &[String]) -> bool {
        self.unknown_scripts
            || self.keep.contains(name)
            || self.dynamic.is_some_and(|d| d.always.contains(name) || (self.ui && d.ui.contains(name)))
            || (!keys.is_empty() && keys.iter().all(|k| self.facts.contains(k)))
    }
}

impl Split {
    fn parse(css: &str) -> Split {
        let mut len = 0;
        let mut items = parse_rules(css, &mut len, true);
        // A fact for each need that decides together with something else: a
        // selector with several needs, or a declaration sent when it is read.
        let mut needs: Vec<&Need> = Vec::new();
        let mut seen: HashSet<&Need> = HashSet::new();
        visit(&items, &mut |item| {
            let (alts, var) = match item {
                Item::Rule { alts, .. } => (alts, false),
                Item::Var { alts, .. } => (alts, true),
                _ => return,
            };
            for need in alts.iter().filter(|alt| var || alt.len() > 1).flatten() {
                if seen.insert(need) {
                    needs.push(need);
                }
            }
        });
        let mut needs: Vec<Need> = needs.into_iter().cloned().collect();
        needs.push(Need::Attr("data-nr-ui".into(), None));
        let facts: Vec<Item> = needs
            .into_iter()
            .map(|need| {
                len += 1;
                Item::Fact { index: len - 1, need }
            })
            .collect();
        items.extend(facts);
        let uses = references(&items, len);
        let mut reads = HashSet::new();
        for (at, _) in css.match_indices("var(") {
            let rest = css[at + 4..].trim_start();
            let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')).unwrap_or(rest.len());
            if rest.starts_with("--") && end > 2 && end <= 64 {
                reads.insert(rest[..end].to_owned());
            }
        }
        Split { items, len, uses, reads }
    }

    /// The custom properties this sheet reads, for the other sheets of the
    /// document.
    pub(crate) fn reads(&self) -> impl Iterator<Item = &str> {
        self.reads.iter().map(String::as_str)
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

    /// Pieces `page` needs that are not in `sent`, plus sent rules that must
    /// be repeated to keep the order.
    pub(crate) fn delta<'a>(&'a self, page: &Page<'a>, sent: &Pieces) -> Pieces {
        let mut wanted = Pieces::with_len(self.len);
        // What the document had when the browser got its styles: the parts a
        // client navigation leaves on screen are not rendered again.
        let mut before: HashSet<&Need> = HashSet::new();
        for item in &self.items {
            if let Item::Fact { index, need } = item {
                if sent.contains(*index) {
                    before.insert(need);
                } else if page.has(need) {
                    wanted.insert(*index);
                }
            }
        }
        let ui = page.ui || before.contains(&Need::Attr("data-nr-ui".into(), None));
        let page = &Page {
            facts: page.facts,
            keep: page.keep.clone(),
            unknown_scripts: page.unknown_scripts,
            before,
            dynamic: page.dynamic,
            ui,
        };
        // Pieces whose references are still to be followed.
        let mut todo: Vec<usize> = Vec::new();
        let mut vars: Vec<Option<&Item>> = vec![None; self.len];
        visit(&self.items, &mut |item| match item {
            Item::Text { index, .. } => {
                wanted.insert(*index);
                todo.push(*index);
            }
            Item::Rule { index, alts, .. } => {
                if page.meets(alts) {
                    wanted.insert(*index);
                    todo.push(*index);
                } else if sent.contains(*index) {
                    todo.push(*index);
                }
            }
            Item::Var { index, name, keys, alts, .. } => {
                vars[*index] = Some(item);
                if sent.contains(*index) {
                    todo.push(*index);
                } else if page.names(name, keys) && page.meets(alts) {
                    wanted.insert(*index);
                    todo.push(*index);
                }
            }
            Item::Group { .. } | Item::Fact { .. } => {}
        });
        // Follow references: a custom property read by a custom property
        // that is sent, keyframes named by one, and so on.
        while let Some(piece) = todo.pop() {
            for &var in &self.uses[piece] {
                if wanted.contains(var) || sent.contains(var) {
                    continue;
                }
                if let Some(Item::Var { alts, .. }) = vars[var]
                    && page.meets(alts)
                {
                    wanted.insert(var);
                    todo.push(var);
                }
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
            Item::Rule { index, props, .. } | Item::Var { index, props, .. } if !props.is_empty() => {
                let new = wanted.contains(*index) && !sent.contains(*index);
                let conflicts =
                    || sending.contains("*") || props.iter().any(|p| p == "*" || sending.contains(p.as_str()));
                if new || (sent.contains(*index) && !sending.is_empty() && conflicts()) {
                    needed.insert(*index);
                    sending.extend(props.iter().map(String::as_str));
                }
            }
            Item::Text { index, .. } | Item::Var { index, .. } | Item::Fact { index, .. }
                if wanted.contains(*index) && !sent.contains(*index) =>
            {
                needed.insert(*index)
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
/// Neighbouring groups with the same opening (`:root{…}` twice once what was
/// between them is left out) are written as one.
fn write_items(items: &[Item], pieces: &Pieces, out: &mut String) -> bool {
    let start = out.len();
    // The group written last, when nothing was written after it.
    let mut last: Option<(&str, usize)> = None;
    for item in items {
        match item {
            Item::Text { index, css } | Item::Rule { index, css, .. } | Item::Var { index, css, .. } => {
                if pieces.contains(*index) {
                    out.push_str(css);
                }
            }
            Item::Fact { .. } => {}
            Item::Group { open, decls, items } => {
                let mut inner = String::new();
                if !write_items(items, pieces, &mut inner) {
                    continue;
                }
                if *decls && inner.ends_with(';') {
                    inner.pop();
                }
                if last.is_some_and(|(o, end)| o == open && end == out.len()) {
                    out.pop();
                    if *decls {
                        out.push(';');
                    }
                } else {
                    out.push_str(open);
                }
                out.push_str(&inner);
                out.push('}');
                last = Some((open, out.len()));
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
                let decls = !body.contains('{');
                items.push(Item::Group {
                    open: format!("{}{{", prelude.trim()),
                    decls,
                    items: parse_fallbacks(body, len),
                })
            }
            None => {
                let decl = prelude.trim().trim_end_matches(';');
                let name = decl.split_once(':').map(|(n, _)| n.trim()).filter(|n| n.starts_with("--"));
                let css = format!("{decl};");
                items.push(match name {
                    Some(name) => Item::Var {
                        index: *len,
                        kind: VarKind::Registration,
                        name: name.to_owned(),
                        keys: vec![fact('w', name)],
                        alts: Vec::new(),
                        css,
                        props: Vec::new(),
                    },
                    None => Item::Text { index: *len, css },
                });
                *len += 1;
            }
        }
        rest = after;
    }
    items
}

/// For each piece, the `Var` pieces it refers to.
fn references(items: &[Item], len: usize) -> Vec<Vec<usize>> {
    struct Piece<'a> {
        index: usize,
        css: &'a str,
        /// For a declaration: what it declares, which is not a reference.
        declares: Option<&'a str>,
    }
    let mut vars: Vec<(usize, VarKind, &str)> = Vec::new();
    let mut pieces: Vec<Piece> = Vec::new();
    visit(items, &mut |item| match item {
        Item::Var { index, kind, name, css, .. } => {
            vars.push((*index, *kind, name.as_str()));
            let declares = matches!(kind, VarKind::Prop | VarKind::Registration).then_some(name.as_str());
            pieces.push(Piece { index: *index, css, declares });
        }
        Item::Text { index, css } | Item::Rule { index, css, .. } => {
            pieces.push(Piece { index: *index, css, declares: None })
        }
        Item::Group { .. } | Item::Fact { .. } => {}
    });
    // The pieces each word appears in, so a name is looked up, not searched.
    let mut by_word: HashMap<String, Vec<usize>> = HashMap::new();
    for (at, piece) in pieces.iter().enumerate() {
        css_words(&piece.css.to_ascii_lowercase(), &mut |w| {
            let list = by_word.entry(w.to_owned()).or_default();
            if list.last() != Some(&at) {
                list.push(at);
            }
        });
    }
    let mut uses = vec![Vec::new(); len];
    for (var, kind, name) in vars {
        let lower = name.to_ascii_lowercase();
        let first = lower.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')).find(|w| !w.is_empty());
        let Some(candidates) = first.and_then(|w| by_word.get(w)) else { continue };
        for piece in candidates.iter().map(|at| &pieces[*at]) {
            if piece.index == var {
                continue;
            }
            // The value of a declaration; all of anything else.
            let css = match piece.declares {
                Some(_) => piece.css.split_once(':').map_or("", |(_, value)| value),
                None => piece.css,
            };
            let refers = match kind {
                VarKind::Registration => mentions(piece.css, name),
                VarKind::Prop => mentions(css, name),
                VarKind::Keyframes => match piece.declares {
                    Some(_) => mentions(css, name),
                    None => named_in(piece.css, name, false, &|p| p.contains("animation") || p.starts_with("--")),
                },
                VarKind::FontFace => match piece.declares {
                    Some(_) => mentions(&css.to_ascii_lowercase(), &lower),
                    None => named_in(piece.css, name, true, &|p| p.contains("font") || p.starts_with("--")),
                },
            };
            if refers {
                uses[piece.index].push(var);
            }
        }
    }
    uses
}

/// Whether `css` contains `name` as a whole identifier.
fn mentions(css: &str, name: &str) -> bool {
    let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
    css.match_indices(name)
        .any(|(at, _)| !ident(css[..at].chars().next_back()) && !ident(css[at + name.len()..].chars().next()))
}

/// Whether `css` names `name` in the value of a declaration whose property
/// `property` accepts (`animation: spin 1s`, `--animate-spin: spin 1s`).
fn named_in(css: &str, name: &str, ignore_case: bool, property: &dyn Fn(&str) -> bool) -> bool {
    let (css, name) = if ignore_case {
        (css.to_ascii_lowercase(), name.to_ascii_lowercase())
    } else {
        (css.to_owned(), name.to_owned())
    };
    let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
    css.match_indices(&name).any(|(at, _)| {
        if ident(css[..at].chars().next_back()) || ident(css[at + name.len()..].chars().next()) {
            return false;
        }
        let start = css[..at].rfind([';', '{']).map_or(0, |p| p + 1);
        css[start..at].split_once(':').is_some_and(|(p, _)| property(p.trim().to_ascii_lowercase().as_str()))
    })
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

/// Every rule of `css` as pieces. A rule is sent when the page has what one
/// of its selectors needs ([`selector_needs`]); a rule that is a list of
/// declarations with custom properties among them is split, so each custom
/// property goes out only when something reads it. Statements (`@import`,
/// `@layer a,b;`) are always sent. Grouping at-rules (`@media`, `@supports`,
/// `@layer`, `@container`) are parsed inside; `@property`, `@keyframes`,
/// `@font-face` and `@layer properties` become registrations sent when
/// something refers to them. Of two identical rules next to each other in
/// the cascade only the last is kept. Only a leading comment of the whole
/// sheet (`top`), such as a license, is kept.
fn parse_rules(css: &str, len: &mut usize, top: bool) -> Vec<Item> {
    let mut blocks = Vec::new();
    let mut rest = css;
    while !rest.trim().is_empty() {
        let (prelude, block, after) = next_block(rest);
        blocks.push((prelude, block));
        rest = after;
    }
    let mut items = Vec::new();
    for (at, (prelude, block)) in blocks.iter().enumerate() {
        let (comment, prelude) = split_comments(prelude);
        if top && !comment.trim().is_empty() {
            items.push(Item::Text { index: *len, css: comment.trim().to_owned() });
            *len += 1;
        }
        let head = normalize(prelude);
        // The same rule again later in this block: the later one decides.
        if block.is_some_and(|b| {
            blocks[at + 1..].iter().any(|(p, o)| *o == Some(b) && normalize(split_comments(p).1) == head)
        }) {
            continue;
        }
        let var = |kind: VarKind, name: String, body: &str, len: &mut usize| {
            let keys = name.split_whitespace().map(|w| fact('w', w)).collect();
            let item = Item::Var {
                index: *len,
                kind,
                name,
                keys,
                alts: Vec::new(),
                css: format!("{}{{{}}}", prelude.trim(), body),
                props: Vec::new(),
            };
            *len += 1;
            item
        };
        match *block {
            Some(body) if head == "@layer properties" => {
                items.push(Item::Group {
                    open: format!("{}{{", prelude.trim()),
                    decls: false,
                    items: parse_fallbacks(body, len),
                });
            }
            Some(body) if head.starts_with("@property --") => {
                let name = head.split_once(' ').map(|(_, n)| n.trim().to_owned()).unwrap_or_default();
                items.push(var(VarKind::Registration, name, body, len));
            }
            Some(body) if head.starts_with("@keyframes ") || head.starts_with("@-webkit-keyframes ") => {
                let name = head.split_once(' ').map(|(_, n)| n.trim().trim_matches(['"', '\'']).to_owned());
                items.push(var(VarKind::Keyframes, name.unwrap_or_default(), body, len));
            }
            Some(body) if head.starts_with("@font-face") => match font_family(body) {
                Some(family) => items.push(var(VarKind::FontFace, family, body, len)),
                None => {
                    items.push(Item::Text { index: *len, css: format!("{}{{{}}}", prelude.trim(), body) });
                    *len += 1;
                }
            },
            Some(body) if head.starts_with("@page") => {
                items.push(Item::Text { index: *len, css: format!("{}{{{}}}", prelude.trim(), body) });
                *len += 1;
            }
            Some(body) if head.starts_with('@') => {
                items.push(Item::Group {
                    open: format!("{}{{", prelude.trim()),
                    decls: false,
                    items: parse_rules(body, len, false),
                });
            }
            Some(body) => {
                let alts = selector_alts(head.as_str());
                match split_declarations(body, &alts) {
                    Some(parts) => {
                        let mut inner = Vec::new();
                        for (name, css) in parts {
                            let props = families(&css);
                            inner.push(match name {
                                Some(name) => Item::Var {
                                    index: *len,
                                    kind: VarKind::Prop,
                                    keys: vec![fact('w', &name)],
                                    name,
                                    alts: alts.clone(),
                                    css,
                                    props,
                                },
                                None => Item::Rule { index: *len, alts: alts.clone(), css, props },
                            });
                            *len += 1;
                        }
                        items.push(Item::Group { open: format!("{}{{", prelude.trim()), decls: true, items: inner });
                    }
                    None => {
                        items.push(Item::Rule {
                            index: *len,
                            alts,
                            css: format!("{}{{{}}}", prelude.trim(), body),
                            props: families(body),
                        });
                        *len += 1;
                    }
                }
            }
            None => {
                // A statement (`@import …;`, `@layer a,b;`, `@apply …;`): keep it.
                let css = prelude.trim();
                if !css.is_empty() {
                    items.push(Item::Rule {
                        index: *len,
                        alts: Vec::new(),
                        css: css.to_owned(),
                        props: vec!["*".into()],
                    });
                    *len += 1;
                }
            }
        }
    }
    items
}

/// The family an `@font-face` body declares, without quotes.
fn font_family(body: &str) -> Option<String> {
    let value = declarations(body).into_iter().find_map(|d| {
        let (name, value) = d.split_once(':')?;
        name.trim().eq_ignore_ascii_case("font-family").then_some(value)
    })?;
    let family = value.trim().trim_matches(['"', '\'']).trim();
    (!family.is_empty()).then(|| family.to_owned())
}

/// The declarations of a rule body without nested rules, split at `;`
/// outside strings and parentheses.
fn declarations(body: &str) -> Vec<&str> {
    let bytes = body.as_bytes();
    let (mut out, mut start, mut depth, mut quote, mut i) = (Vec::new(), 0, 0usize, None, 0);
    while i < bytes.len() {
        match (quote, bytes[i]) {
            (_, b'\\') => i += 1,
            (Some(q), b) if b == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(bytes[i]),
            (None, b'(') => depth += 1,
            (None, b')') => depth = depth.saturating_sub(1),
            (None, b';') if depth == 0 => {
                out.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&body[start.min(body.len())..]);
    out.into_iter().map(str::trim).filter(|d| !d.is_empty()).collect()
}

/// A rule body as pieces: each custom property on its own (with its name),
/// the declarations between them together. `None` when the rule stays whole:
/// it nests rules, declares no custom property, or is a class's rule that
/// sets a couple of them for its own declarations
/// (`.shadow{--tw-shadow:…;box-shadow:…}` reads what it sets).
fn split_declarations(body: &str, alts: &Alts) -> Option<Vec<(Option<String>, String)>> {
    if body.contains('{') {
        return None;
    }
    let decls = declarations(body);
    let custom = decls.iter().filter(|d| d.starts_with("--")).count();
    let of_a_class = alts.iter().any(|needs| needs.iter().any(|n| matches!(n, Need::Class(_))));
    if custom == 0 || (of_a_class && custom < 3 && custom < decls.len()) {
        return None;
    }
    let mut parts: Vec<(Option<String>, String)> = Vec::new();
    for decl in decls {
        let name = decl.split_once(':').map(|(n, _)| n.trim()).filter(|n| n.starts_with("--"));
        match (name, parts.last_mut()) {
            (None, Some((None, css))) => {
                css.push_str(decl);
                css.push(';');
            }
            _ => parts.push((name.map(str::to_owned), format!("{decl};"))),
        }
    }
    Some(parts)
}

/// Elements every document has, whatever the page renders.
const ALWAYS_TAGS: &[&str] = &["html", "body", "head"];

/// What each selector of a list needs. Empty when one of them needs
/// nothing: the rule is always sent.
fn selector_alts(selectors: &str) -> Alts {
    let alts: Alts = split_selectors(selectors).into_iter().map(selector_needs).collect();
    if alts.iter().any(Vec::is_empty) { Vec::new() } else { alts }
}

/// What a selector needs on the page: the class it starts with, and the
/// elements, ids and attributes outside parentheses (`:not(…)`, `:is(…)`,
/// `:has(…)` list alternatives, so what is inside is not required). A
/// selector that is one `:where(…)`/`:is(…)` around a single selector is
/// read as that selector.
fn selector_needs(selector: &str) -> Vec<Need> {
    let mut selector = selector.trim();
    for wrapper in [":where(", ":is("] {
        if let Some(inner) = selector.strip_prefix(wrapper).and_then(|s| s.strip_suffix(')'))
            && split_selectors(inner).len() == 1
            && balanced(inner)
        {
            selector = inner.trim();
        }
    }
    let mut needs: Vec<Need> = owner_class(selector).map(Need::Class).into_iter().collect();
    let bytes = selector.as_bytes();
    let ident_end = |from: usize| {
        let mut i = from;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || !b.is_ascii() => i += 1,
                _ => break,
            }
        }
        i.min(bytes.len())
    };
    let (mut i, mut compound_start) = (0, true);
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'(' => {
                // Skip to the matching `)`.
                let mut depth = 0usize;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' => i += 1,
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
            }
            b'[' => {
                let end = selector[i..].find(']').map_or(bytes.len(), |p| i + p);
                let inside = &selector[i + 1..end];
                let name_end =
                    inside.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(inside.len());
                let name = inside[..name_end].to_ascii_lowercase();
                if !name.is_empty() {
                    let value = inside[name_end..]
                        .strip_prefix('=')
                        .filter(|_| tracks_value(&name))
                        .map(|v| v.trim().trim_matches(['"', '\'']))
                        // `[type=a i]` and escapes: by name only.
                        .filter(|v| !v.contains([' ', '\\', '"', '\'']) && v.len() <= 64);
                    needs.push(Need::Attr(name, value.map(str::to_owned)));
                }
                compound_start = false;
                i = end + 1;
            }
            b'.' => {
                compound_start = false;
                i = ident_end(i + 1);
            }
            b'#' => {
                let end = ident_end(i + 1);
                let id = selector[i + 1..end].replace('\\', "");
                if !id.is_empty() {
                    needs.push(Need::Id(id));
                }
                compound_start = false;
                i = end;
            }
            b':' => {
                let start = i + selector[i..].bytes().take_while(|b| *b == b':').count();
                let end = ident_end(start);
                if let Some(tag) = pseudo_element_tag(&selector[start..end]) {
                    needs.push(Need::Tag(tag.to_owned()));
                }
                compound_start = false;
                i = end.max(i + 1);
            }
            b' ' | b'>' | b'+' | b'~' => {
                compound_start = true;
                i += 1;
            }
            b if compound_start && (b.is_ascii_alphabetic() || !b.is_ascii()) => {
                let end = ident_end(i);
                let tag = selector[i..end].to_ascii_lowercase();
                if !ALWAYS_TAGS.contains(&tag.as_str()) {
                    needs.push(Need::Tag(tag));
                }
                compound_start = false;
                i = end;
            }
            _ => {
                compound_start = false;
                i += 1;
            }
        }
    }
    needs
}

fn balanced(text: &str) -> bool {
    let mut depth = 0i32;
    for b in text.bytes() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// The element a pseudo-element belongs to, when only one has it: the
/// browser's parts of form controls.
fn pseudo_element_tag(pseudo: &str) -> Option<&'static str> {
    let p = pseudo.to_ascii_lowercase();
    let input = [
        "file-selector-button",
        "-webkit-file-upload-button",
        "-webkit-datetime-edit",
        "-webkit-date-and-time-value",
        "-webkit-calendar-picker-indicator",
        "-webkit-inner-spin-button",
        "-webkit-outer-spin-button",
        "-webkit-search-",
        "-webkit-slider-",
        "-moz-range-",
        "-webkit-color-swatch",
        "-moz-color-swatch",
    ];
    if input.iter().any(|name| p.starts_with(name)) {
        return Some("input");
    }
    (p == "-webkit-details-marker").then_some("summary")
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

/// The selectors of a list, split at `,` outside parentheses and brackets.
fn split_selectors(selectors: &str) -> Vec<&str> {
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
    parts
}

/// The class a utility selector starts with, unescaped (`.sm\:p-4:hover` →
/// `sm:p-4`), outside parentheses. [`selector_needs`] unwraps a `:where()`
/// or `:is()` around the whole selector first, so Tailwind's
/// `:where(.space-y-2>*)` still has its owner.
fn owner_class(selector: &str) -> Option<String> {
    let bytes = selector.as_bytes();
    // Outside parentheses: a class inside `:not(.light)` or `:is(.a,.b)` is
    // an alternative or an exclusion, never what the rule is for.
    let mut depth = 0usize;
    let mut dot = None;
    for i in 0..bytes.len() {
        match bytes[i] {
            b'\\' => continue,
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b'.' if depth == 0 && (i == 0 || bytes[i - 1] != b'\\') => {
                dot = Some(i);
                break;
            }
            _ => {}
        }
    }
    let dot = dot?;
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
        if style.is_empty() {
            return "";
        }
        &style[style.find('>').unwrap() + 1..style.rfind("</style>").unwrap()]
    }

    /// Facts of a page that renders `html` (classes included).
    fn facts(html: &str) -> HashSet<String> {
        let mut facts = HashSet::new();
        note_html(html, &mut facts);
        for (at, _) in html.match_indices("class=\"") {
            let list = &html[at + 7..];
            facts.extend(list[..list.find('"').unwrap()].split_whitespace().map(str::to_owned));
        }
        facts
    }

    /// The CSS a fresh browser gets for a page rendering `html`.
    fn sent_for(css: &str, html: &str, keep: &[&str]) -> String {
        let split = Split::parse(css);
        let facts = facts(html);
        let page = Page::new(&facts, keep.iter().copied());
        css_of(&split.style(&SHEET, &split.delta(&page, &split.empty()))).to_owned()
    }

    fn classes(list: &[&str]) -> HashSet<String> {
        list.iter().map(|c| (*c).to_owned()).collect()
    }

    fn delta(split: &Split, list: &[&str], sent: &Pieces) -> Pieces {
        let facts = classes(list);
        split.delta(&Page::new(&facts, []), sent)
    }

    #[test]
    fn owners() {
        assert_eq!(owner_class(".p-4").as_deref(), Some("p-4"));
        assert_eq!(owner_class(".sm\\:w-\\[1\\.5rem\\]").as_deref(), Some("sm:w-[1.5rem]"));
        assert_eq!(selector_needs(":where(.space-y-2>:not(:last-child))"), [Need::Class("space-y-2".into())]);
        assert_eq!(owner_class(":root:not(.light)"), None, "a class inside :not() owns nothing");
        assert_eq!(owner_class(".dark\\:x:where(.dark,.dark *)").as_deref(), Some("dark:x"));
        assert_eq!(owner_class("input"), None);
        assert_eq!(split_selectors(".a:hover,.b\\,c:where(.x,.y)"), [".a:hover", ".b\\,c:where(.x,.y)"]);
        assert!(selector_alts(".a,*").is_empty(), "a selector that needs nothing: always sent");
    }

    #[test]
    fn selectors_need_their_elements_ids_and_attributes() {
        let class = |c: &str| Need::Class(c.to_owned());
        let tag = |t: &str| Need::Tag(t.to_owned());
        let attr = |n: &str, v: Option<&str>| Need::Attr(n.to_owned(), v.map(str::to_owned));
        assert_eq!(selector_needs("details>summary"), [tag("details"), tag("summary")]);
        assert_eq!(selector_needs("input[type=checkbox]:checked"), [tag("input"), attr("type", Some("checkbox"))]);
        assert_eq!(
            selector_needs("[data-theme=\"emerald\"].dark"),
            [class("dark"), attr("data-theme", Some("emerald"))]
        );
        assert_eq!(selector_needs("a[href^=http]"), [tag("a"), attr("href", None)], "an operator: by name");
        assert_eq!(selector_needs("a[title=\"x\"]"), [tag("a"), attr("title", None)], "values are not recorded");
        assert_eq!(selector_needs("#menu li"), [Need::Id("menu".into()), tag("li")]);
        assert_eq!(selector_needs("html:has(#top[data-x]) .card DIV"), [class("card"), tag("div")]);
        assert_eq!(selector_needs(":where(select:is([multiple],[size])) optgroup"), [tag("optgroup")]);
        assert_eq!(selector_needs(":where(ul>li)"), [tag("ul"), tag("li")], "one selector in :where(): read as it");
        assert_eq!(selector_needs("::file-selector-button"), [tag("input")]);
        assert_eq!(selector_needs("::-webkit-datetime-edit-year-field"), [tag("input")]);
        for nothing in ["*", ":root", "html", "body", "::selection", ":not(dialog)", "*,::before", ":-moz-focusring"] {
            assert!(selector_needs(nothing).is_empty(), "{nothing}");
        }
    }

    #[test]
    fn element_attribute_and_id_rules_follow_the_markup() {
        let css = "details{margin:0}summary{display:list-item}input[type=checkbox]{width:1rem}input[type=text]{width:2rem}\
#menu{top:0}[hidden]{display:none}dialog::backdrop{color:red}[aria-expanded=true]{color:red}h1,table{margin:0}";
        assert_eq!(
            sent_for(css, "<h1 id=\"x\">a</h1><input type=\"checkbox\">", &[]),
            "input[type=checkbox]{width:1rem}[aria-expanded=true]{color:red}h1,table{margin:0}",
            "ARIA states change at any time: kept"
        );
        assert_eq!(
            sent_for(css, "<details hidden><summary>a</summary></details><nav id=menu></nav>", &[]),
            "details{margin:0}summary{display:list-item}#menu{top:0}[hidden]{display:none}[aria-expanded=true]{color:red}"
        );
        // A script the page loads (or a safelist) names the element and the attribute.
        assert!(sent_for(css, "<p></p>", &["<dialog", "hidden"]).contains("[hidden]{display:none}dialog::backdrop"));
        assert!(
            !sent_for(css, "<script>let dialog = table.summary</script>", &[]).contains("dialog"),
            "a word is not an element"
        );
        assert!(
            sent_for(css, "<script>document.createElement(tag)</script>", &[]).contains("details{margin:0}"),
            "any element"
        );
        assert!(sent_for(css, "<script>el.innerHTML = '<table><tr>'</script>", &[]).contains("h1,table{margin:0}"));
        // An inline script that creates it.
        assert!(sent_for(css, "<script>document.createElement('dialog')</script>", &[]).contains("dialog::backdrop"));
    }

    #[test]
    fn theme_blocks_go_with_the_theme_the_page_names() {
        let css = ":root{--bg:white}[data-theme=\"noir\"]{--bg:black}[data-theme=\"rose\"]{--bg:pink}\
[data-theme=\"rose\"].dark{--bg:maroon}.dark{--bg:#111}body{background:var(--bg)}";
        assert_eq!(
            sent_for(css, "<p>x</p>", &["dark"]),
            ":root{--bg:white}.dark{--bg:#111}body{background:var(--bg)}",
            "no element has data-theme: no theme block can match"
        );
        assert_eq!(
            sent_for(css, "<div data-theme=\"rose\"></div>", &["dark"]),
            ":root{--bg:white}[data-theme=\"rose\"]{--bg:pink}[data-theme=\"rose\"].dark{--bg:maroon}.dark{--bg:#111}\
body{background:var(--bg)}"
        );
        // A script switches themes at runtime: `dataset.theme = …`.
        let switcher = sent_for(css, "<script>document.documentElement.dataset.theme = t</script>", &["dark"]);
        assert!(switcher.contains("noir") && switcher.contains("pink"), "{switcher}");
        // A script nothing is known about may do the same.
        let facts = facts("<p>x</p>");
        let split = Split::parse(css);
        let unknown = Page::new(&facts, []).with_unknown_scripts(true);
        assert!(split.style(&SHEET, &split.delta(&unknown, &split.empty())).contains("noir"));
        // …or the safelist says so.
        assert!(sent_for(css, "<p>x</p>", &["data-theme"]).contains("noir"));
        // `css_static_attributes`: the script is overruled, the markup decides.
        let fixed = sent_for(
            css,
            "<html data-theme=\"rose\"><script>d.setAttribute('data-theme', saved)</script>",
            &["!data-theme"],
        );
        assert!(fixed.contains("pink") && !fixed.contains("noir"), "{fixed}");
    }

    #[test]
    fn custom_properties_are_sent_when_something_reads_them() {
        let css = ":root{--a:1px;--b:var(--a);--c:red;--unused:var(--c);color:var(--c);--d:4px;--e:5px;--f:6px}\
.x{width:var(--b)}.y{height:calc(var(--d) * 2)}.z{color:color-mix(in oklab,var(--e) 50%,transparent)}\
.w{margin:var(--missing,var(--f))}";
        assert_eq!(
            sent_for(css, "<p class=\"x\"></p>", &[]),
            ":root{--a:1px;--b:var(--a);--c:red;color:var(--c)}.x{width:var(--b)}",
            "--b is read by .x, --a by --b (transitively), --c by the block's own declaration"
        );
        assert_eq!(
            sent_for(css, "<p class=\"y\"></p>", &[]),
            ":root{--c:red;color:var(--c);--d:4px}.y{height:calc(var(--d) * 2)}"
        );
        assert!(sent_for(css, "<p class=\"z\"></p>", &[]).contains("--e:5px}"), "inside color-mix()");
        assert!(sent_for(css, "<p class=\"w\"></p>", &[]).contains("--f:6px}"), "a fallback: var(--a, var(--b))");
        // An inline style reads it; a safelist (`css_keep_vars`) keeps it.
        assert!(sent_for(css, "<p style=\"width:var(--e)\"></p>", &[]).contains("--e:5px"));
        assert!(sent_for(css, "<p></p>", &["--unused"]).contains("--unused:var(--c)"));
        assert!(sent_for(css, "<script>el.style.setProperty('--f', x)</script>", &[]).contains("--f:6px"));
    }

    #[test]
    fn blocks_left_empty_are_not_written_and_neighbours_merge() {
        let css = ":root{--a:1}@supports (color:red){:root{--b:2}}:root{--c:3}@media (width>=1px){:root{--z:0}}.x{top:var(--a);left:var(--c)}";
        assert_eq!(sent_for(css, "<p class=\"x\"></p>", &[]), ":root{--a:1;--c:3}.x{top:var(--a);left:var(--c)}");
    }

    #[test]
    fn keyframes_fonts_and_registrations_follow_their_users() {
        let css = "@font-face{font-family:\"Plus Jakarta\";src:url(a.woff2)}@font-face{font-family:Geist;src:url(g.woff2)}\
@font-face{font-family:Unused;src:url(u.woff2)}@property --angle{syntax:\"<angle>\";inherits:false;initial-value:0deg}\
:root{--font-display:\"Plus Jakarta\",serif;--animate-float:float 6s infinite;--angle:10deg;--k:1}\
body{font-family:geist,sans-serif}.title{font-family:var(--font-display)}.floats{animation:var(--animate-float)}\
.left{float:left}.spin{animation-name:spin}.turn{rotate:var(--angle)}\
@keyframes float{to{translate:0 -4px}}@keyframes spin{to{rotate:1turn}}@keyframes never{to{opacity:0}}";
        let body = sent_for(css, "<p class=\"left\"></p>", &[]);
        assert_eq!(
            body, "@font-face{font-family:Geist;src:url(g.woff2)}body{font-family:geist,sans-serif}.left{float:left}",
            "`float:left` is not the animation `float`"
        );
        let title = sent_for(css, "<p class=\"title floats\"></p>", &[]);
        assert!(
            title.contains("Plus Jakarta\";src") && title.contains("--font-display:"),
            "through a variable: {title}"
        );
        assert!(title.contains("@keyframes float") && title.contains("--animate-float:"), "{title}");
        assert!(!title.contains("@keyframes spin") && !title.contains("Unused") && !title.contains("--k"), "{title}");
        let turn = sent_for(css, "<p class=\"turn spin\"></p>", &[]);
        assert!(
            turn.contains("@property --angle") && turn.contains("--angle:10deg") && turn.contains("@keyframes spin")
        );
        assert!(!body.contains("@property"), "its variable is not sent: neither is the registration");
        // An inline style names the keyframes and the font.
        let inline = sent_for(css, "<p style=\"animation: never 1s; font-family: Unused\"></p>", &[]);
        assert!(inline.contains("@keyframes never") && inline.contains("family:Unused"), "{inline}");
    }

    #[test]
    fn supports_and_media_are_pruned_inside() {
        let css = "@supports (color:color-mix(in lab,red,red)){:root{--glass:color-mix(in oklab,var(--s) 92%,transparent)}\
.card{color:red}}@media print{table{width:100%}}:root{--s:white;--glass:var(--s)}.glass{background:var(--glass)}";
        assert_eq!(sent_for(css, "<p></p>", &[]), "");
        assert_eq!(
            sent_for(css, "<p class=\"glass\"></p>", &[]),
            "@supports (color:color-mix(in lab,red,red)){:root{--glass:color-mix(in oklab,var(--s) 92%,transparent)}}\
:root{--s:white;--glass:var(--s)}.glass{background:var(--glass)}"
        );
    }

    #[test]
    fn identical_rules_are_sent_once() {
        let css = "::file-selector-button{margin:0}a{color:red}::file-selector-button{margin:0}";
        assert_eq!(sent_for(css, "<a></a><input>", &[]), "a{color:red}::file-selector-button{margin:0}");
    }

    #[test]
    fn a_navigation_gets_what_the_new_page_needs() {
        let css = ":root{--a:1;--b:2}.dark{--a:3;--b:4}.x{top:var(--a)}.y{top:var(--b)}.prose table{width:100%}details{margin:0}\
[data-theme=\"noir\"]{--a:9}@keyframes pop{to{scale:1}}.pop{animation:pop 1s}";
        let split = Split::parse(css);
        let first = facts("<main class=\"prose\"><p class=\"x\"></p></main>");
        let mut sent = split.empty();
        let pieces = split.delta(&Page::new(&first, ["dark"]), &sent);
        assert_eq!(css_of(&split.style(&SHEET, &pieces)), ":root{--a:1}.dark{--a:3}.x{top:var(--a)}");
        split.record(&mut sent, &pieces);
        // The browser reports what it has; the region that changes has a
        // table, while `.prose` stays on screen outside of it.
        let id = split.style(&SHEET, &pieces).split('"').nth(1).unwrap().to_owned();
        let known = split.known(&SHEET, &HashSet::from([id]));
        assert_eq!(known, sent);
        let region = facts("<table class=\"y pop\"></table>");
        let next = split.delta(&Page::new(&region, ["dark"]), &known);
        assert_eq!(
            css_of(&split.style(&SHEET, &next)),
            ":root{--b:2}.dark{--b:4}.y{top:var(--b)}.prose table{width:100%}@keyframes pop{to{scale:1}}.pop{animation:pop 1s}",
            "new variables, the rule whose owner is outside the region, the keyframes; no theme block, no <details>"
        );
        split.record(&mut sent, &next);
        assert!(split.delta(&Page::new(&region, ["dark"]), &sent).is_empty(), "nothing new");
        // Without the first page's facts, `.prose table` would not be sent.
        let cold = split.delta(&Page::new(&region, ["dark"]), &split.empty());
        assert!(!css_of(&split.style(&SHEET, &cold)).contains(".prose table"));
    }

    #[test]
    fn a_variable_sent_later_keeps_the_cascade_order() {
        // `[data-x]` comes first in the sheet and has the specificity of
        // `:root`: when it arrives after `:root{--a}` was sent, that is
        // repeated behind it, so `:root` still wins as written.
        let css = "[data-x]{--a:2}:root{--a:1;--b:0}.u{top:var(--a)}";
        let split = Split::parse(css);
        let mut sent = split.empty();
        let first = split.delta(&Page::new(&facts("<p class=\"u\"></p>"), []), &sent);
        assert_eq!(css_of(&split.style(&SHEET, &first)), ":root{--a:1}.u{top:var(--a)}");
        split.record(&mut sent, &first);
        let next = split.delta(&Page::new(&facts("<p data-x></p>"), []), &sent);
        assert_eq!(css_of(&split.style(&SHEET, &next)), "[data-x]{--a:2}:root{--a:1}");
    }

    #[test]
    fn a_dark_block_behind_not_and_a_media_query_is_always_sent() {
        // The UI components' sheet: system dark mode, opted out of by a
        // class or an attribute. Nothing on the page has to match for the
        // block to be needed — the reader can switch at any time.
        let css = "@media (prefers-color-scheme:dark){:root:not(.light):not([data-theme=\"light\"]){--nr-bg:#000;--nr-fg:#fff;--nr-idle:#888}}.card{background:var(--nr-bg);color:var(--nr-fg)}";
        let out = sent_for(css, "<div class=\"card\">x</div>", &[]);
        assert!(
            out.starts_with("@media (prefers-color-scheme:dark){:root:not(.light):not([data-theme=\"light\"]){"),
            "{out}"
        );
        assert!(out.contains("--nr-bg:#000") && out.contains("--nr-fg:#fff"), "{out}");
        assert!(!out.contains("--nr-idle"), "a variable nothing reads is still left out: {out}");
        // Nothing on the page reads them: the block goes entirely.
        assert_eq!(sent_for(css, "<p>x</p>", &[]), "");
    }

    #[test]
    fn presentation_attributes_read_variables_and_fonts() {
        let css = ":root{--mono:M,monospace;--sans:S;--gone:X}@font-face{font-family:M;src:url(m.woff2)}@font-face{font-family:Unused;src:url(u.woff2)}@keyframes spin{to{rotate:1turn}}text{fill:red}";
        // An SVG <text> naming the variable in a presentation attribute.
        let out = sent_for(css, "<svg><text font-family=\"var(--mono), monospace\">x</text></svg>", &[]);
        assert!(out.contains("--mono:M,monospace") && out.contains("@font-face{font-family:M;"), "{out}");
        assert!(!out.contains("--gone") && !out.contains("Unused"), "{out}");
        // Any attribute whose value reads a variable counts.
        assert!(sent_for(css, "<div data-x=\"var(--sans)\"></div>", &[]).contains("--sans:S"));
        // The value is still recorded for attribute selectors.
        let mut f = HashSet::new();
        note_attr("data-theme", Some("var(--mono)"), &mut f);
        assert!(f.contains(" vdata-theme=var(--mono)") && f.contains(" w--mono"), "{f:?}");
    }

    #[test]
    fn framework_scripts_and_html_facts() {
        let mut f = HashSet::new();
        note_html(
            "<DIV id=a data-x='1' hidden style=\"--fill:.5\"><svg viewBox=\"0 0 1 1\"/></div><script>a.dataset.fooBar=1</script>",
            &mut f,
        );
        for key in
            [" tdiv", " ia", " adata-x", " vdata-x=1", " ahidden", " w--fill", " tsvg", " aviewbox", " wdata-foo-bar"]
        {
            assert!(f.contains(key), "{key} missing from {f:?}");
        }
        assert!(!f.iter().any(|k| k.starts_with(" vviewbox")), "values of other attributes are not recorded");
    }

    #[test]
    fn sends_only_used_rules_and_everything_outside_utilities() {
        let split = Split::parse(CSS);
        let pieces = delta(&split, &["px-2", "sm:p-10"], &split.empty());
        let css = split.style(&SHEET, &pieces);
        assert_eq!(
            css_of(&css),
            "@layer theme,base,components,utilities;@layer base{*{margin:0}}\
@layer utilities{.px-2{padding-inline:.5rem}@media (width>=40rem){.sm\\:p-10{padding:2.5rem}}}",
            "`--c`, `@property` and `@keyframes` nothing refers to are left out"
        );
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
        let first = delta(&split, &["px-2", "mt-2"], &sent);
        split.record(&mut sent, &first);
        let next = delta(&split, &["p-4"], &sent);
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
        let first = delta(&split, &["a", "c"], &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &first)),
            "/*! notice */@layer properties{@supports (x:y){*,:before{--tw-a:initial}}}\
@layer utilities{.a{--tw-a:1px;width:var(--tw-a)}.c{color:red}}@property --tw-a{syntax:\"*\";inherits:false}"
        );
        split.record(&mut sent, &first);
        let next = delta(&split, &["b"], &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &next)),
            "@layer properties{@supports (x:y){*,:before{--tw-b:0}}}@layer utilities{.b{height:var(--tw-b)}}\
@property --tw-b{syntax:\"*\";inherits:false}"
        );
        assert!(mentions("a{animation:spin 1s}", "spin") && !mentions("a{animation:spinner}", "spin"));
    }

    #[test]
    fn later_chunks_send_what_is_missing_in_order() {
        let split = Split::parse(CSS);
        let mut sent = split.empty();
        let first = delta(&split, &["px-2"], &sent);
        split.record(&mut sent, &first);
        // `p-4` comes before `px-2`: `px-2` is repeated after it so it still wins.
        let next = delta(&split, &["p-4", "px-2"], &sent);
        assert_eq!(
            css_of(&split.style(&SHEET, &next)),
            "@layer utilities{.p-4{padding:1rem}.px-2{padding-inline:.5rem}}"
        );
        split.record(&mut sent, &next);
        assert!(delta(&split, &["p-4"], &sent).is_empty(), "nothing new");
    }

    #[test]
    fn cached_documents_are_restyled_for_the_browser() {
        let split = Split::parse(CSS);
        let page = delta(&split, &["p-4", "px-2"], &split.empty());
        let html = format!("<head>{}</head>", split.style(&SHEET, &page));
        let had = delta(&split, &["px-2"], &split.empty());
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
        let pieces = delta(&split, &["sm:w-[1.5rem]", "hover:underline"], &split.empty());
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
        // page, with the variables, fonts and keyframes they use.
        let css = "@import url(x.css);:root{--c:red}h1{margin:0}.card{padding:1rem;color:var(--c)}.card .title{font-weight:700}\
@media (width>=40rem){.card{padding:2rem}.wide{width:100%}}@font-face{font-family:F;src:url(f.woff2)}\
.spin{animation:spin 1s}@keyframes spin{to{rotate:1turn}}[data-theme=dark]{--c:blue}";
        assert_eq!(
            sent_for(css, "<h1 class=\"card\"></h1>", &[]),
            "@import url(x.css);:root{--c:red}h1{margin:0}.card{padding:1rem;color:var(--c)}.card .title{font-weight:700}\
@media (width>=40rem){.card{padding:2rem}}"
        );
        let out = sent_for(css, "<p class=\"spin\"></p>", &[]);
        assert!(
            out.contains(".spin{animation:spin 1s}@keyframes spin{to{rotate:1turn}}") && !out.contains(".card"),
            "{out}"
        );
    }
}
