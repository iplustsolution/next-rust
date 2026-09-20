//! Scripts prepared for inline delivery (`[build] inline_assets`).
//!
//! An inline `<script>` has no URL, so an ES module written into the page
//! cannot be imported by another one. The build therefore turns every module
//! of `client/` and `assets/` into a *factory*: an async function that takes
//! an `import` function and returns the module's exports.
//!
//! ```js
//! import { scale } from "./scale.js";
//! export function hydrate(el) { … }
//! ```
//!
//! becomes
//!
//! ```js
//! async(__nr_import)=>{"use strict";const{scale}=await __nr_import("client/scale.js");
//! function hydrate(el) { … }
//! return{get "hydrate"(){return hydrate}}}
//! ```
//!
//! The renderer registers the factories a page needs in `window.__nrm`, keyed
//! by script (`client/chart.js`), and the runtime asks that registry before it
//! imports an island's module from the network. Exports are getters, so they
//! stay live bindings; imports are read once, when the importing module
//! starts (a module that reassigns an export later is not seen by modules
//! that imported it earlier, and import cycles are not supported). A factory
//! is registered by a classic script, so `import.meta` is not available.
//!
//! A module that is only reached through `import("./x.js")` is *lazy*: it is
//! left out of the page and served as a normal ES module at
//! `/_next-rust/m/<hash>`, where the browser fetches it when the code asks
//! for it. Large vendor bundles belong there.

use std::collections::{BTreeMap, BTreeSet};

/// How a prepared script is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A module factory, registered in the page under its key.
    Factory,
    /// A script without imports or exports, written into the page as it is.
    Plain,
    /// An ES module served at `/_next-rust/m/<id>`.
    Lazy,
}

/// One script, ready for the generated code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// `client/chart.js`, `assets/js/site.js`.
    pub key: String,
    /// Content hash of `code`: the `data-nr-js` id, or the URL of a lazy module.
    pub id: String,
    pub kind: Kind,
    pub code: String,
    /// Keys of the modules a factory imports statically.
    pub deps: Vec<String>,
}

