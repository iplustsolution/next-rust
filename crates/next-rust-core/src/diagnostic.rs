//! Structured diagnostics.
//!
//! Every user-facing error produced by scanning, validation or builds is a
//! [`Diagnostic`] with a stable code (e.g. `NR0102`), a one-line title, an
//! explanation, the source locations involved and an actionable hint.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub title: String,
    /// Longer explanation; may span multiple lines.
    pub message: String,
    /// Files involved, in the order they are relevant.
    pub locations: Vec<PathBuf>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, title: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, title)
    }

    pub fn warning(code: &str, title: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, title)
    }

    fn new(severity: Severity, code: &str, title: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.to_owned(),
            title: title.into(),
            message: String::new(),
            locations: Vec::new(),
            help: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn location(mut self, path: impl Into<PathBuf>) -> Self {
        self.locations.push(path.into());
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Render with ANSI colours.
    pub fn render(&self, color: bool) -> String {
        let (red, yellow, bold, dim, cyan, reset) = if color {
            ("\x1b[31m", "\x1b[33m", "\x1b[1m", "\x1b[2m", "\x1b[36m", "\x1b[0m")
        } else {
            ("", "", "", "", "", "")
        };
        let (label, col) = match self.severity {
            Severity::Error => ("error", red),
            Severity::Warning => ("warning", yellow),
        };
        let mut out = format!("{col}{bold}{label}[{}]{reset}{bold}: {}{reset}\n", self.code, self.title);
        for loc in &self.locations {
            out.push_str(&format!("  {cyan}-->{reset} {}\n", loc.display()));
        }
        if !self.message.is_empty() {
            out.push('\n');
            for line in self.message.lines() {
                out.push_str("  ");
                out.push_str(line);
                out.push('\n');
            }
        }
        if let Some(help) = &self.help {
            out.push_str(&format!("\n  {dim}help:{reset} {help}\n"));
        }
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(false))
    }
}

impl std::error::Error for Diagnostic {}

/// A collection of diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    pub fn push(&mut self, d: Diagnostic) {
        self.0.push(d);
    }

    pub fn has_errors(&self) -> bool {
        self.0.iter().any(Diagnostic::is_error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter(|d| d.is_error())
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter(|d| !d.is_error())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.0.iter()
    }

    pub fn render(&self, color: bool) -> String {
        self.0.iter().map(|d| d.render(color)).collect::<Vec<_>>().join("\n")
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(false))
    }
}

impl std::error::Error for Diagnostics {}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders() {
        let d = Diagnostic::error("NR0001", "Route conflict detected")
            .location("app/users/settings/page.rs")
            .message("Static route:\n    /users/settings")
            .help("rename one of the directories");
        let s = d.render(false);
        assert!(s.starts_with("error[NR0001]: Route conflict detected\n"));
        assert!(s.contains("--> app/users/settings/page.rs"));
        assert!(s.contains("    /users/settings"));
        assert!(s.contains("help: rename"));
    }
}
