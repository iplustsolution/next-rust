//! Directory-name → segment parsing.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Classification of a routing directory name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SegmentKind {
    /// The routing root itself.
    Root,
    /// `about` → matches exactly `about`.
    Static(String),
    /// `[id]` → matches one segment.
    Dynamic(String),
    /// `[...slug]` → matches one or more segments.
    CatchAll(String),
    /// `[[...slug]]` → matches zero or more segments.
    OptionalCatchAll(String),
    /// `(marketing)` → organisational only; not part of the URL.
    Group(String),
    /// `@analytics` → a named parallel slot rendered by the parent layout.
    Slot(String),
    /// `(.)photo`, `(..)photo`, `(...)photo` → intercepting route.
    Intercept { level: InterceptLevel, inner: Box<SegmentKind> },
}

/// How far up an intercepting route reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterceptLevel {
    /// `(.)` – same level as the directory containing the interceptor.
    Same,
    /// `(..)`, `(..)(..)` – `n` URL segments up.
    Up(usize),
    /// `(...)` – from the root.
    Root,
}

/// A URL-relevant segment of a route pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PatternSegment {
    Static(String),
    Dynamic(String),
    CatchAll(String),
    OptionalCatchAll(String),
}

impl PatternSegment {
    pub fn param_name(&self) -> Option<&str> {
        match self {
            PatternSegment::Static(_) => None,
            PatternSegment::Dynamic(n) | PatternSegment::CatchAll(n) | PatternSegment::OptionalCatchAll(n) => Some(n),
        }
    }

    pub fn is_catch_all(&self) -> bool {
        matches!(self, PatternSegment::CatchAll(_) | PatternSegment::OptionalCatchAll(_))
    }

    /// Shape without parameter names, used for conflict detection.
    pub fn shape(&self) -> String {
        match self {
            PatternSegment::Static(s) => s.clone(),
            PatternSegment::Dynamic(_) => "[]".into(),
            PatternSegment::CatchAll(_) => "[...]".into(),
            PatternSegment::OptionalCatchAll(_) => "[[...]]".into(),
        }
    }
}

/// A route pattern such as `/blog/[slug]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoutePattern(pub Vec<PatternSegment>);

impl RoutePattern {
    pub fn segments(&self) -> &[PatternSegment] {
        &self.0
    }

    /// Filesystem-style notation: `/blog/[slug]`, `/docs/[...slug]`.
    pub fn to_fs_string(&self) -> String {
        if self.0.is_empty() {
            return "/".into();
        }
        let mut s = String::new();
        for seg in &self.0 {
            s.push('/');
            match seg {
                PatternSegment::Static(v) => s.push_str(v),
                PatternSegment::Dynamic(n) => s.push_str(&format!("[{n}]")),
                PatternSegment::CatchAll(n) => s.push_str(&format!("[...{n}]")),
                PatternSegment::OptionalCatchAll(n) => s.push_str(&format!("[[...{n}]]")),
            }
        }
        s
    }

    /// Express-style notation: `/blog/:slug`, `/docs/*slug`, `/docs/*slug?`.
    pub fn to_display_string(&self) -> String {
        if self.0.is_empty() {
            return "/".into();
        }
        let mut s = String::new();
        for seg in &self.0 {
            s.push('/');
            match seg {
                PatternSegment::Static(v) => s.push_str(v),
                PatternSegment::Dynamic(n) => s.push_str(&format!(":{n}")),
                PatternSegment::CatchAll(n) => s.push_str(&format!("*{n}")),
                PatternSegment::OptionalCatchAll(n) => s.push_str(&format!("*{n}?")),
            }
        }
        // `/docs/*slug?` matches `/docs` too
        s
    }

    /// Normalized shape used to detect routes that resolve to the same URLs.
    pub fn shape(&self) -> String {
        let parts: Vec<String> = self.0.iter().map(PatternSegment::shape).collect();
        format!("/{}", parts.join("/"))
    }

