//! The "middleware" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        h2![id("middleware-files"), a![class("anchor"), href("#middleware-files"), "Middleware files"]],
        pre![code![
            class("language-rust"),
            r#"// app/middleware.rs – every request, before routing
use next_rust::prelude::*;

pub async fn middleware(mut req: Request, next: Next) -> Response {
    let started = std::time::Instant::now();
    if req.path() == "/home" {
        let _ = req.set_path("/");                       // rewrite
    }
    let res = next.run(req).await;
    res.with_header("server-timing", &format!("app;dur={}", started.elapsed().as_millis()))
}"#,
        ],],
        pre![code![
            class("language-rust"),
            r#"// app/admin/middleware.rs – only /admin and below
pub async fn middleware(req: Request, next: Next) -> Response {
    match req.extension::<AuthUser<User>>() {
        Some(AuthUser(user)) if user.is_admin => next.run(req).await,
        Some(_) => Response::text("Forbidden").with_status(403),
        None => Response::redirect("/login"),
    }
}"#,
        ],],
        p!["Middleware can:"],
        ul![
            li!["inspect and modify the request (headers, extensions, path);"],
            li!["short-circuit with its own response (reject, redirect);"],
            li!["modify the response (headers, cookies, status)."],
        ],
        h2![id("order"), a![class("anchor"), href("#order"), "Order"]],
        p!["For ", code!["GET /admin/users"], ":"],
        ol![
            li!["global middleware added with ", code!["App::middleware"], ", in order;"],
            li![code!["app/middleware.rs"], ", ", strong!["before routing"], ", so rewrites affect matching;"],
            li![code!["app/admin/middleware.rs"], ", then deeper files, outermost first;"],
            li!["the page, API handler or server action."],
        ],
        p![
            "Unmatched URLs and static files still pass through global middleware and the root ",
            code!["middleware.rs"],
            ".",
        ],
        h2![id("global-middleware"), a![class("anchor"), href("#global-middleware"), "Global middleware"]],
        p!["Replace ", code!["next_rust::app!()"], " with ", code!["routes!()"], " and your own ", code!["main"], ":",],
        pre![code![
            class("language-rust"),
            r#"use next_rust::*;

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
}"#,
        ],],
        p![
            "Any ",
            code!["async fn(Request, Next) -> Response"],
            " or closure is middleware. For reusable middleware with configuration, implement the ",
            code!["Middleware"],
            " trait.",
        ],
        h2![id("built-in-middleware"), a![class("anchor"), href("#built-in-middleware"), "Built-in middleware"],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["middleware"], th!["behaviour"]]],
                tbody![
                    tr![
                        td![code!["request_id()"]],
                        td![
                            "reuses a safe incoming ",
                            code!["x-request-id"],
                            " or creates a random one, exposes ",
                            code!["RequestId"],
                            ", echoes the header",
                        ],
                    ],
                    tr![
                        td![code!["cors(Cors)"]],
                        td!["preflight handling, allowed origins, credentials, exposed headers, max-age"],
                    ],
                    tr![
                        td![code!["rate_limit(RateLimit)"]],
                        td!["in-memory token bucket per client IP (or a custom key), 429 + ", code!["Retry-After"],],
                    ],
                    tr![
                        td![code!["protected(check, \"/login\")"]],
                        td![
                            "redirect GET requests (with ",
                            code!["?next="],
                            ") or 401 others when ",
                            code!["check(&req)"],
                            " is false",
                        ],
                    ],
                    tr![td![code!["csrf()"]], td!["double-submit token for all unsafe methods (see below)"]],
                    tr![td![code!["sessions(store)"]], td!["server-side sessions"]],
                    tr![
                        td![code!["security_headers()"]],
                        td!["the default security headers (applied automatically unless disabled)"],
                    ],
                ],
            ],
        ],
        p![
            "The rate limiter is per process. For fleets, implement ",
            code!["Middleware"],
            " on top of a shared store such as Redis.",
        ],
        h2![
            id("authentication-primitives"),
            a![class("anchor"), href("#authentication-primitives"), "Authentication primitives"],
        ],
        p![
            "Next Rust doesn't ship an authentication provider. It gives you the pieces to build JWT, session, OAuth or custom authentication:",
        ],
        ul![
            li![
                strong!["Middleware"],
                " decides who the user is and stores it: ",
                code!["req.insert_extension(AuthUser(user))"],
                ".",
            ],
            li![
                strong!["Pages and layouts"],
                " read it:",
                pre![code![
                    class("language-rust"),
                    r#"pub fn Page(auth: Auth<User>) -> Result<impl View> {
    let user = auth.require("/login")?;       // redirect guests
    Ok(h1![format!("Hi {}", user.name)])
}"#,
                ],],
                " ",
            ],
            li![strong!["API routes"], " use ", code!["req.extension::<AuthUser<User>>()"], "."],
            li![strong!["Server actions"], " use ", code!["ActionContext::extension"], "."],
            li![
                strong!["Route protection"],
                " uses nested ",
                code!["middleware.rs"],
                " or ",
                code!["protected(..)"],
                ".",
            ],
            li![
                strong!["Cookies"],
                " default to ",
                code!["HttpOnly"],
                ", ",
                code!["Secure"],
                " (outside development) and ",
                code!["SameSite=Lax"],
                ".",
            ],
        ],
        h2![id("sessions"), a![class("anchor"), href("#sessions"), "Sessions"]],
        pre![code![class("language-rust"), r"App::new(routes()).middleware(sessions(MemorySessionStore::default()))",],],
        pre![code![
            class("language-rust"),
            r#"pub fn Page(Extension(session): Extension<Session>) -> impl View {
    let visits: u32 = session.get("visits").unwrap_or(0) + 1;
    session.insert("visits", visits);
    p![format!("Visits: {visits}")]
}"#,
        ],],
        ul![
            li![
                "The cookie ",
                code!["nr_session"],
                " holds only a random 256-bit id. Data stays on the server, so nothing needs signing.",
            ],
            li![
                code!["session.regenerate()"],
                " after login prevents session fixation. ",
                code!["session.destroy()"],
                " logs out.",
            ],
            li![
                code!["MemorySessionStore"],
                " is for development and single instances. Implement ",
                code!["SessionStore"],
                " (three async methods) for Redis, SQL, etc.",
            ],
        ],
        h2![id("csrf"), a![class("anchor"), href("#csrf"), "CSRF"]],
        ul![
            li![
                p![
                    strong!["Server actions"],
                    " are protected by default. Cross-site requests are rejected based on ",
                    code!["Sec-Fetch-Site"],
                    " and ",
                    code!["Origin"],
                    " compared to ",
                    code!["Host"],
                    " (",
                    code!["[security] csrf = \"origin\""],
                    "). Extra trusted origins go in ",
                    code!["[security] allowed_origins"],
                    ".",
                ],
                " ",
            ],
            li![
                p![
                    strong!["Your own POST forms and API routes"],
                    " can add ",
                    code!["csrf()"],
                    " middleware, which requires a token matching the ",
                    code!["nr_csrf"],
                    " cookie in the ",
                    code!["x-csrf-token"],
                    " header or a ",
                    code!["_csrf"],
                    " form field:",
                ],
                " ",
                pre![code![
                    class("language-rust"),
                    r#"pub fn Page(csrf: CsrfToken) -> impl View {
    form![method("post"), action("/api/settings"), csrf.field(), button!["Save"]]
}"#,
                ],],
                " ",
            ],
        ],
        h2![
            id("cors-example-single-api-route"),
            a![class("anchor"), href("#cors-example-single-api-route"), "CORS example (single API route)"],
        ],
        p!["For CORS on a subtree, put a ", code!["middleware.rs"], " there:"],
        pre![code![
            class("language-rust"),
            r#"// app/api/public/middleware.rs
pub async fn middleware(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.set_header("access-control-allow-origin", "*");
    res
}"#,
        ],],
    ]
}
