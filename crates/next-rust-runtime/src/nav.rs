//! Client navigation: the page cache, prefetching, rendering and morphing.
//!
//! This is `RUNTIME_JS` from `swapStreamed` down to `popstate`, in the order
//! the source has it. Every rule is the source's: which links are internal,
//! when a prefetch fires, what a partial answer replaces, which scripts run
//! again. Nothing here asks the browser for a decision.

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use crate::dom::{self, Node};
use crate::host;
use crate::task::{self, Timer};

/// Pages fetched by prefetching or navigation are reused for this long, so a
/// page is never downloaded twice in a row: a click after a prefetch, a second
/// click, or going back all use the copy already in memory.
const PAGE_TTL: f64 = 30000.0;
/// How long the pointer must rest on a link before its page is fetched.
const PREFETCH_DELAY: f64 = 400.0;
/// A load this document will not see the end of gives up after this long.
const FULL_LOAD_LIMIT: f64 = 30000.0;

/// The head elements the server owns and the runtime therefore replaces.
const MANAGED: &str = "meta[name]:not([name=\"viewport\"]),meta[property],link[rel=\"canonical\"],style[data-nr-css]";
/// The same selector without the styles, which are reconciled by their id
/// rather than removed (the source's `managed.replace(...)`).
const MANAGED_HEAD: &str = "meta[name]:not([name=\"viewport\"]),meta[property],link[rel=\"canonical\"]";

// ---- what the other modules see ---------------------------------------------

/// A page the server answered with, or the response to a form that posted
/// something which is not a page.
pub struct Page {
    /// `res.url`: where the request ended up, after any redirect.
    pub url: String,
    pub html: Option<String>,
    /// The response itself, for a post whose answer is not HTML.
    pub response: Option<Node>,
    /// An intercepted route renders differently depending on where the
    /// navigation started, so that copy is only valid from the same page.
    pub from: Option<String>,
    /// Only the inside of this layout was sent.
    pub partial: Option<String>,
}

/// What a form posts, and the controller it can be dropped with.
pub struct PostBody {
    pub body: crate::forms::Body,
    pub signal: Option<Node>,
}

/// The second argument of `navigate`.
pub struct NavOptions {
    pub replace: bool,
    pub scroll: bool,
    pub history: bool,
    pub fresh: bool,
    pub quiet: bool,
    pub focus: bool,
    /// The link that was clicked, whose open `<details>` menus close.
    pub link: Option<Node>,
    /// A selector (`data-nr-target` on a form): when both the page and the
    /// answer have that element, only its content changes.
    pub target: Option<String>,
    pub post: Option<PostBody>,
    /// A live search drops the request of the keystroke before.
    pub signal: Option<Node>,
}

impl Default for NavOptions {
    fn default() -> Self {
        Self {
            replace: false,
            scroll: true,
            history: true,
            fresh: false,
            quiet: false,
            focus: false,
            link: None,
            target: None,
            post: None,
            signal: None,
        }
    }
}

// ---- state -------------------------------------------------------------------

/// One entry of the page cache. `page` is `None` while the request is still
/// in the air, which is the source's shared promise.
struct Entry {
    at: f64,
    request: u32,
    page: Option<Rc<Page>>,
}

/// A request several callers are waiting on: the source hands them all the
/// same promise, so here they all wait on the same token list.
#[derive(Default)]
struct Flight {
    waiters: Vec<u32>,
    readers: u32,
    done: Option<Result<Rc<Page>, ()>>,
}

#[derive(Default)]
struct State {
    pages: BTreeMap<String, Entry>,
    flights: BTreeMap<u32, Flight>,
    next_flight: u32,
    /// Inline delivery: the ids of the scripts this document has run.
    inlined: Vec<String>,
    /// Where the page is, as `pathname + search`.
    current: String,
    navigating: u32,
    hover_link: Option<Node>,
    hover_timer: Option<Timer>,
}

crate::local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    STATE.with(|state| f(&mut state.borrow_mut()))
}

fn current() -> String {
    with_state(|state| state.current.clone())
}

// ---- pure helpers ---------------------------------------------------------------

/// `[...parts].join(sep)`.
fn join(parts: &[String], sep: &str) -> String {
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(part);
    }
    out
}

/// Set a header, replacing one of the same name: an object literal's keys.
fn set_header(headers: &mut Vec<(String, String)>, name: &str, value: &str) {
    match headers.iter_mut().find(|(key, _)| key == name) {
        Some(entry) => entry.1 = value.to_owned(),
        None => headers.push((name.to_owned(), value.to_owned())),
    }
}

/// The headers of a navigation request, in the order the source writes them.
fn nav_headers(
    from: &str,
    csrf: Option<&str>,
    layouts: &[String],
    styles: &[String],
    scripts: &[String],
) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    set_header(&mut headers, "x-nr-nav", "1");
    set_header(&mut headers, "x-nr-from", from);
    set_header(&mut headers, "accept", "text/html");
    if let Some(token) = csrf {
        set_header(&mut headers, "x-csrf-token", token);
    }
    if !layouts.is_empty() {
        set_header(&mut headers, "x-nr-layouts", &join(layouts, ","));
    }
    // Stylesheets (and utility rules) the page has are not sent again.
    if !styles.is_empty() {
        set_header(&mut headers, "x-nr-styles", &join(styles, ","));
    }
    // Inline delivery: scripts the document has are not sent again either.
    if !scripts.is_empty() {
        set_header(&mut headers, "x-nr-scripts", &join(scripts, ","));
    }
    headers
}

/// `/(?:^|;\s*)next_rust_csrf=([^;]+)/` over `document.cookie`.
fn csrf_token(cookie: &str) -> Option<&str> {
    const NAME: &str = "next_rust_csrf=";
    let mut from = 0;
    while let Some(at) = cookie[from..].find(NAME) {
        let start = from + at;
        let before = &cookie[..start];
        let attached = before.is_empty() || before.trim_end_matches(char::is_whitespace).ends_with(';');
        let value = cookie[start + NAME.len()..].split(';').next().unwrap_or("");
        if attached && !value.is_empty() {
            return Some(value);
        }
        from = start + NAME.len();
    }
    None
}

/// `location.pathname.length > 1 ? location.pathname.replace(/\/+$/, "") : ...`
fn trim_slashes(path: &str) -> &str {
    if path.len() > 1 { path.trim_end_matches('/') } else { path }
}

