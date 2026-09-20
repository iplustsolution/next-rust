//! `[verification]` and `[analytics]`: site-ownership proofs and measurement
//! tags, written by the framework from `next-rust.toml` alone.
//!
//! ```toml
//! [verification]
//! google = "abc…"                      # <meta name="google-site-verification">
//! bing = "0123…"                       # <meta name="msvalidate.01">
//! google_file = "google1234abcd.html"  # served at /google1234abcd.html
//!
//! [analytics]
//! google_analytics = "G-XXXXXXXXXX"
//! exclude_paths = ["/account/*"]
//! ```
//!
//! A key that is absent (or an empty string) writes nothing. Values are
//! checked when the config is validated: `NR0010` for `[verification]`,
//! `NR0011` for `[analytics]`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;

/// `[verification]`: ownership proofs for search engines and ad platforms.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VerificationConfig {
    /// Google Search Console: `<meta name="google-site-verification">`.
    pub google: String,
    /// Bing Webmaster Tools: `<meta name="msvalidate.01">`.
    pub bing: String,
    /// Yandex Webmaster: `<meta name="yandex-verification">`.
    pub yandex: String,
    /// Pinterest: `<meta name="p:domain_verify">`.
    pub pinterest: String,
    /// Meta Business domain verification: `<meta name="facebook-domain-verification">`.
    pub facebook: String,
    /// Baidu: `<meta name="baidu-site-verification">`.
    pub baidu: String,
    /// Naver Search Advisor: `<meta name="naver-site-verification">`.
    pub naver: String,
    /// Seznam Webmaster: `<meta name="seznam-wmt">`.
    pub seznam: String,
    /// Any other `<meta name="…" content="…">` proof.
    pub other: BTreeMap<String, String>,
    /// Google's HTML-file method: the file name (`google1234abcd.html`). The
    /// framework answers `/<name>` with the body Google expects.
    pub google_file: String,
    /// Bing's XML-file method: the code inside `BingSiteAuth.xml`. The
    /// framework answers `/BingSiteAuth.xml`.
    pub bing_file: String,
    /// Lines of `/ads.txt` (authorized digital sellers).
    pub ads_txt: Vec<String>,
    /// Lines of `/app-ads.txt`.
    pub app_ads_txt: Vec<String>,
    /// Android App Links: the JSON of `/.well-known/assetlinks.json`.
    pub assetlinks: String,
    /// iOS Universal Links: the JSON of
    /// `/.well-known/apple-app-site-association` (also answered at
    /// `/apple-app-site-association`).
    pub apple_app_site_association: String,
}

impl VerificationConfig {
    /// `(meta name, content)` for every proof that is set, in a fixed order.
    pub fn metas(&self) -> Vec<(&str, &str)> {
        let named = [
            ("google-site-verification", &self.google),
            ("msvalidate.01", &self.bing),
            ("yandex-verification", &self.yandex),
            ("p:domain_verify", &self.pinterest),
            ("facebook-domain-verification", &self.facebook),
            ("baidu-site-verification", &self.baidu),
            ("naver-site-verification", &self.naver),
            ("seznam-wmt", &self.seznam),
        ];
        named
            .into_iter()
            .map(|(name, value)| (name, value.trim()))
            .chain(self.other.iter().map(|(name, value)| (name.trim(), value.trim())))
            .filter(|(name, value)| !name.is_empty() && !value.is_empty())
            .collect()
    }

    /// `(url path, content type, body)` of every file served from the root.
    pub fn files(&self) -> Vec<(String, &'static str, String)> {
        let mut out = Vec::new();
        let google = self.google_file.trim().trim_start_matches('/');
        if !google.is_empty() && !google.contains(['/', '\\']) {
            out.push((format!("/{google}"), "text/html; charset=utf-8", format!("google-site-verification: {google}")));
        }
        let bing = self.bing_file.trim();
        if !bing.is_empty() {
            out.push((
                "/BingSiteAuth.xml".to_owned(),
                "application/xml; charset=utf-8",
                format!("<?xml version=\"1.0\"?>\n<users>\n\t<user>{bing}</user>\n</users>"),
            ));
        }
        for (path, lines) in [("/ads.txt", &self.ads_txt), ("/app-ads.txt", &self.app_ads_txt)] {
            let lines: Vec<&str> = lines.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            if !lines.is_empty() {
                out.push((path.to_owned(), "text/plain; charset=utf-8", format!("{}\n", lines.join("\n"))));
            }
        }
        // `public/` never serves dot-directories, so these live here.
        for (paths, json) in [
            (&["/.well-known/assetlinks.json"][..], &self.assetlinks),
            (
                &["/.well-known/apple-app-site-association", "/apple-app-site-association"][..],
                &self.apple_app_site_association,
            ),
        ] {
            if !json.trim().is_empty() {
                for path in paths {
                    out.push(((*path).to_owned(), "application/json", json.trim().to_owned()));
                }
            }
        }
        out
    }

