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
- [The auth kit: next_rust::auth](#the-auth-kit-next_rustauth)
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
`Extension`, `FormState`, `CsrfToken`, `Nonce`, `Ctx`, `Rendering`, `RequestInfo`, `ClientIp`, `ResponseHeaders`, `Next`,
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
Other `.rs` files in `app/` are ignored, so helper modules are fine. A special file may declare one
(`#[path = "function.rs"] pub mod function;`): the build follows `#[path]` module declarations and registers
their `#[server_action]`s (`action!(function::save)`); a private one holding actions is `NR0211`. Directories starting with `_` or `.` are
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

Element macros exist for 139 tags (`next_rust::TAGS` lists them), named exactly after the tag, including SVG
(`svg`, `path`, `circle`, `linearGradient`, `text`, `foreignObject`, `filter`, `feGaussianBlur`, …). Void tags ignore children. The only renamed macro is `use_!`
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
`/_next-rust/image`. Image resizing is not implemented yet: the original file is served.

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
| `ClientIp` | the client's IP (with `[server] trust_proxy`: the `X-Forwarded-For` entry your proxy added, counted from the right by `trusted_proxies`; never the client's own entries), `Option<IpAddr>` | no |
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

Redirecting to a value from the request (`?next=`)? Check it first: `if is_local_path(&next) { redirect(next) }`.
`is_local_path` accepts one leading `/` and refuses `//host`, `/\host`, whitespace and control characters
(browsers strip tabs and newlines, so `/<tab>/host` is another site). The framework applies it to `_redirect`,
the referrer and the `next` written by `protected`.

`Error` implements `From<E>` for any `std::error::Error`, so `?` works on database and IO errors. Status
mapping: not found → 404, validation → 422, redirect → 307/308, internal → 500. Inspect with `kind()`,
`status()`, `is_not_found()`, `is_redirect()`, `public_message()`.

## API routes: Request and Response

```rust
// Request
req.method() req.uri() req.path() req.query_string() req.query::<T>()? req.query_param("q")
req.headers() req.header("x-api-key") req.params() req.param("id")
req.cookies() req.remote_addr() req.client_ip() req.is_secure() req.is_websocket_upgrade()
// is_secure(): https, or X-Forwarded-Proto: https only with [server] trust_proxy
req.same_origin()   // false when the browser says another site sent it; call it in state-changing API routes
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

## The auth kit: next_rust::auth

Optional: `next-rust = { features = ["auth"] }` (`"auth-redis"` adds `RedisLimiter` and `RedisGrace`). Crate
`next-rust-auth`. Database-agnostic; you supply the accounts table and a `SessionStore`. Its `SessionStore` and
`MemorySessionStore` are **not** the key-value ones above (`next_rust::SessionStore`): import from
`next_rust::auth::`.

```rust
use next_rust::auth::{AuthKeys, Purpose, SecureCookies, Sessions, Visitor, CurrentSession, RateLimiter,
                      Passwords, PasswordPolicy, Codes, CodeCheck, limits::{key, Quota}};

// Sealed tokens: AES-256-GCM, HKDF key per purpose, expiry inside. Secret >= 32 bytes.
const INVITE: Purpose = Purpose::new("invite/v1");                       // built in: ACCESS, GRACE, CHALLENGE
let keys = AuthKeys::from_secret(secret)?.with_previous(old_secret)?;    // or from_env("AUTH_SECRET", Some("..._PREVIOUS"))
let token = keys.seal(INVITE, &data, Duration::from_secs(3600))?;        // "v1.<base64url>"
let data: T = keys.open(INVITE, &token)?;                                // OpenError::{Malformed, Invalid, Expired, Shape}

// Cookies: `__Host-<base>` + Secure in production, bare name on http://localhost; always HttpOnly, Path=/, Lax.
let c = SecureCookies::new(&jar);  c.set("flow", v, ttl);  c.get("flow");  c.clear("flow");
c.set_sealed(&keys, Purpose::CHALLENGE, "ch", &pending, ttl);  c.get_sealed::<Pending>(&keys, Purpose::CHALLENGE, "ch");

// Sessions: one engine per process (static LazyLock). Builder calls only before it is cloned.
let sessions = Sessions::new(store, keys)            // store: impl SessionStore (User, Lock associated types)
    .config(SessionConfig { ..Default::default() })  // access 15 min, refresh 30 d, absolute 365 d, grace 15 s; cookies at/rt/did
    .limit_refresh(limiter.clone(), Quota::new(600, 900))
    .shared_grace(RedisGrace::new(conn, "app:grace:"))
    .on_theft(|event: &TheftEvent| { /* audit, mail */ });
sessions.establish(&Visitor::from_action(&ctx), &user_id, mfa_verified).await?;   // after every factor checked out
sessions.current(&visitor).await?;          // Option<CurrentSession<U>>; verifies against the store, rotates when needed
sessions.sign_out(&visitor).await;          sessions.revoke_all(&user_id, "password_changed", Some(&keep_sid)).await?;
// Visitor::from_request(&req) | from_context(&ctx) | from_action(&action_ctx)

// app/(account)/middleware.rs
pub async fn middleware(req: Request, next: Next) -> Response { SESSIONS.require("/login").handle(req, next).await }
// page below it:  pub fn Page(session: CurrentSession<User>) -> impl View      (Auth<User> works too)
// SESSIONS.load() = same without the redirect, for pages guests may see.
```

`SessionStore`: `create`, `find_by_refresh_hash` (returns the record, revoked or not, plus a `Lock`: the
transaction holding `FOR UPDATE`), `rotate(lock, old, new)` (insert new, mark old `rotated`, commit),
`revoke`, `revoke_family`, `revoke_user(user, reason, keep)`, `is_live(session_id, user_id, now) -> Option<User>`
(one join, every request). Write them as plain `async fn`. Ids are UUID strings, times are unix seconds,
errors go through `StoreError::new(e)`. The docs site ("Auth & rate limits") has a full sqlx/Postgres store.
A rotated refresh token that comes back after the grace window ends its whole family (`reuse_detected`); inside
the window it needs the family's device cookie too. `sign_out` ends the whole family (`revoke_family`), and
`establish` revokes the session it replaces (`superseded`). `AuthKeys::from_secret` refuses secrets under 32 bytes
or with fewer than 8 distinct bytes (`KeyError::Weak`). Limiter keys over 256 bytes are hashed.

```rust
// Passwords (argon2id on the blocking pool)
let passwords = Passwords::new();                                   // with_params(memory_kib, passes, lanes)
PasswordPolicy::default().min_length(12).check(plain, &[email, phone])?;   // Err(PolicyViolation), Display = form message
let hash = passwords.hash(plain).await?;
let ok = match account { Some(a) => passwords.verify(plain, &a.hash).await, None => passwords.verify_dummy(plain).await };
passwords.needs_rehash(&hash);  passwords.matches_any(plain, history).await;
// Limits: input over 1024 bytes is refused before hashing (`.max_bytes(n)`; `hash` errs, `verify` is false), at most
// one hash per CPU runs at a time (`.max_concurrent(n)`), a stored hash over 256 MiB / 16 passes is refused, and a
// missing or malformed stored hash costs a dummy verification. No Unicode normalisation; length counts chars.

