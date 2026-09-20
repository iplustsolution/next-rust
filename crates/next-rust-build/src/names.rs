//! Short names for release builds (`[build] minify_names`): `data-*`
//! attributes, CSS custom properties, `@keyframes` and `@layer` names, and
//! the ids the project lists in `[build] rename_ids`.
//!
//! The build reads everything that can name one of these (the compiled
//! stylesheet, the UI components' stylesheet and script, the scripts in
//! `client/` and `assets/`, static files and the strings of the Rust source)
//! and gives a short random name to every name it can follow everywhere:
//!
//! * The scripts in `client/` and `assets/` are rewritten here, in their
//!   string literals (`"[data-open]"`, `"--mx"`, `"#menu"`) and `dataset`
//!   accesses (`el.dataset.open`).
//! * Stylesheets and HTML are rewritten by the server as a response leaves
//!   (`next_rust_assets::names`), with the table written to `$OUT_DIR`.
//!
//! A name keeps its spelling when something the build cannot rewrite knows
//! it: a file under `public/`, a script or selector inside a Rust string, a
//! regular expression, a name assembled at runtime (`"data-" + key`,
//! `dataset[key]`), the `islands/` crate (its strings are compiled into a
//! WebAssembly module), the framework's own protocol (`data-nr-*`), or
//! `[build] keep_names`. The build report lists what was kept and why.
//!
//! This makes pages smaller and hard to read. It hides nothing: every name
//! a browser needs is in the page, only shorter.
//!
//! ## What is deliberately left readable
//!
//! Four more transforms were measured and rejected. The byte figures are
//! from the pages of a real site (home, sign-in, sign-up), Brotli quality 11.
//!
//! * **Form field names.** `name="email"` could be mapped per build and
//!   mapped back on the server, but the mapping has no safe edge. Browsers
//!   match saved logins and autofill on `name` (with `autocomplete`), so a
//!   name that changes every deploy breaks password managers for good. A
//!   page held by a CDN, a static export or a browser posts the names of the
//!   build that rendered it, which a newer build no longer knows, so fields
//!   would go missing after a deploy instead of failing loudly. And the name
//!   is also the key of `FormState`, of per-field errors
//!   (`data-nr-error="email"`), of multipart uploads and of the action's own
//!   input struct, so the map would have to reach all of them. It would save
//!   roughly 200 bytes a page. Field names stay as written.
//! * **Island props keys.** The island's component name goes (nothing reads
//!   `data-component`), but the keys inside `data-props` stay: they are read
//!   by the island's own module, usually by destructuring inside
//!   `hydrate(el, props)`, which the build cannot follow back to the key it
//!   came from. Renaming the JSON without the module is a silent break.
//! * **Repeated icons.** Turning duplicate inline `<svg>` drawings into one
//!   `<symbol>` and `<use>` saves 17 kB raw on the busiest page but 31 bytes
//!   after Brotli, and costs bytes on the other two (170 and 132): a
//!   compressor already knows a repeated drawing. `<use>` would also change
//!   how selectors and `currentColor` reach the drawing.
//! * **Sorted class lists.** Sorting the classes inside every `class`
//!   attribute makes pages *bigger* after compression (310, 81 and 33
//!   bytes): the order an author writes repeats across similar elements, and
//!   sorting breaks those repeats.

use std::collections::{BTreeMap, BTreeSet};

use next_rust_assets::names as shared;
use next_rust_core::Config;

use crate::codegen::embeddable_files;
use crate::css_usage;

/// File in `$OUT_DIR` with the table as a Rust slice expression.
pub const FILE: &str = "next_rust_names.rs";

/// Names the client runtime reads or sets. It is served as written, so
/// these keep their spelling (a test in the server crate keeps this list
/// complete); so does everything under `data-nr-`.
pub const RUNTIME_NAMES: &[&str] = shared::RUNTIME_NAMES;

/// The ids the UI components generate (`nr-field-3`, `nr-listbox-0-1`): no
/// stylesheet, script or link can name one, so they are always renamed.
const GENERATED_IDS: &[&str] = &["nr-accordion-+", "nr-field-+", "nr-listbox-+", "nr-tabs-+"];

/// The result of [`plan`].
#[derive(Debug, Default, Clone)]
pub struct Plan {
    /// `(key, short name)`, sorted by key (see `next_rust_assets::names`).
    pub table: Vec<(String, String)>,
    /// Names that keep their spelling, with the reason.
    pub kept: Vec<(String, String)>,
    /// `[build] rename_ids`: a script string that is one of these ids is
    /// renamed as a whole (`getElementById("menu")`).
    pub ids: Vec<String>,
}

impl Plan {
    /// The short form of `key`.
    pub fn find(&self, key: &str) -> Option<String> {
        shared::find(&self.table, key)
    }
}

