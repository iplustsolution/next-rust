//! The one import every island shares, and the opcode table it dispatches on.
//!
//! The browser side of this is a switch statement over `OP` and nothing else,
//! so no island behaviour is readable in the JavaScript that ships. Handles are
//! indices into a table the shim keeps, and `0` means "nothing" — which is why
//! every operation is safe to call on a handle that a query failed to produce.
//!
//! **The numbers in [`op`] are the ABI.** Append, never reorder and never
//! reuse: a module compiled against one numbering runs against a shim that may
//! have been cached by a browser from an earlier deploy.

/// A string that was absent, as opposed to present and empty.
pub const NONE: u32 = u32::MAX;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "e")]
unsafe extern "C" {
    fn h(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32;
}

/// Off the browser the import does not exist, so every call answers "nothing".
/// This is what lets the crate compile, and its pure parts be tested, on the
/// host — `cargo test --workspace` builds it for the machine it runs on.
///
/// "Nothing" is `0` for a handle or a boolean, and [`NONE`] for the operations
/// whose answer is an optional string, so that a host-side test of
/// `el.attr(..)` sees the absent case rather than an empty one.
#[cfg(not(target_arch = "wasm32"))]
unsafe fn h(op: u32, _a: u32, _b: u32, _c: u32, _d: u32, _e: u32) -> u32 {
    #[cfg(test)]
    recorder::record(op, [_a, _b, _c, _d, _e]);
    const OPTIONAL: [u32; 8] = [
        op::GET_ATTR,
        op::GET_PROP_STR,
        op::STYLE_GET,
        op::DATASET_GET,
        op::STORE_GET,
        op::INTL_NUM,
        op::INTL_DATE,
        op::INTL_REGION,
    ];
    if OPTIONAL.contains(&op) { NONE } else { 0 }
}

/// Host-side only: the arguments of the last call to each opcode, so a test
/// can check the *argument layout* a wrapper emits against what the shim's
/// handler reads.
///
/// That layout is the half of the ABI no `cargo test` otherwise sees — the
/// shim is JavaScript, in another crate — and getting it wrong is silent on
/// both sides: the shim reads the wrong slot, answers `0`, and wasm carries
/// on. One such mismatch shipped (`cookie::set` putting its string pointer in
/// the flag slot), and the only symptom was a preference that never persisted.
///
/// It records the raw words, never dereferencing them: a `&str` pointer is 64
/// bits on the machine running the tests and `s()` truncates it to the wasm
/// ABI's 32, so the recorded value is not a pointer here and reading it would
/// fault. A test compares it with `s(the_same_str)` instead.
#[cfg(all(not(target_arch = "wasm32"), test))]
pub(crate) mod recorder {
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    std::thread_local! {
        static CALLS: RefCell<BTreeMap<u32, [u32; 5]>> = const { RefCell::new(BTreeMap::new()) };
    }

    pub(crate) fn record(op: u32, args: [u32; 5]) {
        CALLS.with(|calls| calls.borrow_mut().insert(op, args));
    }

    /// The arguments of the last call to `op` on this thread, if there was one.
    pub(crate) fn last(op: u32) -> Option<[u32; 5]> {
        CALLS.with(|calls| calls.borrow().get(&op).copied())
    }
}

/// Call the host. Safe because every operation is total on the browser side:
/// an unknown opcode, a stale handle and a malformed argument all answer `0`
/// rather than throwing back into wasm.
#[inline]
pub fn call(op: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
    unsafe { h(op, a, b, c, d, e) }
}

#[inline]
pub fn call0(op: u32) -> u32 {
    call(op, 0, 0, 0, 0, 0)
}

#[inline]
pub fn call1(op: u32, a: u32) -> u32 {
    call(op, a, 0, 0, 0, 0)
}

#[inline]
pub fn call2(op: u32, a: u32, b: u32) -> u32 {
    call(op, a, b, 0, 0, 0)
}

/// A string argument, as the pointer and length pair the shim decodes.
#[inline]
pub fn s(text: &str) -> (u32, u32) {
    (text.as_ptr() as u32, text.len() as u32)
}

/// The halves of an `f64`, low word first, as the numeric operations take it.
#[inline]
pub fn f(value: f64) -> (u32, u32) {
    let bits = value.to_bits();
    (bits as u32, (bits >> 32) as u32)
}

/// Call an operation whose only argument is a string, in the first two slots.
#[inline]
pub fn call_s0(op: u32, text: &str) -> u32 {
    let (p, l) = s(text);
    call(op, p, l, 0, 0, 0)
}

/// Call an operation whose second and third arguments are one string.
#[inline]
pub fn call_s(op: u32, a: u32, text: &str) -> u32 {
    let (p, l) = s(text);
    call(op, a, p, l, 0, 0)
}

/// Call an operation taking a handle and two strings — a name and a value.
#[inline]
pub fn call_ss(op: u32, a: u32, name: &str, value: &str) -> u32 {
    let (p, l) = s(name);
    let (q, m) = s(value);
    call(op, a, p, l, q, m)
}

/// The opcode table. The comment on each constant is the browser call it makes.
///
/// 1 to 15 are the original island table and are frozen; everything above was
/// appended for the full client ABI. Gaps are deliberate: 115 to 139 belong to
/// the client-runtime port and live in a separate shim file, and 170 upwards
/// belong to the optional heavy-API extension.
pub mod op {
    // --- 1..15: the original table -----------------------------------------
    /// `addEventListener`
    pub const ON: u32 = 1;
    /// `classList.toggle`
    pub const TOGGLE: u32 = 2;
    /// `querySelector`
    pub const QUERY: u32 = 3;
    /// `setAttribute`
    pub const SET_ATTR: u32 = 4;
    /// `textContent =`
    pub const SET_TEXT: u32 = 5;
    /// `nextRust.action`, fire and forget
    pub const ACTION_FIRE: u32 = 6;
    /// `nextRust.navigate`
    pub const NAV: u32 = 7;
    /// `closest`
    pub const CLOSEST: u32 = 8;
    /// `closest` from the current event's target
    pub const EVT_CLOSEST: u32 = 9;
    /// the current event's modifier and button flags, OR-ed together
    pub const EVT_FLAGS: u32 = 10;
    /// compare `event.key`
    pub const EVT_KEY_IS: u32 = 11;
    /// `preventDefault`
    pub const EVT_PREVENT: u32 = 12;
    /// `focus({preventScroll: true})`
    pub const FOCUS: u32 = 13;
    /// set an IDL boolean property
    pub const PROP_BOOL: u32 = 14;
    /// `history.replaceState` to an element's own `href`
    pub const HISTORY_REPLACE: u32 = 15;

    // --- 16..20: strings out of the DOM ------------------------------------
    /// copy the pending string into linear memory
    pub const STR_TAKE: u32 = 16;
    /// `textContent`
    pub const TEXT: u32 = 17;
    /// `.value`
    pub const VALUE: u32 = 18;
    /// `getAttribute`
    pub const GET_ATTR: u32 = 19;
    /// an IDL string property
    pub const GET_PROP_STR: u32 = 20;

    // --- 21..35, 87: the tree ----------------------------------------------
    pub const QUERY_ALL: u32 = 21;
    pub const LIST_LEN: u32 = 22;
    pub const LIST_GET: u32 = 23;
    pub const PARENT: u32 = 24;
    pub const CHILDREN: u32 = 25;
    pub const CREATE: u32 = 26;
    pub const APPEND: u32 = 27;
    pub const INSERT_BEFORE: u32 = 28;
    pub const REMOVE: u32 = 29;
    pub const REPLACE_WITH: u32 = 30;
    pub const QUERY_DOC: u32 = 31;
    pub const QUERY_ALL_DOC: u32 = 32;
    pub const MATCHES: u32 = 33;
    pub const CLONE: u32 = 34;
    /// `innerHTML =`, for compile-time literal markup only
    pub const SET_HTML: u32 = 35;

    // --- 36..48: attributes, classes, styles, dataset ----------------------
    pub const HAS_ATTR: u32 = 36;
    pub const REMOVE_ATTR: u32 = 37;
    pub const CLASS_ADD: u32 = 38;
    pub const CLASS_REMOVE: u32 = 39;
    pub const CLASS_HAS: u32 = 40;
    pub const SET_PROP_STR: u32 = 41;
    pub const SET_PROP_NUM: u32 = 42;
    pub const GET_PROP_NUM: u32 = 43;
    pub const GET_PROP_BOOL: u32 = 44;
    pub const STYLE_SET: u32 = 45;
    pub const STYLE_GET: u32 = 46;
    pub const DATASET_GET: u32 = 47;
    pub const DATASET_SET: u32 = 48;

    // --- 49: staged arguments ----------------------------------------------
    /// stage a string for the next operation, for the ones that need more
    /// arguments than the import carries
    pub const ARG: u32 = 49;

    // --- 50..61: events -----------------------------------------------------
    pub const ON_DELEGATE: u32 = 50;
    pub const OFF: u32 = 51;
    pub const EVT_STOP: u32 = 52;
    pub const EVT_TARGET: u32 = 53;
    pub const EVT_CURRENT: u32 = 54;
    pub const EVT_KEY: u32 = 55;
    pub const EVT_DETAIL: u32 = 56;
    pub const DISPATCH: u32 = 57;
    pub const LISTEN_WINDOW: u32 = 58;
    pub const LISTEN_DOC: u32 = 59;
    pub const BLUR: u32 = 60;
    pub const ON_OPTS: u32 = 61;

    // --- 62..69: async ------------------------------------------------------
    pub const FETCH: u32 = 62;
    pub const ACTION: u32 = 63;
    pub const TIMEOUT: u32 = 64;
    pub const RAF: u32 = 65;
    pub const IDLE: u32 = 66;
    pub const SSE_OPEN: u32 = 67;
    pub const SSE_CLOSE: u32 = 68;
    pub const ABORT: u32 = 69;

    // --- 70..74: timers and observers --------------------------------------
    pub const SET_TIMEOUT: u32 = 70;
    pub const SET_INTERVAL: u32 = 71;
    pub const CLEAR_TIMER: u32 = 72;
    pub const OBSERVE: u32 = 73;
    pub const UNOBSERVE: u32 = 74;

    // --- 75..82: storage, history, location, formatting --------------------
    pub const STORE_GET: u32 = 75;
    pub const STORE_SET: u32 = 76;
    pub const STORE_DEL: u32 = 77;
    pub const HISTORY: u32 = 78;
    pub const LOCATION: u32 = 79;
    pub const NAVIGATE: u32 = 80;
    pub const INTL_NUM: u32 = 81;
    pub const INTL_DATE: u32 = 82;

    // --- 83..88: misc -------------------------------------------------------
    pub const LOG: u32 = 83;
    pub const SCROLL_INTO_VIEW: u32 = 84;
    pub const RECT: u32 = 85;
    pub const NOW: u32 = 86;
    /// release a handle slot
    pub const FREE: u32 = 87;
    pub const MEDIA: u32 = 88;

    // --- 89..99: roots, geometry, the top layer ----------------------------
    /// name a global object: document, window, body and friends
    pub const ROOT: u32 = 89;
    pub const CONTAINS: u32 = 90;
    pub const GET_BY_ID: u32 = 91;
    pub const CREATE_NS: u32 = 92;
    pub const TOGGLE_ATTR: u32 = 93;
    pub const METRIC: u32 = 94;
    pub const SET_METRIC: u32 = 95;
    pub const SCROLL_BY: u32 = 96;
    pub const SUPPORTS: u32 = 97;
    pub const TOP_LAYER: u32 = 98;
    pub const SET_SELECTION: u32 = 99;

    // --- 100..114: any JavaScript value ------------------------------------
    pub const PROP_H: u32 = 100;
    pub const HAS_PROP: u32 = 101;
    pub const TYPEOF: u32 = 102;
    pub const SET_PROP_H: u32 = 103;
    pub const CALL_S: u32 = 104;
    pub const CALL_H: u32 = 105;
    pub const CALL_N: u32 = 106;
    pub const NEW: u32 = 107;
    pub const STR_OF: u32 = 108;
    pub const NUM_OF: u32 = 109;
    pub const BOOL_OF: u32 = 110;
    pub const INDEX: u32 = 111;
    pub const LEN_OF: u32 = 112;
    pub const EQ: u32 = 113;
    pub const KEYS: u32 = 114;

    // --- 115..116: two reads a selector cannot stand in for -----------------
    /// `getComputedStyle(el, pseudo).prop`. The pseudo-element form is not a
    /// nicety: the loading bar reads `::before`'s opacity and throws it away,
    /// purely to force a style recalculation so its transition has somewhere
    /// to animate from. The name is staged in `ARG 0`.
    pub const COMPUTED_PSEUDO: u32 = 115;
    /// `document.getElementsByName`. A `[name=…]` selector is not the same
    /// thing: restoring focus after a page swap finds the field by its index
    /// among same-named fields, and that index has to be over the same live
    /// collection the source used.
    pub const BY_NAME: u32 = 116;

    // --- 140..152: actions, and the long tail the app islands asked for ----
    pub const ACTION_NEW: u32 = 140;
    pub const ACTION_FIELD: u32 = 141;
    pub const ACTION_FILE_B64: u32 = 142;
    pub const ACTION_FORM_FIELDS: u32 = 143;
    pub const ACTION_SEND: u32 = 144;
    pub const INTL_REGION: u32 = 145;
    /// `nextRust.refresh` / `back` / `forward` / `prefetch` / the loader
    pub const NR_CALL: u32 = 146;
    /// `window.confirm`, which has to stay synchronous
    pub const CONFIRM: u32 = 147;
    /// read or write `document.cookie`
    pub const COOKIE: u32 = 148;
    /// `new FormData(form, submitter)` as an encoded query string
    pub const FORM_ENCODE: u32 = 149;
    /// `Intl.DateTimeFormat().resolvedOptions().timeZone`
    pub const INTL_TZ: u32 = 150;
    pub const EVT_STOP_IMMEDIATE: u32 = 151;
    /// `JSON.parse`, re-encoded as a props blob — the only JSON decoder
    pub const JSON_DECODE: u32 = 152;

    // --- 153..159: the JavaScript bridge for vendored SDKs ------------------
    //
    // Firebase and Razorpay are third-party JavaScript that will never be
    // wasm. What can be wasm is every decision around them — which provider,
    // which error message, which ordering — and these seven operations are
    // what that decision-making needs to drive an SDK it cannot contain.
    /// `window[name]`
    pub const JS_GLOBAL: u32 = 153;
    /// dynamic `import(url)`, resolving to a module namespace handle. The URL
    /// must come from props: the build rewrites module specifiers in script
    /// text and cannot rewrite one inside a data section.
    pub const JS_IMPORT: u32 = 154;
    /// load a classic `<script src>`, for an SDK that is not a module
    pub const JS_SCRIPT: u32 = 155;
    /// wrap a callback as a real JavaScript function value, so an SDK can call
    /// back into wasm. Without this, any SDK taking a handler is unreachable.
    pub const JS_FN: u32 = 156;
    /// `new ctor(a, b)` where `ctor` is a handle rather than a built-in
    pub const JS_NEW: u32 = 157;
    /// `JSON.parse` to a handle. The counterpart to `JSON_DECODE`: this one is
    /// for a value read a few times, that one for props read many times.
    pub const PARSE_JSON: u32 = 158;
    /// call a method and await the promise it returns
    pub const JS_CALL_ASYNC: u32 = 159;

    // --- 160..169: the UI kit ----------------------------------------------
    pub const MEDIA_LISTEN: u32 = 160;
    pub const CLIPBOARD_WRITE: u32 = 161;
    pub const TODAY: u32 = 162;
    pub const DISPATCH_NATIVE: u32 = 163;
    pub const USER_AGENT: u32 = 164;
    pub const CREATE_TEXT: u32 = 165;
    pub const SCROLL_TO: u32 = 166;
    /// `setAttributeNS`, for the handful of SVG attributes that are actually
    /// namespaced. The name is staged in `ARG 0`.
    pub const SET_ATTR_NS: u32 = 167;
    /// An action whose response comes back as a JavaScript value rather than
    /// as a props blob. The blob hashes field names away, which is right for
    /// props and wrong for a response whose *keys are the data* — a
    /// match-the-pairs answer, say. This keeps the names, at the price of a
    /// host round trip per field read.
    pub const ACTION_VAL: u32 = 168;
    /// `String(n)` for a number with a fractional part.
    ///
    /// Formatting an `f64` in Rust links `core::fmt`'s float path, which was
    /// measured at 13 KB brotli — larger than most islands. The browser
    /// already has the exact semantics we want, so it does the work.
    pub const NUM_TO_STR: u32 = 169;
    /// Publish a function on `window.nextRust` that forwards its argument to
    /// a custom event. Wasm cannot create a JavaScript function, so a module
    /// that has a public API callable from page script needs the shim to make
    /// one — but the shim is told the name and the event, never what either
    /// means.
    pub const INSTALL_FORWARDER: u32 = 170;
}

/// Read the string the last operation left pending.
///
/// The transfer is two steps on purpose: the operation stores the string on the
/// browser side and answers its length in bytes, then this copies it in. The
/// shim therefore never calls back into wasm while wasm is inside a host call,
/// and no caller has to guess a buffer size. A `len` of [`NONE`] means the
/// value was absent, which is not the same as empty.
pub fn take(len: u32) -> Option<alloc::string::String> {
    use alloc::vec::Vec;
    if len == NONE {
        return None;
    }
    let len = len as usize;
    if len == 0 {
        return Some(alloc::string::String::new());
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(len);
    let wrote = call1(op::STR_TAKE, bytes.as_mut_ptr() as u32) as usize;
    let wrote = wrote.min(len);
    // SAFETY: the shim wrote `wrote <= len` bytes into the reservation above.
    unsafe { bytes.set_len(wrote) };
    Some(alloc::string::String::from_utf8(bytes).unwrap_or_default())
}

/// Somewhere for an operation to write `f64`s into linear memory.
///
/// Which argument carries the address differs per operation, so the caller
/// passes `slot.ptr()` in the right position rather than this guessing.
pub struct Slot<const N: usize>([f64; N]);

impl<const N: usize> Slot<N> {
    #[inline]
    pub fn new() -> Self {
        Self([0.0; N])
    }

    #[inline]
    pub fn ptr(&mut self) -> u32 {
        self.0.as_mut_ptr() as u32
    }

    #[inline]
    pub fn get(&self, i: usize) -> f64 {
        self.0[i]
    }
}

impl<const N: usize> Default for Slot<N> {
    fn default() -> Self {
        Self::new()
    }
}
