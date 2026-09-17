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

/// A syndication feed, rendered as RSS 2.0, Atom 1.0 or JSON Feed 1.1.
///
/// Serve it from an API route, one URL per format:
///
/// ```ignore
/// // app/feed.xml/route.rs
/// pub async fn GET() -> Response {
///     Response::xml(feed().to_rss())
/// }
/// ```
///
/// Dates are `YYYY-MM-DD` or full RFC 3339 (`2026-09-18T09:30:00Z`); each
/// format receives the shape it requires.
#[derive(Debug, Clone, Default)]
pub struct Feed {
    pub title: String,
    /// The site the feed belongs to, e.g. `https://example.com`.
    pub site_url: String,
    /// This feed's own URL, e.g. `https://example.com/feed.xml`.
    pub feed_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    /// When the feed last changed. Defaults to the newest entry's date.
    pub updated: Option<String>,
    pub icon: Option<String>,
    pub author: Option<String>,
    pub entries: Vec<FeedEntry>,
}

/// One item in a [`Feed`].
#[derive(Debug, Clone, Default)]
pub struct FeedEntry {
    pub url: String,
    pub title: String,
    /// A short plain-text summary.
    pub summary: Option<String>,
    /// The full entry as HTML.
    pub content_html: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    /// Stable identifier. Defaults to `url`.
    pub id: Option<String>,
    /// A representative image.
    pub image: Option<String>,
}

impl FeedEntry {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self { url: url.into(), title: title.into(), ..Default::default() }
    }

    pub fn summary(mut self, text: impl Into<String>) -> Self {
        self.summary = Some(text.into());
        self
    }

    pub fn content_html(mut self, html: impl Into<String>) -> Self {
        self.content_html = Some(html.into());
        self
    }

    pub fn published(mut self, date: impl Into<String>) -> Self {
        self.published = Some(date.into());
        self
    }

    pub fn updated(mut self, date: impl Into<String>) -> Self {
        self.updated = Some(date.into());
        self
    }

    pub fn author(mut self, name: impl Into<String>) -> Self {
        self.author = Some(name.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image = Some(url.into());
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    fn identifier(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.url)
    }
}

impl Feed {
    pub fn new(title: impl Into<String>, site_url: impl Into<String>, feed_url: impl Into<String>) -> Self {
        Self { title: title.into(), site_url: site_url.into(), feed_url: feed_url.into(), ..Default::default() }
    }

    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn updated(mut self, date: impl Into<String>) -> Self {
        self.updated = Some(date.into());
        self
    }

    pub fn icon(mut self, url: impl Into<String>) -> Self {
        self.icon = Some(url.into());
        self
    }

    pub fn author(mut self, name: impl Into<String>) -> Self {
        self.author = Some(name.into());
        self
    }

