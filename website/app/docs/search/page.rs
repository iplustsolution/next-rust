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
                class("doc search-page"),
                id("content"),
                p![class("eyebrow"), "Search"],
                h1![if q.is_empty() { "Search the docs".to_owned() } else { format!("Results for “{q}”") }],
                form![
                    class("search big"),
                    method("get"),
                    action("/docs/search"),
                    role("search"),
                    input![
                        r#type("search"),
                        name("q"),
                        value(&q),
                        placeholder("Try “layouts”, “server actions” or “deploy”"),
                        aria("label", "Search documentation"),
                        autofocus(q.is_empty()),
                    ],
                    button![r#type("submit"), "Search"],
                ],
                (!q.is_empty()).then(|| p![
                    class("result-count"),
                    match count {
                        0 => "Nothing matched. Try fewer or different words.".to_owned(),
                        1 => "1 result".to_owned(),
                        n => format!("{n} results"),
                    }
                ]),
                ul![
                    class("results"),
                    each(hits, |hit| {
                        let url = match &hit.heading {
                            Some(h) => format!("{}#{}", docs::href(hit.doc), h.id),
                            None => docs::href(hit.doc),
                        };
                        li![a![
                            href(url),
                            span![class("result-path"), hit.doc.title, hit.heading.as_ref().map(|h| span![" › ", h.text.clone()])],
                            p![hit.snippet],
                        ]]
                    }),
                ],
            ],
            aside![class("outline")],
    ]
}
