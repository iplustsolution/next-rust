//! Storage, history, location, navigation, formatting and the other things
//! that hang off the window rather than off an element.
//!
//! Formatting is a host call on purpose. Rust can format a number, but
//! formatting *money in a locale* needs ICU data, and linking that into a
//! module a browser downloads would cost more than everything else here put
//! together — while the browser already has it.

use crate::host::{self, call, call_s, call_s0, call0, call1, call2, op, s};
use crate::text::Str;

/// Which storage a key lives in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
enum Kind {
    Local = 0,
    Session = 1,
}

/// `localStorage` and `sessionStorage`.
///
/// Every call is wrapped in a try/catch on the browser side: storage that is
/// blocked — Safari's private mode, a browser with site data disabled — reads
/// as `None` and writes are dropped. An island never has to handle a throw.
pub mod storage {
    use super::{Kind, Str, call, call_s, host, op, s};

    fn get(kind: Kind, key: &str) -> Option<Str> {
        let (p, l) = s(key);
        host::take(call(op::STORE_GET, kind as u32, p, l, 0, 0)).map(Str::new)
    }

    fn set(kind: Kind, key: &str, value: &str) {
        let (p, l) = s(key);
        let (q, m) = s(value);
        call(op::STORE_SET, kind as u32, p, l, q, m);
    }

    fn remove(kind: Kind, key: &str) {
        call_s(op::STORE_DEL, kind as u32, key);
    }

    pub fn local_get(key: &str) -> Option<Str> {
        get(Kind::Local, key)
    }

    pub fn local_set(key: &str, value: &str) {
        set(Kind::Local, key, value);
    }

    pub fn local_remove(key: &str) {
        remove(Kind::Local, key);
    }

    pub fn session_get(key: &str) -> Option<Str> {
        get(Kind::Session, key)
    }

    pub fn session_set(key: &str, value: &str) {
        set(Kind::Session, key, value);
    }

    pub fn session_remove(key: &str) {
        remove(Kind::Session, key);
    }
}

/// The session history.
pub mod history {
    use super::{El, call_s, call1, op};

    pub fn push(url: &str) {
        call_s(op::HISTORY, 0, url);
    }

    pub fn replace(url: &str) {
        call_s(op::HISTORY, 1, url);
    }

    pub fn back() {
        call1(op::NR_CALL, 1);
    }

    pub fn forward() {
        call1(op::NR_CALL, 2);
    }

    /// `history.replaceState` to a link's own `href`, so a tab switch leaves a
    /// shareable URL behind without navigating.
    pub fn replace_href_of(link: El) {
        call1(op::HISTORY_REPLACE, link.0);
    }
}

/// Reading the current URL.
pub mod location {
    use super::{Str, call2, host, op};

    fn part(which: u32) -> Str {
        Str::new(host::take(call2(op::LOCATION, which, 0)).unwrap_or_default())
    }

    pub fn href() -> Str {
        part(0)
    }

    pub fn pathname() -> Str {
        part(1)
    }

    pub fn search() -> Str {
        part(2)
    }

    pub fn hash() -> Str {
        part(3)
    }

    pub fn origin() -> Str {
        part(4)
    }

    /// One query-string parameter, percent-decoded.
    ///
    /// Read out of `search()` here rather than through `URLSearchParams`,
    /// because the answer is a single small string and a host round trip per
    /// lookup costs more than the scan.
    pub fn param(name: &str) -> Option<Str> {
        let search = search();
        let query = search.as_str().strip_prefix('?').unwrap_or(search.as_str());
        for pair in query.split('&') {
            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            if decode(key) == name {
                return Some(Str::from(decode(value).as_str()));
            }
        }
        None
    }

    pub(crate) fn decode(raw: &str) -> alloc::string::String {
        let bytes = raw.as_bytes();
        let mut out = alloc::vec::Vec::with_capacity(bytes.len());
        let mut at = 0;
        while at < bytes.len() {
            match bytes[at] {
                b'+' => {
                    out.push(b' ');
                    at += 1;
                }
                b'%' if at + 2 < bytes.len() => match (hex(bytes[at + 1]), hex(bytes[at + 2])) {
                    (Some(high), Some(low)) => {
                        out.push(high << 4 | low);
                        at += 3;
                    }
                    _ => {
                        out.push(bytes[at]);
                        at += 1;
                    }
                },
                byte => {
                    out.push(byte);
                    at += 1;
                }
            }
        }
        alloc::string::String::from_utf8(out).unwrap_or_default()
    }

