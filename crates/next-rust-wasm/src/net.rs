//! Everything an island waits for: timers, frames, fetches, actions, streams.
//!
//! Each of these is the same shape. A [`Token`] is reserved, one operation is
//! sent to the host carrying that token, and the future parks until the shim
//! resumes it. Nothing here drives anything — [`crate::task`] does the polling
//! and the host does the waiting — so the whole module is argument packing and
//! payload parsing, which is why it costs so little.
//!
//! The payloads are flat on purpose. There is no JSON parser in the module and
//! there is not going to be one: a fetch body is bytes, an event frame is
//! three lines, and an action's field errors are tab-separated pairs. The one
//! exception is an action's *value*, which the shim decodes into the same
//! compact props blob the server sends at hydration, so reading it costs a
//! key lookup instead of a parser.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::dom::{El, File, Val};
use crate::host::{call, call_s, call_s0, call_ss, call1, call2, op, s};
use crate::props::Props;
use crate::task::{Msg, TAG_ERROR, TAG_VALUE, Token};
use crate::text::Str;

// --- the two ways to wait ----------------------------------------------------

/// Waits for the single message a one-shot operation produces, and settles the
/// token as it arrives so that dropping the future afterwards aborts nothing.
struct Wait(Token);

impl Future for Wait {
    type Output = Msg;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Msg> {
        let it = self.get_mut();
        match it.0.poll(cx) {
            Poll::Ready(msg) => {
                it.0.settle();
                Poll::Ready(msg)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Waits for the next message on a token that produces many. The token is
/// borrowed rather than owned, because the stream outlives the frame.
struct Frame<'a>(&'a Token);

impl Future for Frame<'_> {
    type Output = Msg;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Msg> {
        self.0.poll(cx)
    }
}

// --- timers and frames -------------------------------------------------------

/// `setTimeout`. Dropping the future clears the timer.
pub async fn timeout(ms: u32) {
    let token = Token::new();
    call2(op::TIMEOUT, token.id(), ms);
    Wait(token).await;
}

/// `requestAnimationFrame`, resolving to the timestamp the browser passes the
/// callback. Cancelled on drop, so an animation loop stops when its island
/// does.
pub async fn raf() -> f64 {
    let token = Token::new();
    call1(op::RAF, token.id());
    number(&Wait(token).await)
}

/// `requestIdleCallback`, or a one millisecond timer where that does not
/// exist. The deadline is not passed through: an island that needs to know how
/// long it has should measure with [`crate::now`] instead of trusting a number
/// that means something different in every browser.
pub async fn idle() {
    let token = Token::new();
    call1(op::IDLE, token.id());
    Wait(token).await;
}

/// The number a resume carried, either as the eight bytes of an `f64` or as
/// the decimal text of one. Both are accepted because a timestamp is the only
/// numeric payload in the ABI and it is not worth a second opcode to pin down;
/// anything else reads as zero rather than failing.
fn number(msg: &Msg) -> f64 {
    if let Ok(bytes) = <[u8; 8]>::try_from(msg.body.as_slice()) {
        return f64::from_le_bytes(bytes);
    }
    crate::text::parse_f64(as_text(&msg.body).trim()).unwrap_or(0.0)
}

// --- fetch -------------------------------------------------------------------

/// A request, built up and then handed to [`fetch`].
///
/// The pieces are assembled into the `{method, headers, body}` object the
/// shim passes to `fetch` at the moment the request is sent, so building one
/// costs a couple of small strings and no host calls at all.
pub struct Request {
    url: String,
    method: String,
    headers: String,
    body: Option<String>,
}

impl Request {
    pub fn get(url: &str) -> Request {
        Request::with(url, "GET")
    }

    pub fn post(url: &str) -> Request {
        Request::with(url, "POST")
    }

    fn with(url: &str, method: &str) -> Request {
        Request { url: String::from(url), method: String::from(method), headers: String::new(), body: None }
    }

    pub fn method(mut self, method: &str) -> Request {
        self.method.clear();
        self.method.push_str(method);
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Request {
        if !self.headers.is_empty() {
            self.headers.push(',');
        }
        push_json(&mut self.headers, name);
        self.headers.push(':');
        push_json(&mut self.headers, value);
        self
    }

    /// A JSON body, and the content type that goes with it. The string is sent
    /// as it stands — it is the island's own JSON, not something parsed here.
    pub fn json(self, body: &str) -> Request {
        self.header("content-type", "application/json").body(body)
    }

    pub fn body(mut self, body: &str) -> Request {
        self.body = Some(String::from(body));
        self
    }

    fn options(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"method\":");
        push_json(&mut out, &self.method);
        if !self.headers.is_empty() {
            out.push_str(",\"headers\":{");
            out.push_str(&self.headers);
            out.push('}');
        }
        if let Some(body) = &self.body {
            out.push_str(",\"body\":");
            push_json(&mut out, body);
        }
        out.push('}');
        out
    }
}

/// `fetch`. The response body arrives as bytes in linear memory, so this is
/// for the small replies islands actually read; anything large should be a
/// stream. Dropping the future aborts the request.
pub async fn fetch(req: Request) -> Result<Response, Error> {
    let token = Token::new();
    let options = req.options();
    let (url_ptr, url_len) = s(&req.url);
    let (opt_ptr, opt_len) = s(&options);
    call(op::FETCH, token.id(), url_ptr, url_len, opt_ptr, opt_len);
    let msg = Wait(token).await;
    if msg.tag == TAG_VALUE {
        Ok(Response { status: msg.status, body: msg.body })
    } else {
        Err(Error { status: msg.status, body: msg.body })
    }
}

/// What came back. A status the server sent, including a failing one — a 404
/// is a response, not an [`Error`].
pub struct Response {
    pub status: u16,
    body: Vec<u8>,
}

impl Response {
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// The body as text, or empty when it is not UTF-8. There is no `Result`
    /// here on purpose: an island that asked for text and got bytes has
    /// nothing useful to do with the error.
    pub fn text(&self) -> &str {
        as_text(&self.body)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.body
    }
}

/// The request did not produce a response. A status of zero means the browser
/// never got one — offline, blocked, or aborted because the future was
/// dropped.
pub struct Error {
    pub status: u16,
    body: Vec<u8>,
}

impl Error {
    /// The first line of the payload, which is the message and nothing else.
    pub fn message(&self) -> &str {
        message_of(&self.body)
    }

    /// The whole payload as text: the message, and for an action the field
    /// errors under it. Worth reading when a caller wants to log what the
    /// server actually said.
    pub fn body(&self) -> &str {
        as_text(&self.body)
    }
}

// --- actions -----------------------------------------------------------------

/// Call a server action with a JSON body, resolving to the props blob the shim
/// decoded the reply into.
///
/// Both action paths — this and [`Action::send`] — succeed with [`Props`] and
/// fail with [`ActionError`]. There is deliberately only one action error
/// type: the thing an island most often has to do with a failure is branch on
/// its `code()` or paint its `fields()` beside the inputs, and an error type
/// that could not do that would push every caller back into parsing JSON.
///
/// The reply is decoded host-side because the module has no JSON parser: what
/// comes back over the resume is the same compact encoding the server sends at
/// hydration, and it is read with [`Props`] and `k!` exactly like `cx.props`.
pub async fn action(url: &str, json: &str) -> Result<Props<'static>, ActionError> {
    let token = Token::new();
    let (url_ptr, url_len) = s(url);
    let (json_ptr, json_len) = s(json);
    call(op::ACTION, token.id(), url_ptr, url_len, json_ptr, json_len);
    let msg = Wait(token).await;
    if msg.tag == TAG_VALUE { Ok(props(msg.body)) } else { Err(ActionError { status: msg.status, body: msg.body }) }
}

/// A props blob has to outlive the call that produced it, the same way the
/// hydration blob outlives `hydrate_<Name>`, so the bytes are leaked.
///
/// The cost is honest and worth stating plainly, because the `'static`
/// lifetime makes holding one look safe: **do not keep a reply's `Props`**.
/// Read the fields you need out of it and keep those. An island that calls an
/// action on a debounce — an assessment autosaving sixty answers — leaks a
/// reply's worth of memory per call if it keeps them, and nothing at all if it
/// does not.
fn props(body: Vec<u8>) -> Props<'static> {
    Props::new(Box::leak(body.into_boxed_slice()))
}

