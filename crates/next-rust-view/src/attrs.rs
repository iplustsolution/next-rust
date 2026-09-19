//! Attribute helpers.
//!
//! ```
//! use next_rust_view::*;
//! let el = input![r#type("email"), name("email"), required(true), aria("label", "Email")];
//! assert_eq!(render_static(el), r#"<input type="email" name="email" required aria-label="Email">"#);
//! ```

use std::borrow::Cow;

use crate::node::{Attr, AttrValue};
use crate::style::CssClass;

/// Any attribute: `attr("hx-get", "/x")`. Event handler attributes (`on*`)
/// are refused here; see [`raw_attr`].
pub fn attr(name: impl Into<Cow<'static, str>>, value: impl AttrText) -> Attr {
    Attr::new(name, value.into_attr_text())
}

/// An attribute that is allowed to carry script, e.g. `onclick`. The value is
/// still HTML-escaped, but its *content* will execute: never pass user input.
pub fn raw_attr(name: impl Into<Cow<'static, str>>, value: impl AttrText) -> Attr {
    let mut a = Attr::new(name, value.into_attr_text());
    a.trusted = true;
    a
}

/// Values accepted as attribute text.
pub trait AttrText {
    fn into_attr_text(self) -> Cow<'static, str>;
}

impl AttrText for &str {
    fn into_attr_text(self) -> Cow<'static, str> {
        Cow::Owned(self.to_owned())
    }
}
impl AttrText for String {
    fn into_attr_text(self) -> Cow<'static, str> {
        Cow::Owned(self)
    }
}
impl AttrText for &String {
    fn into_attr_text(self) -> Cow<'static, str> {
        Cow::Owned(self.clone())
    }
}
impl AttrText for Cow<'static, str> {
    fn into_attr_text(self) -> Cow<'static, str> {
        self
    }
}
macro_rules! num_attr_text {
    ($($t:ty),*) => {$(
        impl AttrText for $t {
            fn into_attr_text(self) -> Cow<'static, str> { Cow::Owned(self.to_string()) }
        }
    )*};
}
num_attr_text!(i32, i64, u8, u16, u32, u64, usize, f32, f64);

