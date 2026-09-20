//! Components shared by every page of the site, styled with Tailwind CSS.

use next_rust::prelude::*;

use crate::docs::{self, Doc, Heading};

pub const REPO: &str = "https://github.com/iplustsolution/next-rust";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const GITHUB_ICON: &str = r#"<svg viewBox="0 0 16 16" width="1.125rem" height="1.125rem" fill="currentColor" aria-hidden="true"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>"#;
const HEART_ICON: &str = r#"<svg viewBox="0 0 24 24" width="0.9375rem" height="0.9375rem" fill="currentColor" aria-hidden="true"><path d="M2 9.5a5.5 5.5 0 0 1 9.591-3.676.56.56 0 0 0 .818 0A5.49 5.49 0 0 1 22 9.5c0 2.29-1.5 4-3 5.5l-5.492 5.313a2 2 0 0 1-3 .019L5 15c-1.5-1.5-3-3.2-3-5.5"/></svg>"#;
pub const SPONSOR: &str = "https://github.com/sponsors/iplustsolution";
const SEARCH_ICON: &str = r#"<svg viewBox="0 0 24 24" width="1rem" height="1rem" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>"#;
const MENU_ICON: &str = r#"<svg viewBox="0 0 24 24" width="1.125rem" height="1.125rem" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16"/></svg>"#;
const ARROW_LEFT: &str = r#"<svg viewBox="0 0 24 24" width="1rem" height="1rem" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 18l-6-6 6-6"/></svg>"#;
const ARROW_RIGHT: &str = r#"<svg viewBox="0 0 24 24" width="1rem" height="1rem" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 18l6-6-6-6"/></svg>"#;
const EDIT_ICON: &str = r#"<svg viewBox="0 0 24 24" width="0.875rem" height="0.875rem" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>"#;

/// The ambient background, mounted once by the root layout so every page sits
/// on the same light. One warm bloom rises from a point just above the top
/// edge and spreads outwards; the falloff is written as five stops with
/// progressively smaller alpha steps, because a two-stop radial gradient bands
/// badly across a whole viewport.
const AMBIENT_BLOOM: &str = "absolute inset-0 bg-[radial-gradient(95%_68%_at_50%_-8%,var(--color-glow-1)_0%,var(--color-glow-2)_22%,var(--color-glow-3)_42%,var(--color-glow-4)_64%,transparent_86%)] motion-safe:animate-ambient-drift";

/// Two far dimmer blooms to the sides for depth, plus a barely-there wash from
/// the top edge so the very top of the page is not flat.
const AMBIENT_DEPTH: &str = "absolute inset-0 bg-[radial-gradient(52%_40%_at_86%_10%,var(--color-glow-amber)_0%,var(--color-glow-ember)_46%,transparent_76%),radial-gradient(46%_36%_at_8%_44%,var(--color-glow-ember)_0%,transparent_72%),linear-gradient(to_bottom,var(--color-glow-wash)_0%,transparent_34%)]";

/// Site-wide ambient light. `fixed` keeps it a constant ambience instead of a
/// decoration that scrolls away, and `-z-10` puts it behind every page's
/// content while staying above the page background.
pub fn ambient() -> impl View {
    div![
        class("pointer-events-none fixed inset-0 -z-10 overflow-hidden"),
        aria("hidden", "true"),
        div![class(AMBIENT_BLOOM)],
        div![class(AMBIENT_DEPTH)],
    ]
}

/// Colors for the token classes the highlighter emits (`tk`, `tf`, …), on
/// any element that contains highlighted code.
pub const SYNTAX: &str = "[&_.tk]:text-syn-keyword [&_.tt]:text-syn-keyword [&_.tf]:text-syn-function [&_.tp]:text-syn-function [&_.ty]:text-syn-type [&_.tm]:text-syn-macro [&_.ts]:text-syn-string [&_.tn]:text-syn-number [&_.ta]:text-syn-attribute [&_.tc]:text-syn-comment [&_.tc]:italic";

