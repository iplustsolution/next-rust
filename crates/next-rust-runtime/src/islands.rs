//! Island hydration and the declarative bindings.
//!
//! An `<nr-island>` carries its props as JSON in `data-props` and, when it has
//! one, the path of a module in `data-module`. Hydration imports that module
//! and hands it the element and the props; an island without a module, one
//! whose module refuses to load, and one whose module exports no `hydrate`
//! all fall through to [`bind_declarative`], which reads the `data-nr-*`
//! attributes below the element and keeps them in step with the props.
//!
//! Two rules live here rather than in the shim, and must stay here:
//!
//! * **The module guard.** `data-module` may be built from application data,
//!   so a module is imported only when it is a path on this origin. Another
//!   origin, a `//host`, or a scheme would run someone else's code in the
//!   page. The shim's `IMPORT_MODULE` imports whatever it is given: it is
//!   this function that decides what it may be given.
//! * **The registry key.** Inline delivery puts modules in `window.__nrm`
//!   under a key derived from the path, and that derivation is the
//!   framework's, not the browser's.
//!
//! Props are held as a [`Value`] tree rather than as text, because the
//! bindings write to them (`increment:count`, `set:open=false`) and the tree
//! has to go back out as JSON for `nextRust.action(url, state)`.

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use crate::dom::{self, Node};
use crate::host::{self, op};
use crate::nav::NavOptions;
use crate::{json, nav, num, task};

/// How deep a `data-props` tree may nest before it is refused. Props come off
/// the page, so the reader must not be able to run the stack out.
const MAX_DEPTH: u32 = 64;

// ---- the props tree ------------------------------------------------------------

/// A JSON value the bindings can read and write.
///
/// `Map` keeps its members in the order they were written, the way an object
/// does: `update()` walks nothing in key order, but `JSON.stringify(state)`
/// going back to a server action should read the way it arrived.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Vec<Value>),
    Map(Vec<(String, Value)>),
}

impl Value {
    /// An empty object: what `JSON.parse(el.dataset.props || "{}")` gives when
    /// there are no props, and what a `catch` falls back to.
    pub fn empty() -> Self {
        Self::Map(Vec::new())
    }

    /// `!!value`.
    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(on) => *on,
            Self::Num(n) => *n != 0.0 && !n.is_nan(),
            Self::Str(text) => !text.is_empty(),
            Self::List(_) | Self::Map(_) => true,
        }
    }

    /// `typeof value === "object"`, which `null` is not here: the source's
    /// object checks are all guarded by a `!= null` first.
    fn container(&self) -> bool {
        matches!(self, Self::List(_) | Self::Map(_))
    }

    /// `Number(value)`. An object is `NaN`, which is what the arithmetic ops
    /// do with one.
    fn number(&self) -> f64 {
        match self {
            Self::Null => 0.0,
            Self::Bool(on) => f64::from(u8::from(*on)),
            Self::Num(n) => *n,
            Self::Str(text) => num::number(text).unwrap_or(f64::NAN),
            Self::List(_) | Self::Map(_) => f64::NAN,
        }
    }

    /// `String(value)`: what goes into `el.value`, which is a DOMString.
    pub fn display(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(on) => if *on { "true" } else { "false" }.to_string(),
            Self::Num(n) => number_string(*n),
            Self::Str(text) => text.clone(),
            // `String([1,2])` is "1,2" and `String({})` is "[object Object]".
            Self::List(items) => {
                let mut out = String::new();
                for (at, item) in items.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    if !matches!(item, Self::Null) {
                        out.push_str(&item.display());
                    }
                }
                out
            }
            Self::Map(_) => "[object Object]".to_string(),
        }
    }

    /// `JSON.stringify(value)`.
    pub fn json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(on) => out.push_str(if *on { "true" } else { "false" }),
            // `JSON.stringify(NaN)` and `Infinity` are both `null`.
            Self::Num(n) if n.is_nan() || n.is_infinite() => out.push_str("null"),
            Self::Num(n) => out.push_str(&number_string(*n)),
            Self::Str(text) => out.push_str(&json::quote(text)),
            Self::List(items) => {
                out.push('[');
                for (at, item) in items.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    item.write_json(out);
                }
                out.push(']');
            }
            Self::Map(members) => {
                out.push('{');
                for (at, (key, value)) in members.iter().enumerate() {
                    if at > 0 {
                        out.push(',');
                    }
                    out.push_str(&json::quote(key));
                    out.push(':');
                    value.write_json(out);
                }
                out.push('}');
            }
        }
    }
}

/// `path.split(".").reduce((o, k) => (o == null ? o : o[k]), state)`.
///
/// `null` and `undefined` are both `None` here: every caller asks `v == null`
/// or `!!v`, and neither can tell them apart.
pub fn read<'a>(state: &'a Value, path: &str) -> Option<&'a Value> {
    let mut at = Some(state);
    for key in path.split('.') {
        at = match at {
            Some(Value::Map(members)) => members.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            Some(Value::List(items)) => index(key).and_then(|i| items.get(i)),
            // A member of a primitive, of `null`, or of nothing at all.
            _ => None,
        };
    }
    at.filter(|value| !matches!(value, Value::Null))
}

