//! Server actions, requests, and form enhancement.
//!
//! No form reloads the page. A form that goes to this site is sent from here:
//! a GET form (a search box, filters, a picker) becomes a navigation to the
//! URL it builds, a POST form is posted and the answer shown (the page it
//! redirects to, the page it renders, or its JSON through the same events and
//! `data-nr-error` elements as a server action). The browser keeps the forms
//! it alone can do: another origin, `target`, `method="dialog"`,
//! `text/plain`, and any form or button marked `data-nr-reload` /
//! `data-nr-native`. `[client] enhance_forms = false` leaves every form but
//! server actions alone.
//!
//! [`action`], [`request`] and [`stream`] are the three calls behind
//! `nextRust.action`, `nextRust.request` and `nextRust.stream`; `api` forwards
//! to them.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use crate::host::{self, op};
use crate::nav::{NavOptions, PostBody};
use crate::{dom, json, loader, nav, task};

/// A body a form sends, which is also what `nav`'s `PostBody` carries.
pub enum Body {
    /// Fields, as `URLSearchParams`: every form but a multipart one.
    Params(dom::Node),
    /// The `FormData` itself, which is the only thing that can carry a file.
    FormData(dom::Node),
}

impl Body {
    /// A second owner of the same body, for handing one to a navigation.
    fn share(&self) -> Self {
        match self {
            Self::Params(node) => Self::Params(node.share()),
            Self::FormData(node) => Self::FormData(node.share()),
        }
    }

    fn node(&self) -> &dom::Node {
        match self {
            Self::Params(node) | Self::FormData(node) => node,
        }
    }
}

// `DISPATCH` takes the kind of event to build and the flags of its init.
const EVENT: u32 = 0;
const CUSTOM: u32 = 1;
const SUBMIT: u32 = 2;
const BUBBLES: u32 = 1;
const CANCELABLE: u32 = 2;

crate::local! {
    /// The form → the controller of what it is sending (`sending` in the
    /// source, a `WeakMap` there). The key is the handle of the form as this
    /// send first saw it; entries are *found* by identity, because the shim
    /// hands out a fresh handle every time the same form crosses the ABI.
    static SENDING: RefCell<BTreeMap<u32, Sending>> = RefCell::new(BTreeMap::new());
    /// The form → the debounce its live search is waiting out (`liveTimers`).
    /// Dropping a [`task::Timer`] is `clearTimeout`, so replacing an entry
    /// cancels the keystroke before.
    static LIVE_TIMERS: RefCell<BTreeMap<u32, Live>> = RefCell::new(BTreeMap::new());
    /// `live`: whether the submit being handled is a live search, which is
    /// what makes it quiet and lets it drop the request before it.
    static LIVE: Cell<bool> = Cell::new(false);
    /// `enhanceForms`: the page opted out by rendering `#__next_rust_forms`.
    static ENHANCE: Cell<bool> = Cell::new(true);
    /// Tickets, so a send can tell whether the entry in `SENDING` is still
    /// its own (`sending.get(form) === controller`).
    static TICKET: Cell<u32> = Cell::new(0);
}

struct Sending {
    /// The form, kept so entries can be matched by identity and swept when it
    /// leaves the document.
    form: dom::Node,
    controller: dom::Node,
    ticket: u32,
    /// `controller.live`: only a live search gives way to a newer one.
    live: bool,
}

struct Live {
    form: dom::Node,
    /// Held only to cancel it: dropping the entry is `clearTimeout`.
    _timer: task::Timer,
}

// ---- server actions ---------------------------------------------------------

/// What an action answered with: the raw JSON text of its `data`.
pub struct ActionResult {
    /// `body.data` as it was written, `"null"` when the action sent none.
    pub data: String,
    /// The action answered with a redirect and the page is already on its way
    /// there. The source returns `undefined` for this.
    pub redirected: bool,
}

/// An action that did not succeed: `new Error(body.error)` with `errors`,
/// `status`, `code` and `data` hung off it.
///
/// `code` and `data` are what a caller branches on. An action that refuses
/// for a reason the page can do something about — a quota reached, a plan
/// that needs upgrading, a mode that is blocked — says so in `body.code`, and
/// `body.data` carries whatever goes with it. Dropping them here leaves the
/// caller with nothing but a sentence of English to match against.
pub struct ActionError {
    pub message: String,
    pub errors: Vec<(String, String)>,
    pub status: u16,
    /// `body.code`, when the server sent one.
    pub code: Option<String>,
    /// `body.data` as raw JSON, and `None` when the body carried no `data`
    /// at all. The source writes `err.data = body.data`, which leaves
    /// `undefined` there — not `null` — and a page reading `err.data ===
    /// undefined` has to see the same thing here.
    pub data: Option<String>,
}

/// `NR.action(url, input)`. `input_json` is the input already stringified —
/// `"null"` where the caller passed nothing, as `JSON.stringify(input ===
/// undefined ? null : input)` does.
pub async fn action(url: &str, input_json: &str) -> Result<ActionResult, ActionError> {
    let headers = nav::action_headers(&[("content-type", "application/json")]);
    let res = fetch(url, "POST", &headers, Some(Sent::Text(input_json)), None).await;
    // `await res.json().catch(() => ({ ok: false, error: res.statusText }))`:
    // an answer that is not JSON is a failure with whatever the status said.
    let (text, status, status_text) = match &res {
        Some(res) => (response_text(res).await.unwrap_or_default(), status(res), status_text(res)),
        None => (String::new(), 0, String::new()),
    };
    let fallback = !json::parses(&text);
    let body = if fallback { None } else { json::Object::new(&text) };
    let ok = body.as_ref().is_some_and(|body| body.bool("ok"));
    let redirect = body.as_ref().and_then(|body| body.str("redirect")).filter(|to| !to.is_empty());
    // An action may change what pages show: drop every page kept in memory.
    if ok || redirect.is_some() {
        nav::clear_pages();
    }
    if let Some(to) = redirect {
        let _ = nav::navigate(&to, NavOptions::default()).await;
        return Ok(ActionResult { data: String::from("null"), redirected: true });
    }
    if !ok {
        // `new Error(body.error || "Validation failed")`. The status text is
        // *not* a second fallback: it is what the `catch` above puts in
        // `body.error` when the answer was not JSON at all, so it stands in
        // only then. A body that parsed and carries no `error` — a bare
        // `{"ok":false}` — says "Validation failed" in both runtimes, and a
        // page matching on that message must not get "Unprocessable Entity".
        let message = body
            .as_ref()
            .and_then(|body| body.str("error"))
            .filter(|error| !error.is_empty())
            .or_else(|| (fallback && !status_text.is_empty()).then_some(status_text))
            .unwrap_or_else(|| String::from("Validation failed"));
        let errors = body
            .as_ref()
            .and_then(|body| body.raw("errors"))
            .and_then(json::Object::new)
            .map(|errors| errors.entries())
            .unwrap_or_default();
        let code = body.as_ref().and_then(|body| body.str("code")).filter(|code| !code.is_empty());
        let data = body.as_ref().and_then(|body| body.raw("data")).map(ToString::to_string);
        return Err(ActionError { message, errors, status, code, data });
    }
    let data = body.and_then(|body| body.raw("data")).unwrap_or("null").to_string();
    Ok(ActionResult { data, redirected: false })
}

