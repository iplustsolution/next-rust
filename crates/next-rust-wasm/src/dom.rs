//! Elements, lists and any other JavaScript value, as handles.
//!
//! A handle is an index into a table the shim keeps. It is **not** necessarily
//! an element: the same table holds `NodeList`s, `Event`s, `Response`s,
//! `File`s, `FormData`s and module namespaces, which is what lets a dozen
//! capabilities be expressed with the generic property and call operations
//! rather than an opcode each.
//!
//! Handle `0` is "nothing", and every operation on it is a no-op that answers
//! nothing. That is deliberate: a query that found no element returns a handle
//! you can keep calling methods on, so island code reads as a straight line
//! instead of a staircase of `Option`s.

use crate::host::{self, NONE, call, call_s, call_s0, call_ss, call0, call1, call2, op, s};
use crate::text::Str;

/// Any JavaScript value the shim is holding for us.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Val(pub(crate) u32);

/// An element. The same handle space as [`Val`], with the DOM sugar on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct El(pub(crate) u32);

/// A `NodeList` or array.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ElList(pub(crate) u32);

/// What `getBoundingClientRect` answered.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The layout and scroll numbers that are properties rather than geometry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Metric {
    OffsetWidth = 0,
    OffsetHeight = 1,
    ClientWidth = 2,
    ClientHeight = 3,
    ScrollWidth = 4,
    ScrollHeight = 5,
    ScrollLeft = 6,
    ScrollTop = 7,
    InnerWidth = 8,
    InnerHeight = 9,
    ScrollX = 10,
    ScrollY = 11,
}

/// Which global object [`crate::document`] and friends name.
#[repr(u32)]
enum Root {
    Document = 0,
    DocumentElement = 1,
    Body = 2,
    Head = 3,
    Window = 4,
    Active = 5,
}

/// The namespaces an island builds elements in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Namespace {
    Svg = 0,
    MathMl = 1,
    XLink = 2,
}

/// What [`Val::keys`] should collect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Keys {
    /// `Object.keys(value)`
    Own = 0,
    /// `[...el.attributes]`
    Attributes = 1,
    /// `[...value.keys()]`, for `FormData` and `URLSearchParams`
    MapKeys = 2,
    /// `[...value]`, for anything iterable
    Iterable = 3,
}

/// What `typeof` answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JsType {
    Undefined,
    Null,
    Bool,
    Number,
    String,
    Object,
    Function,
}

impl Val {
    pub const NULL: Val = Val(0);

    /// Wrap a raw handle the shim passed back. Only glue should need this.
    #[doc(hidden)]
    pub fn from_handle(handle: u32) -> Val {
        Val(handle)
    }

    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn is_some(self) -> bool {
        self.0 != 0
    }

    /// Read this handle as an element. Handles are untyped, so this only
    /// changes which methods are in scope.
    pub fn el(self) -> El {
        El(self.0)
    }

    /// Read this handle as a list.
    pub fn list(self) -> ElList {
        ElList(self.0)
    }

    pub fn get(self, name: &str) -> Val {
        Val(call_s(op::PROP_H, self.0, name))
    }

    pub fn has(self, name: &str) -> bool {
        call_s(op::HAS_PROP, self.0, name) != 0
    }

    pub fn type_of(self, name: &str) -> JsType {
        match call_s(op::TYPEOF, self.0, name) {
            1 => JsType::Null,
            2 => JsType::Bool,
            3 => JsType::Number,
            4 => JsType::String,
            5 => JsType::Object,
            6 => JsType::Function,
            _ => JsType::Undefined,
        }
    }

    pub fn set(self, name: &str, value: Val) {
        let (p, l) = s(name);
        call(op::SET_PROP_H, self.0, p, l, value.0, 0);
    }

    /// Call a method with no arguments: `form.reset()`, `el.click()`,
    /// `input.select()`, `dialog.close()`.
    pub fn call(self, method: &str) -> Val {
        let (p, l) = s(method);
        Val(call(op::CALL_S, self.0, p, l, 0, NONE))
    }

    pub fn call_str(self, method: &str, arg: &str) -> Val {
        let (p, l) = s(method);
        let (q, m) = s(arg);
        Val(call(op::CALL_S, self.0, p, l, q, m))
    }