/// ```js
/// const keys = path.split(".");
/// const last = keys.pop();
/// keys.reduce((o, k) => (o[k] = o[k] && typeof o[k] === "object" ? o[k] : {}), state)[last] = value;
/// ```
///
/// An intermediate key that is missing, `null` or a primitive becomes an
/// empty object, exactly as the `reduce` does.
pub fn write(state: &mut Value, path: &str, value: Value) {
    let mut keys = path.split('.');
    let Some(mut pending) = keys.next() else { return };
    let mut at = state;
    for key in keys {
        let Some(next) = slot(at, pending) else { return };
        if !next.container() {
            *next = Value::empty();
        }
        at = next;
        pending = key;
    }
    if let Some(place) = slot(at, pending) {
        *place = value;
    }
}

/// The place a key names, created when it is not there. `None` for a
/// primitive, where an assignment in the source goes nowhere.
fn slot<'a>(value: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    match value {
        Value::Map(members) => {
            if !members.iter().any(|(name, _)| name == key) {
                members.push((key.to_owned(), Value::Null));
            }
            members.iter_mut().find(|(name, _)| name == key).map(|(_, value)| value)
        }
        Value::List(items) => {
            let at = index(key)?;
            // `arr[5] = x` on a shorter array fills the gap with holes, which
            // read back as `undefined` and stringify as `null`.
            while items.len() <= at {
                items.push(Value::Null);
            }
            items.get_mut(at)
        }
        _ => None,
    }
}

/// An array index, which is the only kind of key an array answers to.
fn index(key: &str) -> Option<usize> {
    if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut at = 0usize;
    for byte in key.bytes() {
        at = at.checked_mul(10)?.checked_add(usize::from(byte - b'0'))?;
    }
    Some(at)
}

/// `Object.assign(state, result)` for the members of one object.
fn assign(state: &mut Value, members: Vec<(String, Value)>) {
    let Value::Map(into) = state else { return };
    for (key, value) in members {
        match into.iter_mut().find(|(name, _)| *name == key) {
            Some(place) => place.1 = value,
            None => into.push((key, value)),
        }
    }
}

/// `expr.startsWith("!") ? !read(expr.slice(1).trim()) : !!read(expr.trim())`.
pub fn truthy(state: &Value, expr: &str) -> bool {
    match expr.strip_prefix('!') {
        Some(rest) => !read(state, rest.trim()).is_some_and(Value::truthy),
        None => read(state, expr.trim()).is_some_and(Value::truthy),
    }
}

/// `v == null ? "" : typeof v === "object" ? JSON.stringify(v) : String(v)`.
fn shown(value: Option<&Value>) -> String {
    match value {
        None => String::new(),
        Some(value) if value.container() => value.json(),
        Some(value) => value.display(),
    }
}

// ---- reading JSON ----------------------------------------------------------------

/// `JSON.parse(text)`, or `None` where it would have thrown.
///
/// Whole values only, the way `JSON.parse` takes them: trailing text, a
/// leading `+`, and `Infinity` are all refused, so `parse` in the bindings
/// keeps them as the strings they were written as.
pub fn parse_json(text: &str) -> Option<Value> {
    let mut reader = Reader { text, at: 0 };
    let value = reader.value(MAX_DEPTH)?;
    reader.whitespace();
    (reader.at == text.len()).then_some(value)
}