// Rate limits: per action and subject; shared across instances with Redis
let limiter = RateLimiter::memory();                                // RateLimiter::new(RedisLimiter::connect(url, "app:rl:").await?)
limiter.check(&key::scoped("login:net", &key::network(ip)), Quota::new(200, 900)).await?;          // Err(Throttled) -> 429 via `?`
limiter.check(&key::scoped("login:acct", &key::identifier(email)), Quota::new(10, 900).sliding()).await?;
limiter.reset(&k).await;          Quota::cooldown(45)  Quota::per_minute(n)  Quota::per(n, duration)
limiter.requests(Quota::per_minute(120)).scope("api")               // impl Middleware; .key(|req| ..) overrides the network key
// Fails open when the backend is down; `.fail_closed()` to refuse instead.

// One-time codes: stored as sha256("{user}.{purpose}.{code}"), 6 digits, 10 min, 5 tries
let codes = Codes::new(store);                                      // impl CodeStore { put, check }; MemoryCodeStore for tests
let code = codes.issue(user_id, "email_verification").await?;
match codes.verify(user_id, "email_verification", input).await? { CodeCheck::Valid | Wrong { remaining } | Locked | Expired => .. }
```

## Server actions and forms

```rust
#[server_action] pub async fn refresh() -> Result<Stats>                       // 0 arguments
#[server_action] pub async fn create(input: NewUser) -> Result<User>           // 1 deserializable argument
#[server_action] pub async fn logout(ctx: ActionContext) -> Result<()>                // context, no input
#[server_action] pub async fn rename(ctx: ActionContext, input: Rename) -> Result<()> // context + input
#[server_action(body_limit = 30 * 1024 * 1024)]                                  // this action only: larger body
pub async fn upload(ctx: ActionContext, input: Base64File) -> Result<()>
```

`body_limit` (a constant, in bytes) replaces `[server] body_limit` for that one action, for uploads sent as
base64 (≈4/3 of the file size). It takes effect only after the origin and token checks pass, so it can't be
used to push large bodies at the server from outside a page that rendered the action. Keep the global limit
small and raise it per upload action instead.

At most two arguments (`NR0210`), no `self`, sync allowed. `ActionContext` has `cookies`, `headers`,
`extensions` and `client_ip()` (the `X-Forwarded-For` client only with `[server] trust_proxy`, else the peer). Discovered in `app/` and `src/`. Served at
`POST /_next-rust/action/<token>`; reference one from a view with `action!(path::to::fn)`, which sets `action`,
`method="post"` and `data-nr-action` on the `<form>`.

Action URLs are never fixed. `action!(f)` / `ActionRef::url()` render a placeholder, and every HTML response
replaces it with a token signed (HMAC-SHA256) under `NEXT_RUST_SECRET` (32+ bytes; set it in production, or
links break on restart and across instances), expiring after `[security] action_token_ttl`, and bound to the
visitor's `HttpOnly`, `SameSite=Strict` binding cookie (`__Host-next_rust_bind`, `next_rust_bind` in development). Copied,
expired, tampered or guessed URLs get 403 before the action runs. So: never build action URLs by hand, never
return `.url()` from a JSON API, and still check authentication/authorization inside the action (tokens prove
"this browser loaded a page", not "who the user is"). Pages with action URLs are sent `private, no-store`.
Only `application/json` and `application/x-www-form-urlencoded` bodies are accepted.

Without JavaScript: success redirects (303) to the `_redirect` field or the referrer; a validation error
redirects back to the referring page (to `_redirect` only when there is no referrer) with a 60-second flash
cookie that the `FormState` extractor consumes once. Fields starting
with `_` or containing `password`/`token` are never echoed back.

With JavaScript: `{"ok":true,"data":…}` / `{"ok":false,"errors":{…}}` (422) / `{"ok":false,"redirect":"/…"}`.
Elements with `data-nr-error="field"` are filled in automatically; the form gets `aria-busy` while submitting,
`data-nr-state="success"|"error"` afterwards, and dispatches `nr:success` / `nr:error`.

To show the result in place instead of refreshing (a newsletter box, a "saved" note): `stay_on_success(true)` on
the form, `action_result("")` (whole return value) or `action_result("key")` (one field) on elements that
display it as text, `reset_on_success(true)` to clear the fields. `_redirect` then only applies without
JavaScript, so point it at a confirmation page.

**Every other same-origin form is enhanced too, with no code** (`[client] enhance_forms`, default on): Enter in a
search box never reloads the page. Don't write JS that intercepts submits.

| Form | With JavaScript |
| --- | --- |
| GET (or no method), same origin | client navigation to the built URL (native serialization incl. submitter `name`/`value`, `formaction`/`formmethod`); pushes history (same URL → replace), keeps layouts, always fresh, focused field keeps focus + typed text + caret |
| POST to a `route.rs` | `fetch` (urlencoded, or multipart with files when `enctype` says so; `x-csrf-token` sent). Redirect → followed as a navigation; HTML (any status) → shown as the page; `Content-Disposition: attachment` → saved; JSON/text/204 → like an action result: `data-nr-state`, `nr:success`/`nr:error` (`detail.status`), `data-nr-error` filled from `{"errors":{…}}`/`{"error":"…"}`, `stay_on_success`/`action_result` show it in place, otherwise the page refreshes. Network failure → native submit |
| other origin, `target` ≠ `_self`, `method="dialog"`, `enctype="text/plain"`, `reload(true)` (`data-nr-reload`/`data-nr-native`, on the form or a submit button) | native |

All enhanced forms (actions included): `aria-busy` + disabled submitter while sending, re-submits ignored meanwhile;
native validation runs first unless `novalidate`; `form.submit()`/`requestSubmit()` from scripts take the same
path; `replace(true)` / `scroll(false)` as on links. `live_search(250)` (`data-nr-live`, optional
`data-nr-debounce`, GET only, on the form or one field) submits while typing: debounced, previous request
aborted, history replaced, focus kept. `swap_target("#results")` (`data-nr-target`) updates only that element in
place (paired by tag, `id`, `key(..)`; focus/caret/scroll survive), falling back to a page swap when either side
lacks it. HTML semantics are untouched: a `button![..]` in a form still submits unless `r#type("button")`.

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
Alert![color = Color::Success, variant = Variant::Faded, title = "Saved", closable = true, "Live."]  // role=status/alert
Badge![content = "3", Avatar![..]]   Badge![dot = true, placement = Placement::BottomRight, ..]   Kbd!["⌘", "K"]
Progress![label = "Upload", value = 62.0, show_value = true]   Progress![striped = true]   // no value: indeterminate
CircularProgress![value = 75.0, show_value = true, label = "Mastery"]   Skeleton![lines = 3]   Skeleton![class("h-40")]
Stat![label = "Students", value = "1,204", delta = "12%", trend = Trend::Up, icon = icons::Users(), color = Color::Primary]
Tabs![selected = "b", Tab![key = "a", title = "A", p!["panel"]], Tab![key = "b", title = "B", p!["panel"]]]   // script switches
Tabs![variant = TabsVariant::Underlined, Tab![title = "A", href = "/?t=a", selected = true], Tab![title = "B", href = "/?t=b"]]
Breadcrumbs![BreadcrumbItem![href = "/", "Home"], BreadcrumbItem!["Here"]]      // last = aria-current
Steps![current = 1, vertical = false, Step![title = "Account", href = "/a"], Step![title = "Plan"], Step![title = "Done"]]
Accordion![variant = AccordionVariant::Splitted, multiple = false, AccordionItem![title = "Q", expanded = true, p!["A"]]]  // <details>
Modal![id = "confirm", title = "Delete?", size = Width::Md, dismissable = true, scroll_inside = false,
       ModalBody![..], ModalFooter![Button![on_press = Press::close_modal(), "Cancel"], ..]]   // <dialog>
