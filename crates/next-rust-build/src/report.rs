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
    /// Short names for `data-*` attributes, custom properties, keyframes,
    /// layers and ids (`[build] minify_names`).
    pub names: Option<NamesReport>,
    /// The WebAssembly modules the build compiled, one per contributing
    /// crate: `[build] wasm_islands`, `[client] runtime`, `[ui] script`.
    /// Absent from reports written before there were any.
    #[serde(default)]
    pub wasm: Vec<WasmReport>,
}

/// One WebAssembly module under `client/__nrw/`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WasmReport {
    /// Module name, which is also its file name: `islands`, `runtime`, `ui`.
    pub module: String,
    /// Size of the module as written.
    pub bytes: usize,
    /// Size compressed with Brotli, as the server serves it.
    pub brotli: Option<usize>,
    /// Size compressed with gzip.
    pub gzip: Option<usize>,
    /// Whether `wasm-opt` shrank it. `false` means it was skipped, absent or
    /// made no difference — the module is correct either way, only larger.
    pub optimized: bool,
    /// The JavaScript shim that loads and drives this module, when it has
    /// one. It is downloaded with the module, so its size belongs next to it.
    #[serde(default)]
    pub shim: Option<ShimReport>,
    /// What choosing the wasm browser runtime costs against the JavaScript
    /// one, when `[client] runtime = "wasm"` put it in this module.
    #[serde(default)]
    pub runtime: Option<RuntimeReport>,
    /// The islands this module hydrates, from its export section.
    pub islands: Vec<WasmIslandReport>,
}

/// One `hydrate_<Island>` export of a module.
///
/// Both sizes count **function bodies in the code section only**, walked from
/// this island's export. Two things they therefore cannot tell you, and must
/// not be presented as telling you: string literals and other data-section
/// bytes are never attributed to an island, and `wasm-opt` inlines across
/// functions, so these are facts about the compiled module rather than about
/// the source an island was written from.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WasmIslandReport {
    /// Island name, as pages write it: the export name without `hydrate_`.
    pub name: String,
    /// Bytes only this island can reach: roughly what deleting it would give
    /// back. `None` when the module could not be analyzed.
    #[serde(default)]
    pub exclusive: Option<usize>,
    /// Bytes this island can reach at all, shared code included. These
    /// overlap between islands and do not add up to the module size.
    #[serde(default)]
    pub reachable: Option<usize>,
}

/// The cost of the wasm browser runtime against the JavaScript one.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RuntimeReport {
    /// Brotli size of `/_next-rust/runtime.wasm`, the prebuilt module
    /// `[client] runtime = "wasm"` serves. It is a **separate download** from
    /// the islands module beside it, not a share of it: a page with both
    /// fetches each one, and each carries its own copy of the shared
    /// machinery.
    pub wasm_brotli: usize,
    /// Size of the JavaScript runtime it replaces, minified.
    pub js_bytes: usize,
    /// Brotli size of the JavaScript runtime: the number to compare against.
    pub js_brotli: usize,
}

/// The loader a wasm module is served with.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ShimReport {
    /// Size as generated.
    pub bytes: usize,
    /// Size after minification, when the build minifies scripts.
    pub minified: Option<usize>,
    /// Size compressed with Brotli, which is what a visitor downloads.
    pub brotli: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NamesReport {
    /// Names that got a short name.
    pub renamed: usize,
    /// Names that keep their spelling, as `name: reason`.
    pub kept: Vec<String>,
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
