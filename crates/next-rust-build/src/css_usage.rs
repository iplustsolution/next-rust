//! Names used by the project, for removing unused CSS from release builds.
//!
//! Every identifier-like word (`[A-Za-z0-9_-]+`) in the app directory, `src/`,
//! `public/` and `client/` counts as used, whether it appears in Rust, HTML,
//! JavaScript or SVG. CSS files themselves are not read, so a class that only
//! exists in a stylesheet is unused. The list is deliberately generous: a word
//! that merely looks like a class name keeps its rules.

use std::collections::BTreeSet;

use next_rust_core::Config;

use crate::codegen::embeddable_files;

/// File in `$OUT_DIR` with one used name per line.
pub const FILE: &str = "next_rust_css_usage.txt";

/// Extensions never read: stylesheets and common binary formats.
pub(crate) const SKIP: &[&str] = &[
    "css", "png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "woff", "woff2", "ttf", "otf", "eot", "mp4", "webm",
    "mp3", "wav", "pdf", "zip", "gz", "br", "wasm",
];

/// Sorted names used anywhere in the project, plus `[assets] css_safelist`.
pub fn collect(config: &Config) -> Vec<String> {
    let mut names = BTreeSet::new();
    let mut dirs = vec![config.app_dir(), config.root.join("src"), config.public_dir(), config.root.join("client")];
    dirs.extend(config.tailwind.sources.iter().map(|s| config.resolve(s)));
    for dir in dirs {
        for (_, path) in embeddable_files(&dir) {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if SKIP.contains(&ext.as_str()) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else { continue };
            words(&String::from_utf8_lossy(&bytes), &mut names);
        }
    }
    names.extend(config.assets.css_safelist.iter().cloned());
    names.into_iter().collect()
}

fn words(text: &str, into: &mut BTreeSet<String>) {
    for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')) {
        let word = word.trim_matches('-');
        if !word.is_empty() && word.len() <= 128 {
            into.insert(word.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_words_from_sources_but_not_css() {
        let root = std::env::temp_dir().join(format!("nr-css-usage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("public")).unwrap();
        std::fs::write(root.join("app/page.rs"), "div![class(\"hero card-title\"), p![class(styles.sub_title)]]")
            .unwrap();
        std::fs::write(root.join("app/globals.css"), ".only-in-css { color: red }").unwrap();
        std::fs::write(root.join("public/menu.js"), "el.classList.add('is-open')").unwrap();
        let mut config = Config::from_toml_str("[assets]\ncss_safelist = [\"from-safelist\"]").unwrap();
        config.root = root.clone();
        let names = collect(&config);
        for name in ["hero", "card-title", "sub_title", "is-open", "from-safelist"] {
            assert!(names.contains(&name.to_owned()), "{name} missing from {names:?}");
        }
        assert!(!names.contains(&"only-in-css".to_owned()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