Button![on_press = Press::open_modal("confirm"), "Delete…"]                      // showModal(); Escape/backdrop close
Table![columns = ["Name", "Score"], striped = true, hoverable = true, sticky_header = true, empty = EmptyState![..],
       each(rows, |r| tr![td![..], td![..]])]
Tooltip![content = "Copy", side = Side::Bottom, Button![..]]   EmptyState![title = "Nothing yet", description = "…", Button![..]]
Avatar![name = "Ada", status = AvatarStatus::Online]             // Online | Away | Busy | Offline

// Overlays and menus (native popover / <dialog>; the script adds keys)
Dropdown![trigger = Button!["Options"], align = MenuAlign::End, side = Side::Bottom,
          DropdownSection![title = "Project", DropdownItem![href = "/edit", start_content = icon, shortcut = "⌘E", "Edit"]],
          DropdownItem![color = Color::Danger, on_press = action!(delete), "Delete"]]   // also `submit = true` inside a form!
Popover![trigger = Button!["Share"], title = "Share", side = Side::Top, p!["…"]]   ContextMenu![target = Card![..], DropdownItem![..]]
Drawer![id = "filters", title = "Filters", side = DrawerSide::Right, size = Width::Md, ModalBody![..], ModalFooter![..]]  // open_modal("filters")
ConfirmDialog![id = "del", title = "Delete?", confirm_label = "Delete", on_confirm = action!(delete), "Cannot be undone."]  // or confirm = form![..]
Toaster![placement = ToastPlacement::BottomRight, flash.map(|m| Toast![color = Color::Success, title = "Saved", duration = 5000, m])]  // once, in the layout
Button![on_press = Press::toast("Copied"), ..]      // JS: nextRust.toast("Saved") / ({ title, message, color, duration }); `nr:toast` window event
CommandPalette![id = "commands", hotkey = "k", CommandGroup![title = "Pages", CommandItem![href = "/docs", keywords = "guide", "Docs"]]]
LoadingOverlay![visible = true, blur = true, label = "Saving…", children]

// Forms (all submit natively; error slots wired to `data-nr-error`)
SearchInput![action = "/search", value = q, shortcut = "⌘K"]        // a GET form, name="q"
Combobox![label = "Country", name = "country", options = ["India", "Germany"]]   // <input list> + datalist; styled list with the script
TagsInput![label = "Topics", name = "topics", values = ["rust"], options = ["web"], max = 5]   // submits "rust, web"
NumberInput![label = "Seats", name = "seats", value = "3", min = 1.0, max = 10.0, step = 1.0]
Slider![label = "Volume", name = "volume", value = 40.0, min = 0.0, max = 100.0, step = 5.0, unit = "%"]
PinInput![label = "Code", name = "code", length = 6, alphanumeric = false, masked = false]   // one input, autocomplete=one-time-code
Rating![name = "stars", label = "Rating", value = 4.0]   Rating![value = 4.5, readonly = true, size = Size::Sm]   // radios / role=img
SegmentedControl![name = "view", value = "week", label = "View", [("day", "Day"), ("week", "Week")]]   // or Segment![value = .., start_content = .., "Label"]
FileUpload![name = "photo", label = "Photo", accept = "image/*", multiple = true, hint = "Up to 5 MB"]
TimePicker![label = "Starts", name = "start", value = "09:30", step = 900]   ColorPicker![label = "Brand", name = "brand", value = "#7828c8"]
DateRangePicker![label = "Stay", name = "stay", start = "2026-09-01", end = "2026-09-05", min = .., max = ..]   // stay_start / stay_end
FormField![label = "Avatar", name = "avatar", control_id = "avatar", required = true, horizontal = true, any_control]
FormSection![title = "Profile", description = "…", fields…]   FormActions![justify = Justify::Between, bordered = true, buttons…]

