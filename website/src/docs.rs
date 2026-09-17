//! The documentation: navigation order, page content and search.
//!
//! Each page is a Rust module in `content/` that builds its article with the
//! view macros. Headings carry `id`s so they can be linked and listed in the
//! page outline.

use std::collections::HashMap;
use std::sync::OnceLock;

use next_rust::prelude::{Node, render_static};

/// One documentation page.
pub struct Doc {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Builds the article body.
    pub content: fn() -> Node,
    /// Source file, relative to the repository root.
    pub source: &'static str,
}

impl Doc {
    /// The article rendered to HTML. Rendered once, then reused for the page,
    /// its outline and search.
    pub fn html(&self) -> &'static str {
        static RENDERED: OnceLock<HashMap<&'static str, String>> = OnceLock::new();
        RENDERED
            .get_or_init(|| all().map(|d| (d.slug, render_static((d.content)()))).collect())
            .get(self.slug)
            .map(String::as_str)
            .unwrap_or("")
    }
}

/// A sidebar group.
pub struct Section {
    pub title: &'static str,
    pub docs: &'static [Doc],
}

macro_rules! doc {
    ($slug:literal, $module:ident, $title:literal, $description:literal) => {
        Doc {
            slug: $slug,
            title: $title,
            description: $description,
            content: crate::content::$module::content,
            source: concat!("website/src/content/", stringify!($module), ".rs"),
        }
    };
}

/// The slug served at `/docs` itself.
pub const INTRODUCTION: &str = "introduction";

pub const SECTIONS: &[Section] = &[
    Section {
        title: "Get started",
        docs: &[
            doc!(
                "introduction",
                introduction,
                "Introduction",
                "What Next Rust is, and a quick look at how an app is put together."
            ),
            doc!(
                "getting-started",
                getting_started,
                "Installation",
                "Install Rust and the CLI, create a project and run it."
            ),
        ],
    },
    Section {
        title: "Core concepts",
        docs: &[
            doc!(
                "routing",
                routing,
                "Routing",
                "Special files, layouts, route groups, dynamic segments, ranking, slots and interception."
            ),
            doc!(
                "rendering",
                rendering,
                "Rendering & data",
                "SSR, static generation, ISR, streaming, errors, redirects and metadata."
            ),
            doc!("views", views, "Views & components", "The HTML macros, escaping, lists, conditionals and images."),
        ],
    },
    Section {
        title: "Building apps",
        docs: &[
            doc!(
                "api-routes",
                api_routes,
                "API routes",
                "Route handlers, requests and responses, cookies, server-sent events and WebSockets."
            ),
            doc!(
                "middleware",
                middleware,
                "Middleware & auth",
                "Middleware order, built-in middleware, authentication, sessions and CSRF."
            ),
            doc!(
                "server-actions",
                server_actions,
                "Server actions & forms",
                "Forms that call Rust functions, with validation and redirects."
            ),
            doc!(
                "client",
                client,
                "Client & navigation",
                "Islands, WebAssembly, the server/client boundary and client-side navigation."
            ),
            doc!(
                "styling-and-assets",
                styling_and_assets,
                "Styling & assets",
                "Global CSS, CSS modules, public files, hashed assets, images and fonts."
            ),
            doc!("caching", caching, "Caching", "The data cache, the page store, revalidation and custom stores."),
        ],
    },
    Section {
        title: "Shipping",
        docs: &[
            doc!(
                "configuration",
                configuration,
                "Configuration",
                "The next-rust.toml reference, environment files and monorepos."
            ),
            doc!("cli", cli, "CLI", "Every next-rust command and its options."),
            doc!(
                "deployment",
                deployment,
                "Deployment",
                "The single-binary build, VPS, Docker, Kubernetes, proxies and logging."
            ),
            doc!("testing", testing, "Testing", "Test pages, API routes and actions without starting a server."),
        ],
    },
    Section {
        title: "Reference",
        docs: &[
            doc!("plugins", plugins, "Plugins", "Extend the build and the runtime."),
            doc!("security", security, "Security", "What is secure by default, and what to check before you launch."),
            doc!(
                "architecture",
                architecture,
                "Architecture",
                "How the crates fit together, the request pipeline and design decisions."
            ),
            doc!(
                "benchmarks",
                benchmarks,
                "Benchmarks",
                "What is measured, how, and how to run the benchmarks yourself."
            ),
            doc!("diagnostics", diagnostics, "Diagnostics", "Every error and warning code the build can report."),
            doc!("status", status, "Status & roadmap", "What is implemented, what has limits, and what comes next."),
        ],
    },
];

/// All pages in reading order.
pub fn all() -> impl Iterator<Item = &'static Doc> {
    SECTIONS.iter().flat_map(|s| s.docs.iter())
}

pub fn find(slug: &str) -> Option<&'static Doc> {
    all().find(|d| d.slug == slug)
}

/// URL of a page.
pub fn href(doc: &Doc) -> String {
    if doc.slug == INTRODUCTION { "/docs".into() } else { format!("/docs/{}", doc.slug) }
}

/// The pages before and after `slug` in reading order.
pub fn neighbors(slug: &str) -> (Option<&'static Doc>, Option<&'static Doc>) {
    let docs: Vec<&Doc> = all().collect();
    let Some(i) = docs.iter().position(|d| d.slug == slug) else { return (None, None) };
    (i.checked_sub(1).map(|p| docs[p]), docs.get(i + 1).copied())
}