    pub fn call_val(self, method: &str, a: Val, b: Val) -> Val {
        let (p, l) = s(method);
        Val(call(op::CALL_H, self.0, p, l, a.0, b.0))
    }

    pub fn call_num(self, method: &str, arg: f64) -> Val {
        let (p, l) = s(method);
        let (lo, hi) = host::f(arg);
        Val(call(op::CALL_N, self.0, p, l, lo, hi))
    }

    /// `String(value)`.
    pub fn str(self) -> Str {
        Str::new(host::take(call1(op::STR_OF, self.0)).unwrap_or_default())
    }

    pub fn num(self) -> Option<f64> {
        let mut slot = host::Slot::<1>::new();
        let ptr = slot.ptr();
        if call2(op::NUM_OF, self.0, ptr) == 0 { None } else { Some(slot.get(0)) }
    }

    pub fn truthy(self) -> bool {
        call1(op::BOOL_OF, self.0) != 0
    }

    pub fn index(self, i: u32) -> Val {
        Val(call2(op::INDEX, self.0, i))
    }

    pub fn len(self) -> u32 {
        call1(op::LEN_OF, self.0)
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn keys(self, kind: Keys) -> ElList {
        ElList(call2(op::KEYS, self.0, kind as u32))
    }

    /// Whether two handles name the same JavaScript value. Node identity is
    /// what a DOM morph is built on, so this is `===`, not equality of the
    /// handle numbers.
    pub fn same(self, other: Val) -> bool {
        call2(op::EQ, self.0, other.0) != 0
    }

    /// Release the slot. Only worth doing for handles taken in a loop that
    /// runs for the life of the page; hydration-time handles can be kept.
    pub fn free(self) {
        if self.0 != 0 {
            call1(op::FREE, self.0);
        }
    }
}

impl El {
    pub const NULL: El = El(0);

    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn is_some(self) -> bool {
        self.0 != 0
    }

    /// The untyped handle, for the generic property and call operations.
    pub fn val(self) -> Val {
        Val(self.0)
    }

    // --- queries and the tree ----------------------------------------------

    pub fn query(self, selector: &str) -> El {
        El(call_s(op::QUERY, self.0, selector))
    }

    pub fn query_all(self, selector: &str) -> ElList {
        ElList(call_s(op::QUERY_ALL, self.0, selector))
    }

    pub fn closest(self, selector: &str) -> El {
        El(call_s(op::CLOSEST, self.0, selector))
    }

    /// Whether the element matches a selector. The selector is handed to the
    /// browser unchanged, so `:popover-open`, `:has()` and every other
    /// pseudo-class the browser knows work here.
    pub fn matches(self, selector: &str) -> bool {
        call_s(op::MATCHES, self.0, selector) != 0
    }

    pub fn parent(self) -> El {
        El(call1(op::PARENT, self.0))
    }

    pub fn children(self) -> ElList {
        ElList(call1(op::CHILDREN, self.0))
    }

    pub fn prev_sibling(self) -> El {
        self.val().get("previousElementSibling").el()
    }

    pub fn next_sibling(self) -> El {
        self.val().get("nextElementSibling").el()
    }

    /// The form this control belongs to, which is not always an ancestor: a
    /// control can be associated by its `form` attribute instead.
    pub fn form(self) -> El {
        self.val().get("form").el()
    }

    pub fn create(tag: &str) -> El {
        El(call_s0(op::CREATE, tag))
    }

    /// An element in the SVG namespace.
    ///
    /// Always build SVG this way rather than assigning markup. `create` is
    /// `document.createElement`, so `svg`, `g` and `path` come out as
    /// `HTMLUnknownElement`: they append without error and render nothing.
    /// And building nodes is what lets the site drop `unsafe-inline` from its
    /// content-security-policy, which assigning markup does not.
    pub fn create_svg(tag: &str) -> El {
        El(call_s(op::CREATE_NS, 0, tag))
    }

    /// An element in the MathML namespace.
    pub fn create_mathml(tag: &str) -> El {
        El(call_s(op::CREATE_NS, 1, tag))
    }

    /// Set a namespaced attribute — in practice `xlink:href`, and little else.
    /// Ordinary SVG attributes (`d`, `viewBox`, `fill`) are not namespaced and
    /// go through [`El::set_attr`].
    pub fn set_attr_ns(self, ns: Namespace, name: &str, value: &str) -> El {
        let (p, l) = s(name);
        call(op::ARG, 0, p, l, 0, 0);
        let (q, m) = s(value);
        call(op::SET_ATTR_NS, self.0, ns as u32, q, m, 0);
        self
    }

