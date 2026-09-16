//! Tree → flat route list.

use std::path::PathBuf;

use next_rust_core::{Diagnostic, Diagnostics};
use serde::Serialize;

use crate::segment::{InterceptLevel, PatternSegment, RoutePattern, SegmentKind};
use crate::tree::RouteNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteKind {
    /// `page.rs`
    Page,
    /// `page.html`
    Html,
    /// `route.rs`
    Api,
}

impl RouteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RouteKind::Page => "page",
            RouteKind::Html => "html",
            RouteKind::Api => "api",
        }
    }

    pub(crate) fn order(self) -> u8 {
        match self {
            RouteKind::Page => 0,
            RouteKind::Html => 1,
            RouteKind::Api => 2,
        }
    }
}

/// Resolution of a parallel slot for one particular route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SlotEntry {
    pub name: String,
    /// The slot's `page.rs` matching the current route, if any.
    pub page: Option<PathBuf>,
    /// The slot's `default.rs`, used when `page` is `None`.
    pub default: Option<PathBuf>,
}

/// One directory on the path from the routing root to a route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SegmentEntry {
    pub dir: PathBuf,
    /// Directory name (empty for the root).
    pub name: String,
    pub layout: Option<PathBuf>,
    pub template: Option<PathBuf>,
    pub loading: Option<PathBuf>,
    pub error: Option<PathBuf>,
    pub not_found: Option<PathBuf>,
    pub middleware: Option<PathBuf>,
    pub metadata: Option<PathBuf>,
    pub global_error: Option<PathBuf>,
    pub slots: Vec<SlotEntry>,
}

/// Where an intercepting route applies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct InterceptTarget {
    /// Soft navigations originating from URLs under this pattern are intercepted.
    pub context: RoutePattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Route {
    pub kind: RouteKind,
    pub pattern: RoutePattern,
    /// `page.rs`, `page.html` or `route.rs`.
    pub source: PathBuf,
    /// Root → leaf directories (route groups included, slots excluded).
    pub chain: Vec<SegmentEntry>,
    pub intercept: Option<InterceptTarget>,
}

impl Route {
    /// Stable identifier, e.g. `page:/blog/[slug]`.
    pub fn id(&self) -> String {
        match &self.intercept {
            None => format!("{}:{}", self.kind.as_str(), self.pattern.to_fs_string()),
            Some(i) => {
                format!("{}:{}@intercept:{}", self.kind.as_str(), self.pattern.to_fs_string(), i.context.to_fs_string())
            }
        }
    }

    pub fn layouts(&self) -> impl Iterator<Item = &PathBuf> {
        self.chain.iter().filter_map(|s| s.layout.as_ref())
    }

    pub fn middleware(&self) -> impl Iterator<Item = &PathBuf> {
        self.chain.iter().filter_map(|s| s.middleware.as_ref())
    }

    pub fn is_page(&self) -> bool {
        matches!(self.kind, RouteKind::Page | RouteKind::Html)
    }
}

/// Flatten a scanned tree into routes. `mount` is prepended to every pattern.
pub fn flatten(root: &RouteNode, mount: &[PatternSegment], diags: &mut Diagnostics) -> Vec<Route> {
    let mut routes = Vec::new();
    let mut stack: Vec<&RouteNode> = Vec::new();
    walk(root, RoutePattern(mount.to_vec()), None, &mut stack, &mut routes, diags);
    routes
}

