//! Short class names for release builds.
//!
//! After Tailwind generates the CSS, every utility class gets a short random
//! name (`rounded-lg` → `k7`): the selectors in the CSS are rewritten here,
//! and the renaming table is compiled into the app, where the renderer
//! applies it to the HTML (see `next_rust_view::class_names`).
//!
//! * Names are random for every build: the letters are shuffled with a new
//!   seed each time (`NEXT_RUST_CLASS_SEED` fixes the seed, for reproducible
//!   builds or several servers behind one load balancer).
//! * The classes used most often in the source get the shortest names: one
//!   or two characters for the first thousand or so, three beyond.
//! * A short name never equals a class in the CSS, a string in the Rust
//!   source, a word in a script or HTML file, or a class in a stylesheet of
//!   the project, so it can't collide with a class that keeps its name.
//! * The UI components' classes (`nr-btn`, ...) are shortened with them when
//!   the project uses the components; the server renames the component
//!   stylesheet and script to match.
//! * Scripts in `client/` and `assets/` are rewritten to use the short names
//!   where a string can only be a class list (see `js_classes`); a class a
//!   script uses in any other way keeps its name, as do classes that appear
//!   in `public/` or extra Tailwind sources and `[tailwind] keep_classes`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use std::path::{Path, PathBuf};

use next_rust_core::Config;

use crate::codegen::embeddable_files;
use crate::css_usage;

/// File in `$OUT_DIR` with the renaming table as a Rust slice expression.
pub const FILE: &str = "next_rust_class_names.rs";

/// The result of [`shorten`].
pub struct Shortened {
    /// The CSS with short class names.
    pub css: String,
    /// `(class, short name)`, sorted by class.
    pub names: Vec<(String, String)>,
}

/// The seed for this build: `NEXT_RUST_CLASS_SEED` if set, otherwise random.
pub fn seed() -> u64 {
    if let Ok(value) = std::env::var("NEXT_RUST_CLASS_SEED") {
        return value.trim().parse().unwrap_or_else(|_| {
            let mut h = next_rust_assets::Fnv64::new();
            h.write(value.as_bytes());
            h.finish()
        });
    }
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut h = next_rust_assets::Fnv64::new();
    h.write(&nanos.to_le_bytes());
    h.write(&std::process::id().to_le_bytes());
    h.write(&(&nanos as *const u128 as usize).to_le_bytes());
    h.finish()
}

/// The UI components' classes (`nr-btn`, `nr-field`, ...).
pub fn component_classes() -> Vec<String> {
    let mut classes: Vec<String> = next_rust_assets::css::class_selectors(next_rust_ui::UI_CSS.css)
        .all
        .into_iter()
        .filter(|class| class.starts_with("nr-"))
        .collect();
    classes.extend(next_rust_ui::EXTRA_CLASSES.iter().map(|c| (*c).to_owned()));
    classes
}

/// Give the utility classes of `css` (minified Tailwind output) and the
/// `extra` classes short names.
pub fn shorten(css: &str, config: &Config, seed: u64, extra: &[String]) -> Shortened {
    let found = next_rust_assets::css::class_selectors(css);
    let known: BTreeSet<&str> = found.all.iter().chain(extra).map(String::as_str).collect();
    let scripts = script_usage(config, &|c| known.contains(c));
    let keep: BTreeSet<&str> = config.tailwind.keep_classes.iter().map(String::as_str).collect();
    // Tailwind's `group`/`peer` classes only give context to other classes
    // but are Tailwind's own, so they are shortened with the rest; other
    // context classes (`.tk` in `[&_.tk]:text-red-500`) may be anyone's.
    let context = found
        .all
        .iter()
        .filter(|c| c.as_str() == "group" || c.starts_with("group/") || c.as_str() == "peer" || c.starts_with("peer/"));
    let mut candidates: Vec<&String> = found
        .leading
        .iter()
        .chain(context)
        .chain(extra)
        .filter(|class| !keep.contains(class.as_str()) && !scripts.fixed.contains(class.as_str()))
        .collect();
    candidates.sort();
    candidates.dedup();

    // Names that must never be used as a short name.
    let mut reserved = reserved_names(config);
    reserved.extend(found.all.iter().cloned());
    reserved.extend(extra.iter().cloned());
    reserved.extend(keep.iter().map(|k| (*k).to_owned()));

    // Most used first, so they get the shortest names.
    let counts = source_counts(config, &candidates);
    let mut ordered = candidates;
    ordered.sort_by(|a, b| counts.get(b.as_str()).cmp(&counts.get(a.as_str())).then_with(|| a.cmp(b)));

    let mut generator = ShortNames::new(seed);
    let map: BTreeMap<String, String> = ordered
        .into_iter()
        .map(|class| {
            let short = generator.by_ref().find(|name| !reserved.contains(name)).expect("names never run out");
            (class.clone(), short)
        })
        .collect();

    let css = next_rust_assets::css::rename_classes(css, &|class| map.get(class).cloned());
    Shortened { css, names: map.into_iter().collect() }
}

