# Rendering & data

## Rendering modes

| mode | when HTML is produced | set with |
|---|---|---|
| `static` | at build time (`next-rust build`), or on first request, then cached | `pub const RENDERING: Rendering = Rendering::Static;` |
| `dynamic` | on every request | `pub const RENDERING: Rendering = Rendering::Dynamic;` |
| `auto` (default) | decided by the build | nothing |

With `auto`, a page is **static** unless one of these is true:

- it, a layout or template above it, a slot page, or a `metadata`/`load`
  function on its path takes a request-bound argument (`Cookies`, `Headers`,
  `Query`, `Extension`, `Auth`, `FormState`, …, see
  [routing](routing.md#pages));
- it has a dynamic segment and no `generate_params`;
- it is an intercepting route.

`next-rust routes --layouts` and `next-rust build` print the reason a route
is dynamic. Change the project default with `[rendering] default`.

If a static page reads request data anyway (for example, one forced to
`static` that takes `Cookies`), rendering fails with a clear error instead of
silently leaking one user's data into a shared cache.

In development (`next-rust dev`) every page renders per request, so changes
show up immediately.

## Server-side rendering

Dynamic pages render on each request. The response is `text/html`, marked
`private, no-store`, with security headers and a per-request CSP nonce. Page
functions can be `async` and await databases or APIs directly:

```rust
pub async fn Page(Extension(db): Extension<Db>, Path(p): Path<P>) -> Result<impl View> {
    let order = db.order(p.id).await?.or_not_found()?;
    Ok(article![h1![format!("Order {}", order.id)], OrderLines(&order)])
}
```

### `load` and `Data<T>`

To keep data access separate from markup, export `load` and take `Data<T>`:

```rust
pub async fn load(Path(p): Path<P>) -> Result<Vec<Post>> {
    posts::by_author(&p.author).await
}

pub fn Page(Data(posts): Data<Vec<Post>>) -> impl View {
    ul![each(posts, |p| li![p.title])]
}
```

The type connection is checked at compile time. The generated code passes
`load`'s result straight into `Page`. Layouts can use `load` the same way.

## Static generation (SSG)

`next-rust build` compiles the release binary and runs it with `--export`,
which pre-renders every static page into:

- `.next-rust/cache/pages/` – the page cache the production server reads;
- `.next-rust/static/**/index.html` – plain HTML files, usable with any static host.

### `generate_params`

A dynamic route is static when it lists its parameters:

```rust
// app/blog/[slug]/page.rs
pub async fn generate_params() -> Vec<Params> {
    posts::all().await.iter().map(|p| Params::new().with("slug", &p.slug)).collect()
}

/// false: slugs not in the list render the 404 page.
/// true (default): they are rendered on first request, then cached.
pub const DYNAMIC_PARAMS: bool = false;
```

`generate_params` may return `Result<Vec<Params>>`. Returning an error fails
the build.

## Incremental static regeneration (ISR)

```rust
pub const REVALIDATE: u64 = 60;           // seconds
pub const TAGS: &[&str] = &["products"];  // for revalidate_tag
```

- Before the interval passes, the cached HTML is served (`x-nr-cache: HIT`).
- After it, the stale page is still served (`STALE`) while one background
  task re-renders it. Concurrent requests don't start more renders.
- A failed regeneration keeps serving the stale page and logs the error.

On-demand revalidation from an API route, server action or job:

```rust
next_rust::revalidate_path("/blog/hello").await;  // one URL
next_rust::revalidate_tag("products").await;      // every page and data entry with the tag
```

Responses carry `cache-control: public, max-age=0, s-maxage=<revalidate>,
stale-while-revalidate`, so a CDN can cache them too. Set a project-wide
default with `[rendering] revalidate = 300`.

Where cached pages are stored is pluggable. See [caching](caching.md).

## Streaming

A slow page doesn't have to block the whole response. With streaming enabled
(the default), the server sends the **shell** (layouts, fallbacks, everything
that doesn't wait) as soon as it's ready, then streams each suspended part
when it resolves.

### `loading.rs`

```rust
// app/dashboard/loading.rs
pub fn Loading() -> impl View { p![aria("busy", "true"), "Loading dashboard…"] }
```

It wraps everything below the dashboard layout in a suspense boundary. The
layout and the fallback arrive immediately, and the page follows.

### `suspense`

Suspend any async component:

```rust
async fn Weather() -> impl View { let w = api::weather().await; p![w.summary] }

pub fn Page() -> impl View {
    div![
        h1!["Today"],
        suspense(p!["Loading weather…"], Weather()),
        suspense(p!["Loading news…"], News()),
    ]
}
```

Boundaries resolve concurrently and are flushed **in completion order**.
Each resolution is a `<template>` chunk plus a ~60-byte script call that moves
it into place. The script is inlined once, only on pages that actually
suspend, and carries the CSP nonce.

Streaming facts:

- Streaming applies to **dynamic** rendering. Static pages are rendered
  completely at build or revalidation time.
- Crawlers (user agents containing `bot`, `spider`, `crawler`, …) get fully
  rendered HTML in one response.
- Without JavaScript, fallbacks stay visible. Avoid suspense for content that
  must work without JavaScript.
- The status code and headers are sent with the shell. A `not_found()` or
  error thrown inside streamed content renders the nearest boundary in place.
  A redirect becomes a client-side `location.replace`.
- Disable streaming with `[rendering] streaming = false`.

## Errors

```rust
pub async fn Page(Path(p): Path<P>) -> Result<impl View> {
    let order = db::order(p.id).await?;        // any std::error::Error works with `?`
    Ok(OrderView(order))
}
```

- `error.rs` catches errors from the pages and layouts **below** its segment,
  including the page in the same directory, but not errors from its own
  segment's `layout.rs`. The layouts above it still render.
- `ErrorInfo { status, message, digest }`: `message` is the full error in
  development and a generic message in production. `digest` is written to the
  server log so you can correlate reports.
- Without a boundary, `global-error.rs` or the built-in error page renders
  with status 500. In development the built-in page shows the error and the
  source file.
- Errors never crash the process. Each request is isolated.

## Not found

```rust
return Err(not_found());                         // or:
let user = db::user(id).await?.or_not_found()?;  // Option → 404
```

`not_found()` renders the nearest `not-found.rs` inside the layouts above it,
with status 404. Unmatched URLs render the root `app/not-found.rs`, or a
built-in page. Requests that don't accept HTML, or that fall under the API
prefix, get a plain `404 Not Found`.

## Redirects

```rust
return Err(redirect("/login"));             // 307
return Err(permanent_redirect("/new-url")); // 308
```

These work in pages, layouts, `metadata`, `load` and server actions. They're
also available as `Response::redirect(..)` in middleware and API routes, and
through configuration:

```toml
[[redirects]]
source = "/old-blog/:slug"
destination = "/blog/:slug"
permanent = true
```

Trailing slashes are normalized with a 308. `/about/` redirects to `/about`
unless you set `[app] trailing_slash = true`.

## Metadata

Export `metadata` from `layout.rs`, `page.rs` or `metadata.rs`:

```rust
pub fn metadata() -> Metadata {
    Metadata::new()
        .title("Acme")
        .title_template("%s · Acme")          // applied to titles below this segment
        .description("Tools for builders")
        .keywords(["tools", "rust"])
        .canonical("https://acme.dev/")
        .theme_color("#111827")
        .icon("/favicon.ico")
        .manifest("/site.webmanifest")
        .open_graph(OpenGraph {
            site_name: Some("Acme".into()),
            images: vec![OgImage { url: "https://acme.dev/og.png".into(), width: Some(1200), height: Some(630), ..Default::default() }],
            ..Default::default()
        })
        .twitter(Twitter { card: Some("summary_large_image".into()), ..Default::default() })
}
```

Metadata is merged from the root layout down to the page. Every field set by
a child overrides the parent. A title set below a `title_template` is
formatted with it. `absolute_title(..)` ignores templates. Metadata functions
can be `async`, take extractors, and return `Result`. `not_found()` from
metadata renders the 404 page.

All values are HTML-escaped. `canonical` and link URLs with a `javascript:`
scheme are neutralized.

### Generated SEO files

```rust
// app/sitemap.rs
pub async fn sitemap() -> Sitemap {
    Sitemap::new().url("https://acme.dev/").url("https://acme.dev/pricing")
}

// app/robots.rs
pub fn robots() -> Robots {
    Robots::allow_all().sitemap("https://acme.dev/sitemap.xml")
}
```

Static files like `public/robots.txt`, `public/favicon.ico` and
`public/og.png` work too.
