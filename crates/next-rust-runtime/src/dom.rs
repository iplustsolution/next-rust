//! Safe wrappers over the host ABI.
//!
//! Every function here is one or two [`host`] calls with a Rust signature:
//! `Option<Node>` rather than a handle that might be `0`, `Option<String>`
//! rather than a length that might be [`host::NULL_STR`]. Nothing in this
//! module decides anything — the behaviour lives in the modules above it.
//!
//! [`Node`] owns its handle and gives it back when it is dropped, so a
//! navigation that walks a parsed document leaves no handles behind in the
//! shim's table.

#![allow(dead_code)] // the full DOM surface; each module uses what it needs

use alloc::string::String;
use alloc::vec::Vec;

use crate::host::{self, H, op, part};

/// `Node.ELEMENT_NODE`.
pub const ELEMENT: u32 = 1;
/// `Node.TEXT_NODE`.
pub const TEXT: u32 = 3;
/// `Node.COMMENT_NODE`.
pub const COMMENT: u32 = 8;

/// A handle the module owns. Dropping it releases it on the shim's side.
///
/// No `Debug`: deriving it would pull `core::fmt` into the module for the
/// sake of a trait nothing in the browser can print.
pub struct Node(H);

impl Node {
    /// Adopt a handle a call returned; `0` becomes `None`.
    #[inline]
    pub fn new(handle: H) -> Option<Self> {
        (handle != 0).then_some(Self(handle))
    }

    /// The raw handle, for passing back to the ABI.
    #[inline]
    pub fn raw(&self) -> H {
        self.0
    }

    /// A second owner of the same handle. The shim counts references, so the
    /// handle survives until both are dropped.
    #[inline]
    pub fn share(&self) -> Self {
        Self(host::call(op::RETAIN, self.0, 0, 0, 0, 0))
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        host::release(self.0);
    }
}

/// The handle of an optional node, or `0`.
#[inline]
fn raw(node: Option<&Node>) -> H {
    node.map_or(0, Node::raw)
}

// ---- documents -------------------------------------------------------------

pub fn document() -> Node {
    Node(host::call(op::DOCUMENT, 0, 0, 0, 0, 0))
}

pub fn body() -> Node {
    Node(host::call(op::BODY, 0, 0, 0, 0, 0))
}

pub fn head() -> Node {
    Node(host::call(op::HEAD, 0, 0, 0, 0, 0))
}

/// `<html>`, which carries `data-nr-loading`, `data-nr-fade` and `--nr-load`.
pub fn root() -> Node {
    Node(host::call(op::DOC_ELEMENT, 0, 0, 0, 0, 0))
}

pub fn active_element() -> Option<Node> {
    Node::new(host::call(op::ACTIVE_ELEMENT, 0, 0, 0, 0, 0))
}

/// `new DOMParser().parseFromString(html, "text/html")`.
pub fn parse_html(html: &str) -> Node {
    let (p, l) = host::s(html);
    Node(host::call(op::PARSE_HTML, p, l, 0, 0, 0))
}

pub fn title(doc: &Node) -> String {
    host::text(op::TITLE, doc.raw(), 0, 0, 0, 0)
}

pub fn set_title(value: &str) {
    let (p, l) = host::s(value);
    host::call(op::SET_TITLE, p, l, 0, 0, 0);
}

// ---- traversal --------------------------------------------------------------

pub fn node_type(node: &Node) -> u32 {
    host::call(op::NODE_TYPE, node.raw(), 0, 0, 0, 0)
}

pub fn node_name(node: &Node) -> String {
    host::text(op::NODE_NAME, node.raw(), 0, 0, 0, 0)
}

pub fn is_connected(node: &Node) -> bool {
    host::flag(op::IS_CONNECTED, node.raw(), 0, 0, 0, 0)
}

pub fn parent(node: &Node) -> Option<Node> {
    Node::new(host::call(op::PARENT, node.raw(), 0, 0, 0, 0))
}