    pub fn create_text(text: &str) -> El {
        El(call_s0(op::CREATE_TEXT, text))
    }

    pub fn append(self, child: El) -> El {
        call2(op::APPEND, self.0, child.0);
        child
    }

    pub fn insert_before(self, child: El, before: El) {
        call(op::INSERT_BEFORE, self.0, child.0, before.0, 0, 0);
    }

    pub fn remove(self) {
        call1(op::REMOVE, self.0);
    }

    pub fn replace_with(self, other: El) {
        call2(op::REPLACE_WITH, self.0, other.0);
    }

    pub fn clone_node(self, deep: bool) -> El {
        El(call2(op::CLONE, self.0, deep as u32))
    }

    pub fn contains(self, other: El) -> bool {
        call2(op::CONTAINS, self.0, other.0) != 0
    }

    // --- text, attributes, properties --------------------------------------

    pub fn text(self) -> Str {
        Str::new(host::take(call1(op::TEXT, self.0)).unwrap_or_default())
    }

    pub fn set_text(self, text: &str) -> El {
        call_s(op::SET_TEXT, self.0, text);
        self
    }

    /// Replace the element's markup.
    ///
    /// Only ever pass a **compile-time literal** — icon markup, an empty
    /// string to clear the element. Anything read from the DOM, from props or
    /// from a response is user input by the time it reaches here, and this is
    /// the one call in the crate that will render it as markup.
    pub fn set_html_trusted(self, html: &str) {
        call_s(op::SET_HTML, self.0, html);
    }

    pub fn value(self) -> Str {
        Str::new(host::take(call1(op::VALUE, self.0)).unwrap_or_default())
    }

    pub fn set_value(self, value: &str) {
        call_ss(op::SET_PROP_STR, self.0, "value", value);
    }

    /// `None` when the attribute is absent, which is not the same as present
    /// and empty — `[data-x]` and `[data-x=""]` are different states and some
    /// islands key off exactly that difference.
    pub fn attr(self, name: &str) -> Option<Str> {
        host::take(call_s(op::GET_ATTR, self.0, name)).map(Str::new)
    }

    pub fn set_attr(self, name: &str, value: &str) -> El {
        call_ss(op::SET_ATTR, self.0, name, value);
        self
    }

    pub fn remove_attr(self, name: &str) {
        call_s(op::REMOVE_ATTR, self.0, name);
    }

    pub fn has_attr(self, name: &str) -> bool {
        call_s(op::HAS_ATTR, self.0, name) != 0
    }

    pub fn toggle_attr(self, name: &str, on: bool) -> El {
        let (p, l) = s(name);
        call(op::TOGGLE_ATTR, self.0, p, l, on as u32, 0);
        self
    }

    pub fn class_add(self, class: &str) -> El {
        call_s(op::CLASS_ADD, self.0, class);
        self
    }

    pub fn class_remove(self, class: &str) -> El {
        call_s(op::CLASS_REMOVE, self.0, class);
        self
    }

    pub fn class_toggle(self, class: &str, on: bool) -> El {
        let (p, l) = s(class);
        call(op::TOGGLE, self.0, p, l, on as u32, 0);
        self
    }

    pub fn class_has(self, class: &str) -> bool {
        call_s(op::CLASS_HAS, self.0, class) != 0
    }

    pub fn prop_bool(self, name: &str) -> bool {
        call_s(op::GET_PROP_BOOL, self.0, name) != 0
    }

    /// Set a real IDL boolean — `hidden`, `inert`, `checked`, `disabled`,
    /// `open`. These are properties, not attributes, and setting the attribute
    /// instead is a different and usually wrong thing.
    pub fn set_prop_bool(self, name: &str, on: bool) -> El {
        let (p, l) = s(name);
        call(op::PROP_BOOL, self.0, p, l, on as u32, 0);
        self
    }

    pub fn prop_str(self, name: &str) -> Option<Str> {
        host::take(call_s(op::GET_PROP_STR, self.0, name)).map(Str::new)
    }

    pub fn set_prop_str(self, name: &str, value: &str) {
        call_ss(op::SET_PROP_STR, self.0, name, value);
    }

