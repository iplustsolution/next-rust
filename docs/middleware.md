# Middleware, authentication & sessions

## Middleware files

```rust
// app/middleware.rs – every request, before routing
use next_rust::prelude::*;

pub async fn middleware(mut req: Request, next: Next) -> Response {
    let started = std::time::Instant::now();
    if req.path() == "/home" {
        let _ = req.set_path("/");                       // rewrite
    }
    let res = next.run(req).await;
    res.with_header("server-timing", &format!("app;dur={}", started.elapsed().as_millis()))
}
```

```rust
// app/admin/middleware.rs – only /admin and below
pub async fn middleware(req: Request, next: Next) -> Response {
    match req.extension::<AuthUser<User>>() {
        Some(AuthUser(user)) if user.is_admin => next.run(req).await,
        Some(_) => Response::text("Forbidden").with_status(403),
        None => Response::redirect("/login"),
    }
}
```

Middleware can:

- inspect and modify the request (headers, extensions, path);
- short-circuit with its own response (reject, redirect);
- modify the response (headers, cookies, status).

## Order

For `GET /admin/users`:

1. global middleware added with `App::middleware`, in order;
2. `app/middleware.rs`, **before routing**, so rewrites affect matching;
3. `app/admin/middleware.rs`, then deeper files, outermost first;
4. the page, API handler or server action.

Unmatched URLs and static files still pass through global middleware and the
root `middleware.rs`.

## Global middleware

Replace `next_rust::app!()` with `routes!()` and your own `main`:

```rust
use next_rust::*;

next_rust::routes!();

fn main() {
    run_with(
        App::new(routes())
            .middleware(request_id())
            .middleware(cors(Cors::default().allow_origin("https://app.example.com").allow_credentials(true)))
            .middleware(rate_limit(RateLimit::per_minute(120)))
            .middleware(sessions(MemorySessionStore::default()))
            .middleware(authenticate),
    );
}

async fn authenticate(mut req: Request, next: Next) -> Response {
    if let Some(user) = verify_token(req.header("authorization")) {
        req.insert_extension(AuthUser(user));
    }
    next.run(req).await
}
```

Any `async fn(Request, Next) -> Response` or closure is middleware. For
reusable middleware with configuration, implement the `Middleware` trait.

## Built-in middleware

| middleware | behaviour |
|---|---|
| `request_id()` | reuses a safe incoming `x-request-id` or creates a random one, exposes `RequestId`, echoes the header |
| `cors(Cors)` | preflight handling, allowed origins, credentials, exposed headers, max-age |
| `rate_limit(RateLimit)` | in-memory token bucket per client IP (or a custom key), 429 + `Retry-After` |
| `protected(check, "/login")` | redirect GET requests (with `?next=`) or 401 others when `check(&req)` is false |
| `csrf()` | double-submit token for all unsafe methods (see below) |
| `sessions(store)` | server-side sessions |
| `security_headers()` | the default security headers (applied automatically unless disabled) |

The rate limiter is per process. For fleets, implement `Middleware` on top of
a shared store such as Redis.

## Authentication primitives

Next Rust doesn't ship an authentication provider. It gives you the pieces to
build JWT, session, OAuth or custom authentication:

- **Middleware** decides who the user is and stores it:
  `req.insert_extension(AuthUser(user))`.
- **Pages and layouts** read it:
  ```rust
  pub fn Page(auth: Auth<User>) -> Result<impl View> {
      let user = auth.require("/login")?;       // redirect guests
      Ok(h1![format!("Hi {}", user.name)])
  }
  ```
- **API routes** use `req.extension::<AuthUser<User>>()`.
- **Server actions** use `ActionContext::extension`.
- **Route protection** uses nested `middleware.rs` or `protected(..)`.
- **Cookies** default to `HttpOnly`, `Secure` (outside development) and
  `SameSite=Lax`.

## Sessions

```rust
App::new(routes()).middleware(sessions(MemorySessionStore::default()))
```

```rust
pub fn Page(Extension(session): Extension<Session>) -> impl View {
    let visits: u32 = session.get("visits").unwrap_or(0) + 1;
    session.insert("visits", visits);
    p![format!("Visits: {visits}")]
}
```

- The cookie `nr_session` holds only a random 256-bit id. Data stays on the
  server, so nothing needs signing.
- `session.regenerate()` after login prevents session fixation.
  `session.destroy()` logs out.
- `MemorySessionStore` is for development and single instances. Implement
  `SessionStore` (three async methods) for Redis, SQL, etc.

## CSRF

- **Server actions** are protected by default. Cross-site requests are
  rejected based on `Sec-Fetch-Site` and `Origin` compared to `Host`
  (`[security] csrf = "origin"`). Extra trusted origins go in
  `[security] allowed_origins`.
- **Your own POST forms and API routes** can add `csrf()` middleware, which
  requires a token matching the `nr_csrf` cookie in the `x-csrf-token` header
  or a `_csrf` form field:

  ```rust
  pub fn Page(csrf: CsrfToken) -> impl View {
      form![method("post"), action("/api/settings"), csrf.field(), button!["Save"]]
  }
  ```

## CORS example (single API route)

For CORS on a subtree, put a `middleware.rs` there:

```rust
// app/api/public/middleware.rs
pub async fn middleware(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.set_header("access-control-allow-origin", "*");
    res
}
```
