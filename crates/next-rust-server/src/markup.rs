//! The last step of an HTML response in a release build (`[build]
//! minify_names`, `minify_html`, `svg_precision`).
//!
//! Rendering, caching, per-page CSS and inline delivery all work with the
//! markup as the application wrote it. Just before a `text/html` response
//! leaves, this step rewrites its tags, and nothing else:
//!
//! * **Short names.** `data-*` attributes, ids and what refers to them
//!   (`for`, `aria-controls`, `href="#…"`, `url(#…)`), the custom properties
//!   of `style` attributes and everything in `<style>` elements get the short
//!   names the build chose (see `next_rust_assets::names`). The scripts the
//!   build compiled in already use them.
//! * **Compact markup.** Attribute values lose their quotes where HTML does
//!   not need them, `=""` goes, so do a few default attributes and the end
//!   tags HTML makes optional (`</li>`, `</td>`, `</p>`, ...). An end tag
//!   only goes when it ends the innermost element still open, so the step
//!   keeps the open elements as it reads, the way a parser would: one that
//!   closes nothing, or something further out, is markup a browser reads.
//! * **SVG numbers** in `d` and `points` are rounded to `svg_precision`.
//!
//! Text is never touched: not a space of it, in or out of `<pre>` and
//! `<textarea>`. Scripts are copied as they are. A streamed response is
//! rewritten chunk by chunk; a tag cut in two by a chunk boundary waits for
//! its other half.
//!
//! This makes pages smaller and hard to read, nothing more. Whatever a page
//! needs to work is still in it: never put a secret in markup, a stylesheet
//! or a script and rely on this to hide it.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bytes::Bytes;
use futures_util::StreamExt;
use next_rust_assets::names::{self, AttrName};

use crate::app::AppInner;
use crate::response::{Body, Response};

/// `(key, short name)` pairs, sorted by key (see `next_rust_assets::names`).
pub type NameTable = &'static [(&'static str, &'static str)];

/// How a release build writes HTML. Set by the generated code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Markup {
    /// Leave out quotes, `=""`, default attributes and optional end tags.
    pub compact: bool,
    /// Decimals kept in SVG path data (`d`, `points`); `0` keeps them all.
    pub svg_precision: u8,
}

/// Elements whose content is text up to their end tag.
const RAW_TEXT: &[&str] = &["script", "style", "textarea", "title", "xmp", "iframe", "noembed", "noframes"];

/// The elements a response has open at some point in it: what a parser
/// would have on its stack there. A response is rewritten in one pass, so
/// this is what tells an end tag that closes its own element apart from
/// one that closes nothing (a browser makes an empty paragraph out of a
/// `</p>` that has none to close) or one that closes something further out
/// (`<p>a<b>x</p>` ends the `b` too). Only what the compact rules ask for
/// is kept: the names, in the order they were opened.
pub(crate) type Open = Vec<OpenElement>;

#[derive(Debug, Clone)]
pub(crate) struct OpenElement {
    name: String,
    /// Whether its end tag has already been left out of the response. The
    /// element is over, but a parser reading the response back has it open
    /// until something around it ends, so an end tag that reaches over it
    /// is still the one that closes that outer element.
    ended: bool,
}

/// The open elements after a start tag. What a parser closes on its own
/// first is closed here too, so that what is on top is what an end tag
/// would close.
fn open_element(tag: &str, self_closing: bool, open: &mut Open) {
    // Table markup outside a table opens nothing at all: a parser throws
    // the tag away, so a `<td>` in a `<div>` is not an element and its end
    // tag is not one either. A `<template>` is a table of sorts here: the
    // rows and cells in one are built as if a table held them.
    const IN_A_TABLE: &[&str] = &["caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr"];
    if IN_A_TABLE.contains(&tag) && !open.iter().any(|el| el.name == "table" || el.name == "template") {
        return;
    }
    // Inside a table, a cell or a row ends the cell it is written in and
    // everything still open inside that cell: `<td><div><td>` is two cells,
    // not one inside the other. A parser looks out as far as the table.
    if IN_A_TABLE.contains(&tag)
        && let Some(at) = open.iter().rposition(|el| matches!(el.name.as_str(), "td" | "th"))
        && open[at + 1..].iter().all(|el| !matches!(el.name.as_str(), "table" | "template" | "html"))
    {
        open.truncate(at);
    }
    // These end a `<select>` where they stand rather than going into it,
    // and a second `<select>` is thrown away once it has ended the first.
    if matches!(tag, "input" | "keygen" | "textarea" | "select")
        && let Some(at) = open.iter().rposition(|el| el.name == "select")
    {
        open.truncate(at);
        if tag == "select" {
            return;
        }
    }
    // A paragraph ends before anything that cannot stand inside it, and so
    // does everything still open inside the paragraph. The paragraph does
    // not have to be the innermost element: a parser looks out through the
    // elements that hold no scope of their own until it meets one that
    // does, and `<p><nr-b><hr>` ends the paragraph and the `<nr-b>` both.
    const SCOPE: &[&str] =
        &["applet", "button", "caption", "html", "marquee", "math", "object", "svg", "table", "td", "th", "template"];
    // A `<table>` counts here although `closes_p` leaves it out: it ends a
    // paragraph in every document but one in quirks mode, and taking the
    // paragraph as ended only ever keeps an end tag that could have gone.
    if (names::closes_p(tag) || tag == "table")
        && let Some(at) = open.iter().rposition(|el| el.name == "p")
        && open[at + 1..].iter().all(|el| !SCOPE.contains(&el.name.as_str()))
    {
        open.truncate(at);
    }
    let closed_by = |top: &str| match tag {
        "li" => top == "li",
        "dd" | "dt" => matches!(top, "dd" | "dt"),
        "option" => top == "option",
        "optgroup" => matches!(top, "option" | "optgroup"),
        "tr" => matches!(top, "td" | "th" | "tr"),
        "td" | "th" => matches!(top, "td" | "th"),
        "thead" | "tbody" | "tfoot" => matches!(top, "td" | "th" | "tr" | "thead" | "tbody" | "tfoot"),
        _ => false,
    };
    while open.last().is_some_and(|top| closed_by(&top.name)) {
        open.pop();
    }
    // A void element has no content and never goes on the stack. `/>` only
    // ends an element inside `<svg>` or `<math>`; in HTML it is ignored,
    // and `<a href=/x/>` is not even self-closing, only written like it.
    let foreign = matches!(tag, "svg" | "math") || open.iter().any(|el| el.name == "svg" || el.name == "math");
    if !next_rust_view::VOID_TAGS.contains(&tag) && !(self_closing && foreign) {
        open.push(OpenElement { name: tag.to_owned(), ended: false });
    }
}

/// Rewrites HTML for one app.
#[derive(Debug, Clone, Copy)]
pub struct Finisher {
    names: NameTable,
    markup: Markup,
}

/// Stylesheets already renamed, by the hash of their text: a page sends the
/// same stylesheet on every request.
type StyleCache = Mutex<HashMap<(usize, u64, usize), Arc<str>>>;
static STYLES: OnceLock<StyleCache> = OnceLock::new();
const STYLE_CACHE_ENTRIES: usize = 512;

/// The finisher of the app this process runs, for [`finish`].
static INSTALLED: OnceLock<Finisher> = OnceLock::new();

