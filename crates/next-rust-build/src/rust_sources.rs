//! Scripts and stylesheets declared in Rust.
//!
//! `client_script!("chart.js", r#"…"#)`, `asset_script!("js/site.js", r#"…"#)`
//! and `stylesheet!(r#"…"#)` keep browser code and CSS inside `.rs` files. The
//! macros themselves expand to nothing: the build script reads the sources
//! out of the Rust files and writes them under `$OUT_DIR/next_rust_src`
//! (`client/`, `assets/`, `styles/`), and from there they go through the same
//! steps as files on disk: class scanning, short class names, minification,
//! hashing and embedding. A file in `client/` or `assets/` with the same path
//! wins over a declared one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use next_rust_core::{Config, Diagnostic};

use crate::codegen::embeddable_files;

/// Directory under `$OUT_DIR` holding the declared sources as files.
pub const DIR: &str = "next_rust_src";

/// A script declared with `client_script!` or `asset_script!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredScript {
    /// `client` or `assets`.
    pub field: &'static str,
    /// Path below that directory (`islands/chart.js`).
    pub rel: String,
    pub source: String,
    /// The Rust file declaring it.
    pub file: PathBuf,
}

/// Everything declared in the project's Rust files.
#[derive(Debug, Default, Clone)]
pub struct Declared {
    pub scripts: Vec<DeclaredScript>,
    /// CSS of the `stylesheet!` invocations of a file, in order, by file.
    pub stylesheets: BTreeMap<PathBuf, String>,
}

/// The macro invocations of one Rust source text: `(macro name, string arguments)`.
fn invocations(source: &str) -> Vec<(String, Vec<String>)> {
    if !["client_script!", "asset_script!", "stylesheet!"].iter().any(|m| source.contains(m)) {
        return Vec::new();
    }
    let Ok(file) = syn::parse_file(source) else { return Vec::new() };
    let mut out = Vec::new();
    fn walk(items: &[syn::Item], out: &mut Vec<(String, Vec<String>)>) {
        struct Strings(Vec<String>);
        impl syn::parse::Parse for Strings {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                let list = syn::punctuated::Punctuated::<syn::LitStr, syn::Token![,]>::parse_terminated(input)?;
                Ok(Strings(list.into_iter().map(|l| l.value()).collect()))
            }
        }
        for item in items {
            match item {
                syn::Item::Macro(m) => {
                    let Some(name) = m.mac.path.segments.last().map(|s| s.ident.to_string()) else { continue };
                    if matches!(name.as_str(), "client_script" | "asset_script" | "stylesheet")
                        && let Ok(Strings(args)) = syn::parse2::<Strings>(m.mac.tokens.clone())
                    {
                        out.push((name, args));
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, items)) = &m.content {
                        walk(items, out);
                    }
                }
                _ => {}
            }
        }
    }
    walk(&file.items, &mut out);
    out
}

/// The scripts and stylesheets declared in one Rust source text. They are
/// read as scripts and stylesheets from their written-out copies, so code
/// looking at Rust string literals can leave these out.
pub fn declared_literals(source: &str) -> Vec<String> {
    invocations(source).into_iter().filter_map(|(_, mut args)| args.pop()).collect()
}

/// The CSS a `[tailwind] stylesheets` entry stands for: the file itself, or
/// for a Rust file the contents of its `stylesheet!` invocations.
pub fn stylesheet_source(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    if path.extension().is_none_or(|e| e != "rs") {
        return Some(text);
    }
    let css: Vec<String> = invocations(&text)
        .into_iter()
        .filter(|(name, args)| name == "stylesheet" && args.len() == 1)
        .map(|(_, mut args)| args.remove(0))
        .collect();
    if css.is_empty() { None } else { Some(css.join("\n")) }
}

/// Whether a declared script name is a plain relative path. The name becomes
/// a file below `$OUT_DIR`, part of a URL and a JavaScript string in every
/// page that carries the script, so anything that could leave one of those
/// places is refused with a diagnostic rather than escaped three ways.
fn valid_name(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.starts_with('/')
        && rel.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
        && !rel.chars().any(|c| {
            c.is_control()
                || c.is_whitespace()
                || matches!(c, '"' | '\'' | '\\' | '<' | '>' | '`' | '?' | '#' | '*' | '|' | ':')
        })
}