// ---- requests ----------------------------------------------------------------

/// What travels as a request body.
pub enum Payload {
    /// A body the caller encoded: `typeof body === "string"`.
    Text(String),
    /// A value stringified, which also sets `content-type: application/json`.
    Json(String),
    /// A `FormData`, `URLSearchParams`, `Blob` or `ArrayBuffer` the shim holds.
    Handle(dom::Node),
}

/// `as: "blob" | "text" | "response"`, and what a response's type decides.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Want {
    /// JSON when the response says so, text otherwise.
    #[default]
    Auto,
    Text,
    Blob,
    Response,
}

/// The second argument of `nextRust.request` and `nextRust.stream`.
#[derive(Default)]
pub struct Options {
    /// `""` is `"GET"`, as the source's default parameter is.
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Payload>,
    pub signal: Option<dom::Node>,
    /// In milliseconds, and only when the caller passed no signal of its own.
    pub timeout: Option<f64>,
    pub want: Want,
}

/// What a request answered with.
pub enum Answer {
    /// `204`, which is `null` to the caller.
    Empty,
    Text(String),
    /// The raw JSON text of the body, `"null"` when it did not parse
    /// (`await res.json().catch(() => null)`).
    Json(String),
    Blob(dom::Node),
    Response(dom::Node),
}

impl Answer {
    /// `data && typeof data === "object" && (data.error || data.message)`.
    fn message(&self) -> Option<String> {
        let Self::Json(text) = self else { return None };
        let body = json::Object::new(text)?;
        body.str("error").or_else(|| body.str("message")).filter(|message| !message.is_empty())
    }
}

/// A request that failed, or a response that was not 2xx: `new Error(message)`
/// with `.status` and `.data`.
pub struct RequestError {
    pub message: String,
    pub status: u16,
    pub data: Answer,
}

impl RequestError {
    fn new(message: impl Into<String>, status: u16) -> Self {
        Self { message: message.into(), status, data: Answer::Empty }
    }
}

/// Same-origin only: the page's cookies and CSRF token travel with every
/// request, so they must never reach another site.
fn same_origin(url: &str) -> Result<String, RequestError> {
    let Some(parsed) = dom::Url::here(url) else {
        return Err(RequestError::new("[next-rust] not a URL", 0));
    };
    if parsed.origin() != dom::origin() {
        let mut message = String::from("[next-rust] same-origin URLs only: ");
        message.push_str(&parsed.origin());
        return Err(RequestError::new(message, 0));
    }
    Ok(parsed.href())
}

/// `requestInit`: the method, the headers an action would send minus the one
/// that says it *is* an action, and the body with its content type.
fn request_init(options: &Options) -> (String, Vec<(String, String)>) {
    // ASCII: an HTTP method is a token, and Unicode uppercasing is ~1.6 KB of wasm.
    let method = if options.method.is_empty() { String::from("GET") } else { options.method.to_ascii_uppercase() };
    let extra: Vec<(&str, &str)> = options.headers.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut headers = nav::action_headers(&extra);
    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("x-nr-action"));
    if matches!(options.body, Some(Payload::Json(_))) {
        set_header(&mut headers, "content-type", "application/json");
    }
    (method, headers)
}

/// `timeout` is a signal of the runtime's own, and only for a caller that
/// brought none. The timer owns the controller, so its handle is still good
/// when it fires; dropping the timer is `clearTimeout`.
fn timeout_signal(options: &Options) -> (Option<dom::Node>, Option<task::Timer>) {
    let (Some(ms), None) = (options.timeout, options.signal.as_ref()) else { return (None, None) };
    let Some(controller) = dom::Node::new(host::call(op::ABORT_NEW, 0, 0, 0, 0, 0)) else { return (None, None) };
    let signal = dom::Node::new(host::call(op::ABORT_SIGNAL, controller.raw(), 0, 0, 0, 0));
    let timer = task::Timer::after(ms, move || {
        host::call(op::ABORT, controller.raw(), 0, 0, 0, 0);
    });
    (signal, Some(timer))
}

/// A request to this site: JSON in, JSON (or text) out, an error with
/// `status` and `data` when the response is not 2xx.
pub async fn request(url: &str, options: Options) -> Result<Answer, RequestError> {
    let href = same_origin(url)?;
    let (method, headers) = request_init(&options);
    // The timer is dropped on the way out of this function, whichever way it
    // leaves, which is the source's `finally { clearTimeout(timer) }`.
    let (own, _timer) = timeout_signal(&options);
    let signal = options.signal.as_ref().or(own.as_ref());
    let body = options.body.as_ref().map(Payload::sent);
    let res = fetch(&href, &method, &headers, body, signal).await;
    let Some(res) = res else { return Err(RequestError::new("Request failed", 0)) };
    // What was sent may change what pages show.
    if method != "GET" && method != "HEAD" {
        nav::clear_pages();
    }
    let ok = host::flag(op::RES_OK, res.raw(), 0, 0, 0, 0);
    let kind = if ok { options.want } else { Want::Auto };
    if kind == Want::Response {
        return Ok(Answer::Response(res));
    }
    let content_type = header(&res, "content-type").unwrap_or_default();
    let data = if status(&res) == 204 {
        Answer::Empty
    } else if kind == Want::Blob {
        match response_blob(&res).await {
            Some(blob) => Answer::Blob(blob),
            None => return Err(RequestError::new("Request failed", status(&res))),
        }
    } else {
        // The two body reads the source writes are not alike: `await
        // res.json().catch(() => null)` swallows a body that never arrived,
        // and `await res.text()` does not — it rejects, and so does the
        // request. Reading the text and parsing it here covers both, so the
        // failure has to be told apart explicitly, or an aborted request
        // resolves with `""` where the source rejects with the abort.
        let json_body = kind != Want::Text && content_type.contains("json");
        match response_text(&res).await {
            Some(text) if json_body => Answer::Json(if json::parses(&text) { text } else { String::from("null") }),
            Some(text) => Answer::Text(text),
            None if json_body => Answer::Json(String::from("null")),
            None => return Err(RequestError::new("Request failed", status(&res))),
        }
    };
    if !ok {
        let status_text = status_text(&res);
        let message = data
            .message()
            .or_else(|| (!status_text.is_empty()).then_some(status_text))
            .unwrap_or_else(|| String::from("Request failed"));
        return Err(RequestError { message, status: status(&res), data });
    }
    Ok(data)
}

