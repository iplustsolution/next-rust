//! The "caching" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "A cache is a photocopy. Drawing a page, or asking a slow database the same question again, costs time; \
             keeping the last answer and handing out copies costs almost nothing. The only hard part is knowing when \
             the copy is out of date, so this page is mostly about how long a copy may be used and how to throw it \
             away the moment the original changes."
        ),
        p!["Nothing on this page is required. With no configuration the framework keeps its copies in the server's \
             memory, which is right for one server. You come here when you want to cache your own data, clear a copy \
             on demand, or share copies between several servers."],
        p!["Next Rust has two caches:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["cache"], th!["stores"], th!["default store"]]],
                tbody![
                    tr![
                        td![strong!["page store"]],
                        td!["HTML of static pages (pre-rendered at startup + ISR)"],
                        td!["memory"],
                    ],
                    tr![td![strong!["data cache"]], td!["results of ", code!["next_rust::cache(..)"]], td!["memory"],],
                ],
            ],
        ],
        p![
            "Both use the same pluggable abstraction from ",
            code!["next-rust-cache"],
            ". \"Pluggable\" means the place the copies are kept (the ",
            em!["store"],
            ") can be swapped: memory, a folder on disk, or something you write yourself such as Redis.",
        ],
        p!["Words used on this page:"],
        ul![
            li![
                strong!["Static page"],
                ": a page rendered once and reused for every visitor, as opposed to a dynamic page that is rendered \
                 again for each request. See ",
                a![href("/docs/rendering#rendering-modes"), "Rendering modes"],
                ".",
            ],
            li![
                strong!["ISR"],
                " (incremental static regeneration): a static page that is re-rendered in the background once its \
                 copy is older than a number of seconds you choose (",
                code!["REVALIDATE"],
                ").",
            ],
            li![
                strong!["Revalidate"],
                ": check whether a copy is still good and, if not, make a new one. ",
                strong!["Invalidate"],
                ": throw a copy away now.",
            ],
            li![
                strong!["Tag"],
                ": a label you put on copies (",
                code!["\"products\""],
                ") so that one call can throw away every copy carrying it.",
            ],
            li![strong!["Stale"], ": a copy that is older than its allowed lifetime but still exists."],
        ],
        h2![id("data-cache"), a![class("anchor"), href("#data-cache"), "Data cache"]],
        p![
            "The data cache remembers the result of a slow call: a database query, a request to another service. \
             Give it a key (the name of the copy), how long the copy is good for, and the function that produces the \
             value. The function only runs when there is no fresh copy. This example goes in a page file such as ",
            code!["app/products/page.rs"],
            " and keeps the featured products for 300 seconds:",
        ],
        pre![code![
            class("language-rust"),
            r#"use next_rust::{cache, CacheOptions};

pub async fn Page() -> Result<impl View> {
    let products: Vec<Product> = cache(
        "products:featured",
        CacheOptions::revalidate(300).tag("products"),
        || async { api::featured_products().await },
    )
    .await?;
    Ok(ProductGrid(products))
}"#,
        ],],
        ul![
            li![
                "Values are stored as JSON, so they must implement ",
                code!["Serialize"],
                " + ",
                code!["Deserialize"],
                ".",
            ],
            li!["A stale entry (older than ", code!["revalidate"], ") is recomputed inline. Errors aren't cached.",],
            li![
                code!["revalidate_tag(\"products\")"],
                " removes every entry with the tag, in both the data cache and the page store.",
            ],
            li![code!["next_rust::invalidate(\"products:featured\")"], " removes one key."],
        ],
        h2![id("invalidation-api"), a![class("anchor"), href("#invalidation-api"), "Invalidation API"]],
        p!["Waiting for a copy to expire is fine for a news ticker and wrong for a price you just changed. These \
             calls throw copies away at once; the next visitor gets a freshly rendered page. They can be written \
             anywhere server code runs:",],
        pre![code![
            class("language-rust"),
            r#"next_rust::revalidate_path("/products").await;   // page store (+ data key of the same name)
next_rust::revalidate_tag("products").await;     // everything tagged
app.revalidate_path("/products").await;          // instance API, returns whether something was removed"#,
        ],],
        p![
            "Call these from API routes, server actions, jobs or webhooks. For example, a CMS (the editing tool \
             your writers use) can call a URL when someone presses Publish. This file, ",
            code!["app/api/revalidate/route.rs"],
            ", answers that call: it checks a shared secret so strangers cannot empty your cache, then clears \
             everything tagged ",
            code!["posts"],
            ":",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/api/revalidate/route.rs
pub async fn POST(req: Request) -> Result<Response> {
    if req.header("x-secret") != Some(&std::env::var("REVALIDATE_SECRET")?) {
        return Err(Error::http(401, "unauthorized"));
    }
    next_rust::revalidate_tag("posts").await;
    Ok(Response::json(&serde_json::json!({ "ok": true })))
}"#,
        ],],
        h2![id("traits"), a![class("anchor"), href("#traits"), "Traits"]],
        p![
            "This section is for people writing their own store. A store is anything that can save, fetch and \
             delete entries; the framework describes that contract as a Rust trait (from the ",
            code!["next-rust-cache"],
            " crate), and everything else is built on top of it:",
        ],
        pre![code![
            class("language-rust"),
            r"pub trait CacheStore: Send + Sync + 'static {
    fn get<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>>;
    fn set(&self, key: CacheKey, entry: CacheEntry) -> BoxFuture<'_, Result<(), CacheError>>;
    fn delete<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<bool, CacheError>>;
    fn delete_tag<'a>(&'a self, tag: &'a str) -> BoxFuture<'a, Result<usize, CacheError>>;
    fn clear(&self) -> BoxFuture<'_, Result<(), CacheError>>;
}",
        ],],
        ul![
            li![code!["CacheKey"], ": namespaced string (", code!["page:/blog/x"], ", ", code!["data:key"], ").",],
            li![
                code!["CacheEntry"],
                ": ",
                code!["value"],
                " bytes, ",
                code!["created"],
                ", ",
                code!["revalidate"],
                ", ",
                code!["tags"],
                ", ",
                code!["meta"],
                ". ",
                code!["is_stale()"],
                " reports whether it has expired.",
            ],
            li![
                code!["Cache"],
                ": the typed API over any store (",
                code!["get_or_insert_json"],
                ", ",
                code!["revalidate_path"],
                ", ",
                code!["revalidate_tag"],
                ").",
            ],
        ],
        p!["Built-in stores:"],
        ul![
            li![code!["MemoryStore::new(max_entries)"], ": bounded LRU."],
            li![
                code!["FileStore::new(dir)"],
                ": atomic writes (temp file plus rename). Tag invalidation scans metadata files, so it suits single nodes.",
            ],
        ],
        h2![id("custom-stores-redis"), a![class("anchor"), href("#custom-stores-redis"), "Custom stores (Redis, …)"],],
        p![
            "Redis isn't required or bundled. To share caches across instances, implement ",
            code!["CacheStore"],
            " on top of your client of choice:",
        ],
        pre![code![
            class("language-rust"),
            r#"struct RedisStore { client: redis::aio::ConnectionManager }

impl CacheStore for RedisStore {
    fn get<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let mut conn = self.client.clone();
            let bytes: Option<Vec<u8>> = redis::cmd("GET").arg(key.as_str()).query_async(&mut conn).await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(bytes.map(decode_entry))
        })
    }
    // set: SET + SADD tag:<tag> key; delete_tag: SMEMBERS + DEL …
}"#,
        ],],
        p![
            "Install it where the app is built, in ",
            code!["src/main.rs"],
            " (see ",
            a![href("/docs/middleware#global-middleware"), "a custom main"],
            "). ",
            code!["page_store"],
            " replaces where rendered pages are kept, ",
            code!["cache"],
            " replaces where ",
            code!["cache(..)"],
            " values are kept:",
        ],
        pre![code![
            class("language-rust"),
            r"App::new(routes())
    .page_store(RedisStore::new(url).await?)
    .cache(Cache::new(RedisStore::new(url).await?))",
        ],],
        h2![id("http-caching"), a![class("anchor"), href("#http-caching"), "HTTP caching"]],
        p![
            "The two caches above live on your server. Browsers and CDNs (content delivery networks: servers close \
             to the visitor that keep copies of your responses) keep copies too, and the ",
            code!["Cache-Control"],
            " response header tells them how long they may. The framework sets it for you:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["response"], th![code!["Cache-Control"]]]],
                tbody![
                    tr![td!["dynamic page"], td![code!["private, no-cache, no-store, max-age=0, must-revalidate"]],],
                    tr![td!["static page without revalidate"], td![code!["public, max-age=0, must-revalidate"]],],
                    tr![
                        td!["static page with revalidate"],
                        td![code!["public, max-age=0, s-maxage=<n>, stale-while-revalidate"]],
                    ],
                    tr![
                        td![code!["/_next-rust/assets/*"], " (fingerprinted, matching hash)"],
                        td![code!["public, max-age=31536000, immutable"]],
                    ],
                    tr![
                        td![code!["/_next-rust/runtime.js?v=<hash>"]],
                        td![code!["public, max-age=31536000, immutable"]],
                    ],
                    tr![
                        td![code!["public/"], " files"],
                        td![code!["public, max-age=<assets.public_max_age>"], " + ETag"],
                    ],
                ],
            ],
        ],
        p![code!["x-nr-cache: HIT | MISS | STALE | BYPASS"], " shows how a static page was served."],
        p![
            "Files under ",
            code!["/_next-rust/assets/"],
            " can be cached for a year because their URL contains a hash, a short fingerprint of the file's bytes: \
             change the file and the URL changes with it, so an old copy is never served by mistake.",
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        p![
            "Ask for a static page twice with ",
            code!["curl"],
            " and read the header. The commands run in a terminal while the app is running:",
        ],
        pre![code![
            class("language-sh"),
            r"curl -sI http://localhost:3000/blog | grep -i x-nr-cache
# x-nr-cache: MISS     first request: rendered now, copy stored
# x-nr-cache: HIT      later requests: the copy
# x-nr-cache: STALE    older than REVALIDATE: the copy, while a new one is rendered in the background
# x-nr-cache: BYPASS   rendered but not stored: the page answered with a status other than 200",
        ],],
        p![
            "No ",
            code!["x-nr-cache"],
            " header at all means the page is dynamic and is never stored. ",
            code!["next-rust routes --layouts"],
            " says why a page is dynamic.",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["Caching per-visitor data under a shared key."],
                " A key like ",
                code!["\"cart\""],
                " is one copy for everybody. Put the user or session id in the key, or do not cache it.",
            ],
            li![
                strong!["A value that is not JSON."],
                " Cached values are stored as JSON; a type without ",
                code!["Serialize"],
                " and ",
                code!["Deserialize"],
                " does not compile.",
            ],
            li![
                strong!["Expecting errors to be remembered."],
                " A failed call is not cached, so a broken upstream service is asked again on every request. Add \
                 your own fallback if that matters.",
            ],
            li![
                strong!["Several servers, memory store."],
                " Each server keeps its own copies, so ",
                code!["revalidate_tag"],
                " on one does not clear the others. Use a shared store (above) when you run more than one instance.",
            ],
            li![
                strong!["An unprotected revalidation URL."],
                " Anyone who can call it can make your server re-render everything. Check a secret, as in the \
                 example above.",
            ],
        ],
    ]
}
