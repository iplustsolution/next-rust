//! Route-set validation.

use std::collections::{BTreeMap, HashSet};

use next_rust_core::Diagnostic;

use crate::flatten::{Route, RouteKind};
use crate::segment::{PatternSegment, SegmentKind};
use crate::tree::RouteNode;

/// Validate a flattened route set. Returns diagnostics in a deterministic order.
pub fn validate(trees: &[RouteNode], routes: &[Route]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let normal: Vec<&Route> = routes.iter().filter(|r| r.intercept.is_none()).collect();

    // Per-pattern checks.
    for r in routes {
        let segs = r.pattern.segments();
        if let Some(pos) = segs.iter().position(PatternSegment::is_catch_all)
            && pos + 1 != segs.len()
        {
            out.push(
                    Diagnostic::error("NR0105", "Catch-all segment must be the last segment")
                        .location(&r.source)
                        .message(format!(
                            "Route `{}` continues after the catch-all `{}`.\nA catch-all consumes every remaining URL segment, so nothing can follow it.",
                            r.pattern.to_fs_string(),
                            segs[pos].param_name().unwrap_or_default()
                        ))
                        .help("move the nested files next to the catch-all, or use a dynamic segment `[name]` instead"),
                );
        }
        let mut seen = HashSet::new();
        for name in r.pattern.param_names() {
            if !seen.insert(name) {
                out.push(
                    Diagnostic::error("NR0107", format!("Duplicate route parameter `{name}`"))
                        .location(&r.source)
                        .message(format!("Route `{}` uses `{name}` more than once.", r.pattern.to_fs_string()))
                        .help("give every dynamic segment in a route a unique name"),
                );
            }
        }
    }

    // Same URL shape defined more than once.
    let mut by_shape: BTreeMap<String, Vec<&Route>> = BTreeMap::new();
    for r in &normal {
        by_shape.entry(r.pattern.shape()).or_default().push(r);
    }
    for (shape, rs) in &by_shape {
        if rs.len() < 2 {
            continue;
        }
        let pages: Vec<&&Route> = rs.iter().filter(|r| r.is_page()).collect();
        let apis: Vec<&&Route> = rs.iter().filter(|r| r.kind == RouteKind::Api).collect();
        if !pages.is_empty() && !apis.is_empty() {
            let mut d = Diagnostic::error("NR0104", "Page and API route resolve to the same URL")
                .message(format!(
                    "Both a page and a `route.rs` handler resolve to `{}`.\nA URL is either rendered as a page or handled by an API route, never both.",
                    display_shape(shape, rs)
                ))
                .help("move the route.rs into a separate directory such as `api/`");
            for r in rs {
                d = d.location(&r.source);
            }
            out.push(d);
        }
        for group in [pages, apis] {
            if group.len() < 2 {
                continue;
            }
            // page.rs + page.html in the same directory is already NR0101.
            let dirs: HashSet<_> = group.iter().map(|r| r.source.parent()).collect();
            if dirs.len() < 2 {
                continue;
            }
            let mut d = Diagnostic::error("NR0102", "Duplicate route")
                .message(format!(
                    "{} files resolve to the URL `{}`.\nRoute groups like `(marketing)` do not appear in URLs, so\ndirectories in different groups can collide.",
                    group.len(),
                    group[0].pattern.to_display_string()
                ))
                .help("rename or remove one of the routes");
            for r in &group {
                d = d.location(&r.source);
            }
            out.push(d);
        }
    }

    // Different parameter names at the same dynamic position.
    let mut positions: BTreeMap<(String, String), BTreeMap<String, Vec<&Route>>> = BTreeMap::new();
    for r in &normal {
        let segs = r.pattern.segments();
        for (i, seg) in segs.iter().enumerate() {
            if let Some(name) = seg.param_name() {
                let prefix: Vec<String> = segs[..i].iter().map(PatternSegment::shape).collect();
                positions
                    .entry((prefix.join("/"), seg.shape()))
                    .or_default()
                    .entry(name.to_owned())
                    .or_default()
                    .push(r);
            }
        }
    }
    for ((prefix, shape), names) in &positions {
        if names.len() < 2 {
            continue;
        }
        let list: Vec<String> = names.keys().map(|n| format!("`{n}`")).collect();
        let mut d = Diagnostic::error("NR0103", "Conflicting dynamic segment names")
            .message(format!(
                "The dynamic segment at `/{}{}{}` is named differently in different routes: {}.\nRoutes sharing a URL position must use the same parameter name.",
                prefix,
                if prefix.is_empty() { "" } else { "/" },
                shape,
                list.join(", ")
            ))
            .help("rename the directories so they use a single name");
        for rs in names.values() {
            if let Some(r) = rs.first() {
                d = d.location(&r.source);
            }
        }
        out.push(d);
    }

    // Optional catch-all overlapping its parent / a sibling catch-all.
    let shapes: HashSet<&str> = by_shape.keys().map(String::as_str).collect();
    for r in &normal {
        if let Some(PatternSegment::OptionalCatchAll(_)) = r.pattern.segments().last() {
            let parent_shape = {
                let segs: Vec<String> =
                    r.pattern.segments()[..r.pattern.segments().len() - 1].iter().map(PatternSegment::shape).collect();
                format!("/{}", segs.join("/"))
            };
            if let Some(parent) = by_shape.get(&parent_shape) {
                let mut d = Diagnostic::error("NR0106", "Optional catch-all conflicts with a route at its parent path")
                    .message(format!(
                        "`{}` also matches `{}`, which is already defined.",
                        r.pattern.to_fs_string(),
                        parent[0].pattern.to_display_string()
                    ))
                    .help("use a required catch-all `[...name]`, or remove the parent page")
                    .location(&r.source);
                for p in parent {
                    d = d.location(&p.source);
                }
                out.push(d);
            }
            let sibling = parent_shape.trim_end_matches('/').to_owned() + "/[...]";
            if shapes.contains(sibling.as_str()) {
                out.push(
                    Diagnostic::error("NR0116", "Catch-all and optional catch-all at the same position")
                        .location(&r.source)
                        .message(format!(
                            "Both `[...]` and `[[...]]` exist under `{}`; they match the same URLs.",
                            parent_shape
                        ))
                        .help("keep only one of them"),
                );
            }
        }
    }

    // Slots.
    for tree in trees {
        check_slots(tree, &mut out);
    }

    if routes.is_empty() && !trees.is_empty() {
        out.push(
            Diagnostic::warning("NR0118", "No routes found")
                .location(&trees[0].dir)
                .help("create a `page.rs` (or `route.rs`) in the app directory"),
        );
    }
    out
}

fn display_shape(shape: &str, rs: &[&Route]) -> String {
    rs.first().map(|r| r.pattern.to_display_string()).unwrap_or_else(|| shape.to_owned())
}

fn check_slots(node: &RouteNode, out: &mut Vec<Diagnostic>) {
    if let SegmentKind::Slot(name) = &node.kind
        && node.files.page.is_none()
        && node.files.default.is_none()
        && node.children.is_empty()
    {
        out.push(
            Diagnostic::warning("NR0117", format!("Slot `@{name}` has no page.rs or default.rs"))
                .location(&node.dir)
                .help("add a `default.rs` to render when no slot page matches"),
        );
    }
    for c in &node.children {
        check_slots(c, out);
    }
}