// Page sections and layout (HTML + CSS only)
Hero![eyebrow = "New", title = "…", subtitle = "…", glow = true, split = true, align = Align::Start, actions = .., media = ..]
Section![id = "pricing", eyebrow = "Pricing", title = "…", subtitle = "…", muted = true, compact = true, width = Width::Lg, ..]
FeatureGrid![cols = 3, Feature![icon = icons::Zap(), title = "Fast", color = Color::Primary, card = true, href = "/x", "text"]]
PricingTable![PricingCard![name = "Pro", price = "$19", period = "/month", description = "…", badge = "Popular", featured = true,
              cta = Button![..], PricingFeature!["Unlimited"], PricingFeature![included = false, "SSO"]]]
Testimonial![name = "Ada", role = "CTO", avatar = url, rating = 5.0, logo = .., "Quote"]   LogoCloud![title = "Trusted by", img![..], ..]
Cta![title = "Ready?", description = "…", color = Color::Primary, soft = false, split = true, Button![..]]
Faq![structured_data = true, [("Question?", "Answer.")]]          // Accordion preset + FAQPage JSON-LD
Timeline![TimelineItem![title = "Shipped", time = "09:12", datetime = "2026-09-12T09:12", color = Color::Success, icon = .., "text"]]
Carousel![label = "Stories", per_view = 3, gap = 4, peek = true, controls = true, CarouselItem![..]]   // scroll-snap; buttons need the script
Marquee![speed = 30, reverse = false, fade = true, gap = 12, items…]   Bento![cols = 3, BentoItem![col_span = 2, row_span = 2, title = "…", description = "…", href = "/x", ..]]
Footer![brand = .., description = "…", social = .., bottom = "© 2026", bottom_end = .., FooterColumn![title = "Product", FooterLink![href = "/pricing", "Pricing"], FooterLink![href = "https://…", external = true, "X"]]]
Pagination![page = 3, total = 20, href = "/posts?page={page}", siblings = 1, compact = false]   // plain links, rel=prev/next
AspectRatio![ratio = "16 / 9", img![..]]   ScrollArea![max_height = "20rem", label = "Log", ..]   Spacer![]   Spacer![size = 8]
Show![above = Breakpoint::Md, ..]   Hide![above = Breakpoint::Md, ..]   Sticky![top = "0", blurred = true, bordered = true, Navbar![..]]

// Data display
Table![header = fragment![TableSelectAll![], SortHeader![href = "/?sort=name&dir=desc", direction = SortDirection::Asc, "Name"], th!["Score"]],
       each(rows, |r| tr![TableSelect![name = "ids", value = r.id], td![..], td![..]])]
List![divided = true, bordered = true, ListItem![title = "Ada", description = "Admin", href = "/u/ada", start_content = Avatar![..], end_content = ..]]
DescriptionList![horizontal = true, DescriptionItem![term = "Plan", "Pro"]]   DescriptionList![cols = 3, ..]   ChipGroup![label = "Tags", Chip![..]]
Code!["cargo run"]   CodeBlock![language = "rust", title = "main.rs", copy = true, wrap = false, "fn main() {}"]   // children: text or highlighted markup
Callout![kind = CalloutKind::Warning, title = "Heads up", "text"]   // Note | Tip | Important | Warning | Danger
Banner![color = Color::Primary, soft = false, href = "/blog", link_label = "Read more", persist = "launch-2026", "Text"]   // CSS-only dismiss
Meter![label = "Storage", value = 92.0, max = 100.0, low = 60.0, high = 85.0, high_is_bad = true, segments = 4, value_label = "92 GB"]
Sparkline![values = [3.0, 5.0, 8.0], color = Color::Success, width = 120, height = 32, area = true, label = "Signups"]
BarChart![title = "Signups", data = [("Mon", 12.0), ("Tue", 30.0)], unit = "%", height = 192, show_values = false, max = 50.0]
LineChart![title = "Revenue", labels = ["Jan", "Feb"], series = [("2025", vec![1.0, 2.0]), ("2026", vec![2.0, 3.0])]]   // or values = [..]
Donut![title = "Traffic", center_label = "visits", data = [("Direct", 60.0), ("Search", 40.0)], size = 160]
Figure![src = "/a.jpg", alt = "…", caption = "…", ratio = "3 / 2", width = 1200, height = 800, eager = false, bordered = true]
VideoEmbed![youtube = "VIDEO_ID", title = "Launch"]   VideoEmbed![src = "https://player.vimeo.com/video/1", poster = "/p.jpg", title = ".."]
ResultBlock![status = ResultStatus::Error, title = "Payment failed", description = "…", Button![..]]   ResultBlock![code = "404", title = ".."]
```

Modal, tabs with panels and closable alerts need the component script (`data-nr-ui`); without it the modal's
close button (a `method="dialog"` form) and `Escape` still work, the selected tab panel shows, and accordions
(`<details>`) open natively. `Press::open_modal` also renders `commandfor`/`command="show-modal"`, so browsers
with invoker commands open modals and drawers without the script. Script-free: dropdown/popover open, close and
focus return (native `popover`, CSS anchor positioning), rating, segmented control, pin input, banner dismiss,
carousel swipe and arrow keys, marquee, charts (hover values), toasts rendered by the server (they fade out with
CSS). Script only: menu arrow keys and type-ahead, context menu, command palette filter and hotkey,
`nextRust.toast`, combobox list (a `<datalist>` otherwise), tag chips (comma separated text otherwise), slider
fill and value, stepper buttons, file names and drag highlight, carousel buttons, copy button, select-all,
banner persistence. Events: `nr:change` (tabs `detail.key`, tags `detail.values`, table selection
`detail.selected`), `nr:open`/`nr:close` (modal, drawer, dropdown, popover), `nr:dismiss` (alert, banner),
`nr:copy` (code block). Charts take their series colors from `--nr-chart-1` … `--nr-chart-8`.

Icons: every Lucide icon as `next_rust::icons::PascalName()` (`icons::ArrowRight()`, former names like
`icons::Home()` too), returning an `Icon` with `.size(24 | "1.25em")`, `.color(..)`, `.stroke_width(2.0)`,
`.absolute_stroke_width(true)`, `.fill(..)`, `.title(..)` (else `aria-hidden`), `.class(..)` (no classes by default),
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
`/_next-rust/ui.js` only on pages that use them and fall back to native controls without it. Select and date-picker
popovers render in the browser's top layer (never clipped by `overflow: hidden`).

`[ui] script` chooses where that behaviour comes from: `"js"` (the default) serves the script, `"wasm"` serves the
same behaviour compiled into the site's shared WebAssembly module and leaves a two-line loader in the page. The
markup, the stylesheet, the ARIA and the keyboard contract are identical either way, and every component still
works with no script at all — it is a delivery choice, not a behaviour one. `"wasm"` needs `[build] wasm_islands =
true` and an `islands/` crate whose `islands!` names `start: next_rust_ui_wasm::boot` (see `examples/ui`). It costs
bytes rather than saving them: ~31 KB brotli of module against 7.6 KB of minified script, plus the shim. Two
differences to know: `nextRust.toast(…)` is a shim forwarder, so it raises the toast but returns nothing; and
release builds cannot shorten names a wasm module knows, so the components' classes and `data-` attributes keep
their full names in the HTML and CSS.

## Client interactivity

```rust
#[client] pub fn Counter(count: i32) -> impl View { … }
#[client(module = "/_next-rust/client/chart.js")] pub fn Chart(points: Vec<f32>) -> impl View { … }
#[client(script = "chart.js")] pub fn Chart2(points: Vec<f32>) -> impl View { … }   // same module, short form
#[client(wasm = "Chart3")] pub fn Chart3(points: Vec<f32>) -> impl View { … }       // Rust island, one wasm module
```

Not async, not generic, no methods, plain identifier arguments implementing `Serialize`. Props are visible in
the HTML. Files in `client/` are served from `/_next-rust/client/`; a module exports `hydrate(element, props)`.

**WASM islands.** `wasm = "Name"` (an identifier) binds the island to the `hydrate_Name` export of the project's
`islands/` crate: a separate crate at the project root, `crate-type = ["cdylib"]`, `no_std`, `panic = "abort"`,
`opt-level = "z"`, whose one dependency is **`next-rust-wasm`** — the browser API. Turned on by
`[build] wasm_islands = true`; needs `rustup target add wasm32-unknown-unknown`, and emits one module shared by
every island on the site plus a one-line glue module each.

The whole of an island crate's boilerplate is one macro. It emits the allocator, the panic handler and every
export the shim calls, so an island file holds island behaviour and nothing else:

```rust
#![no_std]
extern crate alloc;
mod auth_switch;