/// Whether a link is the page (`active_class`) and whether the page is below
/// it (`active_class_prefix`), both paths normalised first.
fn active_state(here: &str, path: &str) -> (bool, bool) {
    let (here, path) = (trim_slashes(here), trim_slashes(path));
    let exact = here == path;
    let mut below = exact || path == "/";
    if !below {
        let mut prefix = String::with_capacity(path.len() + 1);
        prefix.push_str(path);
        prefix.push('/');
        below = here.starts_with(&prefix);
    }
    (exact, below)
}

/// The cache's own rule: a page older than [`PAGE_TTL`] is gone.
fn evict(pages: &mut BTreeMap<String, Entry>, now: f64) {
    pages.retain(|_, entry| now - entry.at < PAGE_TTL);
}

/// `decodeURIComponent`, which throws on an escape that is not UTF-8: the
/// anchor of a hash nobody could have written is then not found.
fn decode_uri_component(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            let mut byte = 0u8;
            for digit in hex.bytes() {
                byte = byte.checked_mul(16)? + (digit as char).to_digit(16)? as u8;
            }
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// `parseFloat(value) || 0`, written out because `core` has no float parser:
/// the digits become an integer and the decimal point becomes a power of ten.
fn parse_float(text: &str) -> f64 {
    let bytes = text.trim_start().as_bytes();
    let mut i = 0;
    let negative = matches!(bytes.first(), Some(b'-'));
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        i = 1;
    }
    let (mut digits, mut seen, mut scale) = (0f64, false, 0i32);
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        digits = digits * 10.0 + f64::from(bytes[i] - b'0');
        seen = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            digits = digits * 10.0 + f64::from(bytes[i] - b'0');
            seen = true;
            scale -= 1;
            i += 1;
        }
    }
    if !seen {
        return 0.0;
    }
    if i < bytes.len() && (bytes[i] | 0x20) == b'e' {
        let mut j = i + 1;
        let exponent_negative = matches!(bytes.get(j), Some(b'-'));
        if matches!(bytes.get(j), Some(b'+') | Some(b'-')) {
            j += 1;
        }
        let (mut exponent, mut any) = (0i32, false);
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            exponent = exponent.saturating_mul(10).saturating_add(i32::from(bytes[j] - b'0'));
            any = true;
            j += 1;
        }
        if any {
            scale = scale.saturating_add(if exponent_negative { -exponent } else { exponent });
        }
    }
    let mut value = digits;
    for _ in 0..scale.abs().min(400) {
        value = if scale > 0 { value * 10.0 } else { value / 10.0 };
    }
    if negative { -value } else { value }
}

/// A JSON string, for the one event detail this module sends.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str("\\u00");
                let byte = c as u32;
                out.push(char::from_digit(byte >> 4, 16).unwrap_or('0'));
                out.push(char::from_digit(byte & 15, 16).unwrap_or('0'));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---- streamed boundaries ---------------------------------------------------------

fn swap_streamed(doc: &Node) {
    for template in dom::query_all(doc, "template[id^=\"nr-t\"]") {
        let id = dom::prop_str(&template, "id").unwrap_or_default();
        if let Some(rest) = id.get(4..) {
            let mut key = String::with_capacity(rest.len() + 4);
            key.push_str("nr-b");
            key.push_str(rest);
            if let (Some(boundary), Some(content)) = (dom::by_id(doc, &key), dom::template_content(&template)) {
                dom::replace_with(&boundary, &content);
            }
        }
        dom::remove(&template);
    }
}

// ---- layouts ----------------------------------------------------------------------
// The server wraps the children of every reusable layout in comment markers,
// <!--nr-l:KEY--> ... <!--/nr-l:KEY-->. Navigations send the keys on screen,
// and the server answers with only what goes inside the deepest shared layout.

fn layout_markers() -> Vec<Node> {
    let body = dom::body();
    dom::Comments::new(&body).filter(|node| dom::data(node).starts_with("nr-l:")).collect()
}

/// The start and end markers of a layout region, if it is on screen.
fn find_region(key: &str) -> Option<(Node, Node)> {
    let body = dom::body();
    let mut open = String::with_capacity(key.len() + 6);
    open.push_str("nr-l:");
    open.push_str(key);
    let mut close = String::with_capacity(open.len() + 1);
    close.push('/');
    close.push_str(&open);
    let mut start: Option<Node> = None;
    for node in dom::Comments::new(&body) {
        let data = dom::data(&node);
        if data == open {
            start = Some(node);
        } else if let Some(open_marker) = &start {
            let paired = data == close
                && dom::parent(&node).zip(dom::parent(open_marker)).is_some_and(|(a, b)| dom::same(&a, &b));
            if paired {
                return start.map(|open_marker| (open_marker, node));
            }
        }
    }
    None
}

// ---- fetching -----------------------------------------------------------------------

/// One request for a page. `post` is the rest of the request for a form.
///
/// The error carries nothing because the source's does not: every rejection
/// here — the network, a dropped request, an answer that is not a page — is
/// answered the same way, by letting the browser load the page itself.
#[allow(clippy::result_unit_err)]
pub async fn fetch_page(href: &str, full: bool, post: Option<&PostBody>) -> Result<Page, ()> {
    let signal = post.and_then(|post| post.signal.as_ref().map(Node::share));
    request_page(href, full, post, signal.as_ref()).await
}

async fn request_page(href: &str, full: bool, post: Option<&PostBody>, signal: Option<&Node>) -> Result<Page, ()> {
    let keys: Vec<String> = if full {
        Vec::new()
    } else {
        layout_markers().iter().map(|node| dom::data(node).get(5..).unwrap_or("").to_owned()).collect()
    };
    let document = dom::document();
    let styles: Vec<String> = dom::query_all(&document, "style[data-nr-css]")
        .iter()
        .map(|style| dom::dataset(style, "data-nr-css").unwrap_or_default())
        .collect();
    let scripts = with_state(|state| state.inlined.clone());
    let cookie = dom::cookie();
    let csrf = post.and_then(|_| csrf_token(&cookie));
    let headers = nav_headers(&dom::pathname(), csrf, &keys, &styles, &scripts);
    let body = post.map(|post| match &post.body {
        crate::forms::Body::Params(node) | crate::forms::Body::FormData(node) => dom::RequestBody::Object(node),
    });
    let method = if post.is_some() { "POST" } else { "GET" };
    let response = dom::fetch(href, method, &headers, body, signal).await.ok_or(())?;
    let content_type = dom::res_header(&response, "content-type").unwrap_or_default();
    if !content_type.contains("text/html") {
        // What a form posted to may answer with anything: the caller reads it.
        if post.is_some() {
            let url = dom::res_url(&response);
            return Ok(Page { url, html: None, response: Some(response), from: None, partial: None });
        }
        return Err(());
    }
    let url = dom::res_url(&response);
    let html = dom::res_text(&response).await.ok_or(())?;
    let from = header(&response, "x-nr-intercepted").map(|_| dom::pathname());
    Ok(Page { url, html: Some(html), response: None, from, partial: header(&response, "x-nr-partial") })
}