    pub fn prop_num(self, name: &str) -> Option<f64> {
        let (p, l) = s(name);
        let mut slot = host::Slot::<1>::new();
        let ptr = slot.ptr();
        if call(op::GET_PROP_NUM, self.0, p, l, ptr, 0) == 0 { None } else { Some(slot.get(0)) }
    }

    pub fn set_prop_num(self, name: &str, value: f64) {
        let (p, l) = s(name);
        let (lo, hi) = host::f(value);
        call(op::SET_PROP_NUM, self.0, p, l, lo, hi);
    }

    /// The name may be a custom property (`--nr-v`), which is why this sets
    /// through `style.setProperty` rather than assigning to a camel-cased key.
    pub fn style_set(self, name: &str, value: &str) -> El {
        call_ss(op::STYLE_SET, self.0, name, value);
        self
    }

    /// The **computed** value, not the inline one.
    pub fn style(self, name: &str) -> Option<Str> {
        host::take(call_s(op::STYLE_GET, self.0, name)).map(Str::new)
    }

    /// The computed value of a property on a pseudo-element.
    ///
    /// Reading one and discarding it is a legitimate use: it forces a style
    /// recalculation, which is how a CSS transition is given a start point to
    /// animate from.
    pub fn style_of_pseudo(self, pseudo: &str, name: &str) -> Option<Str> {
        let (p, l) = s(pseudo);
        call(op::ARG, 0, p, l, 0, 0);
        host::take(call_s(op::COMPUTED_PSEUDO, self.0, name)).map(Str::new)
    }

    /// `dataset[key]`, so the key is camel-cased: `data-auth-tab` is
    /// `data("authTab")`.
    pub fn data(self, key: &str) -> Option<Str> {
        host::take(call_s(op::DATASET_GET, self.0, key)).map(Str::new)
    }

    pub fn set_data(self, key: &str, value: &str) {
        call_ss(op::DATASET_SET, self.0, key, value);
    }

    // --- focus, geometry, the top layer -------------------------------------

    /// Focus without scrolling — what a roving-tabindex or a restored focus
    /// wants, where moving the page under the reader would be wrong.
    pub fn focus(self) {
        call1(op::FOCUS, self.0);
    }

    /// Focus and let the browser scroll it into view.
    ///
    /// This is the right one when the person is being *sent* to a control —
    /// a command palette opening, a cleared field taking focus back. With
    /// `preventScroll` those are silently wrong on a page taller than the
    /// viewport.
    pub fn focus_visible(self) {
        call2(op::FOCUS, self.0, 1);
    }

    pub fn blur(self) {
        call1(op::BLUR, self.0);
    }

    pub fn scroll_into_view(self, smooth: bool) {
        call2(op::SCROLL_INTO_VIEW, self.0, smooth as u32);
    }

    pub fn scroll_by(self, left: f64) {
        let (lo, hi) = host::f(left);
        call(op::SCROLL_BY, self.0, lo, hi, 0, 0);
    }

    pub fn rect(self) -> Rect {
        let mut slot = host::Slot::<4>::new();
        let ptr = slot.ptr();
        if call2(op::RECT, self.0, ptr) == 0 {
            return Rect::default();
        }
        Rect { x: slot.get(0), y: slot.get(1), w: slot.get(2), h: slot.get(3) }
    }

    pub fn metric(self, metric: Metric) -> f64 {
        let mut slot = host::Slot::<1>::new();
        let ptr = slot.ptr();
        if call(op::METRIC, self.0, metric as u32, ptr, 0, 0) == 0 { 0.0 } else { slot.get(0) }
    }

    pub fn set_metric(self, metric: Metric, value: f64) {
        let (lo, hi) = host::f(value);
        call(op::SET_METRIC, self.0, metric as u32, lo, hi, 0);
    }

    pub fn set_selection(self, start: u32, end: u32) {
        call(op::SET_SELECTION, self.0, start, end, 0, 0);
    }

    /// Hand the submit back to the browser — and so to the framework's own
    /// form runtime — with validation and a real `submit` event, which
    /// dispatching one by hand does not give you.
    pub fn request_submit(self) {
        self.val().call("requestSubmit");
    }