/// HTML written the way this app's pages are, for markup that reaches the
/// browser some other way than a `text/html` response (a field of a JSON
/// response, a server-sent event, a WebSocket message): those are not
/// rewritten on their way out, so without this their `data-*` attributes
/// and ids would keep names the page's stylesheets and scripts no longer
/// use. Development builds return the markup unchanged.
pub fn finish(html: &str) -> Cow<'_, str> {
    match INSTALLED.get() {
        Some(finisher) => Cow::Owned(finisher.finish(html)),
        None => Cow::Borrowed(html),
    }
}

impl Finisher {
    /// `None` when the build asked for nothing (development builds).
    pub fn new(names: NameTable, markup: Markup) -> Option<Self> {
        (!names.is_empty() || markup != Markup::default()).then_some(Finisher { names, markup })
    }

    /// Make this the finisher [`finish`] uses; the first one stays.
    pub(crate) fn install(self) -> Self {
        let _ = INSTALLED.set(self);
        self
    }

    fn find(&self, key: &str) -> Option<String> {
        names::find(self.names, key)
    }

    /// A whole document or fragment.
    pub fn finish(&self, html: &str) -> String {
        let mut out = Vec::with_capacity(html.len());
        self.process(html.as_bytes(), true, &mut Open::new(), &mut out);
        // Only ASCII was added or removed, between characters.
        String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
    }