/// One server-sent event: `{ event, data, id }`, with `data` the raw text.
pub struct SseEvent {
    pub event: String,
    pub data: String,
    pub id: Option<String>,
}

/// Server-sent events from this site (`Response::sse`). Each event is handed
/// to `sink` as it arrives, which is what the source's `yield` does.
pub async fn stream(url: &str, options: Options, mut sink: impl FnMut(SseEvent)) -> Result<(), RequestError> {
    let href = same_origin(url)?;
    let (method, mut headers) = request_init(&options);
    set_header(&mut headers, "accept", "text/event-stream");
    let body = options.body.as_ref().map(Payload::sent);
    let res = fetch(&href, &method, &headers, body, options.signal.as_ref()).await;
    let Some(res) = res else { return Err(RequestError::new("Stream failed", 0)) };
    let ok = host::flag(op::RES_OK, res.raw(), 0, 0, 0, 0);
    // `!res.ok || !res.body`: the body says what went wrong, as text. The
    // reader is taken only once the response is good, because `getReader()`
    // locks the body — taking it first left the failing branch reading a
    // locked body, and every stream that failed carried an empty `data`.
    let reader = if ok { dom::Node::new(host::call(op::RES_READER, res.raw(), 0, 0, 0, 0)) } else { None };
    let Some(reader) = reader else {
        let status_text = status_text(&res);
        let message = if status_text.is_empty() { String::from("Stream failed") } else { status_text };
        let data = Answer::Text(response_text(&res).await.unwrap_or_default());
        return Err(RequestError { message, status: status(&res), data });
    };
    let out = pump(&reader, &mut sink).await;
    // `finally { reader.cancel().catch(() => {}) }`
    host::call(op::READER_CANCEL, reader.raw(), 0, 0, 0, 0);
    out
}

/// Read the body to its end, framing it into events as it goes.
async fn pump(reader: &dom::Node, sink: &mut impl FnMut(SseEvent)) -> Result<(), RequestError> {
    let mut buffer = String::new();
    loop {
        let slot = task::pending();
        host::call(op::READER_READ, reader.raw(), slot.token(), 0, 0, 0);
        let settled = slot.await;
        if settled.failed() {
            return Err(RequestError::new("Stream failed", 0));
        }
        let done = settled.flag & task::DONE != 0;
        // The end closes whatever frame was still open.
        let chunk = settled.text.unwrap_or_default();
        buffer.push_str(if done { "\n\n" } else { &chunk });
        while let Some((at, next)) = frame(&buffer) {
            let event = parse_sse_block(buffer.get(..at).unwrap_or(""));
            // `buffer.slice(at).replace(/^\r?\n\r?\n/, "")`, in place: a long
            // stream must not copy what is left of it once per event. The
            // frame ends on a newline, so this is always a character boundary.
            buffer.drain(..next);
            if let Some(event) = event {
                sink(event);
            }
        }
        if done {
            return Ok(());
        }
    }
}

/// Where the first `\r?\n\r?\n` starts and ends: the source's
/// `buffer.search(...)` and the `replace(/^\r?\n\r?\n/, "")` that follows it.
fn frame(buffer: &str) -> Option<(usize, usize)> {
    let b = buffer.as_bytes();
    for start in 0..b.len() {
        let mut i = start;
        if b.get(i) == Some(&b'\r') {
            i += 1;
        }
        if b.get(i) != Some(&b'\n') {
            continue;
        }
        i += 1;
        if b.get(i) == Some(&b'\r') {
            i += 1;
        }
        if b.get(i) != Some(&b'\n') {
            continue;
        }
        return Some((start, i + 1));
    }
    None
}

/// One block between two blank lines as an event, or nothing when it carried
/// no `data` at all (`if (ev.data.length)`).
pub fn parse_sse_block(block: &str) -> Option<SseEvent> {
    let mut event = String::from("message");
    let mut data: Vec<&str> = Vec::new();
    let mut id = None;
    for line in block.split('\n') {
        // `split(/\r?\n/)`: the carriage return belongs to the break.
        let line = line.strip_suffix('\r').unwrap_or(line);
        // A comment (`: keep-alive`) and a blank line say nothing.
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let (field, value) = match line.find(':') {
            // One space after the colon is the separator, not the value.
            Some(at) => (&line[..at], line[at + 1..].strip_prefix(' ').unwrap_or(&line[at + 1..])),
            None => (line, ""),
        };
        match field {
            "event" => event = value.to_string(),
            "data" => data.push(value),
            "id" => id = Some(value.to_string()),
            _ => {}
        }
    }
    if data.is_empty() {
        return None;
    }
    let mut joined = String::new();
    for (i, line) in data.iter().enumerate() {
        if i > 0 {
            joined.push('\n');
        }
        joined.push_str(line);
    }
    Some(SseEvent { event, data: joined, id })
}

// ---- forms --------------------------------------------------------------------

/// What sending a form means, or nothing when the browser should do it.
enum Request {
    /// A server action: its fields, posted as `URLSearchParams`.
    Action { action: String, body: dom::Node },
    /// A POST to this site.
    Post { url: dom::Url, body: Body },
    /// A GET form: a navigation to the URL its fields build.
    Get { url: dom::Url },
}

impl Request {
    fn body(&self) -> Option<&dom::Node> {
        match self {
            Self::Action { body, .. } => Some(body),
            Self::Post { body, .. } => Some(body.node()),
            Self::Get { .. } => None,
        }
    }

    /// `req.body && !(req.body instanceof FormData) && req.body.get("_redirect")`:
    /// where a browser without JavaScript would have been sent.
    fn redirect_field(&self) -> Option<String> {
        let params = match self {
            Self::Action { body, .. } => body,
            Self::Post { body: Body::Params(body), .. } => body,
            _ => return None,
        };
        let (p, l) = host::s("_redirect");
        host::string(op::USP_GET, params.raw(), p, l, 0, 0).filter(|to| !to.is_empty())
    }
}