    pub fn entry(mut self, entry: FeedEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn entries(mut self, entries: impl IntoIterator<Item = FeedEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// The feed's timestamp: the explicit one, else the newest entry's.
    fn last_updated(&self) -> Option<String> {
        self.updated
            .clone()
            .or_else(|| self.entries.iter().filter_map(|e| e.updated.clone().or_else(|| e.published.clone())).max())
    }

    /// RSS 2.0.
    pub fn to_rss(&self) -> String {
        let esc = |s: &str| next_rust_view::escape_text(s).into_owned();
        let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        x.push_str("<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n<channel>\n");
        x.push_str(&format!("  <title>{}</title>\n", esc(&self.title)));
        x.push_str(&format!("  <link>{}</link>\n", esc(&self.site_url)));
        x.push_str(&format!(
            "  <atom:link href=\"{}\" rel=\"self\" type=\"application/rss+xml\"/>\n",
            next_rust_view::escape_attr(&self.feed_url)
        ));
        x.push_str(&format!("  <description>{}</description>\n", esc(self.description.as_deref().unwrap_or(""))));
        if let Some(l) = &self.language {
            x.push_str(&format!("  <language>{}</language>\n", esc(l)));
        }
        if let Some(d) = self.last_updated().as_deref().and_then(rfc2822) {
            x.push_str(&format!("  <lastBuildDate>{d}</lastBuildDate>\n"));
        }
        for e in &self.entries {
            x.push_str("  <item>\n");
            x.push_str(&format!("    <title>{}</title>\n", esc(&e.title)));
            x.push_str(&format!("    <link>{}</link>\n", esc(&e.url)));
            x.push_str(&format!("    <guid isPermaLink=\"false\">{}</guid>\n", esc(e.identifier())));
            if let Some(d) = e.published.as_deref().and_then(rfc2822) {
                x.push_str(&format!("    <pubDate>{d}</pubDate>\n"));
            }
            if let Some(a) = e.author.as_deref().or(self.author.as_deref()) {
                x.push_str(&format!(
                    "    <dc:creator xmlns:dc=\"http://purl.org/dc/elements/1.1/\">{}</dc:creator>\n",
                    esc(a)
                ));
            }
            if let Some(s) = &e.summary {
                x.push_str(&format!("    <description>{}</description>\n", esc(s)));
            }
            if let Some(c) = &e.content_html {
                x.push_str(&format!(
                    "    <content:encoded xmlns:content=\"http://purl.org/rss/1.0/modules/content/\"><![CDATA[{}]]></content:encoded>\n",
                    c.replace("]]>", "]]]]><![CDATA[>")
                ));
            }
            for t in &e.tags {
                x.push_str(&format!("    <category>{}</category>\n", esc(t)));
            }
            x.push_str("  </item>\n");
        }
        x.push_str("</channel>\n</rss>\n");
        x
    }

    /// Atom 1.0.
    pub fn to_atom(&self) -> String {
        let esc = |s: &str| next_rust_view::escape_text(s).into_owned();
        let attr = |s: &str| next_rust_view::escape_attr(s).into_owned();
        let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        x.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
        x.push_str(&format!("  <title>{}</title>\n", esc(&self.title)));
        x.push_str(&format!("  <id>{}</id>\n", esc(&self.site_url)));
        x.push_str(&format!("  <link href=\"{}\"/>\n", attr(&self.site_url)));
        x.push_str(&format!(
            "  <link href=\"{}\" rel=\"self\" type=\"application/atom+xml\"/>\n",
            attr(&self.feed_url)
        ));
        if let Some(d) = &self.description {
            x.push_str(&format!("  <subtitle>{}</subtitle>\n", esc(d)));
        }
        if let Some(i) = &self.icon {
            x.push_str(&format!("  <icon>{}</icon>\n", esc(i)));
        }
        if let Some(a) = &self.author {
            x.push_str(&format!("  <author><name>{}</name></author>\n", esc(a)));
        }
        x.push_str(&format!("  <updated>{}</updated>\n", rfc3339(self.last_updated().as_deref().unwrap_or(""))));
        for e in &self.entries {
            x.push_str("  <entry>\n");
            x.push_str(&format!("    <title>{}</title>\n", esc(&e.title)));
            x.push_str(&format!("    <id>{}</id>\n", esc(e.identifier())));
            x.push_str(&format!("    <link href=\"{}\"/>\n", attr(&e.url)));
            let updated = e.updated.as_deref().or(e.published.as_deref()).unwrap_or("");
            x.push_str(&format!("    <updated>{}</updated>\n", rfc3339(updated)));
            if let Some(p) = &e.published {
                x.push_str(&format!("    <published>{}</published>\n", rfc3339(p)));
            }
            if let Some(a) = e.author.as_deref().or(self.author.as_deref()) {
                x.push_str(&format!("    <author><name>{}</name></author>\n", esc(a)));
            }
            if let Some(s) = &e.summary {
                x.push_str(&format!("    <summary>{}</summary>\n", esc(s)));
            }
            if let Some(c) = &e.content_html {
                x.push_str(&format!("    <content type=\"html\">{}</content>\n", esc(c)));
            }
            for t in &e.tags {
                x.push_str(&format!("    <category term=\"{}\"/>\n", attr(t)));
            }
            x.push_str("  </entry>\n");
        }
        x.push_str("</feed>\n");
        x
    }

    /// JSON Feed 1.1.
    pub fn to_json(&self) -> String {
        let mut items = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let mut item = serde_json::Map::new();
            item.insert("id".into(), e.identifier().into());
            item.insert("url".into(), e.url.clone().into());
            item.insert("title".into(), e.title.clone().into());
            if let Some(s) = &e.summary {
                item.insert("summary".into(), s.clone().into());
            }
            match &e.content_html {
                Some(c) => item.insert("content_html".into(), c.clone().into()),
                None => item.insert("content_text".into(), e.summary.clone().unwrap_or_default().into()),
            };
            if let Some(p) = &e.published {
                item.insert("date_published".into(), rfc3339(p).into());
            }
            if let Some(u) = &e.updated {
                item.insert("date_modified".into(), rfc3339(u).into());
            }
            if let Some(i) = &e.image {
                item.insert("image".into(), i.clone().into());
            }
            if !e.tags.is_empty() {
                item.insert("tags".into(), e.tags.clone().into());
            }
            if let Some(a) = e.author.as_deref().or(self.author.as_deref()) {
                item.insert("authors".into(), serde_json::json!([{ "name": a }]));
            }
            items.push(serde_json::Value::Object(item));
        }
        let mut feed = serde_json::Map::new();
        feed.insert("version".into(), "https://jsonfeed.org/version/1.1".into());
        feed.insert("title".into(), self.title.clone().into());
        feed.insert("home_page_url".into(), self.site_url.clone().into());
        feed.insert("feed_url".into(), self.feed_url.clone().into());
        if let Some(d) = &self.description {
            feed.insert("description".into(), d.clone().into());
        }
        if let Some(l) = &self.language {
            feed.insert("language".into(), l.clone().into());
        }
        if let Some(i) = &self.icon {
            feed.insert("icon".into(), i.clone().into());
        }
        if let Some(a) = &self.author {
            feed.insert("authors".into(), serde_json::json!([{ "name": a }]));
        }
        feed.insert("items".into(), serde_json::Value::Array(items));
        serde_json::to_string_pretty(&serde_json::Value::Object(feed)).unwrap_or_else(|_| "{}".into())
    }
}

/// `2026-09-18` or `2026-09-18T09:30:00Z` → RFC 3339. Anything else is
/// passed through, so an already-formatted timestamp survives.
fn rfc3339(date: &str) -> String {
    match parse_date(date) {
        Some((y, m, d)) if date.len() == 10 => format!("{y:04}-{m:02}-{d:02}T00:00:00Z"),
        _ => date.to_owned(),
    }
}

/// `2026-09-18` → `Fri, 18 Sep 2026 00:00:00 +0000` (RSS wants RFC 822).
fn rfc2822(date: &str) -> Option<String> {
    let (y, m, d) = parse_date(date)?;
    let time = if date.len() > 10 { date.get(11..19).unwrap_or("00:00:00").to_owned() } else { "00:00:00".into() };
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    // Days since 1970-01-01, which was a Thursday.
    let days = days_from_civil(y as i64, m as i64, d as i64);
    let weekday = DAYS[days.rem_euclid(7) as usize];
    Some(format!("{weekday}, {d:02} {} {y:04} {time} +0000", MONTHS[(m - 1) as usize]))
}

fn parse_date(date: &str) -> Option<(u32, u32, u32)> {
    let bytes = date.as_bytes();
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let y = date.get(0..4)?.parse().ok()?;
    let m: u32 = date.get(5..7)?.parse().ok()?;
    let d: u32 = date.get(8..10)?.parse().ok()?;
    (1..=12).contains(&m).then_some(())?;
    (1..=31).contains(&d).then_some(())?;
    Some((y, m, d))
}

/// Days from 1970-01-01 (Howard Hinnant's algorithm, as in `http_date`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed() -> Feed {
        Feed::new("Next Rust", "https://next-rust.dev", "https://next-rust.dev/feed.xml")
            .description("Notes from the framework")
            .language("en")
            .author("The Next Rust team")
            .entry(
                FeedEntry::new("https://next-rust.dev/blog/one", "One & two")
                    .summary("A <summary>")
                    .content_html("<p>Body</p>")
                    .published("2026-09-18")
                    .tag("rust"),
            )
            .entry(FeedEntry::new("https://next-rust.dev/blog/older", "Older").published("2026-01-02T08:30:00Z"))
    }

    #[test]
    fn rss_escapes_and_dates_are_rfc2822() {
        let rss = feed().to_rss();
        assert!(rss.contains("<title>One &amp; two</title>"), "{rss}");
        assert!(rss.contains("<description>A &lt;summary&gt;</description>"), "{rss}");
        assert!(rss.contains("<![CDATA[<p>Body</p>]]>"), "{rss}");
        // 2026-09-18 is a Friday.
        assert!(rss.contains("<pubDate>Fri, 18 Sep 2026 00:00:00 +0000</pubDate>"), "{rss}");
        assert!(rss.contains("<lastBuildDate>Fri, 18 Sep 2026 00:00:00 +0000</lastBuildDate>"), "{rss}");
        assert!(rss.contains("rel=\"self\""), "{rss}");
        assert!(rss.contains("<category>rust</category>"), "{rss}");
    }

    #[test]
    fn atom_uses_rfc3339_and_keeps_full_timestamps() {
        let atom = feed().to_atom();
        assert!(atom.contains("<published>2026-09-18T00:00:00Z</published>"), "{atom}");
        assert!(atom.contains("<published>2026-01-02T08:30:00Z</published>"), "{atom}");
        assert!(atom.contains("<updated>2026-09-18T00:00:00Z</updated>"), "{atom}");
        assert!(atom.contains("<content type=\"html\">&lt;p&gt;Body&lt;/p&gt;</content>"), "{atom}");
        assert!(atom.contains("<author><name>The Next Rust team</name></author>"), "{atom}");
    }

    #[test]
    fn json_feed_has_the_required_fields() {
        let json: serde_json::Value = serde_json::from_str(&feed().to_json()).expect("valid JSON");
        assert_eq!(json["version"], "https://jsonfeed.org/version/1.1");
        assert_eq!(json["feed_url"], "https://next-rust.dev/feed.xml");
        assert_eq!(json["items"][0]["title"], "One & two");
        assert_eq!(json["items"][0]["content_html"], "<p>Body</p>");
        assert_eq!(json["items"][0]["date_published"], "2026-09-18T00:00:00Z");
        assert_eq!(json["items"][0]["authors"][0]["name"], "The Next Rust team");
        assert_eq!(json["items"][0]["tags"][0], "rust");
    }

    #[test]
    fn weekdays_are_right_across_centuries() {
        assert!(rfc2822("2000-02-29").unwrap().starts_with("Tue, 29 Feb 2000"));
        assert!(rfc2822("1970-01-01").unwrap().starts_with("Thu, 01 Jan 1970"));
        assert!(rfc2822("2026-12-25").unwrap().starts_with("Fri, 25 Dec 2026"));
        assert_eq!(rfc2822("not-a-date"), None);
        assert_eq!(rfc3339("whenever"), "whenever");
    }
}