/// What the project's files say about names.
#[derive(Debug, Default)]
struct Found {
    /// Candidates, with how often each was seen (the most used get the
    /// shortest names).
    seen: BTreeMap<String, usize>,
    /// Names that keep their spelling, with the first reason found.
    pinned: BTreeMap<String, String>,
    /// Families that keep their spelling (`data-chart-`, `--`), with reasons.
    pinned_prefixes: BTreeMap<String, String>,
    /// Every word of files the build cannot rewrite.
    outside_words: BTreeSet<String>,
    /// Words of script and Rust strings (keyframes and layers named there
    /// keep their names).
    string_words: BTreeSet<String>,
}

impl Found {
    fn see(&mut self, key: &str) {
        *self.seen.entry(key.to_owned()).or_insert(0) += 1;
    }

    fn pin(&mut self, key: &str, why: &str) {
        self.pinned.entry(key.to_owned()).or_insert_with(|| why.to_owned());
    }

    fn pin_prefix(&mut self, prefix: &str, why: &str) {
        self.pinned_prefixes.entry(prefix.to_owned()).or_insert_with(|| why.to_owned());
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// The `data-*` and `--*` tokens of `text`; a token that ends in `-` (or is
/// only the prefix) is the start of a family of names.
fn tokens(text: &str, f: &mut dyn FnMut(&str)) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let boundary = i == 0 || !is_name_char(bytes[i - 1] as char);
        if boundary && (bytes[i..].starts_with(b"data-") || bytes[i..].starts_with(b"--")) {
            let end = i + bytes[i..].iter().take_while(|b| is_name_char(**b as char)).count();
            f(&text[i..end]);
            i = end;
        } else {
            i += 1;
        }
    }
}

fn words(text: &str, into: &mut BTreeSet<String>) {
    for word in text.split(|c: char| !is_name_char(c)) {
        if !word.is_empty() && word.len() <= 64 {
            into.insert(word.to_owned());
        }
    }
}

/// A file the build serves as written: every name in it is kept.
fn outside_file(found: &mut Found, text: &str, why: &str) {
    tokens(text, &mut |t| {
        if t.ends_with('-') {
            found.pin_prefix(t, why);
        } else {
            found.pin(t, why);
        }
    });
    words(text, &mut found.outside_words);
}

/// Whether a Rust string holds a script: the names in it cannot be followed.
fn looks_like_script(literal: &str) -> bool {
    ["document.", "window.", "=>", "function", "querySelector", ".style.", "getAttribute", "setAttribute", "dataset"]
        .iter()
        .any(|m| literal.contains(m))
}

/// The strings of the project's Rust source. An attribute name on its own
/// (`"data-state"`, `data("state", …)`) and a custom property in a
/// declaration (`"--x: 1"`, `"var(--x)"`) reach the page through the
/// renderer; anything else (a selector, a script, a bare `"--x"`) may go
/// anywhere, so those names are kept.
fn rust_source(found: &mut Found, source: &str) {
    let declared = crate::rust_sources::declared_literals(source);
    for literal in rust_strings(source) {
        if declared.iter().any(|d| d == literal) {
            continue;
        }
        let script = looks_like_script(literal);
        let bytes = literal.as_bytes();
        let mut at = 0;
        tokens(literal, &mut |t| {
            let start = at + literal[at..].find(t).unwrap_or(0);
            let end = start + t.len();
            at = end;
            // Inside a Tailwind class (`group-has-[[data-on]:checked]:block`,
            // `[--x:1rem]`): it only reaches the stylesheet Tailwind writes.
            let word_start = literal[..start].rfind(|c: char| c.is_whitespace()).map_or(0, |p| p + 1);
            let word_end = literal[end..].find(|c: char| c.is_whitespace()).map_or(literal.len(), |p| end + p);
            let word = &literal[word_start..word_end];
            if !script && word.len() > t.len() && is_tailwind_class(word) {
                if !t.ends_with('-') {
                    found.see(t);
                }
                return;
            }
            if t.ends_with('-') {
                // `format!("data-{name}")` builds names; `data-[state=on]:flex`
                // (a Tailwind variant) and `<!--` do not.
                if literal[end..].starts_with('{') {
                    found.pin_prefix(t, "a Rust string builds names with this prefix");
                }
                return;
            }
            let followed_by_colon = literal[end..].trim_start().starts_with(':');
            let in_var = literal[..start].trim_end().ends_with("var(");
            let obvious = if t.starts_with("data-") {
                literal == t
            } else {
                (followed_by_colon || in_var) && !(start > 0 && bytes[start - 1] == b'"')
            };
            if obvious && !script {
                found.see(t);
            } else {
                found.pin(t, "named in a Rust string the build cannot follow (a script, a selector or a bare name)");
            }
        });
        words(literal, &mut found.string_words);
    }
    // `data("state", …)`: the attribute `data-state`.
    for (at, _) in source.match_indices("data(\"") {
        let rest = &source[at + "data(\"".len()..];
        if let Some(end) = rest.find('"')
            && !rest[..end].is_empty()
            && rest[..end].chars().all(is_name_char)
            && !(at > 0 && is_name_char(source.as_bytes()[at - 1] as char))
        {
            found.see(&format!("data-{}", &rest[..end]));
        }
    }
}