    fn hex(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
}

use crate::dom::El;

/// A client-side navigation through the framework's runtime, which keeps the
/// page rather than reloading it.
pub fn navigate(url: &str) {
    call_s0(op::NAV, url);
}

pub fn navigate_replace(url: &str) {
    let (p, l) = s(url);
    call(op::NAVIGATE, p, l, 1, 0, 0);
}

/// Re-render the current URL in place, keeping the scroll position and
/// bypassing the client cache. Not expressible as a `navigate`, which is why
/// it is its own call.
pub fn refresh() {
    call1(op::NR_CALL, 0);
}

pub fn prefetch(url: &str) {
    let (p, l) = s(url);
    call(op::ARG, 0, p, l, 0, 0);
    call1(op::NR_CALL, 3);
}

/// `document.cookie`. The server reads some of these to render the first
/// paint — a collapsed sidebar, a chosen theme — which is why an island writes
/// a cookie rather than local storage for that kind of state.
pub mod cookie {
    use super::{Str, call_s, call2, host, op};

    pub fn get(name: &str) -> Option<Str> {
        let all = Str::new(host::take(call2(op::COOKIE, 0, 0)).unwrap_or_default());
        for pair in all.as_str().split(';') {
            let pair = pair.trim_start();
            if let Some(value) = pair.strip_prefix(name).and_then(|rest| rest.strip_prefix('=')) {
                return Some(Str::from(value));
            }
        }
        None
    }

    /// Assigned raw, so the caller writes the attributes it wants:
    /// `"pp_sidebar=1; Path=/; Max-Age=31536000; SameSite=Lax"`.
    ///
    /// The `1` in the first slot is what tells the shim this is a write, and
    /// the string goes in the *second and third* — which is why this is
    /// `call_s` and not `call_s0`. With `call_s0` the pointer landed in the
    /// flag slot and the shim read its string from `(len, 0)`, an empty one,
    /// so it assigned `document.cookie = ""`: a silent no-op. Nothing throws,
    /// the island's own state still changes, and the only symptom is that the
    /// server renders the old value on the next request. See the test below.
    pub fn set(raw: &str) {
        call_s(op::COOKIE, 1, raw);
    }
}

/// Numbers and dates, formatted by the browser.
pub mod intl {
    use super::{Str, call, host, op, s};

    /// `opts` is a JSON object string handed straight to `Intl.NumberFormat`:
    /// `r#"{"style":"currency","currency":"USD"}"#`. An empty string is the
    /// defaults. The locale is the document's.
    pub fn number(value: f64, opts: &str) -> Option<Str> {
        format(op::INTL_NUM, value, opts, None)
    }

    /// The same, in an explicit locale.
    ///
    /// This exists because "the document's locale" is the wrong answer often
    /// enough to matter: a price list that shows each currency in its own
    /// locale renders `₹1,19,988` for `en-IN` and `$1,199.88` for `en-US`, and
    /// formatting both in the page's locale is visibly wrong money.
    pub fn number_in(locale: &str, value: f64, opts: &str) -> Option<Str> {
        format(op::INTL_NUM, value, opts, Some(locale))
    }

    /// `ms` is a Unix timestamp in milliseconds; `opts` goes to
    /// `Intl.DateTimeFormat`.
    pub fn date(ms: f64, opts: &str) -> Option<Str> {
        format(op::INTL_DATE, ms, opts, None)
    }

    pub fn date_in(locale: &str, ms: f64, opts: &str) -> Option<Str> {
        format(op::INTL_DATE, ms, opts, Some(locale))
    }

    fn format(code: u32, value: f64, opts: &str, locale: Option<&str>) -> Option<Str> {
        if let Some(locale) = locale {
            let (p, l) = s(locale);
            call(op::ARG, 1, p, l, 0, 0);
        }
        let (lo, hi) = super::host::f(value);
        let (p, l) = s(opts);
        host::take(call(code, lo, hi, p, l, 0)).map(Str::new)
    }

    /// The region the browser's language implies — `IN` for `en-IN` — which is
    /// how a page guesses a default currency. `None` when the browser has no
    /// `Intl.Locale`, so the caller keeps its own fallback.
    pub fn region() -> Option<Str> {
        host::take(super::call0(op::INTL_REGION)).map(Str::new)
    }

