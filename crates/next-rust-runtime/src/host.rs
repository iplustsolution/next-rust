//! The host ABI: two imports the JavaScript shim provides, and nothing else.
//!
//! Everything the browser can do for the runtime goes through `h` (results
//! that are integers or handles) and `f` (results that are numbers), so the
//! shim stays a switch statement over opcodes and carries no behaviour of its
//! own. Handles are indices into a table the shim keeps; `0` means "nothing".
//!
//! Strings are passed as `(ptr, len)` pairs of UTF-8 in and out of the wasm
//! memory. A call that *returns* a string puts it in the shim's pending slot
//! and returns its byte length, or [`NULL_STR`] when there is no string at
//! all; the caller then allocates that many bytes and asks for them with
//! [`op::RESULT_TAKE`]. Two crossings, no allocation on the shim's side, and
//! `Option<String>` comes out the other end.
//!
//! **The order of [`op`] is the ABI.** Append, never renumber: the shim
//! indexes its table with the number the module passes.

#![allow(dead_code)] // the full ABI; each module uses the part it needs

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// A handle into the shim's table. `0` is "nothing".
pub type H = u32;

/// The length a string-returning op reports for `null`/`undefined`.
pub const NULL_STR: u32 = u32::MAX;

/// Every operation the shim implements. The number is the ABI.
pub mod op {
    // -- results and lifetime -------------------------------------------
    /// `(ptr)` copy the pending result string into wasm memory and clear it.
    pub const RESULT_TAKE: u32 = 1;
    /// `(handle)` drop a handle the module will not use again.
    pub const RELEASE: u32 = 2;
    /// `(handle)` a second reference to the same handle.
    pub const RETAIN: u32 = 4;
    /// `(a, b)` whether two handles are the same object. Handles are not
    /// interned — every crossing takes a fresh slot — so this is the only
    /// way to answer the `===` the JavaScript runtime writes on nodes.
    pub const SAME: u32 = 5;
    /// `(ptr, len)` `console.error`.
    pub const CONSOLE_ERROR: u32 = 3;

    // -- documents and traversal ----------------------------------------
    pub const DOCUMENT: u32 = 10;
    pub const BODY: u32 = 11;
    pub const HEAD: u32 = 12;
    pub const DOC_ELEMENT: u32 = 13;
    pub const ACTIVE_ELEMENT: u32 = 14;
    pub const NODE_TYPE: u32 = 15;
    pub const NODE_NAME: u32 = 16;
    pub const IS_CONNECTED: u32 = 17;
    pub const PARENT: u32 = 18;
    pub const FIRST_CHILD: u32 = 19;
    pub const NEXT_SIBLING: u32 = 20;
    pub const PREV_SIBLING: u32 = 21;
    pub const CHILD_NODES: u32 = 22;
    pub const LIST_LEN: u32 = 23;
    pub const LIST_ITEM: u32 = 24;
    pub const QUERY: u32 = 25;
    pub const QUERY_ALL: u32 = 26;
    pub const GET_BY_ID: u32 = 27;
    pub const GET_BY_NAME: u32 = 28;
    pub const CLOSEST: u32 = 29;
    pub const CONTAINS: u32 = 30;
    /// `(root, whatToShow)` a `TreeWalker`; the layout markers are comments.
    pub const WALKER_NEW: u32 = 31;
    pub const WALKER_NEXT: u32 = 32;
    /// `CharacterData.data`, which is what a comment node carries.
    pub const DATA: u32 = 33;
    pub const SET_DATA: u32 = 34;
    pub const CREATE_ELEMENT: u32 = 35;
    /// `(doc, node, deep)` `document.importNode`.
    pub const IMPORT_NODE: u32 = 36;
    pub const APPEND: u32 = 37;
    pub const INSERT_BEFORE: u32 = 38;
    pub const REPLACE_WITH: u32 = 39;
    pub const REMOVE: u32 = 40;
    pub const TEMPLATE_CONTENT: u32 = 41;
    /// `new DOMParser().parseFromString(s, "text/html")`.
    pub const PARSE_HTML: u32 = 42;
    pub const TITLE: u32 = 43;
    pub const SET_TITLE: u32 = 44;