/// Read every declaration in the app directory and `src/`.
pub fn collect(config: &Config) -> (Declared, Vec<Diagnostic>) {
    let mut declared = Declared::default();
    let mut diagnostics = Vec::new();
    let mut seen: BTreeMap<(String, String), PathBuf> = BTreeMap::new();
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in [config.app_dir(), config.root.join("src")] {
        files.extend(
            embeddable_files(&dir).into_iter().map(|(_, p)| p).filter(|p| p.extension().is_some_and(|e| e == "rs")),
        );
    }
    // Stylesheets may live anywhere the configuration points.
    files.extend(
        crate::tailwind::configured_stylesheets(config)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "rs")),
    );
    files.sort();
    files.dedup();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        for (name, mut args) in invocations(&text) {
            match (name.as_str(), args.len()) {
                ("stylesheet", 1) => {
                    let css = declared.stylesheets.entry(file.clone()).or_default();
                    if !css.is_empty() {
                        css.push('\n');
                    }
                    css.push_str(&args.remove(0));
                }
                ("client_script" | "asset_script", 2) => {
                    let field = if name == "client_script" { "client" } else { "assets" };
                    let source = args.remove(1);
                    let rel = args.remove(0);
                    if !valid_name(&rel) {
                        diagnostics.push(
                            Diagnostic::error("NR0214", format!("`{rel}` is not a usable script name"))
                                .location(&file)
                                .message(format!(
                                    "`{name}!(\"{rel}\", …)` names a file below `{field}/`, a URL and a key in the page's module registry."
                                ))
                                .help("use a relative path of letters, digits, `.`, `-`, `_` and `/`, without `..`"),
                        );
                        continue;
                    }
                    if let Some(first) = seen.insert((field.to_owned(), rel.clone()), file.clone()) {
                        diagnostics.push(
                            Diagnostic::error("NR0213", format!("the script `{field}/{rel}` is declared twice"))
                                .location(&file)
                                .message(format!("`{name}!(\"{rel}\", …)` is also in {}.", first.display()))
                                .help("give one of them another name"),
                        );
                        continue;
                    }
                    declared.scripts.push(DeclaredScript { field, rel, source, file: file.clone() });
                }
                _ => {}
            }
        }
    }
    if config.tailwind.enabled {
        for sheet in crate::tailwind::configured_stylesheets(config) {
            if sheet.extension().is_some_and(|e| e == "rs")
                && sheet.is_file()
                && !declared.stylesheets.contains_key(&sheet)
            {
                diagnostics.push(
                    Diagnostic::error("NR0009", "the Rust stylesheet declares no CSS")
                        .location(&sheet)
                        .message("`tailwind.stylesheets` lists this Rust file, but it has no `stylesheet!(r#\"…\"#);` at module level.")
                        .help("put the CSS in `next_rust::stylesheet!(r#\"…\"#);`, or list a .css file instead"),
                );
            }
        }
    }
    declared.scripts.sort_by(|a, b| (a.field, &a.rel).cmp(&(b.field, &b.rel)));
    (declared, diagnostics)
}

/// File name of a Rust stylesheet's CSS under `DIR/styles`: unique per file.
pub fn stylesheet_file(sheet: &Path) -> String {
    let stem = sheet.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let hash = next_rust_assets::content_hash(sheet.to_string_lossy().as_bytes());
    format!("{stem}.{}.css", &hash[..8])
}