/// A header the source tests for truth: an empty one is no header at all.
fn header(response: &Node, name: &str) -> Option<String> {
    dom::res_header(response, name).filter(|value| !value.is_empty())
}

/// The page for `href`: the copy in memory if it is recent, otherwise one
/// request, shared by everyone asking for the same page at the same time.
async fn load_page(href: &str, fresh: bool, full: bool, signal: Option<&Node>) -> Result<Rc<Page>, ()> {
    let now = dom::now();
    enum Hit {
        Ready(Rc<Page>),
        Waiting(u32),
    }
    let hit = with_state(|state| {
        evict(&mut state.pages, now);
        if fresh {
            return None;
        }
        let entry = state.pages.get(href)?;
        if let Some(page) = &entry.page {
            return Some(Hit::Ready(page.clone()));
        }
        let flight = state.flights.get_mut(&entry.request)?;
        flight.readers += 1;
        Some(Hit::Waiting(entry.request))
    });
    match hit {
        Some(Hit::Ready(page)) => return Ok(page),
        Some(Hit::Waiting(request)) => return join_flight(request).await,
        None => {}
    }
    let request = with_state(|state| {
        state.next_flight = state.next_flight.wrapping_add(1).max(1);
        let request = state.next_flight;
        state.pages.insert(href.to_owned(), Entry { at: now, request, page: None });
        state.flights.insert(request, Flight::default());
        request
    });
    let result = request_page(href, full, None, signal).await.map(Rc::new);
    settle(request, href, result.clone());
    result
}

/// Wait for a request someone else started, as a second `await` on its promise.
async fn join_flight(request: u32) -> Result<Rc<Page>, ()> {
    let slot = task::pending();
    let token = slot.token();
    with_state(|state| {
        if let Some(flight) = state.flights.get_mut(&request) {
            flight.waiters.push(token);
        }
    });
    slot.await;
    with_state(|state| {
        let done = match state.flights.get_mut(&request) {
            Some(flight) => {
                flight.readers = flight.readers.saturating_sub(1);
                flight.done.clone()
            }
            None => None,
        };
        if state.flights.get(&request).is_some_and(|flight| flight.readers == 0) {
            state.flights.remove(&request);
        }
        done.unwrap_or(Err(()))
    })
}

/// A request finished: a failed one evicts the entry it made, the way the
/// source's `promise.catch` does, and everyone waiting is woken.
fn settle(request: u32, href: &str, result: Result<Rc<Page>, ()>) {
    let waiters = with_state(|state| {
        let mine = state.pages.get(href).is_some_and(|entry| entry.request == request);
        if mine {
            match &result {
                Ok(page) => {
                    if let Some(entry) = state.pages.get_mut(href) {
                        entry.page = Some(page.clone());
                    }
                }
                Err(()) => {
                    state.pages.remove(href);
                }
            }
        }
        let mut waiters = Vec::new();
        if let Some(flight) = state.flights.get_mut(&request) {
            flight.done = Some(result);
            waiters = core::mem::take(&mut flight.waiters);
            if flight.readers == 0 {
                state.flights.remove(&request);
            }
        }
        waiters
    });
    for token in waiters {
        task::resume(token, 0, 0, host::NULL_STR);
    }
}

/// Fetch a page before it is asked for, and forget how it went.
pub fn prefetch(href: &str) {
    let href = href.to_owned();
    task::spawn(async move {
        let _ = load_page(&href, false, false, None).await;
    });
}

/// Drop every page kept in memory: what a visitor posted, or an action ran,
/// may change what any of them would show now.
pub fn clear_pages() {
    with_state(|state| state.pages.clear());
}

// ---- rendering ----------------------------------------------------------------------

