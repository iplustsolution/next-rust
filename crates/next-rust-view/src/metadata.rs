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
            alternates
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
            h.push_str(&format!(
                "<link rel=\"preload\" href=\"{}\" as=\"{}\"",
                escape_attr(&p.href),
                escape_attr(&p.as_)
            ));
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
}
