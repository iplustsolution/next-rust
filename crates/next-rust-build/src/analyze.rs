//! Source analysis of special files with `syn`.
//!
//! Only *signatures* are inspected: exported functions (name, `async`,
//! argument types, whether they return `Result`), segment configuration
//! constants and `#[server_action]` functions. Function bodies are never
//! interpreted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use next_rust_core::Diagnostic;
use syn::{FnArg, Item, ReturnType, Type, Visibility};

/// How an argument is supplied by generated code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgKind {
    Children,
    Slots,
    /// `Data<T>`: the result of the file's `load` function.
    Data,
    /// `ErrorInfo` for error boundaries.
    ErrorInfo,
    Request,
    Next,
    /// `ActionContext` for server actions.
    ActionContext,
    /// Resolved through `FromContext`.
    Extractor {
        name: String,
        static_safe: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnInfo {
    pub name: String,
    pub is_async: bool,
    pub is_pub: bool,
    pub args: Vec<ArgKind>,
    pub line: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileInfo {
    pub path: PathBuf,
    pub fns: BTreeMap<String, FnInfo>,
    /// `pub const NAME: .. = <expr>` with the expression as text.
    pub consts: BTreeMap<String, String>,
    /// `#[server_action]` functions: (name, argument count).
    pub actions: Vec<(String, usize)>,
}

impl FileInfo {
    pub fn get(&self, name: &str) -> Option<&FnInfo> {
        self.fns.get(name).filter(|f| f.is_pub)
    }

    /// `RENDERING` constant resolved to `static`/`dynamic`/`auto`.
    pub fn rendering(&self) -> Option<&'static str> {
        let v = self.consts.get("RENDERING")?;
        let last = v.rsplit("::").next().unwrap_or(v).trim();
        match last {
            "Static" => Some("static"),
            "Dynamic" => Some("dynamic"),
            "Auto" => Some("auto"),
            _ => None,
        }
    }

    /// `REVALIDATE` as a literal number of seconds, if it is one.
    pub fn revalidate_literal(&self) -> Option<u64> {
        self.consts.get("REVALIDATE")?.replace('_', "").trim().parse().ok()
    }
}

/// Types that never read request data (safe for static rendering).
const STATIC_SAFE: &[&str] = &["Params", "Path", "Nonce"];

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
        Type::Reference(r) => type_name(&r.elem),
        Type::Paren(p) => type_name(&p.elem),
        Type::Group(g) => type_name(&g.elem),
        _ => String::new(),
    }
}

fn classify(ty: &Type) -> ArgKind {
    match type_name(ty).as_str() {
        "Children" => ArgKind::Children,
        "Slots" => ArgKind::Slots,
        "Data" => ArgKind::Data,
        "ErrorInfo" => ArgKind::ErrorInfo,
        "Request" => ArgKind::Request,
        "Next" => ArgKind::Next,
        "ActionContext" => ArgKind::ActionContext,
        name => ArgKind::Extractor { static_safe: STATIC_SAFE.contains(&name), name: name.to_owned() },
    }
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().segments.last().is_some_and(|s| s.ident == name))
}

/// Parse and analyze a Rust source file.
pub fn analyze_file(path: &Path) -> Result<FileInfo, Diagnostic> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| Diagnostic::error("NR0200", "Cannot read source file").location(path).message(e.to_string()))?;
    analyze_source(path, &source)
}

