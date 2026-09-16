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

/// Disable prefetching for a `Link!`.
pub fn prefetch(enabled: bool) -> Attr {
    if enabled { Attr::none() } else { Attr::new("data-nr-prefetch", "false") }
}

/// Replace the history entry instead of pushing a new one (`Link!`).
pub fn replace(enabled: bool) -> Attr {
    if enabled { Attr::new("data-nr-replace", "") } else { Attr::none() }
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
