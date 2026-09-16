# Routing

The routing directory (`app/` by default, see
[configuration](configuration.md#app)) is scanned recursively at build time,
and again whenever files change during `next-rust dev`. The result is compiled
into an in-memory trie. Requests never touch the filesystem for routing.

## Special files

| file | purpose | required export |
|---|---|---|
| `page.rs` | makes the directory a route | `pub fn Page(..) -> impl View` |
| `page.html` | static HTML page (fragment or full document) | — |
| `layout.rs` | wraps the segment and everything below | `pub fn Layout(children: Children, ..)` |
| `template.rs` | like a layout, rendered inside it | `pub fn Template(children: Children, ..)` |
| `loading.rs` | streamed fallback for the segment below | `pub fn Loading(..) -> impl View` (sync) |
| `error.rs` | error boundary for the segment below | `pub fn ErrorBoundary(info: ErrorInfo, ..)` (sync) |
| `not-found.rs` | UI for `not_found()` below this segment | `pub fn NotFound(..)` |
| `global-error.rs` | last-resort error UI (root only) | `pub fn GlobalError(info: ErrorInfo)` |
| `route.rs` | HTTP handlers | `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`, `HEAD` |
| `middleware.rs` | middleware for this segment and below | `pub async fn middleware(req: Request, next: Next) -> Response` |
| `metadata.rs` | metadata for this segment | `pub fn metadata(..) -> Metadata` |
| `default.rs` | fallback for a parallel slot | `pub fn Page(..)` |
| `sitemap.rs` | generates `/sitemap.xml` (root only) | `pub fn sitemap() -> Sitemap` |
| `robots.rs` | generates `/robots.txt` (root only) | `pub fn robots() -> Robots` |

Any of these functions may be `async` (except the synchronous ones marked
above) and may return `Result<..>`. Other `.rs` files in the app directory
are ignored. Put shared code in `src/` and import it with `crate::…`.

Directories starting with `_` (for example `_components`) and `.` are
private: they never become routes.

## Pages

```text
app/page.rs                 → /
app/about/page.rs           → /about
app/blog/page.rs            → /blog
app/blog/[slug]/page.rs     → /blog/:slug
```

Page functions declare the data they need as **arguments**. The generated
code supplies each one:

```rust
pub async fn Page(
    Path(p): Path<PostParams>,   // typed route parameters
    Query(q): Query,             // query string (map) or Query<T> (typed)
    cookies: Cookies,            // read/set cookies
    headers: Headers,            // request headers
    Extension(user): Extension<User>, // values inserted by middleware
) -> Result<impl View> { … }
```

| argument | provides | makes the route dynamic |
|---|---|---|
| `Params` | raw route params | no |
| `Path<T>` | params deserialized into `T` (parse failure → 404) | no |
| `Data<T>` | the result of this file's `load` function | no |
| `Nonce` | CSP nonce for your own inline scripts | no |
| `Query` / `Query<T>` | query string | yes |
| `Cookies` | cookie jar (get/set/delete) | yes |
| `Headers` | request headers | yes |
| `RequestInfo` | method, URI, client address | yes |
| `Extension<T>`, `Option<Extension<T>>` | request extensions | yes |
| `Auth<T>` | the authenticated user, if any | yes |
| `FormState` | errors/values of the last form submission | yes |
| `CsrfToken` | CSRF token + hidden field | yes |
| `ResponseHeaders` | set response headers | yes |
| `Ctx` | the entire request context | yes |

Unknown argument types are compile errors in the generated code: a
misspelled extractor fails the build instead of failing at runtime.

## HTML pages

`page.html` is a first-class route:

- A **fragment** (no `<html>` or `<!doctype>`) is wrapped in the layouts, like a Rust page.
- A **full document** is served as-is, bypassing layouts.

A directory can't contain both `page.rs` and `page.html`. That's error
`NR0101`, unless you set `[app] html_precedence = "rs"` or `"html"`.

## Layouts

```text
app/
├── layout.rs                 RootLayout
└── dashboard/
    ├── layout.rs               DashboardLayout
    └── settings/
        ├── layout.rs             SettingsLayout
        └── page.rs                 SettingsPage
```

Layouts receive their rendered children:

```rust
pub fn Layout(children: Children) -> impl View {
    div![Header(), main![children], Footer()]
}
```

There's no nesting limit. Layouts can take any extractor, for example
`Layout(children: Children, cookies: Cookies)`, and can export `metadata`.
The framework renders the `<html>`, `<head>` and `<body>` elements, so the
root layout renders only body content. Set the document language with
`[app] lang`.

`template.rs` works like `layout.rs` and is rendered inside the layout of the
same segment. On the server the two behave the same. The distinction exists so
client-side navigation can re-create templates while preserving layouts.

## Route groups

Parenthesized directories organize files without affecting URLs:

```text
app/(marketing)/pricing/page.rs     → /pricing
app/(marketing)/layout.rs           layout for marketing pages only
app/(shop)/cart/page.rs             → /cart
```

Groups take part in the layout hierarchy. If two groups resolve to the same
URL, that's error `NR0102`.

## Dynamic segments

| directory | matches | param value |
|---|---|---|
| `[id]` | exactly one segment | `"42"` |
| `[...slug]` | one or more segments | `["a", "b", "c"]` |
| `[[...slug]]` | zero or more segments | `[]` for the parent URL |

```rust
// app/docs/[...slug]/page.rs
#[derive(serde::Deserialize)]
pub struct P { slug: Vec<String> }

pub fn Page(Path(p): Path<P>) -> impl View { h1![p.slug.join(" / ")] }
```

Parameter names must be identifiers. Values are percent-decoded (`%20` →
space), and an encoded slash (`%2F`) stays inside the segment.

## Route ranking

When several routes match a URL, the most specific one wins. Formally, each
pattern segment has a weight:

| segment | weight |
|---|---|
| static (`settings`) | 3 |
| dynamic (`[id]`) | 2 |
| catch-all (`[...path]`) | 1 |
| optional catch-all (`[[...path]]`) | 0 |

A route's **rank** is its sequence of weights. Among matching routes, the one
with the lexicographically greatest rank wins, compared position by position.

```text
/users/settings      rank [3,3]   wins for /users/settings
/users/[id]          rank [3,2]   wins for /users/42
/users/[...path]     rank [3,1]   wins for /users/42/posts/7
```

The matcher implements this without comparing ranks at runtime. The trie
tries static, then dynamic, then catch-all children, and backtracks when a
branch fails. So `/users/settings/posts` still reaches `/users/[id]/posts`
when there's no `/users/settings/posts` page. A randomized test in
`crates/next-rust-router/tests/routing.rs` checks the matcher against a
brute-force implementation of the definition above.

Matching cost doesn't depend on the number of routes. It's proportional to
the number of URL segments, plus backtracking on shared prefixes. See
[benchmarks](benchmarks.md).

## Conflicts detected at build time

| code | problem |
|---|---|
| NR0101 | `page.rs` and `page.html` in one directory |
| NR0102 | two routes resolve to the same URL (often through groups) |
| NR0103 | different parameter names at the same position (`[id]` vs `[userId]`) |
| NR0104 | a page and a `route.rs` for the same URL |
| NR0105 | a catch-all that is not the last segment |
| NR0106 | an optional catch-all overlapping a page at its parent URL |
| NR0107 | the same parameter name twice in one route |
| NR0108 | invalid segment names (`user[id]`, `[a-b]`, `()`) |
| NR0116 | `[...x]` and `[[...x]]` side by side |

All diagnostic codes are listed in [diagnostics.md](diagnostics.md).
Example output:

```text
error[NR0103]: Conflicting dynamic segment names
  --> /app/users/[id]/page.rs
  --> /app/users/[userId]/edit/page.rs

  The dynamic segment at `/users/[]` is named differently in different routes: `id`, `userId`.
  Routes sharing a URL position must use the same parameter name.

  help: rename the directories so they use a single name
```

## API routes

A `route.rs` anywhere in the tree handles HTTP methods for its URL. See
[api-routes.md](api-routes.md). You can also keep API routes in a separate
directory:

```toml
[api]
directory = "api"   # api/users/route.rs → /api/users
prefix = "/api"
```

## Parallel routes (`@slot`)

A layout can render several independent sections, each backed by its own
subtree:

```text
app/dashboard/
├── layout.rs
├── page.rs                    /dashboard (children)
├── settings/page.rs           /dashboard/settings (children)
├── @analytics/
│   ├── page.rs                slot content for /dashboard
│   └── default.rs             slot content for other dashboard URLs
└── @activity/
    └── default.rs
```

```rust
pub fn Layout(children: Children, mut slots: Slots) -> impl View {
    div![main![children], aside![slots.take("analytics")], aside![slots.take("activity")]]
}
```

For each URL, a slot renders the page at the matching path inside the slot
directory. If there isn't one, it renders the slot's `default.rs`, or nothing.
Slots never create URLs of their own.

## Intercepting routes

An intercepting route renders a different page for a URL when the user
navigates to it **client-side** from a given part of the app. The typical
example is showing a photo in a modal from a feed, while a direct visit
shows the full page.

```text
app/feed/page.rs
app/feed/(.)photo/[id]/page.rs   intercepts /feed/photo/:id when navigating from /feed
app/photo/[id]/page.rs           the regular page
```

| marker | target resolves relative to |
|---|---|
| `(.)segment` | the directory containing the marker |
| `(..)segment` | one URL segment up (`(..)(..)` for two) |
| `(...)segment` | the app root |

How it works: the client runtime sends `x-nr-nav: 1` and
`x-nr-from: <current path>` with navigation requests. The server uses the
intercepting page when the target URL matches and `x-nr-from` falls under the
interceptor's directory. Direct loads, reloads and requests without the
runtime always get the regular page.

Limitations: interception works at page level. It isn't combined with
parallel slots, and nesting intercepts is an error (`NR0115`).

## Middleware placement

`app/middleware.rs` runs for every request **before routing**, so it can
rewrite paths. Nested `middleware.rs` files run for their subtree, outermost
first. See [middleware.md](middleware.md).
