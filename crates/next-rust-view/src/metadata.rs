//! Document metadata (`<head>` contents).
//!
//! Layouts and pages each return a [`Metadata`]; the framework merges them
//! from the root layout down to the page with [`Metadata::merge`]: every
//! field set by a child overrides the parent's value, and a parent's
//! `title_template` (e.g. `"%s | Acme"`) is applied to titles defined *below*
//! it.

use crate::escape::{escape_attr, escape_text};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<String>,
    /// Applied to descendant titles; `%s` is replaced by the child title.
    pub title_template: Option<String>,
    /// When true, this title ignores ancestor templates.
    pub title_absolute: bool,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub canonical: Option<String>,
    pub robots: Option<String>,
    pub viewport: Option<String>,
    pub theme_color: Option<String>,
    pub color_scheme: Option<String>,
    pub manifest: Option<String>,
    pub icons: Option<Vec<Icon>>,
    pub open_graph: Option<OpenGraph>,
    pub twitter: Option<Twitter>,
    /// `hreflang` alternates: (language, url).
    pub alternates: Option<Vec<(String, String)>>,
    /// Extra `<meta name=.. content=..>` entries.
    pub other: Vec<(String, String)>,
    /// External stylesheets (`<link rel="stylesheet">`).
    pub stylesheets: Vec<String>,
    /// Preloaded resources (fonts, images).
    pub preloads: Vec<Preload>,
    /// Extra `<link>` tags: feeds, `preconnect`, `me`, …
    pub links: Vec<LinkTag>,
    /// Attributes of the `<html>` element: (name, value).
    pub html_attributes: Vec<(String, String)>,
    /// `theme-color` per media query: (media, color).
    pub theme_colors: Vec<(String, String)>,
    /// `Some(false)`: leave the `[analytics]` tags of `next-rust.toml` off
    /// this page. Writes nothing into `<head>` itself.
    pub analytics: Option<bool>,
}