pub fn first_child(node: &Node) -> Option<Node> {
    Node::new(host::call(op::FIRST_CHILD, node.raw(), 0, 0, 0, 0))
}

pub fn next_sibling(node: &Node) -> Option<Node> {
    Node::new(host::call(op::NEXT_SIBLING, node.raw(), 0, 0, 0, 0))
}

pub fn previous_sibling(node: &Node) -> Option<Node> {
    Node::new(host::call(op::PREV_SIBLING, node.raw(), 0, 0, 0, 0))
}

/// A snapshot of `node.childNodes`, as `[...node.childNodes]` in the source:
/// the caller moves nodes around while it walks them.
pub fn child_nodes(node: &Node) -> Vec<Node> {
    collect(host::call(op::CHILD_NODES, node.raw(), 0, 0, 0, 0))
}

pub fn query(root: &Node, selector: &str) -> Option<Node> {
    let (p, l) = host::s(selector);
    Node::new(host::call(op::QUERY, root.raw(), p, l, 0, 0))
}

pub fn query_all(root: &Node, selector: &str) -> Vec<Node> {
    let (p, l) = host::s(selector);
    collect(host::call(op::QUERY_ALL, root.raw(), p, l, 0, 0))
}

pub fn by_id(doc: &Node, id: &str) -> Option<Node> {
    let (p, l) = host::s(id);
    Node::new(host::call(op::GET_BY_ID, doc.raw(), p, l, 0, 0))
}

pub fn by_name(name: &str) -> Vec<Node> {
    let (p, l) = host::s(name);
    collect(host::call(op::GET_BY_NAME, p, l, 0, 0, 0))
}

pub fn closest(node: &Node, selector: &str) -> Option<Node> {
    let (p, l) = host::s(selector);
    Node::new(host::call(op::CLOSEST, node.raw(), p, l, 0, 0))
}

pub fn contains(node: &Node, other: Option<&Node>) -> bool {
    host::flag(op::CONTAINS, node.raw(), raw(other), 0, 0, 0)
}

/// Every node of a list handle, which is released once it is drained.
fn collect(list: H) -> Vec<Node> {
    if list == 0 {
        return Vec::new();
    }
    let len = host::call(op::LIST_LEN, list, 0, 0, 0, 0);
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        if let Some(node) = Node::new(host::call(op::LIST_ITEM, list, i, 0, 0, 0)) {
            out.push(node);
        }
    }
    host::release(list);
    out
}

/// Every string of a list handle, which is released once it is drained.
fn collect_strings(list: H) -> Vec<String> {
    if list == 0 {
        return Vec::new();
    }
    let len = host::call(op::LIST_LEN, list, 0, 0, 0, 0);
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        out.push(host::text(op::LIST_STR, list, i, 0, 0, 0));
    }
    host::release(list);
    out
}

/// A `TreeWalker` over the comment nodes below `root`: how the layout markers
/// (`<!--nr-l:KEY-->`) are found.
pub struct Comments(H);

impl Comments {
    pub fn new(root: &Node) -> Self {
        Self(host::call(op::WALKER_NEW, root.raw(), host::SHOW_COMMENT, 0, 0, 0))
    }
}

impl Iterator for Comments {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        Node::new(host::call(op::WALKER_NEXT, self.0, 0, 0, 0, 0))
    }
}

impl Drop for Comments {
    fn drop(&mut self) {
        host::release(self.0);
    }
}

/// `CharacterData.data` — the text of a comment or text node.
pub fn data(node: &Node) -> String {
    host::text(op::DATA, node.raw(), 0, 0, 0, 0)
}

pub fn set_data(node: &Node, value: &str) {
    let (p, l) = host::s(value);
    host::call(op::SET_DATA, node.raw(), p, l, 0, 0);
}

// ---- mutation ----------------------------------------------------------------

pub fn create_element(tag: &str) -> Node {
    let (p, l) = host::s(tag);
    Node(host::call(op::CREATE_ELEMENT, p, l, 0, 0, 0))
}