/// `JSON.parse(el.dataset.props || "{}")` with the source's `catch {}`.
pub fn parse_props(text: &str) -> Value {
    parse_json(if text.is_empty() { "{}" } else { text }).unwrap_or_else(Value::empty)
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.at += 1;
        }
        found
    }

    fn word(&mut self, text: &str) -> bool {
        let found = self.text.get(self.at..).is_some_and(|rest| rest.starts_with(text));
        if found {
            self.at += text.len();
        }
        found
    }

    /// The string that starts at the cursor, with its escapes resolved.
    fn string(&mut self) -> Option<String> {
        let start = self.at;
        let bytes = self.text.as_bytes();
        let mut at = start + 1;
        let end = loop {
            match *bytes.get(at)? {
                // A backslash always takes the byte after it with it.
                b'\\' => at += 2,
                b'"' => break at + 1,
                _ => at += 1,
            }
        };
        // `get` is `None` when a `\` skipped into the middle of a character.
        let raw = self.text.get(start..end)?;
        self.at = end;
        unquote(raw)
    }

    fn value(&mut self, depth: u32) -> Option<Value> {
        if depth == 0 {
            return None;
        }
        self.whitespace();
        match self.peek()? {
            b'"' => self.string().map(Value::Str),
            b'{' => {
                self.at += 1;
                let mut members: Vec<(String, Value)> = Vec::new();
                self.whitespace();
                if self.eat(b'}') {
                    return Some(Value::Map(members));
                }
                loop {
                    self.whitespace();
                    if self.peek()? != b'"' {
                        return None;
                    }
                    let key = self.string()?;
                    self.whitespace();
                    if !self.eat(b':') {
                        return None;
                    }
                    let value = self.value(depth - 1)?;
                    // The last of two members with the same name wins, which
                    // is the object `JSON.parse` builds.
                    match members.iter_mut().find(|(name, _)| *name == key) {
                        Some(member) => member.1 = value,
                        None => members.push((key, value)),
                    }
                    self.whitespace();
                    if self.eat(b',') {
                        continue;
                    }
                    return self.eat(b'}').then_some(Value::Map(members));
                }
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                self.whitespace();
                if self.eat(b']') {
                    return Some(Value::List(items));
                }
                loop {
                    items.push(self.value(depth - 1)?);
                    self.whitespace();
                    if self.eat(b',') {
                        continue;
                    }
                    return self.eat(b']').then_some(Value::List(items));
                }
            }
            b't' => self.word("true").then_some(Value::Bool(true)),
            b'f' => self.word("false").then_some(Value::Bool(false)),
            b'n' => self.word("null").then_some(Value::Null),
            // JSON numbers, which is a narrower grammar than `parseFloat`:
            // a leading `+` and `Infinity` are not numbers here.
            b'-' | b'0'..=b'9' => {
                let rest = self.text.get(self.at..)?;
                if rest.strip_prefix('-').unwrap_or(rest).starts_with("Infinity") {
                    return None;
                }
                let (value, used) = num::parse_prefix(rest)?;
                self.at += used;
                Some(Value::Num(value))
            }
            _ => None,
        }
    }
}

/// The text of a JSON string, quotes and escapes and all, as a `String`.
///
/// It borrows [`json`]'s reader rather than keeping a second copy of the
/// escape rules (`😀` and its unpaired halves are fiddly and
/// already tested there) by asking it for a one-member object.
fn unquote(raw: &str) -> Option<String> {
    let mut wrapper = String::with_capacity(raw.len() + 6);
    wrapper.push_str("{\"v\":");
    wrapper.push_str(raw);
    wrapper.push('}');
    json::Object::new(&wrapper)?.str("v")
}

// ---- numbers -----------------------------------------------------------------------

/// `String(n)` for the numbers a counter, an index and a bound field hold.
///
/// Whole numbers — which is nearly all of them — come out exactly. A fraction
/// goes through [`num::to_string`] to twelve places, so a value JavaScript
/// would print as `0.30000000000000004` reads `0.3` here.
pub(crate) fn number_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    const WHOLE: f64 = 9.007_199_254_740_992e15; // 2^53, past which a double counts in twos
    if (-WHOLE..=WHOLE).contains(&value) && value as i64 as f64 == value {
        let mut out = String::new();
        let mut whole = value as i64;
        if whole < 0 {
            out.push('-');
            whole = -whole;
        }
        push_digits(&mut out, whole as u64);
        return out;
    }
    num::to_string(value, 12)
}

fn push_digits(out: &mut String, value: u64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    let mut left = value;
    loop {
        at -= 1;
        digits[at] = b'0' + (left % 10) as u8;
        left /= 10;
        if left == 0 || at == 0 {
            break;
        }
    }
    out.push_str(core::str::from_utf8(&digits[at..]).unwrap_or("0"));
}

// ---- the statements a `data-nr-on-<event>` attribute carries ---------------------------

/// Which of the three arithmetic ops a statement names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arith {
    Increment,
    Decrement,
    Toggle,
}

/// One statement of a `data-nr-on-<event>` attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `prevent`
    Prevent,
    /// `set:open=false` — the path and the value as it was written.
    Set(String, String),
    /// `action:/a/b -> result` — the URL and where to put what comes back.
    Action(String, Option<String>),
    /// `navigate:/x`
    Navigate(String),
    /// `increment:count,2` — the op, the path, and the amount as written.
    Arith(Arith, String, Option<String>),
    /// An op this runtime has no name for, which does nothing at all.
    Nothing,
}

/// Split a `data-nr-on-<event>` attribute into its statements.
///
/// Pure, so it is tested for what the source does with the odd ones: a
/// statement with no colon is an op with an empty argument, a trailing `;`
/// leaves an empty statement behind, and `set` with no `=` slices around an
/// index of `-1` the way JavaScript's `slice` does.
pub fn parse_statements(source: &str) -> Vec<Stmt> {
    source.split(';').map(statement).collect()
}

