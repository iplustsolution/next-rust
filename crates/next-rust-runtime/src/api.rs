//! `window.nextRust`: the runtime's public API.
//!
//! Analytics snippets and SDK wrappers call these from JavaScript, so they
//! keep the names, the arguments and the promises the JavaScript runtime had:
//! `navigate`, `replace`, `back`, `forward`, `refresh`, `prefetch`, `action`,
//! `request`, `stream`, `loader.start` / `loader.reset` and `env`.
//!
//! The shim installs the forwarding functions once ([`op::INSTALL_API`]) and
//! every one of them calls back in through [`api_call`] with a number that
//! says which. Arguments arrive as handles into the shim's table, which are
//! released the moment `api_call` returns: anything that outlives the call —
//! the sink of a stream — is retained here first.
//!
//! Two things a promise can settle with have no shape in the ABI as it
//! stands, and are noted where they are used: a *parsed* value (an action
//! resolves with `body.data`, an object, not with its text) and an `Error`
//! carrying `.status`, `.errors` and `.data`. Both are written here as kinds
//! 3 and 4 of [`op::SETTLE`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use alloc::rc::Rc;

use crate::dom::{self, Node};
use crate::forms::{self, Answer, Options, Payload, Want};
use crate::host::{self, op};
use crate::loader::{self, Done};
use crate::nav::{self, NavOptions};
use crate::num;

/// [`op::SETTLE`]'s kinds.
mod settle {
    /// `(token, 0, handle, 0)`: resolve with the object that handle names.
    pub const HANDLE: u32 = 0;
    /// `(token, 1, ptr, len)`: resolve with a string.
    pub const STRING: u32 = 1;
    /// `(token, 2, ptr, len)`: reject with `new Error(message)`.
    pub const ERROR: u32 = 2;
    /// `(token, 3, ptr, len)`: resolve with `JSON.parse(text)`.
    pub const JSON: u32 = 3;
    /// `(token, 4, ptr, len)`: reject with an `Error` built from
    /// `{message, status, errors, data}`.
    pub const RICH_ERROR: u32 = 4;
}

// ---- installing ----------------------------------------------------------------

/// Publish `window.nextRust` and the environment.
///
/// Which element holds the environment is the framework's business, so it is
/// read here and the shim is handed the JSON text to parse: the shim never
/// looks the element up.
pub fn install() {
    let document = dom::document();
    let text = dom::by_id(&document, "__next_rust_env").map(|element| dom::text_content(&element)).unwrap_or_default();
    set_env(if text.is_empty() { "{}" } else { &text });
}

/// `NR.env = JSON.parse(…)`. A navigation that brought a new environment
/// element calls this with its text.
pub fn set_env(json: &str) {
    let (p, l) = host::s(json);
    host::call(op::INSTALL_API, p, l, 0, 0, 0);
}

// ---- what the shim calls -------------------------------------------------------

/// One call from `window.nextRust`. `which` is the shim's number for the
/// member that was called; the rest are its arguments, as handles, with the
/// promise's token last where there is one.
///
/// # Safety
/// Exported for the shim only.
#[unsafe(no_mangle)]
pub extern "C" fn api_call(which: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
    let _ = d; // no member of the API takes four arguments yet
    match which {
        1 => go(&handle_str(a), false, b),
        2 => go(&handle_str(a), true, b),
        3 => {
            dom::history_back();
        }
        4 => {
            dom::history_forward();
        }
        5 => refresh(a),
        6 => nav::prefetch(&handle_str(a)),
        7 => action(handle_str(a), handle_str(b), c),
        8 => request(handle_str(a), read_options(b), c),
        9 => stream(handle_str(a), read_options(b), c),
        10 => return start_load(a, b),
        11 => loader::reset(),
        _ => {}
    }
    0
}

/// `NR.navigate(href)` and `NR.replace(href)`.
fn go(href: &str, replace: bool, token: u32) {
    let href = href.to_string();
    crate::task::spawn(async move {
        settle_navigation(token, nav::navigate(&href, NavOptions { replace, ..NavOptions::default() }).await);
    });
}

/// `NR.refresh()`: this page again, in place, with nothing kept in memory.
fn refresh(token: u32) {
    crate::task::spawn(async move {
        let href = dom::href();
        let opts = NavOptions { replace: true, scroll: false, fresh: true, ..NavOptions::default() };
        settle_navigation(token, nav::navigate(&href, opts).await);
    });
}