/// Values accepted by [`class`].
pub trait ClassValue {
    fn push_class(self, out: &mut String, sheets: &mut Vec<&'static crate::Stylesheet>);
}

fn push_word(out: &mut String, word: &str) {
    let word = word.trim();
    if word.is_empty() {
        return;
    }
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(word);
}

impl ClassValue for &str {
    fn push_class(self, out: &mut String, _: &mut Vec<&'static crate::Stylesheet>) {
        push_word(out, self);
    }
}
impl ClassValue for String {
    fn push_class(self, out: &mut String, _: &mut Vec<&'static crate::Stylesheet>) {
        push_word(out, &self);
    }
}
impl ClassValue for &String {
    fn push_class(self, out: &mut String, _: &mut Vec<&'static crate::Stylesheet>) {
        push_word(out, self);
    }
}
impl ClassValue for CssClass {
    fn push_class(self, out: &mut String, sheets: &mut Vec<&'static crate::Stylesheet>) {
        push_word(out, self.name);
        sheets.push(self.sheet);
    }
}
impl<T: ClassValue> ClassValue for Option<T> {
    fn push_class(self, out: &mut String, sheets: &mut Vec<&'static crate::Stylesheet>) {
        if let Some(v) = self {
            v.push_class(out, sheets);
        }
    }
}
impl<T: ClassValue, const N: usize> ClassValue for [T; N] {
    fn push_class(self, out: &mut String, sheets: &mut Vec<&'static crate::Stylesheet>) {
        for v in self {
            v.push_class(out, sheets);
        }
    }
}
impl<T: ClassValue> ClassValue for Vec<T> {
    fn push_class(self, out: &mut String, sheets: &mut Vec<&'static crate::Stylesheet>) {
        for v in self {
            v.push_class(out, sheets);
        }
    }
}
/// `(value, condition)` – include `value` only when `condition` is true.
impl<T: ClassValue> ClassValue for (T, bool) {
    fn push_class(self, out: &mut String, sheets: &mut Vec<&'static crate::Stylesheet>) {
        if self.1 {
            self.0.push_class(out, sheets);
        }
    }
}

/// `class("a b")`, `class(styles.card)`, `class(["a", "b"])`,
/// `class([("active", is_active)])`. Multiple `class` parts are merged.
pub fn class(value: impl ClassValue) -> Attr {
    let mut s = String::new();
    let mut sheets = Vec::new();
    value.push_class(&mut s, &mut sheets);
    let mut a = Attr::new("class", s);
    a.sheets = sheets;
    a
}

/// Include `attr` only when `condition` holds.
pub fn attr_if(condition: bool, attr: Attr) -> Attr {
    if condition { attr } else { Attr::none() }
}

/// `data-*` attribute: `data("id", "42")` → `data-id="42"`.
pub fn data(name: &str, value: impl AttrText) -> Attr {
    Attr::new(format!("data-{name}"), value.into_attr_text())
}

/// `aria-*` attribute: `aria("label", "Close")` → `aria-label="Close"`.
pub fn aria(name: &str, value: impl AttrText) -> Attr {
    Attr::new(format!("aria-{name}"), value.into_attr_text())
}

/// Stable identity for list items, used by client-side hydration.
pub fn key(value: impl AttrText) -> Attr {
    Attr::new("data-nr-key", value.into_attr_text())
}

/// Declarative client event binding for interactive islands:
/// `on("click", "increment")` → `data-nr-on-click="increment"`.
pub fn on(event: &str, handler: impl AttrText) -> Attr {
    Attr::new(format!("data-nr-on-{event}"), handler.into_attr_text())
}

/// Disable prefetching for a link. By default a page is prefetched when the
/// mouse rests on its link for 400 ms.
pub fn prefetch(enabled: bool) -> Attr {
    if enabled { Attr::none() } else { Attr::new("data-nr-prefetch", "false") }
}

/// Replace the history entry instead of pushing a new one (`Link!`).
pub fn replace(enabled: bool) -> Attr {
    if enabled { Attr::new("data-nr-replace", "") } else { Attr::none() }
}

/// Make a link do a full page load instead of client-side navigation:
/// `a![href("/logout"), reload(true), "Log out"]`.
pub fn reload(enabled: bool) -> Attr {
    if enabled { Attr::new("data-nr-reload", "") } else { Attr::none() }
}

/// Mark a link as the current page: when the URL path equals the link's path,
/// the link gets this class and `aria-current="page"`. Applied by the server
/// when rendering and kept up to date by the client runtime after every
/// navigation, so it also works in layouts that stay on screen.
///
/// `a![href("/docs"), active_class("active"), "Docs"]`
pub fn active_class(class: impl AttrText) -> Attr {
    Attr::new("data-nr-active", class.into_attr_text())
}

/// Like [`active_class`], but also active on every page below the link's path:
/// a link to `/docs` is active on `/docs/routing`.
pub fn active_class_prefix(class: impl AttrText) -> Attr {
    Attr::new("data-nr-active-prefix", class.into_attr_text())
}

/// Whether a link to `href` is active on `path`, exactly or as a prefix.
pub fn link_is_active(href: &str, path: &str, prefix: bool) -> bool {
    let target = href.split(['?', '#']).next().unwrap_or("");
    let target = if target.len() > 1 { target.trim_end_matches('/') } else { target };
    let path = if path.len() > 1 { path.trim_end_matches('/') } else { path };
    if !target.starts_with('/') || target.starts_with("//") {
        return false;
    }
    path == target || (prefix && (target == "/" || path.strip_prefix(target).is_some_and(|rest| rest.starts_with('/'))))
}

/// Apply [`active_class`] / [`active_class_prefix`] for `path` to a view tree
/// (async boundaries are left to the client runtime).
pub fn mark_active_links(node: &mut crate::Node, path: &str) {
    match node {
        crate::Node::Element(el) => {
            let exact = el.attr("data-nr-active").map(str::to_owned);
            let prefix = el.attr("data-nr-active-prefix").map(str::to_owned);
            if let Some(href) = el.attr("href").map(str::to_owned) {
                for (class, is_prefix) in [(exact, false), (prefix, true)] {
                    if let Some(class) = class
                        && link_is_active(&href, path, is_prefix)
                    {
                        el.set_attr(Attr::new("class", class));
                        el.set_attr(Attr::new("aria-current", "page"));
                    }
                }
            }
            for child in &mut el.children {
                mark_active_links(child, path);
            }
        }
        crate::Node::Fragment(children) => {
            for child in children {
                mark_active_links(child, path);
            }
        }
        _ => {}
    }
}

/// On an action form: after a successful submit, stay on the page instead of
/// refreshing it or following `_redirect` (which then only applies to
/// browsers without JavaScript). Elements marked with [`action_result`] show
/// what the action returned, and the form gets `data-nr-state="success"` for
/// styling.
///
/// ```
/// use next_rust_view::*;
/// let html = render_static(form![stay_on_success(true), reset_on_success(true), p![action_result("")]]);
/// assert_eq!(html, r#"<form data-nr-stay="" data-nr-reset=""><p data-nr-result=""></p></form>"#);
/// ```
pub fn stay_on_success(enabled: bool) -> Attr {
    if enabled { Attr::new("data-nr-stay", "") } else { Attr::none() }
}

/// On a form with [`stay_on_success`]: clear its fields after a successful
/// submit.
pub fn reset_on_success(enabled: bool) -> Attr {
    if enabled { Attr::new("data-nr-reset", "") } else { Attr::none() }
}

/// Inside a form with [`stay_on_success`]: show the action's return value
/// here after a successful submit. `""` shows the whole value (a string or
/// number); a key shows that field of a returned object. Filled as text,
/// never as HTML.
pub fn action_result(key: impl AttrText) -> Attr {
    Attr::new("data-nr-result", key.into_attr_text())
}

/// Keep the scroll position after a client navigation (`Link!`).
pub fn scroll(enabled: bool) -> Attr {
    if enabled { Attr::none() } else { Attr::new("data-nr-scroll", "false") }
}

/// `type` attribute (also available as `r#type`).
pub fn type_(value: impl AttrText) -> Attr {
    Attr::new("type", value.into_attr_text())
}

/// `for` attribute of `<label>`.
pub fn for_(value: impl AttrText) -> Attr {
    Attr::new("for", value.into_attr_text())
}

/// `r#type("submit")`
pub fn r#type(value: impl AttrText) -> Attr {
    type_(value)
}