    // -- attributes and properties --------------------------------------
    pub const GET_ATTR: u32 = 50;
    pub const SET_ATTR: u32 = 51;
    pub const REMOVE_ATTR: u32 = 52;
    pub const HAS_ATTR: u32 = 53;
    pub const ATTR_NAMES: u32 = 54;
    /// `(list, i)` the `i`th entry of a list of strings.
    pub const LIST_STR: u32 = 55;
    pub const CLASS_TOGGLE: u32 = 56;
    pub const PROP_STR: u32 = 57;
    pub const SET_PROP_STR: u32 = 58;
    pub const PROP_BOOL: u32 = 59;
    pub const SET_PROP_BOOL: u32 = 60;
    pub const PROP_NUM: u32 = 61;
    pub const SET_PROP_NUM: u32 = 62;
    pub const HAS_PROP: u32 = 63;
    pub const PROP_IS_STRING: u32 = 64;
    /// A property whose value is an object: `event.target`, `.submitter`, ...
    pub const PROP_HANDLE: u32 = 65;

    // -- events ----------------------------------------------------------
    /// `(target, type_ptr, type_len, cb, flags)`; flags 1 passive, 2 capture.
    ///
    /// A target of `0` is **`window`**, not "nothing". Nothing returns a
    /// handle for it, and the runtime needs it: `popstate`, the `submit`
    /// listener that must run after every other one, and `nr:navigate`.
    pub const ADD_LISTENER: u32 = 70;
    pub const REMOVE_LISTENER: u32 = 71;
    pub const PREVENT_DEFAULT: u32 = 72;
    /// `(target, kind, flags, args_ptr, args_len)` with the arguments packed
    /// as `"<name>\0<detail JSON>"`; kind 0 `Event`, 1 `CustomEvent`,
    /// 2 `SubmitEvent`; flags 1 bubbles, 2 cancelable. A target of `0` is
    /// `window`, as for [`ADD_LISTENER`].
    pub const DISPATCH: u32 = 73;

    // -- forms -----------------------------------------------------------
    pub const FORM_DATA_NEW: u32 = 80;
    pub const FD_LEN: u32 = 81;
    pub const FD_KEY: u32 = 82;
    // 83 was `FD_IS_FILE`. `FD_VALUE` already answers it: a file yields its
    // `.name`, which is the rule the runtime actually wants. Never reused.
    pub const FD_VALUE: u32 = 84;
    pub const USP_NEW: u32 = 85;
    pub const USP_APPEND: u32 = 86;
    pub const USP_GET: u32 = 87;
    pub const USP_STR: u32 = 88;
    pub const FORM_RESET: u32 = 89;
    pub const CHECK_VALIDITY: u32 = 90;
    pub const CSS_ESCAPE: u32 = 91;
    pub const FOCUS: u32 = 92;
    pub const SET_SELECTION: u32 = 93;
    pub const SCROLL_INTO_VIEW: u32 = 94;
    pub const SCROLL_TO: u32 = 95;
    /// Make `HTMLFormElement.prototype.submit` re-enter the module.
    pub const PATCH_FORM_SUBMIT: u32 = 96;
    pub const NATIVE_SUBMIT: u32 = 97;

    // -- styles ----------------------------------------------------------
    /// `(el, pseudo, prop)` `getComputedStyle(el, pseudo).getPropertyValue(p)`.
    pub const COMPUTED: u32 = 100;
    pub const STYLE_SET: u32 = 101;
    pub const STYLE_REMOVE: u32 = 102;

