//! Server actions: `#[server_action]` functions exposed at
//! `POST /_nr/action/<token>`.
//!
//! Security (in the order the checks run):
//!
//! * Only `POST` is accepted.
//! * Cross-site requests are rejected using `Sec-Fetch-Site` / `Origin`
//!   compared to the request host (`[security] csrf = "origin"`, default).
//! * There is no fixed URL per action. Every page view gets its own signed,
//!   expiring token bound to the visitor's `HttpOnly`, `SameSite=Strict`
//!   binding cookie (see `action_token.rs`). Unknown, expired, tampered
//!   or copied tokens are rejected before any action code runs.
//! * Only `application/json` and `application/x-www-form-urlencoded` bodies
//!   are accepted, so `text/plain` cross-site form tricks can't reach the
//!   JSON parser.
//! * With `csrf = "token"`, a double-submit token (`x-csrf-token` header or
//!   `_csrf` field matching the `nr_csrf` cookie) is also required.
//! * Errors are reduced to public messages in production, and responses are
//!   never cached.

use std::collections::BTreeMap;
use std::future::Future;

use http::HeaderMap;
use next_rust_core::config::CsrfMode;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Cookie;
use crate::action_token;
use crate::app::AppInner;
use crate::cookies::Cookies;
use crate::error::{Error, ErrorKind, Result};
use crate::request::Request;
use crate::response::Response;

pub const FLASH_COOKIE: &str = "nr_flash";

/// Shown when an action link is expired, copied from another browser or
/// minted by a server with a different secret.
const STALE_MESSAGE: &str = "This page is out of date. Reload it and try again.";

/// Request data available to actions declared as `fn(ctx: ActionContext, input: T)`.
#[derive(Clone)]
pub struct ActionContext {
    pub cookies: Cookies,
    pub headers: HeaderMap,
    pub extensions: http::Extensions,
}

impl ActionContext {
    pub fn extension<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }
}

pub(crate) async fn handle(inner: &AppInner, mut req: Request) -> Response {
    if req.method() != http::Method::POST {
        return Response::text("Method Not Allowed").with_status(405).with_header("allow", "POST");
    }
    let security = &inner.config.security;
    if security.csrf != CsrfMode::Off && !same_origin(&req, &security.allowed_origins) {
        crate::log::warn("server action rejected: cross-origin request");
        return Response::text("Forbidden: cross-origin server action").with_status(403);
    }
    let token = req.path().trim_start_matches("/_nr/action/");
    let binding = action_token::binding(req.cookies());
    let found = action_token::verify(token, binding.as_deref())
        .and_then(|key| inner.actions.get(&key).copied().ok_or(action_token::Rejection::Forged));
    let index = match found {
        Ok(index) => index,
        Err(reason) => {
            match reason {
                action_token::Rejection::Expired => crate::log::debug("server action rejected: expired token"),
                other => crate::log::warn(&format!("server action rejected: {}", other.as_str())),
            }
            return stale(&req);
        }
    };
    if let Some(res) = check_content_type(&req) {
        return res;
    }
    if security.csrf == CsrfMode::Token && !double_submit_ok(&mut req).await {
        crate::log::warn("server action rejected: missing or invalid CSRF token");
        return Response::text("Invalid CSRF token").with_status(403);
    }
    let mut res = (inner.routes.actions[index].handler)(req).await;
    res.set_header("cache-control", "no-store");
    res
}

/// Mint a URL for action `id` bound to `binding` (used by [`crate::TestClient`]).
pub(crate) fn signed_url(id: &str, binding: &str, ttl: u64) -> String {
    format!("/_nr/action/{}", action_token::mint(&action_token::action_key(id), binding, ttl))
}

fn stale(req: &Request) -> Response {
    let res = if wants_json(req) {
        Response::json(&serde_json::json!({ "ok": false, "error": STALE_MESSAGE, "code": "nr_stale" })).with_status(403)
    } else {
        // Plain form post: show the message on the page it came from.
        flash_back(req, &[], BTreeMap::new(), Some(STALE_MESSAGE.to_owned()))
    };
    res.with_header("cache-control", "no-store")
}

fn check_content_type(req: &Request) -> Option<Response> {
    let ct = req.header("content-type")?;
    let base = ct.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
    match base.as_str() {
        "application/json" | "application/x-www-form-urlencoded" => None,
        b if b.starts_with("multipart/") => Some(
            Response::text("multipart bodies are not supported by server actions; use an API route").with_status(415),
        ),
        _ => Some(Response::text("Unsupported Media Type").with_status(415)),
    }
}