next_rust_wasm::islands! {
    start: crate::site::boot,                         // optional: runs once per page load, no root
    hydrate_AuthSwitch => crate::auth_switch::hydrate,
}
```

The export name is written out in full because a macro cannot build an identifier out of a string. An island
is `pub fn hydrate(cx: Ctx)`, where `Ctx` is `{ root: El, inst: Inst, props: Props }`.

What `next_rust_wasm` gives it: `El`/`ElList` (queries, the tree, attributes, classes, styles, dataset, IDL
properties, geometry, the top layer), `Str` for strings read *out* of the DOM, events as Rust closures
(`el.on("click", move |ev| …)`, `on_delegate`, `on_window`, capture/passive/once) so two instances of one
island never share state, `async` (`spawn`, `timeout`, `raf`, `fetch`, `action`, `sse`) on a small in-crate
executor, timers and the three observers, storage/history/location/cookies, `Intl` number and date formatting
done by the browser rather than by linking ICU, and `Val` — a handle to *any* JavaScript value, which is how
files, `FormData`, `URL`, a `Response` and a vendored SDK are reached without an opcode each. `num` has the
`f64` helpers `core` lacks, plus JavaScript's `Number()`/`String(n)` semantics, which differ from Rust's in
both directions.

Props are a compact binary blob, not JSON: field names are FNV-1a hashes resolved at compile time by `k!`, so
**no field name appears in the module** and nothing parses JSON in the browser (`serde_json` in an island cost
38 KB raw / 18.5 KB gzip and leaked every field name). Read them with `cx.props.i64(k!("count"))`, or declare a
struct with `props!`. An action's reply arrives the same way: `action(url, json).await` resolves to `Props` and
fails with `ActionError` carrying `status()`, `code()`, `message()` and the per-field messages.

The module imports one function, `e.h(op, …)`, and exports `memory`, `alloc`, `dealloc`,
`hydrate_<Name>(root, props_ptr, props_len) -> instance`, `on_event(cb, event)`, `on_resume(token, ptr, len)`
and optionally `start()`. The opcode table is append-only: a module compiled against one numbering may run
against a shim a browser cached from an earlier deploy.

Two more switches put the *framework's* browser logic in the same module: `[client] runtime = "wasm"` (client
navigation, prefetching, the loading bar, form enhancement, hydration) and `[ui] script = "wasm"` (the shipped
components' behaviour). Both default to `"js"`. Everything that is on compiles into **one** module,
`/_next-rust/client/__nrw/islands.wasm`, not one per crate. Also `[build] wasm_opt` (default `true`, runs
`wasm-opt -Oz` when on `PATH`) and `[build] wasm_keep_names` (default `false`, keeps the wasm name section —
not the same key as `[build] keep_names`).

**Any** of the three switches makes the CSP gain `'wasm-unsafe-eval'`, automatically and only then; without it
the browser raises `CompileError`. It composes with `[analytics]`, which widens the same directive. Delivery:
`application/wasm`, brotli/gzip precompressed, a content-hashed `?v=` URL served `immutable`, and a `<link
rel="preload" as="fetch" crossorigin>`. With `[build] inline_assets` the shim is inlined and the `.wasm` stays
a fetch. **Nothing here can fail a build except your own code**: a missing `wasm32-unknown-unknown` target, a
missing `wasm-opt` or a missing `islands/` crate is a `cargo:warning=` naming the fix, and the binary still
builds with the behaviour left out. A missing export, no WebAssembly, or a rejected compile leaves the
server-rendered markup in place; a visitor with JavaScript off gets the server-rendered HTML and the no-JS
form path. Additive: `#[client(script = …)]` islands are unaffected. Limits: this is obfuscation and one
language, **not a security boundary** — the shim is readable JavaScript, the `.wasm` is downloadable and
`wasm2wat`-able, its string literals are plain text, props stay public and secrets stay on the server; and one
small island costs more than the JS it replaces (~1585 B vs 1202 B brotli). Sizes and the full treatment:
`/docs/wasm-islands`.

