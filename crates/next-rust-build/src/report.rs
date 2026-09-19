//! What a release build did, written to `$OUT_DIR/next_rust_build_report.json`
//! for `next-rust build` to show.

use serde::{Deserialize, Serialize};

/// File in `$OUT_DIR` holding the report.
pub const FILE: &str = "next_rust_build_report.json";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BuildReport {
    /// Short class names, when classes were renamed.
    pub classes: Option<ClassReport>,
    /// Classes in the Rust source that look generated but nothing knows:
    /// they are left out of pages.
    pub unknown_classes: Vec<String>,
    /// Scripts embedded in the binary.
    pub scripts: Vec<ScriptReport>,
    /// Every embedded file.
    pub files: Vec<FileReport>,
    /// Size of the Tailwind stylesheet, minified.
    pub tailwind_css: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClassReport {
    pub total: usize,
    pub one_char: usize,
    pub two_chars: usize,
    pub three_chars: usize,
    pub longer: usize,
    /// Classes that keep their names (scripts use them in ways that can't be
    /// rewritten, or `keep_classes`).
    pub kept: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScriptReport {
    pub path: String,
    pub bytes: usize,
    /// Size after class renaming and minification.
    pub minified: usize,
    /// Class occurrences rewritten with short names.
    pub renamed: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileReport {
    pub path: String,
    pub bytes: usize,
    pub brotli: Option<usize>,
    pub gzip: Option<usize>,
}