/// Buttons.
pub const BUTTON: &str = "inline-flex h-11 items-center justify-center gap-2 rounded-[0.625rem] border border-line-strong bg-raised px-5 text-[0.9375rem] font-semibold text-fg transition hover:border-muted active:translate-y-px max-[640px]:flex-[1_1_100%]";
pub const BUTTON_PRIMARY: &str = "inline-flex h-11 items-center justify-center gap-2 rounded-[0.625rem] border border-accent bg-accent px-5 text-[0.9375rem] font-semibold text-on-accent transition hover:brightness-108 active:translate-y-px max-[640px]:flex-[1_1_100%]";
pub const ACTIONS: &str = "mt-9 flex flex-wrap justify-center gap-3";

/// Documentation article body. The content modules only mark structure
/// (`anchor`, `callout`, `card`, `table-wrap`, …); everything is styled here.
const PROSE: &str = concat!(
    "mt-10 text-base/[1.75] text-fg-soft [&>:first-child]:mt-0 ",
    // Readable line length for text; code, tables and grids use the full column.
    "[&>*]:max-w-[44rem] [&>:is(.code,.table-wrap,.card-grid,.feature-list,.crate-grid)]:max-w-none ",
    // Headings and their `#` links.
    "[&_:is(h2,h3,h4)]:font-[650] [&_:is(h2,h3,h4)]:tracking-[-0.02em] [&_:is(h2,h3,h4)]:text-fg ",
    "[&_h2]:mt-14 [&_h2]:mb-4 [&_h2]:border-t [&_h2]:border-line [&_h2]:pt-8 [&_h2]:text-[1.625rem]/[1.25] max-[640px]:[&_h2]:text-[1.4375rem] ",
    "[&_h3]:mt-9 [&_h3]:mb-3 [&_h3]:text-xl/[1.35] [&_h4]:mt-7 [&_h4]:mb-2.5 [&_h4]:text-[1.0625rem] ",
    "[&_.anchor]:after:ml-2 [&_.anchor]:after:font-normal [&_.anchor]:after:text-muted [&_.anchor]:after:opacity-0 [&_.anchor]:after:transition-opacity [&_.anchor]:after:content-['#'] [&_.anchor:hover]:after:opacity-100 ",
    // Text, lists and links.
    "[&_:is(p,ul,ol)]:my-4 [&_ul]:list-disc [&_ol]:list-decimal [&_:is(ul,ol)]:pl-6 [&_li]:my-1.5 [&_li]:pl-1 [&_li]:marker:text-muted ",
    "[&_a:not(.anchor,.card)]:text-accent-fg [&_a:not(.anchor,.card)]:underline [&_a:not(.anchor,.card)]:decoration-accent-fg/35 [&_a:not(.anchor,.card)]:underline-offset-3 [&_a:not(.anchor,.card)]:transition [&_a:not(.anchor,.card):hover]:decoration-current ",
    "[&_strong]:font-[650] [&_strong]:text-fg ",
    // Inline code.
    "[&_:not(pre)>code]:rounded-md [&_:not(pre)>code]:border [&_:not(pre)>code]:border-line [&_:not(pre)>code]:bg-soft [&_:not(pre)>code]:px-1.5 [&_:not(pre)>code]:py-0.5 [&_:not(pre)>code]:font-mono [&_:not(pre)>code]:text-[0.86em] [&_:not(pre)>code]:text-fg [&_:not(pre)>code]:wrap-anywhere [&_:is(h2,h3)_code]:text-[0.85em] ",
    // Code blocks framed by the highlighter.
    "[&_.code]:my-5 [&_.code]:overflow-hidden [&_.code]:rounded-xl [&_.code]:border [&_.code]:border-line [&_.code]:bg-code max-[640px]:[&_.code]:-mx-5 max-[640px]:[&_.code]:rounded-none max-[640px]:[&_.code]:border-x-0 ",
    "[&_.code-bar]:flex [&_.code-bar]:h-[2.375rem] [&_.code-bar]:items-center [&_.code-bar]:border-b [&_.code-bar]:border-line [&_.code-bar]:px-4 [&_.code-bar]:text-[0.78125rem] [&_.code-bar]:font-medium [&_.code-bar]:text-muted ",
    "[&_.code_pre]:overflow-x-auto [&_.code_pre]:px-[1.125rem] [&_.code_pre]:py-4 [&_.code_pre]:font-mono [&_.code_pre]:text-[0.84375rem]/[1.7] [&_.code_pre]:text-fg-soft [&_.code_pre]:[tab-size:4] ",
    // Tables.
    "[&_.table-wrap]:my-5 [&_.table-wrap]:overflow-x-auto [&_.table-wrap]:rounded-xl [&_.table-wrap]:border [&_.table-wrap]:border-line ",
    "[&_table]:w-full [&_table]:border-collapse [&_table]:text-[0.90625rem]/[1.55] ",
    "[&_:is(th,td)]:border-b [&_:is(th,td)]:border-line [&_:is(th,td)]:px-3.5 [&_:is(th,td)]:py-2.5 [&_:is(th,td)]:text-left [&_:is(th,td)]:align-top ",
    "[&_th]:bg-soft [&_th]:text-[0.8125rem] [&_th]:font-semibold [&_th]:whitespace-nowrap [&_th]:text-fg [&_tbody_tr:last-child_td]:border-b-0 [&_td_code]:whitespace-nowrap ",
    // Callouts.
    "[&_.callout]:my-6 [&_.callout]:rounded-[0.625rem] [&_.callout]:border [&_.callout]:border-l-3 [&_.callout]:border-accent/30 [&_.callout]:border-l-accent [&_.callout]:bg-accent/10 [&_.callout]:px-[1.125rem] [&_.callout]:py-3.5 [&_.callout_p]:my-0 [&_.callout_p+p]:mt-2.5 ",
    // Feature lists and link cards.
    "[&_:is(.feature-list,.card-grid)]:my-5 [&_:is(.feature-list,.card-grid)]:grid [&_:is(.feature-list,.card-grid)]:grid-cols-2 [&_:is(.feature-list,.card-grid)]:gap-3 max-[640px]:[&_:is(.feature-list,.card-grid)]:grid-cols-1 ",
    "[&_:is(.feature-list>div,.card)]:flex [&_:is(.feature-list>div,.card)]:flex-col [&_:is(.feature-list>div,.card)]:gap-1.5 [&_:is(.feature-list>div,.card)]:rounded-xl [&_:is(.feature-list>div,.card)]:border [&_:is(.feature-list>div,.card)]:border-line [&_:is(.feature-list>div,.card)]:bg-soft [&_:is(.feature-list>div,.card)]:px-5 [&_:is(.feature-list>div,.card)]:py-[1.125rem] [&_:is(.feature-list>div,.card)]:leading-[1.55] ",
    "[&_:is(.feature-list,.card)_strong]:text-fg [&_:is(.feature-list,.card)_span]:text-[0.90625rem] [&_:is(.feature-list,.card)_span]:text-muted ",
    "[&_.card]:transition [&_.card:hover]:border-accent [&_.card_strong]:after:text-accent-fg [&_.card_strong]:after:content-['_→'] ",
    // The crate graph on the Architecture page.
    "[&_.crate-grid]:my-5 [&_.crate-grid]:grid [&_.crate-grid]:grid-cols-2 [&_.crate-grid]:gap-2.5 max-[640px]:[&_.crate-grid]:grid-cols-1 ",
    "[&_.crate]:flex [&_.crate]:flex-col [&_.crate]:gap-0.5 [&_.crate]:rounded-[0.625rem] [&_.crate]:border [&_.crate]:border-line [&_.crate]:bg-soft [&_.crate]:px-4 [&_.crate]:py-3.5 ",
    "[&_.crate_code]:self-start [&_.crate_code]:border-0 [&_.crate_code]:bg-transparent [&_.crate_code]:p-0 [&_.crate_code]:text-sm/[1.75] [&_.crate_code]:text-accent-fg ",
    "[&_.crate_span]:text-[0.90625rem] [&_.crate_span]:text-fg [&_.crate_small]:text-[0.8125rem] [&_.crate_small]:text-muted"
);