/// `document.importNode(node, true)` into the live document.
pub fn import(node: &Node) -> Option<Node> {
    Node::new(host::call(op::IMPORT_NODE, 0, node.raw(), 1, 0, 0))
}

pub fn append(parent: &Node, child: &Node) {
    host::call(op::APPEND, parent.raw(), child.raw(), 0, 0, 0);
}

pub fn insert_before(parent: &Node, child: &Node, before: Option<&Node>) {
    host::call(op::INSERT_BEFORE, parent.raw(), child.raw(), raw(before), 0, 0);
}

pub fn replace_with(old: &Node, new: &Node) {
    host::call(op::REPLACE_WITH, old.raw(), new.raw(), 0, 0, 0);
}

pub fn remove(node: &Node) {
    host::call(op::REMOVE, node.raw(), 0, 0, 0, 0);
}

/// `<template>.content`.
pub fn template_content(node: &Node) -> Option<Node> {
    Node::new(host::call(op::TEMPLATE_CONTENT, node.raw(), 0, 0, 0, 0))
}

// ---- attributes and properties -------------------------------------------------

pub fn attr(node: &Node, name: &str) -> Option<String> {
    let (p, l) = host::s(name);
    host::string(op::GET_ATTR, node.raw(), p, l, 0, 0)
}

/// `getAttribute(name) || ""`, which is what most of the runtime wants.
pub fn attr_or_empty(node: &Node, name: &str) -> String {
    attr(node, name).unwrap_or_default()
}

pub fn set_attr(node: &Node, name: &str, value: &str) {
    let (p, l) = host::s(name);
    let (q, m) = host::s(value);
    host::call(op::SET_ATTR, node.raw(), p, l, q, m);
}

pub fn remove_attr(node: &Node, name: &str) {
    let (p, l) = host::s(name);
    host::call(op::REMOVE_ATTR, node.raw(), p, l, 0, 0);
}

pub fn has_attr(node: &Node, name: &str) -> bool {
    let (p, l) = host::s(name);
    host::flag(op::HAS_ATTR, node.raw(), p, l, 0, 0)
}

/// `[...el.attributes]` as `(name, value)` pairs.
pub fn attributes(node: &Node) -> Vec<(String, String)> {
    let names = collect_strings(host::call(op::ATTR_NAMES, node.raw(), 0, 0, 0, 0));
    names
        .into_iter()
        .map(|name| {
            let value = attr_or_empty(node, &name);
            (name, value)
        })
        .collect()
}

/// A `data-*` attribute by its dashed name; `None` when it is absent, which
/// is what `el.dataset.x === undefined` distinguishes from `""`.
pub fn dataset(node: &Node, dashed: &str) -> Option<String> {
    attr(node, dashed)
}

pub fn toggle_class(node: &Node, class: &str, on: bool) {
    let (p, l) = host::s(class);
    host::call(op::CLASS_TOGGLE, node.raw(), p, l, u32::from(on), 0);
}

pub fn prop_str(node: &Node, name: &str) -> Option<String> {
    let (p, l) = host::s(name);
    host::string(op::PROP_STR, node.raw(), p, l, 0, 0)
}

pub fn set_prop_str(node: &Node, name: &str, value: &str) {
    let (p, l) = host::s(name);
    let (q, m) = host::s(value);
    host::call(op::SET_PROP_STR, node.raw(), p, l, q, m);
}

pub fn prop_bool(node: &Node, name: &str) -> bool {
    let (p, l) = host::s(name);
    host::flag(op::PROP_BOOL, node.raw(), p, l, 0, 0)
}

pub fn set_prop_bool(node: &Node, name: &str, on: bool) {
    let (p, l) = host::s(name);
    host::call(op::SET_PROP_BOOL, node.raw(), p, l, u32::from(on), 0);
}

pub fn prop_num(node: &Node, name: &str) -> f64 {
    let (p, l) = host::s(name);
    host::callf(op::PROP_NUM, node.raw(), p, l, 0, 0)
}

