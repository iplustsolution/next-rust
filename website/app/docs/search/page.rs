use next_rust::prelude::*;
use crate::{docs, ui};

#[derive(serde::Deserialize, Default)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

pub fn metadata() -> Metadata {
    Metadata::new().title("Search")
}

pub fn Page(Query(query): Query<SearchQuery>) -> impl View {
    let q = query.q.trim().to_owned();
    let hits = docs::search(&q);
    let count = hits.len();
    fragment![
        main![
            class(ui::DOC_MAIN),
            class("mx-auto w-full max-w-[880px]"),
            id("content"),
            p![class(ui::EYEBROW), "Search"],
            h1![
                class(ui::DOC_TITLE),
                if q.is_empty() { "Search the docs".to_owned() } else { format!("Results for “{q}”") }
            ],
            form![
                class("mt-7 mb-2 flex gap-2.5 max-[640px]:flex-wrap"),
                method("get"),
                action("/docs/search"),
                role("search"),
                input![
                    class("h-12 min-w-0 flex-1 rounded-xl border border-line-strong bg-soft px-4 text-base text-fg transition placeholder:text-muted focus:border-accent focus:ring-3 focus:ring-accent/15 focus:outline-none"),
                    r#type("search"),
                    name("q"),
                    value(&q),
                    placeholder("Try “layouts”, “server actions” or “deploy”"),
                    aria("label", "Search documentation"),
                    autofocus(q.is_empty()),
                ],
                button![
                    class("h-12 cursor-pointer rounded-xl bg-accent px-[22px] text-[15px] font-semibold text-on-accent max-[640px]:w-full"),
                    r#type("submit"),
                    "Search"
                ],
            ],
            (!q.is_empty()).then(|| p![
                class("mt-3.5 text-sm/[1.6] text-muted"),
                match count {
                    0 => "Nothing matched. Try fewer or different words.".to_owned(),
                    1 => "1 result".to_owned(),
                    n => format!("{n} results"),
                }
            ]),
            ul![
                class("mt-5 space-y-2.5"),
                each(hits, |hit| {
                    let url = match &hit.heading {
                        Some(h) => format!("{}#{}", docs::href(hit.doc), h.id),
                        None => docs::href(hit.doc),
                    };
                    li![a![
                        class("block rounded-xl border border-line px-[18px] py-4 transition-colors hover:border-line-strong hover:bg-soft"),
                        href(url),
                        span![
                            class("font-semibold text-fg"),
                            hit.doc.title,
                            hit.heading.as_ref().map(|h| span![class("font-medium text-accent-fg"), " › ", h.text.clone()])
                        ],
                        p![class("mt-1.5 text-[14.5px]/[1.6] text-muted"), hit.snippet],
                    ]]
                }),
            ],
        ],
        aside![class(ui::OUTLINE)],
    ]
}