/// Call an action and get the reply back as a JavaScript value handle.
///
/// [`action`] decodes the reply into a props blob, which is what an island
/// branching on a nested response wants. This is for the other case: handing
/// an opaque value on to somebody else — a `nr:success` detail, an SDK — where
/// hashing the field names away would destroy the thing being passed. It is
/// also how a reply whose *keys are the data* is read, since a blob cannot
/// give a key name back.
pub async fn action_val(url: &str, json: &str) -> Result<Val, ActionError> {
    let token = Token::new();
    let (url_ptr, url_len) = s(url);
    let (json_ptr, json_len) = s(json);
    call(op::ACTION_VAL, token.id(), url_ptr, url_len, json_ptr, json_len);
    let msg = Wait(token).await;
    if msg.tag == TAG_VALUE {
        let text = core::str::from_utf8(&msg.body).unwrap_or("0");
        Ok(Val::from_handle(crate::text::parse_i64(text).unwrap_or(0) as u32))
    } else {
        Err(ActionError { status: msg.status, body: msg.body })
    }
}

/// Call an action and get the reply back as JSON text, exactly as the server
/// sent it. `JSON.stringify` is the browser's, so nothing here writes JSON
/// either.
///
/// Reach for this over [`action`] whenever the reply has to survive intact
/// rather than be read: a WebAuthn credential-options object, an SDK's config,
/// anything handed straight on to somebody else. [`action`] decodes into the
/// props blob, which hashes field names one way — right for a reply you
/// branch on by known key, and **data loss** for a deeply nested object whose
/// names have to reach the browser again. That distinction is a silent bug if
/// you get it wrong, not a compile error.
pub async fn action_text(url: &str, json: &str) -> Result<Str, ActionError> {
    let value = action_val(url, json).await?;
    let text = crate::dom::window().get("JSON").call_val("stringify", value, Val::NULL).str();
    value.free();
    Ok(text)
}