/// The section a page belongs to.
pub fn section_of(slug: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|s| s.docs.iter().any(|d| d.slug == slug))
}

/// An `<h2>` or `<h3>` in a page.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// Headings with an `id`, in document order.
pub fn headings(html: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<h") {
        rest = &rest[start + 2..];
        let level = match rest.as_bytes().first() {
            Some(b'2') => 2,
            Some(b'3') => 3,
            _ => continue,
        };
        let Some(open_end) = rest.find('>') else { break };
        let open = &rest[..open_end];
        let close = format!("</h{level}>");
        let Some(end) = rest.find(&close) else { break };
        if let Some(id) = attr(open, "id") {
            out.push(Heading { level, id: id.to_owned(), text: text_of(&rest[open_end + 1..end]) });
        }
        rest = &rest[end..];
    }
    out
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// Visible text of an HTML fragment, with entities decoded.
pub fn text_of(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    crate::highlight::unescape(&text)
}

/// A search result.
pub struct Hit {
    pub doc: &'static Doc,
    /// The section of the page the match is in, if not the top.
    pub heading: Option<Heading>,
    pub snippet: String,
    score: u32,
}

/// Case-insensitive search over titles, headings and text. Results are ranked:
/// title matches first, then headings, then body text.
pub fn search(query: &str) -> Vec<Hit> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).filter(|t| t.len() > 1).collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let matches = |text: &str| {
        let lower = text.to_lowercase();
        terms.iter().all(|t| lower.contains(t.as_str()))
    };
    let mut hits = Vec::new();
    for doc in all() {
        if matches(doc.title) || matches(doc.description) {
            let score = if matches(doc.title) { 100 } else { 60 };
            hits.push(Hit { doc, heading: None, snippet: doc.description.to_owned(), score });
        }
        for (heading, body) in sections(doc.html()) {
            let heading_hit = heading.as_ref().is_some_and(|h| matches(&h.text));
            let text = text_of(body);
            if heading_hit || matches(&text) {
                let snippet = snippet(&text, &terms[0]);
                hits.push(Hit { doc, heading, snippet, score: if heading_hit { 80 } else { 10 } });
            }
        }
    }
    hits.sort_by_key(|h| std::cmp::Reverse(h.score));
    hits.truncate(30);
    hits
}

/// Split a page into (heading, HTML up to the next heading).
fn sections(html: &'static str) -> Vec<(Option<Heading>, &'static str)> {
    let heads = headings(html);
    let mut out = Vec::new();
    let mut positions: Vec<(usize, Option<Heading>)> = vec![(0, None)];
    for h in heads {
        if let Some(pos) = html.find(&format!("id=\"{}\"", h.id)) {
            positions.push((pos, Some(h)));
        }
    }
    for i in 0..positions.len() {
        let start = positions[i].0;
        let end = positions.get(i + 1).map(|p| p.0).unwrap_or(html.len());
        out.push((positions[i].1.clone(), &html[start..end]));
    }
    out
}

fn snippet(text: &str, term: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = text.to_lowercase();
    let at = lower.find(term).unwrap_or(0);
    let mut start = at.saturating_sub(60);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (at + 120).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.push_str(text[start..end].trim());
    if end < text.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_content_and_unique_slug() {
        let mut seen = std::collections::HashSet::new();
        for doc in all() {
            assert!(seen.insert(doc.slug), "duplicate slug {}", doc.slug);
            assert!(doc.html().len() > 200, "{} looks empty", doc.slug);
            assert!(!doc.html().contains(".md\""), "{} links to a Markdown file", doc.slug);
            assert!(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(doc.source).is_file(),
                "{}",
                doc.source
            );
        }
        assert_eq!(all().next().map(|d| d.slug), Some(INTRODUCTION));
    }

    #[test]
    fn internal_links_point_at_real_pages() {
        for doc in all() {
            for part in doc.html().split("href=\"/docs").skip(1) {
                let target = part.split(['"', '#']).next().unwrap_or("");
                let slug = target.trim_start_matches('/');
                assert!(slug.is_empty() || find(slug).is_some(), "{} links to missing page /docs{target}", doc.slug);
            }
        }
    }

    #[test]
    fn headings_and_neighbors() {
        let h = headings(
            r##"<h2 id="a"><a class="anchor" href="#a">One <code>x</code></a></h2><p>t</p><h3 id="b">Two &amp; three</h3>"##,
        );
        assert_eq!(h.len(), 2);
        assert_eq!(h[0], Heading { level: 2, id: "a".into(), text: "One x".into() });
        assert_eq!(h[1].text, "Two & three");
        assert_eq!(neighbors(INTRODUCTION).0.map(|d| d.slug), None);
        assert_eq!(neighbors(INTRODUCTION).1.map(|d| d.slug), Some("getting-started"));
        assert_eq!(href(find("routing").unwrap()), "/docs/routing");
    }

    #[test]
    fn search_ranks_titles_first() {
        let hits = search("routing");
        assert_eq!(hits.first().map(|h| h.doc.slug), Some("routing"));
        assert!(search("x").is_empty(), "single letters are ignored");
        assert!(!search("server actions").is_empty());
    }
}