fn walk<'a>(
    node: &'a RouteNode,
    pattern: RoutePattern,
    intercept: Option<InterceptTarget>,
    stack: &mut Vec<&'a RouteNode>,
    routes: &mut Vec<Route>,
    diags: &mut Diagnostics,
) {
    stack.push(node);

    let make = |kind: RouteKind, source: &PathBuf, stack: &[&RouteNode]| Route {
        kind,
        pattern: pattern.clone(),
        source: source.clone(),
        chain: build_chain(stack),
        intercept: intercept.clone(),
    };
    if let Some(page) = &node.files.page {
        routes.push(make(RouteKind::Page, page, stack));
    } else if let Some(html) = &node.files.page_html {
        routes.push(make(RouteKind::Html, html, stack));
    }
    if let Some(route) = &node.files.route {
        if intercept.is_some() {
            diags.push(
                Diagnostic::warning("NR0119", "route.rs inside an intercepting route is ignored")
                    .location(route)
                    .message("Interception only applies to pages rendered during client navigation."),
            );
        } else {
            routes.push(make(RouteKind::Api, route, stack));
        }
    }

    for child in &node.children {
        match &child.kind {
            SegmentKind::Root | SegmentKind::Slot(_) => {}
            SegmentKind::Group(_) => walk(child, pattern.clone(), intercept.clone(), stack, routes, diags),
            SegmentKind::Static(s) => {
                walk(child, push(&pattern, PatternSegment::Static(s.clone())), intercept.clone(), stack, routes, diags)
            }
            SegmentKind::Dynamic(s) => {
                walk(child, push(&pattern, PatternSegment::Dynamic(s.clone())), intercept.clone(), stack, routes, diags)
            }
            SegmentKind::CatchAll(s) => walk(
                child,
                push(&pattern, PatternSegment::CatchAll(s.clone())),
                intercept.clone(),
                stack,
                routes,
                diags,
            ),
            SegmentKind::OptionalCatchAll(s) => walk(
                child,
                push(&pattern, PatternSegment::OptionalCatchAll(s.clone())),
                intercept.clone(),
                stack,
                routes,
                diags,
            ),
            SegmentKind::Intercept { level, inner } => {
                if intercept.is_some() {
                    diags.push(
                        Diagnostic::error("NR0115", "Nested intercepting routes are not supported")
                            .location(&child.dir),
                    );
                    continue;
                }
                let context = pattern.clone();
                let base: Vec<PatternSegment> = match level {
                    InterceptLevel::Same => context.0.clone(),
                    InterceptLevel::Root => Vec::new(),
                    InterceptLevel::Up(n) => {
                        if *n > context.0.len() {
                            diags.push(
                                Diagnostic::error("NR0115", "Intercepting route reaches above the app root")
                                    .location(&child.dir)
                                    .message(format!(
                                        "`{}` goes up {n} segment(s) but `{}` only has {}.",
                                        child.name,
                                        context.to_fs_string(),
                                        context.0.len()
                                    ))
                                    .help("use `(...)` to intercept from the root"),
                            );
                            continue;
                        }
                        context.0[..context.0.len() - n].to_vec()
                    }
                };
                let seg = match inner.as_ref() {
                    SegmentKind::Static(s) => PatternSegment::Static(s.clone()),
                    SegmentKind::Dynamic(s) => PatternSegment::Dynamic(s.clone()),
                    SegmentKind::CatchAll(s) => PatternSegment::CatchAll(s.clone()),
                    SegmentKind::OptionalCatchAll(s) => PatternSegment::OptionalCatchAll(s.clone()),
                    _ => continue,
                };
                let mut target = base;
                target.push(seg);
                walk(child, RoutePattern(target), Some(InterceptTarget { context }), stack, routes, diags);
            }
        }
    }

    stack.pop();
}

fn push(pattern: &RoutePattern, seg: PatternSegment) -> RoutePattern {
    let mut p = pattern.clone();
    p.0.push(seg);
    p
}

fn build_chain(stack: &[&RouteNode]) -> Vec<SegmentEntry> {
    stack
        .iter()
        .enumerate()
        .map(|(i, node)| {
            // URL-relevant directory names below this node, used to resolve slots.
            let below: Vec<&str> = stack[i + 1..]
                .iter()
                .filter(|n| !matches!(n.kind, SegmentKind::Group(_)))
                .map(|n| n.name.as_str())
                .collect();
            SegmentEntry {
                dir: node.dir.clone(),
                name: node.name.clone(),
                layout: node.files.layout.clone(),
                template: node.files.template.clone(),
                loading: node.files.loading.clone(),
                error: node.files.error.clone(),
                not_found: node.files.not_found.clone(),
                middleware: node.files.middleware.clone(),
                metadata: node.files.metadata.clone(),
                global_error: node.files.global_error.clone(),
                slots: node
                    .slots()
                    .map(|(name, slot)| SlotEntry {
                        name: name.to_owned(),
                        page: descend(slot, &below).and_then(|n| n.files.page.clone()),
                        default: slot.files.default.clone(),
                    })
                    .collect(),
            }
        })
        .collect()
}

fn descend<'a>(node: &'a RouteNode, names: &[&str]) -> Option<&'a RouteNode> {
    let Some((first, rest)) = names.split_first() else { return Some(node) };
    for child in &node.children {
        match &child.kind {
            SegmentKind::Group(_) => {
                if let Some(found) = descend(child, names) {
                    return Some(found);
                }
            }
            _ if child.name == *first => return descend(child, rest),
            _ => {}
        }
    }
    None
}