/// A navigation resolves with nothing. It only rejects where the source's
/// does: a form whose post never reached the server, which is the browser's
/// to send and nothing this promise's caller asked for.
fn settle_navigation(token: u32, out: Result<Option<Node>, nav::Unsent>) {
    match out {
        Ok(_) => settle_handle(token, None),
        Err(nav::Unsent) => settle_error(token, "Navigation failed"),
    }
}

/// `NR.action(url, input)`. The input arrives already stringified, which is
/// what `JSON.stringify(input === undefined ? null : input)` leaves.
fn action(url: String, input: String, token: u32) {
    crate::task::spawn(async move {
        match forms::action(&url, &input).await {
            // A redirect resolves with `undefined`; the page is on its way.
            Ok(answer) if answer.redirected => settle_handle(token, None),
            Ok(answer) => settle_json(token, &answer.data),
            Err(error) => {
                let mut body = String::from("{\"message\":");
                body.push_str(&crate::json::quote(&error.message));
                body.push_str(",\"status\":");
                body.push_str(&crate::islands::number_string(f64::from(error.status)));
                body.push_str(",\"errors\":");
                push_object(&mut body, &error.errors);
                // `code` and `data` are what a caller branches on: a quota
                // reached, a plan to upgrade, a mode that is blocked. Without
                // them the page has only the message to match against.
                if let Some(code) = &error.code {
                    body.push_str(",\"code\":");
                    body.push_str(&crate::json::quote(code));
                }
                // Left out where the answer carried no `data`: the member is
                // then `undefined` on the error, which is what `err.data =
                // body.data` leaves, rather than a `null` the page would have
                // to tell apart from a `data` the server really sent as null.
                if let Some(data) = &error.data {
                    body.push_str(",\"data\":");
                    body.push_str(data);
                }
                body.push('}');
                settle_rich_error(token, &body);
            }
        }
    });
}

/// `NR.request(url, options)`.
fn request(url: String, options: Options, token: u32) {
    crate::task::spawn(async move {
        match forms::request(&url, options).await {
            Ok(Answer::Empty) => settle_json(token, "null"),
            Ok(Answer::Text(text)) => settle_text(token, &text),
            Ok(Answer::Json(text)) => settle_json(token, &text),
            Ok(Answer::Blob(node) | Answer::Response(node)) => settle_handle(token, Some(&node)),
            Err(error) => settle_rich_error(token, &failure(&error)),
        }
    });
}

/// `Object.assign(new Error(message), { status, data })`, as the JSON the
/// shim rebuilds the `Error` from. The source gives the rejections of
/// `request` and of `stream` the same two members, so they are written once.
fn failure(error: &forms::RequestError) -> String {
    let mut body = String::from("{\"message\":");
    body.push_str(&crate::json::quote(&error.message));
    body.push_str(",\"status\":");
    body.push_str(&crate::islands::number_string(f64::from(error.status)));
    body.push_str(",\"data\":");
    match &error.data {
        Answer::Json(text) => body.push_str(text),
        Answer::Text(text) => body.push_str(&crate::json::quote(text)),
        _ => body.push_str("null"),
    }
    body.push('}');
    body
}

/// `NR.stream(url, options)`.
///
/// An async generator is not something WebAssembly can be, so the shape is
/// turned inside out: the events are parsed here and pushed into a queue the
/// shim drains from its generator. The shim knows nothing of server-sent
/// events; it moves frames it is handed.
fn stream(url: String, options: Options, sink: u32) {
    // The shim releases every argument the moment this call returns, and the
    // sink is written to for as long as the stream runs.
    let Some(sink) = Node::new(host::call(op::RETAIN, sink, 0, 0, 0, 0)) else { return };
    crate::task::spawn(async move {
        let target = sink.raw();
        let out = forms::stream(&url, options, |event| {
            let mut frame = String::with_capacity(event.event.len() + event.data.len() + 2);
            frame.push_str(&event.event);
            frame.push('\0');
            frame.push_str(&event.data);
            frame.push('\0');
            if let Some(id) = &event.id {
                frame.push_str(id);
            }
            let (p, l) = host::s(&frame);
            host::call(op::STREAM_PUSH, target, p, l, 0, 0);
        })
        .await;
        match out {
            Ok(()) => {
                host::call(op::STREAM_END, target, 0, 0, 0, 0);
            }
            // The stream's rejection is the same `Error` `request` rejects
            // with — `status` and `data` included, which is the whole of what
            // a caller has to tell a 404 from a 503 with. It travels as the
            // JSON of that error, because the sink's `end` takes a value the
            // shim has to build.
            Err(error) => {
                let body = failure(&error);
                let (p, l) = host::s(&body);
                host::call(op::STREAM_END, target, p, l, 0, 0);
            }
        }
    });
}