pub fn set_prop_num(node: &Node, name: &str, value: f64) {
    let (p, l) = host::s(name);
    let bits = value.to_bits();
    host::call(op::SET_PROP_NUM, node.raw(), p, l, bits as u32, (bits >> 32) as u32);
}

/// `name in node`.
pub fn has_prop(node: &Node, name: &str) -> bool {
    let (p, l) = host::s(name);
    host::flag(op::HAS_PROP, node.raw(), p, l, 0, 0)
}

/// `typeof node[name] === "string"`.
pub fn prop_is_string(node: &Node, name: &str) -> bool {
    let (p, l) = host::s(name);
    host::flag(op::PROP_IS_STRING, node.raw(), p, l, 0, 0)
}

/// A property whose value is an object: `event.target`, `event.submitter`,
/// `event.relatedTarget`, `input.form`, `response.body`.
pub fn prop_node(node: &Node, name: &str) -> Option<Node> {
    let (p, l) = host::s(name);
    Node::new(host::call(op::PROP_HANDLE, node.raw(), p, l, 0, 0))
}

/// `textContent`, the property the runtime reads on scripts and writes on
/// `data-nr-text` / `data-nr-error` / `data-nr-result` elements.
pub fn text_content(node: &Node) -> String {
    prop_str(node, "textContent").unwrap_or_default()
}

pub fn set_text_content(node: &Node, value: &str) {
    set_prop_str(node, "textContent", value);
}

// ---- styles ---------------------------------------------------------------------

/// `getComputedStyle(node, pseudo).getPropertyValue(property)`. An empty
/// `pseudo` is the element itself.
pub fn computed(node: &Node, pseudo: &str, property: &str) -> String {
    let (p, l) = host::s(pseudo);
    let (q, m) = host::s(property);
    host::text(op::COMPUTED, node.raw(), p, l, q, m)
}

pub fn style_set(node: &Node, name: &str, value: &str) {
    let (p, l) = host::s(name);
    let (q, m) = host::s(value);
    host::call(op::STYLE_SET, node.raw(), p, l, q, m);
}

pub fn style_remove(node: &Node, name: &str) {
    let (p, l) = host::s(name);
    host::call(op::STYLE_REMOVE, node.raw(), p, l, 0, 0);
}

// ---- focus and scrolling ----------------------------------------------------------

pub fn focus(node: &Node) {
    host::call(op::FOCUS, node.raw(), 1, 0, 0, 0);
}

pub fn set_selection_range(node: &Node, start: u32, end: u32, direction: &str) {
    let (p, l) = host::s(direction);
    host::call(op::SET_SELECTION, node.raw(), start, end, p, l);
}

pub fn scroll_into_view(node: &Node) {
    host::call(op::SCROLL_INTO_VIEW, node.raw(), 0, 0, 0, 0);
}

pub fn scroll_to_top() {
    host::call(op::SCROLL_TO, 0, 0, 0, 0, 0);
}

// ---- location, history, storage ------------------------------------------------------

pub fn location(which: u32) -> String {
    host::text(op::LOCATION, which, 0, 0, 0, 0)
}

pub fn href() -> String {
    location(part::HREF)
}

pub fn pathname() -> String {
    location(part::PATHNAME)
}

pub fn search() -> String {
    location(part::SEARCH)
}

pub fn origin() -> String {
    location(part::ORIGIN)
}

/// Where the page is, the way the runtime keeps track of it.
pub fn here() -> String {
    let mut out = pathname();
    out.push_str(&search());
    out
}

pub fn assign(url: &str) {
    let (p, l) = host::s(url);
    host::call(op::SET_LOC_HREF, p, l, 0, 0, 0);
}

pub fn set_hash(hash: &str) {
    let (p, l) = host::s(hash);
    host::call(op::SET_LOC_HASH, p, l, 0, 0, 0);
}

