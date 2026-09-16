//! Generated `sitemap.xml` and `robots.txt` (`app/sitemap.rs`, `app/robots.rs`).

use crate::response::{IntoResponse, Response};

/// `pub async fn sitemap() -> Sitemap`
#[derive(Debug, Clone, Default)]
pub struct Sitemap {
    pub entries: Vec<SitemapEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct SitemapEntry {
    pub url: String,
    /// W3C datetime, e.g. `2026-09-17`.
    pub last_modified: Option<String>,
    pub change_frequency: Option<String>,
    pub priority: Option<f32>,
}

impl Sitemap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.entries.push(SitemapEntry { url: url.into(), ..Default::default() });
        self
    }

    pub fn entry(mut self, entry: SitemapEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn to_xml(&self) -> String {
        let esc = |s: &str| next_rust_view::escape_attr(s).into_owned();
        let mut x = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        for e in &self.entries {
            x.push_str("  <url>\n    <loc>");
            x.push_str(&esc(&e.url));
            x.push_str("</loc>\n");
            if let Some(v) = &e.last_modified {
                x.push_str(&format!("    <lastmod>{}</lastmod>\n", esc(v)));
            }
            if let Some(v) = &e.change_frequency {
                x.push_str(&format!("    <changefreq>{}</changefreq>\n", esc(v)));
            }
            if let Some(v) = e.priority {
                x.push_str(&format!("    <priority>{:.1}</priority>\n", v.clamp(0.0, 1.0)));
            }
            x.push_str("  </url>\n");
        }
        x.push_str("</urlset>\n");
        x
    }
}

impl IntoResponse for Sitemap {
    fn into_response(self) -> Response {
        Response::new(http::StatusCode::OK, crate::response::Body::Bytes(self.to_xml().into()))
            .with_content_type("application/xml")
            .with_cache_control("public, max-age=3600")
    }
}

/// `pub async fn robots() -> Robots`
#[derive(Debug, Clone, Default)]
pub struct Robots {
    pub rules: Vec<RobotsRule>,
    pub sitemap: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RobotsRule {
    pub user_agent: String,
    pub allow: Vec<String>,
    pub disallow: Vec<String>,
    pub crawl_delay: Option<u32>,
}

impl Robots {
    /// Allow everything for all user agents.
    pub fn allow_all() -> Self {
        Robots {
            rules: vec![RobotsRule { user_agent: "*".into(), allow: vec!["/".into()], ..Default::default() }],
            ..Default::default()
        }
    }

    pub fn rule(mut self, rule: RobotsRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn sitemap(mut self, url: impl Into<String>) -> Self {
        self.sitemap = Some(url.into());
        self
    }

    pub fn to_text(&self) -> String {
        let clean = |s: &str| s.replace(['\r', '\n'], "");
        let mut t = String::new();
        for r in &self.rules {
            t.push_str(&format!("User-Agent: {}\n", clean(&r.user_agent)));
            for a in &r.allow {
                t.push_str(&format!("Allow: {}\n", clean(a)));
            }
            for d in &r.disallow {
                t.push_str(&format!("Disallow: {}\n", clean(d)));
            }
            if let Some(c) = r.crawl_delay {
                t.push_str(&format!("Crawl-delay: {c}\n"));
            }
            t.push('\n');
        }
        if let Some(h) = &self.host {
            t.push_str(&format!("Host: {}\n", clean(h)));
        }
        if let Some(s) = &self.sitemap {
            t.push_str(&format!("Sitemap: {}\n", clean(s)));
        }
        t
    }
}

impl IntoResponse for Robots {
    fn into_response(self) -> Response {
        Response::text(self.to_text()).with_cache_control("public, max-age=3600")
    }
}
