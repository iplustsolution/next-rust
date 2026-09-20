//! The "api-routes" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "A page is for people; an API route is for programs. It is a web address that answers with data (usually \
             JSON, a plain-text format for structured data) instead of a designed page. A phone app asking for the \
             list of orders, a payment provider reporting that a card was charged, a script on your own page fetching \
             search results: all of them talk to API routes. Think of it as the service hatch of a restaurant, where \
             pages are the dining room."
        ),
        p![
            "If a form on your own site only needs to hand its fields to a Rust function, a ",
            a![href("/docs/server-actions"), "server action"],
            " is less work. Use an API route when the caller is not one of your pages, or when you need full control \
             over the URL, the method and the response.",
        ],
        h2![id("routers"), a![class("anchor"), href("#routers"), code!["route.rs"]]],
        p![
            "An API route is a file named ",
            code!["route.rs"],
            " somewhere under ",
            code!["app/"],
            ". The folders above it give the URL, exactly as for pages: ",
            code!["app/api/users/route.rs"],
            " answers at ",
            code!["/api/users"],
            ". By convention API routes are kept in ",
            code!["app/api/"],
            ", but the folder name is not special; ",
            code!["app/feed.xml/route.rs"],
            " is a valid route too. A folder holds either a ",
            code!["page.rs"],
            " or a ",
            code!["route.rs"],
            ", never both for the same URL (build error ",
            code!["NR0104"],
            "). Without any ",
            code!["route.rs"],
            " the site simply has no data endpoints; pages work all the same.",
        ],
        p![
            "Inside the file, each HTTP method (the verb of a request: ",
            code!["GET"],
            " reads, ",
            code!["POST"],
            " creates, ",
            code!["PUT"],
            "/",
            code!["PATCH"],
            " change, ",
            code!["DELETE"],
            " removes) is a public function of that name. The example below is the file ",
            code!["app/api/users/[id]/route.rs"],
            ". The square brackets make ",
            code!["id"],
            " a placeholder, so it answers ",
            code!["/api/users/7"],
            ", ",
            code!["/api/users/8"],
            " and so on. ",
            code!["GET"],
            " returns one user as JSON, or 404 when there is none; ",
            code!["DELETE"],
            " answers 204 (done, nothing to show):",
        ],
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
        p!["Status codes are the three-digit verdict on every response: 200 fine, 201 created, 204 fine with no \
             body, 400 the request was malformed, 401 not signed in, 403 not allowed, 404 not there, 422 the data \
             failed validation, 500 the server broke.",],
        h2![id("request"), a![class("anchor"), href("#request"), "Request"]],
        p![
            "The ",
            code!["Request"],
            " argument is everything the caller sent. These are the questions you can ask it, inside any handler in a ",
            code!["route.rs"],
            " (or in middleware):",
        ],
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
        p![
            "Reading the body consumes it, so declare the argument as ",
            code!["mut req: Request"],
            " when you call ",
            code!["json"],
            ", ",
            code!["form"],
            ", ",
            code!["text"],
            " or ",
            code!["raw_body"],
            ".",
        ],
        h2![id("response"), a![class("anchor"), href("#response"), "Response"]],
        p![
            "A handler may return plain values (a ",
            code!["String"],
            ", ",
            code!["Json(value)"],
            "), but ",
            code!["Response"],
            " gives full control. The first block lists the constructors, one per kind of answer; the second shows \
             how to adjust any of them: another status code, an extra header, caching instructions, a cookie.",
        ],
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
        p![
            "The content type (also called MIME type: the label such as ",
            code!["application/json"],
            " or ",
            code!["text/html"],
            " that tells the caller what kind of data it received) is set by the constructor; ",
            code!["with_content_type(\"text/csv\")"],
            " overrides it.",
        ],
        h2![id("cookies"), a![class("anchor"), href("#cookies"), "Cookies"]],
        p![
            "A cookie is a small named value the browser stores and sends back with every later request to your \
             site; it is how a server recognises a returning visitor. In a handler, read and write them through ",
            code!["req.cookies()"],
            ":",
        ],
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
        p![
            code!["HttpOnly"],
            " hides the cookie from JavaScript, ",
            code!["Secure"],
            " sends it over HTTPS only, and ",
            code!["SameSite"],
            " limits when other websites can cause it to be sent. The defaults are the safe choice for anything \
             that identifies a user.",
        ],
        h2![id("server-sent-events"), a![class("anchor"), href("#server-sent-events"), "Server-sent events"]],
        p![
            "Server-sent events (SSE) keep one response open and push messages down it as they happen: a progress \
             bar, a live score, text appearing word by word. Traffic goes one way, from server to browser. This \
             handler, in a file such as ",
            code!["app/api/ticks/route.rs"],
            ", sends ten events one second apart:",
        ],
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
        p![
            "In the browser, read the events with ",
            code!["new EventSource(\"/api/ticks\")"],
            " or, for POST requests, with ",
            code!["window.nextRust.stream(url, ..)"],
            " (see ",
            a![href("/docs/client#requests"), "Requests from the browser"],
            ").",
        ],
        h2![id("websockets"), a![class("anchor"), href("#websockets"), "WebSockets"]],
        p![
            "A WebSocket is a connection that stays open in both directions, like a phone call rather than an \
             exchange of letters: chat, multiplayer, collaborative editing. If only the server needs to talk, \
             server-sent events are simpler. WebSocket support is optional so that apps which do not use it stay \
             small. Enable the ",
            code!["websocket"],
            " feature on the dependency in your project's ",
            code!["Cargo.toml"],
            ":",
        ],
        pre![code![class("language-toml"), r#"next-rust = { version = "0.0", features = ["websocket"] }"#]],
        p![
            "In a route, for example ",
            code!["app/api/echo/route.rs"],
            ". This one sends every text message straight back to its sender:",
        ],
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
        p!["Or programmatically, in a custom ", code!["main"], " in ", code!["src/main.rs"], ":"],
        pre![
            code![class("language-rust"), r#"App::new(routes()).ws("/socket", |mut socket| async move { /* … */ })"#,],
        ],
        p![
            "An incoming message may be up to 1 MiB (",
            code!["ws::MAX_MESSAGE_SIZE"],
            "); a larger one closes the connection before it is buffered. ",
            code!["ws::upgrade_with_limit(req, bytes, handler)"],
            " sets another limit. Upgrades from another site are refused with 403.",
        ],
        h2![id("webhooks"), a![class("anchor"), href("#webhooks"), "Webhooks"]],
        p![
            "A webhook is another company's server calling yours when something happens: a payment succeeded, a \
             commit was pushed. Because anyone on the internet can call that URL, the sender signs each message with \
             a secret you both know, and you must verify the signature before trusting the content. Signature \
             schemes (Stripe, GitHub, …) sign the ",
            strong!["raw"],
            " body, byte for byte. Read it before parsing, as in this handler in ",
            code!["app/api/hooks/stripe/route.rs"],
            ":",
        ],
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
        p![
            "Use ",
            code!["next_rust::constant_time_eq"],
            " to compare secrets. An ordinary ",
            code!["=="],
            " stops at the first differing byte, and that tiny timing difference is enough for an attacker to \
             guess a secret piece by piece.",
        ],
        h2![id("programmatic-routes"), a![class("anchor"), href("#programmatic-routes"), "Programmatic routes"],],
        p![
            "Filesystem routes cover most needs, but handlers can also be registered in code, for example from a library. This replaces the one-line ",
            code!["src/main.rs"],
            " and adds a health-check URL (the address a hosting platform pings to see whether the app is alive) and a webhook route:",
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
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        p![
            "First confirm the framework saw the file, then call it. Both commands run in a terminal from the \
             project folder; the second needs ",
            code!["next-rust dev"],
            " running:",
        ],
        pre![code![
            class("language-sh"),
            r#"next-rust routes                                   # the route is listed with its methods
curl -i http://localhost:3000/api/users/7          # status line, headers and the JSON body
curl -i -X POST http://localhost:3000/api/contact \
     -H 'content-type: application/json' -d '{"name":"Ada","email":"ada@example.com","message":"Hi"}'"#,
        ],],
        p![
            "For a permanent check, write a test with ",
            code!["TestClient::post_json"],
            ": see ",
            a![href("/docs/testing"), "Testing"],
            ".",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["A handler with page-style arguments."],
                " Handlers take nothing or exactly one ",
                code!["Request"],
                "; extractors such as ",
                code!["Params"],
                " or ",
                code!["Cookies"],
                " belong to pages. Anything else is build error ",
                code!["NR0205"],
                ". Use ",
                code!["req.param(..)"],
                " and ",
                code!["req.cookies()"],
                ".",
            ],
            li![
                strong!["Lower-case or private function names."],
                " The export is ",
                code!["pub fn GET"],
                ", in capitals. ",
                code!["fn get"],
                " is ignored, and a file with no handler at all fails with ",
                code!["NR0203"],
                ".",
            ],
            li![
                strong!["Trusting a state-changing request blindly."],
                " API routes are not covered by the server-action protections. For routes that change data on \
                 behalf of a signed-in visitor, call ",
                code!["req.same_origin()"],
                " or add the ",
                code!["csrf()"],
                " middleware (",
                a![href("/docs/middleware#csrf"), "CSRF"],
                ").",
            ],
            li![
                strong!["Parsing a webhook before verifying it."],
                " ",
                code!["req.json()"],
                " consumes the body; the signature can then no longer be checked. Read ",
                code!["raw_body()"],
                " first.",
            ],
            li![
                strong!["Uploads rejected with 413."],
                " The body was larger than ",
                code!["[server] body_limit"],
                " in ",
                code!["next-rust.toml"],
                ". Raise it for that one route with ",
                code!["req.set_body_limit(..)"],
                " rather than for the whole site.",
            ],
            li![
                strong!["A new "],
                code!["route.rs"],
                strong![" that answers 404."],
                " Files under ",
                code!["app/"],
                " are read at build time. ",
                code!["next-rust dev"],
                " rebuilds for you; a plain ",
                code!["cargo run"],
                " may need a fresh build.",
            ],
        ],
    ]
}