    pub fn is_dynamic(&self) -> bool {
        self.0.iter().any(|s| !matches!(s, PatternSegment::Static(_)))
    }

    pub fn param_names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(PatternSegment::param_name)
    }

    /// Build a concrete URL path from parameters. Returns `None` if a
    /// parameter is missing. Values are percent-encoded.
    pub fn to_path(&self, params: &crate::Params) -> Option<String> {
        let mut out = String::new();
        for seg in &self.0 {
            match seg {
                PatternSegment::Static(v) => {
                    out.push('/');
                    out.push_str(&crate::encode_segment(v));
                }
                PatternSegment::Dynamic(n) => {
                    out.push('/');
                    out.push_str(&crate::encode_segment(params.get(n)?));
                }
                PatternSegment::CatchAll(n) => {
                    let all = params.get_all(n)?;
                    if all.is_empty() {
                        return None;
                    }
                    for v in all {
                        out.push('/');
                        out.push_str(&crate::encode_segment(v));
                    }
                }
                PatternSegment::OptionalCatchAll(n) => {
                    for v in params.get_all(n).unwrap_or(&[]) {
                        out.push('/');
                        out.push_str(&crate::encode_segment(v));
                    }
                }
            }
        }
        Some(if out.is_empty() { "/".into() } else { out })
    }
}

impl fmt::Display for RoutePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_fs_string())
    }
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse a directory name.
///
/// Returns `Ok(None)` for directories that are ignored by routing:
/// names starting with `_` (private folders for colocated code) or `.`
/// (hidden folders).
pub fn parse_segment(name: &str) -> Result<Option<SegmentKind>, String> {
    if name.is_empty() {
        return Err("empty directory name".into());
    }
    if name.starts_with('_') || name.starts_with('.') {
        return Ok(None);
    }

    // Interception markers.
    if name.starts_with("(.)") || name.starts_with("(..)") || name.starts_with("(...)") {
        let (level, rest) = if let Some(rest) = name.strip_prefix("(...)") {
            (InterceptLevel::Root, rest)
        } else if let Some(rest) = name.strip_prefix("(.)") {
            (InterceptLevel::Same, rest)
        } else {
            let mut rest = name;
            let mut n = 0;
            while let Some(r) = rest.strip_prefix("(..)") {
                rest = r;
                n += 1;
            }
            (InterceptLevel::Up(n), rest)
        };
        if rest.is_empty() {
            return Err(format!("intercepting route `{name}` needs a segment after the marker, e.g. `(.)photo`"));
        }
        let inner = match parse_segment(rest)? {
            Some(
                k @ (SegmentKind::Static(_)
                | SegmentKind::Dynamic(_)
                | SegmentKind::CatchAll(_)
                | SegmentKind::OptionalCatchAll(_)),
            ) => k,
            _ => return Err(format!("intercepting route `{name}` must intercept a static or dynamic segment")),
        };
        return Ok(Some(SegmentKind::Intercept { level, inner: Box::new(inner) }));
    }

    if let Some(inner) = name.strip_prefix('(').and_then(|n| n.strip_suffix(')')) {
        if inner.is_empty() || inner.contains(['(', ')']) {
            return Err(format!("invalid route group name `{name}`; use `(name)`"));
        }
        return Ok(Some(SegmentKind::Group(inner.to_owned())));
    }

    if let Some(inner) = name.strip_prefix('@') {
        if !is_ident(inner) {
            return Err(format!("invalid slot name `{name}`; slot names must be identifiers like `@analytics`"));
        }
        return Ok(Some(SegmentKind::Slot(inner.to_owned())));
    }

    if let Some(inner) = name.strip_prefix("[[").and_then(|n| n.strip_suffix("]]")) {
        let Some(param) = inner.strip_prefix("...") else {
            return Err(format!(
                "`{name}`: double brackets are only valid for optional catch-all segments like `[[...slug]]`"
            ));
        };
        check_param(name, param)?;
        return Ok(Some(SegmentKind::OptionalCatchAll(param.to_owned())));
    }

    if let Some(inner) = name.strip_prefix('[').and_then(|n| n.strip_suffix(']')) {
        if let Some(param) = inner.strip_prefix("...") {
            check_param(name, param)?;
            return Ok(Some(SegmentKind::CatchAll(param.to_owned())));
        }
        check_param(name, inner)?;
        return Ok(Some(SegmentKind::Dynamic(inner.to_owned())));
    }

    if name.contains(['[', ']']) {
        return Err(format!(
            "`{name}` mixes static text with brackets; a dynamic segment must be the whole directory name, e.g. `[id]`"
        ));
    }
    if name.starts_with('(') && name.ends_with(')') {
        return Err(format!("invalid route group name `{name}`"));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(format!("`{name}` contains a path separator"));
    }
    Ok(Some(SegmentKind::Static(name.to_owned())))
}