    // -- fetch and streams ------------------------------------------------
    /// `(url, method, headers, body_kind|body<<8, body_ptr, signal)`, with the
    /// token in the high half of `body_kind`. Resolves by calling `resume`.
    pub const FETCH: u32 = 110;
    pub const RES_STATUS: u32 = 111;
    pub const RES_OK: u32 = 112;
    pub const RES_URL: u32 = 113;
    pub const RES_STATUS_TEXT: u32 = 114;
    pub const RES_HEADER: u32 = 115;
    pub const RES_TEXT: u32 = 116;
    pub const RES_BLOB: u32 = 117;
    pub const RES_READER: u32 = 118;
    pub const READER_READ: u32 = 119;
    pub const READER_CANCEL: u32 = 120;
    pub const ABORT_NEW: u32 = 121;
    pub const ABORT: u32 = 122;
    pub const ABORT_SIGNAL: u32 = 123;
    pub const SIGNAL_ABORTED: u32 = 124;
    pub const OBJECT_URL: u32 = 125;
    pub const REVOKE_URL: u32 = 126;

    // -- timers and time ---------------------------------------------------
    pub const SET_TIMEOUT: u32 = 130;
    pub const CLEAR_TIMEOUT: u32 = 131;
    pub const NOW: u32 = 132;
    pub const PERF_NOW: u32 = 133;

    // -- location, history, storage, URL -----------------------------------
    /// `(which)` 0 href, 1 origin, 2 protocol, 3 pathname, 4 search, 5 hash.
    pub const LOCATION: u32 = 140;
    pub const SET_LOC_HREF: u32 = 141;
    pub const SET_LOC_HASH: u32 = 142;
    pub const HISTORY_PUSH: u32 = 143;
    pub const HISTORY_REPLACE: u32 = 144;
    pub const HISTORY_BACK: u32 = 145;
    pub const HISTORY_FORWARD: u32 = 146;
    pub const SS_GET: u32 = 147;
    pub const SS_SET: u32 = 148;
    pub const SS_REMOVE: u32 = 149;
    pub const COOKIE: u32 = 150;
    /// `new URL(href, base)`; `0` when the browser's parser throws.
    pub const URL_NEW: u32 = 151;
    pub const URL_PART: u32 = 152;
    pub const URL_SET_SEARCH: u32 = 153;
    pub const URL_SET_HASH: u32 = 154;

    // -- view transitions ---------------------------------------------------
    pub const VT_SUPPORTED: u32 = 160;
    /// `(cb, token)` `document.startViewTransition`.
    pub const VT_START: u32 = 161;

    // -- island modules -----------------------------------------------------
    pub const IMPORT_MODULE: u32 = 170;
    pub const REGISTRY_HAS: u32 = 171;
    pub const REGISTRY_LOAD: u32 = 172;
    /// `(module, element, props_json, token)`.
    pub const MODULE_HYDRATE: u32 = 173;

    // -- the public API on `window.nextRust` --------------------------------
    /// Install the forwarding functions; the shim does this once.
    pub const INSTALL_API: u32 = 180;
    /// `(sink, event, data, id)` one server-sent event for `nextRust.stream`.
    pub const STREAM_PUSH: u32 = 181;
    /// `(sink, ptr, len)` the stream ended: nothing where it ran out, or the
    /// JSON of the `Error` it failed with (`message` plus `status` and
    /// `data`), which the shim rebuilds the way [`SETTLE`]'s kind 4 does.
    pub const STREAM_END: u32 = 182;
    /// `(token, kind, a, b)` settle a promise the shim handed out: kind 0 a
    /// handle, 1 a string, 2 an `Error`, 3 a JSON value, 4 an `Error` whose
    /// other members are JSON (a failed action's `errors` and `status`).
    pub const SETTLE: u32 = 183;
    /// `(handle)` a value a caller passed in, as JSON. `nextRust.request`
    /// takes a plain options object and the table cannot walk one.
    pub const STRINGIFY: u32 = 184;
    /// `(cb)` a JavaScript function that re-enters the module.
    /// `nextRust.loader.start()` answers with the function that ends the load.
    pub const CALLBACK_FN: u32 = 185;
}