    /// Problems with the values (part of `Config::validate`).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let bad = |key: &str, value: &str, help: &str| {
            Diagnostic::error("NR0010", format!("Invalid `[verification] {key}`"))
                .message(format!("found `{key} = {value:?}`"))
                .help(help.to_owned())
        };
        for (name, value) in self.metas() {
            let key = match name {
                "google-site-verification" => "google",
                "msvalidate.01" => "bing",
                "yandex-verification" => "yandex",
                "p:domain_verify" => "pinterest",
                "facebook-domain-verification" => "facebook",
                "baidu-site-verification" => "baidu",
                "naver-site-verification" => "naver",
                "seznam-wmt" => "seznam",
                other => other,
            };
            if value.contains(['<', '>']) || value.contains("content=") {
                out.push(bad(
                    key,
                    value,
                    "paste only the value of the tag's `content` attribute, not the whole <meta> tag",
                ));
            } else if value.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"') {
                out.push(bad(key, value, "a verification code has no spaces or quotes; copy the `content` value only"));
            }
            if name.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\'' | '=')) {
                out.push(bad(
                    "other",
                    name,
                    "the key is the meta tag's `name`, e.g. `\"norton-safeweb-site-verification\"`",
                ));
            }
        }
        let google = self.google_file.trim().trim_start_matches('/');
        if !google.is_empty()
            && !(google.starts_with("google")
                && google.ends_with(".html")
                && google.len() > "google.html".len()
                && google.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'))
        {
            out.push(bad(
                "google_file",
                &self.google_file,
                "use the name of the file Search Console offers for download, e.g. `google1234abcd5678ef90.html`",
            ));
        }
        let bing = self.bing_file.trim();
        if !bing.is_empty() && !bing.chars().all(|c| c.is_ascii_alphanumeric()) {
            out.push(bad(
                "bing_file",
                bing,
                "use the code between <user> and </user> in the BingSiteAuth.xml Bing offers, not the file or its name",
            ));
        }
        for (key, json) in
            [("assetlinks", &self.assetlinks), ("apple_app_site_association", &self.apple_app_site_association)]
        {
            if !json.trim().is_empty()
                && let Err(e) = serde_json::from_str::<serde_json::Value>(json)
            {
                out.push(bad(key, &format!("… ({e})"), "the value is the JSON document itself, in a TOML multi-line string: `assetlinks = \'\'\'[ … ]\'\'\'`"));
            }
        }
        for (key, lines) in [("ads_txt", &self.ads_txt), ("app_ads_txt", &self.app_ads_txt)] {
            for line in lines {
                let line = line.trim();
                let record = line.split('#').next().unwrap_or("").trim();
                // `domain, publisher id, DIRECT|RESELLER[, authority id]` or `VARIABLE=value`.
                let ok = record.is_empty()
                    || record.contains('=')
                    || (3..=4).contains(&record.split(',').count())
                        && record
                            .split(',')
                            .nth(2)
                            .is_some_and(|t| matches!(t.trim().to_ascii_uppercase().as_str(), "DIRECT" | "RESELLER"));
                if !ok || line.contains(['\n', '\r']) {
                    out.push(bad(
                        key,
                        line,
                        "one record per entry: `\"google.com, pub-0000000000000000, DIRECT, f08c47fec0942fa0\"` or `\"CONTACT=ads@example.com\"`",
                    ));
                }
            }
        }
        out
    }
}

/// `[analytics] consent`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsentMode {
    /// Tags run as soon as the page loads.
    #[default]
    None,
    /// Google tags start with Consent Mode v2 set to `denied`; every other
    /// tag that sets cookies waits for `nextRust.consent.grant([...])`.
    Required,
}

