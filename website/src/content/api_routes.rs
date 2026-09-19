//! The "api-routes" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        h2![id("routers"), a![class("anchor"), href("#routers"), code!["route.rs"]]],
        pre![code![
            class("language-rust"),
            r#"// app/api/users/[id]/route.rs  →  /api/users/:id
use next_rust::prelude::*;

pub async fn GET(req: Request) -> Result<Response> {
    let id: u64 = req.param("id").and_then(|v| v.parse().ok()).ok_or_else(|| Error::http(400, "invalid id"))?;
    let user = db::user(id).await?.or_not_found()?;
    Ok(Response::json(&user))
}

pub async fn DELETE(req: Request) -> Response {
    Response::status(204)
}"#,
        ],],
        ul![
            li![
                "Export any of ",
                code!["GET"],
                ", ",
                code!["POST"],
                ", ",
                code!["PUT"],
                ", ",
                code!["PATCH"],
                ", ",
                code!["DELETE"],
                ", ",
                code!["OPTIONS"],
                ", ",
                code!["HEAD"],
                ".",
            ],
            li![
                "Handlers take ",
                code!["Request"],
                " or nothing, may be ",
                code!["async"],
                " or sync, and return anything implementing ",
                code!["IntoResponse"],
                ": ",
                code!["Response"],
                ", ",
                code!["Json<T>"],
                ", ",
                code!["Html<T>"],
                ", ",
                code!["String"],
                ", ",
                code!["&'static str"],
                ", ",
                code!["StatusCode"],
                ", ",
                code!["(StatusCode, T)"],
                ", ",
                code!["()"],
                " (204), ",
                code!["Result<T, E>"],
                " where ",
                code!["E: Into<Error>"],
                ".",
            ],
            li![
                code!["HEAD"],
                " is derived from ",
                code!["GET"],
                ". ",
                code!["OPTIONS"],
                " answers with ",
                code!["Allow"],
                ". Other methods get ",
                code!["405"],
                " with ",
                code!["Allow"],
                ".",
            ],
            li![
                "Errors map to status codes: ",
                code!["not_found()"],
                " → 404, ",
                code!["Error::http(code, msg)"],
                " → that code, ",
                code!["Error::validation(..)"],
                " → 422 with JSON field errors, and anything else → 500. The message is hidden in production.",
            ],
            li!["Middleware from ", code!["middleware.rs"], " files above the route applies."],
        ],
        h2![id("request"), a![class("anchor"), href("#request"), "Request"]],
        pre![code![
            class("language-rust"),
            r#"req.method(); req.uri(); req.path();
req.param("id");                       // route parameter
req.params().get_all("slug");          // catch-all segments
req.query::<Filters>()?;               // typed query string
req.query_param("page");
req.header("authorization");
req.cookies().get("session");
req.extension::<User>();               // inserted by middleware
req.remote_addr();
req.client_ip();                       // X-Forwarded-For only with [server] trust_proxy

let body: NewUser = req.json().await?;       // JSON
let form: Login = req.form().await?;         // urlencoded
let text = req.text().await?;
let raw = req.raw_body().await?;             // bytes (webhook signatures)
let stream = req.take_body();                // streaming uploads"#,
        ],],
        p![
            "Bodies are limited by ",
            code!["[server] body_limit"],
            " (default 2 MiB). Larger bodies get ",
            code!["413"],
            ", and ",
            code!["Content-Length"],
            " is checked before reading. Raise the limit for a single route with ",
            code!["req.set_body_limit(..)"],
            " before reading the body.",
        ],
        h2![id("response"), a![class("anchor"), href("#response"), "Response"]],
        pre![code![
            class("language-rust"),
            r#"Response::json(&data)
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
    .with_cookie(Cookie::new("seen", "1").max_age(Duration::from_secs(3600)))"#,
        ],],
        h2![id("cookies"), a![class("anchor"), href("#cookies"), "Cookies"]],
        pre![code![
            class("language-rust"),
            r##"let cookies = req.cookies();
cookies.get("theme");
cookies.set(Cookie::new("theme", "dark"));
cookies.set(Cookie::new("sid", token).max_age(Duration::from_secs(86_400)).same_site(SameSite::Strict));
cookies.set(Cookie::encoded("prefs", r#"{"lang":"en"}"#));   // any text; read with get_decoded
cookies.delete("sid");"##,
        ],],
        p![
            "Unless set explicitly, cookies from ",
            code!["Cookies::set"],
            " get ",
            code!["Path=/"],
            ", ",
            code!["HttpOnly"],
            ", ",
            code!["SameSite=Lax"],
            ", and ",
            code!["Secure"],
            " outside development. Changes made anywhere in the request (middleware, page, action) are applied to the response. Names and values are validated, so a value containing ",
            code![";"],
            " or CR/LF is rejected rather than injected.",
        ],
        h2![id("server-sent-events"), a![class("anchor"), href("#server-sent-events"), "Server-sent events"]],
        pre![code![
            class("language-rust"),
            r#"use next_rust::futures::StreamExt;

pub async fn GET() -> Response {
    let events = next_rust::futures::stream::iter(1..=10).then(|i| async move {
        next_rust::tokio::time::sleep(Duration::from_secs(1)).await;
        SseEvent::json(&serde_json::json!({ "tick": i })).event("tick")
    });
    Response::sse(events)
}"#,
        ],],
        p![
            "Keep-alive comments are sent every 15 seconds. Newlines in event names and ids are stripped, and multi-line data is split correctly.",
        ],
        h2![id("websockets"), a![class("anchor"), href("#websockets"), "WebSockets"]],
        p!["Enable the ", code!["websocket"], " feature:"],
        pre![code![class("language-toml"), r#"next-rust = { version = "0.0", features = ["websocket"] }"#]],
        p!["In a route:"],
        pre![code![
            class("language-rust"),
            r"pub async fn GET(req: Request) -> Response {
    next_rust::ws::upgrade(req, |mut socket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            if let next_rust::ws::Message::Text(text) = msg {
                let _ = socket.send(next_rust::ws::Message::Text(text)).await;
            }
        }
    })
}",
        ],],
        p!["Or programmatically:"],
        pre![
            code![class("language-rust"), r#"App::new(routes()).ws("/socket", |mut socket| async move { /* … */ })"#,],
        ],
        h2![id("webhooks"), a![class("anchor"), href("#webhooks"), "Webhooks"]],
        p!["Signature schemes (Stripe, GitHub, …) sign the ", strong!["raw"], " body. Read it before parsing:",],
        pre![code![
            class("language-rust"),
            r#"pub async fn POST(mut req: Request) -> Result<Response> {
    let signature = req.header("stripe-signature").unwrap_or_default().to_owned();
    let raw = req.raw_body().await?;
    verify(&raw, &signature, &secret)?;          // HMAC with your crypto crate of choice
    let event: StripeEvent = serde_json::from_slice(&raw)?;
    Ok(Response::status(200))
}"#,
        ],],
        p!["Use ", code!["next_rust::constant_time_eq"], " to compare secrets."],
        h2![id("programmatic-routes"), a![class("anchor"), href("#programmatic-routes"), "Programmatic routes"],],
        p![
            "Filesystem routes cover most needs, but handlers can also be registered in code, for example from a library:",
        ],
        pre![code![
            class("language-rust"),
            r#"next_rust::routes!();

fn main() {
    next_rust::run_with(
        next_rust::App::new(routes())
            .get("/healthz", |_req| async { "ok" })
            .route(next_rust::http::Method::POST, "/hooks/:provider", hooks::handle),
    );
}"#,
        ],],
        p![
            "Patterns accept ",
            code!["/users/:id"],
            ", ",
            code!["/files/*rest"],
            " and ",
            code!["/users/[id]"],
            ". A conflict with a file route is reported at startup.",
        ],
    ]
}
