//! `next-rust editor`: VS Code snippets and recommended settings.

use std::path::Path;

use crate::{Args, project, ui};

/// Snippets offered while typing in `.rs` files (type the prefix, e.g. `nrpage`).
pub const SNIPPETS: &str = r##"{
  "Next Rust page": {
    "scope": "rust",
    "prefix": "nrpage",
    "description": "A page with metadata (page.rs)",
    "body": [
      "use next_rust::prelude::*;",
      "",
      "pub fn metadata() -> Metadata {",
      "    Metadata::new().title(\"${1:Title}\")",
      "}",
      "",
      "pub fn Page() -> impl View {",
      "    main![",
      "        h1![\"${1:Title}\"],",
      "        $0",
      "    ]",
      "}"
    ]
  },
  "Next Rust dynamic page": {
    "scope": "rust",
    "prefix": "nrparams",
    "description": "A page with typed route parameters (app/[id]/page.rs)",
    "body": [
      "use next_rust::prelude::*;",
      "",
      "#[derive(serde::Deserialize)]",
      "pub struct RouteParams {",
      "    ${1:id}: ${2:String},",
      "}",
      "",
      "pub fn Page(Path(p): Path<RouteParams>) -> impl View {",
      "    h1![format!(\"{}\", p.${1:id})]",
      "}"
    ]
  },
  "Next Rust layout": {
    "scope": "rust",
    "prefix": "nrlayout",
    "description": "A layout wrapping its children (layout.rs)",
    "body": [
      "use next_rust::prelude::*;",
      "",
      "pub fn Layout(children: Children) -> impl View {",
      "    ${1:section}![${0}children]",
      "}"
    ]
  },
  "Next Rust API route": {
    "scope": "rust",
    "prefix": "nrroute",
    "description": "GET and POST handlers (route.rs)",
    "body": [
      "use next_rust::prelude::*;",
      "",
      "pub async fn GET(req: Request) -> Response {",
      "    Response::json(&serde_json::json!({ \"path\": req.path() }))",
      "}",
      "",
      "pub async fn POST(mut req: Request) -> Result<Response> {",
      "    let body: serde_json::Value = req.json().await?;",
      "    Ok(Response::json(&body).with_status(201))",
      "}"
    ]
  },
  "Next Rust GET handler": {
    "scope": "rust",
    "prefix": "nrget",
    "description": "A GET handler",
    "body": ["pub async fn GET(req: Request) -> Response {", "    $0", "}"]
  },
  "Next Rust POST handler": {
    "scope": "rust",
    "prefix": "nrpost",
    "description": "A POST handler reading JSON",
    "body": [
      "pub async fn POST(mut req: Request) -> Result<Response> {",
      "    let input: ${1:serde_json::Value} = req.json().await?;",
      "    $0",
      "    Ok(Response::json(&input))",
      "}"
    ]
  },
  "Next Rust metadata": {
    "scope": "rust",
    "prefix": "nrmeta",
    "description": "Page metadata (title, description)",
    "body": [
      "pub fn metadata() -> Metadata {",
      "    Metadata::new().title(\"${1:Title}\").description(\"${2:Description}\")",
      "}"
    ]
  },
  "Next Rust data loader": {
    "scope": "rust",
    "prefix": "nrload",
    "description": "load() + Data<T> for a page",
    "body": [
      "pub async fn load() -> Result<${1:Vec<String>}> {",
      "    Ok(${2:vec![]})",
      "}",
      "",
      "pub fn Page(Data(${3:items}): Data<${1:Vec<String>}>) -> impl View {",
      "    ul![each(${3:items}, |item| li![item])]",
      "}"
    ]
  },
  "Next Rust server action": {
    "scope": "rust",
    "prefix": "nraction",
    "description": "A server action callable from forms",
    "body": [
      "#[derive(serde::Deserialize)]",
      "pub struct ${1:Input} {",
      "    ${2:name}: String,",
      "}",
      "",
      "#[server_action]",
      "pub async fn ${3:submit}(input: ${1:Input}) -> Result<()> {",
      "    $0",
      "    Ok(())",
      "}"
    ]
  },
  "Next Rust form": {
    "scope": "rust",
    "prefix": "nrform",
    "description": "A form posting to a server action",
    "body": [
      "form![",
      "    action!(${1:submit}),",
      "    input![name(\"${2:name}\")],",
      "    button![r#type(\"submit\"), \"${3:Send}\"],",
      "]"
    ]
  },
  "Next Rust loading": {
    "scope": "rust",
    "prefix": "nrloading",
    "description": "Loading UI (loading.rs)",
    "body": ["use next_rust::prelude::*;", "", "pub fn Loading() -> impl View {", "    p![\"${1:Loading…}\"]", "}"]
  },
  "Next Rust error boundary": {
    "scope": "rust",
    "prefix": "nrerror",
    "description": "Error UI (error.rs)",
    "body": [
      "use next_rust::prelude::*;",
      "",
      "pub fn ErrorBoundary(info: ErrorInfo) -> impl View {",
      "    div![role(\"alert\"), h2![\"Something went wrong\"], p![info.message]]",
      "}"
    ]
  },
  "Next Rust not found": {
    "scope": "rust",
    "prefix": "nrnotfound",
    "description": "404 UI (not-found.rs)",
    "body": ["use next_rust::prelude::*;", "", "pub fn NotFound() -> impl View {", "    h1![\"${1:Not found}\"]", "}"]
  },
  "Next Rust middleware": {
    "scope": "rust",
    "prefix": "nrmiddleware",
    "description": "Middleware (middleware.rs)",
    "body": [
      "use next_rust::prelude::*;",
      "",
      "pub async fn middleware(req: Request, next: Next) -> Response {",
      "    $0",
      "    next.run(req).await",
      "}"
    ]
  },
  "Next Rust component": {
    "scope": "rust",
    "prefix": "nrcomponent",
    "description": "A reusable view component",
    "body": ["pub fn ${1:Card}(${2:title}: &str) -> impl View {", "    ${3:div}![class(\"${4:card}\"), ${2:title}]", "}"]
  },
  "Next Rust link": {
    "scope": "rust",
    "prefix": "nrlink",
    "description": "Client-navigating link",
    "body": ["Link!(href = \"${1:/}\", \"${2:Home}\")"]
  },
  "Next Rust image": {
    "scope": "rust",
    "prefix": "nrimage",
    "description": "Responsive, lazy-loaded image",
    "body": ["Image!(src = \"${1:/image.jpg}\", width = ${2:1200}, height = ${3:800}, alt = \"${4:Description}\")"]
  },
  "Next Rust CSS module": {
    "scope": "rust",
    "prefix": "nrcss",
    "description": "Scoped CSS module",
    "body": ["let styles = css_module!(\"${1:styles.module.css}\");"]
  }
}
"##;