/// The site header. It lives in the root layout, so it is rendered once per
/// visit and stays on screen (logo included) across every navigation.
pub fn header() -> impl View {
    let nav_link = "inline-flex items-center rounded-lg px-2.5 py-1.5 max-[860px]:min-h-11 max-[860px]:px-3.5 text-sm/[1.6] font-medium text-muted transition-colors hover:text-fg aria-[current=page]:bg-raised aria-[current=page]:text-fg";
    header![
        class("sticky top-0 z-50 h-16 border-b border-line bg-header backdrop-blur-md backdrop-saturate-180"),
        div![
            class("mx-auto flex h-full max-w-[100rem] items-center gap-7 px-6 max-[860px]:gap-3 max-[860px]:px-4"),
            a![
                class("flex min-h-11 items-center gap-2.5 font-[650] tracking-[-0.01em] whitespace-nowrap"),
                href("/"),
                aria("label", "Next Rust home"),
                img![class("size-7 rounded-[0.4375rem]"), src("/logo.svg"), alt(""), width(28), height(28)],
                span!["Next Rust"],
                span![
                    class(
                        "rounded-full border border-line-strong px-[0.4375rem] py-0.5 font-mono text-[0.6875rem]/[1.4] font-medium text-muted max-[640px]:hidden"
                    ),
                    format!("v{VERSION}")
                ],
            ],
            nav![
                class("flex gap-1"),
                aria("label", "Main"),
                a![class(nav_link), href("/docs"), active_class_prefix("active"), "Docs"],
                a![
                    class(nav_link),
                    class("max-[860px]:hidden"),
                    href("/components"),
                    active_class_prefix("active"),
                    "Components"
                ],
                a![
                    class(nav_link),
                    class("max-[860px]:hidden"),
                    href("/icons"),
                    active_class_prefix("active"),
                    "Icons"
                ],
                a![
                    class(nav_link),
                    class("max-[860px]:hidden"),
                    href(format!("{REPO}/tree/main/examples")),
                    target("_blank"),
                    rel("noopener"),
                    "Examples"
                ],
                a![
                    class(nav_link),
                    class("max-[860px]:hidden"),
                    href(format!("{REPO}/blob/main/CHANGELOG.md")),
                    target("_blank"),
                    rel("noopener"),
                    "Changelog"
                ],
            ],
            div![
                class("ml-auto flex items-center gap-2"),
                form![
                    class("relative flex items-center max-[860px]:hidden"),
                    method("get"),
                    action("/docs/search"),
                    role("search"),
                    span![class("pointer-events-none absolute left-[0.6875rem] text-muted"), raw_html(SEARCH_ICON)],
                    input![
                        class(
                            "h-9 w-[16.25rem] rounded-[0.5625rem] border border-line-strong bg-soft pr-3 pl-[2.125rem] text-sm text-fg transition placeholder:text-muted focus:border-accent focus:ring-3 focus:ring-accent/15 focus:outline-none max-[960px]:w-[12.5rem]"
                        ),
                        r#type("search"),
                        name("q"),
                        placeholder("Search documentation"),
                        aria("label", "Search documentation"),
                        autocomplete("off"),
                    ],
                ],
                a![
                    class(
                        "hidden size-11 place-items-center rounded-[0.5625rem] text-muted transition-colors hover:bg-raised hover:text-fg max-[860px]:grid"
                    ),
                    href("/docs/search"),
                    aria("label", "Search"),
                    raw_html(SEARCH_ICON)
                ],
                a![
                    class(
                        "grid size-9 place-items-center rounded-[0.5625rem] text-muted transition-colors hover:bg-raised hover:text-fg max-[860px]:size-11"
                    ),
                    href(REPO),
                    target("_blank"),
                    rel("noopener"),
                    aria("label", "Next Rust on GitHub"),
                    raw_html(GITHUB_ICON)
                ],
                a![
                    class(
                        "ml-1 inline-flex h-9 items-center gap-2 rounded-[0.5625rem] border border-line-strong bg-raised px-3 text-[0.8125rem] font-semibold text-fg transition-colors hover:border-accent max-[1100px]:hidden"
                    ),
                    href(SPONSOR),
                    target("_blank"),
                    rel("noopener"),
                    span![class("text-accent"), raw_html(HEART_ICON)],
                    "Sponsor"
                ],
            ],
        ],
    ]
}

