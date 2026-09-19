//! Stylesheets created by the `css_module!` and `global_css!` macros.
//!
//! The macros read and transform CSS **at compile time** and produce
//! `&'static Stylesheet` values. A stylesheet participates in a page when a
//! [`CssClass`] from it is used in `class(..)`, or when the stylesheet itself
//! is placed in the view tree (global CSS). The renderer emits every used
//! stylesheet exactly once per document as a `<style>` element, so pages
//! only ship the CSS they actually use and streaming never produces a flash
//! of unstyled content.

use std::sync::OnceLock;

use crate::node::{Node, View};

static OVERRIDES: OnceLock<Vec<(&'static Stylesheet, &'static Stylesheet)>> = OnceLock::new();

/// Render `to` wherever the view tree uses `from`, e.g. a stylesheet whose
/// classes were renamed for a release build. Set once at startup; later
/// calls are ignored.
pub fn set_overrides(list: Vec<(&'static Stylesheet, &'static Stylesheet)>) {
    let _ = OVERRIDES.set(list);
}

/// The stylesheet to render for `sheet`.
pub(crate) fn resolve(sheet: &'static Stylesheet) -> &'static Stylesheet {
    OVERRIDES
        .get()
        .and_then(|list| list.iter().find(|(from, _)| std::ptr::eq(*from, sheet)))
        .map_or(sheet, |(_, to)| to)
}

/// A compiled stylesheet.
#[derive(Debug, PartialEq, Eq)]
pub struct Stylesheet {
    /// Content hash (stable across builds for identical CSS).
    pub id: &'static str,
    /// Minified CSS text.
    pub css: &'static str,
    /// `Some` for stylesheets sent per page (the Tailwind build with the
    /// CSS imported into it, `global_css!` sheets, UI components): each page
    /// gets only the rules for the classes it renders, plus those for the
    /// listed classes (added by scripts, outside the view tree). `None`:
    /// sent whole.
    pub per_class: Option<&'static [&'static str]>,
    /// Classes that scripts add, by script (`"client/home.js"`,
    /// `"assets/js/site.js"`, a `public/` path): sent with a page only when
    /// it loads that script.
    pub scripts: &'static [(&'static str, &'static [&'static str])],
}

impl View for &'static Stylesheet {
    fn into_node(self) -> Node {
        Node::Style(self)
    }
}

/// A scoped class name from a CSS module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssClass {
    pub name: &'static str,
    pub sheet: &'static Stylesheet,
}

impl CssClass {
    pub const fn new(name: &'static str, sheet: &'static Stylesheet) -> Self {
        Self { name, sheet }
    }
}

impl std::fmt::Display for CssClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}