Declarative bindings: `data-nr-text`, `data-nr-show` (`"!path"` negates), `data-nr-bind`,
`data-nr-class-<name>`, `data-nr-on-<event>` via `on("click", "…")`, `data-nr-key`, `data-nr-error`.
Operations: `increment:path[,n]`, `decrement:path[,n]`, `toggle:path`, `set:path=<json>`, `prevent`,
`navigate:/url`, `action:<url>[->path]`. There is no `eval`, so a strict CSP works.

```js
window.nextRust.navigate(href) / .replace(href) / .back() / .forward() / .refresh() / .prefetch(href)
await window.nextRust.action(url, input)     // throws with .errors, .status, .code and .data
await window.nextRust.request(url, { method, body, headers, signal, timeout, as })  // same-origin JSON (as: "blob"|"text"|"response"); throws .status/.data
for await (const { event, data, id } of window.nextRust.stream(url, { method: "POST", body })) …  // SSE
window.nextRust.env                          // NEXT_RUST_PUBLIC_* variables
window.addEventListener("nr:navigate", e => e.detail.url)
```

The runtime (`/_next-rust/runtime.js`) is added only to pages with an internal link, a form or an island. It prefetches after
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
        .theme_color_for("(prefers-color-scheme: dark)", "#0d1b2a")   // one per media query
        .icon("/favicon.svg").apple_touch_icon("/apple-touch-icon.png")
        .icon_sized("/favicon-32x32.png", "32x32", "image/png")
        .open_graph(OpenGraph { title: Some("Pricing".into()), images: vec![OgImage { url: "/og.png".into(), ..Default::default() }], ..Default::default() })
        .twitter(Twitter { card: Some("summary_large_image".into()), ..Default::default() })
        .alternate("de", "https://acme.dev/de/pricing")
        .meta("x-custom", "1").stylesheet("/styles.css")
        .preload_font("/fonts/inter.woff2").preload_image("/hero.avif")
        .html_attribute("data-theme", "ocean")   // on <html>; also "class", and "lang" overrides [app] lang
        .analytics(false)                // no [analytics] tags on this page / below this layout
}
```

Layouts and pages merge outside-in; the child wins. `metadata` may be async, take extractors and return
`Result`, and it may take `Data<T>` — it then gets the same `load` call the page does. Returning
`not_found()` from it renders the 404 page — but note it renders the **app-root**
`not-found.rs`, not a per-segment one, because metadata is resolved before the page tree. If you want a
segment's own 404 page, raise `not_found()` from `load` or `Page` and keep `metadata` infallible.

Site verification tags, analytics scripts, `ads.txt` and app-link files are **configuration, not metadata**:
set `[verification]` / `[analytics]` in `next-rust.toml` (see cli-and-config.md) and never write those tags,
snippets or routes by hand. `.analytics(false)` is the only per-page switch; `Metadata::is_noindex()` tells
whether `robots` says `noindex`/`none` (what `[analytics] skip_noindex` looks at).

`html_attribute(name, value)` puts server-rendered attributes on `<html>` (per name, child wins; `lang` overrides
`[app] lang`). Invalid names and `on*` are dropped, values escaped, `class` shortened/styled like any class.
Client navigations swap only the body, so they keep `<html>` as it is, including changes a script made.

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

`static` renders at build time (or first request) and caches; `dynamic` renders per request; `partial` is a
static page with per-request holes (below); `auto` (default) decides at build time. `auto` picks dynamic when the page, any layout or template above it, a slot page, or a
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

### Partial prerendering: `dynamic` holes and `cached` fragments

```rust
pub const RENDERING: Rendering = Rendering::Partial;   // optional: any static page with a hole is served this way
pub const REVALIDATE: u64 = 3600;                      // the shell follows the static/ISR rules (TAGS too)

pub async fn Page() -> Result<impl View> {             // no request extractors here (NR0212 / render error)
    let product = db::product().await?;                // runs once, for the cached shell
    Ok(article![
        h1![product.name.clone()],
        dynamic(p!["Loading cart…"], move |ctx| {      // Fn(Ctx): runs per request, streamed after the shell
            let sku = product.sku.clone();             // captures come from the static render (shared)
            async move {
                let q = ctx.extract::<Query>()?;       // any extractor; or ctx.cookies / ctx.headers / ctx.uri
                let cart = cart::load(ctx.cookies.get("session")).await?;
                Ok::<_, Error>(p![format!("{} in cart", cart.count(&sku))])
            }
        }),
    ])
}

Dynamic::new(fallback).on_error(|info: ErrorInfo| p![info.message]).render(|ctx| async move { .. })

let list = cached("sidebar", CacheOptions::revalidate(300).tag("posts"), || async { ul![..] }).await?;  // -> Node
invalidate_fragment("sidebar").await;
```

- The closure returns a view or `Result<view, E>`; it must be `Fn + Send + Sync + 'static` (clone captured
  values inside, as above) and is in the prelude with `cached`. `CacheOptions` is `next_rust::CacheOptions`.
- Shell = everything outside holes + fallbacks, rendered without a request and cached; each request gets the
  shell bytes first, then every hole concurrently via the suspense `<template>` swap. Response:
  `cache-control: private, no-store`, `x-nr-prerender: partial`, `x-nr-cache` = state of the shell.
- Holes work in layouts and components too. A static page with a hole from anywhere is detected when rendered;
  the build (`next-rust routes`: `◐ partial`) sees holes written in the route's own page/layout/template files.
  On dynamic pages and in `next-rust dev` a hole behaves like `suspense`.
