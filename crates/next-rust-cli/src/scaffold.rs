//! Starter code for special files.
//!
//! Used by `next-rust generate` and by `next-rust dev`, which fills newly
//! created, empty special files (for example `app/about/page.rs`) so a new
//! route works immediately.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Special file names and the template kind that fills them.
const KINDS: &[(&str, &str)] = &[
    ("page.rs", "page"),
    ("layout.rs", "layout"),
    ("template.rs", "template"),
    ("loading.rs", "loading"),
    ("error.rs", "error"),
    ("not-found.rs", "not-found"),
    ("global-error.rs", "global-error"),
    ("route.rs", "api"),
    ("middleware.rs", "middleware"),
    ("metadata.rs", "metadata"),
    ("default.rs", "default"),
    ("sitemap.rs", "sitemap"),
    ("robots.rs", "robots"),
];

/// The template kind for a file name, if it is a special file.
pub fn kind_for(file_name: &str) -> Option<&'static str> {
    KINDS.iter().find(|(name, _)| *name == file_name).map(|(_, kind)| *kind)
}

/// URL-style route of the directory containing `file`, e.g. `/blog/[slug]`.
fn route_of(app_dir: &Path, file: &Path) -> String {
    let dir = file.parent().unwrap_or(app_dir);
    let rel = dir.strip_prefix(app_dir).unwrap_or(Path::new(""));
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    format!("/{}", parts.join("/"))
}