/// An attribute of the form that its submit button may override
/// (`formaction`, `formmethod`, `formenctype`, `formtarget`).
fn form_attr(form: &dom::Node, submitter: Option<&dom::Node>, name: &str) -> String {
    if let Some(submitter) = submitter {
        let mut attribute = String::from("form");
        attribute.push_str(name);
        if let Some(value) = dom::attr(submitter, &attribute).filter(|value| !value.is_empty()) {
            return value;
        }
    }
    dom::attr_or_empty(form, name)
}

/// What sending `form` means, or `None` when the browser should do it.
fn form_request(form: &dom::Node, submitter: Option<&dom::Node>) -> Option<Request> {
    for el in [Some(form), submitter].into_iter().flatten() {
        if ["data-nr-reload", "data-nr-native", "download"].iter().any(|name| dom::has_attr(el, name)) {
            return None;
        }
    }
    let action = dom::dataset(form, "data-nr-action").filter(|action| !action.is_empty());
    let target = form_attr(form, submitter, "target");
    // ASCII, deliberately: `method`, `enctype` and `content-disposition` are
    // HTML and HTTP tokens, and Unicode lowercasing costs about two kilobytes
    // of wasm for characters none of them can contain.
    let method = form_attr(form, submitter, "method").to_ascii_lowercase();
    let enctype = form_attr(form, submitter, "enctype").to_ascii_lowercase();
    let mut url = None;
    if action.is_some() {
        // Server actions take no files.
        if dom::prop_str(form, "enctype").as_deref() == Some("multipart/form-data") {
            return None;
        }
    } else {
        let elsewhere = !target.is_empty() && target != "_self";
        if !ENHANCE.with(Cell::get) || elsewhere || method == "dialog" || enctype == "text/plain" {
            return None;
        }
        let action_url = form_attr(form, submitter, "action");
        let here = dom::href();
        // `new URL(... || location.href, location.href)`, and `catch` is `None`.
        let parsed = dom::Url::here(if action_url.is_empty() { &here } else { &action_url })?;
        if parsed.origin() != dom::origin() || !matches!(parsed.protocol().as_str(), "http:" | "https:") {
            return None;
        }
        url = Some(parsed);
    }
    // Only now: building the data tells the page (`formdata`), once per
    // submission. The shim falls back to `new FormData(form)` where the
    // two-argument constructor throws.
    let data = dom::Node::new(host::call(op::FORM_DATA_NEW, form.raw(), submitter.map_or(0, dom::Node::raw), 0, 0, 0))?;
    let fields = dom::Node::new(host::call(op::USP_NEW, 0, 0, 0, 0, 0))?;
    for i in 0..host::call(op::FD_LEN, data.raw(), 0, 0, 0, 0) {
        let key = host::text(op::FD_KEY, data.raw(), i, 0, 0, 0);
        // A file in a form that cannot carry files goes by its name, as
        // browsers do: `typeof v === "string" ? v : v.name`.
        let value = host::text(op::FD_VALUE, data.raw(), i, 0, 0, 0);
        let (p, l) = host::s(&key);
        let (q, m) = host::s(&value);
        host::call(op::USP_APPEND, fields.raw(), p, l, q, m);
    }
    if let Some(action) = action {
        return Some(Request::Action { action, body: fields });
    }
    let url = url?;
    if method == "post" {
        let body = if enctype == "multipart/form-data" { Body::FormData(data) } else { Body::Params(fields) };
        return Some(Request::Post { url, body });
    }
    // `url.search = fields`
    url.set_search(&host::text(op::USP_STR, fields.raw(), 0, 0, 0, 0));
    Some(Request::Get { url })
}

/// The form sent, and the answer is a value rather than a page.
async fn form_succeeded(form: &dom::Node, data: &str, next: Option<&str>) -> Result<(), Failed> {
    dom::set_attr(form, "data-nr-state", "success");
    dispatch(form, CUSTOM, "nr:success", data, BUBBLES);
    // `data-nr-stay`: show the result in place; `_redirect` is then only
    // where browsers without JavaScript go.
    if dom::has_attr(form, "data-nr-stay") {
        let object = json::Object::new(data);
        for el in dom::query_all(form, "[data-nr-result]") {
            let key = dom::dataset(&el, "data-nr-result").unwrap_or_default();
            // `key && data && typeof data === "object" ? data[key] : data`
            let value = if key.is_empty() || !json::is_object(data) {
                data
            } else {
                object.as_ref().and_then(|object| object.raw(&key)).unwrap_or("null")
            };
            dom::set_text_content(&el, &json::text(value));
        }
        if dom::has_attr(form, "data-nr-reset") {
            host::call(op::FORM_RESET, form.raw(), 0, 0, 0, 0);
        }
        return Ok(());
    }
    let options = match next {
        Some(_) => NavOptions::default(),
        // `NR.refresh()`: this page again, in place.
        None => NavOptions { replace: true, scroll: false, fresh: true, ..NavOptions::default() },
    };
    let here = dom::href();
    let to = next.unwrap_or(&here);
    nav::navigate(to, options).await.map(|_| ()).map_err(|_| Failed)
}

/// The form was refused: the messages go beside the fields they belong to.
fn form_failed(form: &dom::Node, body: &str) {
    dom::set_attr(form, "data-nr-state", "error");
    let object = json::Object::new(body);
    // `body.errors || { _form: body.error }`
    let errors = match object.as_ref().and_then(|object| object.raw("errors")).and_then(json::Object::new) {
        Some(errors) => errors.entries(),
        None => {
            let message = object.as_ref().and_then(|object| object.str("error")).unwrap_or_default();
            alloc::vec![(String::from("_form"), message)]
        }
    };
    for (field, message) in errors {
        let mut selector = String::from("[data-nr-error=\"");
        selector.push_str(&dom::css_escape(&field));
        selector.push_str("\"]");
        if let Some(el) = dom::query(form, &selector) {
            dom::set_text_content(&el, &message);
        }
    }
    dispatch(form, CUSTOM, "nr:error", body, BUBBLES);
}