/// A hairline along the top edge that lights up in the accent towards the
/// middle: the footer's tie-in with the ambient background.
const FOOTER_EDGE: &str = "pointer-events-none absolute inset-x-0 -top-px h-px bg-[linear-gradient(to_right,transparent_0%,var(--color-glow-edge)_50%,transparent_100%)]";

/// A wide, flat bloom rising from that edge, so the page closes on light
/// rather than on a grey slab.
const FOOTER_BLOOM: &str = "pointer-events-none absolute inset-x-0 top-0 -z-10 h-[28rem] bg-[radial-gradient(58%_100%_at_50%_0%,var(--color-glow-1)_0%,var(--color-glow-2)_26%,var(--color-glow-3)_47%,var(--color-glow-4)_68%,transparent_88%)]";

pub fn footer() -> impl View {
    let link = "inline-flex items-center text-[0.9375rem]/[1.6] text-muted transition-colors hover:text-fg max-[860px]:min-h-11";
    let heading = "mb-4 text-[0.75rem] font-[650] tracking-[0.11em] text-fg uppercase max-[860px]:mb-2";
    let list = "flex flex-col gap-2.5 max-[860px]:gap-0";
    let external =
        |text: &'static str, url: String| li![a![class(link), href(url), target("_blank"), rel("noopener"), text]];
    footer![
        class("relative isolate border-t border-line"),
        div![class(FOOTER_EDGE)],
        div![class(FOOTER_BLOOM)],
        div![
            class("mx-auto max-w-[80rem] px-6 pt-16 pb-9 max-[860px]:px-5 max-[860px]:pt-12"),
            div![
                class(
                    "grid grid-cols-[minmax(0,1fr)_auto_auto_auto] gap-x-16 gap-y-11 max-[1100px]:gap-x-10 max-[860px]:grid-cols-2 max-[860px]:gap-x-8"
                ),
                div![
                    class("max-w-[26rem] max-[860px]:col-span-full"),
                    div![
                        class("flex flex-wrap items-center gap-x-3 gap-y-2"),
                        a![
                            class(
                                "inline-flex items-center gap-2.5 text-[1.125rem] font-[650] tracking-[-0.015em] text-fg"
                            ),
                            href("/"),
                            img![
                                class("size-8 rounded-[0.5rem]"),
                                src("/logo.svg"),
                                alt(""),
                                width(32),
                                height(32),
                                loading("lazy")
                            ],
                            "Next Rust",
                        ],
                        span![
                            class(
                                "inline-flex items-center gap-1.5 rounded-full border border-line-strong px-2 py-0.5 font-mono text-[0.6875rem] text-muted"
                            ),
                            span![class("size-1.5 rounded-full bg-accent")],
                            format!("v{VERSION}"),
                        ],
                    ],
                    p![
                        class("mt-4 text-[0.9375rem]/[1.65] text-muted"),
                        "The full-stack web framework for Rust. This site is built with it and runs as a single binary."
                    ],
                    a![
                        class(
                            "mt-6 flex h-11 w-fit items-center gap-2 rounded-[0.625rem] border border-line-strong bg-raised px-4 text-[0.875rem] font-semibold text-fg transition-colors hover:border-accent hover:text-accent-fg"
                        ),
                        href(SPONSOR),
                        target("_blank"),
                        rel("noopener"),
                        span![class("text-accent"), raw_html(HEART_ICON)],
                        "Become a sponsor",
                    ],
                ],
                nav![
                    aria("label", "Documentation links"),
                    p![class(heading), "Docs"],
                    ul![
                        class(list),
                        li![a![class(link), href("/docs/getting-started"), "Getting started"]],
                        li![a![class(link), href("/docs/routing"), "Routing"]],
                        li![a![class(link), href("/components"), "Components gallery"]],
                        li![a![class(link), href("/icons"), "Icons"]],
                    ],
                ],
                nav![
                    aria("label", "Reference links"),
                    p![class(heading), "Reference"],
                    ul![
                        class(list),
                        li![a![class(link), href("/docs/configuration"), "Configuration"]],
                        li![a![class(link), href("/docs/cli"), "CLI"]],
                        li![a![class(link), href("/docs/deployment"), "Deployment"]],
                        li![a![class(link), href("/docs/benchmarks"), "Benchmarks"]],
                    ],
                ],
                nav![
                    aria("label", "Community links"),
                    p![class(heading), "Community"],
                    ul![
                        class(list),
                        external("GitHub", REPO.to_owned()),
                        external("Contributing", format!("{REPO}/blob/main/CONTRIBUTING.md")),
                        external("Issues", format!("{REPO}/issues")),
                        external("Changelog", format!("{REPO}/blob/main/CHANGELOG.md")),
                        external("Sponsor", SPONSOR.to_owned()),
                    ],
                ],
            ],
            div![
                class(
                    "mt-14 flex flex-wrap items-center justify-between gap-x-10 gap-y-2 border-t border-line pt-6 text-[0.84375rem]/[1.6] text-muted max-[860px]:mt-10"
                ),
                p!["Open source under MIT or Apache-2.0."],
                p![
                    "Handled and managed by ",
                    a![
                        class("font-medium text-fg-soft transition-colors hover:text-accent-fg"),
                        href("https://www.iplust.in/"),
                        target("_blank"),
                        rel("noopener"),
                        "I Plus T Solution"
                    ],
                ],
            ],
        ],
    ]
}