/// The key `specifier` names when imported from the script at `from`, when
/// it is a path to another script of the app.
pub fn resolve(from: &str, specifier: &str) -> Option<String> {
    if let Some(rest) = specifier.strip_prefix("/_next-rust/") {
        let rest = rest.split(['?', '#']).next().unwrap_or(rest);
        return (rest.starts_with("client/") || rest.starts_with("assets/")).then(|| rest.to_owned());
    }
    if !(specifier.starts_with("./") || specifier.starts_with("../")) {
        return None;
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for part in specifier.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    // Never above `client/` or `assets/`.
    (parts.len() >= 2).then(|| parts.join("/"))
}

/// What one script imports and exports, with the edits that make it a factory.
#[derive(Debug, Default)]
struct Parsed {
    /// The source has `import` or `export` declarations.
    is_module: bool,
    /// Keys imported statically.
    static_deps: Vec<String>,
    /// Keys imported with `import("…")`.
    dynamic_deps: Vec<String>,
    /// `(start, end, specifier key)` of the string literal of every import
    /// that names another script: static (`from "./x.js"`) and dynamic.
    sources: Vec<(usize, usize, String, bool)>,
    /// Edits turning the module into a factory body: `(start, end, text)`.
    edits: Vec<(usize, usize, String)>,
    /// Statements hoisted to the top of the factory.
    hoisted: String,
    /// `(exported name, local expression)`.
    exports: Vec<(String, String)>,
    /// Namespaces re-exported whole (`export * from`).
    star_exports: Vec<String>,
}

#[cfg(feature = "minify-js")]
fn parse(key: &str, source: &str) -> Result<Parsed, String> {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::{
        Declaration, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier, ImportExpression, Statement,
    };
    use oxc_ast_visit::Visit;
    use oxc_parser::Parser;
    use oxc_span::{GetSpan, SourceType};

    struct Dynamic<'k> {
        key: &'k str,
        found: Vec<(usize, usize, String)>,
    }
    impl<'a> Visit<'a> for Dynamic<'_> {
        fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
            // Minifiers write string literals as templates: `import(\`./x.js\`)`.
            let literal = match &it.source {
                Expression::StringLiteral(s) => Some((s.span, s.value.to_string())),
                Expression::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => {
                    t.quasis[0].value.cooked.as_ref().map(|c| (t.span, c.to_string()))
                }
                _ => None,
            };
            if let Some((span, value)) = literal
                && let Some(target) = resolve(self.key, &value)
            {
                self.found.push((span.start as usize, span.end as usize, target));
            }
            oxc_ast_visit::walk::walk_import_expression(self, it);
        }
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let allocator = Allocator::default();
        let parsed = [SourceType::mjs(), SourceType::cjs().with_module(false)]
            .into_iter()
            .map(|t| Parser::new(&allocator, source, t).parse())
            .find(|p| p.errors.is_empty())
            .ok_or_else(|| "the script does not parse".to_owned())?;
        let mut out = Parsed::default();
        let mut dynamic = Dynamic { key, found: Vec::new() };
        dynamic.visit_program(&parsed.program);
        for (start, end, target) in dynamic.found {
            out.dynamic_deps.push(target.clone());
            out.sources.push((start, end, target, true));
        }
        let text = |span: oxc_span::Span| &source[span.start as usize..span.end as usize];
        let quoted = |s: &str| serde_json::to_string(s).unwrap_or_default();
        let mut temp = 0usize;
        // `await __nr_import("key")` for a script of the app, the browser's
        // own `import()` for anything else (a URL).
        let request = |out: &mut Parsed, literal: &oxc_ast::ast::StringLiteral<'_>| match resolve(key, &literal.value) {
            Some(target) => {
                out.static_deps.push(target.clone());
                out.sources.push((literal.span.start as usize, literal.span.end as usize, target.clone(), false));
                format!("await __nr_import({})", quoted(&target))
            }
            None => format!("await import({})", quoted(&literal.value)),
        };
        for statement in &parsed.program.body {
            match statement {
                Statement::ImportDeclaration(decl) => {
                    out.is_module = true;
                    let from = request(&mut out, &decl.source);
                    let mut named = Vec::new();
                    let mut namespace = None;
                    for spec in decl.specifiers.iter().flatten() {
                        match spec {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                named.push(format!("{}:{}", quoted(&s.imported.name()), s.local.name));
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                named.push(format!("\"default\":{}", s.local.name));
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                namespace = Some(s.local.name.to_string());
                            }
                        }
                    }
                    match (namespace, named.is_empty()) {
                        (None, true) => out.hoisted.push_str(&format!("{from};")),
                        (None, false) => out.hoisted.push_str(&format!("const{{{}}}={from};", named.join(","))),
                        (Some(ns), true) => out.hoisted.push_str(&format!("const {ns}={from};")),
                        (Some(ns), false) => {
                            out.hoisted.push_str(&format!("const {ns}={from};const{{{}}}={ns};", named.join(",")))
                        }
                    }
                    out.edits.push((decl.span.start as usize, decl.span.end as usize, String::new()));
                }
                Statement::ExportNamedDeclaration(decl) => {
                    out.is_module = true;
                    if let Some(source) = &decl.source {
                        // `export { a as b } from "./x.js"`
                        let from = request(&mut out, source);
                        let ns = format!("__nr_m{temp}");
                        temp += 1;
                        out.hoisted.push_str(&format!("const {ns}={from};"));
                        for spec in &decl.specifiers {
                            out.exports.push((
                                spec.exported.name().to_string(),
                                format!("{ns}[{}]", quoted(&spec.local.name())),
                            ));
                        }
                        out.edits.push((decl.span.start as usize, decl.span.end as usize, String::new()));
                    } else if let Some(declaration) = &decl.declaration {
                        let names: Vec<String> = match declaration {
                            Declaration::VariableDeclaration(v) => v
                                .declarations
                                .iter()
                                .flat_map(|d| d.id.get_binding_identifiers())
                                .map(|id| id.name.to_string())
                                .collect(),
                            Declaration::FunctionDeclaration(f) => f.id.iter().map(|id| id.name.to_string()).collect(),
                            Declaration::ClassDeclaration(c) => c.id.iter().map(|id| id.name.to_string()).collect(),
                            _ => Vec::new(),
                        };
                        out.exports.extend(names.into_iter().map(|n| (n.clone(), n)));
                        // Drop the `export` keyword, keep the declaration.
                        out.edits.push((decl.span.start as usize, declaration.span().start as usize, String::new()));
                    } else {
                        // `export { a, b as c };`
                        for spec in &decl.specifiers {
                            out.exports.push((spec.exported.name().to_string(), spec.local.name().to_string()));
                        }
                        out.edits.push((decl.span.start as usize, decl.span.end as usize, String::new()));
                    }
                }
                Statement::ExportDefaultDeclaration(decl) => {
                    out.is_module = true;
                    let named = match &decl.declaration {
                        ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                            f.id.as_ref().map(|id| id.name.to_string())
                        }
                        ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                            c.id.as_ref().map(|id| id.name.to_string())
                        }
                        _ => None,
                    };
                    let inner = decl.declaration.span();
                    match named {
                        Some(name) => {
                            out.exports.push(("default".into(), name));
                            out.edits.push((decl.span.start as usize, inner.start as usize, String::new()));
                        }
                        None => {
                            out.exports.push(("default".into(), "__nr_default".into()));
                            out.edits.push((
                                decl.span.start as usize,
                                decl.span.end as usize,
                                format!("const __nr_default=({});", text(inner)),
                            ));
                        }
                    }
                }
                Statement::ExportAllDeclaration(decl) => {
                    out.is_module = true;
                    let from = request(&mut out, &decl.source);
                    let ns = format!("__nr_m{temp}");
                    temp += 1;
                    out.hoisted.push_str(&format!("const {ns}={from};"));
                    match &decl.exported {
                        Some(name) => out.exports.push((name.name().to_string(), ns)),
                        None => out.star_exports.push(ns),
                    }
                    out.edits.push((decl.span.start as usize, decl.span.end as usize, String::new()));
                }
                _ => {}
            }
        }
        Ok(out)
    }));
    result.unwrap_or_else(|_| Err("the script analysis failed unexpectedly".into()))
}