/// The answer to a posted form that is not a page: a file is saved, anything
/// else reaches the form the way a server action's result does.
async fn form_answer(form: &dom::Node, res: &dom::Node, next: Option<&str>) -> Result<(), Failed> {
    let disposition = header(res, "content-disposition").unwrap_or_default();
    if disposition.to_ascii_lowercase().contains("attachment") {
        let blob = response_blob(res).await.ok_or(Failed)?;
        let a = dom::create_element("a");
        let url = host::text(op::OBJECT_URL, blob.raw(), 0, 0, 0, 0);
        dom::set_prop_str(&a, "href", &url);
        dom::set_prop_str(&a, "download", &filename(&disposition));
        // There is no `click` op: a click dispatched on an anchor runs its
        // activation behaviour, which is all `a.click()` is.
        dispatch(&a, EVENT, "click", "", BUBBLES | CANCELABLE);
        // Long enough for the browser to have taken the file.
        later(60_000.0, move || {
            let (p, l) = host::s(&url);
            host::call(op::REVOKE_URL, p, l, 0, 0, 0);
        });
        return Ok(());
    }
    let text = response_text(res).await.ok_or(Failed)?;
    let json_body = json::parses(&text);
    if host::flag(op::RES_OK, res.raw(), 0, 0, 0, 0) {
        // `text ? data : null`, with a body that is not JSON kept as a string.
        let data = if text.is_empty() {
            String::from("null")
        } else if json_body {
            text
        } else {
            json::quote(&text)
        };
        return form_succeeded(form, &data, next).await;
    }
    // `data && typeof data === "object" ? data : { error: text }`
    let object = json_body && json::is_object(&text);
    let body =
        with_error(if object { &text } else { "{}" }, if object { "" } else { &text }, &status_text(res), status(res));
    form_failed(form, &body);
    Ok(())
}

/// `body.error = body.error || body.message || res.statusText || "Request
/// failed"; body.status = res.status` — as a body of its own, since nothing
/// here can reach into the object the server sent.
fn with_error(body: &str, fallback: &str, status_text: &str, status: u16) -> String {
    let object = json::Object::new(body);
    let members = object.as_ref().map(json::Object::members).unwrap_or_default();
    let of = |name: &str| {
        members.iter().find(|(key, _)| key == name).map(|(_, value)| json::text(value)).filter(|text| !text.is_empty())
    };
    let message = of("error")
        .or_else(|| of("message"))
        .or_else(|| (!fallback.is_empty()).then(|| fallback.to_string()))
        .or_else(|| (!status_text.is_empty()).then(|| status_text.to_string()))
        .unwrap_or_else(|| String::from("Request failed"));
    // The source assigns into the object the server sent, so a member it
    // overwrites keeps the place it already had; only a member that was not
    // there is appended. This is the detail of a `nr:error` event, and a page
    // that walks it with `Object.entries` sees that order.
    let status_number = {
        let mut number = String::new();
        push_number(&mut number, status);
        number
    };
    let quoted = json::quote(&message);
    let mut out = String::from("{");
    let mut first = true;
    let mut push = |out: &mut String, key: &str, value: &str| {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&json::quote(key));
        out.push(':');
        out.push_str(value);
    };
    for (key, value) in &members {
        match key.as_str() {
            "error" => push(&mut out, key, &quoted),
            "status" => push(&mut out, key, &status_number),
            _ => push(&mut out, key, value),
        }
    }
    if !members.iter().any(|(key, _)| key == "error") {
        push(&mut out, "error", &quoted);
    }
    if !members.iter().any(|(key, _)| key == "status") {
        push(&mut out, "status", &status_number);
    }
    out.push('}');
    out
}

/// The send went wrong, which is the source's `catch`: the browser sends the
/// form itself rather than the visitor losing what they typed.
struct Failed;

/// Send `form`. `quiet` is a live search: it changes the page in place, takes
/// no history entry, and gives way to the next keystroke.
async fn send_form(form: dom::Node, submitter: Option<dom::Node>, req: Request, quiet: bool) {
    // Sent twice (a double click, Enter held down): once is enough. Only a
    // live search gives way, to the newer text.
    let before = lookup(&form);
    if let Some(before) = before {
        let found = SENDING.with(|s| s.borrow().get(&before).map(|entry| (entry.controller.raw(), entry.live)));
        match found {
            Some((_, false)) => return,
            Some((controller, true)) => {
                host::call(op::ABORT, controller, 0, 0, 0, 0);
            }
            None => {}
        }
    }
    let Some(controller) = dom::Node::new(host::call(op::ABORT_NEW, 0, 0, 0, 0, 0)) else { return };
    let Some(signal) = dom::Node::new(host::call(op::ABORT_SIGNAL, controller.raw(), 0, 0, 0, 0)) else { return };
    let ticket = TICKET.with(|ticket| {
        ticket.set(ticket.get().wrapping_add(1));
        ticket.get()
    });
    // `sending.set(form, controller)`: the entry a live search is giving up
    // is replaced, never doubled, so this form has one entry either way.
    let key = before.unwrap_or_else(|| form.raw());
    SENDING.with(|sending| {
        let mut sending = sending.borrow_mut();
        // A form that left the document will never be sent again.
        sending.retain(|_, entry| dom::is_connected(&entry.form));
        sending.insert(key, Sending { form: form.share(), controller, ticket, live: quiet });
    });
    // `clearTimeout(liveTimers.get(form))`
    if let Some(waiting) = live_timer(&form) {
        LIVE_TIMERS.with(|timers| timers.borrow_mut().remove(&waiting));
    }
    for el in dom::query_all(&form, "[data-nr-error],[data-nr-result]") {
        dom::set_text_content(&el, "");
    }
    dom::remove_attr(&form, "data-nr-state");
    dom::set_attr(&form, "aria-busy", "true");
    let active = dom::active_element();
    let enabled = submitter.as_ref().filter(|submitter| !dom::prop_bool(submitter, "disabled"));
    // `submitter === document.activeElement`, asked before the button is
    // disabled, because disabling it is what takes the focus away.
    let held = enabled.is_some_and(|submitter| active.as_ref().is_some_and(|active| same(active, submitter)));
    let disabled = enabled.is_some();
    if let Some(submitter) = enabled {
        dom::set_prop_bool(submitter, "disabled", true);
    }
    let next = req.redirect_field();
    // The bar is for actions alone: a form that navigates is already covered
    // by the bar the navigation itself starts.
    let loaded = matches!(req, Request::Action { .. }).then(|| loader::start(None, None));
    let outcome = send(&form, submitter.as_ref(), &req, quiet, &signal, next.as_deref()).await;
    if outcome.is_err() && !host::flag(op::SIGNAL_ABORTED, signal.raw(), 0, 0, 0, 0) {
        // A form that never reached the server is the browser's to send.
        host::call(op::NATIVE_SUBMIT, form.raw(), 0, 0, 0, 0);
    }
    // `finally`: whatever happened, the form is usable again.
    let mine = lookup(&form).filter(|key| SENDING.with(|s| s.borrow().get(key).is_some_and(|e| e.ticket == ticket)));
    if let Some(key) = mine {
        SENDING.with(|sending| sending.borrow_mut().remove(&key));
        dom::remove_attr(&form, "aria-busy");
    }
    if let Some(submitter) = submitter.as_ref().filter(|_| disabled) {
        dom::set_prop_bool(submitter, "disabled", false);
        // Focus went to the body when the button was disabled; it goes back.
        let loose = dom::active_element().is_some_and(|active| same(&active, &dom::body()));
        if held && dom::is_connected(submitter) && loose {
            dom::focus(submitter);
        }
    }
    if let Some(loaded) = loaded {
        loaded.end();
    }
}