- Holes run after the response head: no `cookies.set`, `ResponseHeaders`, `set_status` (a warning is logged);
  errors/panics render the page's nearest `error.rs` (or `on_error`), redirects become `location.replace`.
- Crawlers and `[rendering] streaming = false` get holes rendered in place (no JS needed); otherwise the
  fallback stays without JavaScript.
- `cached` stores rendered HTML plus the stylesheets/classes/flags it needs. Inside a static page or shell the
  page inherits the fragment's tags and shortest lifetime. Don't put a hole inside `cached`; don't cache
  per-user markup under a shared key.

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
`sessions(store)`, `sessions_with_ttl(store, ttl)`. `rate_limit` keys by `client_ip()` (IPv6 by /64 network);
`protected` redirects to `login?next=<path and query, fully percent-encoded>`.

## Styling, assets and fonts

```rust
global_css!("globals.css")       // &'static Stylesheet; put it in the root layout
css_module!("card.module.css")   // styles.card, styles.card_title (compile error if missing)
asset!("fonts/inter.woff2")      // "/_next-rust/assets/fonts/inter.<hash>.woff2", must live under assets/
```

Paths resolve relative to the source file, then the app directory, then the crate root. Each stylesheet is
emitted once per document as `<style data-nr-css="<hash>">` in `<head>`; client navigations only send new
ones. Every sheet is sent **per page**, cut down to what the renderer wrote: a rule goes when the page has
what one of its selectors needs (the class it starts with; every element, id and attribute outside
`:not()/:is()/:where()/:has()`; values compared for `data-*`, `type`, `role`, `dir`, `lang`, `rel`, `target`);
a custom property (`:root`, `.dark`, theme blocks, `@layer theme`) when something sent reads it, also through
another variable, a fallback, `calc()`, an inline `style`, an SVG presentation attribute
(`font-family="var(--mono)"`), an inline script or another stylesheet of the page; `@font-face`/`@keyframes`/
`@property` when a `font-family`/`animation`/variable that is sent names them; empty blocks and at-rules are
not written. Names of the scripts the page loads count as present (`client/` modules of its islands,
`assets/`/`public/` scripts and their relative imports: classes, `setAttribute`/`dataset.x`/`el.hidden`
attributes, ids, `--vars`, `createElement("x")`/`"<x"` elements), as do `aria-*`, `open`, `data-nr-*` and what
the runtime and `ui.js` set. Client navigations stay correct: the `<style>` ids carry what the document had.
Escape hatches: `[tailwind] keep_classes` / `[assets] css_safelist` (classes, ids, attributes, `"<dialog"`),
`[assets] css_keep_vars = ["--brand"]`, and `[assets] css_static_attributes = ["data-theme"]` when a script
names an attribute that only ever has the server-rendered value (drops unused theme blocks). Release builds also prune unused rules project-wide,
rename classes (Tailwind and `nr-*` ones; 1–2 characters, 3 beyond ~1,000 classes) and rewrite the class
lists in `client/`/`assets/` scripts to match (`js_classes.rs`: whole-class-list strings, `classList.*`,
`className =`, `class:`, `class="…"`, `.x` selectors; a lone keyword-like word such as `"block"` pins the
name), then minify and mangle those scripts (`[build] minify_js`; `asset!` hashes the served bytes). Release
builds also leave out generated-looking classes (`sm:x`, `bg-[…]`, `w-1/3`, `!p-0`) that no stylesheet, script or
static file names (`[build] drop_unused_classes`; `next-rust build` lists them), compress embedded files with
Brotli and gzip at build time and serve them precompressed, and compress pages with Brotli when accepted.
Every document starts with `<meta name="generator" content="Next Rust x.y.z">` and the framework scripts carry
a `/*! Next Rust x.y.z */` banner unless `[build] signature = false` (`next_rust::VERSION`, `next_rust::signature()`).

**Scripts and styles in Rust (a project without `.js`/`.css` files).** At module level, in any file under
`src/` or the app directory (the macros expand to nothing; the build script reads the sources and treats them
exactly like files: class scanning, short class names, minification, `asset!` hashing, embedding, same URLs):

```rust
next_rust::client_script!("chart.js", r#"import { scale } from "./scale.js"; export function hydrate(el, props) { … }"#);
next_rust::asset_script!("js/site.js", r#"…"#);      // then script![r#type("module"), src(asset!("js/site.js")), nonce(n.0)]
next_rust::stylesheet!(r#".hero { @apply mx-auto; }"#); // file listed in [tailwind] stylesheets = ["src/styles/theme.rs"]
```

Modules import each other by relative path (`./scale.js`), file or declaration alike; a file in `client/` or
`assets/` with the same path wins. `NR0213`: a path declared twice. `NR0009`: a `.rs` stylesheet entry without
`stylesheet!`. Pick enough `#`s for the raw string (`r##"…"##` when the source contains `"#`).

**Inline delivery: `[build] inline_assets = true`.** No `.js` request at all: the runtime and `ui.js` become
inline `<script type="module" data-nr-js="…" nonce="…">`; `<script src>` of an `assets/`/`client/` script
becomes an inline script with the same attributes; island modules and their static imports are registered in
`window.__nrm` by inline scripts in `<head>` (modules are turned into factories at build time, so they need
no URL) and the runtime hydrates islands from that registry. Works with the strict CSP preset (every inline
script carries the request's nonce) and with client navigation (`x-nr-scripts` lists the `data-nr-js` ids the
document has; they are not sent or run again). A module reached only by a dynamic `import("./vendor/big.js")`
(literal, relative) is **lazy**: never inlined, served at `/_next-rust/m/<hash>` (immutable) and fetched on
demand; put large vendor bundles there. Limits: no import cycles, no `import.meta`, classic scripts with
`defer`/`async` stay linked.

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

Shared state built once at startup (database pool, Redis, HTTP client): `run_with_setup` runs an async closure
before the server starts — after the `.env` files are loaded, on the runtime that serves requests, so
connections opened there stay valid — and `AppBuilder::state(value)` clones the value into every request:

```rust
fn main() {
    next_rust::run_with_setup(routes(), |app| async move {
        let state = Arc::new(AppState::init().await);   // connect, ping, fail fast
        app.state(state)
    });
}
// page / layout / load / action:  Extension(state): Extension<Arc<AppState>>
// route.rs:                        req.extensions().get::<Arc<AppState>>()
```