/// `[analytics] plausible`: a domain, or a table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Plausible {
    /// `plausible = "example.com"`
    Domain(String),
    /// `plausible = { domain = "example.com", script = "https://stats.example.com/js/script.js" }`
    Table(PlausibleTable),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlausibleTable {
    /// The site as it is named in Plausible (`data-domain`).
    pub domain: String,
    /// The script to load: a self-hosted or proxied copy, a script with
    /// extensions, or the per-site `…/js/pa-XXXX.js`.
    pub script: String,
}

impl Plausible {
    pub const DEFAULT_SCRIPT: &'static str = "https://plausible.io/js/script.js";

    pub fn domain(&self) -> &str {
        match self {
            Plausible::Domain(d) => d.trim(),
            Plausible::Table(t) => t.domain.trim(),
        }
    }

    pub fn script(&self) -> &str {
        match self {
            Plausible::Table(t) if !t.script.trim().is_empty() => t.script.trim(),
            _ => Self::DEFAULT_SCRIPT,
        }
    }

    /// Nothing to load.
    pub fn is_empty(&self) -> bool {
        self.domain().is_empty() && self.script() == Self::DEFAULT_SCRIPT
    }
}

/// `[analytics] umami`: a website id (Umami Cloud), or a table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Umami {
    /// `umami = "94db1cb1-74f4-4a40-ad6c-962362670409"`
    WebsiteId(String),
    /// `umami = { website_id = "…", script = "https://stats.example.com/script.js" }`
    Table(UmamiTable),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UmamiTable {
    pub website_id: String,
    /// The tracker of a self-hosted Umami.
    pub script: String,
}

impl Umami {
    pub const DEFAULT_SCRIPT: &'static str = "https://cloud.umami.is/script.js";

    pub fn website_id(&self) -> &str {
        match self {
            Umami::WebsiteId(id) => id.trim(),
            Umami::Table(t) => t.website_id.trim(),
        }
    }

    pub fn script(&self) -> &str {
        match self {
            Umami::Table(t) if !t.script.trim().is_empty() => t.script.trim(),
            _ => Self::DEFAULT_SCRIPT,
        }
    }
}

/// `[analytics]`: measurement and advertising tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnalyticsConfig {
    /// Google Analytics 4 measurement id (`G-XXXXXXXXXX`).
    pub google_analytics: String,
    /// Google Tag Manager container (`GTM-XXXXXXX`).
    pub google_tag_manager: String,
    /// Google Ads conversion id (`AW-123456789`).
    pub google_ads: String,
    /// Microsoft Clarity project id.
    pub microsoft_clarity: String,
    /// Meta (Facebook) Pixel id.
    pub meta_pixel: String,
    /// Pinterest Tag id.
    pub pinterest_tag: String,
    /// LinkedIn Insight Tag partner id.
    pub linkedin_insight: String,
    /// TikTok Pixel id.
    pub tiktok_pixel: String,
    /// Hotjar site id.
    pub hotjar: String,
    /// Cloudflare Web Analytics beacon token.
    pub cloudflare_web_analytics: String,
    /// Plausible Analytics.
    pub plausible: Option<Plausible>,
    /// Umami.
    pub umami: Option<Umami>,

    /// Also load the tags under `next-rust dev`. Verification tags are
    /// always written; analytics are production-only unless this is set.
    pub enabled_in_dev: bool,
    /// Load nothing for visitors who send Do Not Track or Global Privacy Control.
    pub respect_dnt: bool,
    /// Whether tags wait for the visitor's consent.
    pub consent: ConsentMode,
    /// Adds `anonymize_ip` to the Google tag's config. Google Analytics 4
    /// never stores IP addresses, so this only matters to other products
    /// reading the same config.
    pub anonymize_ip: bool,
    /// Pages that get no analytics: `"/account"`, `"/student/*"` (the path
    /// and everything below it), `"/blog/*/edit"` (`*` inside a segment
    /// matches any text).
    pub exclude_paths: Vec<String>,
    /// Leave analytics off pages whose `robots` metadata says `noindex`.
    pub skip_noindex: bool,
    /// Report a page view on client-side navigations to the tags that do
    /// not follow the History API by themselves.
    pub page_view_on_navigation: bool,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            google_analytics: String::new(),
            google_tag_manager: String::new(),
            google_ads: String::new(),
            microsoft_clarity: String::new(),
            meta_pixel: String::new(),
            pinterest_tag: String::new(),
            linkedin_insight: String::new(),
            tiktok_pixel: String::new(),
            hotjar: String::new(),
            cloudflare_web_analytics: String::new(),
            plausible: None,
            umami: None,
            enabled_in_dev: false,
            respect_dnt: true,
            consent: ConsentMode::None,
            anonymize_ip: false,
            exclude_paths: Vec::new(),
            skip_noindex: false,
            page_view_on_navigation: true,
        }
    }
}