/// The `try` of [`send_form`]: everything that can go wrong on the way to the
/// server, so that one `catch` covers all of it.
async fn send(
    form: &dom::Node,
    submitter: Option<&dom::Node>,
    req: &Request,
    quiet: bool,
    signal: &dom::Node,
    next: Option<&str>,
) -> Result<(), Failed> {
    if let Request::Action { action, body } = req {
        let headers = nav::action_headers(&[]);
        let res = fetch(action, "POST", &headers, Some(Sent::Node(body)), None).await.ok_or(Failed)?;
        // `await res.json()`, which rejects when the answer is not JSON.
        let text = response_text(&res).await.ok_or(Failed)?;
        if !json::parses(&text) {
            return Err(Failed);
        }
        let body = json::Object::new(&text);
        let ok = body.as_ref().is_some_and(|body| body.bool("ok"));
        let redirect = body.as_ref().and_then(|body| body.str("redirect")).filter(|to| !to.is_empty());
        if ok || redirect.is_some() {
            nav::clear_pages();
        }
        if let Some(to) = redirect {
            return nav::navigate(&to, NavOptions::default()).await.map(|_| ()).map_err(|_| Failed);
        }
        if ok {
            let data = body.as_ref().and_then(|body| body.raw("data")).unwrap_or("null");
            return form_succeeded(form, data, next).await;
        }
        form_failed(form, &text);
        return Ok(());
    }
    let (url, body) = match req {
        Request::Post { url, body } => (url, Some(body)),
        Request::Get { url } => (url, None),
        Request::Action { .. } => return Err(Failed),
    };
    let scroll = [Some(form), submitter]
        .into_iter()
        .flatten()
        .all(|el| dom::dataset(el, "data-nr-scroll").as_deref() != Some("false"));
    let options = NavOptions {
        post: body.map(|body| PostBody { body: body.share(), signal: Some(signal.share()) }),
        signal: Some(signal.share()),
        // The page this form leads to is never the copy kept in memory.
        fresh: true,
        // A GET form is a search: what the visitor was typing into stays.
        focus: body.is_none(),
        replace: quiet || dom::has_attr(form, "data-nr-replace"),
        scroll,
        quiet,
        target: dom::dataset(form, "data-nr-target"),
        ..NavOptions::default()
    };
    let res = nav::navigate(&url.href(), options).await.map_err(|_| Failed)?;
    // Not a page (JSON, a file, nothing): the form shows it.
    match res {
        Some(res) => form_answer(form, &res, next).await,
        None => Ok(()),
    }
}

// ---- the listeners -------------------------------------------------------------

/// The submit listener, the `form.submit()` hook and the live-search listener.
pub fn install() {
    let document = dom::document();
    ENHANCE.with(|enhance| enhance.set(dom::by_id(&document, "__next_rust_forms").is_none()));
    // On `window`, so every listener on the form or the document has had its
    // say by now: a submit the page cancels (to handle it itself) is left
    // alone. The ABI has no window of its own; this is it.
    let window = dom::prop_node(&document, "defaultView").unwrap_or_else(dom::document);
    listen(&window, "submit", on_submit);
    // `form.submit()` fires no submit event; it still must not reload the page.
    let patched = task::on(|form| {
        let Some(form) = form else { return };
        match form_request(&form, None) {
            Some(req) => task::spawn(send_form(form, None, req, false)),
            None => {
                host::call(op::NATIVE_SUBMIT, form.raw(), 0, 0, 0, 0);
            }
        }
    });
    host::call(op::PATCH_FORM_SUBMIT, patched, 0, 0, 0, 0);
    listen(&document, "input", on_input);
}

fn listen(target: &dom::Node, kind: &str, handler: impl Fn(Option<dom::Node>) + 'static) {
    let cb = task::on(handler);
    let (p, l) = host::s(kind);
    // The registration handle is never given back: these listeners are for
    // the life of the document. (`dom::listen` should absorb this.)
    host::call(op::ADD_LISTENER, target.raw(), p, l, cb, 0);
}

fn on_submit(event: Option<dom::Node>) {
    let Some(event) = event else { return };
    let Some(form) = dom::prop_node(&event, "target") else { return };
    if dom::node_name(&form) != "FORM" || dom::prop_bool(&event, "defaultPrevented") {
        return;
    }
    let submitter = dom::prop_node(&event, "submitter");
    let live = LIVE.with(Cell::get);
    match form_request(&form, submitter.as_ref()) {
        Some(req) => {
            host::call(op::PREVENT_DEFAULT, event.raw(), 0, 0, 0, 0);
            // A live search sends a GET form only: what it typed is the URL.
            if !live || req.body().is_none() {
                task::spawn(send_form(form, submitter, req, live));
            }
        }
        // The browser submits this one itself: a full page load, with the bar.
        None if !live => {
            let target = form_attr(&form, submitter.as_ref(), "target");
            let method = form_attr(&form, submitter.as_ref(), "method").to_ascii_lowercase();
            if (target.is_empty() || target == "_self") && method != "dialog" {
                // Once every listener has had its say: a submit someone
                // cancels loads nothing.
                later(0.0, move || {
                    if !dom::prop_bool(&event, "defaultPrevented") {
                        drop(loader::start(Some(30_000.0), None));
                    }
                });
            }
        }
        None => {}
    }
}

