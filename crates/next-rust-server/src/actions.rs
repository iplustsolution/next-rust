//! Server actions: `#[server_action]` functions exposed at
//! `POST /_nr/action/<id>`.
//!
//! Security:
//!
//! * Only `POST` is accepted.
//! * Cross-site requests are rejected using `Sec-Fetch-Site` / `Origin`
//!   compared to the request host (`[security] csrf = "origin"`, default),
//!   plus a double-submit token with `csrf = "token"`.
//! * Action ids are hashes; the function body is never sent to clients.
//! * Errors are reduced to public messages in production.

use std::collections::BTreeMap;
use std::future::Future;

use http::HeaderMap;
use next_rust_core::config::CsrfMode;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Cookie;
use crate::app::AppInner;
use crate::cookies::Cookies;
use crate::error::{Error, ErrorKind, Result};
use crate::request::Request;
use crate::response::Response;

pub const FLASH_COOKIE: &str = "nr_flash";

/// Stable URL-safe hash of an action id.
pub fn action_hash(id: &str) -> String {
    next_rust_assets::content_hash(id.as_bytes())
}

/// URL of an action.
pub fn action_url(id: &str) -> String {
    format!("/_nr/action/{}", action_hash(id))
}

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

pub(crate) async fn handle(inner: &AppInner, req: Request) -> Response {
    if req.method() != http::Method::POST {
        return Response::text("Method Not Allowed").with_status(405).with_header("allow", "POST");
    }
    let hash = req.path().trim_start_matches("/_nr/action/");
    let Some(&index) = inner.actions.get(hash) else {
        return Response::text("Unknown server action").with_status(404);
    };
    if inner.config.security.csrf != CsrfMode::Off && !same_origin(&req, &inner.config.security.allowed_origins) {
        crate::log::warn("server action rejected: cross-origin request");
        return Response::text("Forbidden: cross-origin server action").with_status(403);
    }
    (inner.routes.actions[index].handler)(req).await
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
/// is loaded.
#[derive(Debug, Clone, Copy)]
pub struct ActionRef {
    pub id: &'static str,
}

impl ActionRef {
    pub const fn new(id: &'static str) -> Self {
        ActionRef { id }
    }

    pub fn url(&self) -> String {
        action_url(self.id)
    }
}

impl next_rust_view::Part for ActionRef {
    fn apply(self, el: &mut next_rust_view::Element) {
        el.set_attr(next_rust_view::attrs::action(self.url()));
        el.set_attr(next_rust_view::attrs::method("post"));
        el.set_attr(next_rust_view::attrs::data("nr-action", self.url()));
    }
}
