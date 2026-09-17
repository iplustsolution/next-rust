//! Short class names for production builds.
//!
//! Release builds of Tailwind projects rename every utility class to a short,
//! random name (`rounded-lg` → `k7`). The build renames the selectors in the
//! generated CSS and hands the renaming table to the app at startup; the
//! renderer then writes the short names wherever the original ones appear in
//! `class` attributes, in [`active_class`](crate::active_class) values, in
//! `data-nr-class-<name>` attributes and in `class="…"` inside raw HTML.

use std::borrow::Cow;
use std::sync::OnceLock;

/// `(original, short)` pairs, sorted by original name.
pub type ClassNames = &'static [(&'static str, &'static str)];

static CLASS_NAMES: OnceLock<ClassNames> = OnceLock::new();

/// Install the renaming table. Called once by the server at startup; later
/// calls are ignored. An empty table leaves class names unchanged.
pub fn set_class_names(names: ClassNames) {
    if !names.is_empty() {
        debug_assert!(names.is_sorted_by(|a, b| a.0 < b.0), "class names must be sorted");
        let _ = CLASS_NAMES.set(names);
    }
}

fn table() -> Option<ClassNames> {
    CLASS_NAMES.get().copied()
}

fn lookup(names: ClassNames, class: &str) -> Option<&'static str> {
    names.binary_search_by(|(original, _)| (*original).cmp(class)).ok().map(|i| names[i].1)
}

/// The short name of one class, if it has one.
pub fn short_class_name(class: &str) -> Option<&'static str> {
    lookup(table()?, class)
}

/// Rename the classes of a space-separated class list.
pub(crate) fn map_class_list(list: &str) -> Cow<'_, str> {
    match table() {
        Some(names) => map_list(names, list),
        None => Cow::Borrowed(list),
    }
}

fn map_list<'a>(names: ClassNames, list: &'a str) -> Cow<'a, str> {
    if !list.split_ascii_whitespace().any(|c| lookup(names, c).is_some()) {
        return Cow::Borrowed(list);
    }
    let mut out = String::with_capacity(list.len());
    for class in list.split_ascii_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(lookup(names, class).unwrap_or(class));
    }
    Cow::Owned(out)
}

/// Rename the classes in `class="…"` / `class='…'` attributes of raw HTML.
pub(crate) fn map_raw_html(html: &str) -> Cow<'_, str> {
    match table() {
        Some(names) if html.contains("class=") => map_html(names, html),
        _ => Cow::Borrowed(html),
    }
}

fn map_html<'a>(names: ClassNames, html: &'a str) -> Cow<'a, str> {
    let b = html.as_bytes();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut search = 0;
    while let Some(found) = html[search..].find("class=") {
        let at = search + found;
        search = at + 6;
        // Only a `class` attribute inside a tag: preceded by whitespace and
        // followed by a quote.
        let attr_start = at > 0 && b[at - 1].is_ascii_whitespace();
        let Some(&quote) = b.get(at + 6) else { break };
        if !attr_start || (quote != b'"' && quote != b'\'') {
            continue;
        }
        let value_start = at + 7;
        let Some(len) = html[value_start..].find(quote as char) else { break };
        let value = &html[value_start..value_start + len];
        if let Cow::Owned(mapped) = map_list(names, value) {
            let out = out.get_or_insert_with(|| String::with_capacity(html.len()));
            out.push_str(&html[copied..value_start]);
            out.push_str(&mapped);
            copied = value_start + len;
        }
        search = value_start + len;
    }
    match out {
        Some(mut out) => {
            out.push_str(&html[copied..]);
            Cow::Owned(out)
        }
        None => Cow::Borrowed(html),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: ClassNames = &[("hover:underline", "b"), ("mt-4", "a"), ("text-[13px]", "c")];

    #[test]
    fn maps_known_classes_and_keeps_the_rest() {
        assert_eq!(map_list(NAMES, "mt-4 card hover:underline"), "a card b");
        assert!(matches!(map_list(NAMES, "card  tk"), Cow::Borrowed("card  tk")));
        assert_eq!(map_list(NAMES, "  mt-4\ttext-[13px] "), "a c");
    }

    #[test]
    fn maps_class_attributes_in_raw_html() {
        let html = r#"<p class="mt-4 x">class="mt-4"</p><span data-class="mt-4" class='text-[13px]'>"#;
        assert_eq!(map_html(NAMES, html), r#"<p class="a x">class="mt-4"</p><span data-class="mt-4" class='c'>"#);
        assert!(matches!(map_html(NAMES, "<b class=\"tk\">"), Cow::Borrowed(_)));
    }
}