pub fn analyze_source(path: &Path, source: &str) -> Result<FileInfo, Diagnostic> {
    let file = syn::parse_file(source).map_err(|e| {
        let start = e.span().start();
        Diagnostic::error("NR0200", "Syntax error")
            .location(format!("{}:{}:{}", path.display(), start.line, start.column + 1))
            .message(e.to_string())
    })?;
    let mut info = FileInfo { path: path.to_path_buf(), ..Default::default() };
    for item in &file.items {
        match item {
            Item::Fn(f) => {
                let args = f
                    .sig
                    .inputs
                    .iter()
                    .map(|a| match a {
                        FnArg::Typed(t) => classify(&t.ty),
                        FnArg::Receiver(_) => ArgKind::Extractor { name: "self".into(), static_safe: false },
                    })
                    .collect::<Vec<_>>();
                if has_attr(&f.attrs, "server_action") {
                    info.actions.push((f.sig.ident.to_string(), args.len()));
                }
                let _ = matches!(f.sig.output, ReturnType::Default);
                info.fns.insert(
                    f.sig.ident.to_string(),
                    FnInfo {
                        name: f.sig.ident.to_string(),
                        is_async: f.sig.asyncness.is_some(),
                        is_pub: matches!(f.vis, Visibility::Public(_)),
                        args,
                        line: f.sig.ident.span().start().line,
                    },
                );
            }
            Item::Const(c) if matches!(c.vis, Visibility::Public(_)) => {
                let expr = &c.expr;
                info.consts.insert(c.ident.to_string(), quote::quote!(#expr).to_string().replace(' ', ""));
            }
            _ => {}
        }
    }
    Ok(info)
}

/// Find `#[server_action]` functions in `src/` and derive module paths
/// (`src/actions/user.rs` → `crate::actions::user`).
pub fn scan_src_actions(src_dir: &Path, skip: &[PathBuf]) -> (Vec<(String, String, usize)>, Vec<Diagnostic>) {
    let mut out = Vec::new();
    let mut diags = Vec::new();
    let mut stack = vec![src_dir.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") && !skip.contains(&p) {
                files.push(p);
            }
        }
    }
    files.sort();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        if !text.contains("server_action") {
            continue;
        }
        match analyze_source(&file, &text) {
            Ok(info) => {
                let Ok(rel) = file.strip_prefix(src_dir) else { continue };
                let mut parts: Vec<String> =
                    rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
                let last = parts.pop().unwrap_or_default();
                let stem = last.trim_end_matches(".rs");
                if !matches!(stem, "main" | "lib" | "mod") {
                    parts.push(stem.to_owned());
                }
                let module = std::iter::once("crate".to_owned()).chain(parts).collect::<Vec<_>>().join("::");
                for (name, argc) in info.actions {
                    out.push((module.clone(), name, argc));
                }
            }
            Err(d) => diags.push(d),
        }
    }
    (out, diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_signatures() {
        let src = r#"
            use next_rust::prelude::*;
            pub const RENDERING: Rendering = Rendering::Static;
            pub const REVALIDATE: u64 = 3_600;
            pub async fn Page(Path(p): Path<P>, cookies: Cookies, Data(d): Data<Vec<u8>>) -> Result<impl View> { todo!() }
            pub fn Layout(children: Children, mut slots: Slots) -> impl View { children }
            fn helper() {}
            #[server_action]
            pub async fn save(ctx: ActionContext, input: Input) -> Result<()> { Ok(()) }
        "#;
        let info = analyze_source(Path::new("page.rs"), src).unwrap();
        let page = info.get("Page").unwrap();
        assert!(page.is_async);
        assert_eq!(
            page.args,
            vec![
                ArgKind::Extractor { name: "Path".into(), static_safe: true },
                ArgKind::Extractor { name: "Cookies".into(), static_safe: false },
                ArgKind::Data
            ]
        );
        assert_eq!(info.get("Layout").unwrap().args, vec![ArgKind::Children, ArgKind::Slots]);
        assert!(info.get("helper").is_none(), "private fns are not exports");
        assert_eq!(info.rendering(), Some("static"));
        assert_eq!(info.revalidate_literal(), Some(3600));
        assert_eq!(info.actions, vec![("save".into(), 2)]);
    }

    #[test]
    fn syntax_errors_have_locations() {
        let err = analyze_source(Path::new("app/page.rs"), "pub fn Page( -> {").unwrap_err();
        assert_eq!(err.code, "NR0200");
        assert!(err.locations[0].to_string_lossy().starts_with("app/page.rs:1:"));
    }
}