fn statement(stmt: &str) -> Stmt {
    let (op, arg) = match stmt.find(':') {
        Some(at) => (stmt[..at].trim(), stmt[at + 1..].trim()),
        None => (stmt.trim(), ""),
    };
    match op {
        "prevent" => Stmt::Prevent,
        "set" => {
            let (path, value) = match arg.find('=') {
                Some(at) => (arg[..at].trim(), arg[at + 1..].trim()),
                // `arg.slice(0, -1)` drops the last character and
                // `arg.slice(0)` keeps the whole of it.
                None => (without_last(arg).trim(), arg),
            };
            Stmt::Set(path.to_owned(), value.to_owned())
        }
        "action" => {
            let mut parts = arg.split("->").map(str::trim);
            let url = parts.next().unwrap_or("").to_owned();
            // `const [url, target] = ...`: a second arrow is nobody's.
            let target = parts.next().map(ToOwned::to_owned);
            Stmt::Action(url, target)
        }
        "navigate" => Stmt::Navigate(arg.to_owned()),
        "increment" | "decrement" | "toggle" => {
            let kind = match op {
                "increment" => Arith::Increment,
                "decrement" => Arith::Decrement,
                _ => Arith::Toggle,
            };
            // `const [k, n] = arg.split(",")`: `n` is undefined without one,
            // and is not trimmed — `Number` trims it later anyway.
            let mut parts = arg.splitn(2, ',');
            let path = parts.next().unwrap_or("").trim().to_owned();
            Stmt::Arith(kind, path, parts.next().map(ToOwned::to_owned))
        }
        _ => Stmt::Nothing,
    }
}

/// `text.slice(0, -1)`: everything but the last character, and `""` when
/// there is not one.
fn without_last(text: &str) -> &str {
    match text.chars().next_back() {
        Some(last) => text.get(..text.len() - last.len_utf8()).unwrap_or(""),
        None => text,
    }
}

/// `try { JSON.parse(v) } catch { return v }`.
fn parse_or_text(raw: &str) -> Value {
    parse_json(raw).unwrap_or_else(|| Value::Str(raw.to_owned()))
}

// ---- hydration ---------------------------------------------------------------------

crate::local! {
    /// `hydrated`, the source's `WeakSet`: the islands this document has
    /// already bound, so a second pass over a layout that stayed on screen
    /// does not bind it twice.
    ///
    /// The key is the handle the island was first seen under and the value is
    /// the island itself, because the shim hands out a fresh handle every
    /// time the same element crosses the ABI: membership is asked of the DOM,
    /// by identity, and the key only keeps the entries apart. An island that
    /// has left the document is swept, which is what makes the set weak.
    static HYDRATED: RefCell<BTreeMap<u32, Node>> = RefCell::new(BTreeMap::new());
}

fn hydrated(island: &Node) -> bool {
    HYDRATED.with(|set| set.borrow().values().any(|known| dom::same(known, island)))
}

fn remember(island: &Node) {
    HYDRATED.with(|set| set.borrow_mut().insert(island.raw(), island.share()));
}

/// Forget the islands that are no longer in the document, which is what the
/// source gets from a `WeakSet` for nothing.
fn sweep() {
    HYDRATED.with(|set| set.borrow_mut().retain(|_, island| dom::is_connected(island)));
}

/// Hydrate every island in the document: what the runtime does when it starts
/// and after every navigation.
pub fn hydrate_document() {
    let document = dom::document();
    hydrate(&document);
}

/// Hydrate every island below `root`.
pub fn hydrate(root: &Node) {
    let root = root.share();
    task::spawn(async move { hydrate_islands(root).await });
}

async fn hydrate_islands(root: Node) {
    sweep();
    for island in dom::query_all(&root, "nr-island") {
        if hydrated(&island) {
            continue;
        }
        remember(&island);
        let props = parse_props(&dom::dataset(&island, "data-props").unwrap_or_default());
        let module = dom::dataset(&island, "data-module").unwrap_or_default();
        if !module.is_empty() {
            match load_module(&module).await {
                // The module refused the guard: the console already says so,
                // and the island falls back to the declarative bindings.
                Ok(None) => {}
                Ok(Some(loaded)) => {
                    // The props go over as the text of the tree that was
                    // parsed, never as the attribute: the shim calls
                    // `JSON.parse` on what it is given, and an attribute that
                    // did not parse here would throw there.
                    let (answer, reason) = module_hydrate(&loaded, &island, &props.json()).await;
                    match answer {
                        MODULE_HYDRATED => continue,
                        MODULE_THREW => failed(&island, reason),
                        // The module has no `hydrate` export: the bindings.
                        _ => {}
                    }
                }
                Err(reason) => failed(&island, reason),
            }
        }
        bind_declarative(&island, props);
    }
}

/// `MODULE_HYDRATE`'s answer: the module hydrated the island.
const MODULE_HYDRATED: u32 = 0;
/// It threw, or its `hydrate` rejected.
const MODULE_THREW: u32 = 1;

/// `console.error(`[next-rust] island ${…} failed to load`, err)`. The ABI's
/// `console.error` takes one string, so the reason follows on its own line.
fn failed(island: &Node, reason: Option<String>) {
    let name = dom::dataset(island, "data-component")
        .filter(|component| !component.is_empty())
        .or_else(|| dom::dataset(island, "data-module"))
        .unwrap_or_default();
    let mut message = String::from("[next-rust] island ");
    message.push_str(&name);
    message.push_str(" failed to load");
    dom::console_error(&message);
    if let Some(reason) = reason.filter(|reason| !reason.is_empty()) {
        dom::console_error(&reason);
    }
}