/// Put the page the server sent on screen. `target`: a selector
/// (`data-nr-target` on a form); when both the page and the response have that
/// element, only its content changes.
fn render(page: &Page, target: Option<&str>) {
    let doc = dom::parse_html(page.html.as_deref().unwrap_or(""));
    swap_streamed(&doc);
    let title = dom::title(&doc);
    dom::set_title(&title);
    let (Some(doc_head), Some(doc_body)) = (dom::prop_node(&doc, "head"), dom::prop_node(&doc, "body")) else {
        return;
    };
    let head = dom::head();
    // Streamed chunks put their styles in <body>; keep every style in <head>,
    // in order, so replacing the body never drops rules.
    for style in dom::query_all(&dom::body(), "style[data-nr-css]") {
        dom::append(&head, &style);
    }
    let mut existing: BTreeMap<String, Node> = BTreeMap::new();
    for style in dom::query_all(&head, "style[data-nr-css]") {
        existing.insert(dom::dataset(&style, "data-nr-css").unwrap_or_default(), style);
    }
    for node in dom::query_all(&head, MANAGED_HEAD) {
        dom::remove(&node);
    }
    let mut restyled = false;
    let mut incoming = dom::query_all(&doc_head, MANAGED);
    incoming.extend(dom::query_all(&doc_body, "style[data-nr-css]"));
    for node in incoming {
        let id = dom::dataset(&node, "data-nr-css").filter(|id| !id.is_empty());
        // Utility rules (`<sheet>~<rules>`) depend on their order: the newest
        // copy goes last.
        let utility = id.as_deref().is_some_and(|id| id.contains('~'));
        restyled = restyled || utility;
        match id.as_deref().and_then(|id| existing.get(id)) {
            Some(kept) => {
                if utility {
                    dom::append(&head, kept);
                }
            }
            None => {
                if let Some(copy) = dom::import(&node) {
                    dom::append(&head, &copy);
                }
            }
        }
        if !dom::parent(&node).is_some_and(|parent| dom::same(&parent, &doc_head)) {
            dom::remove(&node);
        }
    }
    // Pages fetched earlier were styled for the rules the browser had then.
    if restyled {
        clear_pages();
    }
    if let Some(env) = dom::by_id(&doc, "__next_rust_env") {
        let text = dom::text_content(&env);
        crate::api::set_env(if text.is_empty() { "{}" } else { &text });
    }
    // Inline delivery: a script runs once per document, like a module loaded
    // from a URL (classic page scripts ask to run again); module registrations
    // arrive in <head> and run from there, before the islands hydrate.
    for script in dom::query_all(&doc, "script[data-nr-js]") {
        let id = dom::dataset(&script, "data-nr-js").unwrap_or_default();
        let known = with_state(|state| state.inlined.contains(&id));
        if known && !dom::has_attr(&script, "data-nr-rerun") {
            dom::remove(&script);
            continue;
        }
        if !known {
            with_state(|state| state.inlined.push(id));
        }
        if !dom::parent(&script).is_some_and(|parent| dom::same(&parent, &doc_head)) {
            continue;
        }
        let copy = dom::create_element("script");
        for (name, value) in dom::attributes(&script) {
            dom::set_attr(&copy, &name, &value);
        }
        dom::set_text_content(&copy, &dom::text_content(&script));
        dom::append(&head, &copy);
    }
    let mut scripts: Vec<Node> = Vec::new();
    let region = page.partial.as_deref().filter(|key| !key.is_empty()).and_then(find_region);
    let target = target.filter(|selector| !selector.is_empty());
    let document = dom::document();
    let mine = target.and_then(|selector| dom::query(&document, selector));
    let theirs = target.filter(|_| mine.is_some()).and_then(|selector| dom::query(&doc, selector));
    if let (Some(mine), Some(theirs)) = (&mine, &theirs) {
        morph(mine, theirs);
    } else if let Some((start, end)) = region {
        // Keep every shared layout; replace only what is inside the deepest one.
        while let Some(next) = dom::next_sibling(&start) {
            if dom::same(&next, &end) {
                break;
            }
            dom::remove(&next);
        }
        let Some(parent) = dom::parent(&end) else { return };
        let mut added = Vec::new();
        for node in dom::child_nodes(&doc_body) {
            let element = dom::node_type(&node) == dom::ELEMENT;
            if element
                && (dom::prop_str(&node, "id").as_deref() == Some("__next_rust_env")
                    || dom::attr_or_empty(&node, "src").starts_with("/_next-rust/runtime.js"))
            {
                continue;
            }
            if let Some(copy) = dom::import(&node) {
                dom::insert_before(&parent, &copy, Some(&end));
                added.push(copy);
            }
        }
        for node in added {
            if dom::node_type(&node) != dom::ELEMENT {
                continue;
            }
            if dom::node_name(&node) == "SCRIPT" {
                scripts.push(node);
            } else {
                scripts.extend(dom::query_all(&node, "script"));
            }
        }
    } else {
        if let Some(copy) = dom::import(&doc_body) {
            dom::replace_with(&dom::body(), &copy);
        }
        scripts = dom::query_all(&dom::body(), "script");
    }
    // A script the browser has already seen never runs again on its own: it is
    // replaced by a copy of itself, which is a script the document has not run.
    for old in scripts {
        let src = dom::attr_or_empty(&old, "src");
        if dom::prop_str(&old, "type").as_deref() == Some("application/json")
            || src.starts_with("/_next-rust/runtime.js")
            || dom::text_content(&old).starts_with("$nr(")
        {
            continue;
        }
        let copy = dom::create_element("script");
        for (name, value) in dom::attributes(&old) {
            dom::set_attr(&copy, &name, &value);
        }
        dom::set_text_content(&copy, &dom::text_content(&old));
        dom::replace_with(&old, &copy);
    }
    hydrate_after_render();
}

/// `render` hydrates the islands it put on screen.
pub fn hydrate_after_render() {
    crate::islands::hydrate_document();
}

// ---- morphing -------------------------------------------------------------------------
// Make the children of `from` those of `to`, changing what differs and keeping
// what does not: an element that stays keeps its focus, caret, scroll position
// and listeners. Elements pair up by tag, `id` and `data-nr-key`.

fn alike(a: &Node, b: &Node) -> bool {
    let kind = dom::node_type(a);
    kind == dom::node_type(b)
        && dom::node_name(a) == dom::node_name(b)
        && (kind != dom::ELEMENT
            || (dom::prop_str(a, "id").unwrap_or_default() == dom::prop_str(b, "id").unwrap_or_default()
                && dom::attr(a, "data-nr-key") == dom::attr(b, "data-nr-key")))
}

fn morph(from: &Node, to: &Node) {
    let mut old = dom::first_child(from);
    for next in dom::child_nodes(to) {
        let mut hit = old.as_ref().map(Node::share);
        while let Some(candidate) = &hit {
            if alike(candidate, &next) {
                break;
            }
            hit = dom::next_sibling(candidate);
        }
        let Some(hit) = hit else {
            if let Some(copy) = dom::import(&next) {
                dom::insert_before(from, &copy, old.as_ref());
            }
            continue;
        };
        // Everything the answer no longer has, between here and the pair.
        while let Some(gone) = old.take() {
            if dom::same(&gone, &hit) {
                break;
            }
            old = dom::next_sibling(&gone);
            dom::remove(&gone);
        }
        old = dom::next_sibling(&hit);
        if dom::node_type(&hit) != dom::ELEMENT {
            let text = dom::data(&next);
            if dom::data(&hit) != text {
                dom::set_data(&hit, &text);
            }
        } else if dom::node_name(&hit) == "NR-ISLAND" {
            // A hydrated island no longer looks like its markup: new props, new island.
            if dom::dataset(&hit, "data-props") != dom::dataset(&next, "data-props")
                && let Some(copy) = dom::import(&next)
            {
                dom::replace_with(&hit, &copy);
            }
        } else {
            for (name, _) in dom::attributes(&hit) {
                if !dom::has_attr(&next, &name) {
                    dom::remove_attr(&hit, &name);
                }
            }
            for (name, value) in dom::attributes(&next) {
                if dom::attr(&hit, &name).as_deref() != Some(value.as_str()) {
                    dom::set_attr(&hit, &name, &value);
                }
            }
            morph(&hit, &next);
            // What the visitor is typing into stays as typed.
            if !dom::active_element().is_some_and(|active| dom::same(&active, &hit)) {
                for property in ["value", "checked", "selected"] {
                    if !dom::has_prop(&hit, property) {
                        continue;
                    }
                    if dom::prop_is_string(&hit, property) {
                        let value = dom::prop_str(&next, property);
                        if dom::prop_str(&hit, property) != value {
                            dom::set_prop_str(&hit, property, &value.unwrap_or_default());
                        }
                    } else {
                        let value = dom::prop_bool(&next, property);
                        if dom::prop_bool(&hit, property) != value {
                            dom::set_prop_bool(&hit, property, value);
                        }
                    }
                }
            }
        }
    }
    while let Some(gone) = old {
        old = dom::next_sibling(&gone);
        dom::remove(&gone);
    }
}