/// A `<link>` tag beyond the ones [`Metadata`] models directly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkTag {
    pub rel: String,
    pub href: String,
    pub mime: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Icon {
    pub rel: String,
    pub href: String,
    pub sizes: Option<String>,
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenGraph {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub site_name: Option<String>,
    pub kind: Option<String>,
    pub locale: Option<String>,
    pub images: Vec<OgImage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OgImage {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub alt: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Twitter {
    pub card: Option<String>,
    pub site: Option<String>,
    pub creator: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preload {
    pub href: String,
    /// `font`, `image`, `style`, `script`, `fetch`.
    pub as_: String,
    pub mime: Option<String>,
    pub crossorigin: bool,
}

macro_rules! setters {
    ($($name:ident),*) => {$(
        pub fn $name(mut self, v: impl Into<String>) -> Self {
            self.$name = Some(v.into());
            self
        }
    )*};
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    setters!(title, title_template, description, canonical, robots, viewport, theme_color, color_scheme, manifest);

    /// A title that ignores ancestor templates.
    pub fn absolute_title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self.title_absolute = true;
        self
    }

    pub fn keywords<I, S>(mut self, v: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = Some(v.into_iter().map(Into::into).collect());
        self
    }

    pub fn authors<I, S>(mut self, v: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.authors = Some(v.into_iter().map(Into::into).collect());
        self
    }

    pub fn icon(mut self, href: impl Into<String>) -> Self {
        self.icons.get_or_insert_with(Vec::new).push(Icon {
            rel: "icon".into(),
            href: href.into(),
            sizes: None,
            mime: None,
        });
        self
    }

    /// An icon with its size and type, for browsers that pick one of several:
    /// `icon_sized("/favicon-32x32.png", "32x32", "image/png")`.
    pub fn icon_sized(mut self, href: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self {
        self.icons.get_or_insert_with(Vec::new).push(Icon {
            rel: "icon".into(),
            href: href.into(),
            sizes: Some(sizes.into()),
            mime: Some(mime.into()),
        });
        self
    }

    /// A `theme-color` for one media query, typically once for each color
    /// scheme: `theme_color_for("(prefers-color-scheme: dark)", "#0d1b2a")`.
    /// A plain [`theme_color`](Self::theme_color) is still written first, for
    /// browsers that ignore `media`. A child that sets any replaces the
    /// parent's list.
    pub fn theme_color_for(mut self, media: impl Into<String>, color: impl Into<String>) -> Self {
        self.theme_colors.push((media.into(), color.into()));
        self
    }

    pub fn apple_touch_icon(mut self, href: impl Into<String>) -> Self {
        self.icons.get_or_insert_with(Vec::new).push(Icon {
            rel: "apple-touch-icon".into(),
            href: href.into(),
            sizes: None,
            mime: None,
        });
        self
    }

    pub fn open_graph(mut self, og: OpenGraph) -> Self {
        self.open_graph = Some(og);
        self
    }

    pub fn twitter(mut self, tw: Twitter) -> Self {
        self.twitter = Some(tw);
        self
    }

    pub fn alternate(mut self, lang: impl Into<String>, href: impl Into<String>) -> Self {
        self.alternates.get_or_insert_with(Vec::new).push((lang.into(), href.into()));
        self
    }

    /// `analytics(false)` leaves the `[analytics]` tags of `next-rust.toml`
    /// off this page (and, set in a layout, off everything below it; a child
    /// can turn them back on). Verification tags are not affected.
    pub fn analytics(mut self, enabled: bool) -> Self {
        self.analytics = Some(enabled);
        self
    }

    /// The `robots` value asks search engines not to index the page.
    pub fn is_noindex(&self) -> bool {
        self.robots
            .as_deref()
            .is_some_and(|r| r.split(',').any(|d| matches!(d.trim().to_ascii_lowercase().as_str(), "noindex" | "none")))
    }

    pub fn meta(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.other.push((name.into(), content.into()));
        self
    }

    pub fn stylesheet(mut self, href: impl Into<String>) -> Self {
        self.stylesheets.push(href.into());
        self
    }

    /// Preload a font file (adds `crossorigin`, required for fonts).
    pub fn preload_font(mut self, href: impl Into<String>) -> Self {
        let href = href.into();
        let mime = crate_mime(&href);
        self.preloads.push(Preload { href, as_: "font".into(), mime, crossorigin: true });
        self
    }

    pub fn preload_image(mut self, href: impl Into<String>) -> Self {
        self.preloads.push(Preload { href: href.into(), as_: "image".into(), mime: None, crossorigin: false });
        self
    }

    /// A `<link>` tag: `link("preconnect", "https://cdn.example.com")`.
    pub fn link(mut self, rel: impl Into<String>, href: impl Into<String>) -> Self {
        self.links.push(LinkTag { rel: rel.into(), href: href.into(), ..Default::default() });
        self
    }

    /// An attribute of the `<html>` element, e.g. `html_attribute("data-theme", "ocean")`
    /// or `html_attribute("class", "dark")`, so a theme applies before any
    /// script runs. `lang` overrides `[app] lang` for the page.
    ///
    /// The element is written with the document: client-side navigations keep
    /// the attributes the page already has (and what scripts changed since).
    /// Names that are not valid attribute names and event handlers (`on*`) are
    /// never rendered; values are escaped, and `class` gets the same short
    /// names as every other class in release builds.
    pub fn html_attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        let value = value.into();
        if let Some(slot) = self.html_attributes.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = value;
        } else {
            self.html_attributes.push((name, value));
        }
        self
    }

    /// A feed readers can subscribe to. Browsers and feed readers look for
    /// `rel="alternate"` with a feed media type:
    /// `feed("RSS", "/feed.xml", "application/rss+xml")`.
    pub fn feed(mut self, title: impl Into<String>, href: impl Into<String>, mime: impl Into<String>) -> Self {
        self.links.push(LinkTag {
            rel: "alternate".into(),
            href: href.into(),
            mime: Some(mime.into()),
            title: Some(title.into()),
        });
        self
    }

    /// Merge `child` into `self` (child wins).
    pub fn merge(mut self, child: Metadata) -> Metadata {
        if let Some(t) = child.title {
            self.title = Some(match (&self.title_template, child.title_absolute) {
                (Some(tpl), false) => tpl.replace("%s", &t),
                _ => t,
            });
        }
        macro_rules! take {
            ($($f:ident),*) => {$( if child.$f.is_some() { self.$f = child.$f; } )*};
        }
        take!(
            title_template,
            description,
            keywords,
            authors,
            canonical,
            robots,
            viewport,
            theme_color,
            color_scheme,
            manifest,
            icons,
            open_graph,
            twitter,
            alternates,
            analytics
        );
        for (k, v) in child.other {
            if let Some(slot) = self.other.iter_mut().find(|(n, _)| *n == k) {
                slot.1 = v;
            } else {
                self.other.push((k, v));
            }
        }
        if !child.theme_colors.is_empty() {
            self.theme_colors = child.theme_colors;
        }
        for (name, value) in child.html_attributes {
            self = self.html_attribute(name, value);
        }
        for l in child.links {
            if !self.links.contains(&l) {
                self.links.push(l);
            }
        }
        for s in child.stylesheets {
            if !self.stylesheets.contains(&s) {
                self.stylesheets.push(s);
            }
        }
        for p in child.preloads {
            if !self.preloads.contains(&p) {
                self.preloads.push(p);
            }
        }
        self
    }

    /// Render `<head>` contents.
    pub fn render_head(&self) -> String {
        let mut h = String::with_capacity(512);
        h.push_str("<meta charset=\"utf-8\">");
        let viewport = self.viewport.as_deref().unwrap_or("width=device-width, initial-scale=1");
        meta_name(&mut h, "viewport", viewport);
        if let Some(t) = &self.title {
            h.push_str("<title>");
            h.push_str(&escape_text(t));
            h.push_str("</title>");
        }
        if let Some(d) = &self.description {
            meta_name(&mut h, "description", d);
        }
        if let Some(k) = &self.keywords {
            meta_name(&mut h, "keywords", &k.join(", "));
        }
        if let Some(a) = &self.authors {
            for author in a {
                meta_name(&mut h, "author", author);
            }
        }
        if let Some(r) = &self.robots {
            meta_name(&mut h, "robots", r);
        }
        if let Some(c) = &self.theme_color {
            meta_name(&mut h, "theme-color", c);
        }
        for (media, color) in &self.theme_colors {
            h.push_str(&format!(
                "<meta name=\"theme-color\" media=\"{}\" content=\"{}\">",
                escape_attr(media),
                escape_attr(color)
            ));
        }
        if let Some(c) = &self.color_scheme {
            meta_name(&mut h, "color-scheme", c);
        }
        if let Some(c) = &self.canonical {
            link(&mut h, "canonical", c, &[]);
        }
        if let Some(alts) = &self.alternates {
            for (lang, href) in alts {
                link(&mut h, "alternate", href, &[("hreflang", lang)]);
            }
        }
        if let Some(m) = &self.manifest {
            link(&mut h, "manifest", m, &[]);
        }
        for l in &self.links {
            let mut extra: Vec<(&str, &str)> = Vec::new();
            if let Some(t) = &l.mime {
                extra.push(("type", t));
            }
            if let Some(t) = &l.title {
                extra.push(("title", t));
            }
            link(&mut h, &l.rel, &l.href, &extra);
        }
        if let Some(icons) = &self.icons {
            for i in icons {
                let mut extra = Vec::new();
                if let Some(s) = &i.sizes {
                    extra.push(("sizes", s.as_str()));
                }
                if let Some(t) = &i.mime {
                    extra.push(("type", t.as_str()));
                }
                link(&mut h, &i.rel, &i.href, &extra);
            }
        }
        if let Some(og) = &self.open_graph {
            let title = og.title.as_ref().or(self.title.as_ref());
            let desc = og.description.as_ref().or(self.description.as_ref());
            for (p, v) in [
                ("og:title", title),
                ("og:description", desc),
                ("og:url", og.url.as_ref()),
                ("og:site_name", og.site_name.as_ref()),
                ("og:type", og.kind.as_ref()),
                ("og:locale", og.locale.as_ref()),
            ] {
                if let Some(v) = v {
                    meta_property(&mut h, p, v);
                }
            }
            for img in &og.images {
                meta_property(&mut h, "og:image", &img.url);
                if let Some(w) = img.width {
                    meta_property(&mut h, "og:image:width", &w.to_string());
                }
                if let Some(hh) = img.height {
                    meta_property(&mut h, "og:image:height", &hh.to_string());
                }
                if let Some(a) = &img.alt {
                    meta_property(&mut h, "og:image:alt", a);
                }
            }
        }
        if let Some(tw) = &self.twitter {
            for (n, v) in [
                ("twitter:card", tw.card.as_ref()),
                ("twitter:site", tw.site.as_ref()),
                ("twitter:creator", tw.creator.as_ref()),
                ("twitter:title", tw.title.as_ref().or(self.title.as_ref())),
                ("twitter:description", tw.description.as_ref().or(self.description.as_ref())),
            ] {
                if let Some(v) = v {
                    meta_name(&mut h, n, v);
                }
            }
            for img in &tw.images {
                meta_name(&mut h, "twitter:image", img);
            }
        }
        for (n, v) in &self.other {
            meta_name(&mut h, n, v);
        }
        for p in &self.preloads {
            // The same check every other `<link href>` gets: a preload never
            // navigates, but one sanitiser for one attribute is easier to
            // trust than two rules that differ by which branch wrote the tag.
            let href = if crate::escape::is_safe_url(&p.href) { p.href.as_str() } else { "#" };
            h.push_str(&format!("<link rel=\"preload\" href=\"{}\" as=\"{}\"", escape_attr(href), escape_attr(&p.as_)));
            if let Some(m) = &p.mime {
                h.push_str(&format!(" type=\"{}\"", escape_attr(m)));
            }
            if p.crossorigin {
                h.push_str(" crossorigin");
            }
            h.push('>');
        }
        for s in &self.stylesheets {
            link(&mut h, "stylesheet", s, &[]);
        }
        h
    }
}

fn crate_mime(href: &str) -> Option<String> {
    let ext = href.rsplit('.').next()?.to_ascii_lowercase();
    Some(
        match ext.as_str() {
            "woff2" => "font/woff2",
            "woff" => "font/woff",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            _ => return None,
        }
        .into(),
    )
}

fn meta_name(h: &mut String, name: &str, content: &str) {
    h.push_str(&format!("<meta name=\"{}\" content=\"{}\">", escape_attr(name), escape_attr(content)));
}

fn meta_property(h: &mut String, prop: &str, content: &str) {
    h.push_str(&format!("<meta property=\"{}\" content=\"{}\">", escape_attr(prop), escape_attr(content)));
}

fn link(h: &mut String, rel: &str, href: &str, extra: &[(&str, &str)]) {
    let href = if crate::escape::is_safe_url(href) { href } else { "#" };
    h.push_str(&format!("<link rel=\"{}\" href=\"{}\"", escape_attr(rel), escape_attr(href)));
    for (k, v) in extra {
        h.push_str(&format!(" {k}=\"{}\"", escape_attr(v)));
    }
    h.push('>');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytics_opt_out_merges_and_writes_nothing() {
        let layout = Metadata::new().analytics(false);
        assert_eq!(Metadata::default().merge(layout.clone()).merge(Metadata::new().title("x")).analytics, Some(false));
        assert_eq!(
            Metadata::default().merge(layout.clone()).merge(Metadata::new().analytics(true)).analytics,
            Some(true)
        );
        assert_eq!(layout.render_head(), Metadata::new().render_head());
        assert!(Metadata::new().robots("NoIndex, follow").is_noindex());
        assert!(Metadata::new().robots("none").is_noindex());
        assert!(!Metadata::new().robots("index, follow").is_noindex());
        assert!(!Metadata::new().is_noindex());
    }

    #[test]
    fn merges_with_templates() {
        let root = Metadata::new().title("Acme").title_template("%s | Acme").description("root");
        let blog = Metadata::new().title("Blog").title_template("%s – Blog | Acme");
        let post = Metadata::new().title("Hello").keywords(["rust"]);
        let merged = Metadata::default().merge(root.clone()).merge(blog).merge(post);
        assert_eq!(merged.title.as_deref(), Some("Hello – Blog | Acme"));
        assert_eq!(merged.description.as_deref(), Some("root"));
        let merged = Metadata::default().merge(root.clone());
        assert_eq!(merged.title.as_deref(), Some("Acme"), "template does not apply to the same segment");
        let merged = Metadata::default().merge(root).merge(Metadata::new().absolute_title("Standalone"));
        assert_eq!(merged.title.as_deref(), Some("Standalone"));
    }

    #[test]
    fn sized_icons_and_theme_colors_per_scheme() {
        let root = Metadata::new()
            .theme_color("#0d1b2a")
            .theme_color_for("(prefers-color-scheme: light)", "#fafaf7")
            .theme_color_for("(prefers-color-scheme: dark)", "#0d1b2a")
            .icon_sized("/favicon-32x32.png", "32x32", "image/png");
        let h = Metadata::default().merge(root.clone()).render_head();
        assert!(h.contains(r##"<meta name="theme-color" content="#0d1b2a">"##), "{h}");
        assert!(h.contains(r##"<meta name="theme-color" media="(prefers-color-scheme: light)" content="#fafaf7">"##));
        assert!(h.contains(r#"<link rel="icon" href="/favicon-32x32.png" sizes="32x32" type="image/png">"#));
        // A child's list replaces the parent's.
        let page = Metadata::new().theme_color_for("all", "\"x");
        let merged = Metadata::default().merge(root).merge(page);
        assert_eq!(merged.theme_colors, [("all".to_owned(), "\"x".to_owned())]);
        assert!(merged.render_head().contains(r#"media="all" content="&quot;x""#));
    }

    #[test]
    fn html_attributes_merge_child_wins() {
        let root = Metadata::new().html_attribute("data-theme", "pass-point").html_attribute("class", "dark");
        let page = Metadata::new().html_attribute("class", "light").html_attribute("lang", "de");
        let merged = Metadata::default().merge(root).merge(page);
        assert_eq!(
            merged.html_attributes,
            [
                ("data-theme".into(), "pass-point".into()),
                ("class".into(), "light".into()),
                ("lang".into(), "de".into())
            ]
        );
        // Setting a name twice keeps one entry.
        let m = Metadata::new().html_attribute("dir", "ltr").html_attribute("dir", "rtl");
        assert_eq!(m.html_attributes, [("dir".into(), "rtl".into())]);
    }

    #[test]
    fn renders_escaped_head() {
        let m = Metadata::new()
            .title("<script>")
            .description("a \"quote\"")
            .canonical("javascript:alert(1)")
            .open_graph(OpenGraph {
                images: vec![OgImage { url: "/og.png".into(), width: Some(1200), ..Default::default() }],
                ..Default::default()
            })
            .preload_font("/fonts/inter.woff2");
        let h = m.render_head();
        assert!(h.contains("<title>&lt;script&gt;</title>"));
        assert!(h.contains("content=\"a &quot;quote&quot;\""));
        assert!(h.contains("<link rel=\"canonical\" href=\"#\">"));
        assert!(h.contains("<meta property=\"og:title\" content=\"&lt;script&gt;\">"));
        assert!(h.contains("og:image:width"));
        assert!(h.contains("as=\"font\" type=\"font/woff2\" crossorigin"));
    }

    /// xorshift64*: seeded, so a failure is reproducible.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn text(&mut self) -> String {
            const PIECES: &[&str] = &[
                "\"",
                "'",
                "<",
                ">",
                "&",
                "`",
                "=",
                "/",
                " ",
                "\t",
                "\n",
                "\r",
                "\u{0}",
                "\u{7f}",
                "\u{feff}",
                "onload=alert(1)",
                "><script>alert(1)</script>",
                "\" onmouseover=\"alert(1)",
                "javascript:alert(1)",
                "jAvAsCrIpT:alert(1)",
                "java\u{0}script:alert(1)",
                "jAvAsCrIpT&#58;alert(1)",
                "&#106;avascript:alert(1)",
                "vbscript:x",
                "data:text/html,<script>alert(1)</script>",
                "data:image/svg+xml,<svg onload=alert(1)>",
                "data:image/png;base64,iVBOR",
                "blob:https://example.com/u",
                "filesystem:https://example.com/temporary/x",
                "\\evil.example",
                "//evil.example",
                "/ok",
                "https://example.com/ok",
                "héllo",
                "%s",
                "-->",
                "<!--",
            ];
            (0..self.below(5)).map(|_| PIECES[self.below(PIECES.len())]).collect()
        }

        fn maybe(&mut self) -> Option<String> {
            (!self.next().is_multiple_of(4)).then(|| self.text())
        }
    }

    /// Elements `render_head` is allowed to write, and the attributes they
    /// may carry. Anything else in the output came from a value.
    const HEAD_TAGS: &[&str] = &["meta", "title", "/title", "link"];
    const HEAD_ATTRS: &[&str] = &[
        "charset",
        "name",
        "content",
        "property",
        "rel",
        "href",
        "type",
        "title",
        "sizes",
        "hreflang",
        "media",
        "as",
        "crossorigin",
    ];

    /// Split `<head>` markup into (tag name, [(attribute, value)]) and the
    /// text between tags, refusing anything that is not a well-formed tag.
    type Tag = (String, Vec<(String, String)>);

    fn parse_head(html: &str) -> Result<(Vec<Tag>, String), String> {
        let b = html.as_bytes();
        let mut i = 0usize;
        let mut tags: Vec<Tag> = Vec::new();
        let mut text = String::new();
        while i < b.len() {
            let Some(lt) = html[i..].find('<').map(|o| i + o) else {
                text.push_str(&html[i..]);
                break;
            };
            text.push_str(&html[i..lt]);
            let mut j = lt + 1;
            let mut quote: Option<u8> = None;
            loop {
                let c = *b.get(j).ok_or("unterminated tag")?;
                match quote {
                    Some(q) if c == q => quote = None,
                    Some(_) => {}
                    None if c == b'"' || c == b'\'' => quote = Some(c),
                    None if c == b'>' => break,
                    None if c == b'<' => return Err(format!("`<` inside a tag: {:?}", &html[lt..=j])),
                    None => {}
                }
                j += 1;
            }
            let inner = &html[lt + 1..j];
            let mut parts = inner.splitn(2, ' ');
            let name = parts.next().unwrap_or("").to_owned();
            let mut attrs = Vec::new();
            let mut rest = parts.next().unwrap_or("");
            while !rest.trim_start().is_empty() {
                rest = rest.trim_start();
                let end_name = rest.find(['=', ' ']).unwrap_or(rest.len());
                let key = rest[..end_name].to_owned();
                rest = &rest[end_name..];
                match rest.strip_prefix('=') {
                    Some(after) => {
                        let quoted = after.strip_prefix('"').ok_or(format!("unquoted value for {key}"))?;
                        let end = quoted.find('"').ok_or(format!("unterminated value for {key}"))?;
                        attrs.push((key, quoted[..end].to_owned()));
                        rest = &quoted[end + 1..];
                    }
                    // A bare attribute such as `crossorigin`.
                    None => attrs.push((key, String::new())),
                }
            }
            tags.push((name, attrs));
            i = j + 1;
        }
        Ok((tags, text))
    }

    /// No metadata value can add a tag or an attribute to `<head>`, and no
    /// value reaches a URL attribute as a scheme that runs script.
    #[test]
    fn no_metadata_value_escapes_its_attribute() {
        let mut rng = Rng(0xC0FF_EE01_5EED);
        for _ in 0..3_000 {
            let mut m = Metadata::new();
            m.title = rng.maybe();
            m.title_template = rng.maybe();
            m.description = rng.maybe();
            m.keywords = Some((0..rng.below(3)).map(|_| rng.text()).collect());
            m.authors = Some((0..rng.below(3)).map(|_| rng.text()).collect());
            m.canonical = rng.maybe();
            m.robots = rng.maybe();
            m.viewport = rng.maybe();
            m.theme_color = rng.maybe();
            m.color_scheme = rng.maybe();
            m.manifest = rng.maybe();
            m.icons = Some(
                (0..rng.below(3))
                    .map(|_| Icon { rel: rng.text(), href: rng.text(), sizes: rng.maybe(), mime: rng.maybe() })
                    .collect(),
            );
            m.open_graph = Some(OpenGraph {
                title: rng.maybe(),
                description: rng.maybe(),
                url: rng.maybe(),
                site_name: rng.maybe(),
                kind: rng.maybe(),
                locale: rng.maybe(),
                images: (0..rng.below(3))
                    .map(|_| OgImage {
                        url: rng.text(),
                        width: Some(rng.next() as u32),
                        height: Some(rng.next() as u32),
                        alt: rng.maybe(),
                    })
                    .collect(),
            });
            m.twitter = Some(Twitter {
                card: rng.maybe(),
                site: rng.maybe(),
                creator: rng.maybe(),
                title: rng.maybe(),
                description: rng.maybe(),
                images: (0..rng.below(3)).map(|_| rng.text()).collect(),
            });
            m.alternates = Some((0..rng.below(3)).map(|_| (rng.text(), rng.text())).collect());
            m.other = (0..rng.below(3)).map(|_| (rng.text(), rng.text())).collect();
            m.stylesheets = (0..rng.below(3)).map(|_| rng.text()).collect();
            m.preloads = (0..rng.below(3))
                .map(|_| Preload {
                    href: rng.text(),
                    as_: rng.text(),
                    mime: rng.maybe(),
                    crossorigin: rng.next().is_multiple_of(2),
                })
                .collect();
            m.links = (0..rng.below(3))
                .map(|_| LinkTag { rel: rng.text(), href: rng.text(), mime: rng.maybe(), title: rng.maybe() })
                .collect();
            m.theme_colors = (0..rng.below(3)).map(|_| (rng.text(), rng.text())).collect();

            let head = m.render_head();
            let (tags, text) = parse_head(&head).unwrap_or_else(|e| panic!("{e}\n{head}"));
            for (name, attrs) in &tags {
                assert!(HEAD_TAGS.contains(&name.as_str()), "injected <{name}>\n{head}");
                for (key, value) in attrs {
                    assert!(HEAD_ATTRS.contains(&key.as_str()), "injected attribute {key}=\n{head}");
                    assert!(!key.starts_with("on"), "event handler {key}\n{head}");
                    assert!(!value.contains('"') && !value.contains('<'), "{key}={value:?}\n{head}");
                    if key == "href" {
                        assert!(crate::escape::is_safe_url(value), "script URL in href: {value:?}\n{head}");
                    }
                }
            }
            // Text between tags is only ever the title, and it is escaped.
            assert!(!text.contains('<') && !text.contains("&&"), "{text:?}\n{head}");
        }
    }

    /// Two things worth stating plainly rather than assuming.
    #[test]
    fn what_a_url_in_metadata_is_and_is_not_checked_for() {
        // `<link href>` is sanitised: a script URL becomes `#`.
        for bad in ["javascript:alert(1)", "java\u{0}script:alert(1)", "vbscript:x", "data:text/html,<b>"] {
            let h = Metadata::new().canonical(bad).render_head();
            assert!(h.contains(r##"<link rel="canonical" href="#">"##), "{bad}: {h}");
        }
        // An HTML entity is not a scheme once the attribute is escaped: the
        // browser reads `javascript&#38;#58;alert(1)`, a relative URL.
        let h = Metadata::new().canonical("jAvAsCrIpT&#58;alert(1)").render_head();
        assert!(h.contains(r#"href="jAvAsCrIpT&amp;#58;alert(1)""#), "{h}");
        // `og:image` and `twitter:image` are `content=` on a `<meta>`, not a
        // URL the browser will follow from the page, so they are escaped but
        // not scheme-checked. Nothing executes; it is worth knowing.
        let h = Metadata::new()
            .open_graph(OpenGraph {
                images: vec![OgImage { url: "javascript:alert(1)".into(), ..Default::default() }],
                ..Default::default()
            })
            .render_head();
        assert!(h.contains(r#"<meta property="og:image" content="javascript:alert(1)">"#), "{h}");
        // Red before the fix: a preload wrote its `href` with escaping but
        // without the scheme check that every other `<link>` gets.
        let h = Metadata::new().preload_image("javascript:alert(1)").render_head();
        assert!(h.contains(r##"<link rel="preload" href="#" as="image">"##), "{h}");
        // A URL that starts with `\` or `//` is a *relative* URL, not script,
        // so `is_safe_url` allows it and `<link>` keeps it. A canonical or
        // manifest built from request data can therefore point at another
        // origin — that is the caller's check to make, not this one's.
        let h = Metadata::new().canonical("\\evil.example").manifest("//evil.example/m").render_head();
        assert!(h.contains(r#"<link rel="canonical" href="\evil.example">"#), "{h}");
        assert!(h.contains(r#"<link rel="manifest" href="//evil.example/m">"#), "{h}");
    }

    #[test]
    fn merging_generated_metadata_terminates_and_keeps_the_head_safe() {
        let mut rng = Rng(0xBEEF_1234_0009);
        let start = std::time::Instant::now();
        let mut merged = Metadata::new();
        for _ in 0..2_000 {
            let child = Metadata::new()
                .title(rng.text())
                .title_template(rng.text())
                .meta(rng.text(), rng.text())
                .html_attribute(rng.text(), rng.text())
                .link(rng.text(), rng.text())
                .stylesheet(rng.text());
            merged = merged.merge(child);
        }
        // `%s` in a title is substituted into the *template*, never the other
        // way round, so a title full of `%s` cannot grow without bound.
        assert!(merged.title.as_deref().unwrap_or("").len() < 10_000, "{:?}", merged.title);
        assert!(parse_head(&merged.render_head()).is_ok());
        assert!(start.elapsed() < std::time::Duration::from_secs(10), "took {:?}", start.elapsed());
    }
}