/// An action built field by field, rather than as one JSON string.
///
/// This is the form that can carry a file: the payload is assembled on the
/// browser side, and the island only ever names the pieces. A five megabyte
/// upload is a handle and a field name here, never five megabytes of linear
/// memory.
pub struct Action(u32);

impl Action {
    pub fn new(url: &str) -> Action {
        Action(call_s0(op::ACTION_NEW, url))
    }

    pub fn field(self, name: &str, value: &str) -> Action {
        call_ss(op::ACTION_FIELD, self.0, name, value);
        self
    }

    /// Attach a file, base64 encoded by the shim as it reads it.
    pub fn file_base64(self, name: &str, file: File) -> Action {
        let (ptr, len) = s(name);
        call(op::ACTION_FILE_B64, self.0, ptr, len, (file.0).0, 0);
        self
    }

    /// Copy every string entry of a form, as `new FormData(form)` sees it.
    /// Files are not copied — name those with [`Action::file_base64`].
    pub fn form_fields(self, form: El) -> Action {
        call2(op::ACTION_FORM_FIELDS, self.0, form.0);
        self
    }

    /// Send it. Dropping the future aborts the request.
    pub async fn send(self) -> Result<Props<'static>, ActionError> {
        let token = Token::new();
        call2(op::ACTION_SEND, self.0, token.id());
        let msg = Wait(token).await;
        if msg.tag == TAG_VALUE { Ok(props(msg.body)) } else { Err(ActionError { status: msg.status, body: msg.body }) }
    }
}

/// Why an action failed, as the flat payload the shim resumes with:
/// `[message]\n[field\tmessage\n]*`.
///
/// Splitting bytes rather than parsing JSON is the whole point — the encoding
/// exists so that the one thing an island always has to handle, a form coming
/// back with errors on it, costs a few hundred bytes instead of a parser.
///
/// Names beginning with an underscore are reserved for the envelope rather
/// than being form fields: `_code` carries the machine-readable code and
/// `_form` a message that belongs above the form instead of beside an input.
/// Neither appears in [`ActionError::fields`].
///
/// One caveat worth knowing before you branch on [`ActionError::code`]: the
/// framework's own `nextRust.action` currently rejects with an error carrying
/// only `errors` and `status`, so a `code` the server sent is dropped before
/// it reaches here. `nextRust.request` and `stream` do carry it. Until that is
/// fixed in the runtime, `code()` is reliable on the streaming path and empty
/// on the action path — which is a real product bug where an island branches
/// on a code to show "you are over quota" rather than a raw message.
pub struct ActionError {
    status: u16,
    body: Vec<u8>,
}