/// `history.pushState({nr:1}, "", url)` or `replaceState`.
pub fn push_state(url: &str, replace: bool) {
    let (p, l) = host::s(url);
    host::call(if replace { op::HISTORY_REPLACE } else { op::HISTORY_PUSH }, p, l, 0, 0, 0);
}

pub fn history_back() {
    host::call(op::HISTORY_BACK, 0, 0, 0, 0, 0);
}

pub fn history_forward() {
    host::call(op::HISTORY_FORWARD, 0, 0, 0, 0, 0);
}

/// `sessionStorage`, which throws in a private window: every one of these
/// swallows that on the shim's side, like the `try`/`catch` in the source.
pub fn storage_get(key: &str) -> Option<String> {
    let (p, l) = host::s(key);
    host::string(op::SS_GET, p, l, 0, 0, 0)
}

pub fn storage_set(key: &str, value: &str) {
    let (p, l) = host::s(key);
    let (q, m) = host::s(value);
    host::call(op::SS_SET, p, l, q, m, 0);
}

pub fn storage_remove(key: &str) {
    let (p, l) = host::s(key);
    host::call(op::SS_REMOVE, p, l, 0, 0, 0);
}

pub fn cookie() -> String {
    host::text(op::COOKIE, 0, 0, 0, 0, 0)
}

// ---- URLs -----------------------------------------------------------------------------

/// `new URL(href, base)`. The browser's parser, never a Rust one: the
/// same-origin checks the runtime makes have to agree with it exactly.
pub struct Url(H);

impl Url {
    /// `None` when the browser's parser throws, which is what every `try` /
    /// `catch` around `new URL` in the source is for.
    pub fn parse(href: &str, base: Option<&str>) -> Option<Self> {
        let (p, l) = host::s(href);
        let (q, m) = base.map_or((0, 0), host::s);
        let handle = host::call(op::URL_NEW, p, l, q, m, 0);
        (handle != 0).then_some(Self(handle))
    }

    /// Relative to the current document, which is the common case.
    pub fn here(href: &str) -> Option<Self> {
        let base = crate::dom::href();
        Self::parse(href, Some(&base))
    }

    fn get(&self, which: u32) -> String {
        host::text(op::URL_PART, self.0, which, 0, 0, 0)
    }

    pub fn href(&self) -> String {
        self.get(part::HREF)
    }

    pub fn origin(&self) -> String {
        self.get(part::ORIGIN)
    }

    pub fn protocol(&self) -> String {
        self.get(part::PROTOCOL)
    }

    pub fn pathname(&self) -> String {
        self.get(part::PATHNAME)
    }

    pub fn search(&self) -> String {
        self.get(part::SEARCH)
    }

    pub fn hash(&self) -> String {
        self.get(part::HASH)
    }

    /// `url.pathname + url.search`, how the runtime compares two pages.
    pub fn page(&self) -> String {
        let mut out = self.pathname();
        out.push_str(&self.search());
        out
    }

    pub fn set_search(&self, value: &str) {
        let (p, l) = host::s(value);
        host::call(op::URL_SET_SEARCH, self.0, p, l, 0, 0);
    }

    pub fn set_hash(&self, value: &str) {
        let (p, l) = host::s(value);
        host::call(op::URL_SET_HASH, self.0, p, l, 0, 0);
    }
}

impl Drop for Url {
    fn drop(&mut self) {
        host::release(self.0);
    }
}

// ---- miscellaneous -----------------------------------------------------------------------

pub fn console_error(message: &str) {
    let (p, l) = host::s(message);
    host::call(op::CONSOLE_ERROR, p, l, 0, 0, 0);
}

pub fn now() -> f64 {
    host::callf(op::NOW, 0, 0, 0, 0, 0)
}

pub fn performance_now() -> f64 {
    host::callf(op::PERF_NOW, 0, 0, 0, 0, 0)
}

pub fn css_escape(value: &str) -> String {
    let (p, l) = host::s(value);
    host::text(op::CSS_ESCAPE, p, l, 0, 0, 0)
}

// ---- identity ----------------------------------------------------------------------------