/// Which part of a `URL` or of `location` to read.
pub mod part {
    pub const HREF: u32 = 0;
    pub const ORIGIN: u32 = 1;
    pub const PROTOCOL: u32 = 2;
    pub const PATHNAME: u32 = 3;
    pub const SEARCH: u32 = 4;
    pub const HASH: u32 = 5;
}

/// `NodeFilter.SHOW_COMMENT`.
pub const SHOW_COMMENT: u32 = 128;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "e")]
unsafe extern "C" {
    /// Every operation whose result is an integer, a boolean or a handle.
    fn h(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32;
    /// The same table, for operations whose result is a number.
    fn f(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> f64;
}

// Off the browser the ABI is inert: the pure logic is what the workspace
// tests, and every DOM call answers "nothing". That keeps `cargo test
// --workspace` meaningful without a mock DOM pretending to be a browser.
#[cfg(not(target_arch = "wasm32"))]
unsafe fn h(_op: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32) -> u32 {
    0
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn f(_op: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32) -> f64 {
    0.0
}

/// One call, integer result.
#[inline]
pub fn call(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
    unsafe { h(op, a, b, c, d, e) }
}

/// One call, number result.
#[inline]
pub fn callf(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> f64 {
    unsafe { f(op, a, b, c, d, e) }
}

/// A string argument as the `(ptr, len)` the ABI takes.
#[inline]
pub fn s(text: &str) -> (u32, u32) {
    (text.as_ptr() as u32, text.len() as u32)
}

/// The string a call left in the pending slot, or `None` for `null`.
fn take(len: u32) -> Option<String> {
    if len == NULL_STR {
        return None;
    }
    let mut bytes = vec![0u8; len as usize];
    call(op::RESULT_TAKE, bytes.as_mut_ptr() as u32, len, 0, 0, 0);
    // The shim only ever writes what the DOM gave it, which is UTF-16 encoded
    // as UTF-8: lossy is unreachable, but it must not be a panic if it is not.
    // The shim only ever writes what the DOM gave it, so this cannot fail;
    // `from_utf8_lossy` would drag `core::fmt`'s escaping in for a branch
    // that is unreachable.
    Some(String::from_utf8(bytes).unwrap_or_default())
}

/// A call whose result is a string, or `None`.
#[inline]
pub fn string(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> Option<String> {
    take(call(op, a, b, c, d, e))
}

/// A call whose result is a string, with `""` for `null`.
#[inline]
pub fn text(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> String {
    string(op, a, b, c, d, e).unwrap_or_default()
}

/// A call whose result is a boolean.
#[inline]
pub fn flag(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> bool {
    call(op, a, b, c, d, e) != 0
}

/// Give a handle back to the shim. Nothing is released twice: `0` is a no-op
/// on the shim's side as well.
#[inline]
pub fn release(handle: H) {
    if handle != 0 {
        call(op::RELEASE, handle, 0, 0, 0, 0);
    }
}

/// Memory the shim writes into. Every buffer it is given comes from here, and
/// [`dealloc`] gives it back.
///
/// # Safety
/// Exported for the shim only. A pointer it returns is valid for `len` bytes
/// until the module passes it to `dealloc`.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> u32 {
    let mut buffer = Vec::<u8>::with_capacity(len as usize);
    let ptr = buffer.as_mut_ptr() as u32;
    core::mem::forget(buffer);
    ptr
}

/// Release memory [`alloc()`] handed out.
///
/// # Safety
/// `ptr` must be a pointer [`alloc()`] returned for exactly `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: u32, len: u32) {
    if ptr != 0 {
        drop(unsafe { Vec::from_raw_parts(ptr as *mut u8, 0, len as usize) });
    }
}