// ---- focus -------------------------------------------------------------------------------

/// The field being typed into, found again after the page around it was
/// replaced (same `id`, or same `name` in the same position): focus, the text
/// as typed and the caret all carry over.
struct Refocus {
    element: Node,
    id: String,
    name: String,
    value: Option<String>,
    start: Option<f64>,
    end: Option<f64>,
    direction: String,
    at: usize,
}

fn keep_focus() -> Option<Refocus> {
    let element = dom::active_element()?;
    let id = dom::prop_str(&element, "id").unwrap_or_default();
    let name = dom::prop_str(&element, "name").unwrap_or_default();
    if id.is_empty() && name.is_empty() {
        return None;
    }
    let at = if name.is_empty() {
        0
    } else {
        // `indexOf` says -1 when the field is gone; the first one then stands in.
        dom::by_name(&name).iter().position(|node| dom::same(node, &element)).unwrap_or(usize::MAX)
    };
    Some(Refocus {
        value: dom::prop_str(&element, "value"),
        start: dom::prop_number(&element, "selectionStart"),
        end: dom::prop_number(&element, "selectionEnd"),
        direction: dom::prop_str(&element, "selectionDirection").unwrap_or_default(),
        element,
        id,
        name,
        at,
    })
}

impl Refocus {
    fn apply(&self) {
        if dom::is_connected(&self.element) {
            if !dom::active_element().is_some_and(|active| dom::same(&active, &self.element)) {
                dom::focus(&self.element);
            }
            return;
        }
        let named = if self.name.is_empty() { Vec::new() } else { dom::by_name(&self.name) };
        let found = (!self.id.is_empty())
            .then(|| dom::by_id(&dom::document(), &self.id))
            .flatten()
            .or_else(|| named.get(self.at).map(Node::share))
            .or_else(|| named.first().map(Node::share));
        let Some(found) = found else { return };
        dom::focus(&found);
        let Some(start) = self.start else { return };
        if !dom::prop_is_string(&found, "value") {
            return;
        }
        if let Some(value) = &self.value {
            dom::set_prop_str(&found, "value", value);
        }
        let end = self.end.unwrap_or(start);
        dom::set_selection_range(&found, start.max(0.0) as u32, end.max(0.0) as u32, &self.direction);
    }
}

// ---- active links ---------------------------------------------------------------------------

/// Links marked with active_class / active_class_prefix follow the current URL.
pub fn mark_active_links() {
    let here = dom::pathname();
    let document = dom::document();
    for link in dom::query_all(&document, "a[data-nr-active],a[data-nr-active-prefix]") {
        let href = dom::prop_str(&link, "href").unwrap_or_default();
        let Some(url) = dom::Url::here(&href) else { continue };
        let (exact, below) = active_state(&here, &url.pathname());
        let mut current = false;
        for (attribute, on) in [("data-nr-active", exact), ("data-nr-active-prefix", below)] {
            let Some(class) = dom::dataset(&link, attribute) else { continue };
            if !class.is_empty() {
                dom::toggle_class(&link, &class, on);
            }
            current = current || on;
        }
        if current {
            dom::set_attr(&link, "aria-current", "page");
        } else if dom::attr(&link, "aria-current").as_deref() == Some("page") {
            dom::remove_attr(&link, "aria-current");
        }
    }
}

// ---- page fades ---------------------------------------------------------------------------------
// The motion stylesheet ([animation] in next-rust.toml) sets --nr-page-fade on
// <html>: how long one page takes to fade into the next, 0 when off or when
// the visitor asked for reduced motion.

fn page_fade() -> f64 {
    parse_float(&dom::computed(&dom::root(), "", "--nr-page-fade"))
}

/// Run `swap` (which replaces the page) as a fade: a view transition where the
/// browser has them, otherwise the body fades out, swaps and fades back in.
/// `false` is the swap having thrown, which the caller answers with a full load.
async fn fade(swap: Box<dyn FnOnce() -> bool>) -> bool {
    let ms = page_fade();
    if ms == 0.0 {
        return swap();
    }
    if dom::view_transitions() {
        let done = Rc::new(Cell::new(false));
        let held = Rc::new(RefCell::new(Some(swap)));
        let (ran, taken) = (done.clone(), held.clone());
        let slot = task::pending();
        let callback = task::on(move |_| {
            if let Some(swap) = taken.borrow_mut().take() {
                ran.set(swap());
            }
        });
        dom::start_view_transition(callback, slot.token());
        // The next navigation may cut this one short: the page is swapped all the same.
        slot.await;
        task::forget(callback);
        return done.get();
    }
    let root = dom::root();
    dom::set_attr(&root, "data-nr-fade", "out");
    task::pause(ms / 2.0).await;
    let ok = swap();
    dom::set_attr(&root, "data-nr-fade", "in");
    task::spawn(async move {
        task::pause(ms / 2.0).await;
        let root = dom::root();
        if dom::attr(&root, "data-nr-fade").as_deref() == Some("in") {
            dom::remove_attr(&root, "data-nr-fade");
        }
    });
    ok
}

// ---- navigation -------------------------------------------------------------------------------------

/// A navigation that did not finish: `unsent` is a form that never reached the
/// server (the source rethrows, and the browser sends it itself), anything
/// else is answered with a full page load of `url`.
struct Fail {
    unsent: bool,
    url: Option<String>,
}

impl Fail {
    fn unsent() -> Self {
        Self { unsent: true, url: None }
    }

    fn reload(url: Option<String>) -> Self {
        Self { unsent: false, url }
    }
}

/// Whether the caller dropped this navigation while it was in the air.
fn dropped(signal: Option<&Node>) -> bool {
    signal.is_some_and(dom::signal_aborted)
}

/// A form whose post never reached the server: the one failure `navigate`
/// hands back rather than answering with a full page load, because the form
/// that sent it has to submit it the browser's way instead.
pub struct Unsent;