    /// Put this element in the browser's top layer.
    ///
    /// **The element has to be a popover already**, which means a `popover`
    /// attribute in the markup or `set_prop_str("popover", "manual")` first.
    /// `showPopover()` throws on anything else and the host swallows the
    /// throw, so a box that is an ordinary element until it opens — the kit's
    /// listboxes and calendars are, so that a page with no module still shows
    /// them — silently stays in the flow, clipped by whatever contains it.
    ///
    /// Returns `false` when the browser has no popover support *or* when the
    /// call threw, so a caller can fall back rather than assume it worked.
    pub fn show_popover(self) -> bool {
        call2(op::TOP_LAYER, self.0, 0) != 0
    }

    pub fn hide_popover(self) -> bool {
        call2(op::TOP_LAYER, self.0, 1) != 0
    }

    pub fn popover_open(self) -> bool {
        call2(op::TOP_LAYER, self.0, 2) != 0
    }

    pub fn show_modal(self) -> bool {
        call2(op::TOP_LAYER, self.0, 3) != 0
    }

    pub fn close_dialog(self) {
        call2(op::TOP_LAYER, self.0, 4);
    }

    pub fn dialog_open(self) -> bool {
        call2(op::TOP_LAYER, self.0, 5) != 0
    }

    /// The files an `<input type=file>` holds.
    pub fn files(self) -> FileList {
        FileList(self.val().get("files"))
    }

    /// The form's successful controls, already encoded as a query string.
    ///
    /// The browser builds this with `new FormData(form, submitter)`, so entry
    /// order, duplicates, `form`-associated controls, `<select multiple>` and
    /// image buttons are all the browser's answer rather than ours. Files are
    /// dropped, as the JavaScript this replaces did.
    pub fn form_encode(self, submitter: El) -> Str {
        Str::new(host::take(call2(op::FORM_ENCODE, self.0, submitter.0)).unwrap_or_default())
    }

    pub fn free(self) {
        self.val().free();
    }
}

impl ElList {
    pub const EMPTY: ElList = ElList(0);

    pub fn len(self) -> u32 {
        call1(op::LIST_LEN, self.0)
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn get(self, i: u32) -> El {
        El(call2(op::LIST_GET, self.0, i))
    }

    pub fn first(self) -> El {
        self.get(0)
    }

    pub fn iter(self) -> ElListIter {
        ElListIter { list: self, at: 0, len: self.len() }
    }

    pub fn free(self) {
        if self.0 != 0 {
            call1(op::FREE, self.0);
        }
    }
}

impl IntoIterator for ElList {
    type Item = El;
    type IntoIter = ElListIter;

    fn into_iter(self) -> ElListIter {
        self.iter()
    }
}

/// Walks a list, asking the browser for one element at a time. The length is
/// read once: a live `NodeList` that changes under us would otherwise make the
/// iteration's end a moving target.
pub struct ElListIter {
    list: ElList,
    at: u32,
    len: u32,
}

impl Iterator for ElListIter {
    type Item = El;

    fn next(&mut self) -> Option<El> {
        if self.at >= self.len {
            return None;
        }
        let el = self.list.get(self.at);
        self.at += 1;
        Some(el)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.len - self.at) as usize;
        (left, Some(left))
    }
}

/// A `FileList` from an `<input type=file>`.
#[derive(Clone, Copy, Debug)]
pub struct FileList(Val);

impl FileList {
    pub fn len(self) -> u32 {
        self.0.len()
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn get(self, i: u32) -> File {
        File(self.0.index(i))
    }

    pub fn first(self) -> File {
        self.get(0)
    }
}

/// One file the user picked. Its bytes never enter linear memory: an island
/// names the file and the shim reads it — see [`crate::Action::file_base64`],
/// which is how a five megabyte avatar reaches the server without a five
/// megabyte allocation here.
#[derive(Clone, Copy, Debug)]
pub struct File(pub(crate) Val);

impl File {
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    pub fn name(self) -> Str {
        self.0.get("name").str()
    }

    pub fn mime(self) -> Str {
        self.0.get("type").str()
    }