/// `data-module` may be built from application data, so a module is imported
/// only when it is a path on this origin: another origin (or `//host`, or a
/// scheme) would run someone else's code in the page.
///
/// This is the framework's rule and it stays in Rust. The shim imports
/// whatever it is handed.
fn module_url(src: &str) -> Option<String> {
    let bytes = src.as_bytes();
    // `/^\/[^/\\]/`: an absolute path whose second character is neither a
    // second slash nor a backslash.
    let path = bytes.first() == Some(&b'/') && !matches!(bytes.get(1), None | Some(b'/') | Some(b'\\'));
    if path {
        let origin = dom::origin();
        if let Some(url) = dom::Url::parse(src, Some(&origin))
            && url.origin() == origin
        {
            return Some(url.href());
        }
    }
    let mut message = String::from("[next-rust] island module refused: ");
    message.push_str(src);
    message.push_str(" is not a same-origin path starting with \"/\"");
    dom::console_error(&message);
    None
}

/// Inline delivery (`[build] inline_assets`) puts modules in the page's own
/// registry under a key derived from the path: the leading `/_next-rust/` and
/// any query are not part of it.
fn registry_key(src: &str) -> &str {
    let rest = src.strip_prefix("/_next-rust/").unwrap_or(src);
    rest.split('?').next().unwrap_or(rest)
}

/// The module for `src`: the page's registry when it has it, the network
/// otherwise. `Ok(None)` is a module the guard refused.
async fn load_module(src: &str) -> Result<Option<Node>, Option<String>> {
    let Some(url) = module_url(src) else { return Ok(None) };
    let key = registry_key(src);
    let (p, l) = host::s(key);
    let slot = task::pending();
    if host::flag(op::REGISTRY_HAS, p, l, 0, 0, 0) {
        host::call(op::REGISTRY_LOAD, p, l, slot.token(), 0, 0);
    } else {
        let (p, l) = host::s(&url);
        host::call(op::IMPORT_MODULE, p, l, slot.token(), 0, 0);
    }
    let settled = slot.await;
    if settled.failed() { Err(settled.text) } else { Ok(settled.node()) }
}

/// `await mod.hydrate(el, props)`, when the module has one. The answer is
/// [`MODULE_HYDRATED`], [`MODULE_THREW`] with what it threw, or `2` for a
/// module that exports no `hydrate` at all, which falls through to the
/// declarative bindings.
async fn module_hydrate(module: &Node, island: &Node, props: &str) -> (u32, Option<String>) {
    let slot = task::pending();
    let (p, l) = host::s(props);
    host::call(op::MODULE_HYDRATE, module.raw(), island.raw(), p, l, slot.token());
    let settled = slot.await;
    (settled.flag, settled.text)
}

// ---- the declarative bindings ---------------------------------------------------------

/// Declarative behaviour: `data-nr-text`, `data-nr-show`, `data-nr-bind`,
/// `data-nr-class-<name>` and `data-nr-on-<event>="increment:count; set:open=false"`.
///
/// The props are the state. Every listener holds the same tree, and every
/// statement that changes it is followed by one `update()` over the island.
pub fn bind_declarative(root: &Node, props: Value) {
    let root = Rc::new(root.share());
    let state = Rc::new(RefCell::new(props));

    // Which events anything below the island listens for.
    let mut types: Vec<String> = Vec::new();
    for element in dom::query_all(&root, "*") {
        for (name, _) in dom::attributes(&element) {
            if let Some(kind) = name.strip_prefix("data-nr-on-")
                && !types.iter().any(|seen| seen == kind)
            {
                types.push(kind.to_owned());
            }
        }
    }

    for kind in types {
        let mut selector = String::from("[data-nr-on-");
        selector.push_str(&kind);
        selector.push(']');
        let mut attribute = String::from("data-nr-on-");
        attribute.push_str(&kind);
        let island = root.clone();
        let props = state.clone();
        let listener = task::on(move |event| {
            let Some(event) = event else { return };
            let Some(target) = dom::prop_node(&event, "target") else { return };
            let Some(element) = dom::closest(&target, &selector) else { return };
            if !dom::contains(&island, Some(&element)) {
                return;
            }
            let steps = parse_statements(&dom::attr_or_empty(&element, &attribute));
            let root = island.clone();
            let state = props.clone();
            // The event is kept for the statements, which run in this turn up
            // to the first `action` exactly as the source's `for` loop does.
            let event = event.share();
            task::spawn(async move { run(state, root, steps, event).await });
        });
        dom::add_listener(Some(&root), &kind, listener, 0);
    }

    // `data-nr-bind` both ways: what the visitor types goes back into the props.
    let bound_root = root.clone();
    let bound_state = state.clone();
    let listener = task::on(move |event| {
        let Some(event) = event else { return };
        let Some(element) = dom::prop_node(&event, "target") else { return };
        let Some(path) = dom::dataset(&element, "data-nr-bind").filter(|path| !path.is_empty()) else { return };
        let kind = dom::prop_str(&element, "type").unwrap_or_default();
        let value = match kind.as_str() {
            "checkbox" => Value::Bool(dom::prop_bool(&element, "checked")),
            "number" => Value::Num(num::number(&field(&element)).unwrap_or(f64::NAN)),
            _ => Value::Str(field(&element)),
        };
        write(&mut bound_state.borrow_mut(), &path, value);
        update(&bound_root, &bound_state.borrow());
    });
    dom::add_listener(Some(&root), "input", listener, 0);

    update(&root, &state.borrow());
}