#[cfg(not(feature = "minify-js"))]
fn parse(_key: &str, _source: &str) -> Result<Parsed, String> {
    Err("next-rust-build was compiled without the `minify-js` feature".into())
}

fn apply(source: &str, mut edits: Vec<(usize, usize, String)>) -> String {
    edits.sort_by_key(|e| (e.0, e.1));
    let mut out = String::with_capacity(source.len());
    let mut last = 0;
    for (start, end, replacement) in edits {
        if start < last {
            continue;
        }
        out.push_str(&source[last..start]);
        out.push_str(&replacement);
        last = end;
    }
    out.push_str(&source[last..]);
    out
}

/// Whether a script key can be written into a page. The key ends up in the
/// module registry as a JavaScript string inside a `<script>` element; a name
/// holding a quote, a backslash, an angle bracket or a control character is
/// left to the browser's own module loader instead of being escaped into
/// something the page cannot be read back from.
fn safe_key(key: &str) -> bool {
    !key.chars().any(|c| c.is_control() || matches!(c, '"' | '\'' | '\\' | '<' | '>' | '`'))
}

/// The URL a lazy module is served at.
pub fn lazy_url(id: &str) -> String {
    format!("/_next-rust/m/{id}")
}

/// Prepare `sources` (script key → code as it would be served) for inline
/// delivery. Scripts that cannot be analyzed are left out, with a warning
/// each; the pages then load them from their URL as usual.
pub fn prepare(sources: &BTreeMap<String, String>) -> (Vec<Prepared>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut parsed: BTreeMap<&str, Parsed> = BTreeMap::new();
    for (key, source) in sources {
        if !matches!(key.rsplit('.').next(), Some("js" | "mjs")) {
            continue;
        }
        if !safe_key(key) {
            warnings.push(format!(
                "{key} is not written into pages; its name cannot be written into a script. Use a name without quotes, backslashes or angle brackets."
            ));
            continue;
        }
        match parse(key, source) {
            Ok(p) => {
                parsed.insert(key, p);
            }
            Err(e) => warnings.push(format!("{key} is not written into pages; it could not be analyzed: {e}")),
        }
    }
    let exists = |k: &str| parsed.contains_key(k);

    // Lazy modules: the targets of dynamic imports, and what those import.
    let mut lazy: BTreeSet<String> = BTreeSet::new();
    let mut todo: Vec<String> =
        parsed.values().flat_map(|p| p.dynamic_deps.iter().cloned()).filter(|k| exists(k)).collect();
    while let Some(key) = todo.pop() {
        if lazy.insert(key.clone()) {
            let p = &parsed[key.as_str()];
            todo.extend(p.static_deps.iter().chain(&p.dynamic_deps).filter(|k| exists(k)).cloned());
        }
    }
    // Their code, imports pointing at each other's URLs; dependencies first.
    let mut lazy_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut lazy_code: BTreeMap<String, String> = BTreeMap::new();
    fn build_lazy(
        key: &str,
        sources: &BTreeMap<String, String>,
        parsed: &BTreeMap<&str, Parsed>,
        ids: &mut BTreeMap<String, String>,
        code: &mut BTreeMap<String, String>,
        stack: &mut Vec<String>,
    ) {
        if ids.contains_key(key) || stack.iter().any(|k| k == key) {
            return;
        }
        stack.push(key.to_owned());
        let p = &parsed[key];
        for (_, _, target, _) in &p.sources {
            if parsed.contains_key(target.as_str()) {
                build_lazy(target, sources, parsed, ids, code, stack);
            }
        }
        stack.pop();
        let edits = p
            .sources
            .iter()
            .filter_map(|(start, end, target, _)| {
                // A cycle leaves the import as written, which still resolves
                // when both were served from `/_next-rust/client/`.
                ids.get(target).map(|id| (*start, *end, format!("\"{}\"", lazy_url(id))))
            })
            .collect();
        let text = apply(&sources[key], edits);
        let id = next_rust_assets::content_hash(text.as_bytes());
        ids.insert(key.to_owned(), id);
        code.insert(key.to_owned(), text);
    }
    for key in &lazy {
        build_lazy(key, sources, &parsed, &mut lazy_ids, &mut lazy_code, &mut Vec::new());
    }

    // Modules something imports statically need a factory even when they are
    // also imported lazily.
    let statically_imported: BTreeSet<&String> = parsed.values().flat_map(|p| p.static_deps.iter()).collect();
    let mut out = Vec::new();
    for (key, p) in &parsed {
        let source = &sources[*key];
        let key = (*key).to_owned();
        if let Some(id) = lazy_ids.get(&key) {
            out.push(Prepared {
                key: key.clone(),
                id: id.clone(),
                kind: Kind::Lazy,
                code: lazy_code[&key].clone(),
                deps: Vec::new(),
            });
            if !statically_imported.contains(&key) {
                continue;
            }
        }
        // Dynamic imports of the app's scripts go to the lazy URLs.
        let dynamic: Vec<(usize, usize, String)> = p
            .sources
            .iter()
            .filter(|(_, _, _, dynamic)| *dynamic)
            .filter_map(|(start, end, target, _)| {
                lazy_ids.get(target).map(|id| (*start, *end, format!("\"{}\"", lazy_url(id))))
            })
            .collect();
        let client = key.starts_with("client/");
        let (kind, code, deps) = if p.is_module || client {
            let mut edits = p.edits.clone();
            edits.extend(dynamic);
            let mut exports: Vec<String> = p
                .exports
                .iter()
                .map(|(name, local)| {
                    format!("get {}(){{return {local}}}", serde_json::to_string(name).unwrap_or_default())
                })
                .collect();
            exports.extend(p.star_exports.iter().map(|ns| format!("...{ns}")));
            // The line break ends a trailing `//` comment of the source.
            let code = format!(
                "async(__nr_import)=>{{\"use strict\";{}{}\nreturn{{{}}}}}",
                p.hoisted,
                apply(source, edits),
                exports.join(",")
            );
            let mut deps: Vec<String> = p.static_deps.iter().filter(|k| exists(k)).cloned().collect();
            deps.sort();
            deps.dedup();
            (Kind::Factory, code, deps)
        } else {
            (Kind::Plain, apply(source, dynamic), Vec::new())
        };
        let code = next_rust_assets::html::escape_inline_script(&code);
        let id = next_rust_assets::content_hash(code.as_bytes())[..12].to_owned();
        out.push(Prepared { key, id, kind, code, deps });
    }
    out.sort_by(|a, b| (&a.key, a.kind as u8).cmp(&(&b.key, b.kind as u8)));
    (out, warnings)
}

