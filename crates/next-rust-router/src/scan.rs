//! Recursive filesystem scanner.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use next_rust_core::config::HtmlPrecedence;
use next_rust_core::{Config, Diagnostic, Diagnostics};

use crate::flatten::{Route, flatten};
use crate::segment::{PatternSegment, SegmentKind, parse_segment};
use crate::tree::{RouteNode, SpecialFiles};
use crate::validate::validate;

/// Every file name with framework meaning, with a one-line description.
pub const SPECIAL_FILES: &[(&str, &str)] = &[
    ("page.rs", "UI for a route; makes the directory publicly routable"),
    ("page.html", "static HTML page for a route (alternative to page.rs)"),
    ("layout.rs", "UI shared by a segment and its children; wraps them"),
    ("template.rs", "like a layout, rendered inside it (re-created on client navigation)"),
    ("loading.rs", "fallback shown while the segment below streams in"),
    ("error.rs", "error boundary for the segment below"),
    ("global-error.rs", "root-level error document (app root only)"),
    ("not-found.rs", "UI for `not_found()` and unmatched URLs below this segment"),
    ("middleware.rs", "request middleware for this segment and everything below"),
    ("route.rs", "HTTP handlers (GET, POST, ...) – API endpoint"),
    ("metadata.rs", "document metadata for this segment (title, description, ...)"),
    ("default.rs", "fallback content of a parallel slot (`@slot`) with no matching page"),
    ("sitemap.rs", "generates /sitemap.xml (app root only)"),
    ("robots.rs", "generates /robots.txt (app root only)"),
];

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub html_precedence: HtmlPrecedence,
    pub follow_symlinks: bool,
    /// URL prefix the scanned directory is mounted at (e.g. `/api`).
    pub mount: String,
    /// Only `route.rs` (and middleware) files are allowed.
    pub api_only: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self { html_precedence: HtmlPrecedence::Error, follow_symlinks: true, mount: String::new(), api_only: false }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScanOutput {
    /// One tree per scanned root.
    pub trees: Vec<RouteNode>,
    pub routes: Vec<Route>,
    pub diagnostics: Diagnostics,
}

impl ScanOutput {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    pub fn tree(&self) -> Option<&RouteNode> {
        self.trees.first()
    }
}

/// Scan the directories configured in `config` (app directory and optional
/// API directory), flatten and validate the result.
pub fn scan_project(config: &Config) -> ScanOutput {
    let mut out = ScanOutput::default();
    for d in config.validate() {
        out.diagnostics.push(d);
    }
    if out.diagnostics.has_errors() {
        return out;
    }
    let opts = ScanOptions {
        html_precedence: config.app.html_precedence,
        follow_symlinks: config.app.follow_symlinks,
        mount: String::new(),
        api_only: false,
    };
    let mut roots = vec![(config.app_dir(), opts.clone())];
    if let Some(api) = config.api_dir() {
        roots.push((api, ScanOptions { mount: config.api.prefix.clone(), api_only: true, ..opts }));
    }
    for (dir, opts) in roots {
        let (tree, diags) = scan_tree(&dir, &opts);
        out.diagnostics.0.extend(diags);
        if let Some(tree) = tree {
            let mount = mount_segments(&opts.mount);
            out.routes.extend(flatten(&tree, &mount, &mut out.diagnostics));
            out.trees.push(tree);
        }
    }
    out.diagnostics.0.extend(validate(&out.trees, &out.routes));
    sort_routes(&mut out.routes);
    out
}

/// Scan a single directory with default mount.
pub fn scan(root: &Path, opts: &ScanOptions) -> ScanOutput {
    let mut out = ScanOutput::default();
    let (tree, diags) = scan_tree(root, opts);
    out.diagnostics.0.extend(diags);
    if let Some(tree) = tree {
        out.routes = flatten(&tree, &mount_segments(&opts.mount), &mut out.diagnostics);
        out.trees.push(tree);
    }
    out.diagnostics.0.extend(validate(&out.trees, &out.routes));
    sort_routes(&mut out.routes);
    out
}

fn sort_routes(routes: &mut [Route]) {
    routes.sort_by(|a, b| {
        crate::rank::compare_patterns(&a.pattern, &b.pattern)
            .then_with(|| a.kind.order().cmp(&b.kind.order()))
            .then_with(|| a.source.cmp(&b.source))
    });
}

fn mount_segments(mount: &str) -> Vec<PatternSegment> {
    mount.split('/').filter(|s| !s.is_empty()).map(|s| PatternSegment::Static(s.to_owned())).collect()
}

fn scan_tree(root: &Path, opts: &ScanOptions) -> (Option<RouteNode>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let meta = match std::fs::metadata(root) {
        Ok(m) => m,
        Err(e) => {
            diags.push(
                Diagnostic::error("NR0001", "Routing directory not found")
                    .location(root)
                    .message(format!("{e}"))
                    .help("check `[app] directory` in next-rust.toml"),
            );
            return (None, diags);
        }
    };
    if !meta.is_dir() {
        diags.push(Diagnostic::error("NR0002", "Routing path is not a directory").location(root));
        return (None, diags);
    }
    let mut ancestors = HashSet::new();
    let mut scanner = Scanner { opts, diags: &mut diags, root };
    let node = scanner.scan_dir(root, String::new(), SegmentKind::Root, &mut ancestors);
    (Some(node), diags)
}

struct Scanner<'a> {
    opts: &'a ScanOptions,
    diags: &'a mut Vec<Diagnostic>,
    root: &'a Path,
}