/// `NR.loader.start(limit, from)`, which answers with the function that ends
/// that load.
fn start_load(limit: u32, from: u32) -> u32 {
    let done = loader::start(handle_num(limit), handle_num(from));
    let slot: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(done)));
    let callback = crate::task::on(move |_| {
        // `done` does nothing the second time, which a taken slot is.
        let taken = slot.borrow_mut().take();
        if let Some(done) = taken {
            done.end();
        }
    });
    host::call(op::CALLBACK_FN, callback, 0, 0, 0, 0)
}

// ---- reading the arguments -------------------------------------------------------

/// `String(H[handle])`. Every argument the shim passes is a handle, strings
/// included; [`op::USP_STR`] is the table's `String(value)`.
fn handle_str(handle: u32) -> String {
    if handle == 0 {
        return String::new();
    }
    host::text(op::USP_STR, handle, 0, 0, 0, 0)
}

/// A number argument. `0` is the handle the shim makes of `undefined`, which
/// is an argument that was not passed at all.
fn handle_num(handle: u32) -> Option<f64> {
    (handle != 0).then(|| num::number(&handle_str(handle))).flatten()
}

/// `JSON.stringify(H[handle])`, or `"null"` where there is nothing to write.
fn stringify(node: &Node) -> String {
    let text = host::text(op::STRINGIFY, node.raw(), 0, 0, 0, 0);
    if text.is_empty() { String::from("null") } else { text }
}

/// The second argument of `request` and `stream`, as this runtime holds it.
fn read_options(handle: u32) -> Options {
    let mut options = Options::default();
    let Some(node) = Node::new(host::call(op::RETAIN, handle, 0, 0, 0, 0)) else { return options };
    options.method = dom::prop_str(&node, "method").unwrap_or_default();
    options.want = match dom::prop_str(&node, "as").unwrap_or_default().as_str() {
        "blob" => Want::Blob,
        "text" => Want::Text,
        "response" => Want::Response,
        _ => Want::Auto,
    };
    let timeout = dom::prop_num(&node, "timeout");
    // `if (options.timeout && …)`: zero and `NaN` are no timeout at all.
    if timeout != 0.0 && !timeout.is_nan() {
        options.timeout = Some(timeout);
    }
    options.signal = dom::prop_node(&node, "signal");
    options.headers = read_headers(&node);
    options.body = read_body(&node);
    options
}

/// `options.headers`, a plain object, as the pairs a request sends.
fn read_headers(options: &Node) -> Vec<(String, String)> {
    let Some(headers) = dom::prop_node(options, "headers") else { return Vec::new() };
    let text = stringify(&headers);
    let Some(object) = crate::json::Object::new(&text) else { return Vec::new() };
    // `crate::json::text` is `String(value)` for a written value, which is
    // what a header field becomes on its way into `fetch`.
    object.members().into_iter().map(|(name, raw)| (name, crate::json::text(raw))).collect()
}

/// `options.body`. A string, a `FormData`, a `URLSearchParams`, a `Blob` or
/// an `ArrayBuffer` travels as it is; anything else is stringified and sent
/// as JSON, which is what sets `content-type: application/json`.
fn read_body(options: &Node) -> Option<Payload> {
    if dom::prop_is_string(options, "body") {
        return Some(Payload::Text(dom::prop_str(options, "body").unwrap_or_default()));
    }
    let body = dom::prop_node(options, "body")?;
    // `instanceof` is not in the table, so the four kinds are known by what
    // only they have: `append` on the two field collections, `byteLength` on
    // an `ArrayBuffer`, `size` and `type` on a `Blob`.
    let raw = dom::has_prop(&body, "append")
        || dom::has_prop(&body, "byteLength")
        || (dom::has_prop(&body, "size") && dom::has_prop(&body, "type"));
    if raw {
        return Some(Payload::Handle(body));
    }
    Some(Payload::Json(stringify(&body)))
}