/// Write the declarations under `dir` and remove what an earlier build left
/// there. Files whose contents did not change are not touched.
pub fn write(declared: &Declared, dir: &Path) -> std::io::Result<()> {
    let mut keep: Vec<PathBuf> = Vec::new();
    for script in &declared.scripts {
        let target = dir.join(script.field).join(&script.rel);
        crate::write_if_changed(&target, script.source.as_bytes())?;
        keep.push(target);
    }
    for (sheet, css) in &declared.stylesheets {
        let target = dir.join("styles").join(stylesheet_file(sheet));
        crate::write_if_changed(&target, css.as_bytes())?;
        keep.push(target);
    }
    // `client/__nrw` is not ours: `wasm_islands` writes the module, its
    // loader and the per-island glue there, and it runs *after* this. Sweeping
    // them away here would delete them on every build and have `wasm_islands`
    // write them straight back, which defeats its `write_if_changed` and
    // churns the files' timestamps for nothing.
    let wasm = dir.join("client").join(crate::wasm_islands::DIR);
    for (_, path) in embeddable_files(dir) {
        let generated_wasm =
            path.starts_with(&wasm) || path == dir.join("client").join(crate::wasm_islands::LOADER_FILE);
        if !generated_wasm && !keep.contains(&path) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

/// The script directories of a project as `(field, directory)`: `client/` and
/// `assets/`, then the declared scripts. Earlier entries win.
pub fn script_dirs(config: &Config) -> Vec<(&'static str, PathBuf)> {
    let mut dirs = vec![("client", config.root.join("client")), ("assets", config.root.join("assets"))];
    if let Some(generated) = &config.generated {
        dirs.push(("client", generated.join("client")));
        dirs.push(("assets", generated.join("assets")));
    }
    dirs
}

/// Every file of `field` (`client` or `assets`) as `(relative path, absolute
/// path)`, sorted: the files on disk and the scripts declared in Rust.
pub fn files(config: &Config, field: &str) -> Vec<(String, PathBuf)> {
    let mut out: BTreeMap<String, PathBuf> = BTreeMap::new();
    for (f, dir) in script_dirs(config) {
        if f == field {
            for (rel, path) in embeddable_files(&dir) {
                out.entry(rel).or_insert(path);
            }
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> (PathBuf, Config) {
        let root = std::env::temp_dir().join(format!("nr-rust-sources-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("src/client")).unwrap();
        let mut config =
            Config::from_toml_str("[tailwind]\nenabled = true\nstylesheets = [\"src/theme.rs\"]\n").unwrap();
        config.root = root.clone();
        (root, config)
    }

    #[test]
    fn declarations_become_files() {
        let (root, mut config) = project("files");
        std::fs::write(
            root.join("src/client/chart.rs"),
            "next_rust::client_script!(\"chart.js\", r#\"import { x } from \"./scale.js\";\nexport function hydrate() {}\"#);\nmod inner { asset_script!(\"js/site.js\", \"console.log(1)\"); }\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/theme.rs"),
            "stylesheet!(r#\".hero { color: red }\"#);\nstylesheet!(\".b{}\");\n",
        )
        .unwrap();
        std::fs::write(root.join("app/page.rs"), "pub fn Page() {}").unwrap();
        let (declared, diagnostics) = collect(&config);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let names: Vec<_> = declared.scripts.iter().map(|s| format!("{}/{}", s.field, s.rel)).collect();
        assert_eq!(names, ["assets/js/site.js", "client/chart.js"]);
        assert!(declared.scripts[1].source.starts_with("import { x }"));
        assert_eq!(declared.stylesheets[&root.join("src/theme.rs")], ".hero { color: red }\n.b{}");
        assert_eq!(stylesheet_source(&root.join("src/theme.rs")).as_deref(), Some(".hero { color: red }\n.b{}"));

        let out = root.join("out");
        write(&declared, &out).unwrap();
        std::fs::write(out.join("client/stale.js"), "old").unwrap();
        write(&declared, &out).unwrap();
        assert!(!out.join("client/stale.js").exists());
        assert_eq!(std::fs::read_to_string(out.join("assets/js/site.js")).unwrap(), "console.log(1)");

        // A file on disk wins over a declaration of the same name.
        std::fs::create_dir_all(root.join("client")).unwrap();
        std::fs::write(root.join("client/chart.js"), "// disk").unwrap();
        config.generated = Some(out.clone());
        let client = files(&config, "client");
        assert_eq!(client, [("chart.js".to_owned(), root.join("client/chart.js"))]);
        assert_eq!(files(&config, "assets"), [("js/site.js".to_owned(), out.join("assets/js/site.js"))]);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The name of a declared script is a path, a URL and a JavaScript
    /// string: one that would leave any of the three is refused.
    #[test]
    fn script_names_must_be_plain_relative_paths() {
        let (root, config) = project("names");
        std::fs::write(
            root.join("src/a.rs"),
            "client_script!(\"../../escape.js\", \"1\");\nasset_script!(\"js/x\\\"];alert(1).js\", \"2\");\nclient_script!(\"/abs.js\", \"3\");\nclient_script!(\"ok/fine.js\", \"4\");\n",
        )
        .unwrap();
        std::fs::write(root.join("src/theme.rs"), "stylesheet!(\".a{}\");").unwrap();
        let (declared, diagnostics) = collect(&config);
        assert_eq!(declared.scripts.iter().map(|s| s.rel.as_str()).collect::<Vec<_>>(), ["ok/fine.js"]);
        assert_eq!(diagnostics.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["NR0214", "NR0214", "NR0214"]);
        // Nothing is written outside the output directory.
        let out = root.join("out");
        write(&declared, &out).unwrap();
        assert!(out.join("client/ok/fine.js").is_file());
        assert!(!root.join("escape.js").exists() && !root.parent().unwrap().join("escape.js").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicates_and_empty_stylesheets_are_reported() {
        let (root, config) = project("errors");
        std::fs::write(root.join("src/a.rs"), "client_script!(\"x.js\", \"1\");").unwrap();
        std::fs::write(root.join("src/b.rs"), "client_script!(\"x.js\", \"2\");").unwrap();
        std::fs::write(root.join("src/theme.rs"), "pub const CSS: &str = \"\";").unwrap();
        let (declared, diagnostics) = collect(&config);
        assert_eq!(declared.scripts.len(), 1);
        let codes: Vec<_> = diagnostics.iter().map(|d| d.code.clone()).collect();
        assert_eq!(codes, ["NR0213", "NR0009"]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