async fn double_submit_ok(req: &mut Request) -> bool {
    let Some(cookie) = req.cookies().get(crate::CSRF_COOKIE) else { return false };
    let provided = match req.header("x-csrf-token") {
        Some(v) => Some(v.to_owned()),
        None if is_form(req) => crate::middleware::form_field(req, "_csrf").await,
        None => None,
    };
    provided.is_some_and(|p| crate::constant_time_eq(cookie.as_bytes(), p.as_bytes()))
}

/// `true` unless a browser signals a cross-site request.
pub(crate) fn same_origin(req: &Request, allowed: &[String]) -> bool {
    let fetch_site = req.header("sec-fetch-site");
    if matches!(fetch_site, Some("same-origin") | Some("none")) {
        return true;
    }
    let Some(origin) = req.header("origin") else {
        // Non-browser clients send neither header and cannot carry ambient cookies cross-site.
        return fetch_site.is_none();
    };
    if allowed.iter().any(|a| a == origin) {
        return true;
    }
    let host = if crate::trust_proxy() {
        req.header("x-forwarded-host").or_else(|| req.header("host"))
    } else {
        req.header("host")
    };
    let origin_host = origin.split_once("://").map(|(_, h)| h);
    matches!((origin_host, host), (Some(o), Some(h)) if o.eq_ignore_ascii_case(h))
}

fn is_form(req: &Request) -> bool {
    req.header("content-type").is_some_and(|c| c.starts_with("application/x-www-form-urlencoded"))
}

fn wants_json(req: &Request) -> bool {
    !is_form(req)
        || req.header("x-nr-action").is_some()
        || req.header("accept").is_some_and(|a| a.starts_with("application/json"))
}

async fn read_input<I: DeserializeOwned>(req: &mut Request) -> Result<(I, Vec<(String, String)>)> {
    if req.header("content-type").is_some_and(|c| c.starts_with("multipart/")) {
        return Err(Error::http(415, "multipart bodies are not supported by server actions; use an API route"));
    }
    let bytes = req.bytes().await?;
    if is_form(req) {
        let fields: Vec<(String, String)> = serde_urlencoded::from_bytes(&bytes).unwrap_or_default();
        let input =
            serde_urlencoded::from_bytes(&bytes).map_err(|e| Error::http(400, format!("invalid form data: {e}")))?;
        Ok((input, fields))
    } else {
        let slice: &[u8] = if bytes.iter().all(u8::is_ascii_whitespace) { b"null" } else { &bytes };
        let input = serde_json::from_slice(slice).map_err(|e| Error::http(400, format!("invalid JSON input: {e}")))?;
        Ok((input, Vec::new()))
    }
}

fn local_path(candidate: &str) -> Option<String> {
    (candidate.starts_with('/') && !candidate.starts_with("//") && !candidate.contains('\\'))
        .then(|| candidate.to_owned())
}

fn back_location(req: &Request, fields: &[(String, String)]) -> String {
    if let Some(r) = fields.iter().find(|(k, _)| k == "_redirect").and_then(|(_, v)| local_path(v)) {
        return r;
    }
    let host = req.header("host").unwrap_or_default();
    req.header("referer")
        .and_then(|r| r.split_once("://").map(|(_, rest)| rest.to_owned()))
        .and_then(|rest| rest.strip_prefix(host).map(str::to_owned))
        .and_then(|p| local_path(&p))
        .unwrap_or_else(|| "/".into())
}

fn respond<O: Serialize>(req: &Request, json: bool, fields: &[(String, String)], result: Result<O>) -> Response {
    match (json, result) {
        (true, Ok(data)) => Response::json(&serde_json::json!({ "ok": true, "data": data })),
        (false, Ok(_)) => Response::see_other(&back_location(req, fields)),
        (json, Err(e)) => {
            let status = e.status();
            match e.into_kind() {
                ErrorKind::Redirect { location, .. } => {
                    if json {
                        Response::json(&serde_json::json!({ "ok": false, "redirect": location }))
                    } else {
                        Response::see_other(&location)
                    }
                }
                ErrorKind::Validation(errors) => {
                    if json {
                        Response::json(&serde_json::json!({ "ok": false, "errors": errors })).with_status(422)
                    } else {
                        flash_back(req, fields, errors, None)
                    }
                }
                kind => {
                    let err = Error::new(kind);
                    if status.is_server_error() {
                        crate::log::error(&format!("server action failed: {}", err.detailed_message()));
                    }
                    let message = if crate::is_dev() { err.detailed_message() } else { err.public_message() };
                    if json {
                        Response::json(&serde_json::json!({ "ok": false, "error": message }))
                            .with_status(status.as_u16())
                    } else {
                        flash_back(req, fields, BTreeMap::new(), Some(message))
                    }
                }
            }
        }
    }
}