/// Go to `href` without a page load.
///
/// `Ok(Some(response))` is the answer to a post that was not a page, which the
/// form that sent it reads. `Err(())` is the source's one `throw`: a form whose
/// post never reached the server, which `sendForm` answers with a native
/// submit. Every other failure has already fallen back to a full page load by
/// the time this returns.
pub async fn navigate(href: &str, opts: NavOptions) -> Result<Option<Node>, Unsent> {
    let Some(url) = dom::Url::here(href) else { return Ok(None) };
    if url.origin() != dom::origin() {
        core::mem::forget(crate::loader::start(Some(FULL_LOAD_LIMIT), None));
        dom::assign(&url.href());
        return Ok(None);
    }
    let hash = url.hash();
    if opts.history && !opts.fresh && url.page() == current() && !hash.is_empty() {
        dom::set_hash(&hash);
        return Ok(None);
    }
    let root = dom::root();
    let signal = opts.signal.as_ref().map(Node::share);
    with_state(|state| {
        state.navigating += 1;
    });
    dom::set_attr(&root, "data-nr-navigating", "");
    let loaded = crate::loader::start(None, None);
    let outcome = attempt(&url, &hash, &opts, signal.as_ref()).await;
    let last = with_state(|state| {
        state.navigating = state.navigating.saturating_sub(1);
        state.navigating == 0
    });
    if last {
        dom::remove_attr(&root, "data-nr-navigating");
    }
    loaded.end();
    match outcome {
        Ok(response) => Ok(response),
        Err(fail) => {
            // A live search drops the request of the keystroke before: nothing to show.
            if dropped(signal.as_ref()) {
                return Ok(None);
            }
            // A form that never reached the server is the browser's to send.
            if fail.unsent {
                return Err(Unsent);
            }
            dom::assign(&fail.url.unwrap_or_else(|| url.href()));
            Ok(None)
        }
    }
}

async fn attempt(url: &dom::Url, hash: &str, opts: &NavOptions, signal: Option<&Node>) -> Result<Option<Node>, Fail> {
    let href = url.href();
    let mut page: Rc<Page>;
    if let Some(post) = &opts.post {
        let mut answer =
            request_page(&href, false, Some(post), post.signal.as_ref()).await.map_err(|()| Fail::unsent())?;
        // What was posted may change what pages show: drop every page kept in memory.
        clear_pages();
        if let Some(response) = answer.response.take() {
            // Not a page (JSON, a file, nothing): the form shows it, unless the
            // server asked for a full page load of where the form led.
            if header(&response, "x-nr-reload").is_some() {
                return Err(Fail::reload(Some(answer.url)));
            }
            return Ok(Some(response));
        }
        page = Rc::new(answer);
    } else {
        page = match load_page(&href, opts.fresh, false, signal).await {
            Ok(page) => page,
            Err(()) => {
                if dropped(signal) {
                    return Ok(None);
                }
                // A failed prefetch gets one more try.
                load_page(&href, true, false, signal).await.map_err(|()| Fail::reload(None))?
            }
        };
        if page.from.as_deref().is_some_and(|from| from != dom::pathname()) {
            let last = page.url.clone();
            page = load_page(&href, true, false, signal).await.map_err(|()| Fail::reload(Some(last)))?;
        }
    }
    // A partial page needs its layout on screen; if it is gone, get the full page.
    if page.partial.as_deref().is_some_and(|key| !key.is_empty() && find_region(key).is_none()) {
        let last = page.url.clone();
        let from = if last.is_empty() { href.clone() } else { last.clone() };
        page = load_page(&from, true, true, signal).await.map_err(|()| Fail::reload(Some(last)))?;
    }
    if dropped(signal) {
        return Ok(None);
    }
    let fallback = if page.url.is_empty() { href.clone() } else { page.url.clone() };
    let Some(final_url) = dom::Url::parse(&fallback, None) else {
        return Err(Fail::reload(Some(fallback)));
    };
    if final_url.hash().is_empty() {
        final_url.set_hash(hash);
    }
    let final_href = final_url.href();
    let final_page = final_url.page();
    // A live search and a form that updates one element change the page in
    // place: no fade, and the page stays where it is scrolled.
    let document = dom::document();
    let quiet = opts.quiet
        || opts
            .target
            .as_deref()
            .filter(|selector| !selector.is_empty())
            .is_some_and(|selector| dom::query(&document, selector).is_some());
    let rendered = page.clone();
    let target = opts.target.clone();
    let link = opts.link.as_ref().map(Node::share);
    let (focus, history, replace, scroll) = (opts.focus, opts.history, opts.replace, opts.scroll);
    let (swap_href, swap_page, swap_hash) = (final_href.clone(), final_page, hash.to_owned());
    let swap: Box<dyn FnOnce() -> bool> = Box::new(move || {
        let refocus = if focus { keep_focus() } else { None };
        render(&rendered, target.as_deref());
        // The same URL again (a search repeated) takes no new history entry.
        if history {
            dom::push_state(&swap_href, replace || swap_href == dom::href());
        }
        if let Some(refocus) = refocus {
            refocus.apply();
        }
        with_state(|state| state.current = swap_page);
        mark_active_links();
        // A link inside a menu that stays on screen (an open <details>) closes it.
        let mut open =
            link.as_ref().filter(|link| dom::is_connected(link)).and_then(|link| dom::closest(link, "details[open]"));
        while let Some(details) = open {
            dom::set_prop_bool(&details, "open", false);
            open = dom::prop_node(&details, "parentElement").and_then(|parent| dom::closest(&parent, "details[open]"));
        }
        if scroll && !quiet {
            let anchor = match swap_hash.get(1..).filter(|id| !id.is_empty()) {
                Some(id) => match decode_uri_component(id) {
                    Some(id) => dom::by_id(&dom::document(), &id),
                    None => return false,
                },
                None => None,
            };
            match anchor {
                Some(anchor) => dom::scroll_into_view(&anchor),
                None => dom::scroll_to_top(),
            }
        }
        true
    });
    let swapped = if quiet { swap() } else { fade(swap).await };
    if !swapped {
        return Err(Fail::reload(Some(fallback)));
    }
    let mut detail = String::from("{\"url\":");
    detail.push_str(&json_string(&final_href));
    detail.push('}');
    dom::dispatch(None, dom::CUSTOM_EVENT, "nr:navigate", &detail, 0);
    Ok(None)
}

