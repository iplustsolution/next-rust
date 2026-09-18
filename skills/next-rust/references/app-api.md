# Application API reference

Verified against Next Rust 0.1.7. Signatures are copy-paste accurate. Search for what you need.

## Contents

- [Entry points and the prelude](#entry-points-and-the-prelude)
- [Special files](#special-files)
- [Route segment syntax](#route-segment-syntax)
- [The view DSL](#the-view-dsl)
- [Attributes](#attributes)
- [Data loading and extractors](#data-loading-and-extractors)
- [Errors and control flow](#errors-and-control-flow)
- [API routes: Request and Response](#api-routes-request-and-response)
- [Cookies, sessions, CSRF](#cookies-sessions-csrf)
- [Server actions and forms](#server-actions-and-forms)
- [UI components](#ui-components)
- [Client interactivity](#client-interactivity)
- [Metadata, sitemap, robots](#metadata-sitemap-robots)
- [Rendering modes, ISR and caching](#rendering-modes-isr-and-caching)
- [Middleware](#middleware)
- [Styling, assets and fonts](#styling-assets-and-fonts)
- [Testing](#testing)
- [Custom main, background jobs, SSE, WebSockets](#custom-main-background-jobs-sse-websockets)
- [Name traps](#name-traps)

## Entry points and the prelude

```rust
// build.rs
fn main() { next_rust_build::generate(); }

// src/main.rs
next_rust::app!();        // = routes!(); fn main() { next_rust::run(routes()) }
next_rust::routes!();     // just includes the generated `routes()` — use in src/lib.rs for tests
```

`use next_rust::prelude::*;` brings in the macros (`action`, `asset`, `client`, `css_module`, `global_css`,
`server`, `server_action`), `Params`, the server types (`Data`, `Request`, `Response`, `Json`, `Html`,
`Error`, `Result`, `Query`, `Path`, `Headers`, `Cookies`, `Cookie`, `SameSite`, `Session`, `Auth`, `AuthUser`,
`Extension`, `FormState`, `CsrfToken`, `Nonce`, `Ctx`, `Rendering`, `RequestInfo`, `ResponseHeaders`, `Next`,
`IntoResponse`, `OrNotFound`, `Sitemap`, `Robots`, `SseEvent`, `ActionContext`, `ErrorInfo`, `not_found`,
`redirect`, `permanent_redirect`), the metadata types and everything in `next_rust_view` (all element macros,
attribute helpers, `View`, `Node`, `Element`, `Children`, `Slots`, `Metadata`, `each`, `when`, `suspense`,
`raw_html`, `fragment!`, `Link!`, `Image!`).

## Special files

Every export must be `pub`. "sync or async" means either compiles; `Result<impl View, E>` is accepted wherever
`impl View` is.

| File | Required export | Notes |
| --- | --- | --- |
| `page.rs` | `pub fn Page(..) -> impl View` | sync or async |
| `page.html` | — | A fragment is wrapped in layouts; a full document bypasses them. Both `page.rs` and `page.html` in one directory is `NR0101` unless `[app] html_precedence` says which wins. |
| `layout.rs` | `pub fn Layout(children: Children, ..)` | sync or async; may also take `Slots` |
| `template.rs` | `pub fn Template(children: Children, ..)` | like a layout but re-created per navigation |
| `loading.rs` | `pub fn Loading(..) -> impl View` | **must be sync** |
| `error.rs` | `pub fn ErrorBoundary(info: ErrorInfo, ..) -> impl View` | **must be sync**; catches errors below its segment, not its own `layout.rs` |
| `global-error.rs` | `pub fn GlobalError(info: ErrorInfo, ..) -> impl View` | app root only, **must be sync** |
| `not-found.rs` | `pub fn NotFound(..) -> impl View` | sync or async |
| `route.rs` | one or more of `GET HEAD POST PUT PATCH DELETE OPTIONS` | each takes nothing or one `Request` |
| `middleware.rs` | `pub async fn middleware(req: Request, next: Next) -> Response` | signature is exactly `(Request, Next)` |
| `metadata.rs` | `pub fn metadata(..) -> Metadata` | sync or async, may return `Result<Metadata>`; can also be exported from `layout.rs`/`page.rs`, where `metadata.rs` wins for that segment |
| `default.rs` | `pub fn Page(..)` | fallback for an unmatched parallel slot |
| `sitemap.rs` | `pub fn sitemap() -> Sitemap` | app root only |
| `robots.rs` | `pub fn robots() -> Robots` | app root only |

`ErrorInfo { status: u16, message: String, digest: String }`.

Per-page constants (read at build time, must be literals):

```rust
pub const RENDERING: Rendering = Rendering::Static;   // Static | Dynamic | Auto (default)
pub const REVALIDATE: u64 = 60;
pub const DYNAMIC_PARAMS: bool = false;               // default true: allow params outside generate_params
pub const TAGS: &[&str] = &["products"];
```

Files under `app/` are compiled as modules of your crate (absolute `#[path]` includes), so refer to your own
code as `crate::db::posts()` — the crate's own name does not resolve inside them.
Other `.rs` files in `app/` are ignored, so helper modules are fine. Directories starting with `_` or `.` are
never routed. Argument placement is checked at build time (`NR0206`): `Children` only in `Layout`/`Template`,
`Slots` only in `Layout`, `ErrorInfo` only in the error boundaries, `Data<T>` only where a `load` exists in
the same file.

## Route segment syntax

| Directory | Meaning |
| --- | --- |
| `about` | static segment |
| `[id]` | one segment → `params.get("id")` |
| `[...slug]` | one or more segments → `params.get_all("slug")` |
| `[[...slug]]` | zero or more segments |
| `(marketing)` | group: organises files and layouts, not part of the URL |
| `@analytics` | parallel slot, rendered by the parent layout via `slots.take("analytics")` |
| `(.)photo`, `(..)photo`, `(..)(..)photo`, `(...)photo` | intercepting routes: same level, one up, two up, from the root |
| `_components`, `.private` | ignored |

Parameter and slot names must be identifiers. Interception only applies to client-side navigations (the
runtime sends `x-nr-nav: 1` and `x-nr-from`); a direct load renders the normal page. Ranking: static beats
dynamic beats catch-all beats optional catch-all.

## The view DSL

```rust
pub trait View { fn into_node(self) -> Node; }
pub enum Node { Empty, Text(..), Raw(..), Element(..), Fragment(..), Suspense(..), Style(..) }
```

`View` is implemented for `Node`, `Element`, string types, `char`, all numbers, `Option<V>`, `Vec<V>`,
`[V; N]`, `Box<V>`, `()`, tuples up to 8, and `&'static Stylesheet`. So `Option<Node>` renders nothing when
`None`, and a `Vec` renders in order — no need for a wrapper.

Element macros exist for 121 tags (`next_rust::TAGS` lists them), named exactly after the tag, including SVG
(`svg`, `path`, `circle`, `linearGradient`, …). Void tags ignore children. The only renamed macro is `use_!`
(`use` is a keyword) — every other tag is itself: `time!`, `form!`, `label!`, `select!`, `style!`. `Element::custom("my-tag")` builds a tag
name at runtime.

```rust
fragment![a, b]                       // several nodes without a wrapper element
each(items, |i| li![i.name])          // iterate
when(flag, || p!["shown"])            // conditional
raw_html(html_string)                 // unescaped: never user input
suspense(p!["Loading…"], async { slow().await })   // streams the fallback first
island("Counter", props_json, ssr)    // what #[client] expands to
```

Streaming requires `[rendering] streaming = true` (default). Rendering outside a request:
`render_static(view)` (sync, suspense renders its fallback) and `render_to_string(view).await`.

## Attributes

```rust
class("a b")                     // repeated class(..) parts merge
class(["a", "b"])                // arrays
class(("active", is_active))     // conditional
class(styles.card_title)         // CSS module class
attr("data-x", "1")              // rejects on* handlers
raw_attr("onclick", "…")         // allows them
data("id", "42")  aria("label", "Close")  key(item.id)
attr_if(cond, disabled(true))
```

Text helpers: `id href src srcset sizes alt title style name value placeholder method action enctype rel
target width height lang dir role tabindex content charset http_equiv min max step pattern minlength maxlength
autocomplete inputmode colspan rowspan loading decoding fetchpriority download crossorigin integrity
referrerpolicy datetime form_attr list accept rows cols wrap label_attr media as_ nonce slot_attr translate
spellcheck enterkeyhint view_box fill stroke d xmlns`, plus `type_`/`r#type`, `for_`/`r#for`.

Boolean helpers: `disabled checked selected required readonly multiple hidden autofocus open defer novalidate
controls autoplay muted playsinline inert ismap reversed nomodule`, plus `r#async`.

Navigation helpers: `prefetch(false)`, `replace(true)`, `reload(true)`, `scroll(false)`,
`active_class("active")`, `active_class_prefix("active")`.

```rust
Link!(href = "/about", class = "nav", prefetch = false, "About")   // any attrs::* helper works as key = value
Image!(src = "/lake.jpg", width = 1200, height = 800, alt = "A lake")
```

`Image!` emits `srcset`/`sizes`, `loading="lazy"` (unless `priority = true`) and routes local images through
`/_nr/image`. Image resizing is not implemented yet: the original file is served.

## Data loading and extractors

```rust
pub async fn load(/* extractors */) -> Result<T>   // must return Result, same file as the consumer
pub fn Page(Data(data): Data<T>) -> impl View
pub async fn generate_params() -> Vec<Params>      // no arguments; Result<Vec<Params>> also accepted
```

`Params`: `new()`, `with(name, value)`, `insert`, `get(&str) -> Option<&str>` (borrowed, so
`params.get("slug").unwrap_or_default()` gives a `&str`),
`get_all(&str) -> Option<&[String]>`, `value`, `iter`, `len`, `is_empty`.

| Extractor | What you get | Static-safe |
| --- | --- | --- |
| `Params` | route parameters | yes |
| `Path<T>` | deserialized parameters; failure → 404 | yes |
| `Nonce` | CSP nonce for inline `<script>` | yes |
| `Data<T>` | the result of `load` | yes |
| `Query<T>` / `Query` | query string (`QueryMap` by default); bad input → 400 | no |
| `Headers` | `.get(name)` | no |
| `Cookies` | read/write cookies | no |
| `RequestInfo` | method, uri, remote address | no |
| `Extension<T>` | value inserted by middleware | no |
| `Auth<T>` | `.user()`, `.is_authenticated()`, `.require("/login")?` | no |
| `ResponseHeaders` | `.set(name, value)` on the response | no |
| `CsrfToken` | `.field()` renders the hidden input | no |
| `FormState` | `.error(field)`, `.value(field)`, `.has_errors()` after a no-JS submit | no |
| `Ctx` | the whole `RequestContext` (`path()`, `set_status(code)`, `params`, `config`, `dev`, …) | no |

Anything not static-safe makes the page dynamic under `auto`. `next-rust routes --layouts` prints the reason a
route is dynamic — including when a *layout* above the page is what forced it.

## Errors and control flow

```rust
pub type Result<T, E = Error> = std::result::Result<T, E>;

not_found()                                   // 404 → nearest not-found.rs
redirect("/login")                            // 307
permanent_redirect("/new")                    // 308
Error::validation([("email", "Invalid")])     // 422 with field errors
Error::http(409, "Already exists")
Error::msg("something went wrong")            // 500
option.or_not_found()?                        // Option<T> → Result<T>
```

`Error` implements `From<E>` for any `std::error::Error`, so `?` works on database and IO errors. Status
mapping: not found → 404, validation → 422, redirect → 307/308, internal → 500. Inspect with `kind()`,
`status()`, `is_not_found()`, `is_redirect()`, `public_message()`.

## API routes: Request and Response

```rust
// Request
req.method() req.uri() req.path() req.query_string() req.query::<T>()? req.query_param("q")
req.headers() req.header("x-api-key") req.params() req.param("id")
req.cookies() req.remote_addr() req.is_secure() req.is_websocket_upgrade()
req.extension::<T>() req.insert_extension(value) req.set_path("/rewritten")?  // middleware rewrite
req.bytes().await? req.text().await? req.json::<T>().await? req.form::<T>().await?
req.set_body_limit(10 * 1024 * 1024)          // default [server] body_limit = 2 MiB, over-limit → 413
```

```rust
// Response
Response::text("hi") Response::html(markup) Response::json(&value) Response::bytes(b)
Response::status(204) Response::not_found() Response::stream(s)
Response::redirect("/x")        // 307
Response::permanent_redirect("/x")  // 308
Response::see_other("/x")       // 303
Response::sse(events) Response::sse_with_keep_alive(events, Duration::from_secs(15))
    .with_status(201).with_header("x-total", "9").with_content_type("text/csv")
    .with_cache_control("public, max-age=60").with_cookie(cookie)
```

`IntoResponse` is implemented for `Response`, `Json<T>`, `Html<T>`, `String`, `&'static str`, `Bytes`,
`Vec<u8>`, `StatusCode`, `(StatusCode, T)`, `()` → 204 and `Result<T, E: Into<Error>>`.

## Cookies, sessions, CSRF

```rust
cookies.set(Cookie::new("theme", "dark").max_age(Duration::from_secs(86400)).same_site(SameSite::Lax));
cookies.get("theme");  cookies.get_decoded("name");  cookies.delete("theme");
```

`Cookies::set` defaults to `Path=/`, `HttpOnly`, `SameSite=Lax`, and `Secure` outside development.
`Cookie::encoded(name, value)` percent-encodes; read it back with `get_decoded`.

Sessions are middleware plus an extension: `App::new(routes()).middleware(sessions(MemorySessionStore::new()))`
then `Extension<Session>` with `get`, `insert`, `remove`, `regenerate`, `destroy`. CSRF: `[security] csrf`
(`origin` by default), the `csrf()` middleware, and `CsrfToken.field()` for the hidden input.

## Server actions and forms

```rust
#[server_action] pub async fn refresh() -> Result<Stats>                       // 0 arguments
#[server_action] pub async fn create(input: NewUser) -> Result<User>           // 1 deserializable argument
#[server_action] pub async fn logout(ctx: ActionContext, input: ()) -> Result<()>   // context + input
```

At most two arguments (`NR0210`), no `self`, sync allowed. Discovered in `app/` and `src/`. Served at
`POST /_nr/action/<token>`; reference one from a view with `action!(path::to::fn)`, which sets `action`,
`method="post"` and `data-nr-action` on the `<form>`.

Action URLs are never fixed. `action!(f)` / `ActionRef::url()` render a placeholder, and every HTML response
replaces it with a token signed (HMAC-SHA256) under `NEXT_RUST_SECRET` (32+ bytes; set it in production, or
links break on restart and across instances), expiring after `[security] action_token_ttl`, and bound to the
visitor's `HttpOnly`, `SameSite=Strict` binding cookie (`__Host-nr_bind`, `nr_bind` in development). Copied,
expired, tampered or guessed URLs get 403 before the action runs. So: never build action URLs by hand, never
return `.url()` from a JSON API, and still check authentication/authorization inside the action (tokens prove
"this browser loaded a page", not "who the user is"). Pages with action URLs are sent `private, no-store`.
Only `application/json` and `application/x-www-form-urlencoded` bodies are accepted.

Without JavaScript: success redirects (303) to the `_redirect` field or the referrer; a validation error
redirects back with a 60-second flash cookie that the `FormState` extractor consumes once. Fields starting
with `_` or containing `password`/`token` are never echoed back.

With JavaScript: `{"ok":true,"data":…}` / `{"ok":false,"errors":{…}}` (422) / `{"ok":false,"redirect":"/…"}`.
Elements with `data-nr-error="field"` are filled in automatically; the form gets `aria-busy` while submitting
and dispatches `nr:success` / `nr:error`.

## UI components

`use next_rust::ui::*;` (crate `next-rust-ui`). Every component is a builder with a same-named macro:
`key = value` sets a property, anything else is added like in an element macro (`class(..)`, `id(..)`,
`aria(..)`, `data(..)`, children). Every property is optional.

```rust
Button![color = Color::Danger, variant = Variant::Bordered, size = Size::Sm, "Delete"]   // primary/solid/md by default
Button![submit = true, full_width = true, "Save"]    // type="submit"; `type="button"` otherwise
Button![href = "/x", "Go"]                           // a link (client-side navigation)
Button![on_press = action!(like), "Like"]            // or Press::from(action!(like)).input(&v).no_refresh(),
                                                     // Press::navigate("/x"), Press::emit("name"), Press::script("js")
Input![name = "email", label = "Email", kind = "email", value = form.value("email"), error_message = form.error("email")]
PasswordInput![name = "password", label = "Password", new_password = true]
Textarea![label = "Message", rows = 4]
Select![name = "plan", label = "Plan", placeholder = "Choose", SelectItem![value = "pro", description = "Teams", "Pro"]]
Select![multiple = true, values = ["a"], [("a", "A"), ("b", "B")]]
DatePicker![name = "day", label = "Day", min = "2026-01-01", max = "2026-12-31", first_day_of_week = 1]   // ISO values
Checkbox![name = "terms", required = true, "I agree"]   Switch![checked = true, "Wi-Fi"]
RadioGroup![name = "plan", value = "pro", Radio![value = "free", "Free"], Radio![value = "pro", "Pro"]]
Avatar![src = url, name = "Ada Lovelace"]   AvatarGroup![max = 3, total = 10, avatars…]
Card![CardHeader![..], CardBody![..], CardFooter![..]]   Chip![color = Color::Success, dot = true, "Online"]
Spinner![]   Divider![]   Container![width = Width::Lg, ..]   Stack![row = true, gap = 4, align = Align::Center, ..]
Grid![cols = 3, ..]   AppShell![navbar = Navbar![brand = .., menu_toggle = true, NavbarItem![href = "/", "Home"]],
                                sidebar = Sidebar![SidebarItem![href = "/", "Home"]], children]
```

Icons: every Lucide icon as `next_rust::icons::PascalName()` (`icons::ArrowRight()`, former names like
`icons::Home()` too), returning an `Icon` with `.size(24 | "1.25em")`, `.color(..)`, `.stroke_width(2.0)`,
`.absolute_stroke_width(true)`, `.fill(..)`, `.title(..)` (else `aria-hidden`), `.class(..)`, `.unstyled(true)`,
`.with(attr)`; `icons::by_name("house")` for names from data. Use them in `start_content`/`end_content`.

Common properties: `color` (`Color::{Default, Primary, Secondary, Success, Warning, Danger}`), `size`
(`Size::{Sm, Md, Lg}`), `radius` (`Radius::{None, Sm, Md, Lg, Full}`), `variant` (`Variant::…` for buttons and
chips, `FieldVariant::{Flat, Bordered, Faded, Underlined}` for fields), `label_placement`
(`LabelPlacement::{Inside, Outside, OutsideLeft}`), `class`, `unstyled`. Fields also take `label_class`,
`wrapper_class`, `input_class`, `description_class`, `error_class`; their non-class attributes go to the control.

Styling: default classes (`nr-*`) live in the CSS `components` layer, so any class you add (Tailwind or your own
CSS) wins without `!important`. Theme with CSS variables on `:root` (`--nr-primary`, `--nr-radius-md`,
`--nr-font`, ...); dark mode follows the system or `class="dark"`/`data-theme="dark"`. Fields with a `name`
render `data-nr-error=name`, so server-action validation errors appear under them. Interactive components load
`/_nr/ui.js` only on pages that use them and fall back to native controls without it. Select and date-picker
popovers render in the browser's top layer (never clipped by `overflow: hidden`).

## Client interactivity

```rust
#[client] pub fn Counter(count: i32) -> impl View { … }
#[client(module = "/_nr/client/chart.js")] pub fn Chart(points: Vec<f32>) -> impl View { … }
```

Not async, not generic, no methods, plain identifier arguments implementing `Serialize`. Props are visible in
the HTML. Files in `client/` are served from `/_nr/client/`; a module exports `hydrate(element, props)`.

Declarative bindings: `data-nr-text`, `data-nr-show` (`"!path"` negates), `data-nr-bind`,
`data-nr-class-<name>`, `data-nr-on-<event>` via `on("click", "…")`, `data-nr-key`, `data-nr-error`.
Operations: `increment:path[,n]`, `decrement:path[,n]`, `toggle:path`, `set:path=<json>`, `prevent`,
`navigate:/url`, `action:<url>[->path]`. There is no `eval`, so a strict CSP works.

```js
window.nextRust.navigate(href) / .replace(href) / .back() / .forward() / .refresh() / .prefetch(href)
await window.nextRust.action(url, input)     // throws with .errors and .status
window.nextRust.env                          // NEXT_RUST_PUBLIC_* variables
window.addEventListener("nr:navigate", e => e.detail.url)
```

The runtime (`/_nr/runtime.js`) is added only to pages with an internal link or an island. It prefetches after
400 ms of mouse hover, caches pages for 30 s, and swaps only the part of the page below shared layouts.

## Metadata, sitemap, robots

```rust
pub fn metadata() -> Metadata {
    Metadata::new()
        .title("Pricing")
        .title_template("%s | Acme")     // applies to descendants, not this segment
        .absolute_title("Acme")          // ignore an ancestor template
        .description("Plans and pricing")
        .keywords(["rust", "web"]).authors(["Ada"])
        .canonical("https://acme.dev/pricing")
        .robots("index, follow").viewport("width=device-width, initial-scale=1")
        .theme_color("#f26b2a").color_scheme("light dark").manifest("/manifest.json")
        .icon("/favicon.svg").apple_touch_icon("/apple-touch-icon.png")
        .open_graph(OpenGraph { title: Some("Pricing".into()), images: vec![OgImage { url: "/og.png".into(), ..Default::default() }], ..Default::default() })
        .twitter(Twitter { card: Some("summary_large_image".into()), ..Default::default() })
        .alternate("de", "https://acme.dev/de/pricing")
        .meta("x-custom", "1").stylesheet("/styles.css")
        .preload_font("/fonts/inter.woff2").preload_image("/hero.avif")
}
```

Layouts and pages merge outside-in; the child wins. `metadata` may be async, take extractors and return
`Result`, and it may take `Data<T>` — it then gets the same `load` call the page does. Returning
`not_found()` from it renders the 404 page — but note it renders the **app-root**
`not-found.rs`, not a per-segment one, because metadata is resolved before the page tree. If you want a
segment's own 404 page, raise `not_found()` from `load` or `Page` and keep `metadata` infallible.

```rust
// app/sitemap.rs → /sitemap.xml
pub async fn sitemap() -> Sitemap { Sitemap::new().url("https://acme.dev/").url("https://acme.dev/pricing") }
// app/robots.rs → /robots.txt
pub fn robots() -> Robots { Robots::allow_all().sitemap("https://acme.dev/sitemap.xml") }
```

`SitemapEntry { url, last_modified, change_frequency, priority }` and
`RobotsRule { user_agent, allow, disallow, crawl_delay }` are in the prelude for the detailed forms.

Feeds are a route, not a special file — `Feed` renders the same entries three ways:

```rust
// app/feed.xml/route.rs
pub async fn GET() -> Response {
    let feed = Feed::new("Acme", "https://acme.dev", "https://acme.dev/feed.xml")
        .description("Notes from the team")
        .language("en")
        .entry(
            FeedEntry::new(post.url(), post.title)
                .summary(post.summary)
                .content_html(html)         // full text
                .published("2026-09-18")    // YYYY-MM-DD or RFC 3339
                .tag("rust"),
        );
    Response::xml(feed.to_rss())            // .to_atom() | .to_json()
}
```

Advertise it with `Metadata::feed(title, href, mime)`; `Metadata::link(rel, href)` covers `preconnect`,
`me` and other link tags. `Response::xml` and `Response::svg` set the media types.

## Rendering modes, ISR and caching

`static` renders at build time (or first request) and caches; `dynamic` renders per request; `auto` (default)
decides at build time. `auto` picks dynamic when the page, any layout or template above it, a slot page, or a
`load`/`metadata` on the path takes a request-bound extractor; when a dynamic segment has no
`generate_params`; or for intercepting routes.

ISR: `REVALIDATE` + `TAGS` constants. Responses carry `x-nr-cache: HIT | MISS | STALE | BYPASS`; a stale page
is served while one background re-render runs.

```rust
next_rust::revalidate_path("/blog/hello").await;
next_rust::revalidate_tag("products").await;

let posts = cache("posts", CacheOptions::revalidate(60).tag("posts"), || async { fetch().await }).await?;
invalidate("posts").await;
```

Cached values are JSON, so they need `Serialize + Deserialize`; errors are never cached. Stores:
`MemoryStore::new(max_entries)` (bounded LRU) and `FileStore::new(dir)`, wired with
`App::new(routes()).cache(Cache::new(store)).page_store(store)`.

## Middleware

```rust
// app/middleware.rs — every request, before routing
pub async fn middleware(mut req: Request, next: Next) -> Response {
    if req.path().starts_with("/admin") && req.header("authorization").is_none() {
        return Response::redirect("/login");
    }
    next.run(req).await
}
```

Order: `App::middleware(..)` globals, then the app-root `middleware.rs`, then nested ones outermost-first,
then the page/API/action. Built-ins: `request_id()`, `cors(Cors::default().allow_origin(..))`,
`rate_limit(RateLimit::per_minute(60))`, `protected(check, "/login")`, `csrf()`, `security_headers()`,
`sessions(store)`, `sessions_with_ttl(store, ttl)`.

## Styling, assets and fonts

```rust
global_css!("globals.css")       // &'static Stylesheet; put it in the root layout
css_module!("card.module.css")   // styles.card, styles.card_title (compile error if missing)
asset!("fonts/inter.woff2")      // "/_nr/assets/fonts/inter.<hash>.woff2", must live under assets/
```

Paths resolve relative to the source file, then the app directory, then the crate root. Each stylesheet is
emitted once per document as `<style data-nr-css="<hash>">` in `<head>`; client navigations only send new
ones. Release builds prune unused rules and (with Tailwind) rename classes.

```rust
static INTER: LazyLock<LocalFont> = LazyLock::new(||
    LocalFont::new("Inter", asset!("fonts/inter-var.woff2")).weight("100 900").display("swap"));
pub fn metadata() -> Metadata { INTER.metadata() }   // preload + @font-face
```

`public/` is served from the site root, with ETag, `Last-Modified`, 304 and range support. Routes win over
files with the same path.

## Testing

```rust
let client = next_rust::TestClient::new(my_app::routes());        // Environment::Test
let client = next_rust::TestClient::production(my_app::routes()); // production behaviour
let res = client.get("/blog/hello").await;
res.status; res.text; res.header("x-nr-cache"); res.json::<T>(); res.cookies(); res.chunks;
client.get_with_headers("/", &[("accept-language", "de")]).await;
client.post_json("/api/users", &body).await;
client.post_form(&client.action_url("src/actions.rs::signup"), "email=a%40b.c").await;  // signed for this client
client.navigate("/blog", &previous_html).await;   // client-side navigation, partial rendering
client.app().export().await;                      // pre-render every static page
next_rust::layout_keys(&res.text);                // which layouts were rendered
next_rust::strip_layout_markers(&res.text);       // comparable markup
```

`TestClient` keeps a cookie jar across requests, so sessions and flash state work.

## Custom main, background jobs, SSE, WebSockets

```rust
next_rust::routes!();
fn main() {
    next_rust::run_with(
        next_rust::App::new(routes())
            .middleware(next_rust::request_id())
            .get("/healthz", |_req| async { "ok" })
            .cache(next_rust::Cache::new(next_rust::MemoryStore::new(1000))),
    );
}
```

`App` also has `handle(req)` (useful in tests), `revalidate_path`, `revalidate_tag`, `export()`,
`prerender()`, `route_table()`, `serve()` and `serve_listener(listener)`.

Jobs: `Scheduler::new().every("cleanup", Duration::from_secs(3600), job).daily_at("digest", 6, 0, job).start()`,
and `Queue::start(concurrency, worker)` with `queue.push(item)`.

SSE: `Response::sse(stream_of_sse_events)`; build events with `SseEvent::data("x")` or
`SseEvent::json(&value).event("update").id("7")`.

WebSockets (feature `websocket`): `next_rust::ws::upgrade(req, |socket| async move { … })` or
`App::ws(pattern, handler)`.

## Name traps

- `error.rs` exports `ErrorBoundary`; `global-error.rs` exports `GlobalError`.
- `pub const TAGS` inside `page.rs` means ISR cache tags. `next_rust::TAGS` is the list of element macros.
- `load` must return `Result<T>`; a bare `T` does not compile.
- `generate_params` takes no arguments.
- `<use>` → `use_!`; `type` → `r#type`/`type_`; `for` → `r#for`/`for_`; `form`/`label`/`slot`/`as` →
  `form_attr`/`label_attr`/`slot_attr`/`as_`.
- `#[client]` functions expand to return `Node`, not `impl View`.