    fn css(&self, css: &str) -> Arc<str> {
        let mut h = next_rust_assets::Fnv64::new();
        h.write(css.as_bytes());
        let key = (css.len(), h.finish(), self.names.as_ptr() as usize);
        let cache = STYLES.get_or_init(Default::default);
        if let Some(hit) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
            return hit;
        }
        let renamed: Arc<str> = names::rename_css(css, &|key| self.find(key)).into();
        if css.len() >= 1024
            && let Ok(mut cache) = cache.lock()
            && cache.len() < STYLE_CACHE_ENTRIES
        {
            cache.insert(key, renamed.clone());
        }
        renamed
    }

    /// Rewrite `input` into `out`. Returns how much of `input` was used: all
    /// of it when `last`, otherwise up to the start of a tag, comment or
    /// raw-text element that is not complete yet.
    fn process(&self, input: &[u8], last: bool, open: &mut Open, out: &mut Vec<u8>) -> usize {
        let len = input.len();
        let mut copied = 0;
        let mut i = 0;
        // `return stop!(at)`: everything before `at` is done.
        macro_rules! stop {
            ($at:expr) => {{
                let at = if last { len } else { $at };
                out.extend_from_slice(&input[copied..at]);
                at
            }};
        }
        while i < len {
            let Some(lt) = input[i..].iter().position(|b| *b == b'<').map(|p| i + p) else { break };
            let Some(&next) = input.get(lt + 1) else { return stop!(lt) };
            if next == b'!' {
                // A comment, a doctype, or the junk a browser also makes a
                // comment of: all of it is one node, and what looks like a
                // tag inside it is not one. A chunk that ends in the middle
                // of the `<!--` cannot tell which of them this is, so it
                // waits rather than read the comment as markup.
                let end = match input[lt..].starts_with(b"<!--") {
                    true => find(input, lt + 4, b"-->").map(|end| end + 3),
                    false => input[lt..].iter().position(|b| *b == b'>').map(|p| lt + p + 1),
                };
                match end {
                    Some(end) => i = end,
                    None => return stop!(lt),
                }
                continue;
            }
            if next == b'/' {
                let Some(gt) = input[lt..].iter().position(|b| *b == b'>').map(|p| lt + p) else { return stop!(lt) };
                i = gt + 1;
                if self.markup.compact {
                    let lowered = lower(&input[lt + 2..gt]);
                    let name = lowered.trim();
                    // Only an end tag that closes the element it belongs to
                    // may go: one that closes nothing, or that reaches over
                    // an element still open, does work a browser can see.
                    match open.iter().rposition(|el| el.name == name) {
                        Some(at) if open[at + 1..].iter().all(|el| el.ended) => {
                            match self.end_tag_needed(open, at, &input[gt + 1..]) {
                                Some(false) => {
                                    out.extend_from_slice(&input[copied..lt]);
                                    copied = i;
                                    // The element stays: a parser reading the
                                    // response back has it open until what is
                                    // around it ends, and so has everything
                                    // inside it whose end tag also went.
                                    open[at].ended = true;
                                }
                                // What follows is not here yet.
                                None if !last => return stop!(lt),
                                _ => open.truncate(at),
                            }
                        }
                        // A parser closes everything this end tag reaches
                        // over; nothing around here can be left out.
                        Some(at) => open.truncate(at),
                        None => {}
                    }
                }
                continue;
            }
            if !next.is_ascii_alphabetic() {
                // `<!doctype …>`, `<?…>`, or a `<` that is text.
                i = lt + 1;
                continue;
            }
            let Some(gt) = tag_end(input, lt) else { return stop!(lt) };
            let name_end =
                lt + 1 + input[lt + 1..gt].iter().take_while(|b| !b.is_ascii_whitespace() && **b != b'/').count();
            let tag = lower(&input[lt + 1..name_end]);
            // The content of a raw-text element, up to its end tag.
            let content_end = if RAW_TEXT.contains(&tag.as_str()) {
                match find_end_tag(input, gt + 1, &tag) {
                    Some(end) => Some(end),
                    None if last => Some(len),
                    None => return stop!(lt),
                }
            } else {
                None
            };
            if self.markup.compact {
                open_element(&tag, input[gt - 1] == b'/', open);
            }
            if name_end < gt
                && let Ok(text) = std::str::from_utf8(&input[name_end..gt])
                && let Some(attrs) = self.attributes(&tag, text)
            {
                out.extend_from_slice(&input[copied..name_end]);
                out.extend_from_slice(attrs.as_bytes());
                copied = gt;
            }
            i = gt + 1;
            if let Some(end) = content_end {
                if tag == "style"
                    && !self.names.is_empty()
                    && let Ok(css) = std::str::from_utf8(&input[i..end])
                    && !css.is_empty()
                {
                    out.extend_from_slice(&input[copied..i]);
                    out.extend_from_slice(self.css(css).as_bytes());
                    copied = end;
                }
                i = end;
            }
        }
        out.extend_from_slice(&input[copied..]);
        len
    }

    /// Whether the end tag of the element at `at` has to be written, given
    /// what is open around it and what follows it. `None`: not enough of
    /// the document to tell yet.
    fn end_tag_needed(&self, open: &Open, at: usize, after: &[u8]) -> Option<bool> {
        const OPTIONAL: &[&str] =
            &["li", "dt", "dd", "option", "optgroup", "tr", "td", "th", "thead", "tbody", "tfoot", "p"];
        let name = open[at].name.as_str();
        if !OPTIONAL.contains(&name) {
            return Some(true);
        }
        // Only when the next tag follows at once: text, even a space,
        // would become part of the element.
        if after.first().is_some_and(|b| *b != b'<') {
            return Some(true);
        }
        // A `<select>` is the one element whose content a parser moves
        // about or throws away, and it only keeps the shape these rules
        // know while there is nothing but options inside it: an `<input>`
        // in a `<select>`, say, ends the select where it stands.
        if let Some(select) = open[..at].iter().rposition(|el| el.name == "select")
            && !open[select + 1..=at].iter().all(|el| matches!(el.name.as_str(), "option" | "optgroup"))
        {
            return Some(true);
        }
        let closing = after.get(1)? == &b'/';
        let from = if closing { 2 } else { 1 };
        let end = from + after.get(from..)?.iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'-').count();
        if end == after.len() {
            // The tag's name may go on in the next chunk.
            return None;
        }
        if end == from {
            // `<!`, `</>`: not a tag that ends anything.
            return Some(true);
        }
        let next = lower(&after[from..end]);
        let parent = at.checked_sub(1).map_or("", |below| open[below].name.as_str());
        if !closing {
            return Some(!names::end_tag_optional(name, Some(&next), parent));
        }
        // An end tag follows. It only ends this element's parent if it
        // belongs to something still open around it: a stray one is
        // ignored by a browser, and then the element goes on to hold
        // whatever comes after it.
        match open[..at].iter().any(|el| el.name == next) {
            true => Some(!names::end_tag_optional(name, None, parent)),
            false => Some(true),
        }
    }

    /// The attributes of a start tag (everything between its name and `>`),
    /// rewritten; `None` when nothing changes.
    fn attributes(&self, tag: &str, text: &str) -> Option<String> {
        let b = text.as_bytes();
        let mut out = String::with_capacity(text.len());
        let mut changed = false;
        let mut bare_value = false;
        let mut self_closing = false;
        let mut seen: Vec<String> = Vec::new();
        let mut i = 0;
        loop {
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= b.len() {
                break;
            }
            if b[i] == b'/' {
                // `/>`; a stray `/` between attributes means nothing.
                self_closing = i + 1 == b.len();
                changed |= !self_closing;
                i += 1;
                continue;
            }
            let name_start = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'/') {
                i += 1;
            }
            // `=` right at the start is part of the name (as HTML reads it),
            // and so is everything after it up to the next `=`: `<a =x>` is
            // one attribute called `=x`, not `=` followed by `x`.
            if i == name_start {
                i += 1;
                while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'/') {
                    i += 1;
                }
            }
            let name = &text[name_start..i];
            let mut after = i;
            while after < b.len() && b[after].is_ascii_whitespace() {
                after += 1;
            }
            let mut value: Option<(&str, Option<u8>)> = None;
            if b.get(after) == Some(&b'=') {
                after += 1;
                while after < b.len() && b[after].is_ascii_whitespace() {
                    after += 1;
                }
                match b.get(after) {
                    Some(q @ (b'"' | b'\'')) => {
                        let start = after + 1;
                        let end = text[start..].find(*q as char).map_or(b.len(), |e| start + e);
                        value = Some((&text[start..end], Some(*q)));
                        i = (end + 1).min(b.len());
                    }
                    _ => {
                        let end = after + b[after..].iter().take_while(|c| !c.is_ascii_whitespace()).count();
                        value = Some((&text[after..end], None));
                        i = end;
                    }
                }
            }
            let lower_name = name.to_ascii_lowercase();
            // A browser keeps the first of two attributes of the same name
            // and drops the rest, so leaving the rest out changes nothing.
            // Writing them out would: the rules below may take the first one
            // away (`<form method=get method=post>` submits with GET), and
            // then a later copy would be the one that counts.
            if seen.contains(&lower_name) {
                changed = true;
                continue;
            }
            seen.push(lower_name.clone());
            let new_name: Cow<'_, str> = match self.rename_attr(&lower_name) {
                AttrName::Keep => Cow::Borrowed(name),
                AttrName::Drop => {
                    changed = true;
                    continue;
                }
                AttrName::Renamed(short) => {
                    changed = true;
                    Cow::Owned(short.into_owned())
                }
            };
            let new_value: Option<Cow<'_, str>> = value.map(|(v, _)| self.value(&lower_name, v));
            if self.markup.compact
                && let Some(v) = &new_value
                && names::is_default_attr(tag, &lower_name, v)
            {
                changed = true;
                continue;
            }
            out.push(' ');
            out.push_str(&new_name);
            bare_value = false;
            let (Some(new), Some((old, quote))) = (new_value, value) else { continue };
            changed |= new != old;
            // A value the source wrote without quotes and this step does not
            // change goes back as it was: the browser read it that way, so
            // quotes would only make the response bigger, and a value like
            // `a"b` cannot be quoted without changing where it ends.
            let as_written = quote.is_none() && new == old;
            // A value full of `&quot;` (island props are JSON) costs six
            // bytes a quote. Single quotes let it carry `"` as itself, which
            // a parser decodes to the same value.
            if self.markup.compact && new.contains("&quot;") && !new.contains('\'') && !new.contains("&#39;") {
                changed = true;
                out.push_str("='");
                out.push_str(&new.replace("&quot;", "\""));
                out.push('\'');
                continue;
            }
            let bare = if self.markup.compact { names::unquoted(&new) || as_written } else { quote.is_none() };
            if self.markup.compact && new.is_empty() {
                changed = true;
            } else if bare {
                changed |= quote.is_some();
                bare_value = true;
                out.push('=');
                out.push_str(&new);
            } else {
                // The quote has to be one the value does not hold itself, or
                // the value would end early. When it holds both, the one in
                // the value is written as the reference a browser reads back
                // as that same character.
                let q = match quote {
                    Some(q) if !new.contains(q as char) => q as char,
                    _ if !new.contains('"') => '"',
                    _ if !new.contains('\'') => '\'',
                    _ => '"',
                };
                changed |= quote != Some(q as u8);
                out.push('=');
                out.push(q);
                match new.contains(q) {
                    true => {
                        changed = true;
                        out.push_str(&new.replace(q, if q == '"' { "&quot;" } else { "&#39;" }));
                    }
                    false => out.push_str(&new),
                }
                out.push(q);
            }
        }
        if !changed {
            return None;
        }
        if self_closing {
            // `href=#a/>` would make the slash part of the value.
            out.push_str(if bare_value { " /" } else { "/" });
        }
        Some(out)
    }

    fn rename_attr<'a>(&self, name: &'a str) -> AttrName<'a> {
        if self.names.is_empty() { AttrName::Keep } else { names::attr_name(name, &|key| self.find(key)) }
    }

    fn value<'a>(&self, name: &str, value: &'a str) -> Cow<'a, str> {
        let value = if self.names.is_empty() {
            Cow::Borrowed(value)
        } else {
            names::attr_value(name, value, &|key| self.find(key))
        };
        if self.markup.svg_precision > 0 && matches!(name, "d" | "points") {
            return match names::trim_numbers(&value, self.markup.svg_precision) {
                Cow::Owned(trimmed) => Cow::Owned(trimmed),
                Cow::Borrowed(_) => value,
            };
        }
        value
    }
}

fn lower(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_ascii_lowercase()
}

fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|p| from + p)
}

/// The `>` that ends the start tag at `lt`, outside quoted attribute values.
fn tag_end(input: &[u8], lt: usize) -> Option<usize> {
    let mut quote = None;
    let mut after_eq = false;
    for (at, b) in input.iter().enumerate().skip(lt + 1) {
        match (quote, *b) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            // A quote opens a value only right after `=`.
            (None, b'"' | b'\'') if after_eq => quote = Some(*b),
            (None, b'>') => return Some(at),
            _ => {}
        }
        if quote.is_none() && !b.is_ascii_whitespace() {
            after_eq = *b == b'=';
        }
    }
    None
}

