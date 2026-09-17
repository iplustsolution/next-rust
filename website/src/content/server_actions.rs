//! The "server-actions" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "A server action is an async Rust function that browsers can invoke. It works as a plain HTML form submission (no JavaScript needed) and as a JSON call from the client runtime.",
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
            code!["POST /_nr/action/<id>"],
            ". The id is a hash of the source file path and function name, so it stays stable across builds. Actions in ",
            code!["src/"],
            " must be reachable as ",
            code!["crate::path::to::module"],
            " from the crate root.",
        ],
        h2![id("signatures"), a![class("anchor"), href("#signatures"), "Signatures"]],
        pre![code![
            class("language-rust"),
            r"#[server_action] pub async fn refresh() -> Result<Stats>
#[server_action] pub async fn save(input: Draft) -> Result<Draft>
#[server_action] pub async fn logout(ctx: ActionContext, _input: ()) -> Result<()>",
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
                " (for example the authenticated user).",
            ],
            li![
                "On ",
                code!["wasm32"],
                " targets, ",
                code!["#[server_action]"],
                " functions are compiled out entirely. See ",
                a![href("/docs/client#serverclient-boundary"), "client.md"],
                ".",
            ],
        ],
        h2![id("forms"), a![class("anchor"), href("#forms"), "Forms"]],
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
                            " back. Errors, the message, and non-sensitive submitted values are stored in a 60-second flash cookie",
                        ],
                    ],
                ],
            ],
        ],
        p![
            "The page reads them with the ",
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
            " events on the form.",
        ],
        p![
            code!["FormState"],
            " makes a page dynamic. Pages that only render a form without reading its state can stay static.",
        ],
        h2![id("json-calls"), a![class("anchor"), href("#json-calls"), "JSON calls"]],
        pre![code![class("language-js"), r#"const user = await nextRust.action("/_nr/action/…", { email, name });"#,],],
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
            " and pass it to an island. See the ",
            code!["server-actions"],
            " example, where an island refreshes a counter via ",
            code!["data-nr-on-click=\"action:<url>->count\""],
            ".",
        ],
        h2![id("security"), a![class("anchor"), href("#security"), "Security"]],
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
            li![code!["[security] csrf = \"off\""], " disables the check. Don't."],
            li!["Validate input inside the action. Deserialization only checks shape."],
            li![
                "In production, internal error messages are replaced by \"Internal Server Error\". Validation messages and ",
                code!["Error::http"],
                " messages are public.",
            ],
            li!["Multipart bodies are rejected (", code!["415"], "). Use an API route for uploads."],
        ],
    ]
}