impl AnalyticsConfig {
    /// The Plausible settings, when there is something to load.
    pub fn plausible(&self) -> Option<&Plausible> {
        self.plausible.as_ref().filter(|p| !p.is_empty())
    }

    /// The Umami settings, when a website id is set.
    pub fn umami(&self) -> Option<&Umami> {
        self.umami.as_ref().filter(|u| !u.website_id().is_empty())
    }

    /// At least one tag is configured.
    pub fn any(&self) -> bool {
        [
            &self.google_analytics,
            &self.google_tag_manager,
            &self.google_ads,
            &self.microsoft_clarity,
            &self.meta_pixel,
            &self.pinterest_tag,
            &self.linkedin_insight,
            &self.tiktok_pixel,
            &self.hotjar,
            &self.cloudflare_web_analytics,
        ]
        .iter()
        .any(|v| !v.trim().is_empty())
            || self.plausible().is_some()
            || self.umami().is_some()
    }

    /// `path` (without the base path) is under `exclude_paths`.
    pub fn excludes(&self, path: &str) -> bool {
        self.exclude_paths.iter().any(|pattern| path_matches(pattern, path))
    }

    /// Problems with the values (part of `Config::validate`).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let mut bad = |key: &str, value: &str, help: &str| {
            out.push(
                Diagnostic::error("NR0011", format!("Invalid `[analytics] {key}`"))
                    .message(format!("found `{key} = {value:?}`"))
                    .help(help.to_owned()),
            );
        };
        let upper_alnum = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());

        let ga = self.google_analytics.trim();
        if ga.starts_with("UA-") {
            bad(
                "google_analytics",
                ga,
                "Universal Analytics (`UA-…`) stopped processing data in 2023. Create a Google Analytics 4 property and use its measurement id, `G-XXXXXXXXXX` (Admin → Data streams → your web stream)",
            );
        } else if ga.starts_with("GTM-") {
            bad("google_analytics", ga, "that is a Tag Manager container; put it in `google_tag_manager`");
        } else if ga.starts_with("AW-") {
            bad("google_analytics", ga, "that is a Google Ads id; put it in `google_ads`");
        } else if !ga.is_empty() && !ga.strip_prefix("G-").is_some_and(|id| id.len() >= 4 && upper_alnum(id)) {
            bad("google_analytics", ga, "a Google Analytics 4 measurement id looks like `G-XXXXXXXXXX`");
        }

        let gtm = self.google_tag_manager.trim();
        if gtm.starts_with("G-") {
            bad("google_tag_manager", gtm, "that is a Google Analytics id; put it in `google_analytics`");
        } else if !gtm.is_empty() && !gtm.strip_prefix("GTM-").is_some_and(upper_alnum) {
            bad("google_tag_manager", gtm, "a Tag Manager container id looks like `GTM-XXXXXXX`");
        }

        let ads = self.google_ads.trim();
        if !ads.is_empty() && !ads.strip_prefix("AW-").is_some_and(digits) {
            bad("google_ads", ads, "a Google Ads conversion id looks like `AW-123456789`");
        }

        let clarity = self.microsoft_clarity.trim();
        if !clarity.is_empty() && !clarity.chars().all(|c| c.is_ascii_alphanumeric()) {
            bad(
                "microsoft_clarity",
                clarity,
                "the Clarity project id is the short letters-and-digits code in Settings → Overview, e.g. `abcd1234ef`",
            );
        }

        for (key, value, what) in [
            ("meta_pixel", &self.meta_pixel, "the Meta Pixel (dataset) id is digits only, e.g. `1234567890123456`"),
            ("pinterest_tag", &self.pinterest_tag, "the Pinterest Tag id is digits only, e.g. `2612345678901`"),
            ("linkedin_insight", &self.linkedin_insight, "the LinkedIn partner id is digits only, e.g. `1234567`"),
            ("hotjar", &self.hotjar, "the Hotjar site id is digits only, e.g. `1234567`"),
        ] {
            let value = value.trim();
            if !value.is_empty() && !digits(value) {
                bad(key, value, what);
            }
        }

        let tiktok = self.tiktok_pixel.trim();
        if !tiktok.is_empty() && !(tiktok.len() >= 8 && upper_alnum(tiktok)) {
            bad(
                "tiktok_pixel",
                tiktok,
                "a TikTok Pixel id is upper-case letters and digits, e.g. `CABCDEFGHIJKLMNOPQRS`",
            );
        }

        let cf = self.cloudflare_web_analytics.trim();
        // Cloudflare does not document the token's shape; only obvious
        // mistakes (a pasted snippet, a URL) are refused.
        if !cf.is_empty() && !((16..=64).contains(&cf.len()) && cf.chars().all(|c| c.is_ascii_alphanumeric())) {
            bad(
                "cloudflare_web_analytics",
                cf,
                "use the 32-character `token` from the beacon snippet (Web Analytics → Manage site), not the whole snippet",
            );
        }

        if let Some(p) = self.plausible() {
            let domain = p.domain();
            let per_site = script_file(p.script()).starts_with("pa-");
            if domain.is_empty() && !per_site {
                bad(
                    "plausible",
                    domain,
                    "set the site's domain as it is named in Plausible: `plausible = \"example.com\"`",
                );
            } else if domain.contains("://") || domain.contains(char::is_whitespace) {
                bad(
                    "plausible",
                    domain,
                    "use the bare domain as it is named in Plausible, e.g. `example.com` (no `https://`)",
                );
            }
            if !is_https_url(p.script()) {
                bad("plausible.script", p.script(), "the script must be a full `https://…` URL");
            }
        }
        if let Some(u) = self.umami() {
            let id = u.website_id();
            let uuid = id.len() == 36
                && id
                    .chars()
                    .enumerate()
                    .all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() });
            if !uuid {
                bad("umami", id, "the Umami website id is a UUID, e.g. `94db1cb1-74f4-4a40-ad6c-962362670409`");
            }
            if !is_https_url(u.script()) {
                bad("umami.script", u.script(), "the script must be a full `https://…` URL");
            }
        }

        for pattern in &self.exclude_paths {
            if !pattern.starts_with('/') {
                bad("exclude_paths", pattern, "every pattern starts with `/`, e.g. `\"/account/*\"`");
            }
        }
        out
    }
}