fn field(element: &Node) -> String {
    dom::prop_str(element, "value").unwrap_or_default()
}

/// One attribute's statements, in order, with `update()` after the last.
async fn run(state: Rc<RefCell<Value>>, root: Rc<Node>, steps: Vec<Stmt>, event: Node) {
    for step in steps {
        match step {
            Stmt::Prevent => dom::prevent_default(&event),
            Stmt::Nothing => {}
            Stmt::Set(path, raw) => {
                let value = parse_or_text(&raw);
                write(&mut state.borrow_mut(), &path, value);
            }
            Stmt::Navigate(href) => {
                // The source does not await this one either.
                task::spawn(async move {
                    let _ = nav::navigate(&href, NavOptions::default()).await;
                });
            }
            Stmt::Arith(kind, path, by) => {
                let mut state = state.borrow_mut();
                // `Number(read(k) || 0)`, so a falsy member counts as zero.
                let current = read(&state, &path).filter(|value| value.truthy()).map_or(0.0, Value::number);
                let value = match kind {
                    Arith::Toggle => Value::Bool(!read(&state, &path).is_some_and(Value::truthy)),
                    Arith::Increment => Value::Num(current + amount(by.as_deref())),
                    Arith::Decrement => Value::Num(current - amount(by.as_deref())),
                };
                write(&mut state, &path, value);
            }
            Stmt::Action(url, target) => {
                // The borrow ends before the await: a listener that fires
                // while the action is in the air must be able to read it.
                let input = state.borrow().json();
                match crate::forms::action(&url, &input).await {
                    Ok(answer) => {
                        // A redirect resolves with nothing and the page is
                        // already on its way.
                        let value = if answer.redirected {
                            Value::Null
                        } else {
                            parse_json(&answer.data).unwrap_or(Value::Null)
                        };
                        let mut state = state.borrow_mut();
                        match target {
                            Some(target) => write(&mut state, &target, value),
                            None => {
                                if let Value::Map(members) = value {
                                    assign(&mut state, members);
                                }
                            }
                        }
                    }
                    Err(error) => write(&mut state.borrow_mut(), "error", Value::Str(error.message)),
                }
            }
        }
    }
    update(&root, &state.borrow());
}

/// `Number(n || 1)`: the step of an `increment` or a `decrement`.
fn amount(by: Option<&str>) -> f64 {
    // `Number("")` is 0, but `n || 1` turns that into 1, as it does `None`.
    match by.filter(|text| !text.is_empty()) {
        None => 1.0,
        Some(text) => num::number(text).unwrap_or(f64::NAN),
    }
}