// ---- links ------------------------------------------------------------------------------------------

/// Every same-origin link navigates without a full page load, whether it was
/// written with Link! or as a plain a![href(..)]. The browser handles a link
/// normally when it opens elsewhere (target, download, rel="external"), points
/// to another origin or protocol, or opts out with data-nr-reload.
fn internal_link(target: &Node) -> Option<Node> {
    let link = dom::closest(target, "a[href]")?;
    if !dom::prop_is_string(&link, "href") {
        return None;
    }
    if dom::has_attr(&link, "data-nr-reload") || dom::has_attr(&link, "download") {
        return None;
    }
    let opens = dom::prop_str(&link, "target").unwrap_or_default();
    if !opens.is_empty() && opens != "_self" {
        return None;
    }
    if dom::attr_or_empty(&link, "rel").split_whitespace().any(|rel| rel == "external") {
        return None;
    }
    let url = dom::Url::here(&dom::prop_str(&link, "href").unwrap_or_default())?;
    if url.origin() != dom::origin() || !web_protocol(&url.protocol()) {
        return None;
    }
    Some(link)
}

/// `/^https?:$/`.
fn web_protocol(protocol: &str) -> bool {
    protocol == "http:" || protocol == "https:"
}

/// A link the browser follows itself (another origin, data-nr-reload): the bar
/// runs until the new page takes over, or gives up after a while.
fn full_load(event: &Node) {
    let Some(link) = dom::prop_node(event, "target").and_then(|target| dom::closest(&target, "a[href]")) else {
        return;
    };
    if !dom::prop_is_string(&link, "href") || dom::has_attr(&link, "download") {
        return;
    }
    let opens = dom::prop_str(&link, "target").unwrap_or_default();
    if !opens.is_empty() && opens != "_self" {
        return;
    }
    let Some(url) = dom::Url::here(&dom::prop_str(&link, "href").unwrap_or_default()) else { return };
    if !web_protocol(&url.protocol()) {
        return;
    }
    if !url.hash().is_empty() && url.origin() == dom::origin() && url.page() == current() {
        return;
    }
    // Once every listener has had its say: a click someone cancels loads nothing.
    let event = event.share();
    task::spawn(async move {
        task::pause(0.0).await;
        if !dom::prop_bool(&event, "defaultPrevented") {
            core::mem::forget(crate::loader::start(Some(FULL_LOAD_LIMIT), None));
        }
    });
}

// ---- prefetching on intent -----------------------------------------------------------------------------
// Prefetch only when the mouse rests on a link for PREFETCH_DELAY ms. Moving
// or scrolling past links, pressing a button and touch taps never fetch early;
// those pages load on click.

fn cancel_intent() {
    with_state(|state| {
        state.hover_timer = None;
        state.hover_link = None;
    });
}

fn intent(event: Option<Node>) {
    let Some(event) = event else { return };
    if dom::prop_str(&event, "pointerType").as_deref() != Some("mouse") {
        return;
    }
    let link = dom::prop_node(&event, "target").as_ref().and_then(internal_link);
    let unchanged = with_state(|state| match (&state.hover_link, &link) {
        (Some(hovered), Some(link)) => dom::same(hovered, link),
        (None, None) => true,
        _ => false,
    });
    if unchanged {
        return;
    }
    cancel_intent();
    let Some(link) = link else { return };
    if dom::dataset(&link, "data-nr-prefetch").as_deref() == Some("false") {
        return;
    }
    let href = dom::prop_str(&link, "href").unwrap_or_default();
    let Some(url) = dom::Url::parse(&href, None) else { return };
    // Same-page anchors and the page that is already open need no fetch.
    if !url.hash().is_empty() || url.page() == current() {
        return;
    }
    let watched = link.share();
    let timer = Timer::after(PREFETCH_DELAY, move || {
        let still = with_state(|state| {
            state.hover_timer = None;
            state.hover_link.as_ref().is_some_and(|hovered| dom::same(hovered, &watched))
        });
        if still && dom::is_connected(&watched) {
            prefetch(&dom::prop_str(&watched, "href").unwrap_or_default());
        }
    });
    with_state(|state| {
        state.hover_link = Some(link);
        state.hover_timer = Some(timer);
    });
}

// ---- server actions -----------------------------------------------------------------------------------

/// With `[security] csrf = "token"` the server also expects the `next_rust_csrf`
/// cookie echoed in a header.
pub fn with_csrf(headers: &mut Vec<(String, String)>) {
    let cookie = dom::cookie();
    if let Some(token) = csrf_token(&cookie) {
        set_header(headers, "x-csrf-token", token);
    }
}

/// The headers of a server action, with `extra` overriding them the way the
/// source's object spread does.
pub fn action_headers(extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    set_header(&mut headers, "x-nr-action", "1");
    set_header(&mut headers, "accept", "application/json");
    for (name, value) in extra {
        set_header(&mut headers, name, value);
    }
    with_csrf(&mut headers);
    headers
}

// ---- listeners ------------------------------------------------------------------------------------------