/// Whether two handles are the same object: the `===` the source writes on
/// nodes (`hit !== document.activeElement`, `hoverLink === a`).
///
/// It cannot be handle equality. The shim takes a fresh slot every time a
/// node crosses, so the same element asked for twice comes back as two
/// different numbers; [`op::SAME`] compares the objects behind them.
pub fn same(a: &Node, b: &Node) -> bool {
    a.raw() == b.raw() || host::flag(op::SAME, a.raw(), b.raw(), 0, 0, 0)
}

/// A number property, `None` when it is `null`: `input.selectionStart` is a
/// number on a text field and `null` on a colour picker, and the source tells
/// them apart with `typeof`. [`op::PROP_NUM`] cannot, because `+null` is `0`.
pub fn prop_number(node: &Node, name: &str) -> Option<f64> {
    let text = prop_str(node, name)?;
    let mut digits = 0f64;
    let mut bytes = text.bytes();
    let negative = text.starts_with('-');
    if negative || text.starts_with('+') {
        bytes.next();
    }
    let mut seen = false;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        digits = digits * 10.0 + f64::from(byte - b'0');
        seen = true;
    }
    seen.then_some(if negative { -digits } else { digits })
}

// ---- events -------------------------------------------------------------------------------

/// `addEventListener` flags.
pub const PASSIVE: u32 = 1;
pub const CAPTURE: u32 = 2;

/// Which constructor [`dispatch`] uses.
pub const EVENT: u32 = 0;
pub const CUSTOM_EVENT: u32 = 1;
pub const SUBMIT_EVENT: u32 = 2;

/// [`dispatch`] flags.
pub const BUBBLES: u32 = 1;
pub const CANCELABLE: u32 = 2;

/// A listener on `target`, or on `window` when there is none: `popstate` and
/// `nr:navigate` are the window's, everything else the document's.
pub fn add_listener(target: Option<&Node>, event: &str, callback: u32, flags: u32) {
    let (p, l) = host::s(event);
    host::call(op::ADD_LISTENER, raw(target), p, l, callback, flags);
}

pub fn remove_listener(target: Option<&Node>, event: &str, callback: u32, flags: u32) {
    let (p, l) = host::s(event);
    host::call(op::REMOVE_LISTENER, raw(target), p, l, callback, flags);
}

pub fn prevent_default(event: &Node) {
    host::call(op::PREVENT_DEFAULT, event.raw(), 0, 0, 0, 0);
}

/// A listener for the life of the document: the callback is registered and
/// never given back, which is what every listener this runtime installs is.
pub fn listen(target: Option<&Node>, event: &str, handler: impl Fn(Option<Node>) + 'static) {
    add_listener(target, event, crate::task::on(handler), 0);
}

/// `target.dispatchEvent(new <kind>(name, { detail, bubbles, cancelable }))`.
///
/// The name and the detail's JSON travel as one string with a NUL between
/// them, which is what the shim splits on: five arguments have no room for
/// two strings and the rest.
pub fn dispatch(target: Option<&Node>, kind: u32, name: &str, detail: &str, flags: u32) {
    let mut packed = String::with_capacity(name.len() + detail.len() + 1);
    packed.push_str(name);
    packed.push('\0');
    packed.push_str(detail);
    let (p, l) = host::s(&packed);
    host::call(op::DISPATCH, raw(target), kind, flags, p, l);
}

// ---- view transitions -----------------------------------------------------------------------

/// Whether the browser has `document.startViewTransition`.
pub fn view_transitions() -> bool {
    host::flag(op::VT_SUPPORTED, 0, 0, 0, 0, 0)
}

/// `document.startViewTransition(callback)`. The shim swallows the rejections
/// of `ready` and `finished` — a navigation that cuts this one short is not an
/// error — and settles `token` with `updateCallbackDone`.
pub fn start_view_transition(callback: u32, token: u32) {
    host::call(op::VT_START, callback, token, 0, 0, 0);
}

// ---- fetch ------------------------------------------------------------------------------------