impl ActionError {
    pub fn status(&self) -> u16 {
        self.status
    }

    /// What went wrong, in one line, for a human.
    pub fn message(&self) -> &str {
        message_of(&self.body)
    }

    /// The machine-readable code, empty when the server sent none. This is
    /// what to compare against for the cases an island handles specially,
    /// such as `next_rust_stale` after a deploy.
    pub fn code(&self) -> &str {
        entry(&self.body, "_code").unwrap_or("")
    }

    /// The message to show above the form: the one the server marked as
    /// form-level, or failing that the message itself.
    pub fn form_error(&self) -> &str {
        entry(&self.body, "_form").unwrap_or_else(|| self.message())
    }

    /// The form-level error exactly as the server sent it, or `None`.
    ///
    /// Unlike [`ActionError::form_error`] this does **not** fall back to
    /// [`ActionError::message`]. An island that shows its own sentence when
    /// the server gave no form-level error has to tell those two cases apart:
    /// with only the falling-back accessor, a 500 or any other non-validation
    /// failure puts the server's raw wording in front of someone trying to
    /// sign in. Comparing `form_error()` against `message()` is not a
    /// substitute — it is a guess that breaks against a server whose
    /// form error happens to equal its top-level message, which is exactly
    /// what happens when the runtime builds that one-line message *from* the
    /// form error. (Comparing the two by address rather than by text would be
    /// exact, since `_form` lies after the payload's first newline and the
    /// message starts at the first byte — but needing that trick at all is
    /// the sign the accessor was missing.)
    pub fn form(&self) -> Option<&str> {
        entry(&self.body, "_form")
    }

    /// The error on one named field, if it has one.
    pub fn field(&self, name: &str) -> Option<&str> {
        if name.starts_with('_') {
            return None;
        }
        entry(&self.body, name)
    }

    /// Every field error, in the order the server wrote them.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &str)> {
        entries(&self.body).filter(|(name, _)| !name.starts_with('_'))
    }

    /// Build one from a resume payload, for the tests that feed the parser by
    /// hand. Off the browser only: on it, the way to get one of these is to
    /// have sent an action.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_payload(payload: &[u8]) -> ActionError {
        let msg = crate::task::decode(payload);
        ActionError { status: msg.status, body: msg.body }
    }
}

// --- server-sent events ------------------------------------------------------

/// An open `EventSource`, or the shim's `fetch`-backed equivalent for a
/// stream that had to be a POST.
///
/// Dropping one closes the stream, so a stream stops when the island holding
/// it goes away, with no explicit teardown to forget.
pub struct Sse {
    token: Token,
    ended: bool,
    /// Why the stream ended, when it did not end cleanly. Without this a 429
    /// on the stream is indistinguishable from the server finishing, and an
    /// island that branches on an error code has no code to branch on.
    failure: Option<(u16, String)>,
}

/// A handle that can stop a stream from somewhere else.
///
/// [`Sse::close`] consumes the stream, which is no use to a Stop button whose
/// handler does not own it — the send loop does, parked in [`Sse::next`].
/// This is `Copy`, so it can be captured by as many closures as need it.
#[derive(Clone, Copy, Debug)]
pub struct SseStop(u32);

impl SseStop {
    /// Close the stream. The pending `next()` then returns `None` rather than
    /// staying parked.
    pub fn stop(self) {
        call1(op::SSE_CLOSE, self.0);
    }
}

/// One frame off a stream, as the `event\nid\ndata` the shim packs. The data
/// keeps its own newlines: only the first two separate the header lines.
pub struct SseEvent {
    pub event: Str,
    pub data: Str,
    pub id: Str,
}