/// "about-us" → "About Us". Route groups, dynamic segments and slots are
/// skipped when picking the title; the root is "Home".
pub fn title_for(route: &str) -> String {
    let segment = route
        .split('/')
        .rev()
        .find(|s| !s.is_empty() && !s.starts_with('(') && !s.starts_with('[') && !s.starts_with('@'));
    match segment {
        None => "Home".into(),
        Some(s) => s
            .split(['-', '_'])
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut chars = w.chars();
                chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Dynamic parameter names in a route (`[id]`, `[...slug]`, `[[...slug]]`).
fn params_of(route: &str) -> Vec<String> {
    route
        .split('/')
        .filter_map(|s| s.strip_prefix('[').and_then(|s| s.strip_suffix(']')))
        .map(|s| s.trim_start_matches('[').trim_end_matches(']').trim_start_matches("...").to_owned())
        .collect()
}

/// Starter code for a template kind at a route.
pub fn template(kind: &str, route: &str) -> Option<String> {
    let title = title_for(route);
    let params = params_of(route);
    let display_route = if route.is_empty() { "/" } else { route };
    Some(match kind {
        "page" if !params.is_empty() => {
            let lines: String = params
                .iter()
                .map(|p| format!("        p![format!(\"{p}: {{:?}}\", params.get_all({p:?}).unwrap_or_default())],\n"))
                .collect();
            format!(
                "use next_rust::prelude::*;\n\npub fn metadata() -> Metadata {{\n    Metadata::new().title({title:?})\n}}\n\npub fn Page(params: Params) -> impl View {{\n    main![\n        h1![{title:?}],\n{lines}    ]\n}}\n"
            )
        }
        "page" | "default" => format!(
            "use next_rust::prelude::*;\n\npub fn metadata() -> Metadata {{\n    Metadata::new().title({title:?})\n}}\n\npub fn Page() -> impl View {{\n    main![\n        h1![{title:?}],\n        p![\"This is the {display_route} page.\"],\n    ]\n}}\n"
        ),
        "layout" => "use next_rust::prelude::*;\n\npub fn Layout(children: Children) -> impl View {\n    section![children]\n}\n".into(),
        "template" => "use next_rust::prelude::*;\n\npub fn Template(children: Children) -> impl View {\n    div![children]\n}\n".into(),
        "loading" => "use next_rust::prelude::*;\n\npub fn Loading() -> impl View {\n    p![aria(\"busy\", \"true\"), \"Loading…\"]\n}\n".into(),
        "error" => "use next_rust::prelude::*;\n\npub fn ErrorBoundary(info: ErrorInfo) -> impl View {\n    div![\n        role(\"alert\"),\n        h2![\"Something went wrong\"],\n        p![info.message],\n        small![format!(\"Reference: {}\", info.digest)],\n    ]\n}\n".into(),
        "global-error" => "use next_rust::prelude::*;\n\npub fn GlobalError(info: ErrorInfo) -> impl View {\n    main![\n        h1![\"Something went wrong\"],\n        p![format!(\"Reference: {}\", info.digest)],\n    ]\n}\n".into(),
        "not-found" => "use next_rust::prelude::*;\n\npub fn NotFound() -> impl View {\n    main![h1![\"Not found\"], Link!(href = \"/\", \"Back to home\")]\n}\n".into(),
        "api" => format!(
            "use next_rust::prelude::*;\n\n/// GET {display_route}\npub async fn GET(req: Request) -> Response {{\n    Response::json(&serde_json::json!({{ \"path\": req.path() }}))\n}}\n\n/// POST {display_route}\npub async fn POST(mut req: Request) -> Result<Response> {{\n    let body: serde_json::Value = req.json().await?;\n    Ok(Response::json(&body).with_status(201))\n}}\n"
        ),
        "middleware" => "use next_rust::prelude::*;\n\npub async fn middleware(req: Request, next: Next) -> Response {\n    next.run(req).await\n}\n".into(),
        "metadata" => format!("use next_rust::prelude::*;\n\npub fn metadata() -> Metadata {{\n    Metadata::new().title({title:?})\n}}\n"),
        "sitemap" => "use next_rust::prelude::*;\n\npub async fn sitemap() -> Sitemap {\n    Sitemap::new().url(\"https://example.com/\")\n}\n".into(),
        "robots" => "use next_rust::prelude::*;\n\npub fn robots() -> Robots {\n    Robots::allow_all()\n}\n".into(),
        _ => return None,
    })
}

/// Starter code for a special file at `file` inside `app_dir`.
pub fn starter_for(app_dir: &Path, file: &Path) -> Option<String> {
    let kind = kind_for(file.file_name()?.to_str()?)?;
    template(kind, &route_of(app_dir, file))
}

/// Tracks special files so only newly created ones are filled.
#[derive(Default)]
pub struct Scaffolder {
    seen: HashSet<PathBuf>,
}

impl Scaffolder {
    /// Fill special files that appeared since the last call and are empty.
    /// Returns the files that were filled. Existing content is never touched.
    pub fn fill_new(&mut self, roots: &[PathBuf]) -> Vec<PathBuf> {
        let mut current = HashSet::new();
        for root in roots {
            collect_special(root, &mut current);
        }
        let mut filled = Vec::new();
        let mut new_files: Vec<&PathBuf> = current.difference(&self.seen).collect();
        new_files.sort();
        for file in new_files {
            let empty = std::fs::read_to_string(file).is_ok_and(|text| text.trim().is_empty());
            if !empty {
                continue;
            }
            let root = roots.iter().find(|r| file.starts_with(r)).cloned().unwrap_or_default();
            if let Some(code) = starter_for(&root, file)
                && std::fs::write(file, code).is_ok()
            {
                filled.push(file.clone());
            }
        }
        self.seen = current;
        filled
    }
}

fn collect_special(dir: &Path, out: &mut HashSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if !name.starts_with('.') && !name.starts_with('_') {
                collect_special(&path, out);
            }
        } else if kind_for(&name).is_some() {
            out.insert(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_routes() {
        assert_eq!(title_for("/"), "Home");
        assert_eq!(title_for("/about-us"), "About Us");
        assert_eq!(title_for("/(marketing)/pricing_plans"), "Pricing Plans");
        assert_eq!(title_for("/blog/[slug]"), "Blog");
        assert_eq!(route_of(Path::new("/p/app"), Path::new("/p/app/blog/[slug]/page.rs")), "/blog/[slug]");
        assert_eq!(route_of(Path::new("/p/app"), Path::new("/p/app/page.rs")), "/");
    }

    #[test]
    fn templates_for_every_special_file() {
        for (name, _) in KINDS {
            let code = starter_for(Path::new("/p/app"), &Path::new("/p/app/about").join(name));
            assert!(code.is_some_and(|c| c.starts_with("use next_rust::prelude::*;")), "{name}");
        }
        let page = starter_for(Path::new("/p/app"), Path::new("/p/app/about/page.rs")).unwrap();
        assert!(page.contains("h1![\"About\"]") && page.contains("This is the /about page."));
        let dynamic = template("page", "/users/[id]").unwrap();
        assert!(dynamic.contains("pub fn Page(params: Params)") && dynamic.contains("params.get_all(\"id\")"));
        assert!(starter_for(Path::new("/p/app"), Path::new("/p/app/about/helper.rs")).is_none());
    }

    #[test]
    fn fills_only_new_empty_files() {
        let root = std::env::temp_dir().join(format!("nr-scaffold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = root.join("app");
        std::fs::create_dir_all(app.join("about")).unwrap();
        std::fs::write(app.join("page.rs"), "// my code\n").unwrap();
        std::fs::create_dir_all(app.join("_private")).unwrap();
        std::fs::write(app.join("_private/page.rs"), "").unwrap();

        let mut s = Scaffolder::default();
        assert!(s.fill_new(std::slice::from_ref(&app)).is_empty(), "existing content is untouched");

        std::fs::write(app.join("about/page.rs"), "").unwrap();
        std::fs::write(app.join("about/route.rs"), "\n").unwrap();
        let filled = s.fill_new(std::slice::from_ref(&app));
        assert_eq!(filled, vec![app.join("about/page.rs"), app.join("about/route.rs")]);
        assert!(std::fs::read_to_string(app.join("about/page.rs")).unwrap().contains("h1![\"About\"]"));
        assert_eq!(std::fs::read_to_string(app.join("page.rs")).unwrap(), "// my code\n");
        assert_eq!(std::fs::read_to_string(app.join("_private/page.rs")).unwrap(), "", "private folders are skipped");

        // A file the user empties later is left alone.
        std::fs::write(app.join("about/page.rs"), "").unwrap();
        assert!(s.fill_new(std::slice::from_ref(&app)).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
