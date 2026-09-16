# API routes

## `route.rs`

```rust
// app/api/users/[id]/route.rs  →  /api/users/:id
use next_rust::prelude::*;

pub async fn GET(req: Request) -> Result<Response> {
    let id: u64 = req.param("id").and_then(|v| v.parse().ok()).ok_or_else(|| Error::http(400, "invalid id"))?;
    let user = db::user(id).await?.or_not_found()?;
    Ok(Response::json(&user))
}

pub async fn DELETE(req: Request) -> Response {
    Response::status(204)
}
```

- Export any of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`, `HEAD`.
- Handlers take `Request` or nothing, may be `async` or sync, and return
  anything implementing `IntoResponse`: `Response`, `Json<T>`, `Html<T>`,
  `String`, `&'static str`, `StatusCode`, `(StatusCode, T)`, `()` (204),
  `Result<T, E>` where `E: Into<Error>`.
- `HEAD` is derived from `GET`. `OPTIONS` answers with `Allow`. Other methods
  get `405` with `Allow`.
- Errors map to status codes: `not_found()` → 404, `Error::http(code, msg)`
  → that code, `Error::validation(..)` → 422 with JSON field errors, and
  anything else → 500. The message is hidden in production.
- Middleware from `middleware.rs` files above the route applies.

## Request

```rust
req.method(); req.uri(); req.path();
req.param("id");                       // route parameter
req.params().get_all("slug");          // catch-all segments
req.query::<Filters>()?;               // typed query string
req.query_param("page");
req.header("authorization");
req.cookies().get("session");
req.extension::<User>();               // inserted by middleware
req.remote_addr();

let body: NewUser = req.json().await?;       // JSON
let form: Login = req.form().await?;         // urlencoded
let text = req.text().await?;
let raw = req.raw_body().await?;             // bytes (webhook signatures)
let stream = req.take_body();                // streaming uploads
```

Bodies are limited by `[server] body_limit` (default 2 MiB). Larger bodies
get `413`, and `Content-Length` is checked before reading. Raise the limit
for a single route with `req.set_body_limit(..)` before reading the body.

## Response

```rust
Response::json(&data)
Response::text("hello")
Response::html("<p>hi</p>")
Response::bytes(vec![..])
Response::stream(my_byte_stream)
Response::redirect("/login")               // 307
Response::permanent_redirect("/new")       // 308
Response::see_other("/done")               // 303
Response::status(204)
Response::not_found()

Response::json(&data)
    .with_status(201)
    .with_header("x-request-cost", "3")     // invalid names/values (CR/LF) are rejected
    .with_cache_control("public, max-age=60")
    .with_cookie(Cookie::new("seen", "1").max_age(Duration::from_secs(3600)))
```

## Cookies

```rust
let cookies = req.cookies();
cookies.get("theme");
cookies.set(Cookie::new("theme", "dark"));
cookies.set(Cookie::new("sid", token).max_age(Duration::from_secs(86_400)).same_site(SameSite::Strict));
cookies.set(Cookie::encoded("prefs", r#"{"lang":"en"}"#));   // any text; read with get_decoded
cookies.delete("sid");
```

Unless set explicitly, cookies from `Cookies::set` get `Path=/`, `HttpOnly`,
`SameSite=Lax`, and `Secure` outside development. Changes made anywhere in
the request (middleware, page, action) are applied to the response. Names
and values are validated, so a value containing `;` or CR/LF is rejected
rather than injected.

## Server-sent events

```rust
use next_rust::futures::StreamExt;

pub async fn GET() -> Response {
    let events = next_rust::futures::stream::iter(1..=10).then(|i| async move {
        next_rust::tokio::time::sleep(Duration::from_secs(1)).await;
        SseEvent::json(&serde_json::json!({ "tick": i })).event("tick")
    });
    Response::sse(events)
}
```

Keep-alive comments are sent every 15 seconds. Newlines in event names and
ids are stripped, and multi-line data is split correctly.

## WebSockets

Enable the `websocket` feature:

```toml
next-rust = { version = "0.1", features = ["websocket"] }
```

In a route:

```rust
pub async fn GET(req: Request) -> Response {
    next_rust::ws::upgrade(req, |mut socket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            if let next_rust::ws::Message::Text(text) = msg {
                let _ = socket.send(next_rust::ws::Message::Text(text)).await;
            }
        }
    })
}
```

Or programmatically:

```rust
App::new(routes()).ws("/socket", |mut socket| async move { /* … */ })
```

## Webhooks

Signature schemes (Stripe, GitHub, …) sign the **raw** body. Read it before
parsing:

```rust
pub async fn POST(mut req: Request) -> Result<Response> {
    let signature = req.header("stripe-signature").unwrap_or_default().to_owned();
    let raw = req.raw_body().await?;
    verify(&raw, &signature, &secret)?;          // HMAC with your crypto crate of choice
    let event: StripeEvent = serde_json::from_slice(&raw)?;
    Ok(Response::status(200))
}
```

Use `next_rust::constant_time_eq` to compare secrets.

## Programmatic routes

Filesystem routes cover most needs, but handlers can also be registered in
code, for example from a library:

```rust
next_rust::routes!();

fn main() {
    next_rust::run_with(
        next_rust::App::new(routes())
            .get("/healthz", |_req| async { "ok" })
            .route(next_rust::http::Method::POST, "/hooks/:provider", hooks::handle),
    );
}
```

Patterns accept `/users/:id`, `/files/*rest` and `/users/[id]`. A conflict
with a file route is reported at startup.