fn sidebar_links() -> impl View {
    each(docs::SECTIONS, |section| {
        div![
            class("not-first:mt-[1.625rem]"),
            p![class("mb-2 px-2.5 text-[0.8125rem] font-[650] text-fg"), section.title],
            ul![each(section.docs, |doc| li![a![
                class(
                    "block rounded-[0.4375rem] px-2.5 py-1.5 text-sm/[1.45] max-[860px]:flex max-[860px]:min-h-11 max-[860px]:items-center max-[860px]:text-[0.9375rem] text-muted transition-colors hover:bg-soft hover:text-fg aria-[current=page]:bg-accent/10 aria-[current=page]:font-[550] aria-[current=page]:text-accent-fg"
                ),
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
            class("group sticky top-16 z-40 hidden border-b border-line bg-header backdrop-blur-md max-[860px]:block"),
            summary![
                class(
                    "flex min-h-12 cursor-pointer list-none items-center gap-2.5 px-5 text-sm/[1.6] font-medium text-fg-soft group-open:border-b group-open:border-line [&::-webkit-details-marker]:hidden"
                ),
                raw_html(MENU_ICON),
                span!["Documentation menu"]
            ],
            nav![
                class("max-h-[calc(100vh-7rem)] overflow-y-auto px-2.5 pt-5 pb-7"),
                aria("label", "Documentation"),
                sidebar_links()
            ],
        ],
        aside![
            class(
                "sticky top-16 max-h-[calc(100vh-4rem)] self-start overflow-y-auto overscroll-contain pt-7 pr-4 pb-12 pl-6 [scrollbar-width:thin] max-[860px]:hidden"
            ),
            nav![aria("label", "Documentation"), sidebar_links()]
        ],
    ]
}

/// Sidebar and grid shared by every docs page (`app/docs/layout.rs`). It
/// reads nothing from the request, so navigating between docs pages only
/// replaces the article and its outline.
pub fn docs_shell(children: Children) -> impl View {
    div![
        class(
            "mx-auto grid max-w-[100rem] grid-cols-[17rem_minmax(0,1fr)_15rem] min-[1600px]:grid-cols-[18rem_minmax(0,1fr)_17rem] max-[1240px]:grid-cols-[16rem_minmax(0,1fr)] max-[860px]:block"
        ),
        sidebar(),
        children
    ]
}

/// Main column of a docs page.
pub const DOC_MAIN: &str = "min-w-0 border-l border-line px-16 pt-12 pb-20 max-[1240px]:px-12 max-[1240px]:pt-11 max-[1240px]:pb-18 max-[860px]:border-l-0 max-[860px]:px-5 max-[860px]:pt-9 max-[860px]:pb-16";
pub const EYEBROW: &str = "mb-2.5 text-[0.8125rem] font-semibold text-accent-fg";
pub const DOC_TITLE: &str = "text-[clamp(2rem,4.5vw,2.625rem)] leading-[1.12] font-bold tracking-[-0.035em]";
pub const OUTLINE: &str = "sticky top-16 max-h-[calc(100vh-4rem)] self-start overflow-y-auto overscroll-contain py-12 pr-6 pl-2 max-[1240px]:hidden";

/// "On this page" outline.
pub fn outline(headings: &[Heading]) -> impl View {
    let items: Vec<Heading> = headings.to_vec();
    aside![
        class(OUTLINE),
        (!items.is_empty()).then(|| {
            nav![
                aria("label", "On this page"),
                p![class("mb-2.5 text-[0.8125rem] font-[650]"), "On this page"],
                ul![each(items, |h| li![a![
                    class(if h.level == 3 {
                        "block py-1 pl-3.5 text-[0.84375rem]/[1.45] text-muted transition-colors hover:text-fg"
                    } else {
                        "block py-1 text-[0.84375rem]/[1.45] text-muted transition-colors hover:text-fg"
                    }),
                    href(format!("#{}", h.id)),
                    h.text
                ]])],
            ]
        }),
    ]
}

fn pager(doc: &Doc) -> impl View {
    let (prev, next) = docs::neighbors(doc.slug);
    let link = "flex flex-col gap-1 rounded-xl border border-line px-5 py-4 transition-colors hover:border-line-strong hover:bg-soft";
    let label = "inline-flex items-center gap-1 text-[0.8125rem] text-muted";
    nav![
        class("mt-3 grid grid-cols-2 gap-4 max-[640px]:grid-cols-1"),
        aria("label", "Previous and next page"),
        prev.map(|d| a![
            class(link),
            href(docs::href(d)),
            span![class(label), raw_html(ARROW_LEFT), "Previous"],
            strong![class("font-semibold text-accent-fg"), d.title],
        ]),
        next.map(|d| a![
            class(link),
            class("col-start-2 items-end text-right max-[640px]:col-start-1"),
            href(docs::href(d)),
            span![class(label), "Next", raw_html(ARROW_RIGHT)],
            strong![class("font-semibold text-accent-fg"), d.title],
        ]),
    ]
}

/// Widest comfortable line of prose (about 85 characters). Code blocks,
/// tables and card grids may use the whole column (`DOC_COLUMN`).
const MEASURE: &str = "max-w-[44rem]";
const DOC_COLUMN: &str = "mx-auto max-w-[60rem]";

/// The outline again, folded, for screens too narrow for the third column.
fn outline_inline(headings: &[Heading]) -> impl View {
    let items: Vec<Heading> = headings.to_vec();
    (!items.is_empty()).then(|| {
        details![
            class("group mt-7 rounded-xl border border-line bg-soft min-[1241px]:hidden"),
            class(MEASURE),
            summary![
                class(
                    "flex min-h-11 cursor-pointer list-none items-center justify-between gap-2.5 px-4 text-sm/[1.6] font-[650] text-fg [&::-webkit-details-marker]:hidden"
                ),
                "On this page",
                span![class("text-muted transition-transform group-open:rotate-90"), raw_html(ARROW_RIGHT)],
            ],
            nav![
                class("border-t border-line px-2 py-2"),
                aria("label", "On this page"),
                ul![each(items, |h| li![a![
                    class(
                        "flex min-h-11 items-center rounded-lg px-2 text-sm/[1.45] text-muted transition-colors hover:text-fg"
                    ),
                    class(("pl-5", h.level == 3)),
                    href(format!("#{}", h.id)),
                    h.text
                ]])],
            ],
        ]
    })
}

/// A documentation page inside the docs shell: article, pager and outline.
pub fn doc_page(doc: &'static Doc) -> impl View {
    let headings = docs::headings(doc.html());
    let section = docs::section_of(doc.slug).map(|s| s.title).unwrap_or("Docs");
    fragment![
        main![
            class(DOC_MAIN),
            id("content"),
            div![
                class(DOC_COLUMN),
                article![
                    p![class(EYEBROW), section],
                    h1![class(DOC_TITLE), class(MEASURE), doc.title],
                    p![class("mt-3.5 text-lg/[1.6] text-muted"), class(MEASURE), doc.description],
                    p![
                        class("mt-3 text-[0.9375rem]/[1.65] text-fg-soft"),
                        class(MEASURE),
                        strong![class("font-[650] text-fg"), "When you need it. "],
                        doc.when
                    ],
                    outline_inline(&headings),
                    div![class(PROSE), class(SYNTAX), raw_html(crate::highlight::code_blocks(doc.html()))],
                ],
                div![
                    class("mt-14 border-t border-line pt-5"),
                    a![
                        class("inline-flex min-h-11 items-center gap-2 text-sm/[1.6] text-muted hover:text-fg"),
                        href(format!("{REPO}/blob/main/{}", doc.source)),
                        target("_blank"),
                        rel("noopener"),
                        raw_html(EDIT_ICON),
                        "Edit this page on GitHub",
                    ],
                ],
                pager(doc),
            ],
        ],
        outline(&headings),
    ]
}