impl Scanner<'_> {
    fn scan_dir(&mut self, dir: &Path, name: String, kind: SegmentKind, ancestors: &mut HashSet<PathBuf>) -> RouteNode {
        let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        ancestors.insert(canonical.clone());

        let mut node =
            RouteNode { name, kind, dir: dir.to_path_buf(), files: SpecialFiles::default(), children: Vec::new() };

        let mut entries: Vec<_> = match std::fs::read_dir(dir) {
            Ok(rd) => rd.filter_map(Result::ok).collect(),
            Err(e) => {
                self.diags
                    .push(Diagnostic::error("NR0109", "Cannot read directory").location(dir).message(e.to_string()));
                ancestors.remove(&canonical);
                return node;
            }
        };
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                self.diags.push(
                    Diagnostic::warning("NR0113", "Skipping non UTF-8 path")
                        .location(&path)
                        .message("Route directories and special files must have UTF-8 names."),
                );
                continue;
            };
            let Ok(ft) = entry.file_type() else { continue };
            let (is_dir, is_file) = if ft.is_symlink() {
                if !self.opts.follow_symlinks {
                    continue;
                }
                match std::fs::metadata(&path) {
                    Ok(m) => (m.is_dir(), m.is_file()),
                    Err(_) => {
                        self.diags.push(Diagnostic::warning("NR0114", "Broken symbolic link ignored").location(&path));
                        continue;
                    }
                }
            } else {
                (ft.is_dir(), ft.is_file())
            };

            if is_file {
                self.register_file(&mut node, &file_name, path);
            } else if is_dir {
                let kind = match parse_segment(&file_name) {
                    Ok(Some(kind)) => kind,
                    Ok(None) => continue,
                    Err(reason) => {
                        self.diags.push(
                            Diagnostic::error("NR0108", "Invalid route segment name")
                                .location(&path)
                                .message(reason)
                                .help("prefix the directory with `_` to exclude it from routing"),
                        );
                        continue;
                    }
                };
                let child_canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if ancestors.contains(&child_canonical) {
                    self.diags.push(
                        Diagnostic::warning("NR0110", "Symbolic link loop skipped")
                            .location(&path)
                            .message(format!("`{}` points back to one of its parent directories.", path.display())),
                    );
                    continue;
                }
                let child = self.scan_dir(&path, file_name, kind, ancestors);
                node.children.push(child);
            }
        }

        self.resolve_html_conflict(&mut node);
        ancestors.remove(&canonical);
        node
    }

    fn register_file(&mut self, node: &mut RouteNode, file_name: &str, path: PathBuf) {
        let is_root = matches!(node.kind, SegmentKind::Root);
        if let Some(slot) = node.files.slot_mut(file_name) {
            let root_only = matches!(file_name, "global-error.rs" | "sitemap.rs" | "robots.rs");
            if root_only && !is_root {
                self.diags.push(
                    Diagnostic::error("NR0111", format!("`{file_name}` is only valid in the app root"))
                        .location(&path)
                        .help(format!("move it to {}", self.root.join(file_name).display())),
                );
                return;
            }
            if self.opts.api_only && !matches!(file_name, "route.rs" | "middleware.rs") {
                self.diags.push(
                    Diagnostic::warning("NR0112", format!("`{file_name}` ignored in the API directory"))
                        .location(&path)
                        .message("The API directory only supports `route.rs` and `middleware.rs`."),
                );
                return;
            }
            *slot = Some(path);
            return;
        }
        // Helpful hints for common mistakes.
        let lower = file_name.to_ascii_lowercase();
        if lower != file_name && node.files.clone().slot_mut(&lower).is_some() {
            self.diags.push(
                Diagnostic::warning("NR0120", format!("`{file_name}` looks like special file `{lower}`"))
                    .location(&path)
                    .message("Special file names are case-sensitive; this file is ignored."),
            );
        } else if let Some(stem) = ["page", "layout", "route", "loading", "error", "not-found", "template"]
            .into_iter()
            .find(|s| ["tsx", "jsx", "ts", "js", "mdx"].iter().any(|ext| file_name == format!("{s}.{ext}")))
        {
            self.diags.push(
                Diagnostic::warning("NR0121", format!("`{file_name}` is not a Next Rust file"))
                    .location(&path)
                    .help(format!("Next Rust uses Rust files: rename to `{stem}.rs`")),
            );
        }
    }

    fn resolve_html_conflict(&mut self, node: &mut RouteNode) {
        if let (Some(rs), Some(html)) = (&node.files.page, &node.files.page_html) {
            match self.opts.html_precedence {
                HtmlPrecedence::Error => {
                    self.diags.push(
                        Diagnostic::error("NR0101", "Both page.rs and page.html define the same route")
                            .location(rs)
                            .location(html)
                            .message("A route directory may contain either `page.rs` or `page.html`, not both.")
                            .help("delete one of the files, or set `[app] html_precedence = \"rs\"` (or \"html\")"),
                    );
                }
                HtmlPrecedence::Rs => {
                    self.diags.push(
                        Diagnostic::warning("NR0101", "page.html ignored because page.rs takes precedence")
                            .location(html),
                    );
                    node.files.page_html = None;
                }
                HtmlPrecedence::Html => {
                    self.diags.push(
                        Diagnostic::warning("NR0101", "page.rs ignored because page.html takes precedence")
                            .location(rs),
                    );
                    node.files.page = None;
                }
            }
        }
    }
}