/// Start of `</tag` after `from`, in any case. The name has to end where it
/// is found: inside a raw-text element a browser only ends the element on
/// `</tag` followed by whitespace, `/` or `>`, so `content:"</style"` in a
/// stylesheet and `</scriptish>` in a script are text, and this step has to
/// read them as text too or it would rewrite text as if it were markup.
/// `None` also means "not enough of the stream to tell yet".
fn find_end_tag(input: &[u8], from: usize, tag: &str) -> Option<usize> {
    let mut at = from;
    while let Some(lt) = find(input, at, b"</") {
        let name = input.get(lt + 2..lt + 2 + tag.len())?;
        let after = *input.get(lt + 2 + tag.len())?;
        if name.eq_ignore_ascii_case(tag.as_bytes()) && (after.is_ascii_whitespace() || matches!(after, b'/' | b'>')) {
            return Some(lt);
        }
        at = lt + 2;
    }
    None
}

/// Rewrite a `text/html` response as it leaves (see the module).
pub(crate) fn apply(inner: &AppInner, res: &mut Response) {
    let Some(finisher) = inner.finisher else { return };
    // Only HTML as text: a body that is already compressed is sent as it is.
    if !res.header("content-type").is_some_and(|c| c.starts_with("text/html"))
        || res.header("content-encoding").is_some()
    {
        return;
    }
    match std::mem::replace(&mut res.body, Body::Empty) {
        Body::Bytes(bytes) => {
            let mut out = Vec::with_capacity(bytes.len());
            finisher.process(&bytes, true, &mut Open::new(), &mut out);
            res.body = Body::Bytes(Bytes::from(out));
            res.headers.remove("content-length");
        }
        Body::Stream(stream) => {
            res.headers.remove("content-length");
            struct State {
                inner: crate::response::ByteStream,
                finisher: Finisher,
                carry: Vec<u8>,
                open: Open,
                done: bool,
            }
            let state = State { inner: stream, finisher, carry: Vec::new(), open: Open::new(), done: false };
            res.body = Body::Stream(Box::pin(futures_util::stream::unfold(state, |mut s| async move {
                if s.done {
                    return None;
                }
                loop {
                    match s.inner.next().await {
                        Some(Ok(chunk)) => {
                            let input = if s.carry.is_empty() {
                                chunk.to_vec()
                            } else {
                                let mut joined = std::mem::take(&mut s.carry);
                                joined.extend_from_slice(&chunk);
                                joined
                            };
                            let mut out = Vec::with_capacity(input.len());
                            let used = s.finisher.process(&input, false, &mut s.open, &mut out);
                            s.carry = input[used..].to_vec();
                            if !out.is_empty() {
                                return Some((Ok(Bytes::from(out)), s));
                            }
                        }
                        Some(Err(e)) => {
                            s.done = true;
                            return Some((Err(e), s));
                        }
                        None => {
                            s.done = true;
                            if s.carry.is_empty() {
                                return None;
                            }
                            let mut out = Vec::with_capacity(s.carry.len());
                            let carry = std::mem::take(&mut s.carry);
                            s.finisher.process(&carry, true, &mut s.open, &mut out);
                            return Some((Ok(Bytes::from(out)), s));
                        }
                    }
                }
            })));
        }
        Body::Empty => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: NameTable = &[
        ("#auth-*", "w-"),
        ("#nr-listbox-+", "z-"),
        ("--brand", "--a"),
        ("@k:fade", "c"),
        ("data-component", ""),
        ("data-open", "data-k"),
        ("data-strength-for", "data-m"),
    ];

    fn names_only() -> Finisher {
        Finisher::new(NAMES, Markup::default()).unwrap()
    }

    fn compact() -> Finisher {
        Finisher::new(&[], Markup { compact: true, svg_precision: 3 }).unwrap()
    }

    /// The client runtime is served as written, so every name it reads or
    /// sets must be one a build never renames: `data-nr-*`, or listed in
    /// `next_rust_assets::names::RUNTIME_NAMES`.
    #[test]
    fn the_runtime_only_uses_names_a_build_keeps() {
        let kept = |name: &str| name.starts_with("data-nr-") || names::RUNTIME_NAMES.contains(&name);
        for (what, script) in [
            ("runtime", crate::client_runtime::RUNTIME_JS_MIN),
            ("streaming runtime", next_rust_view::STREAMING_RUNTIME),
        ] {
            let mut found = Vec::new();
            // Tokens in strings: `"data-x"`, `[data-x]`, `"--x"`.
            let b = script.as_bytes();
            for (at, _) in script.match_indices("data-").chain(script.match_indices("--")) {
                let boundary = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || matches!(b[at - 1], b'-' | b'_'));
                // In a string or a selector; `a,--r<=0` is a decrement.
                let quoted =
                    at > 0 && (matches!(b[at - 1], b'"' | b'\'' | b'`' | b'[') || script[..at].ends_with("var("));
                let end =
                    at + b[at..].iter().take_while(|c| c.is_ascii_alphanumeric() || matches!(**c, b'-' | b'_')).count();
                let name = &script[at..end];
                // `<!--nr-l:` is a comment marker.
                if boundary && quoted && !name.ends_with('-') && name.len() > 2 && !script[..at].ends_with("<!") {
                    found.push(name.to_owned());
                }
            }
            for (at, _) in script.match_indices("dataset.") {
                let rest = &script[at + "dataset.".len()..];
                let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
                found.push(names::dataset_attr(&rest[..end]));
            }
            found.sort();
            found.dedup();
            // The island's component name is only read for an error message;
            // builds leave the attribute out.
            let unknown: Vec<&String> = found.iter().filter(|n| !kept(n) && *n != "data-component").collect();
            assert!(
                unknown.is_empty(),
                "the {what} uses {unknown:?}: add them to `next_rust_assets::names::RUNTIME_NAMES` so release builds keep them"
            );
        }
    }

    #[test]
    fn nothing_to_do_in_development() {
        assert!(Finisher::new(&[], Markup::default()).is_none());
    }

    #[test]
    fn names_are_rewritten_in_tags_and_styles_only() {
        let html = r##"<!DOCTYPE html><html data-open=""><head><style data-nr-css="x">[data-open]{color:var(--brand);animation:fade 1s}@keyframes fade{to{opacity:1}}</style><script>el.dataset.open; "--brand"; "[data-open]"</script></head><body><!-- data-open --brand --><label for="auth-email" style="--brand:red">data-open --brand</label><input id="auth-email" aria-describedby="auth-email-error nr-listbox-0-1" data-strength-for="auth-email"><nr-island data-component="Counter" data-props="{&quot;x&quot;:&quot;data-open&quot;}"><a href="#auth-email" data-open>x</a></nr-island><textarea><b data-open></textarea><svg><use href="#auth-icon"/></svg></body></html>"##;
        let expected = r##"<!DOCTYPE html><html data-k=""><head><style data-nr-css="x">[data-k]{color:var(--a);animation:c 1s}@keyframes c{to{opacity:1}}</style><script>el.dataset.open; "--brand"; "[data-open]"</script></head><body><!-- data-open --brand --><label for="w-email" style="--a:red">data-open --brand</label><input id="w-email" aria-describedby="w-email-error z-0-1" data-m="w-email"><nr-island data-props="{&quot;x&quot;:&quot;data-open&quot;}"><a href="#w-email" data-k>x</a></nr-island><textarea><b data-open></textarea><svg><use href="#w-icon"/></svg></body></html>"##;
        assert_eq!(names_only().finish(html), expected);
    }

    #[test]
    fn compact_markup_keeps_the_document_the_same() {
        let html = r##"<form method="get" action="/s" class="a b"><input type="text" name="q" value="" required disabled=""><select><option value="">All</option><option value="1" selected>One</option></select><ul><li><p>One</p></li><li>Two</li> <li>Three</li></ul><p>Text</p><span>inline</span><p>Last</p></form><table><thead><tr><th>A</th></tr></thead><tbody><tr><td title='say "hi"'>1</td><td>2</td></tr></tbody></table><a href="/a/b/">x</a><svg viewBox="0 0 24 24"><path d="M1.23456 2"/><use href="#i"/></svg><pre>  <b class="x">kept  </b>
</pre>"##;
        let expected = r##"<form action=/s class="a b"><input type=text name=q value required disabled><select><option value>All<option value=1 selected>One</select><ul><li><p>One<li>Two</li> <li>Three</ul><p>Text</p><span>inline</span><p>Last</form><table><thead><tr><th>A<tbody><tr><td title='say "hi"'>1<td>2</table><a href=/a/b/>x</a><svg viewBox="0 0 24 24"><path d="M1.235 2"/><use href=#i /></svg><pre>  <b class=x>kept  </b>
</pre>"##;
        assert_eq!(compact().finish(html), expected);
    }

    #[test]
    fn end_tags_stay_where_html_needs_them() {
        let f = compact();
        // A suspense boundary between a list and its items, an island around a paragraph.
        let html = "<ul><nr-b id=\"nr-b1\"><li>x</li></nr-b></ul><nr-island><p>x</p></nr-island><a><p>x</p></a><dl><dt>a</dt><dd>b</dd></dl>";
        assert_eq!(
            f.finish(html),
            "<ul><nr-b id=nr-b1><li>x</li></nr-b></ul><nr-island><p>x</p></nr-island><a><p>x</p></a><dl><dt>a<dd>b</dl>"
        );
    }

    #[test]
    fn scripts_and_odd_markup_pass_through() {
        let f = Finisher::new(NAMES, Markup { compact: true, svg_precision: 0 }).unwrap();
        for html in [
            "<script>if (a < b && c > d) { x = \"<div data-open=''>\" }</script>",
            "<script type=\"application/json\">{\"data-open\":\"<\\/script\"}</script>",
            "a < b and <3 and <>",
            "<!-- <div data-open=\"\"> -->",
            "<title><b data-open=\"\"></title>",
        ] {
            let out = f.finish(html);
            assert!(out == html || out == html.replace("type=\"application/json\"", "type=application/json"), "{out}");
        }
        // An attribute value may hold `>`.
        assert_eq!(f.finish("<a title=\"a > b\" data-open=\"\">x</a>"), "<a title=\"a > b\" data-k>x</a>");
    }

    /// An inlined script carries whatever the application wrote, including
    /// strings that look like markup. Everything from a `<script>` to its
    /// end tag is content, so none of it may be read as a tag, a comment or
    /// the end of the document: the inliner had a bug of exactly this shape,
    /// where `"</head>"` inside application code was taken for the real one.
    #[test]
    fn markup_inside_a_script_is_not_markup() {
        let f = Finisher::new(NAMES, Markup { compact: true, svg_precision: 3 }).unwrap();
        let script = concat!(
            "<script type=\"module\" data-nr-js=\"a1\">",
            r#"const a = "</head>", b = '<style data-open="">x</style>', c = `<!-- <li>no</li>`;"#,
            r#"const d = /<\/head>|<!--|<li>/g, e = "<nr-island data-component='X'>";"#,
            "</script>",
        );
        let page = format!("<html><head>{script}</head><body><li data-open=\"\">x</li></body></html>");
        let out = f.finish(&page);
        // The body of the script is copied out byte for byte (its start tag
        // is a tag like any other, so its quotes may go).
        let body = script.split_once('>').unwrap().1.strip_suffix("</script>").unwrap();
        assert!(out.contains(body), "{out}");
        assert!(out.contains("<script type=module data-nr-js=a1>") && out.contains("</script></head>"), "{out}");
        // The markup around it was minified as usual. The `</li>` stays:
        // `</body>` follows, and `li` only ends by itself inside a list.
        assert!(out.ends_with("<body><li data-k>x</li></body></html>"), "{out}");
        // A comment holding a script start tag does not open one either.
        let tricky = "<!-- <script> --><p data-open=\"\">x</p><script>\"<!--\"</script>";
        assert_eq!(f.finish(tricky), "<!-- <script> --><p data-k>x</p><script>\"<!--\"</script>");
    }

    /// Each of these is a way a value can stop meaning what it meant. A
    /// browser reads an unquoted value up to the next space or `>`, so the
    /// slash of `href=/x/` belongs to the value, and a quote inside one
    /// does too: putting quotes around such a value would end it early.
    #[test]
    fn unquoted_values_keep_what_they_held() {
        let f = Finisher::new(NAMES, Markup { compact: true, svg_precision: 3 }).unwrap();
        for html in [
            // The `/` is part of the value, not a self-closing tag.
            "<a href=/x/>t</a>",
            // A boolean attribute after a value that has no quotes to end.
            "<a href=#x download>t</a>",
            // `&` and an entity, and a value with `=` in it.
            "<a href=/a?x=1&y=2&amp;z=3>t</a>",
            // A value holding the quote character it would be wrapped in.
            r#"<a href=a"b>t</a>"#,
            "<a href=a'b>t</a>",
        ] {
            assert_eq!(f.finish(html), html, "{html}");
        }
        // Quotes only go where the value can do without them.
        assert_eq!(f.finish(r#"<a title='a"b' data-open="">t</a>"#), r#"<a title='a"b' data-k>t</a>"#);
        assert_eq!(f.finish(r#"<a title="a > b" data-open="">t</a>"#), r#"<a title="a > b" data-k>t</a>"#);
        // An id or a `for` can hold anything at all; a space keeps the quotes.
        assert_eq!(f.finish(r#"<label for="a b" data-open="">t</label>"#), r#"<label for="a b" data-k>t</label>"#);
        assert_eq!(f.finish(r#"<b id="a=b" data-open="">t</b>"#), r#"<b id="a=b" data-k>t</b>"#);
        assert_eq!(f.finish("<b id=\"a`b\" data-open=\"\">t</b>"), "<b id=\"a`b\" data-k>t</b>");
    }

    /// A browser keeps the first of two attributes of the same name. The
    /// rules that leave an attribute out have to reckon with that, or the
    /// second one becomes the one that counts.
    #[test]
    fn duplicate_attributes_keep_the_first() {
        let f = compact();
        // `method=get` is the default and goes; the form still submits with GET.
        assert_eq!(f.finish("<form method=get method=post>x</form>"), "<form>x</form>");
        assert_eq!(f.finish(r#"<input value="" value="x">"#), "<input value>");
        assert_eq!(f.finish(r#"<b ID="a" id="b">t</b>"#), "<b ID=a>t</b>");
    }

    /// Inside a raw-text element only `</name` followed by whitespace, `/`
    /// or `>` ends the element. Stopping anywhere else would read the rest
    /// of the text as markup and rewrite it.
    #[test]
    fn raw_text_ends_only_where_a_browser_ends_it() {
        let f = Finisher::new(NAMES, Markup { compact: true, svg_precision: 3 }).unwrap();
        // `</style"` is part of the string: the selector after it is still CSS.
        assert_eq!(
            f.finish(r#"<style>a{content:"</style"}b[data-open]{--brand:1}</style>"#),
            r#"<style>a{content:"</style"}b[data-k]{--a:1}</style>"#
        );
        // A script says `<\/script` for the same reason, and keeps it.
        let js = r#"<script>var s="<\/script";var t="[data-open]"</script>"#;
        assert_eq!(f.finish(js), js);
        // A tag that only starts like the end tag leaves the text alone.
        let odd = r#"<style>a{content:"</styles>"}</style><p data-open="">x</p>"#;
        assert_eq!(f.finish(odd), r#"<style>a{content:"</styles>"}</style><p data-k>x</p>"#);
    }

    /// Tag and attribute names are case insensitive in HTML and a name may
    /// hold a colon, so the lookups have to be made on a lowered copy while
    /// the name itself is written back as it was.
    #[test]
    fn names_in_any_case_and_with_namespaces() {
        let f = Finisher::new(NAMES, Markup { compact: true, svg_precision: 3 }).unwrap();
        assert_eq!(f.finish(r#"<INPUT TYPE="TEXT" DATA-OPEN="">"#), "<INPUT TYPE=TEXT data-k>");
        assert_eq!(f.finish(r#"<SCRIPT TYPE="TEXT/JAVASCRIPT">a</SCRIPT>"#), "<SCRIPT>a</SCRIPT>");
        assert_eq!(
            f.finish(r#"<SVG viewBox="0 0 2 2"><PATH D="M1.23456 2"/></SVG>"#),
            r#"<SVG viewBox="0 0 2 2"><PATH D="M1.235 2"/></SVG>"#
        );
        assert_eq!(f.finish(r##"<use xlink:href="#auth-x" data-open="">"##), "<use xlink:href=#w-x data-k>");
    }

    /// `</p>` may only go when a tag follows it at once: a space, a letter
    /// or a comment between the two would land inside the paragraph, or
    /// keep it open past where it ended.
    #[test]
    fn paragraph_end_tags_stay_where_anything_follows_them() {
        let f = compact();
        assert_eq!(f.finish("<p>x</p><!--c--><div>y</div>"), "<p>x</p><!--c--><div>y</div>");
        assert_eq!(f.finish("<p>x</p> <div>y</div>"), "<p>x</p> <div>y</div>");
        assert_eq!(f.finish("<p>x</p>tail<div>y</div>"), "<p>x</p>tail<div>y</div>");
        // A tag right after it, and the end of the paragraph's parent.
        assert_eq!(f.finish("<p>x</p><div>y</div>"), "<p>x<div>y</div>");
        assert_eq!(f.finish("<div><p>x</p></div>"), "<div><p>x</div>");
    }

    /// Path data is rounded, never re-written: the digits of an arc flag
    /// and the exponent of a small number both change what the number
    /// means if they are read as decimals.
    #[test]
    fn svg_path_numbers_keep_their_meaning() {
        let f = compact();
        let d = |data: &str| f.finish(&format!("<path d=\"{data}\"/>"));
        assert_eq!(d("a1 1 0 01.5.5"), "<path d=\"a1 1 0 01.5.5\"/>");
        assert_eq!(d("a1 1 0 01.23456.5"), "<path d=\"a1 1 0 01.235.5\"/>");
        assert_eq!(d("M2.5e-7 1.23456e3"), "<path d=\"M2.5e-7 1.23456e3\"/>");
        // A value with no spaces in it also loses its quotes.
        assert_eq!(d("M+1.23456-.56789"), "<path d=M+1.235-.568 />");
        assert_eq!(d("M-.00049 1.00049"), "<path d=\"M-0 1\"/>");
    }

    #[test]
    fn chunks_give_the_same_document_wherever_they_are_cut() {
        let html = r##"<!DOCTYPE html><html><head><style>[data-open]{--brand:1}</style></head><body><!--nr-l:abc--><ul><li data-open="">One</li><li>Two</li></ul><p title="a > b">x</p><script>"<li></li>"</script><textarea></li></textarea></body></html>"##;
        let f = Finisher::new(NAMES, Markup { compact: true, svg_precision: 3 }).unwrap();
        let whole = f.finish(html);
        for cut in 1..html.len() {
            let mut out = Vec::new();
            let mut carry: Vec<u8> = Vec::new();
            let mut open = Open::new();
            for (part, last) in [(&html.as_bytes()[..cut], false), (&html.as_bytes()[cut..], true)] {
                carry.extend_from_slice(part);
                let used = f.process(&carry.clone(), last, &mut open, &mut out);
                carry.drain(..used);
            }
            assert_eq!(String::from_utf8(out).unwrap(), whole, "cut at {cut}");
        }
    }

    // -----------------------------------------------------------------
    // Generated documents
    // -----------------------------------------------------------------

    /// How many documents the property test below builds, on how many of
    /// them every chunk boundary is tried, and on how many every pair of
    /// boundaries. Cutting a document at every boundary costs a pass over
    /// it per boundary, and every pair costs a pass per pair, so the three
    /// numbers fall away like that to keep the test to a few seconds.
    const DOCUMENTS: usize = 4000;
    const CUT_EVERYWHERE: usize = 150;
    const CUT_IN_PAIRS: usize = 10;

    /// splitmix64. The documents are the same on every run, so a failure
    /// can be looked at again by its number.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
            from[self.below(from.len())]
        }

        fn one_in(&mut self, n: usize) -> bool {
            self.below(n) == 0
        }
    }

    /// Text as a browser reads it: characters that look like the start of
    /// markup but are not, and characters that take more than one byte, so
    /// that a chunk boundary can fall inside one.
    const TEXT: &[&str] = &[
        "x",
        "Hello world",
        "  two  spaces  ",
        "a < b",
        "3 & 4",
        "&amp;x",
        "<3",
        "é×ü",
        "中文字",
        "🎉 ok",
        "line\nbreak",
        "`tick`",
        "a=b",
        "/slash/",
        "-- dashes --",
    ];

    const COMMENTS: &[&str] = &[
        "<!--c-->",
        "<!-- <div id=\"a\" data-open=\"\">text</div> -->",
        "<!-- a -- b -->",
        "<!--nr-l:1a2b-->",
        "<!---->",
    ];

    /// One attribute, with the quoting a browser would accept for the
    /// value it holds.
    fn attribute(rng: &mut Rng, out: &mut String) {
        const NAMES: &[&str] = &[
            "class",
            "title",
            "id",
            "for",
            "aria-controls",
            "aria-describedby",
            "href",
            "style",
            "type",
            "value",
            "name",
            "hidden",
            "required",
            "download",
            "method",
            "xlink:href",
            "data-open",
            "data-x",
            "data-component",
            "data-strength-for",
            "DATA-OPEN",
            "TITLE",
            "viewBox",
            "d",
        ];
        const VALUES: &[&str] = &[
            "",
            "a",
            "a b",
            "a  b",
            "auth-email",
            "#auth-email",
            "auth-email other",
            "/a/b?x=1&y=2",
            "a>b",
            "a<b",
            "a=b",
            "say \"hi\"",
            "it's",
            "a`b",
            "&quot;q&quot;",
            "text/javascript",
            "text/css",
            "get",
            "--brand:red",
            "M1.23456 2.5L.98765-1.00049",
            "0 0 24 24",
            "é×",
            "nr-b1",
            "/x/",
        ];
        out.push_str(rng.pick(NAMES));
        if rng.one_in(6) {
            // A boolean attribute: no value at all.
            return;
        }
        let value = rng.pick(VALUES);
        out.push('=');
        // A value with no space and no `>` in it can stand without quotes,
        // whatever else it holds.
        let bare = !value.is_empty() && !value.bytes().any(|c| c.is_ascii_whitespace() || matches!(c, b'"' | b'>'));
        match rng.below(3) {
            0 if bare => out.push_str(value),
            _ if !value.contains('"') => {
                out.push('"');
                out.push_str(value);
                out.push('"');
            }
            _ => {
                out.push('\'');
                out.push_str(value);
                out.push('\'');
            }
        }
    }

    fn start_tag(rng: &mut Rng, tag: &str, out: &mut String) {
        out.push('<');
        out.push_str(tag);
        for _ in 0..rng.below(4) {
            out.push(' ');
            attribute(rng, out);
        }
        out.push('>');
    }

    fn end_tag(tag: &str, out: &mut String) {
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }

    /// An element with no content and no end tag, closed in each of the
    /// three ways HTML allows.
    fn void_element(rng: &mut Rng, out: &mut String) {
        let tag = rng.pick(&["br", "input", "img", "hr", "meta", "wbr"]);
        out.push('<');
        out.push_str(tag);
        for _ in 0..rng.below(3) {
            out.push(' ');
            attribute(rng, out);
        }
        out.push_str(rng.pick(&[">", "/>", " />"]));
    }

    /// An element whose content is text to the browser: the rewriter has
    /// to copy it out as it stands (a stylesheet aside, which it renames).
    fn raw_element(rng: &mut Rng, out: &mut String) {
        let (tag, content) = match rng.below(5) {
            0 => (
                "script",
                rng.pick(&[
                    "if (a < b && c > d) { x = \"<div data-open=''>\" }",
                    "var s = \"<\\/script\"; el.dataset.open = ''",
                    "{\"data-open\":\"<\\/script\",\"a\":\"</p>\"}",
                    "",
                ]),
            ),
            1 => (
                "style",
                rng.pick(&[
                    "[data-open]{color:var(--brand);animation:fade 1s}",
                    "@layer components{#auth-email{top:0}}@keyframes fade{to{opacity:1}}",
                    "a{content:\"</style\"}b[data-open]{--brand:1}",
                    "@layer é-x{a{color:red}}",
                ]),
            ),
            2 => ("textarea", rng.pick(&["  <b data-open=\"\">  \n kept  ", "</li> & < >", "é×"])),
            3 => ("title", rng.pick(&["A & B < C", "<b data-open=\"\">", "×"])),
            _ => ("iframe", rng.pick(&["<p>fallback</p>", ""])),
        };
        start_tag(rng, tag, out);
        out.push_str(content);
        end_tag(tag, out);
    }

    /// Elements whose end tag HTML makes optional, in the parent they
    /// usually have and in parents that make the end tag necessary.
    fn optional_end_tags(rng: &mut Rng, depth: usize, out: &mut String) {
        let (parent, child, other) = match rng.below(6) {
            0 => (rng.pick(&["ul", "ol", "menu", "x-widget", "template", "a", "nr-b"]), "li", "li"),
            1 => (rng.pick(&["dl", "x-widget", "template", "a"]), "dt", "dd"),
            2 => (rng.pick(&["select", "x-widget", "template", "a"]), "option", "option"),
            3 => (rng.pick(&["select", "x-widget", "template"]), "optgroup", "optgroup"),
            4 => (rng.pick(&["tr", "x-widget", "template", "a"]), "td", "th"),
            _ => (rng.pick(&["div", "nr-island", "template", "a", "x-widget"]), "p", "p"),
        };
        start_tag(rng, parent, out);
        for tag in [child, other] {
            start_tag(rng, tag, out);
            node(rng, depth.saturating_sub(1), out);
            end_tag(tag, out);
            // A space, a comment or text between two of them keeps the end
            // tag: it would land inside the element otherwise.
            out.push_str(rng.pick(&["", "", "", " ", "<!--x-->", "t"]));
        }
        end_tag(parent, out);
    }

    fn table(rng: &mut Rng, depth: usize, out: &mut String) {
        start_tag(rng, "table", out);
        for section in ["thead", "tbody", "tfoot"] {
            if rng.one_in(3) {
                continue;
            }
            start_tag(rng, section, out);
            for _ in 0..1 + rng.below(2) {
                start_tag(rng, "tr", out);
                for cell in ["th", "td"] {
                    start_tag(rng, cell, out);
                    node(rng, depth.saturating_sub(1), out);
                    end_tag(cell, out);
                }
                end_tag("tr", out);
            }
            end_tag(section, out);
        }
        end_tag("table", out);
    }

    fn svg(rng: &mut Rng, out: &mut String) {
        out.push_str("<svg viewBox=\"0 0 24 24\">");
        out.push_str(rng.pick(&[
            "<path d=\"M1.23456 2.5L.98765-1.00049\"/>",
            "<path d=\"a1 1 0 01.5.5\"/>",
            "<path d=\"M2.5e-7 1.23456e3\" fill=\"url(#auth-icon)\"/>",
            "<path d=M+1.23456-.56789 />",
            "<use href=\"#auth-icon\"/>",
            "<polygon points=\"1.00049,2.5 3,4\"/>",
        ]));
        out.push_str("</svg>");
    }

    /// One piece of a document: text, a comment, or an element of some
    /// kind with more inside it.
    fn node(rng: &mut Rng, depth: usize, out: &mut String) {
        match rng.below(if depth == 0 { 4 } else { 14 }) {
            0 => out.push_str(rng.pick(TEXT)),
            1 => out.push_str(rng.pick(COMMENTS)),
            2 => void_element(rng, out),
            3 => raw_element(rng, out),
            4 | 5 => {
                let tag = rng.pick(&["div", "span", "a", "section", "nr-island", "x-widget", "template", "nr-b", "b"]);
                start_tag(rng, tag, out);
                for _ in 0..1 + rng.below(2) {
                    node(rng, depth - 1, out);
                }
                end_tag(tag, out);
            }
            6 | 7 => optional_end_tags(rng, depth, out),
            8 => table(rng, depth, out),
            9 => svg(rng, out),
            10 => {
                // Text that has to keep every space of it.
                out.push_str("<pre>  <b class=\"x\">kept  </b>\n");
                out.push_str(rng.pick(TEXT));
                out.push_str("</pre>");
            }
            11 => {
                start_tag(rng, "p", out);
                out.push_str(rng.pick(TEXT));
                end_tag("p", out);
                out.push_str(rng.pick(&["", " ", "<!--after-->", "\n", "tail"]));
            }
            12 => {
                out.push_str("<template id=\"nr-t1\">");
                node(rng, depth - 1, out);
                out.push_str("</template>");
            }
            _ => {
                out.push_str("<nr-b id=\"nr-b1\">");
                node(rng, depth - 1, out);
                out.push_str("</nr-b>");
            }
        }
    }

    /// A document of a few elements. They stay small on purpose: the
    /// chunk checks below cost a pass over a document per boundary, and a
    /// pass per pair of boundaries, so a long one is quickly a second of
    /// test time on its own.
    fn document(rng: &mut Rng) -> String {
        let mut out = String::new();
        if rng.one_in(3) {
            out.push_str("<!DOCTYPE html>");
        }
        // Now and then a deeper one, for the way the pieces meet.
        let depth = if rng.one_in(8) { 3 } else { 2 };
        for _ in 0..1 + rng.below(2) {
            node(rng, depth, &mut out);
        }
        out
    }

    /// The `>` that ends a tag, the way a browser finds it: a quote only
    /// opens a value when it comes right after the `=`.
    fn tag_end_of(b: &[u8], lt: usize) -> Option<usize> {
        let mut quote: Option<u8> = None;
        let mut eq = false;
        for (at, byte) in b.iter().enumerate().skip(lt + 1) {
            match (quote, *byte) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, c @ (b'"' | b'\'')) if eq => quote = Some(c),
                (None, b'>') => return Some(at),
                _ => {}
            }
            if quote.is_none() && !byte.is_ascii_whitespace() {
                eq = *byte == b'=';
            }
        }
        None
    }

    /// What a browser takes as text: everything outside tags and comments,
    /// and, kept apart, the contents of the elements whose content is text
    /// (`<pre>` is not one of those: its text is in the first list, spaces
    /// and all). Read straight from the tokenizer's rules and not from the
    /// rewriter's, so that text turned into markup, or markup into text,
    /// shows up as a difference.
    fn text_of(html: &str) -> (String, Vec<(String, String)>) {
        let b = html.as_bytes();
        let mut text: Vec<u8> = Vec::new();
        let mut raw: Vec<(String, String)> = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] != b'<' {
                text.push(b[i]);
                i += 1;
                continue;
            }
            if b[i..].starts_with(b"<!--") {
                i = html[i + 4..].find("-->").map_or(b.len(), |e| i + 4 + e + 3);
                continue;
            }
            if b[i..].starts_with(b"<!") {
                // A doctype or a bogus comment: a node of its own, not text.
                i = html[i..].find('>').map_or(b.len(), |e| i + e + 1);
                continue;
            }
            let closing = b[i..].starts_with(b"</");
            let name_at = if closing { i + 2 } else { i + 1 };
            if !b.get(name_at).is_some_and(u8::is_ascii_alphabetic) {
                // `<` on its own, `<3`: text.
                text.push(b[i]);
                i += 1;
                continue;
            }
            // A tag with no `>` is dropped at the end of a document.
            let Some(gt) = tag_end_of(b, i) else { break };
            let name_end =
                name_at + b[name_at..gt].iter().take_while(|c| !c.is_ascii_whitespace() && **c != b'/').count();
            let name = html[name_at..name_end].to_ascii_lowercase();
            i = gt + 1;
            if closing || !RAW_TEXT.contains(&name.as_str()) {
                continue;
            }
            let mut at = i;
            let end = loop {
                let Some(lt) = html[at..].find("</").map(|p| at + p) else { break b.len() };
                let after = lt + 2 + name.len();
                if b.get(lt + 2..after).is_some_and(|n| n.eq_ignore_ascii_case(name.as_bytes()))
                    && b.get(after).is_some_and(|c| c.is_ascii_whitespace() || matches!(*c, b'/' | b'>'))
                {
                    break lt;
                }
                at = lt + 2;
            };
            raw.push((name, html[i..end].to_owned()));
            i = end;
        }
        (String::from_utf8_lossy(&text).into_owned(), raw)
    }

    /// The document fed in pieces, the way `apply` feeds a stream: what a
    /// call leaves unused waits for the next piece.
    fn in_pieces(f: &Finisher, doc: &[u8], cuts: &[usize]) -> String {
        let mut out = Vec::new();
        let mut carry: Vec<u8> = Vec::new();
        let mut open = Open::new();
        let mut from = 0;
        for (n, to) in cuts.iter().copied().chain([doc.len()]).enumerate() {
            carry.extend_from_slice(&doc[from..to]);
            from = to;
            let used = f.process(&carry.clone(), n == cuts.len(), &mut open, &mut out);
            carry.drain(..used);
        }
        String::from_utf8(out).expect("the pieces put back together are still UTF-8")
    }

    /// Generated documents, each one checked against everything this step
    /// promises: the same text, the same document however the stream is
    /// cut, nothing gained by running it twice, and never a longer
    /// response than it was given.
    #[test]
    fn generated_documents_keep_their_text_and_their_shape() {
        let both = Finisher::new(NAMES, Markup { compact: true, svg_precision: 3 }).unwrap();
        let plain = compact();
        let mut rng = Rng(0x5eed_1234_5678_9abc);
        let (mut everywhere, mut in_pairs) = (0, 0);
        for n in 0..DOCUMENTS {
            let doc = document(&mut rng);
            let bytes = doc.as_bytes();
            let out = both.finish(&doc);
            // Text, in and out of the elements whose content is text. A
            // stylesheet is the one piece of text this step does rewrite.
            let apart = |raw: Vec<(String, String)>| -> Vec<(String, String)> {
                raw.into_iter().filter(|(tag, _)| tag != "style").collect()
            };
            let (text, raw) = text_of(&doc);
            let (text_after, raw_after) = text_of(&out);
            assert_eq!(text, text_after, "document {n}: the text changed\nin  {doc}\nout {out}");
            assert_eq!(
                apart(raw),
                apart(raw_after),
                "document {n}: text inside an element changed\nin  {doc}\nout {out}"
            );
            // Nothing left to take out: an end tag that stayed did so
            // because of what is open around it, which a second pass reads
            // the same way.
            assert_eq!(both.finish(&out), out, "document {n}: a second pass changes it\nin  {doc}\nout {out}");
            // Compact markup on its own: a response never grows (what a
            // short name costs is the build's to decide, not this step's).
            let small = plain.finish(&doc);
            assert!(
                small.len() <= doc.len(),
                "document {n}: grew by {}\nin  {doc}\nout {small}",
                small.len() - doc.len()
            );
            assert_eq!(
                text_of(&doc),
                text_of(&small),
                "document {n}: compact markup changed the text\nin  {doc}\nout {small}"
            );
            assert_eq!(plain.finish(&small), small, "document {n}: a second compact pass changes it\nin  {doc}");
            // Every chunk boundary, including the ones inside a character.
            if everywhere < CUT_EVERYWHERE && bytes.len() <= 400 {
                everywhere += 1;
                for cut in 1..bytes.len() {
                    assert_eq!(in_pieces(&both, bytes, &[cut]), out, "document {n}: cut at {cut}\nin  {doc}");
                    let second = cut + 1 + (cut * 37 + n * 11) % (bytes.len() - cut);
                    assert_eq!(
                        in_pieces(&both, bytes, &[cut, second.min(bytes.len())]),
                        out,
                        "document {n}: cut at {cut} and {second}\nin  {doc}"
                    );
                }
            }
            if in_pairs < CUT_IN_PAIRS && bytes.len() <= 120 {
                in_pairs += 1;
                for first in 1..bytes.len() {
                    for second in first + 1..=bytes.len() {
                        assert_eq!(
                            in_pieces(&both, bytes, &[first, second]),
                            out,
                            "document {n}: cut at {first} and {second}\nin  {doc}"
                        );
                    }
                }
            }
        }
    }
}