/// `https://host/…` with a plain host: what may be written into a page and a
/// `Content-Security-Policy`.
fn is_https_url(url: &str) -> bool {
    url.strip_prefix("https://").is_some_and(|rest| {
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        !host.is_empty()
            && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
            && !url
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | ';' | ','))
    })
}

/// The file name of a script URL.
pub fn script_file(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or("").rsplit('/').next().unwrap_or("")
}

/// `scheme://host[:port]` of a URL.
pub fn origin_of(url: &str) -> &str {
    let after = url.find("://").map_or(0, |i| i + 3);
    let end = url[after..].find(['/', '?', '#']).map_or(url.len(), |i| after + i);
    &url[..end]
}

/// Path patterns of `exclude_paths`: a trailing `/*` (or `/**`) matches the
/// path itself and everything below it; elsewhere `*` matches any text inside
/// one segment. Trailing slashes are ignored.
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let trim = |s: &str| if s.len() > 1 { s.trim_end_matches('/').to_owned() } else { s.to_owned() };
    let path = trim(path);
    let pattern = trim(pattern);
    if let Some(prefix) = pattern.strip_suffix("/**").or_else(|| pattern.strip_suffix("/*")) {
        if prefix.is_empty() {
            return true;
        }
        let pat: Vec<&str> = prefix.split('/').collect();
        let segs: Vec<&str> = path.split('/').collect();
        return segs.len() >= pat.len() && pat.iter().zip(&segs).all(|(p, s)| segment_matches(p, s));
    }
    let pat: Vec<&str> = pattern.split('/').collect();
    let segs: Vec<&str> = path.split('/').collect();
    pat.len() == segs.len() && pat.iter().zip(&segs).all(|(p, s)| segment_matches(p, s))
}

