//! Components shared by every page of the site.

use next_rust::prelude::*;

use crate::docs::{self, Doc, Heading};

pub const REPO: &str = "https://github.com/iplustsolution/next-rust";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const GITHUB_ICON: &str = r#"<svg viewBox="0 0 16 16" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>"#;
const SEARCH_ICON: &str = r#"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>"#;
const MENU_ICON: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16"/></svg>"#;
const ARROW_LEFT: &str = r#"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 18l-6-6 6-6"/></svg>"#;
const ARROW_RIGHT: &str = r#"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 18l6-6-6-6"/></svg>"#;
const EDIT_ICON: &str = r#"<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>"#;

/// The site header. It lives in the root layout, so it is rendered once per
/// visit and stays on screen (logo included) across every navigation.
pub fn header() -> impl View {
    header![
        class("site-header"),
        div![
            class("header-inner"),
            a![
                class("brand"),
                href("/"),
                aria("label", "Next Rust home"),
                img![src("/logo.svg"), alt(""), width(28), height(28)],
                span!["Next Rust"],
                span![class("version"), format!("v{VERSION}")],
            ],
            nav![
                class("header-nav"),
                aria("label", "Main"),
                a![class("nav-link"), href("/docs"), active_class_prefix("active"), "Docs"],
                a![
                    class("nav-link"),
                    href(format!("{REPO}/tree/main/examples")),
                    target("_blank"),
                    rel("noopener"),
                    "Examples"
                ],
                a![
                    class("nav-link"),
                    href(format!("{REPO}/blob/main/CHANGELOG.md")),
                    target("_blank"),
                    rel("noopener"),
                    "Changelog"
                ],
            ],
            div![
                class("header-end"),
                form![
                    class("search"),
                    method("get"),
                    action("/docs/search"),
                    role("search"),
                    raw_html(SEARCH_ICON),
                    input![
                        r#type("search"),
                        name("q"),
                        placeholder("Search documentation"),
                        aria("label", "Search documentation"),
                        autocomplete("off"),
                    ],
                ],
                a![
                    class("icon-link search-link"),
                    href("/docs/search"),
                    aria("label", "Search"),
                    raw_html(SEARCH_ICON)
                ],
                a![
                    class("icon-link"),
                    href(REPO),
                    target("_blank"),
                    rel("noopener"),
                    aria("label", "Next Rust on GitHub"),
                    raw_html(GITHUB_ICON)
                ],
            ],
        ],
    ]
}

pub fn footer() -> impl View {
    footer![
        class("site-footer"),
        div![
            class("footer-inner"),
            div![
                class("footer-brand"),
                img![src("/logo.svg"), alt(""), width(24), height(24)],
                p![
                    "Next Rust is open source under MIT or Apache-2.0. This site is built with Next Rust and runs as a single binary."
                ],
            ],
            div![
                class("footer-links"),
                a![href("/docs"), "Documentation"],
                a![href(REPO), target("_blank"), rel("noopener"), "GitHub"],
                a![
                    href(format!("{REPO}/blob/main/CONTRIBUTING.md")),
                    target("_blank"),
                    rel("noopener"),
                    "Contributing"
                ],
                a![href(format!("{REPO}/issues")), target("_blank"), rel("noopener"), "Issues"],
            ],
            p![
                class("credit"),
                "Handled and managed by ",
                a![href("https://www.iplust.in/"), target("_blank"), rel("noopener"), "I Plus T Solution"],
            ],
        ],
    ]
}

fn sidebar_links() -> impl View {
    each(docs::SECTIONS, |section| {
        div![
            class("side-section"),
            p![class("side-title"), section.title],
            ul![each(section.docs, |doc| li![a![
                class("side-link"),
                href(docs::href(doc)),
                // Highlighted by the framework for the current URL, so the
                // sidebar can live in a layout that stays on screen.
                active_class("active"),
                doc.title,
            ]])],
        ]
    })
}

/// Left navigation on wide screens, a collapsible menu on small ones.
pub fn sidebar() -> impl View {
    fragment![
        details![
            class("mobile-nav"),
            summary![raw_html(MENU_ICON), span!["Documentation menu"]],
            nav![aria("label", "Documentation"), sidebar_links()],
        ],
        aside![class("sidebar"), nav![aria("label", "Documentation"), sidebar_links()]],
    ]
}

/// Sidebar and grid shared by every docs page (`app/docs/layout.rs`). It
/// reads nothing from the request, so navigating between docs pages only
/// replaces the article and its outline.
pub fn docs_shell(children: Children) -> impl View {
    div![class("docs"), sidebar(), children]
}

/// "On this page" outline.
pub fn outline(headings: &[Heading]) -> impl View {
    let items: Vec<Heading> = headings.to_vec();
    aside![
        class("outline"),
        (!items.is_empty()).then(|| {
            nav![
                aria("label", "On this page"),
                p![class("outline-title"), "On this page"],
                ul![each(items, |h| li![
                    class(if h.level == 3 { "depth-3" } else { "depth-2" }),
                    a![href(format!("#{}", h.id)), h.text]
                ])],
            ]
        }),
    ]
}

fn pager(doc: &Doc) -> impl View {
    let (prev, next) = docs::neighbors(doc.slug);
    nav![
        class("pager"),
        aria("label", "Previous and next page"),
        prev.map(|d| a![
            class("pager-link prev"),
            href(docs::href(d)),
            span![class("pager-label"), raw_html(ARROW_LEFT), "Previous"],
            strong![d.title],
        ]),
        next.map(|d| a![
            class("pager-link next"),
            href(docs::href(d)),
            span![class("pager-label"), "Next", raw_html(ARROW_RIGHT)],
            strong![d.title],
        ]),
    ]
}

/// A documentation page inside the docs shell: article, pager and outline.
pub fn doc_page(doc: &'static Doc) -> impl View {
    let headings = docs::headings(doc.html());
    let section = docs::section_of(doc.slug).map(|s| s.title).unwrap_or("Docs");
    fragment![
        main![
            class("doc"),
            id("content"),
            article![
                p![class("eyebrow"), section],
                h1![doc.title],
                p![class("lead"), doc.description],
                div![class("prose"), raw_html(crate::highlight::code_blocks(doc.html()))],
            ],
            div![
                class("doc-meta"),
                a![
                    href(format!("{REPO}/blob/main/{}", doc.source)),
                    target("_blank"),
                    rel("noopener"),
                    raw_html(EDIT_ICON),
                    "Edit this page on GitHub",
                ],
            ],
            pager(doc),
        ],
        outline(&headings),
    ]
}
