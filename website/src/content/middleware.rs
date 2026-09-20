//! The "middleware" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "Middleware is the doorman of your site. Every request passes it on the way in and every response passes \
             it on the way out, so it is the one place to check a ticket, turn someone away, note the time, or stamp \
             something on the reply, without repeating that code in every page."
        ),
        p![
            "Typical jobs: send guests to the login page, limit how many requests one visitor may make, add a header \
             to every response, measure how long requests take. A site needs no middleware at all; without a ",
            code!["middleware.rs"],
            " file requests go straight to the page or API route.",
        ],
        p!["Words used on this page:"],
        ul![
            li![
                strong!["Request"],
                " and ",
                strong!["response"],
                ": what the browser sends (a URL, headers, maybe a form) and what the server sends back (a status \
                 code, headers, the page).",
            ],
            li![
                strong!["Header"],
                ": a named note attached to a request or response, such as ",
                code!["authorization"],
                " or ",
                code!["cache-control"],
                ".",
            ],
            li![
                strong!["Extension"],
                ": a value middleware attaches to the request for later code to read, such as the signed-in user.",
            ],
            li![
                strong!["Session"],
                ": the server's memory of one visitor between requests, found again through a cookie.",
            ],
            li![
                strong!["CSRF"],
                " (cross-site request forgery): another website tricking a signed-in visitor's browser into \
                 submitting something to yours.",
            ],
            li![
                strong!["CORS"],
                " (cross-origin resource sharing): the browser rule that decides whether a script on another site \
                 may read your API's answers.",
            ],
        ],
        h2![id("middleware-files"), a![class("anchor"), href("#middleware-files"), "Middleware files"]],
        p![
            "A file named ",
            code!["middleware.rs"],
            " inside ",
            code!["app/"],
            " guards the folder it sits in and everything below it. It must export one function with exactly this \
             shape: it receives the request and ",
            code!["next"],
            " (the rest of the chain), and returns a response. Calling ",
            code!["next.run(req).await"],
            " lets the request continue; returning without calling it stops the request there. Any other signature \
             is build error ",
            code!["NR0205"],
            ".",
        ],
        p![
            "The first example is ",
            code!["app/middleware.rs"],
            ", at the top of ",
            code!["app/"],
            ", so it sees every request. It serves ",
            code!["/home"],
            " as if the visitor had asked for ",
            code!["/"],
            " (a rewrite: the address bar does not change) and adds a timing header to every response:",
        ],
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
        p![
            "The second example is ",
            code!["app/admin/middleware.rs"],
            ". Because of where it lives it only runs for ",
            code!["/admin"],
            " and the pages under it. It lets administrators through, answers other signed-in users with 403 \
             (forbidden) and sends guests to the login page:",
        ],
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
        p![
            "Create the file by hand or with ",
            code!["next-rust generate middleware /admin"],
            ". Like every file under ",
            code!["app/"],
            ", a new one needs a rebuild; ",
            code!["next-rust dev"],
            " does that for you.",
        ],
        h2![id("order"), a![class("anchor"), href("#order"), "Order"]],
        p!["Several middleware can apply to one request. They run from the outside in, like doors in a corridor."],
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
        p![
            "Global middleware is not a file under ",
            code!["app/"],
            "; it is added in code where the server is started, and it runs first, for everything. This is where \
             the ready-made pieces below (request ids, CORS, rate limits, sessions) are switched on. A new project's ",
            code!["src/main.rs"],
            " is the single line ",
            code!["next_rust::app!();"],
            ", which has no room for options. Replace ",
            code!["next_rust::app!()"],
            " with ",
            code!["routes!()"],
            " and your own ",
            code!["main"],
            ". The whole of ",
            code!["src/main.rs"],
            " then looks like this:",
        ],
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
        p![
            "These come with the framework. Each is a function that returns middleware, to pass to ",
            code!["App::middleware(..)"],
            " as in the example above.",
        ],
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
            "In plain terms: ",
            code!["request_id"],
            " gives every request a reference number you can find again in the logs; ",
            code!["cors"],
            " lets a website on another domain call your API; ",
            code!["rate_limit"],
            " answers 429 (too many requests) when one visitor sends too much; ",
            code!["protected"],
            " keeps guests out of a part of the site.",
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
            "Next Rust doesn't ship an authentication provider. It gives you the pieces to build JWT, session, OAuth or custom authentication. (The optional auth kit with sessions, passwords and rate limits built on these pieces is described in ",
            a![href("/docs/auth"), "Auth & rate limits"],
            ".) The idea is a relay: middleware works out who is asking once, and everything after it just reads the answer.",
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
        p![
            "A session is a small box of data the server keeps per visitor: a visit counter, a shopping cart, the \
             signed-in user's id. The visitor's browser holds only the key to the box (a cookie). Switch sessions on \
             with one line of global middleware in ",
            code!["src/main.rs"],
            ":",
        ],
        pre![code![class("language-rust"), r"App::new(routes()).middleware(sessions(MemorySessionStore::default()))",],],
        p![
            "Then any page can ask for the session. This page, for example ",
            code!["app/visits/page.rs"],
            ", counts how often the same visitor has opened it. Without the ",
            code!["sessions(..)"],
            " line above, the page answers with a 500 error because there is no session to hand over.",
        ],
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
                code!["next_rust_session"],
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
        p!["The attack: a visitor is signed in to your site, opens a malicious page elsewhere, and that page quietly \
             submits a form to your site using the visitor's cookies. The defence is to make sure a state-changing \
             request really came from one of your own pages.",],
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
                    code!["next_rust_csrf"],
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
        p![
            "By default a script running on another website may not read your API's responses. An ",
            code!["access-control-allow-origin"],
            " header lifts that for the origins you name (",
            code!["*"],
            " means anyone, which is only right for public, cookie-less data). For CORS on a subtree, put a ",
            code!["middleware.rs"],
            " there. This one, ",
            code!["app/api/public/middleware.rs"],
            ", opens up everything under ",
            code!["/api/public"],
            " and nothing else:",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/api/public/middleware.rs
pub async fn middleware(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.set_header("access-control-allow-origin", "*");
    res
}"#,
        ],],
        p![
            "For preflight requests, credentials and exposed headers, use the built-in ",
            code!["cors(Cors::default()..)"],
            " instead of setting headers by hand.",
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        ul![
            li![
                "Look at the response headers. With the first example in place, this terminal command prints the \
                 timing header the middleware added: ",
                code!["curl -sI http://localhost:3000/ | grep -i server-timing"],
                ".",
            ],
            li![
                "For a guard, request the protected URL without signing in and expect the redirect: ",
                code!["curl -sI http://localhost:3000/admin"],
                " should answer ",
                code!["307"],
                " with ",
                code!["location: /login"],
                ".",
            ],
            li![
                "In a test, ",
                code!["TestClient"],
                " runs the same chain: see ",
                a![href("/docs/testing"), "Testing"],
                ".",
            ],
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["Forgetting to call "],
                code!["next.run(req).await"],
                ". The request never reaches the page, and every URL answers with whatever the middleware returned.",
            ],
            li![
                strong!["The wrong signature."],
                " The function is named ",
                code!["middleware"],
                ", is ",
                code!["pub async"],
                " and takes exactly ",
                code!["(Request, Next)"],
                ". Extractors such as ",
                code!["Cookies"],
                " are for pages; here you read ",
                code!["req.cookies()"],
                " and ",
                code!["req.header(..)"],
                ".",
            ],
            li![
                strong!["Expecting a nested file to see other folders."],
                " ",
                code!["app/admin/middleware.rs"],
                " never runs for ",
                code!["/account"],
                ". Put shared checks in ",
                code!["app/middleware.rs"],
                " or in a route group folder that contains both.",
            ],
            li![
                strong!["Heavy work in the root middleware."],
                " It also runs for static files and unknown URLs, so a database query there is paid on every image \
                 request. Check the path first, or move the check down into the folder that needs it.",
            ],
            li![
                strong!["Redirecting to a URL taken from the request."],
                " Check it with ",
                code!["is_local_path"],
                " first, or an attacker can craft a login link that ends on their site.",
            ],
        ],
    ]
}
