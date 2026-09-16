use std::path::{Path, PathBuf};

use next_rust_core::config::HtmlPrecedence;
use next_rust_router::manifest::RouteManifest;
use next_rust_router::rank::precedence;
use next_rust_router::*;

struct TempApp {
    root: PathBuf,
}

impl TempApp {
    fn new(files: &[&str]) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "nr-routing-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        for f in files {
            let p = root.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "").unwrap();
        }
        Self { root }
    }

    fn scan(&self) -> ScanOutput {
        scan(&self.root, &ScanOptions::default())
    }

    fn scan_with(&self, opts: ScanOptions) -> ScanOutput {
        scan(&self.root, &opts)
    }

    fn rel(&self, p: &Path) -> String {
        manifest::relative(p, &self.root)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn patterns(out: &ScanOutput) -> Vec<String> {
    out.routes
        .iter()
        .filter(|r| r.intercept.is_none())
        .map(|r| format!("{} {}", r.kind.as_str(), r.pattern.to_display_string()))
        .collect()
}

fn codes(out: &ScanOutput) -> Vec<String> {
    out.diagnostics.iter().map(|d| d.code.clone()).collect()
}

fn matcher(out: &ScanOutput) -> Matcher<String> {
    let mut m = Matcher::new();
    for r in out.routes.iter().filter(|r| r.intercept.is_none()) {
        m.insert(r.pattern.segments(), r.pattern.to_fs_string()).unwrap();
    }
    m
}

#[test]
fn maps_files_to_urls() {
    let app = TempApp::new(&[
        "layout.rs",
        "page.rs",
        "about/page.rs",
        "blog/layout.rs",
        "blog/page.rs",
        "blog/[slug]/page.rs",
        "users/[id]/page.rs",
        "docs/[...slug]/page.rs",
        "shop/[[...filters]]/page.rs",
        "(marketing)/pricing/page.rs",
        "(marketing)/contact/page.rs",
        "api/users/route.rs",
        "legacy/page.html",
        "_components/button.rs",
        "_components/page.rs",
        ".hidden/page.rs",
        "about/helper.rs",
    ]);
    let out = app.scan();
    assert!(!out.has_errors(), "{}", out.diagnostics);
    assert_eq!(
        patterns(&out),
        vec![
            "page /",
            "page /about",
            "api /api/users",
            "page /blog",
            "page /blog/:slug",
            "page /contact",
            "page /docs/*slug",
            "html /legacy",
            "page /pricing",
            "page /shop/*filters?",
            "page /users/:id",
        ]
    );
}

#[test]
fn nested_layout_chain_includes_groups() {
    let app = TempApp::new(&[
        "layout.rs",
        "(dashboard)/layout.rs",
        "(dashboard)/dashboard/layout.rs",
        "(dashboard)/dashboard/settings/layout.rs",
        "(dashboard)/dashboard/settings/page.rs",
        "(dashboard)/dashboard/settings/loading.rs",
        "(dashboard)/dashboard/error.rs",
        "not-found.rs",
        "middleware.rs",
        "(dashboard)/dashboard/middleware.rs",
    ]);
    let out = app.scan();
    assert!(!out.has_errors(), "{}", out.diagnostics);
    let route = &out.routes[0];
    assert_eq!(route.pattern.to_display_string(), "/dashboard/settings");
    let layouts: Vec<String> = route.layouts().map(|p| app.rel(p)).collect();
    assert_eq!(
        layouts,
        vec![
            "layout.rs",
            "(dashboard)/layout.rs",
            "(dashboard)/dashboard/layout.rs",
            "(dashboard)/dashboard/settings/layout.rs"
        ]
    );
    let mw: Vec<String> = route.middleware().map(|p| app.rel(p)).collect();
    assert_eq!(mw, vec!["middleware.rs", "(dashboard)/dashboard/middleware.rs"]);
    assert_eq!(route.chain.len(), 4);
    assert!(route.chain[3].loading.is_some());
    assert!(route.chain[2].error.is_some());
    assert!(route.chain[0].not_found.is_some());
}

#[test]
fn deep_nesting_has_no_limit() {
    let mut files = Vec::new();
    let mut dir = String::new();
    for i in 0..40 {
        dir.push_str(&format!("s{i}/"));
        files.push(format!("{dir}layout.rs"));
    }
    files.push(format!("{dir}page.rs"));
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let app = TempApp::new(&refs);
    let out = app.scan();
    assert_eq!(out.routes[0].layouts().count(), 40);
}

#[test]
fn unicode_and_spaces() {
    let app = TempApp::new(&["hello world/page.rs", "日本語/[id]/page.rs", "café/page.rs"]);
    let out = app.scan();
    assert!(!out.has_errors());
    let m = matcher(&out);
    assert_eq!(m.at("/hello%20world").unwrap().value, "/hello world");
    assert_eq!(m.at("/caf%C3%A9").unwrap().value, "/café");
    let hit = m.at("/%E6%97%A5%E6%9C%AC%E8%AA%9E/42").unwrap();
    assert_eq!(hit.value, "/日本語/[id]");
    assert_eq!(hit.params.get("id"), Some("42"));
    // Paths built from patterns are encoded.
    let r = out.routes.iter().find(|r| r.pattern.to_fs_string() == "/hello world").unwrap();
    assert_eq!(r.pattern.to_path(&Params::new()).unwrap(), "/hello%20world");
}

#[test]
fn page_and_html_conflict() {
    let app = TempApp::new(&["legacy/page.rs", "legacy/page.html"]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0101"]);
    assert!(out.has_errors());
    let rendered = out.diagnostics.render(false);
    assert!(rendered.contains("page.rs") && rendered.contains("page.html"));

    let out = app.scan_with(ScanOptions { html_precedence: HtmlPrecedence::Html, ..Default::default() });
    assert!(!out.has_errors());
    assert_eq!(patterns(&out), vec!["html /legacy"]);
    let out = app.scan_with(ScanOptions { html_precedence: HtmlPrecedence::Rs, ..Default::default() });
    assert_eq!(patterns(&out), vec!["page /legacy"]);
}

#[test]
fn duplicate_routes_through_groups() {
    let app = TempApp::new(&["(a)/about/page.rs", "(b)/about/page.rs"]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0102"]);
    let d = &out.diagnostics.0[0];
    assert_eq!(d.locations.len(), 2);
    assert!(d.message.contains("/about"));
}

#[test]
fn page_and_api_conflict() {
    let app = TempApp::new(&["users/page.rs", "users/route.rs"]);
    assert_eq!(codes(&app.scan()), vec!["NR0104"]);
}

#[test]
fn conflicting_param_names() {
    let app = TempApp::new(&["users/[id]/page.rs", "users/[userId]/edit/page.rs"]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0103"]);
    assert!(out.diagnostics.0[0].message.contains("`id`"));
    assert!(out.diagnostics.0[0].message.contains("`userId`"));
}

#[test]
fn same_param_name_different_depth_is_fine() {
    let app = TempApp::new(&["users/[id]/page.rs", "users/[id]/edit/page.rs", "posts/[id]/page.rs"]);
    assert!(codes(&app.scan()).is_empty());
}

#[test]
fn catch_all_must_be_last() {
    let app = TempApp::new(&["docs/[...slug]/page.rs", "docs/[...slug]/edit/page.rs"]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0105"]);
}

#[test]
fn optional_catch_all_conflicts_with_parent() {
    let app = TempApp::new(&["docs/page.rs", "docs/[[...slug]]/page.rs"]);
    assert_eq!(codes(&app.scan()), vec!["NR0106"]);
    let app = TempApp::new(&["docs/[...a]/page.rs", "docs/[[...a]]/page.rs"]);
    assert_eq!(codes(&app.scan()), vec!["NR0116"]);
}

#[test]
fn duplicate_param_in_one_route() {
    let app = TempApp::new(&["[id]/x/[id]/page.rs"]);
    assert_eq!(codes(&app.scan()), vec!["NR0107"]);
}

#[test]
fn invalid_names_and_misplaced_files() {
    let app = TempApp::new(&[
        "page.rs",
        "user[id]/page.rs",
        "[a-b]/page.rs",
        "blog/global-error.rs",
        "Layout.rs",
        "about/page.tsx",
    ]);
    let out = app.scan();
    let mut c = codes(&out);
    c.sort();
    assert_eq!(c, vec!["NR0108", "NR0108", "NR0111", "NR0120", "NR0121"]);
}

#[cfg(unix)]
#[test]
fn symlink_loops_are_detected() {
    let app = TempApp::new(&["page.rs", "a/page.rs"]);
    std::os::unix::fs::symlink(app.root.join("a"), app.root.join("a/loop")).unwrap();
    std::os::unix::fs::symlink(app.root.join("a"), app.root.join("alias")).unwrap();
    let out = app.scan();
    assert!(codes(&out).contains(&"NR0110".to_string()));
    // Non-loop symlink is followed.
    assert!(patterns(&out).contains(&"page /alias".to_string()));
}

#[test]
fn empty_app_warns() {
    let app = TempApp::new(&[]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0118"]);
    assert!(!out.has_errors());
}

#[test]
fn missing_directory_is_an_error() {
    let out = scan(Path::new("/definitely/not/here/nr"), &ScanOptions::default());
    assert_eq!(codes(&out), vec!["NR0001"]);
}

#[test]
fn parallel_slots_resolve_per_route() {
    let app = TempApp::new(&[
        "dashboard/layout.rs",
        "dashboard/page.rs",
        "dashboard/settings/page.rs",
        "dashboard/@analytics/page.rs",
        "dashboard/@analytics/default.rs",
        "dashboard/@team/settings/page.rs",
        "dashboard/@team/default.rs",
        "dashboard/@empty/.keep",
    ]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0117"]);
    // Slots never create URLs.
    assert_eq!(patterns(&out), vec!["page /dashboard", "page /dashboard/settings"]);
    let dash = &out.routes[0];
    let slots = &dash.chain[1].slots;
    let names: Vec<&str> = slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["analytics", "empty", "team"]);
    assert_eq!(app.rel(slots[0].page.as_ref().unwrap()), "dashboard/@analytics/page.rs");
    assert!(slots[2].page.is_none());
    assert!(slots[2].default.is_some());

    let settings = &out.routes[1];
    let slots = &settings.chain[1].slots;
    assert!(slots[0].page.is_none(), "analytics has no settings page");
    assert_eq!(app.rel(slots[2].page.as_ref().unwrap()), "dashboard/@team/settings/page.rs");
}

#[test]
fn intercepting_routes() {
    let app = TempApp::new(&[
        "feed/page.rs",
        "feed/(.)photo/[id]/page.rs",
        "photo/[id]/page.rs",
        "shop/items/(..)(..)cart/page.rs",
        "cart/page.rs",
        "a/(...)login/page.rs",
        "login/page.rs",
        "x/(..)(..)(..)y/page.rs",
    ]);
    let out = app.scan();
    assert_eq!(codes(&out), vec!["NR0115"]);
    let intercepts: Vec<String> = out
        .routes
        .iter()
        .filter_map(|r| {
            r.intercept.as_ref().map(|i| format!("{} from {}", r.pattern.to_fs_string(), i.context.to_fs_string()))
        })
        .collect();
    assert_eq!(intercepts, vec!["/cart from /shop/items", "/feed/photo/[id] from /feed", "/login from /a"]);
    // Intercepts do not count as duplicates of the real routes.
    assert!(!out.has_errors() || codes(&out) == vec!["NR0115"]);
}

#[test]
fn route_precedence_static_dynamic_catch_all() {
    let app = TempApp::new(&[
        "users/settings/page.rs",
        "users/[id]/page.rs",
        "users/[id]/posts/page.rs",
        "users/[...path]/page.rs",
        "[[...all]]/page.rs",
    ]);
    let out = app.scan();
    assert!(!out.has_errors(), "{}", out.diagnostics);
    let m = matcher(&out);
    assert_eq!(m.at("/users/settings").unwrap().value, "/users/settings");
    assert_eq!(m.at("/users/42").unwrap().value, "/users/[id]");
    assert_eq!(m.at("/users/42/posts").unwrap().value, "/users/[id]/posts");
    let hit = m.at("/users/42/other/deep").unwrap();
    assert_eq!(hit.value, "/users/[...path]");
    assert_eq!(hit.params.get_all("path").unwrap(), ["42", "other", "deep"]);
    // Backtracking: "settings" static branch has no "posts" child.
    assert_eq!(m.at("/users/settings/posts").unwrap().value, "/users/[id]/posts");
    let root = m.at("/").unwrap();
    assert_eq!(root.value, "/[[...all]]");
    assert_eq!(root.params.get_all("all").unwrap().len(), 0);
    assert_eq!(m.at("/x/y").unwrap().params.get_all("all").unwrap(), ["x", "y"]);
    // Listing order follows precedence.
    let listed: Vec<String> = out.routes.iter().map(|r| r.pattern.to_fs_string()).collect();
    assert_eq!(listed, vec!["/users/settings", "/users/[id]", "/users/[id]/posts", "/users/[...path]", "/[[...all]]"]);
}

#[test]
fn matcher_edge_cases() {
    let mut m = Matcher::new();
    m.insert(&[], "root").unwrap();
    m.insert(&[PatternSegment::Static("docs".into()), PatternSegment::CatchAll("slug".into())], "docs").unwrap();
    m.insert(&[PatternSegment::Static("a".into())], "a").unwrap();
    assert_eq!(*m.at("/").unwrap().value, "root");
    assert_eq!(*m.at("").unwrap().value, "root");
    assert_eq!(*m.at("/a/").unwrap().value, "a");
    assert!(m.at("/a//").is_none());
    assert!(m.at("/docs").is_none(), "required catch-all needs a segment");
    assert_eq!(m.at("/docs/a%2Fb").unwrap().params.get_all("slug").unwrap(), ["a/b"]);
    assert!(m.at("/docs/%FF").is_none(), "invalid utf-8");
    assert!(m.at("/docs/%zz").is_none(), "invalid escape");
    assert_eq!(m.insert(&[PatternSegment::Static("a".into())], "dup"), Err(InsertError::Duplicate));
    assert_eq!(
        m.insert(&[PatternSegment::CatchAll("x".into()), PatternSegment::Static("y".into())], "bad"),
        Err(InsertError::CatchAllNotLast)
    );
    m.insert(&[PatternSegment::Dynamic("id".into())], "id").unwrap();
    assert!(matches!(
        m.insert(&[PatternSegment::Dynamic("slug".into()), PatternSegment::Static("z".into())], "z"),
        Err(InsertError::ParamNameConflict { .. })
    ));
    assert_eq!(m.len(), 4);
}

/// Brute force: among all patterns matching `path`, pick the greatest rank.
fn brute_force<'a>(patterns: &'a [RoutePattern], path: &[&str]) -> Option<&'a RoutePattern> {
    fn matches(p: &[PatternSegment], path: &[&str]) -> bool {
        match (p.split_first(), path.split_first()) {
            (None, None) => true,
            (Some((PatternSegment::OptionalCatchAll(_), _)), _) => true,
            (Some((PatternSegment::CatchAll(_), _)), Some(_)) => true,
            (Some((PatternSegment::Static(s), rest)), Some((seg, prest))) => s == seg && matches(rest, prest),
            (Some((PatternSegment::Dynamic(_), rest)), Some((_, prest))) => matches(rest, prest),
            _ => false,
        }
    }
    patterns.iter().filter(|p| matches(p.segments(), path)).max_by(|a, b| precedence(a, b))
}

#[test]
fn matcher_agrees_with_formal_ranking() {
    // Deterministic LCG so the test needs no dependencies and is reproducible.
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut rnd = |n: u64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    let words = ["a", "b", "c"];
    for _ in 0..300 {
        let mut m = Matcher::new();
        let mut pats: Vec<RoutePattern> = Vec::new();
        for _ in 0..rnd(12) + 1 {
            let len = rnd(4) as usize;
            let mut segs = Vec::new();
            for i in 0..len {
                let seg = match rnd(8) {
                    0..=3 => PatternSegment::Static(words[rnd(3) as usize].into()),
                    4..=5 => PatternSegment::Dynamic(format!("p{i}")),
                    6 => PatternSegment::CatchAll(format!("p{i}")),
                    _ => PatternSegment::OptionalCatchAll(format!("p{i}")),
                };
                let stop = seg.is_catch_all();
                segs.push(seg);
                if stop {
                    break;
                }
            }
            let pat = RoutePattern(segs);
            // Keep only sets the validator would accept: no duplicate shapes,
            // no optional catch-all overlapping its parent or a sibling catch-all.
            let shape = pat.shape();
            let conflicts = pats.iter().any(|p| {
                let s = p.shape();
                s == shape
                    || (shape.ends_with("/[[...]]")
                        && (s == shape.trim_end_matches("/[[...]]").to_string() + "/[...]"
                            || s == norm(shape.trim_end_matches("/[[...]]"))))
                    || (s.ends_with("/[[...]]")
                        && (shape == s.trim_end_matches("/[[...]]").to_string() + "/[...]"
                            || shape == norm(s.trim_end_matches("/[[...]]"))))
            });
            if !conflicts && m.insert(pat.segments(), pat.clone()).is_ok() {
                pats.push(pat);
            }
        }
        for _ in 0..30 {
            let path: Vec<&str> = (0..rnd(5)).map(|_| words[rnd(3) as usize]).collect();
            let expected = brute_force(&pats, &path);
            let got = m.at_segments(&path).map(|h| h.value);
            assert_eq!(
                got.map(|p| p.shape()),
                expected.map(|p| p.shape()),
                "path {path:?} patterns {:?}",
                pats.iter().map(|p| p.to_fs_string()).collect::<Vec<_>>()
            );
        }
    }

    fn norm(s: &str) -> String {
        if s.is_empty() { "/".into() } else { s.into() }
    }
}

#[test]
fn manifest_roundtrip() {
    let app = TempApp::new(&["layout.rs", "page.rs", "blog/[slug]/page.rs", "api/x/route.rs"]);
    let out = app.scan();
    let manifest = RouteManifest::new(&out.routes, &app.root);
    let json = manifest.to_json();
    let back = RouteManifest::from_json(&json).unwrap();
    assert_eq!(back, manifest);
    let blog = back.routes.iter().find(|r| r.pattern == "/blog/[slug]").unwrap();
    assert_eq!(blog.path, "/blog/:slug");
    assert_eq!(blog.source, "blog/[slug]/page.rs");
    assert_eq!(blog.layouts, vec!["layout.rs"]);
}

#[test]
fn project_scan_with_api_directory_and_custom_app_dir() {
    let app = TempApp::new(&["website/routes/page.rs", "backend/users/route.rs", "backend/users/page.rs"]);
    let config = next_rust_core::Config::from_toml_str(
        "[app]\ndirectory = \"website/routes\"\n[api]\ndirectory = \"backend\"\nprefix = \"/v1\"",
    )
    .map(|mut c| {
        c.root = app.root.clone();
        c
    })
    .unwrap();
    let out = scan_project(&config);
    assert_eq!(patterns(&out), vec!["page /", "api /v1/users"]);
    assert_eq!(codes(&out), vec!["NR0112"]);
    let tree = out.tree().unwrap().render_tree(Path::new("app"));
    assert!(tree.starts_with("app  [page.rs]"));
}