/// Put every binding below the island in step with the props.
pub fn update(root: &Node, state: &Value) {
    for element in dom::query_all(root, "[data-nr-text]") {
        let path = dom::attr_or_empty(&element, "data-nr-text");
        let text = shown(read(state, &path));
        dom::set_text_content(&element, &text);
    }
    for element in dom::query_all(root, "[data-nr-show]") {
        let expr = dom::attr_or_empty(&element, "data-nr-show");
        dom::set_prop_bool(&element, "hidden", !truthy(state, &expr));
    }
    let active = dom::active_element();
    for element in dom::query_all(root, "[data-nr-bind]") {
        let path = dom::attr_or_empty(&element, "data-nr-bind");
        let value = read(state, &path);
        if dom::prop_str(&element, "type").as_deref() == Some("checkbox") {
            dom::set_prop_bool(&element, "checked", value.is_some_and(Value::truthy));
        } else if !active.as_ref().is_some_and(|focused| dom::same(focused, &element)) {
            // `el.value = v == null ? "" : v`, where the DOM makes a string
            // of whatever it is given.
            let text = value.map_or_else(String::new, Value::display);
            dom::set_prop_str(&element, "value", &text);
        }
    }
    for element in dom::query_all(root, "*") {
        for (name, expr) in dom::attributes(&element) {
            if let Some(class) = name.strip_prefix("data-nr-class-") {
                dom::toggle_class(&element, class, truthy(state, &expr));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn props(text: &str) -> Value {
        parse_props(text)
    }

    // ---- the statement parser ------------------------------------------------

    #[test]
    fn the_statements_the_source_documents() {
        assert_eq!(parse_statements("increment:count"), vec![Stmt::Arith(Arith::Increment, "count".into(), None)]);
        assert_eq!(parse_statements("set:open=false"), vec![Stmt::Set("open".into(), "false".into())]);
        assert_eq!(
            parse_statements("prevent; toggle:x"),
            vec![Stmt::Prevent, Stmt::Arith(Arith::Toggle, "x".into(), None)]
        );
        assert_eq!(parse_statements("action:/a/b -> result"), vec![Stmt::Action("/a/b".into(), Some("result".into()))]);
        assert_eq!(parse_statements("navigate:/x"), vec![Stmt::Navigate("/x".into())]);
    }

    #[test]
    fn whitespace_and_several_statements() {
        assert_eq!(
            parse_statements("  prevent ;  set: a.b = 3 ; decrement:n,2 "),
            vec![
                Stmt::Prevent,
                Stmt::Set("a.b".into(), "3".into()),
                Stmt::Arith(Arith::Decrement, "n".into(), Some("2".into()))
            ]
        );
        // An action with no target, and a second arrow nobody reads.
        assert_eq!(parse_statements("action:/go"), vec![Stmt::Action("/go".into(), None)]);
        assert_eq!(parse_statements("action:/go -> a -> b"), vec![Stmt::Action("/go".into(), Some("a".into()))]);
    }

    #[test]
    fn malformed_statements_do_what_the_source_does() {
        // No colon at all: an op with an empty argument, which names nothing.
        assert_eq!(parse_statements("increment"), vec![Stmt::Arith(Arith::Increment, String::new(), None)]);
        assert_eq!(parse_statements("wat"), vec![Stmt::Nothing]);
        assert_eq!(parse_statements("wat:arg"), vec![Stmt::Nothing]);
        // A trailing `;` leaves an empty statement behind, and an empty
        // attribute is one empty statement.
        assert_eq!(parse_statements("prevent;"), vec![Stmt::Prevent, Stmt::Nothing]);
        assert_eq!(parse_statements(""), vec![Stmt::Nothing]);
        assert_eq!(parse_statements(";"), vec![Stmt::Nothing, Stmt::Nothing]);
        // `set` with no `=`: `slice(0, -1)` and `slice(0)`, as written.
        assert_eq!(parse_statements("set:open"), vec![Stmt::Set("ope".into(), "open".into())]);
        assert_eq!(parse_statements("set:"), vec![Stmt::Set(String::new(), String::new())]);
        // `prevent` ignores whatever follows it.
        assert_eq!(parse_statements("prevent:x"), vec![Stmt::Prevent]);
        // A colon inside the value stays in it.
        assert_eq!(parse_statements("set:url=http://x"), vec![Stmt::Set("url".into(), "http://x".into())]);
    }

    #[test]
    fn a_step_that_is_not_a_number_is_kept_as_written() {
        assert_eq!(
            parse_statements("increment:n, 4 "),
            vec![Stmt::Arith(Arith::Increment, "n".into(), Some(" 4".into()))]
        );
        assert_eq!(
            parse_statements("increment:n,"),
            vec![Stmt::Arith(Arith::Increment, "n".into(), Some(String::new()))]
        );
        assert_eq!(amount(None), 1.0);
        assert_eq!(amount(Some("")), 1.0);
        assert_eq!(amount(Some(" 4 ")), 4.0);
        assert!(amount(Some("x")).is_nan());
    }

    // ---- the props tree --------------------------------------------------------

    #[test]
    fn reading_a_dotted_path() {
        let state = props(r#"{"a":{"b":[1,{"c":"x"}]},"n":null,"z":0}"#);
        assert_eq!(read(&state, "a.b.0"), Some(&Value::Num(1.0)));
        assert_eq!(read(&state, "a.b.1.c"), Some(&Value::Str("x".into())));
        assert_eq!(read(&state, "a.missing.deep"), None);
        assert_eq!(read(&state, "n"), None);
        assert_eq!(read(&state, "z"), Some(&Value::Num(0.0)));
        assert_eq!(read(&state, ""), None);
    }

    #[test]
    fn writing_builds_the_objects_on_the_way() {
        let mut state = Value::empty();
        write(&mut state, "a.b.c", Value::Num(1.0));
        assert_eq!(state.json(), r#"{"a":{"b":{"c":1}}}"#);
        // A primitive in the way is replaced, as the source's `reduce` does.
        write(&mut state, "a.b", Value::Num(2.0));
        write(&mut state, "a.b.c", Value::Bool(true));
        assert_eq!(state.json(), r#"{"a":{"b":{"c":true}}}"#);
        // An existing object is kept rather than rebuilt.
        write(&mut state, "a.d", Value::Str("x".into()));
        assert_eq!(state.json(), r#"{"a":{"b":{"c":true},"d":"x"}}"#);
    }

    #[test]
    fn truthiness_and_the_bang_prefix() {
        let state = props(r#"{"on":true,"off":false,"empty":"","zero":0,"text":"no","list":[],"map":{}}"#);
        assert!(truthy(&state, "on"));
        assert!(!truthy(&state, "off"));
        assert!(!truthy(&state, " empty "));
        assert!(!truthy(&state, "zero"));
        assert!(truthy(&state, "text"));
        assert!(truthy(&state, "list"));
        assert!(truthy(&state, "map"));
        assert!(truthy(&state, "!off"));
        assert!(truthy(&state, "! missing"));
        assert!(!truthy(&state, "!on"));
    }

    #[test]
    fn what_a_binding_puts_on_screen() {
        let state = props(r#"{"s":"hi","n":2.5,"b":true,"o":{"a":1},"l":[1,"x"],"z":null}"#);
        assert_eq!(shown(read(&state, "s")), "hi");
        assert_eq!(shown(read(&state, "n")), "2.5");
        assert_eq!(shown(read(&state, "b")), "true");
        assert_eq!(shown(read(&state, "o")), r#"{"a":1}"#);
        assert_eq!(shown(read(&state, "l")), r#"[1,"x"]"#);
        assert_eq!(shown(read(&state, "z")), "");
        assert_eq!(shown(read(&state, "missing")), "");
    }

    #[test]
    fn the_tree_goes_back_out_as_json() {
        for text in [r#"{"a":[1,2,{"b":null}],"c":"x\ny","d":true}"#, "{}", r#"{"e":[]}"#] {
            assert_eq!(parse_props(text).json(), text);
        }
        // Escapes survive the round trip.
        let state = props(r#"{"m":"a\"b\\cé"}"#);
        assert_eq!(state.json(), r#"{"m":"a\"b\\cé"}"#);
        assert_eq!(read(&state, "m"), Some(&Value::Str("a\"b\\cé".into())));
    }

    #[test]
    fn props_that_do_not_parse_are_an_empty_object() {
        for text in ["", "{", "not json", r#"{"a":1} trailing"#, "{a:1}", r#"{"a":+1}"#, "Infinity"] {
            assert_eq!(parse_props(text), Value::empty(), "{text}");
        }
        assert_eq!(parse_props("  {  }  "), Value::empty());
        // A hostile tree is refused rather than run off the stack.
        let deep = alloc::string::String::from("[").repeat(200);
        assert_eq!(parse_props(&deep), Value::empty());
    }

    #[test]
    fn the_last_of_two_members_with_one_name_wins() {
        assert_eq!(props(r#"{"a":1,"a":2}"#).json(), r#"{"a":2}"#);
    }

    #[test]
    fn numbers_read_and_write_the_way_javascript_does() {
        assert_eq!(number_string(0.0), "0");
        assert_eq!(number_string(-0.0), "0");
        assert_eq!(number_string(42.0), "42");
        assert_eq!(number_string(-3.0), "-3");
        assert_eq!(number_string(2.5), "2.5");
        assert_eq!(number_string(f64::NAN), "NaN");
        assert_eq!(number_string(f64::INFINITY), "Infinity");
        assert_eq!(number_string(f64::NEG_INFINITY), "-Infinity");
        // `JSON.stringify` has no way to write either of the last two.
        assert_eq!(Value::Num(f64::NAN).json(), "null");
        assert_eq!(Value::Num(f64::INFINITY).json(), "null");
    }

    #[test]
    fn the_registry_key_and_the_module_guard() {
        assert_eq!(registry_key("/_next-rust/counter.js"), "counter.js");
        assert_eq!(registry_key("/_next-rust/counter.js?v=2"), "counter.js");
        assert_eq!(registry_key("/islands/x.js"), "/islands/x.js");
        assert_eq!(registry_key("/_next-rust/a/b.js?x=1&y=2"), "a/b.js");
        // Off the browser every DOM call answers nothing, so the guard can
        // only be tested for what it refuses outright.
        for refused in ["", "//evil.example/x.js", "https://evil.example/x.js", "/", "/\\evil", "x.js", "./x.js"] {
            assert_eq!(module_url(refused), None, "{refused}");
        }
    }

    #[test]
    fn assigning_what_an_action_answered() {
        let mut state = props(r#"{"a":1,"b":2}"#);
        let Value::Map(members) = props(r#"{"b":3,"c":4}"#) else { panic!("an object") };
        assign(&mut state, members);
        assert_eq!(state.json(), r#"{"a":1,"b":3,"c":4}"#);
    }

    #[test]
    fn an_array_index_is_the_only_key_an_array_answers_to() {
        let mut state = props(r#"{"l":[1,2]}"#);
        write(&mut state, "l.1", Value::Num(9.0));
        assert_eq!(state.json(), r#"{"l":[1,9]}"#);
        // A gap fills with holes, which stringify as null.
        write(&mut state, "l.4", Value::Num(5.0));
        assert_eq!(state.json(), r#"{"l":[1,9,null,null,5]}"#);
        // A name is not an index.
        write(&mut state, "l.x", Value::Num(0.0));
        assert_eq!(state.json(), r#"{"l":[1,9,null,null,5]}"#);
    }
}