/// Classes every page gets the rules of: `[tailwind] keep_classes`,
/// `safelist`, and classes named in extra Tailwind sources (files the
/// renderer never sees and no page is known to load).
pub fn outside_classes(css: &str, config: &Config) -> Vec<String> {
    let found = next_rust_assets::css::class_selectors(css);
    let scripts = script_usage(config, &|c| found.all.contains(c));
    let mut classes: BTreeSet<String> = scripts.sources;
    classes.extend(config.tailwind.keep_classes.iter().cloned());
    classes.extend(config.tailwind.safelist.iter().cloned());
    classes.into_iter().collect()
}

/// Classes each script adds, by script key (see `next_rust_view::script_key`):
/// a page that loads the script gets their rules. Scripts a script imports
/// count towards it.
pub fn script_classes(css: &str, config: &Config) -> Vec<(String, Vec<String>)> {
    let found = next_rust_assets::css::class_selectors(css);
    let usage = script_usage(config, &|c| found.all.contains(c));
    let mut out = Vec::new();
    for key in usage.by_script.keys() {
        let mut classes = BTreeSet::new();
        let mut todo = vec![key.clone()];
        let mut seen = BTreeSet::new();
        while let Some(k) = todo.pop() {
            if !seen.insert(k.clone()) {
                continue;
            }
            if let Some(c) = usage.by_script.get(&k) {
                classes.extend(c.iter().cloned());
            }
            todo.extend(usage.imports.get(&k).into_iter().flatten().cloned());
        }
        if !classes.is_empty() {
            out.push((key.clone(), classes.into_iter().collect()));
        }
    }
    out
}