    pub fn size(self) -> f64 {
        self.0.get("size").num().unwrap_or(0.0)
    }
}

// --- naming the globals ------------------------------------------------------

fn root(which: Root) -> u32 {
    call1(op::ROOT, which as u32)
}

/// The document. It is not an element, but the property and call operations
/// work on it, so `document().val().get("visibilityState")` and friends do.
pub fn document() -> El {
    El(root(Root::Document))
}

pub fn window() -> Val {
    Val(root(Root::Window))
}

pub fn body() -> El {
    El(root(Root::Body))
}

pub fn head() -> El {
    El(root(Root::Head))
}

/// `<html>`.
pub fn html() -> El {
    El(root(Root::DocumentElement))
}

pub fn active() -> El {
    El(root(Root::Active))
}

/// Whether the tab is hidden. Read rather than tracked, so a page that *starts*
/// hidden — a prerender, a restored background tab — is not miscounted.
pub fn document_hidden() -> bool {
    document().prop_bool("hidden")
}

pub fn query(selector: &str) -> El {
    El(call_s0(op::QUERY_DOC, selector))
}

pub fn query_all(selector: &str) -> ElList {
    ElList(call_s0(op::QUERY_ALL_DOC, selector))
}

pub fn query_in(el: El, selector: &str) -> El {
    el.query(selector)
}

/// Every element with this `name`, as the live collection `getElementsByName`
/// returns. Not the same as a `[name=…]` selector: the name needs no escaping
/// here, and code that restores focus by index has to index the same list.
pub fn by_name(name: &str) -> ElList {
    ElList(call_s0(op::BY_NAME, name))
}

pub fn by_id(id: &str) -> El {
    El(call_s0(op::GET_BY_ID, id))
}

/// A feature the browser may or may not have. Probed on the browser side,
/// because the answer is `typeof`, not a version number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feature {
    Popover,
    Dialog,
    ViewTransition,
    Clipboard,
    IntlLocale,
}

pub fn supports(feature: Feature) -> bool {
    let name = match feature {
        Feature::Popover => "popover",
        Feature::Dialog => "dialog",
        Feature::ViewTransition => "viewtransition",
        Feature::Clipboard => "clipboard",
        Feature::IntlLocale => "intl-locale",
    };
    let (p, l) = s(name);
    call(op::SUPPORTS, 1, p, l, 0, 0) != 0
}

/// `CSS.supports`.
pub fn css_supports(declaration: &str) -> bool {
    let (p, l) = s(declaration);
    call(op::SUPPORTS, 0, p, l, 0, 0) != 0
}

pub fn media_matches(query: &str) -> bool {
    call_s0(op::MEDIA, query) != 0
}

pub fn scroll_to(x: f64, y: f64) {
    let (xlo, xhi) = host::f(x);
    let (ylo, yhi) = host::f(y);
    call(op::SCROLL_TO, xlo, xhi, ylo, yhi, 0);
}

pub fn scroll_y() -> f64 {
    window().el().metric(Metric::ScrollY)
}

/// `window.confirm`. Synchronous on purpose: the handlers that use it have to
/// decide whether to prevent an event's default before the dispatch finishes,
/// and an async answer arrives too late to do that.
pub fn confirm(text: &str) -> bool {
    call_s0(op::CONFIRM, text) != 0
}

pub fn user_agent() -> Str {
    Str::new(host::take(call0(op::USER_AGENT)).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Off the browser every host call answers zero, which must read as
    /// "nothing found" all the way up rather than as a panic or a hang. The
    /// crate is compiled for the host by `cargo test --workspace`, so this is
    /// the shape of every test that can run without a browser.
    #[test]
    fn a_null_handle_absorbs_everything() {
        let el = El::NULL;
        assert!(el.is_null());
        assert!(el.query("[data-x]").is_null());
        assert!(el.closest("main").is_null());
        assert!(el.parent().is_null());
        assert!(!el.matches("div"));
        assert!(!el.has_attr("hidden"));
        assert_eq!(el.attr("data-x"), None);
        assert_eq!(el.prop_num("value"), None);
        assert!(el.text().is_empty());
        assert_eq!(el.rect(), Rect::default());
        assert_eq!(el.metric(Metric::ScrollTop), 0.0);
        assert!(el.children().is_empty());
        assert_eq!(el.children().iter().count(), 0);
        // Chaining stays legal, which is the whole point of not using Option.
        el.class_add("a").class_remove("b").set_attr("c", "d").set_text("e");
    }

    #[test]
    fn lists_iterate_to_their_length() {
        let list = ElList::EMPTY;
        assert!(list.is_empty());
        assert_eq!(list.iter().size_hint(), (0, Some(0)));
        assert!(list.first().is_null());
    }
}
