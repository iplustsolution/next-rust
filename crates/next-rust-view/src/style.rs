//! Stylesheets created by the `css_module!` and `global_css!` macros.
//!
//! The macros read and transform CSS **at compile time** and produce
//! `&'static Stylesheet` values. A stylesheet participates in a page when a
//! [`CssClass`] from it is used in `class(..)`, or when the stylesheet itself
//! is placed in the view tree (global CSS). The renderer emits every used
//! stylesheet exactly once per document as a `<style>` element, so pages
//! only ship the CSS they actually use and streaming never produces a flash
//! of unstyled content.

use crate::node::{Node, View};

/// A compiled stylesheet.
#[derive(Debug, PartialEq, Eq)]
pub struct Stylesheet {
    /// Content hash (stable across builds for identical CSS).
    pub id: &'static str,
    /// Minified CSS text.
    pub css: &'static str,
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