/// Every class something knows: the Tailwind stylesheet (`css`, or the
/// class list the generator wrote under `tailwind_out`), every stylesheet
/// in the project, every word in scripts and static files, `.class`
/// selectors in Rust strings, `extra`, `keep_classes` and `safelist`.
/// Sorted. Pages leave out generated-looking classes outside this set.
pub fn known_classes(css: &str, config: &Config, extra: &[String], tailwind_out: Option<&Path>) -> Vec<String> {
    let mut known: BTreeSet<String> = next_rust_assets::css::class_selectors(css).all;
    if let Some(list) =
        tailwind_out.and_then(|d| std::fs::read_to_string(d.join("next_rust_tailwind_classes.txt")).ok())
    {
        known.extend(list.lines().map(str::to_owned));
    }
    known.extend(extra.iter().cloned());
    known.extend(config.tailwind.keep_classes.iter().cloned());
    known.extend(config.tailwind.safelist.iter().cloned());
    fn token(t: &str) -> &str {
        t.trim_matches(|c: char| matches!(c, '.' | '"' | '\'' | '`' | ',' | ';' | '(' | ')'))
    }
    for dir in project_dirs(config) {
        for (_, path) in embeddable_files(&dir) {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if css_usage::SKIP.contains(&ext.as_str()) && ext != "css" {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            match ext.as_str() {
                "css" => known.extend(next_rust_assets::css::class_selectors(&text).all),
                "rs" => {
                    // Inline scripts and styles: `.hero` in a string.
                    for literal in string_literals(&text) {
                        known.extend(
                            literal
                                .split_whitespace()
                                .filter(|t| t.starts_with('.') || t.starts_with("(\".") || t.starts_with("'."))
                                .map(|t| token(t).to_owned())
                                .filter(|t| !t.is_empty()),
                        );
                    }
                }
                _ => known.extend(text.split_whitespace().map(|t| token(t).to_owned()).filter(|t| !t.is_empty())),
            }
        }
    }
    for stylesheet in &config.tailwind.stylesheets {
        if let Ok(text) = std::fs::read_to_string(config.resolve(stylesheet)) {
            known.extend(next_rust_assets::css::class_selectors(&text).all);
        }
    }
    known.into_iter().collect()
}

/// Generated-looking classes (`sm:x`, `bg-[…]`, `w-1/3`, `!p-0`) in the Rust
/// source that nothing knows, sorted: the build report names them.
pub fn unknown_in_source(config: &Config, known: &[String]) -> Vec<String> {
    let looks_generated = next_rust_view::class_names::looks_generated;
    let mut out = BTreeSet::new();
    for dir in [config.app_dir(), config.root.join("src")] {
        for (_, path) in embeddable_files(&dir) {
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            for literal in string_literals(&text) {
                // Only strings that are class lists: every token class-shaped.
                let literal = continued(literal);
                let tokens: Vec<&str> = literal.split_whitespace().collect();
                let class_shaped = |t: &str| {
                    t.strip_prefix(['!', '-']).unwrap_or(t).starts_with(|c: char| c.is_ascii_lowercase())
                        && t.chars().all(|c| c.is_ascii_alphanumeric() || "-_:/[]().%,#+!".contains(c))
                };
                if tokens.is_empty() || !tokens.iter().all(|t| class_shaped(t)) {
                    continue;
                }
                // A lone `campaign:read` is more likely a scope than a class.
                if tokens.len() == 1 && !tokens[0].contains(['[', '/', '!']) {
                    continue;
                }
                for t in tokens {
                    if looks_generated(t) && known.binary_search_by(|k| k.as_str().cmp(t)).is_err() {
                        out.insert(t.to_owned());
                    }
                }
            }
        }
    }
    out.into_iter().collect()
}

/// A string literal with its `\`-newline continuations applied, as Rust
/// does: the backslash, the newline and the next line's indentation go.
fn continued(literal: &str) -> String {
    let mut out = String::with_capacity(literal.len());
    let mut rest = literal;
    while let Some(at) = rest.find("\\\n") {
        out.push_str(&rest[..at]);
        rest = rest[at + 2..].trim_start();
    }
    out.push_str(rest);
    out
}

/// The directories whose files may name classes.
fn project_dirs(config: &Config) -> Vec<PathBuf> {
    let mut dirs = vec![
        config.app_dir(),
        config.root.join("src"),
        config.root.join("styles"),
        config.public_dir(),
        config.root.join("client"),
        config.root.join("assets"),
    ];
    dirs.extend(config.tailwind.sources.iter().map(|s| config.resolve(s)));
    dirs
}

/// A list of strings as a Rust expression of type `&[&str]`.
pub fn list_source(items: &[String]) -> String {
    format!("&{items:?}")
}

/// `(key, list)` pairs as a Rust expression of type `&[(&str, &[&str])]`.
pub fn map_source(items: &[(String, Vec<String>)]) -> String {
    let mut out = String::from("&[\n");
    for (key, list) in items {
        out.push_str(&format!("    ({key:?}, &{list:?}),\n"));
    }
    out.push(']');
    out
}

/// The renaming table as a Rust expression of type `&[(&str, &str)]`.
pub fn table_source(names: &[(String, String)]) -> String {
    let mut out = String::from("&[\n");
    for (class, short) in names {
        out.push_str(&format!("    ({class:?}, {short:?}),\n"));
    }
    out.push(']');
    out
}

/// How code the renderer never sees uses classes.
#[derive(Debug, Default)]
pub struct ScriptUsage {
    /// Classes that must keep their names: used by scripts in ways that can't
    /// be rewritten, or by files that are served as written (`public/`,
    /// extra Tailwind sources).
    pub fixed: BTreeSet<String>,
    /// Classes in class lists of `client/` and `assets/` scripts, which the
    /// release build rewrites with the short names.
    pub renamable: BTreeSet<String>,
    /// Classes each script (`client/x.js`, `assets/x.js`, `x.js` under
    /// `public/`) adds, fixed or renamable.
    pub by_script: BTreeMap<String, BTreeSet<String>>,
    /// Scripts each script imports with a relative path, by key.
    pub imports: BTreeMap<String, Vec<String>>,
    /// Classes named in extra Tailwind sources.
    pub sources: BTreeSet<String>,
}

/// Whether a project uses the UI components (`next_rust::ui`).
pub fn uses_components(config: &Config) -> bool {
    [config.app_dir(), config.root.join("src")].iter().any(|dir| {
        embeddable_files(dir).iter().any(|(_, path)| {
            path.extension().is_some_and(|e| e == "rs")
                && std::fs::read_to_string(path)
                    .is_ok_and(|s| s.contains("next_rust::ui") || s.contains("next_rust_ui") || s.contains("ui::"))
        })
    })
}

/// Classes of the stylesheet (`is_class`) as scripts and static files use them.
pub fn script_usage(config: &Config, is_class: &dyn Fn(&str) -> bool) -> ScriptUsage {
    let mut usage = ScriptUsage::default();
    let is_script = |rel: &str| matches!(rel.rsplit('.').next(), Some("js" | "mjs"));
    // Scripts that are compiled into the binary can be rewritten.
    for (prefix, dir) in [("client", config.root.join("client")), ("assets", config.root.join("assets"))] {
        for (rel, path) in embeddable_files(&dir) {
            let Ok(source) = std::fs::read_to_string(&path) else { continue };
            let key = format!("{prefix}/{rel}");
            let mut own = BTreeSet::new();
            if is_script(&rel) {
                usage.imports.insert(key.clone(), relative_imports(&key, &source));
                if let Ok(found) = crate::js_classes::analyze(&source, is_class) {
                    own.extend(found.fixed.iter().cloned());
                    own.extend(found.renamable.iter().map(|(_, _, c)| c.clone()));
                    usage.fixed.extend(found.fixed);
                    usage.renamable.extend(found.renamable.into_iter().map(|(_, _, c)| c));
                    usage.by_script.insert(key, own);
                    continue;
                }
            }
            fixed_in(&source, is_class, &mut own);
            usage.fixed.extend(own.iter().cloned());
            usage.by_script.insert(key, own);
        }
    }
    // `public/` is served as written: every class a file names keeps it, and
    // a page gets the rules when it loads the file.
    for (rel, path) in embeddable_files(&config.public_dir()) {
        let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if ext == "rs" || css_usage::SKIP.contains(&ext.as_str()) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let source = String::from_utf8_lossy(&bytes);
        let mut own = BTreeSet::new();
        fixed_in(&source, is_class, &mut own);
        usage.fixed.extend(own.iter().cloned());
        if is_script(&rel) {
            usage.imports.insert(rel.clone(), relative_imports(&rel, &source));
        }
        usage.by_script.insert(rel, own);
    }
    // Extra Tailwind sources: nothing loads them, so their classes go everywhere.
    for dir in config.tailwind.sources.iter().map(|s| config.resolve(s)) {
        for (_, path) in embeddable_files(&dir) {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if ext == "rs" || css_usage::SKIP.contains(&ext.as_str()) {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                fixed_in(&String::from_utf8_lossy(&bytes), is_class, &mut usage.sources);
            }
        }
    }
    usage.fixed.extend(usage.sources.iter().cloned());
    usage.renamable.retain(|c| !usage.fixed.contains(c));
    usage
}

/// Keys of the scripts `source` (at `key`) imports with a relative path:
/// `import … from "./x.js"`, `import("./x.js")`.
fn relative_imports(key: &str, source: &str) -> Vec<String> {
    let dir = key.rsplit_once('/').map_or("", |(d, _)| d);
    let mut out = Vec::new();
    for quote in ['"', '\''] {
        for (at, _) in source.match_indices(quote) {
            let rest = &source[at + 1..];
            let Some(end) = rest.find(quote) else { continue };
            let target = &rest[..end];
            if !(target.starts_with("./") || target.starts_with("../")) {
                continue;
            }
            let before = source[..at].trim_end();
            if !(before.ends_with("from") || before.ends_with("import(") || before.ends_with("import")) {
                continue;
            }
            let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
            for part in target.split('/') {
                match part {
                    "." | "" => {}
                    ".." => {
                        parts.pop();
                    }
                    p => parts.push(p),
                }
            }
            out.push(parts.join("/"));
        }
    }
    out
}

/// Every class name that appears in `text` as a whole token.
fn fixed_in(text: &str, is_class: &dyn Fn(&str) -> bool, into: &mut BTreeSet<String>) {
    let part = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':');
    let boundary = |c: char| !part(c) && !matches!(c, '[' | ']' | '(' | ')' | '.' | '/' | '!' | '%' | ',' | '#');
    for token in text.split(boundary) {
        if !token.is_empty() && is_class(token) {
            into.insert(token.to_owned());
        }
    }
}

