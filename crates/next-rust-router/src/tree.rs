use std::path::PathBuf;

use crate::segment::SegmentKind;

/// Special files found in one routing directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecialFiles {
    pub page: Option<PathBuf>,
    pub page_html: Option<PathBuf>,
    pub layout: Option<PathBuf>,
    pub template: Option<PathBuf>,
    pub loading: Option<PathBuf>,
    pub error: Option<PathBuf>,
    pub global_error: Option<PathBuf>,
    pub not_found: Option<PathBuf>,
    pub middleware: Option<PathBuf>,
    pub route: Option<PathBuf>,
    pub metadata: Option<PathBuf>,
    pub default: Option<PathBuf>,
    pub sitemap: Option<PathBuf>,
    pub robots: Option<PathBuf>,
}

impl SpecialFiles {
    pub fn is_empty(&self) -> bool {
        *self == SpecialFiles::default()
    }

    /// Mutable slot for a special file name, if the name is special.
    pub(crate) fn slot_mut(&mut self, file_name: &str) -> Option<&mut Option<PathBuf>> {
        Some(match file_name {
            "page.rs" => &mut self.page,
            "page.html" => &mut self.page_html,
            "layout.rs" => &mut self.layout,
            "template.rs" => &mut self.template,
            "loading.rs" => &mut self.loading,
            "error.rs" => &mut self.error,
            "global-error.rs" => &mut self.global_error,
            "not-found.rs" => &mut self.not_found,
            "middleware.rs" => &mut self.middleware,
            "route.rs" => &mut self.route,
            "metadata.rs" => &mut self.metadata,
            "default.rs" => &mut self.default,
            "sitemap.rs" => &mut self.sitemap,
            "robots.rs" => &mut self.robots,
            _ => return None,
        })
    }

    /// Iterate over all present files.
    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        [
            &self.page,
            &self.page_html,
            &self.layout,
            &self.template,
            &self.loading,
            &self.error,
            &self.global_error,
            &self.not_found,
            &self.middleware,
            &self.route,
            &self.metadata,
            &self.default,
            &self.sitemap,
            &self.robots,
        ]
        .into_iter()
        .flatten()
    }
}

/// One directory in the routing tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteNode {
    /// Directory name (empty for the root).
    pub name: String,
    pub kind: SegmentKind,
    /// Absolute directory path.
    pub dir: PathBuf,
    pub files: SpecialFiles,
    /// Child directories sorted by name (slots included).
    pub children: Vec<RouteNode>,
}

impl RouteNode {
    pub fn slots(&self) -> impl Iterator<Item = (&str, &RouteNode)> {
        self.children.iter().filter_map(|c| match &c.kind {
            SegmentKind::Slot(name) => Some((name.as_str(), c)),
            _ => None,
        })
    }

    /// Total number of directories in this subtree.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(RouteNode::count).sum::<usize>()
    }

    /// Render an indented tree (used by `next-rust routes --tree`).
    pub fn render_tree(&self, root: &std::path::Path) -> String {
        let mut out = String::new();
        self.render_into(&mut out, root, "", true, true);
        out
    }

    fn render_into(&self, out: &mut String, root: &std::path::Path, prefix: &str, last: bool, is_root: bool) {
        let label = if is_root { root.display().to_string() } else { self.name.clone() };
        let mut files: Vec<String> =
            self.files.iter().filter_map(|p| p.file_name().and_then(|f| f.to_str()).map(str::to_owned)).collect();
        files.sort();
        let files = if files.is_empty() { String::new() } else { format!("  [{}]", files.join(", ")) };
        if is_root {
            out.push_str(&format!("{label}{files}\n"));
        } else {
            out.push_str(&format!("{prefix}{}{label}{files}\n", if last { "└── " } else { "├── " }));
        }
        let child_prefix =
            if is_root { String::new() } else { format!("{prefix}{}", if last { "    " } else { "│   " }) };
        let n = self.children.len();
        for (i, c) in self.children.iter().enumerate() {
            c.render_into(out, root, &child_prefix, i + 1 == n, false);
        }
    }
}