    /// The browser's IANA time zone, such as `Asia/Kolkata`.
    pub fn time_zone() -> Str {
        Str::new(host::take(super::call0(op::INTL_TZ)).unwrap_or_default())
    }
}

/// Publish `window.nextRust.<name>`, forwarding whatever it is called with to
/// a custom event on the window.
///
/// This exists because WebAssembly cannot create a JavaScript function, so a
/// module with a public API that page script calls — a `toast()` the host
/// application raises — needs the shim to make one. The forwarder is generic:
/// it is given a name and an event name and knows nothing about either, and
/// it assigns onto `window.nextRust` rather than replacing it, so anything
/// another layer published survives.
///
/// The argument arrives at the listener as a JSON string in `detail`, which
/// [`crate::Event::detail`] reads back. `String(value)` would have turned an
/// options object into `[object Object]`.
///
/// One honest difference from a hand-written JavaScript API: a forwarder
/// returns nothing. A JavaScript `toast()` that returned its element cannot be
/// reproduced this way.
pub fn install_forwarder(name: &str, event: &str) {
    let (p, l) = s(name);
    let (q, m) = s(event);
    call(op::INSTALL_FORWARDER, p, l, q, m, 0);
}

/// The wall clock, in Unix milliseconds.
///
/// Distinct from [`now`], which is the monotonic page-relative clock. A
/// timestamp sent to a server wants this one; measuring an elapsed duration
/// wants the other.
pub fn date_now() -> f64 {
    let mut slot = host::Slot::<4>::new();
    let ptr = slot.ptr();
    call1(op::TODAY, ptr);
    slot.get(3)
}

/// The local calendar date, as the browser reckons it: year, month (1 to 12)
/// and day. Doing this from a timestamp in Rust would need the browser's time
/// zone rules, which is the one part of a calendar that cannot be computed.
pub fn today() -> (i32, u32, u32) {
    let mut slot = host::Slot::<4>::new();
    let ptr = slot.ptr();
    call1(op::TODAY, ptr);
    (slot.get(0) as i32, slot.get(1) as u32, slot.get(2) as u32)
}

/// `performance.now()`, monotonic and page-relative.
pub fn now() -> f64 {
    let mut slot = host::Slot::<1>::new();
    let ptr = slot.ptr();
    call1(op::NOW, ptr);
    slot.get(0)
}

/// Write to the console. Kept out of release builds unless the `debug` feature
/// is on, because the message string is bytes in the module and readable to
/// anyone who downloads it.
pub fn log(message: &str) {
    let (p, l) = s(message);
    call(op::LOG, p, l, 0, 0, 0);
}

pub fn error(message: &str) {
    let (p, l) = s(message);
    call(op::LOG, p, l, 1, 0, 0);
}

/// Off the browser the shim is absent, so this is what every accessor above
/// answers. Kept as a test rather than a comment because it is the property
/// that lets the crate be tested at all without a browser.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_answer_empty_without_a_host() {
        assert!(location::href().is_empty());
        assert_eq!(location::param("q"), None);
        assert_eq!(storage::local_get("k"), None);
        assert_eq!(cookie::get("pp_sidebar"), None);
        assert_eq!(intl::number(1.0, ""), None);
        assert_eq!(now(), 0.0);
    }

    /// The shim's handler for `op::COOKIE` is
    /// `(a, b, c) => { if (a) { document.cookie = s(b, c); return 0 } return t(document.cookie) }`
    /// (crates/next-rust-build/src/wasm_shim.rs). So a write is the flag `1`
    /// in the first slot and the string in the second and third, and a read is
    /// a zero first slot. Nothing in either language checks that, and getting
    /// it wrong throws nothing on either side — `cookie::set` shipped passing
    /// its `(ptr, len)` in slots one and two, so the shim saw a truthy flag,
    /// decoded an empty string out of `(len, 0)` and assigned
    /// `document.cookie = ""`. A preference that quietly never persisted, with
    /// nothing in any log and no cargo test that could see it.
    #[test]
    fn writing_a_cookie_sends_the_flag_and_the_string_the_shim_reads() {
        let raw = "pp_sidebar=1; Path=/; Max-Age=31536000; SameSite=Lax";
        cookie::set(raw);
        let [a, b, c, ..] = crate::host::recorder::last(op::COOKIE).expect("cookie::set calls the host");
        assert_eq!(a, 1, "the first slot is the shim's write flag, not part of the string");
        assert_eq!((b, c), host::s(raw), "the shim decodes the string from the second and third slots");

        // …and a read leaves the flag clear, or it would assign instead.
        let _ = cookie::get("pp_sidebar");
        let [a, ..] = crate::host::recorder::last(op::COOKIE).expect("cookie::get calls the host");
        assert_eq!(a, 0, "a read must not look like a write");
    }

    /// Query-string decoding is ours, so it gets tested properly: it is the
    /// one piece of parsing in this module.
    #[test]
    fn query_parameters_decode() {
        assert_eq!(location::decode("a%20b"), "a b");
        assert_eq!(location::decode("a+b"), "a b");
        assert_eq!(location::decode("%E2%9C%93"), "✓");
        // A malformed escape is left alone rather than dropped.
        assert_eq!(location::decode("100%"), "100%");
        assert_eq!(location::decode("%zz"), "%zz");
        assert_eq!(location::decode(""), "");
    }
}