/// What a request sends: text the module built, or an object the shim already
/// holds a handle for (`URLSearchParams`, `FormData`, a `Blob`).
#[derive(Clone, Copy)]
pub enum RequestBody<'a> {
    Text(&'a str),
    Object(&'a Node),
}

/// `fetch(url, { method, headers, body, signal, credentials: "same-origin" })`.
/// Every request this runtime makes is same-origin, so the shim needs no say
/// in that. `None` is a request that got no answer: the network, or a signal
/// that was aborted.
///
/// The ABI carries five words, so everything but the body's handle travels in
/// one block: the signal and the body's length as two `u32`s, then the URL,
/// the method and the header names and values NUL-terminated, then the body's
/// bytes.
pub async fn fetch(
    url: &str,
    method: &str,
    headers: &[(String, String)],
    body: Option<RequestBody<'_>>,
    signal: Option<&Node>,
) -> Option<Node> {
    let text = match body {
        Some(RequestBody::Text(text)) => text.as_bytes(),
        _ => &[],
    };
    let mut block = Vec::with_capacity(64 + url.len() + text.len());
    block.extend_from_slice(&raw(signal).to_le_bytes());
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
    let (kind, handle) = match body {
        Some(RequestBody::Object(node)) => (2, node.raw()),
        Some(RequestBody::Text(_)) => (1, 0),
        None => (0, 0),
    };
    let slot = crate::task::pending();
    host::call(op::FETCH, block.as_ptr() as u32, block.len() as u32, kind, handle, slot.token());
    let settled = slot.await;
    if settled.failed() { None } else { settled.node() }
}

pub fn res_ok(response: &Node) -> bool {
    host::flag(op::RES_OK, response.raw(), 0, 0, 0, 0)
}

pub fn res_status(response: &Node) -> u32 {
    host::call(op::RES_STATUS, response.raw(), 0, 0, 0, 0)
}

pub fn res_status_text(response: &Node) -> String {
    host::text(op::RES_STATUS_TEXT, response.raw(), 0, 0, 0, 0)
}

/// `res.url`: where the request ended up, after any redirect.
pub fn res_url(response: &Node) -> String {
    host::text(op::RES_URL, response.raw(), 0, 0, 0, 0)
}

/// `res.headers.get(name)`, which is `None` for a header that is not there.
pub fn res_header(response: &Node, name: &str) -> Option<String> {
    let (p, l) = host::s(name);
    host::string(op::RES_HEADER, response.raw(), p, l, 0, 0)
}

/// `await res.text()`; `None` when the body could not be read.
pub async fn res_text(response: &Node) -> Option<String> {
    let slot = crate::task::pending();
    host::call(op::RES_TEXT, response.raw(), slot.token(), 0, 0, 0);
    let settled = slot.await;
    if settled.failed() { None } else { Some(settled.text.unwrap_or_default()) }
}

/// `await res.blob()`, which a form saves as a file.
pub async fn res_blob(response: &Node) -> Option<Node> {
    let slot = crate::task::pending();
    host::call(op::RES_BLOB, response.raw(), slot.token(), 0, 0, 0);
    let settled = slot.await;
    if settled.failed() { None } else { settled.node() }
}

// ---- aborting ------------------------------------------------------------------------------------

/// `new AbortController()`.
pub fn abort_controller() -> Option<Node> {
    Node::new(host::call(op::ABORT_NEW, 0, 0, 0, 0, 0))
}

pub fn abort(controller: &Node) {
    host::call(op::ABORT, controller.raw(), 0, 0, 0, 0);
}

pub fn abort_signal(controller: &Node) -> Option<Node> {
    Node::new(host::call(op::ABORT_SIGNAL, controller.raw(), 0, 0, 0, 0))
}

/// Whether the request behind this signal was dropped.
pub fn signal_aborted(signal: &Node) -> bool {
    host::flag(op::SIGNAL_ABORTED, signal.raw(), 0, 0, 0, 0)
}
