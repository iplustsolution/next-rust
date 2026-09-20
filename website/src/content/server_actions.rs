//! The "server-actions" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "A server action is a form that hands its answers directly to a Rust function. You write the function \
             (\"create this account\", \"save this comment\"), point a form at it, and the framework does the \
             plumbing in between: it invents the web address, carries the fields across, checks that the request \
             really came from your page, and brings errors back next to the right field. The function runs on your \
             server, never in the visitor's browser, so it can safely touch the database."
        ),
        p![
            "Without server actions you would write an ",
            a![href("/docs/api-routes"), "API route"],
            " for every form, plus JavaScript to call it and show the errors. Actions are the shortcut for the common \
             case; API routes remain the tool for callers that are not your own pages.",
        ],
        p![
            "A server action is an async Rust function that browsers can invoke. It works as a plain HTML form submission (no JavaScript needed) and as a JSON call from the client runtime.",
        ],
        p![
            "The example below is a file ",
            code!["src/actions.rs"],
            " (any name works; remember ",
            code!["mod actions;"],
            " in ",
            code!["src/main.rs"],
            " so it is part of the crate). ",
            code!["Signup"],
            " describes the form's fields, one per ",
            code!["name(..)"],
            " of an input. The function checks the email, returns a per-field error when it is wrong, and otherwise \
             creates the user. The attribute ",
            code!["#[server_action]"],
            " is what makes it callable from a form:",
        ],
        pre![code![
            class("language-rust"),
            r#"// src/actions.rs (or any special file in app/)
use next_rust::prelude::*;

#[derive(serde::Deserialize)]
pub struct Signup {
    email: String,
    name: String,
}

#[server_action]
pub async fn signup(input: Signup) -> Result<User> {
    if !input.email.contains('@') {
        return Err(Error::validation([("email", "Enter a valid email address")]));
    }
    let user = db::create_user(&input.email, &input.name).await?;
    Ok(user)
}"#,
        ],],
        p![
            "The build finds ",
            code!["#[server_action]"],
            " functions in the app directory and in ",
            code!["src/"],
            ", and serves them at ",
            code!["POST /_next-rust/action/<token>"],
            ". There is no fixed URL: every page view gets its own signed, expiring token, bound to the visitor's browser (see ",
            a![href("#security"), "Security"],
            "). Actions in ",
            code!["src/"],
            " must be reachable as ",
            code!["crate::path::to::module"],
            " from the crate root.",
        ],
        h2![id("next-to-a-page"), a![class("anchor"), href("#next-to-a-page"), "Actions next to a page"]],
        p![
            "Other ",
            code![".rs"],
            " files in the app directory are not routes, so a page can keep its actions in a file of their own and declare it as a module. The build follows ",
            code!["#[path]"],
            " module declarations of every special file (and theirs in turn) and registers the actions it finds. \
             Below, two files in the same folder: ",
            code!["page.rs"],
            " is the page with the form, ",
            code!["function.rs"],
            " (the name is yours to choose) holds what happens on submit. \"Special file\" means one of the names \
             the framework gives meaning to: ",
            code!["page.rs"],
            ", ",
            code!["layout.rs"],
            ", ",
            code!["route.rs"],
            " and so on.",
        ],
        pre![code![
            class("language-rust"),
            r#"// app/(auth)/signup/page.rs
#[path = "function.rs"]
pub mod function;

pub fn Page() -> impl View {
    form![action!(function::create_account), /* fields */]
}

// app/(auth)/signup/function.rs
#[server_action]
pub async fn create_account(ctx: ActionContext, input: NewAccount) -> Result<()> { /* ... */ }"#,
        ],],
        p![
            "The module must be ",
            code!["pub"],
            " (or ",
            code!["pub(crate)"],
            "): the route table registers its actions from outside the page. A private one that holds actions is ",
            code!["NR0211"],
            ". Only ",
            code!["#[path = \"...\"]"],
            " declarations are followed, resolved next to the declaring file.",
        ],
        h2![id("signatures"), a![class("anchor"), href("#signatures"), "Signatures"]],
        p![
            "An action takes nothing, the submitted data, the request context, or both, in that order. These are \
             the four shapes; more than two arguments is build error ",
            code!["NR0210"],
            ":",
        ],
        pre![code![
            class("language-rust"),
            r"#[server_action] pub async fn refresh() -> Result<Stats>
#[server_action] pub async fn save(input: Draft) -> Result<Draft>
#[server_action] pub async fn logout(ctx: ActionContext) -> Result<()>
#[server_action] pub async fn rename(ctx: ActionContext, input: Rename) -> Result<()>",
        ],],
        ul![
            li![
                "The input must implement ",
                code!["Deserialize"],
                ". Form posts are decoded as ",
                code!["application/x-www-form-urlencoded"],
                ", everything else as JSON.",
            ],
            li!["The output must implement ", code!["Serialize"], "."],
            li![
                code!["ActionContext"],
                " gives access to ",
                code!["cookies"],
                ", ",
                code!["headers"],
                " and ",
                code!["extensions"],
                " (for example the authenticated user), and ",
                code!["client_ip()"],
                " for per-client limits (it reads ",
                code!["X-Forwarded-For"],
                " only with ",
                code!["[server] trust_proxy"],
                ").",
            ],
            li![
                code!["#[server_action(body_limit = 30 * 1024 * 1024)]"],
                " lets one action accept a larger body than ",
                code!["[server] body_limit"],
                " (2 MiB by default), for an upload sent as base64. The value must be a constant. It applies only after the request has passed the action's origin and token checks, so only a page that rendered the action can send the larger body; every other action and route keeps the server's limit.",
            ],
            li![
                "On ",
                code!["wasm32"],
                " targets, ",
                code!["#[server_action]"],
                " functions are compiled out entirely. See ",
                a![href("/docs/client#serverclient-boundary"), "Client & navigation"],
                ".",
            ],
        ],
        h2![id("forms"), a![class("anchor"), href("#forms"), "Forms"]],
        p![
            "This is the page that uses the action, for example ",
            code!["app/signup/page.rs"],
            ". Three things to notice: ",
            code!["action!(..)"],
            " connects the form to the function; ",
            code!["FormState"],
            " brings back the errors and what the visitor typed after a failed submit; and the hidden ",
            code!["_redirect"],
            " field says where to go after success.",
        ],
        pre![code![
            class("language-rust"),
            r#"use crate::actions;

pub fn Page(form: FormState) -> impl View {
    form![
        action!(actions::signup),
        label!["Email ", input![name("email"), r#type("email"), value(form.value("email"))]],
        small![data("nr-error", "email"), form.error("email").unwrap_or_default().to_owned()],
        label!["Name ", input![name("name"), value(form.value("name"))]],
        input![r#type("hidden"), name("_redirect"), value("/welcome")],
        button![r#type("submit"), "Sign up"],
    ]
}"#,
        ],],
        p![
            code!["action!(path)"],
            " sets ",
            code!["method=\"post\""],
            ", the action URL, and ",
            code!["data-nr-action"],
            " for enhancement.",
        ],
        div![
            class("callout"),
            p![
                strong!["Want the server's message to be the one people see?"],
                " Add ",
                code!["novalidate"],
                " to the form. Otherwise an ",
                code!["input![r#type(\"email\")]"],
                " lets the browser block the submit with its own bubble, and the ",
                code!["data-nr-error"],
                " slot never fills.",
            ],
        ],
        p!["Every action form works two ways, and you write it once. The plain way is how forms have always \
             worked: the browser sends the fields, the server answers with a redirect (status 303, \"now go to \
             this page\"), the browser loads that page. The enhanced way needs the client runtime (the framework's \
             small script): it sends the same fields in the background and updates the page without reloading it.",],
        p![strong!["Without JavaScript"], ", the browser posts the form:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["action result"], th!["response"]]],
                tbody![
                    tr![
                        td![code!["Ok(_)"]],
                        td![
                            code!["303"],
                            " to ",
                            code!["_redirect"],
                            " (local paths only) or back to the same-origin referring page",
                        ],
                    ],
                    tr![td![code!["Err(redirect(url))"]], td![code!["303"], " to ", code!["url"]]],
                    tr![
                        td![code!["Err(Error::validation(..))"], " or another error"],
                        td![
                            code!["303"],
                            " back to the form's page (the same-origin referrer; ",
                            code!["_redirect"],
                            " only when there is none). Errors, the message, and non-sensitive submitted values are stored in a 60-second flash cookie",
                        ],
                    ],
                ],
            ],
        ],
        p![
            "A flash cookie is a note the server leaves for the very next page view and deletes once read; it is how \
             error messages survive the redirect. The page reads them with the ",
            code!["FormState"],
            " extractor: ",
            code!["form.error(\"email\")"],
            ", ",
            code!["form.value(\"email\")"],
            ", ",
            code!["form.message"],
            ", ",
            code!["form.has_errors()"],
            ". The flash is consumed once. Fields named like ",
            code!["password"],
            " or ",
            code!["token"],
            ", or starting with ",
            code!["_"],
            ", are never echoed back.",
        ],
        p![
            strong!["With the client runtime"],
            " loaded, the form is submitted with ",
            code!["fetch"],
            ". The runtime writes validation errors into elements with a matching ",
            code!["data-nr-error=\"field\""],
            ", follows redirects client-side, and on success calls ",
            code!["nextRust.refresh()"],
            " or navigates to ",
            code!["_redirect"],
            ". It also dispatches ",
            code!["nr:success"],
            " / ",
            code!["nr:error"],
            " events on the form. While it is being sent the form has ",
            code!["aria-busy"],
            " and its submit button is disabled; a second submit in that time is ignored.",
        ],
        p![
            "Forms that are not server actions (a GET search form, a POST to a ",
            code!["route.rs"],
            ") are sent without a page reload too: see ",
            a![href("/docs/client#forms"), "Forms never reload the page"],
            ".",
        ],
        p![
            code!["FormState"],
            " makes a page dynamic. Pages that only render a form without reading its state can stay static.",
        ],
        p![
            "The client runtime is only sent to pages that need it: a page with an internal link, an island or an \
             action form (action forms count from version 0.1.8 on). On a page without it, the form still works the \
             plain way, with a full page load. That is why the ",
            code!["_redirect"],
            " field matters even if all your visitors have JavaScript.",
        ],
        h3![id("result-in-place"), a![class("anchor"), href("#result-in-place"), "Showing the result in place"]],
        p![
            "A newsletter box in a footer should say \"You're on the list\" where it is, not reload the page. ",
            code!["stay_on_success(true)"],
            " keeps the page as it is after a successful submit; elements marked with ",
            code!["action_result(..)"],
            " show what the action returned (as text), and ",
            code!["reset_on_success(true)"],
            " clears the fields. The form goes wherever the box is rendered (a layout or a component), the action in ",
            code!["src/actions.rs"],
            " or next to it:",
        ],
        pre![code![
            class("language-rust"),
            r##"form![
    action!(subscribe),
    stay_on_success(true),
    reset_on_success(true),
    class("group"),
    input![r#type("email"), name("email"), required(true)],
    small![data("nr-error", "email")],
    button![r#type("submit"), "Subscribe"],
    p![class("hidden group-data-[nr-state=success]:block"), action_result("")],
    input![r#type("hidden"), name("_redirect"), value("/newsletter/thanks")],   // without JavaScript
]

#[server_action]
pub async fn subscribe(input: Subscribe) -> Result<String> {
    newsletter::add(&input.email).await?;
    Ok("You're on the list. Check your inbox to confirm.".into())
}"##,
        ],],
        p![
            "After every enhanced submit the form carries ",
            code!["data-nr-state=\"success\""],
            " or ",
            code!["\"error\""],
            ", so a success panel can be shown with CSS alone. ",
            code!["action_result(\"message\")"],
            " shows one field of a returned object. Without JavaScript the browser still posts the form and goes to ",
            code!["_redirect"],
            ", so point it at a confirmation page.",
        ],
        h2![id("json-calls"), a![class("anchor"), href("#json-calls"), "JSON calls"]],
        p![
            "An action can also be called from your own browser script, without a form: a \"like\" button, an \
             autosave. This line belongs in a script under ",
            code!["client/"],
            " or ",
            code!["assets/"],
            " (see ",
            a![href("/docs/client"), "Client & navigation"],
            "); it sends the object as JSON and resolves with what the action returned:",
        ],
        pre![code![
            class("language-js"),
            r#"const user = await nextRust.action(url, { email, name }); // url from action!(signup).url()"#,
        ],],
        p![
            "Responses are ",
            code!["{\"ok\":true,\"data\":…}"],
            ", ",
            code!["{\"ok\":false,\"errors\":{…}}"],
            " (422), ",
            code!["{\"ok\":false,\"error\":\"…\"}"],
            ", or ",
            code!["{\"ok\":false,\"redirect\":\"/…\"}"],
            " (followed automatically). From Rust, get the URL with ",
            code!["action!(signup).url()"],
            " and pass it to an island. It returns a placeholder that each HTML response replaces with a URL signed for the visitor, so use it only in rendered pages, not in JSON API responses. See the ",
            code!["server-actions"],
            " example, where an island refreshes a counter via ",
            code!["data-nr-on-click=\"action:<url>->count\""],
            ".",
        ],
        h2![id("security"), a![class("anchor"), href("#security"), "Security"]],
        p![
            "A function anyone on the internet can call needs guarding. The framework does most of it for you; this \
             section explains what it does, what you must set (",
            code!["NEXT_RUST_SECRET"],
            ") and what remains your job (checking who the user is).",
        ],
        p!["Action URLs are dynamic. For every page view the server mints a token for each action on the page. A \
             token is an unguessable ticket; HMAC-SHA256 is the signature that proves the server issued it and \
             nobody altered it:",],
        pre![code![r"/_next-rust/action/<base64url(version | action key | expiry | nonce | HMAC-SHA256 tag)>"]],
        ul![
            li![
                strong!["Action key"],
                ": an HMAC of the action's id under the server secret. It can't be derived from your source code, so actions can't be guessed or enumerated.",
            ],
            li![strong!["Nonce"], ": 64 random bits. No two page views share a URL."],
            li![
                strong!["Expiry"],
                ": ",
                code!["[security] action_token_ttl"],
                " (12 hours by default). After that the page shows \"This page is out of date. Reload it and try again.\"",
            ],
            li![
                strong!["Binding"],
                ": the tag also covers a random value in the ",
                code!["__Host-next_rust_bind"],
                " cookie (",
                code!["HttpOnly"],
                ", ",
                code!["Secure"],
                ", ",
                code!["SameSite=Strict"],
                "; ",
                code!["next_rust_bind"],
                " in development). A URL copied out of one browser is rejected from any other client, and cross-site requests never carry the cookie.",
            ],
        ],
        p![
            "Tokens are signed with ",
            code!["NEXT_RUST_SECRET"],
            " (at least 32 bytes; generate one with ",
            code!["openssl rand -hex 32"],
            "). Set it in production: without it each process picks a random secret, so pages left open stop working after a restart and URLs don't work across instances. A secret shorter than 32 bytes is a startup error. Pages containing action URLs are sent as ",
            code!["Cache-Control: private, no-store"],
            ", including statically rendered ones (the server still caches their HTML and fills in fresh URLs per request). Action URLs are left out of request logs.",
        ],
        p![
            strong!["What tokens don't do"],
            ": they prove a request comes from a browser that loaded one of your pages, not who the user is. Anyone can open a public page and get their own token. Check authentication and authorization inside every action that needs them, as you would in an API route.",
        ],
        p!["The other checks, in the order they run:"],
        ul![
            li!["Only ", code!["POST"], " is accepted."],
            li![
                "Cross-site requests are rejected (",
                code!["403"],
                "). Browsers send ",
                code!["Sec-Fetch-Site"],
                " / ",
                code!["Origin"],
                ", which must match the request host or ",
                code!["[security] allowed_origins"],
                ". Clients that send neither (curl, server-to-server) aren't browsers and can't carry a victim's cookies. Protect those calls with authentication as usual.",
            ],
            li![
                "Unknown, expired, tampered or copied tokens are rejected (",
                code!["403"],
                ") before any action code runs.",
            ],
            li![
                "Only ",
                code!["application/json"],
                " and ",
                code!["application/x-www-form-urlencoded"],
                " bodies are accepted (",
                code!["415"],
                " otherwise), so a cross-site ",
                code!["text/plain"],
                " form can't reach the JSON parser.",
            ],
            li![
                code!["[security] csrf = \"token\""],
                " also requires the ",
                code!["next_rust_csrf"],
                " cookie echoed in ",
                code!["x-csrf-token"],
                " (the client runtime does this) or a ",
                code!["_csrf"],
                " field (add ",
                code!["csrf.field()"],
                " to forms that must work without JavaScript).",
            ],
            li![
                code!["[security] csrf = \"off\""],
                " disables the origin check. Don't. Signed URLs are checked regardless.",
            ],
            li!["Responses carry ", code!["Cache-Control: no-store"], "."],
            li!["Validate input inside the action. Deserialization only checks shape."],
            li![
                "In production, internal error messages are replaced by \"Internal Server Error\". Validation messages and ",
                code!["Error::http"],
                " messages are public.",
            ],
            li!["Multipart bodies are rejected (", code!["415"], "). Use an API route for uploads."],
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        ol![
            li![
                "Run ",
                code!["next-rust dev"],
                ", open the page and view its source: the ",
                code!["<form>"],
                " has ",
                code!["method=\"post\""],
                ", a ",
                code!["data-nr-action"],
                " attribute and an ",
                code!["action"],
                " starting with ",
                code!["/_next-rust/action/"],
                ".",
            ],
            li![
                "Submit a wrong value. The message appears in the ",
                code!["data-nr-error"],
                " element and what you typed is still there.",
            ],
            li![
                "Turn JavaScript off in the browser's developer tools and submit again. The page reloads, shows the \
                 same message, and a correct submit ends on the ",
                code!["_redirect"],
                " page. If this works, both paths work.",
            ],
            li![
                "For a permanent check, post to the action from a test with ",
                code!["client.post_form(&client.action_url(\"src/actions.rs::signup\"), \"email=a%40b.c\")"],
                ": see ",
                a![href("/docs/testing"), "Testing"],
                ".",
            ],
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["The browser's own bubble instead of your message."],
                " Add ",
                code!["novalidate(true)"],
                " to the form when the server's wording should win.",
            ],
            li![
                strong!["No "],
                code!["_redirect"],
                strong![" field."],
                " Without JavaScript a successful submit then returns to the form's own page, which looks as if \
                 nothing happened.",
            ],
            li![
                strong!["Field names that do not match."],
                " Each ",
                code!["name(\"email\")"],
                " must be a field of the input struct, and the key in ",
                code!["Error::validation([(\"email\", ..)])"],
                " must match the ",
                code!["data-nr-error"],
                " slot. A mismatch is a 422 with no visible message.",
            ],
            li![
                strong!["Building the action URL by hand, or returning it from a JSON API."],
                " URLs are signed per visitor and per page view; only ",
                code!["action!(..)"],
                " in a rendered page produces a working one.",
            ],
            li![
                strong!["No "],
                code!["NEXT_RUST_SECRET"],
                strong![" in production."],
                " Forms on pages left open stop working after every restart, and randomly behind a load balancer.",
            ],
            li![
                strong!["Treating the token as a login."],
                " It proves the request came from your page, not who sent it. Check the user inside the action.",
            ],
            li![
                strong!["A private module holding actions"],
                " (",
                code!["NR0211"],
                "), or an action file in ",
                code!["src/"],
                " that no ",
                code!["mod"],
                " declaration reaches.",
            ],
            li![
                strong!["File uploads through an action."],
                " Multipart forms are refused; send the file to an API route, or as base64 with a raised ",
                code!["body_limit"],
                ".",
            ],
        ],
    ]
}