/// Open a stream with a GET, which is what `EventSource` can do by itself.
pub fn sse(url: &str) -> Sse {
    let token = Token::new();
    call_s(op::SSE_OPEN, token.id(), url);
    Sse { token, ended: false, failure: None }
}

/// Open a stream with a POST body, which `EventSource` cannot do — the shim
/// reads the body out of the staged argument and falls back to `fetch` with a
/// reader. The body is staged rather than passed because the operation's five
/// arguments are already spoken for.
pub fn sse_post(url: &str, body_json: &str) -> Sse {
    let token = Token::new();
    let (ptr, len) = s(body_json);
    call(op::ARG, 0, ptr, len, 0, 0);
    call_s(op::SSE_OPEN, token.id(), url);
    Sse { token, ended: false, failure: None }
}

impl Sse {
    /// The next frame, or `None` once the stream has ended — which covers the
    /// server closing it, an error, and a stream that was already closed here.
    pub async fn next(&mut self) -> Option<SseEvent> {
        if self.ended {
            return None;
        }
        let msg = Frame(&self.token).await;
        if msg.tag != TAG_VALUE {
            self.ended = true;
            if msg.tag == TAG_ERROR {
                self.failure = Some((msg.status, String::from_utf8(msg.body).unwrap_or_default()));
            }
            self.token.settle();
            return None;
        }
        Some(frame(&msg.body))
    }

    /// Why the stream ended, if it did not end cleanly.
    ///
    /// `next()` answers `None` for both a clean end and a failure, because a
    /// consumer usually wants one loop either way. This is how to tell them
    /// apart afterwards — and it is the only place a stream's error code is
    /// available, so an island that shows "you are over quota" rather than a
    /// raw message has to read it.
    pub fn error(&self) -> Option<&str> {
        self.failure.as_ref().map(|(_, message)| message.as_str())
    }

    /// The status the stream failed with, or `0` — for a network failure, an
    /// abort, or a stream that ended cleanly.
    pub fn status(&self) -> u16 {
        self.failure.as_ref().map_or(0, |(status, _)| *status)
    }

    /// A handle that can stop this stream from another closure.
    pub fn aborter(&self) -> SseStop {
        SseStop(self.token.id())
    }

    /// Close the stream now. Settling first is what keeps this from also
    /// sending an abort as the token drops.
    pub fn close(self) {
        if !self.ended {
            call1(op::SSE_CLOSE, self.token.id());
        }
        self.token.settle();
    }
}

fn frame(body: &[u8]) -> SseEvent {
    let text = as_text(body);
    let (event, rest) = text.split_once('\n').unwrap_or((text, ""));
    let (id, data) = rest.split_once('\n').unwrap_or((rest, ""));
    SseEvent { event: Str::from(event), data: Str::from(data), id: Str::from(id) }
}

// --- flat payloads -----------------------------------------------------------

/// Bytes as text, or empty when they are not UTF-8. Every payload parser below
/// starts here, which is why none of them can fail: a malformed resume reads
/// as an empty message with no fields rather than as an error an island would
/// have to handle.
fn as_text(body: &[u8]) -> &str {
    core::str::from_utf8(body).unwrap_or("")
}

fn message_of(body: &[u8]) -> &str {
    let text = as_text(body);
    match text.split_once('\n') {
        Some((first, _)) => first,
        None => text,
    }
}

fn entries(body: &[u8]) -> impl Iterator<Item = (&str, &str)> {
    let text = as_text(body);
    let rest = match text.split_once('\n') {
        Some((_, rest)) => rest,
        None => "",
    };
    rest.split('\n').filter_map(|line| line.split_once('\t'))
}

fn entry<'a>(body: &'a [u8], name: &str) -> Option<&'a str> {
    entries(body).find(|(field, _)| *field == name).map(|(_, message)| message)
}

/// Append a JSON string literal. The escaping is the minimum the grammar
/// needs — quote, backslash and the control characters — because the strings
/// this ever sees are an island's own header values and request bodies.
fn push_json(out: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                let code = ch as u32;
                out.push_str("\\u00");
                out.push(HEX[(code >> 4) as usize] as char);
                out.push(HEX[(code & 0xf) as usize] as char);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}