/// Where the page is, the ids of the scripts it has run, and every listener
/// this module needs. The shim calls it once, through `start`.
pub fn install() {
    let document = dom::document();
    let inlined = dom::query_all(&document, "script[data-nr-js]")
        .iter()
        .map(|script| dom::dataset(script, "data-nr-js").unwrap_or_default())
        .collect();
    with_state(|state| {
        state.current = dom::here();
        state.inlined = inlined;
    });

    let click = task::on(|event| {
        let Some(event) = event else { return };
        if dom::prop_bool(&event, "defaultPrevented")
            || dom::prop_num(&event, "button") != 0.0
            || dom::prop_bool(&event, "metaKey")
            || dom::prop_bool(&event, "ctrlKey")
            || dom::prop_bool(&event, "shiftKey")
            || dom::prop_bool(&event, "altKey")
        {
            return;
        }
        let Some(link) = dom::prop_node(&event, "target").as_ref().and_then(internal_link) else {
            full_load(&event);
            return;
        };
        dom::prevent_default(&event);
        let href = dom::prop_str(&link, "href").unwrap_or_default();
        let opts = NavOptions {
            replace: dom::has_attr(&link, "data-nr-replace"),
            scroll: dom::dataset(&link, "data-nr-scroll").as_deref() != Some("false"),
            link: Some(link),
            ..NavOptions::default()
        };
        task::spawn(async move {
            // The source never looks at what a click's navigation returns.
            let _ = navigate(&href, opts).await;
        });
    });
    dom::add_listener(Some(&document), "click", click, 0);

    let hover = task::on(intent);
    dom::add_listener(Some(&document), "pointerover", hover, dom::PASSIVE);
    dom::add_listener(Some(&document), "pointermove", hover, dom::PASSIVE);
    let away = task::on(|event| {
        let related = event.as_ref().and_then(|event| dom::prop_node(event, "relatedTarget"));
        let leaving = with_state(|state| {
            state.hover_link.as_ref().is_some_and(|hovered| !dom::contains(hovered, related.as_ref()))
        });
        if leaving {
            cancel_intent();
        }
    });
    dom::add_listener(Some(&document), "pointerout", away, dom::PASSIVE);
    let cancel = task::on(|_| cancel_intent());
    dom::add_listener(Some(&document), "pointerdown", cancel, dom::PASSIVE);
    dom::add_listener(Some(&document), "scroll", cancel, dom::PASSIVE | dom::CAPTURE);

    let popstate = task::on(|_| {
        if dom::here() != current() {
            task::spawn(async {
                let opts = NavOptions { history: false, scroll: false, ..NavOptions::default() };
                let _ = navigate(&dom::href(), opts).await;
            });
        }
    });
    dom::add_listener(None, "popstate", popstate, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_kept_for_thirty_seconds_and_no_longer() {
        let mut pages = BTreeMap::new();
        for (key, at) in [("/fresh", 29_999.0), ("/exactly", 30_000.0), ("/stale", 30_001.0)] {
            pages.insert(key.to_owned(), Entry { at: 100_000.0 - at, request: 1, page: None });
        }
        evict(&mut pages, 100_000.0);
        assert!(pages.contains_key("/fresh"));
        // The source deletes at `>= PAGE_TTL`, so the exact age is already gone.
        assert!(!pages.contains_key("/exactly"));
        assert!(!pages.contains_key("/stale"));
    }

    #[test]
    fn a_link_is_active_for_its_own_path_and_below_it() {
        assert_eq!(active_state("/docs", "/docs"), (true, true));
        assert_eq!(active_state("/docs/", "/docs"), (true, true));
        assert_eq!(active_state("/docs", "/docs///"), (true, true));
        assert_eq!(active_state("/docs/guide", "/docs"), (false, true));
        assert_eq!(active_state("/docs/guide", "/docs/guide/deeper"), (false, false));
        // A link to "/" is above everything but is only itself exactly.
        assert_eq!(active_state("/docs", "/"), (false, true));
        assert_eq!(active_state("/", "/"), (true, true));
        // "/docs-and-more" is not below "/docs".
        assert_eq!(active_state("/docs-and-more", "/docs"), (false, false));
    }

    #[test]
    fn the_root_path_keeps_its_slash() {
        assert_eq!(trim_slashes("/"), "/");
        assert_eq!(trim_slashes("/a/"), "/a");
        assert_eq!(trim_slashes(""), "");
    }

    #[test]
    fn a_navigation_says_where_it_is_and_what_it_already_has() {
        let layouts = ["root".to_owned(), "docs".to_owned()];
        let styles = ["app".to_owned()];
        let scripts = ["a1".to_owned()];
        let headers = nav_headers("/docs", None, &layouts, &styles, &scripts);
        assert_eq!(
            headers,
            [
                ("x-nr-nav".to_owned(), "1".to_owned()),
                ("x-nr-from".to_owned(), "/docs".to_owned()),
                ("accept".to_owned(), "text/html".to_owned()),
                ("x-nr-layouts".to_owned(), "root,docs".to_owned()),
                ("x-nr-styles".to_owned(), "app".to_owned()),
                ("x-nr-scripts".to_owned(), "a1".to_owned()),
            ]
        );
    }

    #[test]
    fn nothing_the_page_does_not_have_is_sent() {
        let headers = nav_headers("/", None, &[], &[], &[]);
        assert_eq!(headers.len(), 3);
        assert!(headers.iter().all(|(name, _)| !name.starts_with("x-nr-l")));
    }

    #[test]
    fn a_posted_page_echoes_the_csrf_cookie() {
        let headers = nav_headers("/", Some("abc123"), &[], &[], &[]);
        assert_eq!(headers[3], ("x-csrf-token".to_owned(), "abc123".to_owned()));
    }

    #[test]
    fn the_csrf_cookie_is_read_whole_or_not_at_all() {
        assert_eq!(csrf_token("next_rust_csrf=abc"), Some("abc"));
        assert_eq!(csrf_token("a=1; next_rust_csrf=abc; b=2"), Some("abc"));
        assert_eq!(csrf_token("a=1;next_rust_csrf=abc"), Some("abc"));
        // A cookie whose name merely ends in ours is a different cookie.
        assert_eq!(csrf_token("my_next_rust_csrf=abc"), None);
        assert_eq!(csrf_token("a=1; my_next_rust_csrf=abc; next_rust_csrf=real"), Some("real"));
        assert_eq!(csrf_token("next_rust_csrf=; a=1"), None);
        assert_eq!(csrf_token(""), None);
    }

    #[test]
    fn a_header_of_the_same_name_is_replaced_not_repeated() {
        let mut headers = Vec::new();
        set_header(&mut headers, "accept", "text/html");
        set_header(&mut headers, "accept", "application/json");
        assert_eq!(headers, [("accept".to_owned(), "application/json".to_owned())]);
    }

    #[test]
    fn an_anchor_is_decoded_the_way_the_browser_decodes_it() {
        assert_eq!(decode_uri_component("plain").as_deref(), Some("plain"));
        assert_eq!(decode_uri_component("caf%C3%A9").as_deref(), Some("café"));
        assert_eq!(decode_uri_component("%zz"), None);
        assert_eq!(decode_uri_component("%C3"), None);
    }

    #[test]
    fn a_fade_that_is_not_a_number_is_no_fade() {
        assert_eq!(parse_float("300"), 300.0);
        assert_eq!(parse_float("0.25s"), 0.25);
        assert_eq!(parse_float(" -1.5e2px"), -150.0);
        assert_eq!(parse_float(""), 0.0);
        assert_eq!(parse_float("none"), 0.0);
    }

    #[test]
    fn an_event_detail_is_json() {
        assert_eq!(json_string("/a\"b"), "\"/a\\\"b\"");
    }
}