/// Whether a word of a Rust string is a Tailwind class with a variant or an
/// arbitrary value: `data-[state=on]:flex`, `text-[var(--x)]`, `[--x:1]`.
fn is_tailwind_class(word: &str) -> bool {
    let name = word.strip_prefix(['!', '-']).unwrap_or(word);
    name.starts_with(|c: char| c.is_ascii_lowercase() || c == '[')
        && name.contains(['[', ':'])
        && !name.contains("://")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || "-_:/[]().%,#+!&*>~=@'".contains(c))
        && name.matches('[').count() == name.matches(']').count()
}

/// The string literals of Rust source (`"…"`, `r#"…"#`), outside comments.
///
/// Shared with `class_names`, which reads the same literals out of the islands
/// crate: a name compiled into a WebAssembly module cannot be rewritten later,
/// whether it is a `data-*` attribute or a class.
pub(crate) fn rust_strings(source: &str) -> Vec<&str> {
    let b = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                i = source[i..].find('\n').map_or(b.len(), |p| i + p);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = source[i + 2..].find("*/").map_or(b.len(), |p| i + 2 + p + 2);
                continue;
            }
            // A char literal (`'"'`), not a lifetime.
            b'\'' => {
                let close = if b.get(i + 1) == Some(&b'\\') { 3 } else { 2 };
                if b.get(i + close) == Some(&b'\'') {
                    i += close + 1;
                    continue;
                }
            }
            b'"' => {
                let start = i + 1;
                i = start;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                let end = i.min(b.len());
                if source.is_char_boundary(end) {
                    out.push(&source[start..end]);
                }
            }
            b'r' if matches!(b.get(i + 1), Some(b'#' | b'"'))
                && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) =>
            {
                let hashes = b[i + 1..].iter().take_while(|c| **c == b'#').count();
                if b.get(i + 1 + hashes) == Some(&b'"') {
                    let start = i + 2 + hashes;
                    let close = format!("\"{}", "#".repeat(hashes));
                    let end = source[start..].find(&close).map_or(b.len(), |e| start + e);
                    out.push(&source[start..end]);
                    i = end + close.len();
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// What a script says about names, and where it can be rewritten.
#[derive(Debug, Default)]
pub struct ScriptNames {
    /// Byte ranges of string contents (literals, template parts).
    strings: Vec<(usize, usize)>,
    /// Byte ranges of the `key` in `x.dataset.key`, with the attribute.
    dataset: Vec<(usize, usize, String)>,
    /// Names in regular expressions and in `dataset["…"]`.
    fixed: Vec<String>,
    /// `dataset[expr]`: any string of the script may be a `dataset` key.
    computed_dataset: bool,
    /// `dataset` used as a whole (`{...el.dataset}`, `Object.keys(el.dataset)`).
    opaque_dataset: bool,
}

/// Parse a script and find its strings and `dataset` accesses.
#[cfg(feature = "minify-js")]
pub fn analyze_script(source: &str) -> Result<ScriptNames, String> {
    use oxc_allocator::Allocator;
    use oxc_ast_visit::Visit;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    struct Finder {
        out: ScriptNames,
        /// Starts of `x.dataset` expressions, and of those a member access uses.
        dataset_at: BTreeSet<u32>,
        consumed: BTreeSet<u32>,
    }

    impl Finder {
        fn is_dataset(e: &oxc_ast::ast::Expression<'_>) -> Option<u32> {
            match e {
                oxc_ast::ast::Expression::StaticMemberExpression(m) if m.property.name == "dataset" => {
                    Some(m.span.start)
                }
                oxc_ast::ast::Expression::ParenthesizedExpression(p) => Self::is_dataset(&p.expression),
                _ => None,
            }
        }
    }

    impl<'a> Visit<'a> for Finder {
        fn visit_string_literal(&mut self, it: &oxc_ast::ast::StringLiteral<'a>) {
            let (start, end) = (it.span.start as usize, it.span.end as usize);
            if end > start + 1 {
                self.out.strings.push((start + 1, end - 1));
            }
        }

        fn visit_template_element(&mut self, it: &oxc_ast::ast::TemplateElement<'a>) {
            self.out.strings.push((it.span.start as usize, it.span.end as usize));
        }

        fn visit_reg_exp_literal(&mut self, it: &oxc_ast::ast::RegExpLiteral<'a>) {
            if let Some(raw) = &it.raw {
                tokens(raw.as_str(), &mut |t| self.out.fixed.push(t.to_owned()));
            }
        }

        fn visit_static_member_expression(&mut self, it: &oxc_ast::ast::StaticMemberExpression<'a>) {
            if it.property.name == "dataset" {
                self.dataset_at.insert(it.span.start);
            }
            if let Some(at) = Self::is_dataset(&it.object) {
                self.consumed.insert(at);
                let span = it.property.span;
                self.out.dataset.push((
                    span.start as usize,
                    span.end as usize,
                    shared::dataset_attr(it.property.name.as_str()),
                ));
            }
            oxc_ast_visit::walk::walk_static_member_expression(self, it);
        }

        fn visit_computed_member_expression(&mut self, it: &oxc_ast::ast::ComputedMemberExpression<'a>) {
            if let Some(at) = Self::is_dataset(&it.object) {
                self.consumed.insert(at);
                match &it.expression {
                    oxc_ast::ast::Expression::StringLiteral(s) => {
                        self.out.fixed.push(shared::dataset_attr(s.value.as_str()));
                    }
                    _ => self.out.computed_dataset = true,
                }
            }
            oxc_ast_visit::walk::walk_computed_member_expression(self, it);
        }
    }

    let result = std::panic::catch_unwind(|| {
        let allocator = Allocator::default();
        let parsed = [SourceType::mjs(), SourceType::cjs().with_module(false)]
            .into_iter()
            .map(|t| Parser::new(&allocator, source, t).parse())
            .find(|p| p.errors.is_empty())
            .ok_or_else(|| "the script does not parse".to_owned())?;
        let mut finder = Finder { out: ScriptNames::default(), dataset_at: BTreeSet::new(), consumed: BTreeSet::new() };
        finder.visit_program(&parsed.program);
        finder.out.opaque_dataset = finder.dataset_at.difference(&finder.consumed).next().is_some();
        Ok(finder.out)
    });
    result.unwrap_or_else(|_| Err("the script analysis failed unexpectedly".into()))
}

#[cfg(not(feature = "minify-js"))]
pub fn analyze_script(_source: &str) -> Result<ScriptNames, String> {
    Err("next-rust-build was compiled without the `minify-js` feature".into())
}

impl ScriptNames {
    /// `source` with its names replaced through `plan`, and how many were.
    pub fn apply(&self, source: &str, plan: &Plan) -> (String, usize) {
        let find = |key: &str| plan.find(key);
        let mut edits: Vec<(usize, usize, String)> = Vec::new();
        for (start, end) in &self.strings {
            let text = &source[*start..*end];
            // A string that is an id the project lets the build rename.
            let whole = if listed(&plan.ids, text) { find(&format!("#{text}")) } else { None };
            match whole {
                Some(short) => edits.push((*start, *end, short)),
                None => {
                    if let std::borrow::Cow::Owned(renamed) = shared::rename_tokens(text, &find) {
                        edits.push((*start, *end, renamed));
                    }
                }
            }
        }
        for (start, end, attr) in &self.dataset {
            if let Some(short) = find(attr).filter(|s| !s.is_empty()) {
                edits.push((*start, *end, shared::dataset_key(&short)));
            }
        }
        edits.sort_by_key(|e| e.0);
        let mut out = String::with_capacity(source.len());
        let mut last = 0;
        let mut count = 0;
        for (start, end, text) in edits {
            if start < last {
                continue;
            }
            out.push_str(&source[last..start]);
            out.push_str(&text);
            last = end;
            count += 1;
        }
        out.push_str(&source[last..]);
        (out, count)
    }
}

/// A script the build rewrites: its strings are candidates, what it builds
/// at runtime is kept.
fn script(found: &mut Found, path: &str, source: &str) {
    let names = match analyze_script(source) {
        Ok(names) => names,
        Err(e) => {
            println!("cargo:warning=Next Rust: the names in {path} keep their spelling; it could not be analyzed: {e}");
            return outside_file(found, source, &format!("{path} could not be analyzed"));
        }
    };
    let mut plain = BTreeSet::new();
    for (start, end) in &names.strings {
        let text = &source[*start..*end];
        let mut rest = text;
        tokens(text, &mut |t| {
            let at = rest.find(t).map_or(0, |p| p + t.len());
            rest = &rest[at..];
            // `"data-" + key`, `--${name}`: the rest of the name is made at runtime.
            if t.ends_with('-') && rest.is_empty() {
                found.pin_prefix(t, &format!("{path} builds names with this prefix"));
            } else if !t.ends_with('-') {
                found.see(t);
            }
        });
        if !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            plain.insert(text.to_owned());
        }
        words(text, &mut found.string_words);
    }
    for (_, _, attr) in &names.dataset {
        found.see(attr);
    }
    for name in &names.fixed {
        found.pin(name, &format!("{path} names it in a regular expression or `dataset[\"…\"]`"));
    }
    if names.opaque_dataset {
        found.pin_prefix("data-", &format!("{path} reads `dataset` as a whole"));
    }
    if names.computed_dataset {
        for key in plain {
            found.pin(&shared::dataset_attr(&key), &format!("{path} uses `dataset[…]` and the string \"{key}\""));
        }
    }
}

/// The project's own stylesheets: `.css` files next to the source and the
/// stylesheets declared in Rust. The compiled Tailwind stylesheet, which
/// imports most of them, is added by the caller.
pub fn project_stylesheets(config: &Config) -> Vec<String> {
    let mut sheets = Vec::new();
    let mut dirs = vec![config.app_dir(), config.root.join("src"), config.root.join("styles")];
    if let Some(generated) = &config.generated {
        dirs.push(generated.clone());
    }
    for dir in dirs {
        for (_, path) in embeddable_files(&dir) {
            if path.extension().is_some_and(|e| e == "css")
                && let Ok(css) = std::fs::read_to_string(&path)
            {
                sheets.push(css);
            }
        }
    }
    for sheet in &config.tailwind.stylesheets {
        if let Some(css) = crate::rust_sources::stylesheet_source(&config.resolve(sheet)) {
            sheets.push(css);
        }
    }
    sheets
}

/// The ids a string names: `#menu` in a selector, and a string that is
/// nothing but an id-shaped word (`getElementById("menu")`).
fn ids_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    for (at, _) in text.match_indices('#') {
        let start = at + 1;
        if b.get(start).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_') {
            let end = start + b[start..].iter().take_while(|c| is_name_char(**c as char)).count();
            out.push(text[start..end].to_owned());
        }
    }
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        out.push(text.to_owned());
    }
    out
}