/// Names a short class name must not take: strings in the Rust source,
/// words in scripts and static files, and classes in the project's own
/// stylesheets. A Rust identifier is not one: `let a` never becomes a class.
fn reserved_names(config: &Config) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let words = |text: &str, into: &mut BTreeSet<String>| {
        for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')) {
            let word = word.trim_matches('-');
            if !word.is_empty() && word.len() <= 4 {
                into.insert(word.to_owned());
            }
        }
    };
    let mut dirs = vec![config.app_dir(), config.root.join("src"), config.public_dir(), config.root.join("client")];
    dirs.push(config.root.join("assets"));
    dirs.extend(config.tailwind.sources.iter().map(|s| config.resolve(s)));
    for dir in dirs {
        for (_, path) in embeddable_files(&dir) {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if css_usage::SKIP.contains(&ext.as_str()) && ext != "css" {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            match ext.as_str() {
                // Identifiers never become classes; strings may.
                "rs" | "js" | "mjs" => {
                    for literal in string_literals(&text) {
                        words(literal, &mut names);
                    }
                }
                "css" => names.extend(next_rust_assets::css::class_selectors(&text).all),
                _ => words(&text, &mut names),
            }
        }
    }
    for stylesheet in &config.tailwind.stylesheets {
        if let Ok(css) = std::fs::read_to_string(config.resolve(stylesheet)) {
            names.extend(next_rust_assets::css::class_selectors(&css).all);
        }
    }
    names
}

