//! The "caching" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
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
        p!["Both use the same pluggable abstraction from ", code!["next-rust-cache"], "."],
        h2![id("data-cache"), a![class("anchor"), href("#data-cache"), "Data cache"]],
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
        pre![code![
            class("language-rust"),
            r#"next_rust::revalidate_path("/products").await;   // page store (+ data key of the same name)
next_rust::revalidate_tag("products").await;     // everything tagged
app.revalidate_path("/products").await;          // instance API, returns whether something was removed"#,
        ],],
        p!["Call these from API routes, server actions, jobs or webhooks. For example, a CMS publishing hook:",],
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
        p!["Install it:"],
        pre![code![
            class("language-rust"),
            r"App::new(routes())
    .page_store(RedisStore::new(url).await?)
    .cache(Cache::new(RedisStore::new(url).await?))",
        ],],
        h2![id("http-caching"), a![class("anchor"), href("#http-caching"), "HTTP caching"]],
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
    ]
}