/// Whether `name` matches an entry of a list: the name itself, or `prefix*`.
fn listed(list: &[String], name: &str) -> bool {
    list.iter().any(|entry| match entry.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => entry == name || entry.trim_start_matches('#') == name,
    })
}

/// How often `word` is a whole token of `text`.
fn count_word(text: &str, word: &str) -> usize {
    text.match_indices(word)
        .filter(|(at, _)| {
            let before = text[..*at].chars().next_back();
            let after = text[at + word.len()..].chars().next();
            !before.is_some_and(is_name_char) && !after.is_some_and(is_name_char)
        })
        .count()
}

/// Decide the short names of a build. `stylesheets` are the stylesheets of
/// the app as the server sends them (the compiled Tailwind CSS, the UI
/// components' CSS); `ui_script` is the components' script when the app uses
/// them.
pub fn plan(config: &Config, stylesheets: &[&str], ui_script: Option<&str>, seed: u64) -> Plan {
    let mut found = Found::default();

    // Stylesheets: every name, and keyframes named outside an animation.
    let mut keyframes_in_context: BTreeMap<String, usize> = BTreeMap::new();
    for css in stylesheets {
        let seen = std::cell::RefCell::new(Vec::new());
        shared::rename_css(css, &|key| {
            seen.borrow_mut().push(key.to_owned());
            None
        });
        for key in seen.into_inner() {
            if let Some(name) = key.strip_prefix("@k:") {
                *keyframes_in_context.entry(name.to_owned()).or_insert(0) += 1;
            }
            // Ids are renamed only when the project asks for them.
            if !key.starts_with('#') {
                found.see(&key);
            }
        }
    }
    // An `animation` also holds keywords (`ease-in`, `both`): only what has
    // `@keyframes` is a name.
    let mut defined = BTreeSet::new();
    for css in stylesheets {
        for (at, _) in css.match_indices("keyframes") {
            let rest = css[at + "keyframes".len()..].trim_start();
            let end = rest.find(|c: char| !is_name_char(c)).unwrap_or(rest.len());
            if at > 0 && !rest[..end].is_empty() {
                defined.insert(rest[..end].to_owned());
            }
        }
    }
    found.seen.retain(|key, _| key.strip_prefix("@k:").is_none_or(|name| defined.contains(name)));
    for (name, in_context) in keyframes_in_context.iter().filter(|(name, _)| defined.contains(*name)) {
        let total: usize = stylesheets.iter().map(|css| count_word(css, name)).sum();
        if total > *in_context {
            found.pin(&format!("@k:{name}"), "the stylesheet names it outside an `animation`");
        }
    }

    // The UI components' script is rewritten by the server, as a whole.
    if let Some(js) = ui_script {
        tokens(js, &mut |t| {
            if !t.ends_with('-') {
                found.see(t);
            }
        });
        for (at, _) in js.match_indices("dataset.") {
            let rest = &js[at + "dataset.".len()..];
            let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
            found.see(&shared::dataset_attr(&rest[..end]));
        }
    }

    // Scripts the build rewrites.
    for field in ["client", "assets"] {
        for (rel, path) in crate::rust_sources::files(config, field) {
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let text = String::from_utf8_lossy(&bytes);
            let key = format!("{field}/{rel}");
            if matches!(rel.rsplit('.').next(), Some("js" | "mjs")) {
                script(&mut found, &key, &text);
            } else if rel.ends_with(".css") || !css_usage::SKIP.contains(&rel.rsplit('.').next().unwrap_or("")) {
                outside_file(&mut found, &text, &format!("{key} is served as written"));
            }
        }
    }
    // Files served as written.
    let mut outside = vec![config.public_dir()];
    outside.extend(config.tailwind.sources.iter().map(|s| config.resolve(s)));
    for dir in outside {
        for (rel, path) in embeddable_files(&dir) {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if ext == "rs" || (css_usage::SKIP.contains(&ext.as_str()) && ext != "css") {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                outside_file(&mut found, &String::from_utf8_lossy(&bytes), &format!("{rel} is served as written"));
            }
        }
    }
    // The Rust source.
    for dir in [config.app_dir(), config.root.join("src")] {
        for (_, path) in embeddable_files(&dir) {
            if path.extension().is_some_and(|e| e == "rs")
                && let Ok(source) = std::fs::read_to_string(&path)
            {
                rust_source(&mut found, &source);
            }
        }
    }
    // The islands crate (`[build] wasm_islands`) becomes a WebAssembly
    // module: the names in it end up inside the binary, where nothing can
    // rewrite them, so every name it mentions keeps its spelling.
    let islands = config.root.join("islands");
    if config.build.wasm_islands && islands.is_dir() {
        for (rel, path) in embeddable_files(&islands) {
            if path.extension().is_some_and(|e| e == "rs")
                && let Ok(source) = std::fs::read_to_string(&path)
            {
                for literal in rust_strings(&source) {
                    outside_file(&mut found, literal, &format!("islands/{rel} compiles it into WebAssembly"));
                    for id in ids_in(literal) {
                        found.pin(&format!("#{id}"), &format!("islands/{rel} compiles it into WebAssembly"));
                    }
                }
            }
        }
    }

    // Decide.
    let keep = &config.build.keep_names;
    let mut kept: Vec<(String, String)> = Vec::new();
    let mut renamable: Vec<(&String, usize)> = Vec::new();
    for (key, count) in &found.seen {
        let name = key.split_once(':').map_or(key.as_str(), |(_, n)| n);
        let why = if listed(keep, key) || listed(keep, name) {
            Some("`[build] keep_names`".to_owned())
        } else if key.starts_with("data-nr-") || RUNTIME_NAMES.contains(&key.as_str()) {
            Some("part of the framework's protocol with its client runtime".to_owned())
        } else if let Some(why) = found.pinned.get(key) {
            Some(why.clone())
        } else if let Some((_, why)) = found.pinned_prefixes.iter().find(|(p, _)| key.starts_with(p.as_str())) {
            Some(why.clone())
        } else if key.starts_with('@') && found.outside_words.contains(name) {
            Some("a file served as written names it".to_owned())
        } else if key.starts_with("@k:") && found.string_words.contains(name) {
            Some("a script or Rust string names it".to_owned())
        } else {
            None
        };
        match why {
            Some(why) => kept.push((key.clone(), why)),
            None => renamable.push((key, *count)),
        }
    }
    // The island's component name is for people; nothing reads it unless
    // the project itself names the attribute.
    let drop_component = !found.seen.contains_key("data-component")
        && !found.pinned.contains_key("data-component")
        && !listed(keep, "data-component")
        && !found.pinned_prefixes.keys().any(|p| "data-component".starts_with(p.as_str()));

    // Most used first: they get the shortest names.
    renamable.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let taken: BTreeSet<&str> = found.seen.keys().chain(found.pinned.keys()).map(String::as_str).collect();
    let mut table: BTreeMap<String, String> = BTreeMap::new();
    let mut generators: BTreeMap<&str, crate::class_names::ShortNames> = BTreeMap::new();
    let mut next = |kind: &'static str, accept: &dyn Fn(&str) -> bool| -> String {
        let salt = kind.bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
        let generator = generators.entry(kind).or_insert_with(|| crate::class_names::ShortNames::new(seed ^ salt));
        generator.by_ref().find(|n| accept(n)).expect("names never run out")
    };
    for (key, _) in renamable {
        let short = if key.starts_with("data-") {
            // Letters and digits only: `dataset.k7` needs no camel case.
            let short = next("data", &|n| {
                n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                    && !n.starts_with("nr")
                    && !taken.contains(format!("data-{n}").as_str())
                    && !found.outside_words.contains(&format!("data-{n}"))
            });
            format!("data-{short}")
        } else if key.starts_with("--") {
            let short = next("prop", &|n| {
                !taken.contains(format!("--{n}").as_str())
                    && !found.outside_words.contains(&format!("--{n}"))
                    && !n.starts_with("nr-")
                    && !n.ends_with('-')
            });
            format!("--{short}")
        } else if key.starts_with("@k:") {
            next("keyframes", &|n| {
                !taken.contains(format!("@k:{n}").as_str()) && !CSS_WORDS.contains(&n) && !n.ends_with('-')
            })
        } else if key.starts_with("@l:") {
            next("layer", &|n| {
                !taken.contains(format!("@l:{n}").as_str()) && !CSS_WORDS.contains(&n) && !n.ends_with('-')
            })
        } else {
            continue;
        };
        table.insert(key.clone(), short);
    }
    // Ids: the UI components' generated ones and the project's list. The
    // short prefix ends in `-` and an id on its own never does, so the two
    // cannot meet.
    let all_words: BTreeSet<&str> = found.outside_words.iter().chain(&found.string_words).map(String::as_str).collect();
    for pattern in GENERATED_IDS.iter().map(|p| (*p).to_owned()).chain(config.build.rename_ids.iter().cloned()) {
        let pattern = pattern.trim_start_matches('#').to_owned();
        if pattern.is_empty() || listed(keep, &pattern) || listed(keep, &format!("#{pattern}")) {
            continue;
        }
        // An id something unrewritable names keeps its spelling, whatever
        // `rename_ids` says; for a family, one such id keeps the family.
        let prefix = pattern.trim_end_matches(['*', '+']);
        if let Some((id, why)) = found.pinned.iter().find(|(k, _)| {
            k.strip_prefix('#').is_some_and(|id| if pattern == prefix { id == prefix } else { id.starts_with(prefix) })
        }) {
            kept.push((format!("#{pattern}"), format!("{} names {id}", why.clone())));
            continue;
        }
        let family = pattern.ends_with(['*', '+']);
        let short = next("id", &|n| {
            !n.ends_with('-')
                && !n.contains('_')
                && !all_words.contains(n)
                && !all_words.iter().any(|w| family && w.starts_with(&format!("{n}-")))
        });
        table.insert(format!("#{pattern}"), if family { format!("{short}-") } else { short });
    }
    if drop_component {
        table.insert("data-component".to_owned(), String::new());
    }
    for (prefix, why) in &found.pinned_prefixes {
        if prefix == "data-" || prefix == "--" || found.seen.keys().any(|k| k.starts_with(prefix.as_str())) {
            kept.push((format!("{prefix}*"), why.clone()));
        }
    }
    kept.sort();
    kept.dedup();
    let ids = config.build.rename_ids.iter().map(|id| id.trim_start_matches('#').to_owned()).collect();
    Plan { table: table.into_iter().collect(), kept, ids }
}