/// `r#for("email")`
pub fn r#for(value: impl AttrText) -> Attr {
    for_(value)
}

/// `r#async(true)` for scripts.
pub fn r#async(on: bool) -> Attr {
    Attr::boolean("async", on)
}

macro_rules! text_attrs {
    ($($(#[$m:meta])* $fn_name:ident => $name:literal),* $(,)?) => {$(
        $(#[$m])*
        #[doc = concat!("`", $name, "` attribute.")]
        pub fn $fn_name(value: impl AttrText) -> Attr {
            Attr::new($name, value.into_attr_text())
        }
    )*};
}

text_attrs! {
    id => "id",
    href => "href",
    src => "src",
    srcset => "srcset",
    sizes => "sizes",
    alt => "alt",
    title => "title",
    style => "style",
    name => "name",
    value => "value",
    placeholder => "placeholder",
    method => "method",
    action => "action",
    enctype => "enctype",
    rel => "rel",
    target => "target",
    width => "width",
    height => "height",
    lang => "lang",
    dir => "dir",
    role => "role",
    tabindex => "tabindex",
    content => "content",
    charset => "charset",
    http_equiv => "http-equiv",
    min => "min",
    max => "max",
    step => "step",
    pattern => "pattern",
    minlength => "minlength",
    maxlength => "maxlength",
    autocomplete => "autocomplete",
    inputmode => "inputmode",
    colspan => "colspan",
    rowspan => "rowspan",
    loading => "loading",
    decoding => "decoding",
    fetchpriority => "fetchpriority",
    download => "download",
    crossorigin => "crossorigin",
    integrity => "integrity",
    referrerpolicy => "referrerpolicy",
    datetime => "datetime",
    form_attr => "form",
    list => "list",
    accept => "accept",
    rows => "rows",
    cols => "cols",
    wrap => "wrap",
    label_attr => "label",
    media => "media",
    as_ => "as",
    nonce => "nonce",
    slot_attr => "slot",
    translate => "translate",
    spellcheck => "spellcheck",
    enterkeyhint => "enterkeyhint",
    view_box => "viewBox",
    fill => "fill",
    stroke => "stroke",
    d => "d",
    xmlns => "xmlns",
}

macro_rules! bool_attrs {
    ($($fn_name:ident => $name:literal),* $(,)?) => {$(
        #[doc = concat!("`", $name, "` boolean attribute.")]
        pub fn $fn_name(on: bool) -> Attr {
            Attr::boolean($name, on)
        }
    )*};
}

bool_attrs! {
    disabled => "disabled",
    checked => "checked",
    selected => "selected",
    required => "required",
    readonly => "readonly",
    multiple => "multiple",
    hidden => "hidden",
    autofocus => "autofocus",
    open => "open",
    defer => "defer",
    novalidate => "novalidate",
    controls => "controls",
    autoplay => "autoplay",
    muted => "muted",
    playsinline => "playsinline",
    inert => "inert",
    ismap => "ismap",
    reversed => "reversed",
    nomodule => "nomodule",
}

impl AttrValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            AttrValue::Text(t) => Some(t),
            AttrValue::Bool(_) => None,
        }
    }
}
