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
//! * The classes used most often in the source get the shortest names.
//! * A short name never equals a word used anywhere in the project or a class
//!   in the CSS, so it can't collide with a class that keeps its name.
//! * Classes that appear in `client/`, `public/` or extra Tailwind sources
//!   (scripts and HTML the renderer never sees) and `[tailwind] keep_classes`
//!   keep their names.

use std::collections::{BTreeMap, BTreeSet, HashMap};

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

/// Give the utility classes of `css` (minified Tailwind output) short names.
pub fn shorten(css: &str, config: &Config, seed: u64) -> Shortened {
    let found = next_rust_assets::css::class_selectors(css);
    let outside = outside_texts(config);
    let keep: BTreeSet<&str> = config.tailwind.keep_classes.iter().map(String::as_str).collect();
    let candidates: Vec<&String> = found
        .leading
        .iter()
        .filter(|class| !keep.contains(class.as_str()) && !outside.iter().any(|text| contains_class(text, class)))
        .collect();

    // Names that must never be used as a short name.
    let mut reserved: BTreeSet<String> = css_usage::collect(config).into_iter().collect();
    reserved.extend(found.all.iter().cloned());
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

/// The renaming table as a Rust expression of type `&[(&str, &str)]`.
pub fn table_source(names: &[(String, String)]) -> String {
    let mut out = String::from("&[\n");
    for (class, short) in names {
        out.push_str(&format!("    ({class:?}, {short:?}),\n"));
    }
    out.push(']');
    out
}

/// Text of files the renderer never sees: scripts, HTML and SVG in
/// `client/`, `public/` and extra Tailwind sources.
fn outside_texts(config: &Config) -> Vec<String> {
    let mut dirs = vec![config.public_dir(), config.root.join("client")];
    dirs.extend(config.tailwind.sources.iter().map(|s| config.resolve(s)));
    let mut texts = Vec::new();
    for dir in dirs {
        for (_, path) in embeddable_files(&dir) {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if ext == "rs" || css_usage::SKIP.contains(&ext.as_str()) {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                texts.push(String::from_utf8_lossy(&bytes).into_owned());
            }
        }
    }
    texts
}

/// Whether `class` appears in `text` as a whole class name.
fn contains_class(text: &str, class: &str) -> bool {
    let part = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':');
    text.match_indices(class).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(part) && !text[at + class.len()..].chars().next().is_some_and(part)
    })
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
/// shuffled by the seed. Names start with a letter and use only lowercase
/// letters and digits, so they are valid in selectors and in attribute names
/// (`data-nr-class-<name>`) without escaping.
struct ShortNames {
    first: Vec<u8>,
    rest: Vec<u8>,
    len: u32,
    index: u64,
}

impl ShortNames {
    fn new(seed: u64) -> Self {
        let mut rng = seed;
        let mut first: Vec<u8> = (b'a'..=b'z').collect();
        let mut rest: Vec<u8> = (b'a'..=b'z').chain(b'0'..=b'9').collect();
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
    fn short_names_are_unique_and_short_first() {
        let names: Vec<String> = ShortNames::new(7).take(26 + 26 * 36 + 5).collect();
        assert!(names[..26].iter().all(|n| n.len() == 1));
        assert!(names[26..26 + 936].iter().all(|n| n.len() == 2));
        assert_eq!(names[26 + 936].len(), 3);
        assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len());
        assert!(names.iter().all(|n| n.starts_with(|c: char| c.is_ascii_lowercase())));
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

        let out = shorten(css, &config, 42);
        let names: BTreeMap<_, _> = out.names.iter().cloned().collect();
        let shortened: Vec<&str> = names.keys().map(String::as_str).collect();
        assert_eq!(shortened, ["[&_.tk]:text-red", "group-hover:block", "hover:underline", "mt-4"]);
        // `mt-4` is used most, so it gets a one-letter name.
        assert_eq!(names["mt-4"].len(), 1);
        for short in names.values() {
            assert!(!["group", "tk", "is-open", "kept", "b", "p", "el"].contains(&short.as_str()));
        }
        let expected = format!(
            ".{}{{margin-top:1rem}}.{}{{&:hover{{text-decoration:underline}}}}.is-open{{display:block}}.kept{{color:red}}.{}:is(:where(.group):hover *){{display:block}}.{} .tk{{color:red}}",
            names["mt-4"], names["hover:underline"], names["group-hover:block"], names["[&_.tk]:text-red"]
        );
        assert_eq!(out.css, expected);
        // Another seed, other names.
        assert_ne!(shorten(css, &config, 43).names, out.names);
        std::fs::remove_dir_all(root).unwrap();
    }
}