pub const EXTENSIONS: &str = r#"{
  "recommendations": ["rust-lang.rust-analyzer"]
}
"#;

/// Re-run build scripts on save, so routes added while editing are picked up
/// by code completion right away.
pub const SETTINGS: &str = r#"{
  "rust-analyzer.cargo.buildScripts.rebuildOnSave": true,
  "editor.snippetSuggestions": "top"
}
"#;

/// The editor files as (relative path, contents).
pub fn files() -> [(&'static str, &'static str); 3] {
    [
        (".vscode/next-rust.code-snippets", SNIPPETS),
        (".vscode/extensions.json", EXTENSIONS),
        (".vscode/settings.json", SETTINGS),
    ]
}

pub fn run(args: &[String]) -> Result<(), String> {
    let a = Args::new(args);
    if a.flag(&["-h", "--help"]) {
        println!(
            "next-rust editor [--force]\n\nAdd VS Code snippets (type `nrpage`, `nrroute`, `nrlayout`, ...), recommend the rust-analyzer\nextension and enable build-script rebuilds on save. Existing files are kept unless --force is given."
        );
        return Ok(());
    }
    let root = project::load_config()?.root;
    write_files(&root, a.flag(&["--force"]))
}

pub fn write_files(root: &Path, force: bool) -> Result<(), String> {
    for (rel, contents) in files() {
        let path = root.join(rel);
        if path.exists() && !force {
            ui::warn(&format!("{rel} already exists (use --force to replace it)"));
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, contents).map_err(|e| format!("{}: {e}", path.display()))?;
        ui::ok(&format!("wrote {rel}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_files_are_valid_json() {
        for (name, contents) in files() {
            let value: serde_json::Value = serde_json::from_str(contents).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(value.is_object(), "{name}");
        }
        let snippets: serde_json::Value = serde_json::from_str(SNIPPETS).unwrap();
        for (name, snippet) in snippets.as_object().unwrap() {
            assert!(snippet["prefix"].as_str().is_some_and(|p| p.starts_with("nr")), "{name}");
            assert!(snippet["body"].is_array(), "{name}");
        }
    }
}
