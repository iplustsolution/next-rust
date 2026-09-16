//! Serializable route manifest (`.next-rust/manifest/routes.json`).
//!
//! The manifest is a stable, versioned JSON document describing every route
//! of a build. Tooling (deploy adapters, analyzers, CDNs) should read it
//! instead of re-scanning the filesystem.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::flatten::Route;
use crate::params::Params;

pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RouteManifest {
    pub version: u32,
    pub framework_version: String,
    pub routes: Vec<ManifestRoute>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ManifestRoute {
    pub id: String,
    /// `page`, `html` or `api`.
    pub kind: String,
    /// Display form: `/blog/:slug`.
    pub path: String,
    /// Filesystem form: `/blog/[slug]`.
    pub pattern: String,
    /// Paths relative to the project root, `/`-separated.
    pub source: String,
    pub layouts: Vec<String>,
    pub templates: Vec<String>,
    pub loading: Vec<String>,
    pub error_boundaries: Vec<String>,
    pub not_found: Vec<String>,
    pub middleware: Vec<String>,
    pub metadata: Vec<String>,
    pub slots: Vec<String>,
    /// HTTP methods (API routes: exported handlers; pages: `GET`, `HEAD`).
    pub methods: Vec<String>,
    /// `static`, `dynamic` (after build analysis).
    pub rendering: String,
    pub revalidate: Option<u64>,
    /// Parameters pre-rendered at build time.
    pub static_params: Vec<Params>,
    pub intercepts_from: Option<String>,
}

impl ManifestRoute {
    pub fn from_route(route: &Route, project_root: &Path) -> Self {
        let rel = |p: &Path| relative(p, project_root);
        let collect = |f: &dyn Fn(&crate::SegmentEntry) -> Option<&std::path::PathBuf>| -> Vec<String> {
            route.chain.iter().filter_map(f).map(|p| rel(p)).collect()
        };
        Self {
            id: route.id(),
            kind: route.kind.as_str().into(),
            path: route.pattern.to_display_string(),
            pattern: route.pattern.to_fs_string(),
            source: rel(&route.source),
            layouts: collect(&|s| s.layout.as_ref()),
            templates: collect(&|s| s.template.as_ref()),
            loading: collect(&|s| s.loading.as_ref()),
            error_boundaries: collect(&|s| s.error.as_ref()),
            not_found: collect(&|s| s.not_found.as_ref()),
            middleware: collect(&|s| s.middleware.as_ref()),
            metadata: collect(&|s| s.metadata.as_ref()),
            slots: route.chain.iter().flat_map(|s| s.slots.iter().map(|sl| format!("@{}", sl.name))).collect(),
            methods: if route.is_page() { vec!["GET".into(), "HEAD".into()] } else { Vec::new() },
            rendering: String::new(),
            revalidate: None,
            static_params: Vec::new(),
            intercepts_from: route.intercept.as_ref().map(|i| i.context.to_fs_string()),
        }
    }
}

impl RouteManifest {
    pub fn new(routes: &[Route], project_root: &Path) -> Self {
        Self {
            version: MANIFEST_VERSION,
            framework_version: env!("CARGO_PKG_VERSION").into(),
            routes: routes.iter().map(|r| ManifestRoute::from_route(r, project_root)).collect(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("manifest serialization cannot fail")
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// Path relative to `base` with `/` separators (falls back to the full path).
pub fn relative(path: &Path, base: &Path) -> String {
    let p = path.strip_prefix(base).unwrap_or(path);
    p.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}
