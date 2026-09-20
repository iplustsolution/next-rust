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
    /// One sentence on when a reader needs this page, shown under the title.
    pub when: &'static str,
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
    ($slug:literal, $module:ident, $title:literal, $description:literal, $when:literal) => {
        Doc {
            slug: $slug,
            title: $title,
            description: $description,
            when: $when,
            content: crate::content::$module::content,
            source: concat!("website/src/content/", stringify!($module), ".rs"),
        }
    };
}

/// The slug served at `/docs` itself.
pub const INTRODUCTION: &str = "introduction";

pub const SECTIONS: &[Section] = &[
    Section {
        title: "Start",
        docs: &[
            doc!(
                "introduction",
                introduction,
                "Introduction",
                "What Next Rust is, and a quick look at how an app is put together.",
                "Start here if you are new: it shows what the framework does and where to go next."
            ),
            doc!(
                "getting-started",
                getting_started,
                "Getting started",
                "Install the CLI, create a project, add a page and ship a binary, in about five minutes.",
                "Follow it top to bottom the first time you use Next Rust, or when adding it to an existing Cargo project."
            ),
            doc!(
                "project-structure",
                project_structure,
                "Project structure",
                "Every file and folder of a Next Rust project: what it is in plain words, what it does, when you touch it and what happens without it.",
                "Read it when you open a project and wonder what a file is for, or which file to open to change something."
            ),
            doc!(
                "coming-from-nextjs",
                coming_from_nextjs,
                "Coming from Next.js",
                "Each Next.js App Router concept next to its Next Rust equivalent, and the differences that matter.",
                "Read it if you already know Next.js and want to map what you know onto Rust files and functions."
            ),
        ],
    },
    Section {
        title: "Core",
        docs: &[
            doc!(
                "routing",
                routing,
                "Routing",
                "Special files, layouts, route groups, dynamic segments, ranking, slots and interception.",
                "Come here whenever you add a folder under app/ and need to know which URL it gets and which layouts wrap it."
            ),
            doc!(
                "rendering",
                rendering,
                "Rendering & data",
                "SSR, static generation, ISR, streaming, errors, redirects and metadata.",
                "Read it when a page needs data, a loading or error state, a redirect, a 404 or page metadata."
            ),
            doc!(
                "views",
                views,
                "Views & components",
                "The HTML macros, escaping, lists, conditionals and images.",
                "Read it before writing markup: it explains the macro syntax every page, layout and component uses."
            ),
            doc!(
                "middleware",
                middleware,
                "Middleware & auth",
                "Middleware order, built-in middleware, authentication, sessions and CSRF.",
                "You need it to run code before every request: logins, sessions, headers, rate limits or redirects."
            ),
            doc!(
                "auth",
                auth,
                "Auth & rate limits",
                "The optional auth kit: sealed tokens, __Host- cookies, rotating sessions, passwords, codes and limiters.",
                "Read it when you build sign-in yourself: sessions that can be revoked, password hashing, one-time codes or per-account limits."
            ),
        ],
    },
    Section {
        title: "Data",
        docs: &[
            doc!(
                "api-routes",
                api_routes,
                "API routes",
                "Route handlers, requests and responses, cookies, server-sent events and WebSockets.",
                "Use it to serve JSON, webhooks, file downloads, event streams or WebSockets from a route.rs file."
            ),
            doc!(
                "server-actions",
                server_actions,
                "Server actions & forms",
                "Forms that call Rust functions, with validation and redirects.",
                "Read it when a form or a button has to change data on the server."
            ),
            doc!(
                "caching",
                caching,
                "Caching",
                "The data cache, the page store, revalidation and custom stores.",
                "You need it when pages or queries are too slow to compute on every request, or must refresh on demand."
            ),
            doc!(
                "background-jobs",
                background_jobs,
                "Background jobs",
                "Durable job queues with retries, cron-style scheduled tasks, the Redis backend and graceful shutdown.",
                "Read it when work should happen outside a request: mail, webhooks, slow APIs, or anything on a schedule."
            ),
        ],
    },
    Section {
        title: "UI",
        docs: &[
            doc!(
                "client",
                client,
                "Client & navigation",
                "Islands, the server/client boundary and client-side navigation.",
                "Read it when a page needs interactivity in the browser, or to understand how links navigate without a reload."
            ),
            doc!(
                "wasm-islands",
                wasm_islands,
                "WebAssembly islands",
                "Browser logic in Rust: the switches, the ABI, real sizes, delivery, CSP and what happens when it doesn't run.",
                "Read it before turning WebAssembly on — including the part about what it is not, which is a security boundary."
            ),
            doc!(
                "styling-and-assets",
                styling_and_assets,
                "Styling & assets",
                "Global CSS, CSS modules, public files, hashed assets, images and fonts.",
                "Use it for plain CSS, CSS modules, fonts, images and files in public/. For utility classes see Tailwind CSS."
            ),
            doc!(
                "tailwind",
                tailwind,
                "Tailwind CSS",
                "Style with Tailwind classes. Built in: no Node.js, no CSS files, your theme in next-rust.toml.",
                "Read it if your project has Tailwind enabled, which is the default for new projects."
            ),
            doc!(
                "components",
                components,
                "UI components",
                "Buttons, fields, select, date picker, toggles, avatars, cards and layout, ready to use.",
                "Check it before hand-building a form control or a layout primitive: it probably exists already."
            ),
        ],
    },
    Section {
        title: "Deploy",
        docs: &[
            doc!(
                "configuration",
                configuration,
                "Configuration",
                "The next-rust.toml reference, environment files and monorepos.",
                "Look things up here when you change a port, a header, a rendering default or any other project-wide setting."
            ),
            doc!(
                "cli",
                cli,
                "CLI",
                "Every next-rust command and its options.",
                "A reference for the next-rust command: look up a flag, or find the command for a job."
            ),
            doc!(
                "deployment",
                deployment,
                "Deployment",
                "The single-binary build, VPS, Docker, Kubernetes, proxies and logging.",
                "Read it when you are ready to put the app on a server, in a container or behind a proxy."
            ),
            doc!(
                "analytics",
                analytics,
                "Analytics & verification",
                "Search Console, Bing and other site verification, Google Analytics, Tag Manager, pixels and privacy-friendly analytics, from next-rust.toml alone.",
                "Read it before launch: to verify the site with search engines, add analytics without writing a script, keep signed-in areas out of it, or ask for consent first."
            ),
            doc!(
                "testing",
                testing,
                "Testing",
                "Test pages, API routes and actions without starting a server.",
                "Use it to cover routes with cargo test, in CI or locally, without opening a port."
            ),
            doc!(
                "security",
                security,
                "Security",
                "What is secure by default, and what to check before you launch.",
                "Read it once before launch, and again whenever you handle logins, uploads or user-supplied HTML."
            ),
        ],
    },
    Section {
        title: "Reference",
        docs: &[
            doc!(
                "diagnostics",
                diagnostics,
                "Diagnostics",
                "Every error and warning code the build can report.",
                "Come here when a build fails with an NR code and you want the cause and the fix."
            ),
            doc!(
                "plugins",
                plugins,
                "Plugins",
                "Extend the build and the runtime.",
                "You need it only when you package middleware, startup hooks or build steps as a reusable crate."
            ),
            doc!(
                "architecture",
                architecture,
                "Architecture",
                "How the crates fit together, the request pipeline and design decisions.",
                "For contributors and the curious: read it before changing the framework itself."
            ),
            doc!(
                "benchmarks",
                benchmarks,
                "Benchmarks",
                "What is measured, how, and how to run the benchmarks yourself.",
                "Read it when you want numbers, or want to reproduce them on your own hardware."
            ),
            doc!(
                "status",
                status,
                "Status & roadmap",
                "What is implemented, what has limits, and what comes next.",
                "Check it before you depend on a feature in production."
            ),
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
            // Start after the heading's opening tag, so the snippet is text only.
            let start = html[pos..].find('>').map_or(pos, |end| pos + end + 1);
            positions.push((start, Some(h)));
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
    fn pages_open_with_an_introduction_and_keep_heading_order() {
        for doc in all() {
            assert!(doc.when.ends_with('.'), "{}: `when` is a sentence", doc.slug);
            let html = doc.html();
            assert!(!html.contains("<h1"), "{}: the page title is the only h1", doc.slug);
            let mut level = 2;
            for h in headings(html) {
                assert!(h.level <= level + 1, "{}: #{} skips a heading level", doc.slug, h.id);
                level = h.level;
            }
        }
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
        assert!(hits.iter().all(|h| !h.snippet.contains("id=\"")), "snippets are text, not markup");
    }
}