/// Live search: a GET form marked `data-nr-live` (or the field itself) is
/// sent as the visitor types, `data-nr-debounce` (or `data-nr-live="ms"`,
/// else 250) ms after the last change. The previous request is dropped,
/// history is replaced, focus stays.
fn on_input(event: Option<dom::Node>) {
    let Some(event) = event else { return };
    let Some(el) = dom::prop_node(&event, "target") else { return };
    let Some(form) = dom::prop_node(&el, "form") else { return };
    if dom::prop_bool(&event, "isComposing") {
        return;
    }
    if !dom::has_attr(&form, "data-nr-live") && !dom::has_attr(&el, "data-nr-live") {
        return;
    }
    let ms = [
        dom::dataset(&el, "data-nr-debounce"),
        dom::dataset(&form, "data-nr-debounce"),
        dom::dataset(&el, "data-nr-live"),
        dom::dataset(&form, "data-nr-live"),
    ]
    .into_iter()
    .flatten()
    .find(|value| !value.is_empty())
    // `+value`: text that is not a number is `NaN`, which `setTimeout` takes
    // as no wait at all.
    .map_or(250.0, |value| crate::num::number(&value).unwrap_or(0.0));
    let waiting = live_timer(&form).unwrap_or_else(|| form.raw());
    let live = form.share();
    let timer = task::Timer::after(ms, move || {
        // A form the visitor has left, or one that will not validate, sends
        // nothing.
        let valid = dom::prop_bool(&live, "noValidate") || host::flag(op::CHECK_VALIDITY, live.raw(), 0, 0, 0, 0);
        if !dom::is_connected(&live) || !valid {
            return;
        }
        LIVE.with(|flag| flag.set(true));
        // A submit event, so listeners that validate or cancel still get
        // their say.
        dispatch(&live, SUBMIT, "submit", "", BUBBLES | CANCELABLE);
        LIVE.with(|flag| flag.set(false));
    });
    LIVE_TIMERS.with(|timers| {
        let mut timers = timers.borrow_mut();
        timers.retain(|_, entry| dom::is_connected(&entry.form));
        // Replacing the entry drops the timer that was in it: `clearTimeout`.
        timers.insert(waiting, Live { form, _timer: timer });
    });
}

// ---- the shim ---------------------------------------------------------------------

/// A body on its way to the shim, borrowed from whoever owns it.
enum Sent<'a> {
    /// Text, which travels inside the request block.
    Text(&'a str),
    /// A `FormData`, `URLSearchParams` or `Blob` the shim already holds.
    Node(&'a dom::Node),
}

impl Payload {
    fn sent(&self) -> Sent<'_> {
        match self {
            Self::Text(text) | Self::Json(text) => Sent::Text(text),
            Self::Handle(node) => Sent::Node(node),
        }
    }
}

/// `fetch(url, init)`, always `credentials: "same-origin"`.
///
/// The ABI carries five words, so everything but the body's handle travels in
/// one block: the signal and the body's length as two `u32`s, then the URL,
/// the method and the header names and values NUL-terminated, then the body's
/// bytes. (`dom::fetch` should absorb this.)
async fn fetch(
    url: &str,
    method: &str,
    headers: &[(String, String)],
    body: Option<Sent<'_>>,
    signal: Option<&dom::Node>,
) -> Option<dom::Node> {
    let text: &[u8] = match &body {
        Some(Sent::Text(text)) => text.as_bytes(),
        _ => &[],
    };
    let mut block = Vec::with_capacity(64 + url.len() + text.len());
    block.extend_from_slice(&signal.map_or(0, dom::Node::raw).to_le_bytes());
    block.extend_from_slice(&(text.len() as u32).to_le_bytes());
    for field in [url, method] {
        block.extend_from_slice(field.as_bytes());
        block.push(0);
    }
    for (name, value) in headers {
        for field in [name, value] {
            block.extend_from_slice(field.as_bytes());
            block.push(0);
        }
    }
    block.extend_from_slice(text);
    let (kind, handle) = match &body {
        Some(Sent::Node(node)) => (2, node.raw()),
        Some(_) => (1, 0),
        None => (0, 0),
    };
    let slot = task::pending();
    host::call(op::FETCH, block.as_ptr() as u32, block.len() as u32, kind, handle, slot.token());
    let settled = slot.await;
    if settled.failed() { None } else { settled.node() }
}

/// `await res.text()`, `None` when it rejects.
async fn response_text(res: &dom::Node) -> Option<String> {
    let slot = task::pending();
    host::call(op::RES_TEXT, res.raw(), slot.token(), 0, 0, 0);
    let settled = slot.await;
    (!settled.failed()).then(|| settled.text.unwrap_or_default())
}

/// `await res.blob()`.
async fn response_blob(res: &dom::Node) -> Option<dom::Node> {
    let slot = task::pending();
    host::call(op::RES_BLOB, res.raw(), slot.token(), 0, 0, 0);
    let settled = slot.await;
    if settled.failed() { None } else { settled.node() }
}

fn header(res: &dom::Node, name: &str) -> Option<String> {
    let (p, l) = host::s(name);
    host::string(op::RES_HEADER, res.raw(), p, l, 0, 0)
}

fn status(res: &dom::Node) -> u16 {
    host::call(op::RES_STATUS, res.raw(), 0, 0, 0, 0) as u16
}

fn status_text(res: &dom::Node) -> String {
    host::text(op::RES_STATUS_TEXT, res.raw(), 0, 0, 0, 0)
}

/// `target.dispatchEvent(new <kind>(name, { detail, bubbles, cancelable }))`.
/// The name and the detail's JSON travel as one string with a NUL between
/// them, which is what the shim splits on. (`dom::dispatch` should absorb this.)
fn dispatch(target: &dom::Node, kind: u32, name: &str, detail: &str, flags: u32) {
    let mut packed = String::with_capacity(name.len() + detail.len() + 1);
    packed.push_str(name);
    packed.push('\0');
    packed.push_str(detail);
    let (p, l) = host::s(&packed);
    host::call(op::DISPATCH, target.raw(), kind, flags, p, l);
}

/// `setTimeout(f, ms)` with nothing to cancel: [`task::Timer`] stops itself
/// when it is dropped, and these must outlive the call that started them.
/// (`task::defer` should absorb this.)
fn later(ms: f64, handler: impl Fn() + 'static) {
    let cb = task::on(move |_| handler());
    host::call(op::SET_TIMEOUT, ms.max(0.0) as u32, cb, 0, 0, 0);
}

/// Whether two handles are the same node, which is what `===` asks of two DOM
/// objects. The shim hands out a fresh handle every crossing, so they cannot
/// simply be compared; a node contains itself and nothing that contains it,
/// so containment both ways is identity. (`dom::same` should absorb this.)
fn same(a: &dom::Node, b: &dom::Node) -> bool {
    dom::contains(a, Some(b)) && dom::contains(b, Some(a))
}

/// The key `form` is filed under in `SENDING`, if it is sending.
fn lookup(form: &dom::Node) -> Option<u32> {
    SENDING.with(|sending| sending.borrow().iter().find_map(|(key, entry)| same(&entry.form, form).then_some(*key)))
}