/// The contents of the string literals in Rust or JavaScript source
/// (`"…"`, `'…'`, `` `…` `` and `r#"…"#`).
fn string_literals(source: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            q @ (b'"' | b'\'' | b'`') => {
                let start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != q && (q == b'`' || bytes[i] != b'\n') {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                out.push(&source[start..i.min(bytes.len())]);
            }
            b'r' if bytes.get(i + 1).is_some_and(|b| *b == b'#' || *b == b'"')
                && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')) =>
            {
                let hashes = bytes[i + 1..].iter().take_while(|b| **b == b'#').count();
                if bytes.get(i + 1 + hashes) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                let start = i + 2 + hashes;
                let close = format!("\"{}", "#".repeat(hashes));
                let end = source[start..].find(&close).map_or(bytes.len(), |e| start + e);
                out.push(&source[start..end]);
                i = end + close.len();
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// How often each candidate appears as a token in the Rust source.
fn source_counts<'a>(config: &Config, candidates: &[&'a String]) -> HashMap<&'a str, usize> {
    let wanted: HashMap<&str, &'a str> = candidates.iter().map(|c| (c.as_str(), c.as_str())).collect();
    let mut counts = HashMap::new();
    for dir in crate::tailwind::scanned_dirs(config) {
        for (_, path) in embeddable_files(&dir) {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            for token in text.split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`')) {
                if let Some(class) = wanted.get(token) {
                    *counts.entry(*class).or_insert(0) += 1;
                }
            }
        }
    }
    counts
}

/// Short names in order of length (`a`, `k`, …, `q3`, …), with the letters
/// shuffled by the seed. Names start with a lowercase letter or `_` and go
/// on with lowercase letters, digits, `_` and `-`: valid in selectors and in
/// attribute names (`data-nr-class-<name>`) without escaping, and never
/// changed by the case-insensitive parts of HTML. That gives 27 one-character
/// and 1,026 two-character names.
struct ShortNames {
    first: Vec<u8>,
    rest: Vec<u8>,
    len: u32,
    index: u64,
}

impl ShortNames {
    fn new(seed: u64) -> Self {
        let mut rng = seed;
        let mut first: Vec<u8> = (b'a'..=b'z').chain(std::iter::once(b'_')).collect();
        let mut rest: Vec<u8> = (b'a'..=b'z').chain(b'0'..=b'9').chain(*b"_-").collect();
        shuffle(&mut first, &mut rng);
        shuffle(&mut rest, &mut rng);
        Self { first, rest, len: 1, index: 0 }
    }
}

impl Iterator for ShortNames {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let per_length = self.first.len() as u64 * (self.rest.len() as u64).pow(self.len - 1);
        if self.index == per_length {
            self.len += 1;
            self.index = 0;
        }
        let mut n = self.index;
        self.index += 1;
        let mut name = Vec::with_capacity(self.len as usize);
        for _ in 1..self.len {
            name.push(self.rest[(n % self.rest.len() as u64) as usize]);
            n /= self.rest.len() as u64;
        }
        name.push(self.first[n as usize]);
        name.reverse();
        Some(String::from_utf8(name).expect("ascii"))
    }
}

fn shuffle(items: &mut [u8], state: &mut u64) {
    for i in (1..items.len()).rev() {
        let j = (splitmix64(state) % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_continuations_join_class_lists() {
        assert_eq!(continued("a b \\\n        c-[x,\\\n   y] d"), "a b c-[x,y] d");
        assert_eq!(continued("plain"), "plain");
    }

    #[test]
    fn short_names_are_unique_and_short_first() {
        let names: Vec<String> = ShortNames::new(7).take(27 + 27 * 38 + 5).collect();
        assert!(names[..27].iter().all(|n| n.len() == 1));
        assert!(names[27..27 + 1026].iter().all(|n| n.len() == 2));
        assert_eq!(names[27 + 1026].len(), 3);
        assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len());
        assert!(names.iter().all(|n| n.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')));
        assert_ne!(ShortNames::new(1).take(40).collect::<Vec<_>>(), ShortNames::new(2).take(40).collect::<Vec<_>>());
    }

    #[test]
    fn shortens_utilities_but_not_context_or_outside_classes() {
        let root = std::env::temp_dir().join(format!("nr-class-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("public")).unwrap();
        std::fs::write(
            root.join("app/page.rs"),
            r#"div![class("group mt-4"), p![class("mt-4 hover:underline is-open")], raw_html("<b class=\"tk\">")]"#,
        )
        .unwrap();
        std::fs::write(root.join("public/menu.js"), "el.classList.toggle('is-open')").unwrap();
        let mut config = Config::from_toml_str("[tailwind]\nenabled = true\nkeep_classes = [\"kept\"]").unwrap();
        config.root = root.clone();
        let css = r".mt-4{margin-top:1rem}.hover\:underline{&:hover{text-decoration:underline}}.is-open{display:block}.kept{color:red}.group-hover\:block:is(:where(.group):hover *){display:block}.\[\&_\.tk\]\:text-red .tk{color:red}";

        let out = shorten(css, &config, 42, &[]);
        let names: BTreeMap<_, _> = out.names.iter().cloned().collect();
        let shortened: Vec<&str> = names.keys().map(String::as_str).collect();
        assert_eq!(shortened, ["[&_.tk]:text-red", "group", "group-hover:block", "hover:underline", "mt-4"]);
        // `mt-4` is used most, so it gets a one-letter name.
        assert_eq!(names["mt-4"].len(), 1);
        for short in names.values() {
            assert!(!["group", "tk", "is-open", "kept", "b", "p", "el"].contains(&short.as_str()));
        }
        let expected = format!(
            ".{}{{margin-top:1rem}}.{}{{&:hover{{text-decoration:underline}}}}.is-open{{display:block}}.kept{{color:red}}.{}:is(:where(.{}):hover *){{display:block}}.{} .tk{{color:red}}",
            names["mt-4"],
            names["hover:underline"],
            names["group-hover:block"],
            names["group"],
            names["[&_.tk]:text-red"]
        );
        assert_eq!(out.css, expected);
        // Another seed, other names.
        assert_ne!(shorten(css, &config, 43, &[]).names, out.names);
        // Pages always get the rules of classes scripts add, and kept ones.
        assert_eq!(outside_classes(css, &config), ["kept"]);
        assert_eq!(script_classes(css, &config), [("menu.js".to_owned(), vec!["is-open".to_owned()])]);
        assert_eq!(map_source(&script_classes(css, &config)), "&[\n    (\"menu.js\", &[\"is-open\"]),\n]");
        std::fs::remove_dir_all(root).unwrap();
    }
}