fn check_param(name: &str, param: &str) -> Result<(), String> {
    if param.is_empty() {
        return Err(format!("`{name}` has an empty parameter name"));
    }
    if !is_ident(param) {
        return Err(format!(
            "`{name}`: parameter `{param}` must be a valid identifier (letters, digits and `_`, not starting with a digit)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_kinds() {
        assert_eq!(parse_segment("about").unwrap(), Some(SegmentKind::Static("about".into())));
        assert_eq!(parse_segment("hello world").unwrap(), Some(SegmentKind::Static("hello world".into())));
        assert_eq!(parse_segment("héllo-日本").unwrap(), Some(SegmentKind::Static("héllo-日本".into())));
        assert_eq!(parse_segment("[id]").unwrap(), Some(SegmentKind::Dynamic("id".into())));
        assert_eq!(parse_segment("[...slug]").unwrap(), Some(SegmentKind::CatchAll("slug".into())));
        assert_eq!(parse_segment("[[...slug]]").unwrap(), Some(SegmentKind::OptionalCatchAll("slug".into())));
        assert_eq!(parse_segment("(marketing)").unwrap(), Some(SegmentKind::Group("marketing".into())));
        assert_eq!(parse_segment("@team").unwrap(), Some(SegmentKind::Slot("team".into())));
        assert_eq!(parse_segment("_components").unwrap(), None);
        assert_eq!(parse_segment(".git").unwrap(), None);
        assert_eq!(
            parse_segment("(..)(..)photo").unwrap(),
            Some(SegmentKind::Intercept {
                level: InterceptLevel::Up(2),
                inner: Box::new(SegmentKind::Static("photo".into()))
            })
        );
        assert_eq!(
            parse_segment("(.)[id]").unwrap(),
            Some(SegmentKind::Intercept {
                level: InterceptLevel::Same,
                inner: Box::new(SegmentKind::Dynamic("id".into()))
            })
        );
        assert!(matches!(
            parse_segment("(...)x").unwrap(),
            Some(SegmentKind::Intercept { level: InterceptLevel::Root, .. })
        ));
    }

    #[test]
    fn rejects_invalid() {
        for bad in
            ["[]", "[...]", "[[slug]]", "[a-b]", "[1a]", "user[id]", "()", "@", "@a-b", "(.)", "(.)(group)", "[[...a]"]
        {
            assert!(parse_segment(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn pattern_strings() {
        let p = RoutePattern(vec![
            PatternSegment::Static("docs".into()),
            PatternSegment::Dynamic("v".into()),
            PatternSegment::OptionalCatchAll("rest".into()),
        ]);
        assert_eq!(p.to_fs_string(), "/docs/[v]/[[...rest]]");
        assert_eq!(p.to_display_string(), "/docs/:v/*rest?");
        assert_eq!(p.shape(), "/docs/[]/[[...]]");
        let params = crate::Params::new().with("v", "1 2").with("rest", ["a", "b"]);
        assert_eq!(p.to_path(&params).unwrap(), "/docs/1%202/a/b");
        assert_eq!(RoutePattern::default().to_fs_string(), "/");
    }
}
