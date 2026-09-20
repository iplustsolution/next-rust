//! The names the WebAssembly component module knows are kept in release.
//!
//! `[build] minify_names` shortens `nr-*` classes, `data-*` attributes and
//! `--nr-*` custom properties in the markup, the stylesheet and the scripts
//! the build can rewrite. It cannot rewrite a wasm module — the names live in
//! a data section, and a shorter string would move every string after it — so
//! with `[ui] script = "wasm"` the build is told to keep them instead, from
//! [`next_rust_ui::UI_WASM_NAMES`].
//!
//! A name the module uses and the list does not carry is a **release-only**
//! break: everything works in development, and in production the component
//! renders unstyled or stops responding, with nothing in any log. That is
//! what this test is for.
//!
//! It reads the sibling crate's sources from disk rather than `include_str!`,
//! because a published `next-rust-ui` does not ship them; when they are not
//! there it says so and stops, which is the honest outcome for a package
//! being built on its own.

use std::path::{Path, PathBuf};

/// Where the component behaviour lives, relative to this crate.
fn wasm_crate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../next-rust-ui-wasm/src")
}

/// Every `nr-…`, `data-…` and `--nr-…` token in a source file.
fn names_in(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    for prefix in ["--nr-", "data-", "nr-"] {
        for (at, _) in source.match_indices(prefix) {
            // Only whole tokens: `data-nr-ui` is a name, the `nr-` inside
            // `nr-select-native` is not a second one.
            if at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'-') {
                continue;
            }
            let end = at
                + bytes[at..]
                    .iter()
                    .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || **b == b'-')
                    .count();
            let name = source[at..end].trim_end_matches('-').to_owned();
            if name.len() > prefix.len() && !found.contains(&name) {
                found.push(name);
            }
        }
    }
    found
}

/// Whether the component script names this, the way the renderer reads it:
/// literally, or as the `dataset.camelCase` an attribute is written through.
/// `next_rust_view::css_split::script_words` does the same translation, and
/// this test is only useful if it agrees with it.
fn script_declares(script: &str, name: &str) -> bool {
    if script.contains(name) {
        return true;
    }
    let Some(rest) = name.strip_prefix("data-") else { return false };
    let mut key = String::from("dataset.");
    let mut upper = false;
    for c in rest.chars() {
        if c == '-' {
            upper = true;
            continue;
        }
        key.push(if upper { c.to_ascii_uppercase() } else { c });
        upper = false;
    }
    script.contains(&key)
}

/// The list, as a set.
fn listed() -> Vec<&'static str> {
    next_rust_ui::UI_WASM_NAMES.lines().map(str::trim).filter(|line| !line.is_empty()).collect()
}

#[test]
fn every_name_the_wasm_components_use_is_kept_in_release_builds() {
    let dir = wasm_crate();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("skipped: {} is not in this package", dir.display());
        return;
    };
    let listed = listed();
    let mut missing: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        for name in names_in(&source) {
            if !listed.contains(&name.as_str()) {
                missing.push((file.clone(), name));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "add these to crates/next-rust-ui/src/wasm_names.txt, or a release build renames them out from under the \
         module: {missing:?}"
    );
}

/// The per-page stylesheet keeps the rules for names only the framework's
/// scripts set, and it learns them from [`next_rust_ui::UI_JS_MIN`]. The
/// wasm module must therefore not be the *only* thing that names a class or
/// custom property the stylesheet has a rule for — otherwise that rule is
/// pruned off every page and the component renders unstyled.
#[test]
fn the_wasm_module_names_no_styled_name_the_script_does_not() {
    let css = next_rust_ui::UI_CSS.css;
    let script = next_rust_ui::UI_JS_MIN;
    let mut orphans = Vec::new();
    for name in listed() {
        // Only names the stylesheet actually has a rule for matter here.
        if !css.contains(name) {
            continue;
        }
        if !script_declares(script, name) {
            orphans.push(name);
        }
    }
    assert!(
        orphans.is_empty(),
        "the stylesheet styles these and only the wasm module names them, so per-page CSS would drop the rules: \
         {orphans:?}"
    );
}

/// The list is what a human maintains, so it should read like one: sorted,
/// no duplicates, no blank lines, nothing that is not a name.
#[test]
fn the_kept_name_list_is_tidy() {
    let listed = listed();
    assert!(!listed.is_empty());
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    assert_eq!(listed, sorted, "crates/next-rust-ui/src/wasm_names.txt is not sorted");
    let mut deduped = sorted.clone();
    deduped.dedup();
    assert_eq!(deduped.len(), listed.len(), "a name is listed twice");
    for name in &listed {
        assert!(
            name.starts_with("nr-") || name.starts_with("data-") || name.starts_with("--nr-"),
            "`{name}` is not a class, a `data-` attribute or a custom property"
        );
        assert!(!name.ends_with('-'), "`{name}` looks truncated");
    }
}

/// Every value the renderer writes into `data-nr-ui` is listed for the
/// browser layer, whichever one is serving.
#[test]
fn the_component_kinds_are_a_tidy_list_too() {
    let kinds = next_rust_ui::UI_WASM_KINDS;
    let mut sorted = kinds.to_vec();
    sorted.sort_unstable();
    assert_eq!(kinds, sorted.as_slice(), "UI_WASM_KINDS is not sorted");
    sorted.dedup();
    assert_eq!(sorted.len(), kinds.len(), "a kind is listed twice");
    for kind in kinds {
        assert!(
            kind.chars().all(|c| c.is_ascii_lowercase()),
            "`{kind}` is not a plain lowercase word, and the markup writes it verbatim"
        );
    }
}

/// The kinds list is the contract between the renderer and the browser
/// layer, so it is checked against the renderer rather than trusted.
///
/// A kind the markup writes and the list omits is a component that the wasm
/// module silently never wires up — which is how `press` was missed until a
/// browser found it. The renderer writes the value two ways, through the
/// `attr` helper and through `Attr::new`, and both are read here.
#[test]
fn the_kinds_list_matches_what_the_renderer_writes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("skipped: {} is not readable", dir.display());
        return;
    };
    let mut written: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        for marker in ["\"data-nr-ui\", \"", "data(\"nr-ui\", \""] {
            for (at, _) in source.match_indices(marker) {
                let rest = &source[at + marker.len()..];
                let Some(end) = rest.find('"') else { continue };
                let kind = &rest[..end];
                if !kind.is_empty()
                    && kind.chars().all(|c| c.is_ascii_lowercase())
                    && !written.iter().any(|k| k == kind)
                {
                    written.push(kind.to_owned());
                }
            }
        }
    }
    written.sort();
    assert!(!written.is_empty(), "no `data-nr-ui` values found; the marker this test looks for has changed");
    let listed: Vec<String> = next_rust_ui::UI_WASM_KINDS.iter().map(|k| (*k).to_owned()).collect();
    let missing: Vec<&String> = written.iter().filter(|k| !listed.contains(k)).collect();
    let stale: Vec<&String> = listed.iter().filter(|k| !written.contains(k)).collect();
    assert!(missing.is_empty(), "the markup writes these and UI_WASM_KINDS does not list them: {missing:?}");
    assert!(stale.is_empty(), "UI_WASM_KINDS lists these and nothing renders them any more: {stale:?}");
}