/// Words a short keyframes or layer name must not be: CSS reads them as
/// keywords where the name goes.
const CSS_WORDS: &[&str] = &["in", "to", "at", "or", "on", "s", "ms", "x", "y", "z", "e", "none", "all", "both"];

#[cfg(all(test, feature = "minify-js"))]
mod tests {
    use super::*;

    fn project(name: &str, files: &[(&str, &str)], toml: &str) -> Config {
        let root = std::env::temp_dir().join(format!("nr-names-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let mut config = Config::from_toml_str(toml).unwrap();
        config.root = root;
        config
    }

    const CSS: &str = "@layer theme,components;@layer theme{:root,[data-theme=dark]{--brand:#fff;--mx:0;--kept:1}}@layer components{.card[data-open]{color:var(--brand);animation:fade-up 1s ease-in both}#menu{top:var(--mx)}.spin{animation:spin 1s}[data-nr-state=error]{color:red;transition:var(--nr-page-fade)}[data-chart-a],[data-pick-a]{--from-public:1}}@keyframes fade-up{to{opacity:1}}@keyframes spin{to{rotate:1turn}}";

    #[test]
    fn plans_short_names_and_keeps_what_it_cannot_follow() {
        let config = project(
            "plan",
            &[
                (
                    "app/page.rs",
                    r##"// A comment naming `[data-open]` and "--brand" pins nothing.
                         div![data("open", ""), data("state", "x"), style("--mx: 4px"), id("menu"), attr("data-theme", "dark"),
                         class("group-has-[[data-open]:checked]:block text-[var(--brand)] [--mx:1rem] data-[open]:flex"),
                         script![r#"document.documentElement.setAttribute("data-js", "")"#], raw_html("<i data-raw>"), island_props("--bare")]"##,
                ),
                (
                    "client/menu.js",
                    r##"export function hydrate(el) {
                         el.querySelector("[data-open]").dataset.open = "";
                         el.style.setProperty("--mx", "1px");
                         for (const k of ["a", "b"]) el.setAttribute("data-chart-" + k, "");
                         el.dataset[el.id ? "pickA" : "pickB"] = "1";
                         document.getElementById("menu"); document.querySelector("#menu a, #auth-email");
                         if (/data-rx/.test(el.outerHTML)) el.dataset.rx = "";
                       }"##,
                ),
                ("public/legacy.js", "el.style.setProperty('--from-public', 1)"),
            ],
            "[build]\nkeep_names = [\"--kept\"]\nrename_ids = [\"menu\", \"auth-*\"]",
        );
        let plan = plan(&config, &[CSS], None, 7);
        let short = |key: &str| plan.find(key);
        for renamed in ["data-open", "--brand", "--mx", "@k:fade-up", "@l:theme", "@l:components", "#menu"] {
            assert!(short(renamed).is_some(), "{renamed} gets a short name: {plan:?}");
        }
        for keyword in ["@k:ease-in", "@k:both", "@k:1s"] {
            assert!(short(keyword).is_none(), "{keyword} is not a keyframes name");
        }
        assert!(short("data-open").unwrap().starts_with("data-"));
        assert!(short("--mx").unwrap().starts_with("--"));
        assert_eq!(short("#auth-email").map(|s| s.ends_with("-email")), Some(true));
        assert!(short("#nr-listbox-0-1").is_some_and(|s| s.ends_with("-0-1")));
        assert_eq!(short("data-component").as_deref(), Some(""), "nothing names data-component: it is left out");
        let kept: BTreeMap<&str, &str> = plan.kept.iter().map(|(k, w)| (k.as_str(), w.as_str())).collect();
        for (name, why) in [
            ("--kept", "keep_names"),
            ("data-nr-state", "protocol"),
            ("--nr-page-fade", "protocol"),
            ("data-chart-a", "builds names with this prefix"),
            ("--from-public", "served as written"),
            ("data-pick-a", "dataset[…]"),
            ("data-rx", "regular expression"),
            ("@k:spin", "outside an `animation`"),
        ] {
            assert!(short(name).is_none(), "{name} keeps its name");
            assert!(kept.get(name).is_some_and(|w| w.contains(why)), "{name}: {:?}", kept.get(name));
        }
        // Named in Rust strings the build cannot follow.
        for name in ["data-js", "data-raw", "--bare"] {
            assert!(short(name).is_none(), "{name} keeps its name: {plan:?}");
        }
        // Another seed, other names; the same seed, the same names.
        assert_ne!(super::plan(&config, &[CSS], None, 8).table, plan.table);
        assert_eq!(super::plan(&config, &[CSS], None, 7).table, plan.table);
        // Short names never collide within a kind.
        for kind in ["data-", "--", "@k:", "@l:", "#"] {
            let values: Vec<&String> =
                plan.table.iter().filter(|(k, v)| k.starts_with(kind) && !v.is_empty()).map(|(_, v)| v).collect();
            assert_eq!(values.iter().collect::<BTreeSet<_>>().len(), values.len(), "{kind}");
        }

        // The script, rewritten.
        let source = std::fs::read_to_string(config.root.join("client/menu.js")).unwrap();
        let (out, count) = analyze_script(&source).unwrap().apply(&source, &plan);
        let open = short("data-open").unwrap();
        assert!(out.contains(&format!("\"[{open}]\").dataset.{} = \"\"", &open["data-".len()..])), "{out}");
        assert!(out.contains(&format!("setProperty(\"{}\"", short("--mx").unwrap())), "{out}");
        assert!(out.contains(&format!("getElementById(\"{}\")", short("#menu").unwrap())), "{out}");
        assert!(
            out.contains(&format!("\"#{} a, #{}\"", short("#menu").unwrap(), short("#auth-email").unwrap())),
            "{out}"
        );
        assert!(out.contains("\"data-chart-\" + k") && out.contains("dataset.rx"), "{out}");
        assert_eq!(count, 5);
        std::fs::remove_dir_all(&config.root).unwrap();
    }

    #[test]
    fn a_script_reading_dataset_as_a_whole_keeps_every_attribute() {
        let config = project(
            "opaque",
            &[("client/all.js", "export const hydrate = (el) => console.log({ ...el.dataset }, el.dataset.open);")],
            "",
        );
        let plan = plan(&config, &["[data-open]{--x:1}"], None, 1);
        assert!(plan.find("data-open").is_none() && plan.find("--x").is_some(), "{plan:?}");
        assert!(plan.kept.iter().any(|(k, why)| k == "data-open" && why.contains("as a whole")));
        std::fs::remove_dir_all(&config.root).unwrap();
    }

    #[test]
    fn the_ui_script_is_renamed_like_a_parsed_one() {
        // The server rewrites the components' script as text; the result
        // must be what rewriting its parsed strings gives.
        let config = project("ui", &[], "");
        let plan = plan(&config, &[next_rust_ui::UI_CSS.css], Some(next_rust_ui::UI_JS_MIN), 3);
        assert!(plan.find("data-open").is_some() && plan.find("data-nr-ui").is_none(), "{plan:?}");
        let find = |key: &str| plan.find(key);
        for js in [next_rust_ui::UI_JS_MIN, next_rust_ui::UI_JS] {
            let as_text = shared::rename_js_text(js, &find);
            let (parsed, count) = analyze_script(js).unwrap().apply(js, &plan);
            assert!(count > 10, "{count}");
            assert_eq!(as_text, parsed);
            assert!(analyze_script(&as_text).is_ok(), "still a script");
        }
        std::fs::remove_dir_all(&config.root).ok();
    }
}