// ---- settling the promises the shim handed out -------------------------------------

fn settle_handle(token: u32, node: Option<&Node>) {
    host::call(op::SETTLE, token, settle::HANDLE, node.map_or(0, Node::raw), 0, 0);
}

fn settle_text(token: u32, text: &str) {
    let (p, l) = host::s(text);
    host::call(op::SETTLE, token, settle::STRING, p, l, 0);
}

/// Resolve with a value, not with its text: `nextRust.action` answers with
/// `body.data` the way the JavaScript runtime did.
fn settle_json(token: u32, text: &str) {
    let (p, l) = host::s(if text.is_empty() { "null" } else { text });
    host::call(op::SETTLE, token, settle::JSON, p, l, 0);
}

/// Reject with an `Error` carrying `.status`, `.errors`, `.code` and `.data`.
fn settle_rich_error(token: u32, body: &str) {
    let (p, l) = host::s(body);
    host::call(op::SETTLE, token, settle::RICH_ERROR, p, l, 0);
}

/// Reject with `new Error(message)` and nothing else hung off it.
fn settle_error(token: u32, message: &str) {
    let (p, l) = host::s(message);
    host::call(op::SETTLE, token, settle::ERROR, p, l, 0);
}

/// `{"a":"b"}` from pairs, for the `errors` of a failed action.
fn push_object(out: &mut String, pairs: &[(String, String)]) {
    out.push('{');
    for (at, (key, value)) in pairs.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push_str(&crate::json::quote(key));
        out.push(':');
        out.push_str(&crate::json::quote(value));
    }
    out.push('}');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_errors_object_is_written_the_way_the_page_reads_it() {
        let mut out = String::new();
        push_object(&mut out, &[]);
        assert_eq!(out, "{}");
        let mut out = String::new();
        push_object(&mut out, &[("email".to_string(), "Required".to_string()), ("a\"b".to_string(), "x".to_string())]);
        assert_eq!(out, r#"{"email":"Required","a\"b":"x"}"#);
    }

    #[test]
    fn a_handle_of_nothing_is_an_argument_that_was_not_passed() {
        assert_eq!(handle_str(0), "");
        assert_eq!(handle_num(0), None);
    }

    /// A rejected `request` and a rejected `stream` are the same object to the
    /// page: the shim rebuilds both with `Object.assign(new Error(message),
    /// rest)`, so every member but `message` has to survive this encoding.
    /// `stream` once sent the bare message, which left `e.status` and
    /// `e.data` `undefined` where the JavaScript runtime gives `404` and the
    /// body — a divergence only a browser could see.
    #[test]
    fn a_failure_carries_the_status_and_the_body_the_server_sent() {
        let text = failure(&forms::RequestError {
            message: "Not Found".into(),
            status: 404,
            data: forms::Answer::Text("Not Found".into()),
        });
        assert_eq!(text, r#"{"message":"Not Found","status":404,"data":"Not Found"}"#);

        // A JSON body is spliced in as-is: the page gets the object the server
        // sent, not a string holding its source.
        let text = failure(&forms::RequestError {
            message: "validation failed".into(),
            status: 422,
            data: forms::Answer::Json(r#"{"fields":{"text":"Say something"}}"#.into()),
        });
        assert_eq!(text, r#"{"message":"validation failed","status":422,"data":{"fields":{"text":"Say something"}}}"#);

        // No body at all is `null`, never a missing member: the shim spreads
        // whatever is here onto the Error.
        let text = failure(&forms::RequestError { message: "gone".into(), status: 410, data: forms::Answer::Empty });
        assert_eq!(text, r#"{"message":"gone","status":410,"data":null}"#);

        // A message with a quote in it has to stay parseable.
        let text =
            failure(&forms::RequestError { message: r#"say "no""#.into(), status: 400, data: forms::Answer::Empty });
        assert_eq!(text, r#"{"message":"say \"no\"","status":400,"data":null}"#);
    }
}