#[cfg(all(test, feature = "minify-js"))]
mod tests {
    use super::*;

    fn sources(files: &[(&str, &str)]) -> BTreeMap<String, String> {
        files.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    fn find<'a>(all: &'a [Prepared], key: &str, kind: Kind) -> &'a Prepared {
        all.iter().find(|p| p.key == key && p.kind == kind).unwrap_or_else(|| panic!("{key} {kind:?} in {all:#?}"))
    }

    #[test]
    fn modules_become_factories() {
        let (all, warnings) = prepare(&sources(&[
            (
                "client/practice.js",
                "import { el, icon as i } from \"./answers.js\";\nimport * as voice from './lib/voice.js';\nimport d from \"./answers.js\";\nexport function hydrate(root) { el(root); }\nexport const a = 1, { b, c: [e] } = i;\nexport default class Thing {}\n// end",
            ),
            (
                "client/answers.js",
                "const el = () => 1;\nlet icon = 2;\nexport { el, icon as \"icon\" };\nexport default function () {}\n",
            ),
            (
                "client/lib/voice.js",
                "export * from \"../answers.js\";\nexport * as all from \"../answers.js\";\nexport { el as element } from \"../answers.js\";",
            ),
        ]));
        assert!(warnings.is_empty(), "{warnings:?}");
        let practice = find(&all, "client/practice.js", Kind::Factory);
        assert_eq!(practice.deps, ["client/answers.js", "client/lib/voice.js"]);
        assert!(practice.code.starts_with("async(__nr_import)=>{\"use strict\";const{\"el\":el,\"icon\":i}=await __nr_import(\"client/answers.js\");const voice=await __nr_import(\"client/lib/voice.js\");const{\"default\":d}=await __nr_import(\"client/answers.js\");"), "{}", practice.code);
        assert!(!practice.code.contains("export") && !practice.code.contains("import {"), "{}", practice.code);
        assert!(practice.code.contains("\nfunction hydrate(root) { el(root); }\nconst a = 1,"), "{}", practice.code);
        assert!(practice.code.ends_with("// end\nreturn{get \"hydrate\"(){return hydrate},get \"a\"(){return a},get \"b\"(){return b},get \"e\"(){return e},get \"default\"(){return Thing}}}"), "{}", practice.code);
        let answers = find(&all, "client/answers.js", Kind::Factory);
        assert!(answers.code.contains("const __nr_default=(function () {});"), "{}", answers.code);
        assert!(answers.code.ends_with(
            "return{get \"el\"(){return el},get \"icon\"(){return icon},get \"default\"(){return __nr_default}}}"
        ));
        let voice = find(&all, "client/lib/voice.js", Kind::Factory);
        assert!(
            voice.code.contains("get \"all\"(){return __nr_m1}")
                && voice.code.contains("get \"element\"(){return __nr_m2[\"el\"]}")
                && voice.code.ends_with(",...__nr_m0}}"),
            "{}",
            voice.code
        );
        assert_eq!(practice.id.len(), 12);
    }

    #[test]
    fn dynamic_imports_are_lazy_and_plain_scripts_stay_plain() {
        let (all, _) = prepare(&sources(&[
            (
                "client/social.js",
                "export async function hydrate() { const fb = await import(\"./vendor/fb.js\"); await import(name); }",
            ),
            (
                "client/minified.js",
                "export const load = () => import(`./vendor/fb.js`), other = () => import(`./${name}.js`);",
            ),
            ("client/vendor/fb.js", "import { x } from \"./dep.js\"; export const auth = x;"),
            ("client/vendor/dep.js", "export const x = 1;"),
            (
                "assets/js/site.js",
                "document.title = \"</script><!-- x\"; import(\"/_next-rust/client/vendor/dep.js\");",
            ),
            ("assets/logo.svg", "<svg/>"),
        ]));
        let dep = find(&all, "client/vendor/dep.js", Kind::Lazy);
        let fb = find(&all, "client/vendor/fb.js", Kind::Lazy);
        assert_eq!(dep.code, "export const x = 1;");
        assert_eq!(fb.code, format!("import {{ x }} from \"/_next-rust/m/{}\"; export const auth = x;", dep.id));
        assert_eq!(dep.id.len(), 16);
        // Only reached lazily: no factory, so it never lands in a page.
        assert!(!all.iter().any(|p| p.key == "client/vendor/fb.js" && p.kind == Kind::Factory));
        let social = find(&all, "client/social.js", Kind::Factory);
        assert!(social.code.contains(&format!("await import(\"/_next-rust/m/{}\")", fb.id)), "{}", social.code);
        assert!(social.code.contains("await import(name)"));
        let minified = find(&all, "client/minified.js", Kind::Factory);
        assert!(minified.code.contains(&format!("import(\"/_next-rust/m/{}\")", fb.id)), "{}", minified.code);
        assert!(minified.code.contains("import(`./${name}.js`)"), "{}", minified.code);
        let site = find(&all, "assets/js/site.js", Kind::Plain);
        assert_eq!(
            site.code,
            format!("document.title = \"<\\/script><\\!-- x\"; import(\"/_next-rust/m/{}\");", dep.id)
        );
        assert!(!all.iter().any(|p| p.key == "assets/logo.svg"));
    }

    #[test]
    fn specifiers_resolve_inside_the_app_only() {
        assert_eq!(resolve("client/a/b.js", "./c.js").as_deref(), Some("client/a/c.js"));
        assert_eq!(resolve("client/a/b.js", "../c.js").as_deref(), Some("client/c.js"));
        assert_eq!(resolve("client/b.js", "../../etc/passwd"), None);
        assert_eq!(resolve("client/b.js", "/_next-rust/client/x.js?v=1").as_deref(), Some("client/x.js"));
        assert_eq!(resolve("client/b.js", "https://cdn.example/x.js"), None);
        assert_eq!(resolve("client/b.js", "/_next-rust/runtime.js"), None);
    }

    /// A file name is not code: one that could not be written into a page as
    /// a JavaScript string is left out, so nothing has to escape it later.
    #[test]
    fn names_that_cannot_be_written_into_a_page_are_left_out() {
        let (all, warnings) = prepare(&sources(&[
            ("client/x\"];alert(1);r.f[\"y.js", "export const a = 1;"),
            ("client/z</script><img src=x onerror=alert(2)>.js", "export const b = 1;"),
            ("client/back\\slash.js", "export const c = 1;"),
            ("client/ok.js", "export const d = 1;"),
        ]));
        assert_eq!(all.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(), ["client/ok.js"]);
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.iter().all(|w| w.contains("cannot be written into a script")), "{warnings:?}");
    }

    #[test]
    fn broken_scripts_are_warnings() {
        let (all, warnings) = prepare(&sources(&[("client/bad.js", "function (")]));
        assert!(all.is_empty());
        assert_eq!(warnings.len(), 1);
    }
}
