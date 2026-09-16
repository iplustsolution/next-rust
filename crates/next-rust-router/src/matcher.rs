//! Trie-based URL matcher.

use std::collections::HashMap;

use crate::params::{ParamValue, Params};
use crate::segment::PatternSegment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertError {
    /// A value is already registered for this exact pattern shape.
    Duplicate,
    /// Same position, different parameter name.
    ParamNameConflict { existing: String, new: String },
    /// A catch-all is followed by more segments.
    CatchAllNotLast,
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::Duplicate => f.write_str("a route with this pattern already exists"),
            InsertError::ParamNameConflict { existing, new } => {
                write!(f, "parameter `{new}` conflicts with `{existing}` at the same position")
            }
            InsertError::CatchAllNotLast => f.write_str("catch-all segment must be last"),
        }
    }
}

impl std::error::Error for InsertError {}

#[derive(Debug, Clone)]
struct Node<T> {
    value: Option<T>,
    statics: HashMap<String, Node<T>>,
    dynamic: Option<(String, Box<Node<T>>)>,
    catch_all: Option<(String, T)>,
    optional_catch_all: Option<(String, T)>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self { value: None, statics: HashMap::new(), dynamic: None, catch_all: None, optional_catch_all: None }
    }
}

/// Maps URL paths to values.
#[derive(Debug, Clone)]
pub struct Matcher<T> {
    root: Node<T>,
    len: usize,
}

impl<T> Default for Matcher<T> {
    fn default() -> Self {
        Self { root: Node::default(), len: 0 }
    }
}

/// Result of a successful match.
#[derive(Debug, Clone, PartialEq)]
pub struct Match<'a, T> {
    pub value: &'a T,
    pub params: Params,
}

impl<T> Matcher<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, pattern: &[PatternSegment], value: T) -> Result<(), InsertError> {
        let mut node = &mut self.root;
        for (i, seg) in pattern.iter().enumerate() {
            let last = i + 1 == pattern.len();
            match seg {
                PatternSegment::Static(s) => node = node.statics.entry(s.clone()).or_default(),
                PatternSegment::Dynamic(name) => {
                    let (existing, child) = node.dynamic.get_or_insert_with(|| (name.clone(), Box::default()));
                    if existing != name {
                        return Err(InsertError::ParamNameConflict { existing: existing.clone(), new: name.clone() });
                    }
                    node = child;
                }
                PatternSegment::CatchAll(name) | PatternSegment::OptionalCatchAll(name) => {
                    if !last {
                        return Err(InsertError::CatchAllNotLast);
                    }
                    let slot = if matches!(seg, PatternSegment::CatchAll(_)) {
                        &mut node.catch_all
                    } else {
                        &mut node.optional_catch_all
                    };
                    if slot.is_some() {
                        return Err(InsertError::Duplicate);
                    }
                    *slot = Some((name.clone(), value));
                    self.len += 1;
                    return Ok(());
                }
            }
        }
        if node.value.is_some() {
            return Err(InsertError::Duplicate);
        }
        node.value = Some(value);
        self.len += 1;
        Ok(())
    }

    /// Match a URL path such as `/blog/hello%20world`. Segments are
    /// percent-decoded; empty inner segments (`//`) never match. A single
    /// trailing slash is ignored.
    pub fn at(&self, path: &str) -> Option<Match<'_, T>> {
        let trimmed = path.strip_prefix('/').unwrap_or(path);
        let trimmed = trimmed.strip_suffix('/').unwrap_or(trimmed);
        let mut segs = Vec::new();
        if !trimmed.is_empty() {
            for raw in trimmed.split('/') {
                if raw.is_empty() {
                    return None;
                }
                segs.push(crate::decode_segment(raw)?);
            }
        }
        let refs: Vec<&str> = segs.iter().map(String::as_str).collect();
        self.at_segments(&refs)
    }

    /// Match pre-split, decoded segments.
    pub fn at_segments(&self, segs: &[&str]) -> Option<Match<'_, T>> {
        let mut params = Params::new();
        let value = find(&self.root, segs, &mut params)?;
        Some(Match { value, params })
    }
}

fn find<'a, T>(node: &'a Node<T>, segs: &[&str], params: &mut Params) -> Option<&'a T> {
    let Some((first, rest)) = segs.split_first() else {
        if let Some(v) = &node.value {
            return Some(v);
        }
        if let Some((name, v)) = &node.optional_catch_all {
            params.insert(name.clone(), ParamValue::Many(Vec::new()));
            return Some(v);
        }
        return None;
    };

    if let Some(child) = node.statics.get(*first)
        && let Some(v) = find(child, rest, params)
    {
        return Some(v);
    }
    if let Some((name, child)) = &node.dynamic {
        let before = params.len();
        params.insert(name.clone(), ParamValue::One((*first).to_owned()));
        if let Some(v) = find(child, rest, params) {
            return Some(v);
        }
        while params.len() > before {
            params.pop();
        }
    }
    let all = || ParamValue::Many(segs.iter().map(|s| (*s).to_owned()).collect());
    if let Some((name, v)) = &node.catch_all {
        params.insert(name.clone(), all());
        return Some(v);
    }
    if let Some((name, v)) = &node.optional_catch_all {
        params.insert(name.clone(), all());
        return Some(v);
    }
    None
}