`state` takes any `Clone + Send + Sync + 'static`; wrap it in an `Arc`. A handler asking for state that was
never registered gets a 500, not a panic. The setup closure also runs for `--routes` and `--export` (what
`next-rust build` invokes): check `std::env::args()` there and skip connecting.

`App` also has `handle(req)` (useful in tests), `revalidate_path`, `revalidate_tag`, `export()`,
`prerender()`, `route_table()`, `serve()` and `serve_listener(listener)`.

`AppBuilder::on_serve(|| async { … })` runs when the server starts listening (not for `--routes`, `--export`
or `App::handle`); `on_shutdown(|| async { … })` runs after connections have drained on SIGINT/SIGTERM.

### Background jobs (`next_rust::jobs`)

```rust
use next_rust::jobs::*;

#[job(queue = "mail", max_attempts = 5, timeout = "30s")]            // all arguments optional
pub async fn send_welcome(input: Welcome) -> Result<(), JobError> { … }  // input: Serialize + Deserialize

#[cron("0 0 3 * * *")]                                                // UTC; 5 fields, or 6 with seconds first
pub async fn nightly_cleanup() { … }

fn main() {
    next_rust::run_with_setup(routes(), |app| async move {
        let runner = JobRunner::new()                     // or JobRunner::with_backend(RedisBackend::connect(url, "app:jobs").await?)
            .queue("mail", 4)                             // concurrency per queue, per process (default 10)
            .register(send_welcome)                       // explicit: a process runs only what it registers
            .cron(nightly_cleanup)
            .schedule("digest", "0 7 * * MON-FRI", || async { … })
            .on_failure(|job, f| eprintln!("{} failed: {}", job.name, f.error));
        let status = status_handler(runner.client());     // read-only JSON; protect the path
        app.jobs(runner).get("/internal/jobs", status)    // AppJobsExt: start on serve, drain on shutdown
    });
}

enqueue(send_welcome, input).await?;                      // also enqueue_in(duration, job, input), enqueue_at(time, job, input)
jobs.job(charge, order).unique("charge:42").priority(50).send().await?;   // jobs: Extension<JobClient> or JobClient::global()
```

- `#[job]` signatures: `()`, `(input)`, `(JobContext)`, `(input, JobContext)`; returns `()` or `Result<(), E>` where
  `E: Into<JobError>` (every `std::error::Error`; a `String` needs `JobError::msg`). The macro replaces the
  function with a unit struct of the same name implementing `Job`; call the body as `send_welcome::perform(input)`.
  Arguments: `name` (stored name — keep stable), `queue`, `max_attempts` (5), `timeout` (`"5m"`), `backoff`
  (`"5s"`), `max_backoff` (`"1h"`), `priority` (-100..=100), `path = "next_rust_jobs"` for crates without `next-rust`.
- Failures retry with exponential backoff ±20 % jitter; `JobError::fatal(..)` dead-letters at once,
  `JobError::retry_in(delay, ..)` overrides the delay; panics and timeouts count as failed attempts.
  Dead letters: `jobs.dead_jobs(queue, n)`, `retry_dead`, `purge_dead`; `jobs.cancel(queue, id)` for jobs not started.
- `.unique(key)` = one job per key until it finishes; `.unique_for(key, window)` = idempotency key that outlives the job.
  `Enqueued { id, duplicate }` tells which happened.
- Delivery is **at least once** (visibility timeout 60 s + heartbeat + reaper): jobs must be idempotent.
- Limits: a serialised input over 1 MiB is refused with `Error::Invalid` (`JobClient::max_payload_bytes(n)`; queue a
  reference, not the data); job, queue and id names are 1..=128 bytes, unique keys up to 512, no control characters;
  stored error messages are cut to 4 KB; cron expressions up to 256 bytes; `MemoryBackend::max_jobs(n)` caps the
  in-memory store. Custom backends call `JobRecord::validate()`. `status_handler` shows names, counts and last
  errors (never payloads): keep it behind auth.
- Cron: `* , - /`, names (`JAN`, `MON-FRI`), `@hourly|@daily|@weekly|@monthly|@yearly`, `@every 90s`. One instance
  runs each tick (a per-tick lock in the backend — across instances only with Redis); failed ticks are not
  retried, so enqueue a job from the task when the work must not be lost.
- Redis backend: feature `jobs-redis` on `next-rust` (crate feature `redis` on `next-rust-jobs`), Redis ≥ 5; on
  Cluster use a hash-tagged prefix (`"{app}:jobs"`). Enqueue-only process: `JobClient::new(backend).install_global()`.
- Without the app builder: `let handle = runner.start()?; … handle.shutdown().await;` (waits `shutdown_timeout`, 30 s).

The older fire-and-forget helpers are still in the same module:
`Scheduler::new().every("cleanup", Duration::from_secs(3600), job).daily_at("digest", 6, 0, job).start()`,
and `Queue::start(concurrency, worker)` with `queue.push(item)` — no retries, nothing survives a restart.

SSE: `Response::sse(stream_of_sse_events)`; build events with `SseEvent::data("x")` or
`SseEvent::json(&value).event("update").id("7")`.

WebSockets (feature `websocket`): `next_rust::ws::upgrade(req, |socket| async move { … })` or
`App::ws(pattern, handler)`. Upgrades from another site are refused with 403 (`req.same_origin()`). Incoming
messages are limited to 1 MiB (`ws::MAX_MESSAGE_SIZE`); `ws::upgrade_with_limit(req, bytes, handler)` sets another limit.

## Name traps

- `error.rs` exports `ErrorBoundary`; `global-error.rs` exports `GlobalError`.
- `pub const TAGS` inside `page.rs` means ISR cache tags. `next_rust::TAGS` is the list of element macros.
- `load` must return `Result<T>`; a bare `T` does not compile.
- `generate_params` takes no arguments.
- `<use>` → `use_!`; `type` → `r#type`/`type_`; `for` → `r#for`/`for_`; `form`/`label`/`slot`/`as` →
  `form_attr`/`label_attr`/`slot_attr`/`as_`.
- `#[client]` functions expand to return `Node`, not `impl View`.
