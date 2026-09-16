//! Local font helper.

use next_rust_view::{Metadata, Node, raw_html};

/// A self-hosted font.
///
/// ```ignore
/// static INTER: LazyLock<LocalFont> = LazyLock::new(|| {
///     LocalFont::new("Inter", asset!("fonts/inter-var.woff2")).weight("100 900")
/// });
/// // layout.rs
/// pub fn metadata() -> Metadata { INTER.metadata() }
/// pub fn Layout(children: Children) -> impl View { div![INTER.style(), children] }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFont {
    family: String,
    sources: Vec<String>,
    weight: String,
    style: String,
    display: String,
    fallback: String,
    preload: bool,
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, '"' | '\'' | '<' | '>' | ';' | '{' | '}' | '\\' | '\n' | '\r')).collect()
}

impl LocalFont {
    pub fn new(family: &str, url: &str) -> Self {
        LocalFont {
            family: clean(family),
            sources: vec![clean(url)],
            weight: "400".into(),
            style: "normal".into(),
            display: "swap".into(),
            fallback: "system-ui, sans-serif".into(),
            preload: true,
        }
    }

    /// Additional source (e.g. a `.woff` fallback).
    pub fn source(mut self, url: &str) -> Self {
        self.sources.push(clean(url));
        self
    }

    /// `"400"`, `"700"`, or a variable range like `"100 900"`.
    pub fn weight(mut self, w: &str) -> Self {
        self.weight = clean(w);
        self
    }

    pub fn style(mut self, s: &str) -> Self {
        self.style = clean(s);
        self
    }

    /// `font-display` (default `swap`).
    pub fn display(mut self, d: &str) -> Self {
        self.display = clean(d);
        self
    }

    pub fn fallback(mut self, f: &str) -> Self {
        self.fallback = clean(f);
        self
    }

    pub fn preload(mut self, p: bool) -> Self {
        self.preload = p;
        self
    }

    /// `font-family` value including fallbacks.
    pub fn family(&self) -> String {
        format!("\"{}\", {}", self.family, self.fallback)
    }

    pub fn css(&self) -> String {
        let srcs: Vec<String> = self
            .sources
            .iter()
            .map(|u| {
                let fmt = match u.rsplit('.').next() {
                    Some("woff2") => " format(\"woff2\")",
                    Some("woff") => " format(\"woff\")",
                    Some("ttf") => " format(\"truetype\")",
                    Some("otf") => " format(\"opentype\")",
                    _ => "",
                };
                format!("url(\"{u}\"){fmt}")
            })
            .collect();
        format!(
            "@font-face{{font-family:\"{}\";src:{};font-weight:{};font-style:{};font-display:{}}}",
            self.family,
            srcs.join(","),
            self.weight,
            self.style,
            self.display
        )
    }

    /// `<style>` element with the `@font-face` rule.
    pub fn style_node(&self) -> Node {
        raw_html(format!("<style>{}</style>", self.css()))
    }

    /// Metadata preloading the primary font file.
    pub fn metadata(&self) -> Metadata {
        match (self.preload, self.sources.first()) {
            (true, Some(src)) => Metadata::new().preload_font(src.clone()),
            _ => Metadata::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_css_is_sanitized() {
        let f = LocalFont::new("Inter\"</style>", "/fonts/inter.woff2").weight("100 900");
        assert_eq!(
            f.css(),
            "@font-face{font-family:\"Inter/style\";src:url(\"/fonts/inter.woff2\") format(\"woff2\");font-weight:100 900;font-style:normal;font-display:swap}"
        );
        assert_eq!(f.metadata().preloads.len(), 1);
    }
}