fn segment_matches(pattern: &str, segment: &str) -> bool {
    let Some((first, rest)) = pattern.split_once('*') else { return pattern == segment };
    let Some(mut left) = segment.strip_prefix(first) else { return false };
    let mut parts: Vec<&str> = rest.split('*').collect();
    let last = parts.pop().unwrap_or("");
    for part in parts {
        match left.find(part) {
            Some(at) => left = &left[at + part.len()..],
            None => return false,
        }
    }
    left.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    fn codes(config: &Config) -> Vec<String> {
        let mut v = config.verification.diagnostics();
        v.extend(config.analytics.diagnostics());
        v.into_iter().map(|d| d.code).collect()
    }

    fn parse(toml: &str) -> Config {
        toml::from_str(toml).expect("valid config")
    }

    #[test]
    fn empty_sections_are_valid_and_write_nothing() {
        let config = parse("[verification]\n[analytics]\n");
        assert!(config.verification.metas().is_empty());
        assert!(config.verification.files().is_empty());
        assert!(!config.analytics.any());
        assert!(codes(&config).is_empty());
        // Empty strings are the same as absent keys: a template can list them all.
        let config = parse("[verification]\ngoogle = \"\"\n[analytics]\ngoogle_analytics = \"\"\nplausible = \"\"\n");
        assert!(config.verification.metas().is_empty());
        assert!(!config.analytics.any());
        assert!(codes(&config).is_empty());
    }

    #[test]
    fn defaults() {
        let a = AnalyticsConfig::default();
        assert!(!a.enabled_in_dev && a.respect_dnt && a.page_view_on_navigation && !a.skip_noindex);
        assert_eq!(a.consent, ConsentMode::None);
    }

    #[test]
    fn verification_metas_and_files() {
        let config = parse(
            r##"
[verification]
google = "g-token_1"
bing = "0123456789ABCDEF"
yandex = "y1"
pinterest = "p1"
facebook = "f1"
baidu = "codeva-b1"
naver = "n1"
seznam = "s1"
google_file = "google1234abcd.html"
bing_file = "0123ABCD"
ads_txt = ["google.com, pub-1, DIRECT, f08c47fec0942fa0", "# note", "CONTACT=ads@example.com"]
other = { "norton-safeweb-site-verification" = "nn" }
"##,
        );
        assert!(codes(&config).is_empty(), "{:?}", config.verification.diagnostics());
        let metas = config.verification.metas();
        assert_eq!(
            metas,
            [
                ("google-site-verification", "g-token_1"),
                ("msvalidate.01", "0123456789ABCDEF"),
                ("yandex-verification", "y1"),
                ("p:domain_verify", "p1"),
                ("facebook-domain-verification", "f1"),
                ("baidu-site-verification", "codeva-b1"),
                ("naver-site-verification", "n1"),
                ("seznam-wmt", "s1"),
                ("norton-safeweb-site-verification", "nn"),
            ]
        );
        let files = config.verification.files();
        assert_eq!(files[0].0, "/google1234abcd.html");
        assert_eq!(files[0].2, "google-site-verification: google1234abcd.html");
        assert_eq!(files[1].0, "/BingSiteAuth.xml");
        assert!(files[1].2.contains("<user>0123ABCD</user>"));
        assert_eq!(files[2].0, "/ads.txt");
        assert!(files[2].2.starts_with("google.com, pub-1, DIRECT, f08c47fec0942fa0\n# note\n"));
        assert_eq!(files.len(), 3);
        let links = parse(
            "[verification]\nassetlinks = \'\'\'[{\"relation\": []}]\'\'\'\napple_app_site_association = \'{\"applinks\": {}}\'\n",
        );
        assert!(codes(&links).is_empty());
        let files = links.verification.files();
        assert_eq!(
            files.iter().map(|f| (f.0.as_str(), f.1)).collect::<Vec<_>>(),
            [
                ("/.well-known/assetlinks.json", "application/json"),
                ("/.well-known/apple-app-site-association", "application/json"),
                ("/apple-app-site-association", "application/json"),
            ]
        );
        assert_eq!(codes(&parse("[verification]\nassetlinks = \"not json\"\n")), ["NR0010"]);
        // The config travels inside a release binary as JSON.
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&json).unwrap().verification, config.verification);
    }

    #[test]
    fn a_pasted_meta_tag_is_rejected() {
        let config = parse("[verification]\ngoogle = '<meta name=\"google-site-verification\" content=\"x\" />'\n");
        let diags = config.verification.diagnostics();
        assert_eq!(diags[0].code, "NR0010");
        assert!(diags[0].help.as_deref().unwrap().contains("content"));
        assert_eq!(codes(&parse("[verification]\ngoogle_file = \"evil/../x.html\"\n")), ["NR0010"]);
        assert_eq!(codes(&parse("[verification]\nbing_file = \"BingSiteAuth.xml\"\n")), ["NR0010"]);
        assert_eq!(codes(&parse("[verification]\nads_txt = [\"nonsense\"]\n")), ["NR0010"]);
    }

    #[test]
    fn analytics_ids_are_checked() {
        let ok = parse(
            r#"
[analytics]
google_analytics = "G-ABC123DEF4"
google_tag_manager = "GTM-ABC1234"
google_ads = "AW-123456789"
microsoft_clarity = "abcd1234ef"
meta_pixel = "1234567890123456"
pinterest_tag = "2612345678901"
linkedin_insight = "1234567"
tiktok_pixel = "CABCDEFGHIJKLMNOPQRS"
hotjar = "1234567"
cloudflare_web_analytics = "0123456789abcdef0123456789abcdef"
plausible = "example.com"
umami = { website_id = "94db1cb1-74f4-4a40-ad6c-962362670409", script = "https://stats.example.com/script.js" }
exclude_paths = ["/student/*"]
"#,
        );
        assert!(codes(&ok).is_empty(), "{:?}", ok.analytics.diagnostics());
        assert!(ok.analytics.any());
        // The config travels inside a release binary as JSON.
        let json = serde_json::to_string(&ok).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&json).unwrap().analytics, ok.analytics);

        let ua = parse("[analytics]\ngoogle_analytics = \"UA-12345-1\"\n");
        let diags = ua.analytics.diagnostics();
        assert_eq!(diags[0].code, "NR0011");
        assert!(diags[0].help.as_deref().unwrap().contains("Universal Analytics"));
        assert!(diags[0].help.as_deref().unwrap().contains("G-"));

        for bad in [
            "google_analytics = \"GTM-ABC\"",
            "google_analytics = \"g-abc\"",
            "google_tag_manager = \"G-ABC123\"",
            "google_ads = \"123\"",
            "microsoft_clarity = \"<script>\"",
            "meta_pixel = \"abc\"",
            "tiktok_pixel = \"abc\"",
            "cloudflare_web_analytics = \"https://example.com/beacon.js\"",
            "plausible = \"https://example.com\"",
            "plausible = { domain = \"example.com\", script = \"http://x/js/script.js\" }",
            "umami = \"not-a-uuid\"",
            "exclude_paths = [\"student\"]",
        ] {
            assert_eq!(codes(&parse(&format!("[analytics]\n{bad}\n"))), ["NR0011"], "{bad}");
        }
    }

    #[test]
    fn unknown_keys_fail() {
        assert!(toml::from_str::<Config>("[analytics]\ngoogle = \"G-1\"\n").is_err());
        assert!(toml::from_str::<Config>("[verification]\ngogle = \"x\"\n").is_err());
        assert!(toml::from_str::<Config>("[analytics]\nconsent = \"maybe\"\n").is_err());
        assert_eq!(parse("[analytics]\nconsent = \"required\"\n").analytics.consent, ConsentMode::Required);
    }

    #[test]
    fn plausible_forms() {
        let p = parse("[analytics]\nplausible = \"example.com\"\n").analytics;
        assert_eq!(p.plausible().unwrap().script(), Plausible::DEFAULT_SCRIPT);
        let p = parse("[analytics.plausible]\nscript = \"https://plausible.io/js/pa-AbC123.js\"\n");
        assert!(p.analytics.any(), "a per-site script needs no domain");
        assert!(codes(&p).is_empty());
    }

    #[test]
    fn exclude_patterns() {
        assert!(path_matches("/student/*", "/student"));
        assert!(path_matches("/student/*", "/student/"));
        assert!(path_matches("/student/*", "/student/courses/3"));
        assert!(!path_matches("/student/*", "/students"));
        assert!(!path_matches("/student/*", "/"));
        assert!(path_matches("/login", "/login/"));
        assert!(!path_matches("/login", "/login/reset"));
        assert!(path_matches("/blog/*/edit", "/blog/hello/edit"));
        assert!(!path_matches("/blog/*/edit", "/blog/a/b/edit"));
        assert!(path_matches("/preview-*", "/preview-42"));
        assert!(path_matches("/*", "/anything/at/all"));
        assert_eq!(origin_of("https://stats.example.com/js/script.js"), "https://stats.example.com");
        assert_eq!(script_file("https://plausible.io/js/pa-Ab.js?x=1"), "pa-Ab.js");
    }
}