/// The key `form` is filed under in `LIVE_TIMERS`, if it is waiting.
fn live_timer(form: &dom::Node) -> Option<u32> {
    LIVE_TIMERS.with(|timers| timers.borrow().iter().find_map(|(key, entry)| same(&entry.form, form).then_some(*key)))
}

fn set_header(headers: &mut Vec<(String, String)>, name: &str, value: &str) {
    if let Some(header) = headers.iter_mut().find(|(key, _)| key.eq_ignore_ascii_case(name)) {
        header.1 = value.to_string();
        return;
    }
    headers.push((name.to_string(), value.to_string()));
}

/// `(disposition.match(/filename="?([^";]+)/i) || [])[1] || ""`.
fn filename(disposition: &str) -> String {
    let lower = disposition.to_ascii_lowercase();
    let Some(at) = lower.find("filename=") else { return String::new() };
    let rest = disposition.get(at + "filename=".len()..).unwrap_or("");
    let rest = rest.strip_prefix('"').unwrap_or(rest);
    rest.split(['"', ';']).next().unwrap_or("").to_string()
}

/// A status code as its digits, without reaching for `core::fmt`.
fn push_number(out: &mut String, value: u16) {
    if value >= 10 {
        push_number(out, value / 10);
    }
    out.push(char::from_digit(u32::from(value % 10), 10).unwrap_or('0'));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `body.error = body.error || body.message || res.statusText || "Request
    /// failed"; body.status = res.status`, assigned **into** the object the
    /// server sent: a member that was already there keeps its place, and only
    /// a member that was missing is appended. The result is the detail of a
    /// `nr:error` event, which pages walk with `Object.entries`.
    #[test]
    fn an_error_body_keeps_the_order_the_server_sent() {
        let body = r#"{"error":"validation failed","fields":{"a":"b"}}"#;
        assert_eq!(
            with_error(body, "", "Unprocessable Entity", 422),
            r#"{"error":"validation failed","fields":{"a":"b"},"status":422}"#
        );
        // Nothing to overwrite: both members are appended, in the source's order.
        assert_eq!(
            with_error(r#"{"fields":{}}"#, "", "Not Found", 404),
            r#"{"fields":{},"error":"Not Found","status":404}"#
        );
        // An empty object and no fallback at all.
        assert_eq!(with_error("{}", "", "", 500), r#"{"error":"Request failed","status":500}"#);
        // `message` stands in for `error`, and a `status` already there is replaced in place.
        assert_eq!(
            with_error(r#"{"status":1,"message":"nope"}"#, "", "Bad Request", 400),
            r#"{"status":400,"message":"nope","error":"nope"}"#
        );
        // A body that is not an object at all reaches this as `{}` plus the raw text.
        assert_eq!(with_error("{}", "boom", "Bad Gateway", 502), r#"{"error":"boom","status":502}"#);
    }

    #[test]
    fn one_event_per_block() {
        let event = parse_sse_block("event: tick\ndata: hello\nid: 7").unwrap();
        assert_eq!(event.event, "tick");
        assert_eq!(event.data, "hello");
        assert_eq!(event.id.as_deref(), Some("7"));
    }

    #[test]
    fn a_block_without_data_is_no_event() {
        assert!(parse_sse_block("event: ping\nid: 1").is_none());
        assert!(parse_sse_block("").is_none());
        assert!(parse_sse_block(": just a comment").is_none());
        // One empty `data:` line is still an event, with empty data.
        let event = parse_sse_block("data:").unwrap();
        assert_eq!(event.data, "");
        assert_eq!(event.event, "message");
        assert_eq!(event.id, None);
    }

    #[test]
    fn several_data_lines_join_with_newlines() {
        let event = parse_sse_block("data: one\ndata: two\ndata:\ndata: three").unwrap();
        assert_eq!(event.data, "one\ntwo\n\nthree");
    }

    #[test]
    fn only_the_first_space_after_the_colon_is_the_separator() {
        let event = parse_sse_block("data:  two spaces\ndata:none").unwrap();
        assert_eq!(event.data, " two spaces\nnone");
    }

    #[test]
    fn a_value_may_hold_colons_and_json() {
        let event = parse_sse_block("data: {\"a\": \"b: c\"}").unwrap();
        assert_eq!(event.data, "{\"a\": \"b: c\"}");
    }

    #[test]
    fn carriage_returns_belong_to_the_break() {
        let event = parse_sse_block("event: tick\r\ndata: hello\r").unwrap();
        assert_eq!(event.event, "tick");
        assert_eq!(event.data, "hello");
    }

    #[test]
    fn a_field_without_a_colon_has_an_empty_value() {
        let event = parse_sse_block("data\nevent").unwrap();
        assert_eq!(event.data, "");
        assert_eq!(event.event, "");
    }

    #[test]
    fn fields_nobody_knows_are_ignored() {
        let event = parse_sse_block("retry: 500\ndata: x\nwhat: ever").unwrap();
        assert_eq!(event.data, "x");
        assert_eq!(event.event, "message");
    }

    #[test]
    fn frames_end_at_a_blank_line_however_it_is_written() {
        assert_eq!(frame("data: a\n\ndata: b"), Some((7, 9)));
        assert_eq!(frame("data: a\r\n\r\nrest"), Some((7, 11)));
        assert_eq!(frame("data: a\n\r\nrest"), Some((7, 10)));
        assert_eq!(frame("data: a\r\n\nrest"), Some((7, 10)));
        assert_eq!(frame("data: a\ndata: b"), None);
        assert_eq!(frame(""), None);
        assert_eq!(frame("\n\n"), Some((0, 2)));
    }

    #[test]
    fn a_filename_comes_out_of_the_disposition() {
        assert_eq!(filename("attachment; filename=\"report.csv\""), "report.csv");
        assert_eq!(filename("attachment; filename=report.csv"), "report.csv");
        assert_eq!(filename("ATTACHMENT; FileName=\"a b.pdf\"; size=1"), "a b.pdf");
        assert_eq!(filename("attachment"), "");
        assert_eq!(filename("inline"), "");
        // `filename*=` is not `filename=`, and the regex does not match it either.
        assert_eq!(filename("attachment; filename*=UTF-8''x.csv"), "");
    }

    #[test]
    fn numbers_without_core_fmt() {
        let mut out = String::new();
        push_number(&mut out, 0);
        push_number(&mut out, 7);
        push_number(&mut out, 422);
        push_number(&mut out, 65535);
        assert_eq!(out, "0742265535");
    }
}