fn flash_back(
    req: &Request,
    fields: &[(String, String)],
    errors: BTreeMap<String, String>,
    message: Option<String>,
) -> Response {
    let values: BTreeMap<String, String> = fields
        .iter()
        .filter(|(k, _)| {
            !k.starts_with('_')
                && !k.to_ascii_lowercase().contains("password")
                && !k.to_ascii_lowercase().contains("token")
        })
        .map(|(k, v)| (k.clone(), v.chars().take(256).collect()))
        .collect();
    let json = serde_json::json!({ "errors": errors, "message": message, "values": values }).to_string();
    let json: String = json.chars().take(3000).collect();
    req.cookies().set(Cookie::encoded(FLASH_COOKIE, &json).max_age(std::time::Duration::from_secs(60)));
    Response::see_other(&back_location(req, fields))
}

pub(crate) fn decode_flash(raw: &str) -> crate::context::FormState {
    #[derive(serde::Deserialize, Default)]
    struct Flash {
        #[serde(default)]
        errors: BTreeMap<String, String>,
        message: Option<String>,
        #[serde(default)]
        values: BTreeMap<String, String>,
    }
    let f: Flash = serde_json::from_str(raw).unwrap_or_default();
    crate::context::FormState { errors: f.errors, message: f.message, values: f.values }
}

/// Adapter for `async fn action(input: I) -> Result<O>` (used by `#[server_action]`).
pub async fn run_action<I, O, F, Fut>(mut req: Request, f: F) -> Response
where
    I: DeserializeOwned,
    O: Serialize,
    F: FnOnce(I) -> Fut,
    Fut: Future<Output = Result<O>>,
{
    let json = wants_json(&req);
    match read_input::<I>(&mut req).await {
        Ok((input, fields)) => {
            let result = f(input).await;
            respond(&req, json, &fields, result)
        }
        Err(e) => respond::<()>(&req, json, &[], Err(e)),
    }
}

/// Adapter for `async fn action(ctx: ActionContext, input: I) -> Result<O>`.
pub async fn run_action_ctx<I, O, F, Fut>(mut req: Request, f: F) -> Response
where
    I: DeserializeOwned,
    O: Serialize,
    F: FnOnce(ActionContext, I) -> Fut,
    Fut: Future<Output = Result<O>>,
{
    let json = wants_json(&req);
    let ctx = ActionContext {
        cookies: req.cookies().clone(),
        headers: req.headers().clone(),
        extensions: req.extensions().clone(),
    };
    match read_input::<I>(&mut req).await {
        Ok((input, fields)) => {
            let result = f(ctx, input).await;
            respond(&req, json, &fields, result)
        }
        Err(e) => respond::<()>(&req, json, &[], Err(e)),
    }
}

/// Adapter for `async fn action() -> Result<O>`.
pub async fn run_action0<O, F, Fut>(mut req: Request, f: F) -> Response
where
    O: Serialize,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<O>>,
{
    let json = wants_json(&req);
    let fields: Vec<(String, String)> = if is_form(&req) {
        req.bytes().await.ok().and_then(|b| serde_urlencoded::from_bytes(&b).ok()).unwrap_or_default()
    } else {
        Vec::new()
    };
    let result = f().await;
    respond(&req, json, &fields, result)
}

/// Reference to a server action, created with `action!(path::to::fn)`.
///
/// As a part of `form![..]` it sets `method="post"` and the action URL, so
/// the form works without JavaScript and is enhanced when the client runtime
/// is loaded. The URL is different for every visitor and page view.
#[derive(Debug, Clone, Copy)]
pub struct ActionRef {
    pub id: &'static str,
}

impl ActionRef {
    pub const fn new(id: &'static str) -> Self {
        ActionRef { id }
    }

    /// Placeholder URL for rendered HTML. Every HTML response replaces it
    /// with a URL signed for the visitor, so it only works inside a page.
    pub fn url(&self) -> String {
        action_token::marker_url(self.id)
    }
}

/// `on_press = action!(save)` on UI components.
impl From<ActionRef> for next_rust_ui::Press {
    fn from(action: ActionRef) -> Self {
        next_rust_ui::Press::action(action.url())
    }
}

impl next_rust_view::Part for ActionRef {
    fn apply(self, el: &mut next_rust_view::Element) {
        el.set_attr(next_rust_view::attrs::action(self.url()));
        el.set_attr(next_rust_view::attrs::method("post"));
        el.set_attr(next_rust_view::attrs::data("nr-action", self.url()));
    }
}
