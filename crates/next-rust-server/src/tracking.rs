//! Site verification and analytics: what `[verification]` and `[analytics]`
//! in `next-rust.toml` turn into.
//!
//! Compiled once per process ([`Tracking::new`]):
//!
//! * **Verification** `<meta>` tags go into every document's `<head>`, in
//!   every environment, and the file-based proofs (`/google….html`,
//!   `/BingSiteAuth.xml`, `/ads.txt`, `/app-ads.txt`) are answered from the
//!   root.
//! * **Analytics** are one inline script in `<head>` (with the request's
//!   nonce) that loads every configured tag, preceded by `preconnect` hints,
//!   plus `<noscript>` fallbacks right after `<body>`. They are written into
//!   full documents only: a client-side navigation keeps the scripts of the
//!   document it started in, so its responses carry none.
//! * The **`strict` CSP preset** is widened by exactly the origins the
//!   configured tags need.
//!
//! ## Pages without analytics
//!
//! `exclude_paths`, `Metadata::analytics(false)` and `skip_noindex` leave a
//! document without the tags. Tags that are loaded follow the History API on
//! their own, so moving between a page with analytics and one without must be
//! a full page load: the server answers such a client navigation with
//! [`full_load`], which makes the client runtime load the URL normally.
//! A page that opted out through its metadata cannot be recognized from a
//! URL, so its document carries a small script ([`OPT_OUT_JS`]) instead, which reloads
//! after the visitor navigates away from it.

use next_rust_core::Environment;
use next_rust_core::config::Config;
use next_rust_core::tracking::{AnalyticsConfig, ConsentMode, origin_of, script_file};
use next_rust_view::{Metadata, escape_attr};

use crate::request::Request;
use crate::response::Response;

/// Placed in documents that opted out of analytics through their metadata:
/// the next page may have analytics, which only a full load can start.
const OPT_OUT_JS: &str = "addEventListener(\"nr:navigate\",function(){location.reload()})";

/// What a document gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// No analytics configured for this environment, or the path is excluded.
    Nothing,
    /// The page opted out (`Metadata::analytics(false)`, `skip_noindex`).
    OptOut,
    Tags,
}

/// Origins a set of tags needs, per CSP directive.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Sources {
    script: Vec<String>,
    connect: Vec<String>,
    img: Vec<String>,
    frame: Vec<String>,
    font: Vec<String>,
}

impl Sources {
    fn add(&mut self, script: &[&str], connect: &[&str], img: &[&str], frame: &[&str]) {
        for (list, new) in
            [(&mut self.script, script), (&mut self.connect, connect), (&mut self.img, img), (&mut self.frame, frame)]
        {
            for origin in new {
                if !list.iter().any(|o| o == origin) {
                    list.push((*origin).to_owned());
                }
            }
        }
    }
}

/// The analytics of an app, ready to write.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tags {
    /// Head markup up to the nonce, and after it.
    head: (String, String),
    /// The text of the inline script.
    script: String,
    /// `<noscript>` fallbacks, right after `<body>`.
    body_start: String,
    sources: Sources,
}

#[derive(Debug, Default)]
pub(crate) struct Tracking {
    /// `<meta>` verification tags; part of every document.
    verification: String,
    /// `(path, content type, body)`
    files: Vec<(String, &'static str, String)>,
    tags: Option<Tags>,
    analytics: AnalyticsConfig,
    base_path: String,
    /// `Content-Security-Policy` of the app with `{nonce}` still in it.
    csp: Option<String>,
}

impl Tracking {
    pub(crate) fn new(config: &Config, env: Environment) -> Self {
        let verification = config
            .verification
            .metas()
            .into_iter()
            .map(|(name, content)| {
                format!("<meta name=\"{}\" content=\"{}\">", escape_attr(name), escape_attr(content))
            })
            .collect();
        let analytics = config.analytics.clone();
        // The build stops on these (`NR0011`); a config swapped in at run
        // time (`NEXT_RUST_CONFIG`) is held to the same rules.
        let problems = analytics.diagnostics();
        for problem in &problems {
            crate::log::error(&format!("[analytics] is ignored: {}", problem.render(false)));
        }
        let active = problems.is_empty() && analytics.any() && (!env.is_dev() || analytics.enabled_in_dev);
        let tags = active.then(|| compile(&analytics));
        let csp = config.security.csp.as_deref().map(|csp| {
            if csp.trim() == "strict" { strict_csp(tags.as_ref().map(|t| &t.sources)) } else { csp.to_owned() }
        });
        Tracking {
            verification,
            files: config.verification.files(),
            tags,
            analytics,
            base_path: config.app.base_path.clone(),
            csp,
        }
    }

    /// The app's `Content-Security-Policy` (`{nonce}` not yet filled in):
    /// the `strict` preset allows the origins of the configured tags.
    pub(crate) fn csp(&self) -> Option<&str> {
        self.csp.as_deref()
    }

    /// Verification `<meta>` tags.
    pub(crate) fn verification_head(&self) -> &str {
        &self.verification
    }

    /// A file-based proof at `path`.
    pub(crate) fn file(&self, path: &str) -> Option<Response> {
        let (_, content_type, body) = self.files.iter().find(|(p, ..)| p == path)?;
        Some(
            Response::text(body.clone())
                .with_header("content-type", content_type)
                .with_cache_control("public, max-age=3600"),
        )
    }

    /// Whether analytics run at all in this process.
    #[cfg(test)]
    pub(crate) fn active(&self) -> bool {
        self.tags.is_some()
    }

    fn excluded(&self, path: &str) -> bool {
        self.analytics.excludes(path)
    }

    /// What the document at `path` gets.
    pub(crate) fn decide(&self, path: &str, metadata: Option<&Metadata>) -> Decision {
        if self.tags.is_none() || self.excluded(path) {
            return Decision::Nothing;
        }
        let opted_out = metadata.is_some_and(|m| {
            m.analytics == Some(false) || (self.analytics.skip_noindex && m.is_noindex() && m.analytics != Some(true))
        });
        if opted_out { Decision::OptOut } else { Decision::Tags }
    }

    /// Head markup for `decision`.
    pub(crate) fn head(&self, decision: Decision, nonce: &str) -> String {
        match (decision, &self.tags) {
            (Decision::Tags, Some(tags)) => format!("{}{}{}", tags.head.0, escape_attr(nonce), tags.head.1),
            (Decision::OptOut, Some(_)) => opt_out_script(nonce),
            _ => String::new(),
        }
    }

    /// Markup right after `<body>` for `decision`.
    pub(crate) fn body_start(&self, decision: Decision) -> &str {
        match (decision, &self.tags) {
            (Decision::Tags, Some(tags)) => &tags.body_start,
            _ => "",
        }
    }

    /// A client navigation from a page with analytics to one without (or
    /// back), as far as the two paths tell: answered with [`full_load`].
    pub(crate) fn crosses_boundary(&self, req: &Request) -> bool {
        if self.tags.is_none() || self.analytics.exclude_paths.is_empty() || !is_navigation(req) {
            return false;
        }
        let Some(from) = req.header("x-nr-from") else { return false };
        let from = from.strip_prefix(self.base_path.as_str()).filter(|_| !self.base_path.is_empty()).unwrap_or(from);
        let from = if from.is_empty() { "/" } else { from };
        self.excluded(from) != self.excluded(req.path())
    }

    /// Prepare cached HTML for a client navigation: `Err` when the page
    /// opted out and must be loaded in full, otherwise the HTML without the
    /// analytics, and whether anything was removed.
    ///
    /// The tags are found by what no HTML writer changes (the text of the
    /// script, the URLs of the `<noscript>` fallbacks), not by their markup.
    pub(crate) fn for_navigation(&self, mut html: String) -> Result<(String, bool), ()> {
        let Some(tags) = &self.tags else { return Ok((html, false)) };
        if html.contains(OPT_OUT_JS) {
            return Err(());
        }
        if !remove_enclosing(&mut html, &tags.script, "<script", "</script>", usize::MAX) {
            return Ok((html, false));
        }
        for url in NOSCRIPT_URLS {
            // A fallback is a few hundred bytes; anything larger is the app's own markup.
            remove_enclosing(&mut html, url, "<noscript", "</noscript>", 512);
        }
        Ok((html, true))
    }
}

/// What the `<noscript>` fallbacks load.
const NOSCRIPT_URLS: [&str; 4] = [
    "https://www.googletagmanager.com/ns.html?id=",
    "https://www.facebook.com/tr?id=",
    "https://ct.pinterest.com/v3/?tid=",
    "https://px.ads.linkedin.com/collect/?pid=",
];

/// Remove the first `open…close` element around `needle`, if it is at most
/// `max` bytes long.
fn remove_enclosing(html: &mut String, needle: &str, open: &str, close: &str, max: usize) -> bool {
    let Some(at) = html.find(needle) else { return false };
    let Some(start) = html[..at].rfind(open) else { return false };
    let Some(end) = html[at..].find(close).map(|i| at + i + close.len()) else { return false };
    if end - start > max.max(needle.len()) || html[start..at].contains(close) {
        return false;
    }
    html.replace_range(start..end, "");
    true
}

pub(crate) fn is_navigation(req: &Request) -> bool {
    req.header("x-nr-nav") == Some("1")
}

fn opt_out_script(nonce: &str) -> String {
    format!("<script nonce=\"{}\">{OPT_OUT_JS}</script>", escape_attr(nonce))
}

/// The answer to a client navigation that has to be a full page load. It is
/// not HTML, so the client runtime falls back to `location.href = url`.
pub(crate) fn full_load() -> Response {
    Response::status(204)
        .with_header("x-nr-reload", "1")
        .with_cache_control("private, no-cache, no-store, max-age=0, must-revalidate")
        .with_header("vary", "x-nr-nav, x-nr-from")
}

/// A JavaScript string literal that is safe inside `<script>`.
fn js(value: &str) -> String {
    serde_json::to_string(value.trim()).unwrap_or_else(|_| "\"\"".to_owned()).replace('<', "\\u003c")
}

const GTM: &str = "https://www.googletagmanager.com";

fn compile(a: &AnalyticsConfig) -> Tags {
    let required = a.consent == ConsentMode::Required;
    let set = |v: &str| !v.trim().is_empty();
    let (ga, gtm, ads) = (set(&a.google_analytics), set(&a.google_tag_manager), set(&a.google_ads));
    let google = ga || gtm || ads;

    let mut sources = Sources::default();
    // Origins worth connecting to before the scripts ask for them.
    let mut preconnect: Vec<String> = Vec::new();
    let mut hint = |origins: &[&str]| {
        for o in origins {
            if !preconnect.iter().any(|p| p == o) {
                preconnect.push((*o).to_owned());
            }
        }
    };
    let mut noscript = String::new();
    // `X(category, loader)`: category 0 loads right away.
    let mut loaders = String::new();
    // Page views on client navigations, for tags that do not follow the History API.
    let mut on_navigate = String::new();
    let waits = |category: &str| if required { format!("\"{category}\"") } else { "0".to_owned() };

    if ga || ads {
        let first = if ga { &a.google_analytics } else { &a.google_ads };
        let mut body = format!("L({});g(\"js\",new Date);", js(&format!("{GTM}/gtag/js?id={}", first.trim())));
        if ga {
            let options = if a.anonymize_ip { ",{anonymize_ip:!0}" } else { "" };
            body.push_str(&format!("g(\"config\",{}{options});", js(&a.google_analytics)));
            sources.add(
                &[GTM],
                &[GTM, "https://*.google-analytics.com", "https://*.google.com"],
                &[GTM, "https://*.google-analytics.com"],
                &[],
            );
            hint(&[GTM, "https://www.google-analytics.com"]);
        }
        if ads {
            body.push_str(&format!("g(\"config\",{});", js(&a.google_ads)));
            let pixels = [
                GTM,
                "https://www.googleadservices.com",
                "https://googleads.g.doubleclick.net",
                "https://pagead2.googlesyndication.com",
                "https://www.google.com",
            ];
            sources.add(&[GTM, "https://www.googleadservices.com", "https://www.google.com"], &pixels, &pixels, &[GTM]);
            sources.add(&[], &["https://ad.doubleclick.net"], &[], &[]);
            hint(&[GTM]);
        }
        loaders.push_str(&format!("X(0,function(){{{}}});", body.trim_end_matches(';')));
    }
    if gtm {
        let id = a.google_tag_manager.trim();
        loaders.push_str(&format!(
            "X(0,function(){{w.dataLayer.push({{\"gtm.start\":+new Date,event:\"gtm.js\"}});L({})}});",
            js(&format!("{GTM}/gtm.js?id={id}"))
        ));
        noscript.push_str(&format!(
            "<noscript><iframe src=\"{GTM}/ns.html?id={}\" height=\"0\" width=\"0\" style=\"display:none;visibility:hidden\"></iframe></noscript>",
            escape_attr(id)
        ));
        sources.add(&[GTM], &[GTM, "https://www.google.com"], &[GTM], &[GTM]);
        hint(&[GTM]);
        on_navigate
            .push_str("w.dataLayer.push({event:\"nr.page_view\",page_location:location.href,page_title:d.title});");
    }
    if set(&a.microsoft_clarity) {
        let consent = if required { "w.clarity(\"consentv2\",V());" } else { "" };
        loaders.push_str(&format!(
            "X({},function(){{w.clarity=w.clarity||function(){{(w.clarity.q=w.clarity.q||[]).push(arguments)}};{consent}L({})}});",
            waits("analytics"),
            js(&format!("https://www.clarity.ms/tag/{}", a.microsoft_clarity.trim()))
        ));
        sources.add(
            &["https://*.clarity.ms"],
            &["https://*.clarity.ms"],
            &["https://*.clarity.ms", "https://c.bing.com"],
            &[],
        );
        if !required {
            hint(&["https://www.clarity.ms"]);
        }
    }
    if set(&a.hotjar) {
        let id = a.hotjar.trim();
        loaders.push_str(&format!(
            "X({},function(){{w.hj=w.hj||function(){{(w.hj.q=w.hj.q||[]).push(arguments)}};w._hjSettings={{hjid:{id},hjsv:6}};L(\"https://static.hotjar.com/c/hotjar-{id}.js?sv=6\")}});",
            waits("analytics"),
        ));
        sources.add(
            &["https://*.hotjar.com"],
            &["https://*.hotjar.com", "https://*.hotjar.io", "wss://*.hotjar.com"],
            &["https://*.hotjar.com"],
            &["https://*.hotjar.com"],
        );
        sources.font.push("https://*.hotjar.com".to_owned());
        if !required {
            hint(&["https://static.hotjar.com"]);
        }
    }
    if set(&a.meta_pixel) {
        let id = a.meta_pixel.trim();
        loaders.push_str(&format!(
            "X({},function(){{if(w.fbq)return;var f=w.fbq=function(){{f.callMethod?f.callMethod.apply(f,arguments):f.queue.push(arguments)}};w._fbq||(w._fbq=f);f.push=f;f.loaded=!0;f.version=\"2.0\";f.queue=[];{push_state}L(\"https://connect.facebook.net/en_US/fbevents.js\");f(\"init\",{});f(\"track\",\"PageView\")}});",
            waits("ads"),
            js(id),
            // The pixel follows the History API by itself.
            push_state = if a.page_view_on_navigation { "" } else { "f.disablePushState=!0;" }
        ));
        if !required {
            noscript.push_str(&format!(
                "<noscript><img height=\"1\" width=\"1\" style=\"display:none\" alt=\"\" src=\"https://www.facebook.com/tr?id={}&amp;ev=PageView&amp;noscript=1\"></noscript>",
                escape_attr(id)
            ));
            hint(&["https://connect.facebook.net"]);
        }
        sources.add(
            &["https://connect.facebook.net"],
            &["https://www.facebook.com"],
            &["https://www.facebook.com"],
            &[],
        );
    }
    if set(&a.pinterest_tag) {
        let id = a.pinterest_tag.trim();
        loaders.push_str(&format!(
            "X({},function(){{if(!w.pintrk){{var p=w.pintrk=function(){{p.queue.push(Array.prototype.slice.call(arguments))}};p.queue=[];p.version=\"3.0\";L(\"https://s.pinimg.com/ct/core.js\")}}w.pintrk(\"load\",{});w.pintrk(\"page\")}});",
            waits("ads"),
            js(id)
        ));
        if !required {
            noscript.push_str(&format!(
                "<noscript><img height=\"1\" width=\"1\" style=\"display:none\" alt=\"\" src=\"https://ct.pinterest.com/v3/?tid={}&amp;noscript=1\"></noscript>",
                escape_attr(id)
            ));
            hint(&["https://s.pinimg.com"]);
        }
        sources.add(&["https://s.pinimg.com"], &["https://ct.pinterest.com"], &["https://ct.pinterest.com"], &[]);
        on_navigate.push_str("w.pintrk&&w.pintrk(\"page\");");
    }
    if set(&a.linkedin_insight) {
        let id = a.linkedin_insight.trim();
        loaders.push_str(&format!(
            "X({},function(){{w._linkedin_partner_id={id};(w._linkedin_data_partner_ids=w._linkedin_data_partner_ids||[]).push({id});if(!w.lintrk){{w.lintrk=function(a,b){{w.lintrk.q.push([a,b])}};w.lintrk.q=[]}}L(\"https://snap.licdn.com/li.lms-analytics/insight.min.js\")}});",
            waits("ads"),
            id = js(id)
        ));
        if !required {
            noscript.push_str(&format!(
                "<noscript><img height=\"1\" width=\"1\" style=\"display:none\" alt=\"\" src=\"https://px.ads.linkedin.com/collect/?pid={}&amp;fmt=img\"></noscript>",
                escape_attr(id)
            ));
            hint(&["https://snap.licdn.com"]);
        }
        sources.add(
            &["https://snap.licdn.com"],
            &["https://px.ads.linkedin.com", "https://cdn.linkedin.oribi.io"],
            &["https://px.ads.linkedin.com"],
            &[],
        );
    }
    if set(&a.tiktok_pixel) {
        loaders.push_str(&format!(
            "X({},function(){{var t=w.ttq=w.ttq||[],u=\"https://analytics.tiktok.com/i18n/pixel/events.js\",i={};w.TiktokAnalyticsObject=\"ttq\";t.methods=\"page track identify instances debug on off once ready alias group enableCookie disableCookie holdConsent revokeConsent grantConsent\".split(\" \");t.setAndDefer=function(o,m){{o[m]=function(){{o.push([m].concat(Array.prototype.slice.call(arguments,0)))}}}};t.methods.forEach(function(m){{t.setAndDefer(t,m)}});t.instance=function(k){{var e=t._i[k]||[];t.methods.forEach(function(m){{t.setAndDefer(e,m)}});return e}};t._i=t._i||{{}};t._i[i]=[];t._i[i]._u=u;t._t=t._t||{{}};t._t[i]=+new Date;t._o=t._o||{{}};t._o[i]={{}};L(u+\"?sdkid=\"+i+\"&lib=ttq\");t.page()}});",
            waits("ads"),
            js(&a.tiktok_pixel)
        ));
        sources.add(
            &["https://analytics.tiktok.com"],
            &["https://analytics.tiktok.com"],
            &["https://analytics.tiktok.com"],
            &[],
        );
        if !required {
            hint(&["https://analytics.tiktok.com"]);
        }
    }
    if let Some(p) = a.plausible() {
        let origin = origin_of(p.script()).to_owned();
        let stub = "w.plausible=w.plausible||function(){(w.plausible.q=w.plausible.q||[]).push(arguments)};";
        if script_file(p.script()).starts_with("pa-") {
            // The per-site script carries its settings and starts with `init()`.
            loaders.push_str(&format!(
                "X(0,function(){{{stub}w.plausible.init=w.plausible.init||function(i){{w.plausible.o=i||{{}}}};w.plausible.init();L({})}});",
                js(p.script())
            ));
        } else {
            loaders.push_str(&format!(
                "X(0,function(){{{stub}L({},{{\"data-domain\":{}}})}});",
                js(p.script()),
                js(p.domain())
            ));
        }
        sources.add(&[&origin], &[&origin], &[], &[]);
        hint(&[&origin]);
    }
    if let Some(u) = a.umami() {
        let origin = origin_of(u.script()).to_owned();
        let dnt = if a.respect_dnt { ",\"data-do-not-track\":\"true\"" } else { "" };
        loaders.push_str(&format!(
            "X(0,function(){{L({},{{\"data-website-id\":{}{dnt}}})}});",
            js(u.script()),
            js(u.website_id())
        ));
        // Umami Cloud collects on its API host.
        let collect: &[&str] = if origin == "https://cloud.umami.is" {
            &["https://cloud.umami.is", "https://api-gateway.umami.dev"]
        } else {
            &[]
        };
        sources.add(&[&origin], &[&origin], &[], &[]);
        sources.add(&[], collect, &[], &[]);
        hint(&[&origin]);
    }
    if set(&a.cloudflare_web_analytics) {
        let beacon = format!("{{\"token\":\"{}\"}}", a.cloudflare_web_analytics.trim());
        loaders.push_str(&format!(
            "X(0,function(){{L(\"https://static.cloudflareinsights.com/beacon.min.js\",{{\"data-cf-beacon\":{}}})}});",
            js(&beacon)
        ));
        sources.add(&["https://static.cloudflareinsights.com"], &["https://cloudflareinsights.com"], &[], &[]);
        hint(&["https://static.cloudflareinsights.com"]);
    }

    // The script: what may run (T), the loader (L), consent, then the tags.
    let mut s = String::with_capacity(1024 + loaders.len());
    s.push_str("!function(w,d){var n=w.nextRust=w.nextRust||{},c=d.currentScript,o=c&&c.nonce,N=navigator,");
    if a.respect_dnt {
        s.push_str("T=!(N.globalPrivacyControl||N.doNotTrack==\"1\"||w.doNotTrack==\"1\");");
    } else {
        s.push_str("T=!0;");
    }
    s.push_str(
        "function L(s,a){var e=d.createElement(\"script\"),k;e.async=!0;e.src=s;o&&(e.nonce=o);for(k in a)e.setAttribute(k,a[k]);d.head.appendChild(e)}",
    );
    if a.respect_dnt && !preconnect.is_empty() {
        // Visitors who opted out do not even connect.
        let list: Vec<String> = preconnect.iter().map(|o| js(o)).collect();
        s.push_str(&format!(
            "T&&[{}].forEach(function(h){{var e=d.createElement(\"link\");e.rel=\"preconnect\";e.href=h;d.head.appendChild(e)}});",
            list.join(",")
        ));
    }
    if google || required {
        s.push_str("w.dataLayer=w.dataLayer||[];function g(){w.dataLayer.push(arguments)}");
        if ga || ads {
            s.push_str("w.gtag=g;");
        }
    }
    if required {
        let mut update = String::new();
        if google {
            update.push_str("g(\"consent\",\"update\",M());");
        }
        if set(&a.microsoft_clarity) {
            // Clarity's consent API v2 (the keys are spelled this way).
            s.push_str("function V(){var m=M();return{ad_Storage:m.ad_storage,analytics_Storage:m.analytics_storage}}");
            update.push_str("w.clarity&&w.clarity(\"consentv2\",V());");
        }
        s.push_str(
            "var K=\"nr-consent\",G={analytics:0,ads:0},Q=[];try{(localStorage.getItem(K)||\"\").split(\",\").forEach(function(k){k in G&&(G[k]=1)})}catch(e){}",
        );
        s.push_str(
            "function M(){var a=G.ads?\"granted\":\"denied\";return{ad_storage:a,ad_user_data:a,ad_personalization:a,analytics_storage:G.analytics?\"granted\":\"denied\"}}",
        );
        s.push_str("function R(){T&&(Q=Q.filter(function(q){return!G[q[0]]||(q[1](),!1)}))}");
        s.push_str(&format!(
            "function S(a,v){{(typeof a==\"string\"?[a]:a||Object.keys(G)).forEach(function(k){{k in G&&(G[k]=v)}});try{{localStorage.setItem(K,Object.keys(G).filter(function(k){{return G[k]}}).join(\",\"))}}catch(e){{}}{update}R()}}"
        ));
        s.push_str(
            "n.consent={grant:function(a){S(a,1)},revoke:function(a){S(a,0)},state:function(){return{analytics:!!G.analytics,ads:!!G.ads}}};",
        );
        if google {
            s.push_str(
                "g(\"consent\",\"default\",{ad_storage:\"denied\",ad_user_data:\"denied\",ad_personalization:\"denied\",analytics_storage:\"denied\",wait_for_update:500});(G.analytics||G.ads)&&g(\"consent\",\"update\",M());",
            );
        }
        s.push_str("function X(c,f){c?Q.push([c,f]):T&&f()}");
    } else {
        s.push_str("function X(c,f){T&&f()}");
    }
    s.push_str(&loaders);
    if required {
        s.push_str("R();");
    }
    if a.page_view_on_navigation && !on_navigate.is_empty() {
        s.push_str(&format!(
            "T&&w.addEventListener(\"nr:navigate\",function(){{{}}});",
            on_navigate.trim_end_matches(';')
        ));
    }
    s.push_str("}(window,document)");

    let mut before = String::new();
    if !a.respect_dnt {
        for origin in &preconnect {
            before.push_str(&format!("<link rel=\"preconnect\" href=\"{}\">", escape_attr(origin)));
        }
    }
    before.push_str("<script nonce=\"");
    Tags { head: (before, format!("\">{s}</script>")), script: s, body_start: noscript, sources }
}

/// The `strict` preset, allowing the origins of the configured tags.
fn strict_csp(sources: Option<&Sources>) -> String {
    let base = crate::middleware::STRICT_CSP;
    let Some(sources) = sources else { return base.to_owned() };
    let mut frame_done = sources.frame.is_empty();
    let mut out: Vec<String> = base
        .split(';')
        .map(|directive| {
            let directive = directive.trim();
            let name = directive.split_ascii_whitespace().next().unwrap_or("");
            let extra = match name {
                "script-src" => &sources.script,
                "connect-src" => &sources.connect,
                "img-src" => &sources.img,
                "font-src" => &sources.font,
                "frame-src" => {
                    frame_done = true;
                    &sources.frame
                }
                _ => return directive.to_owned(),
            };
            if extra.is_empty() { directive.to_owned() } else { format!("{directive} {}", extra.join(" ")) }
        })
        .collect();
    if !frame_done {
        out.push(format!("frame-src 'self' {}", sources.frame.join(" ")));
    }
    out.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracking(toml: &str, env: Environment) -> Tracking {
        let config = Config::from_toml_str(toml).expect("valid config");
        assert!(config.analytics.diagnostics().is_empty(), "{:?}", config.analytics.diagnostics());
        Tracking::new(&config, env)
    }

    fn head(toml: &str) -> String {
        tracking(toml, Environment::Production).head(Decision::Tags, "N0")
    }

    #[test]
    fn nothing_configured_writes_nothing() {
        let t = tracking("[analytics]\n[verification]\n", Environment::Production);
        assert!(!t.active());
        assert_eq!(t.decide("/", None), Decision::Nothing);
        assert_eq!(t.head(Decision::Nothing, "n"), "");
        assert_eq!(t.verification_head(), "");
        assert_eq!(t.csp(), None);
    }

    #[test]
    fn verification_is_written_in_every_environment() {
        let toml = "[verification]\ngoogle = \"tok_en-1\"\nbing = \"AB12\"\nother = { \"x-proof\" = \"a&b\" }\ngoogle_file = \"google12ab.html\"\n";
        for env in [Environment::Development, Environment::Production] {
            let t = tracking(toml, env);
            assert_eq!(
                t.verification_head(),
                "<meta name=\"google-site-verification\" content=\"tok_en-1\"><meta name=\"msvalidate.01\" content=\"AB12\"><meta name=\"x-proof\" content=\"a&amp;b\">"
            );
            assert!(t.file("/google12ab.html").is_some());
            assert!(t.file("/google12ab.htm").is_none());
        }
    }

    #[test]
    fn analytics_are_production_only_unless_enabled_in_dev() {
        let toml = "[analytics]\ngoogle_analytics = \"G-ABCD1234\"\n";
        assert!(!tracking(toml, Environment::Development).active());
        assert!(tracking(toml, Environment::Production).active());
        assert!(tracking(&format!("{toml}enabled_in_dev = true\n"), Environment::Development).active());
    }

    #[test]
    fn google_analytics() {
        let html = head("[analytics]\ngoogle_analytics = \"G-ABCD1234\"\nrespect_dnt = false\n");
        assert!(html.starts_with(
            "<link rel=\"preconnect\" href=\"https://www.googletagmanager.com\"><link rel=\"preconnect\" href=\"https://www.google-analytics.com\"><script nonce=\"N0\">!function(w,d){"
        ));
        assert!(html.contains("T=!0;"));
        assert!(html.contains("w.dataLayer=w.dataLayer||[];function g(){w.dataLayer.push(arguments)}w.gtag=g;"));
        assert!(html.contains(
            "X(0,function(){L(\"https://www.googletagmanager.com/gtag/js?id=G-ABCD1234\");g(\"js\",new Date);g(\"config\",\"G-ABCD1234\")});"
        ));
        assert!(html.ends_with("}(window,document)</script>"));
        assert_eq!(html.matches("<script").count(), 1);
        assert!(!html.contains("consent"));
        assert!(!html.contains("nr:navigate"), "GA4 follows the History API itself");
    }

    #[test]
    fn dnt_is_checked_before_anything_connects() {
        let html = head("[analytics]\ngoogle_analytics = \"G-ABCD1234\"\n");
        assert!(html.starts_with("<script nonce=\"N0\">"), "no <link> a browser would follow: {html}");
        assert!(html.contains("T=!(N.globalPrivacyControl||N.doNotTrack==\"1\"||w.doNotTrack==\"1\");"));
        assert!(
            html.contains("T&&[\"https://www.googletagmanager.com\",\"https://www.google-analytics.com\"].forEach(")
        );
        assert!(html.contains("function X(c,f){T&&f()}"));
    }

    #[test]
    fn tag_manager_has_a_noscript_frame_after_body() {
        let t = tracking("[analytics]\ngoogle_tag_manager = \"GTM-ABC123\"\n", Environment::Production);
        let html = t.head(Decision::Tags, "N0");
        assert!(html.contains(
            "X(0,function(){w.dataLayer.push({\"gtm.start\":+new Date,event:\"gtm.js\"});L(\"https://www.googletagmanager.com/gtm.js?id=GTM-ABC123\")});"
        ));
        assert!(
            html.contains("w.addEventListener(\"nr:navigate\",function(){w.dataLayer.push({event:\"nr.page_view\"")
        );
        assert_eq!(
            t.body_start(Decision::Tags),
            "<noscript><iframe src=\"https://www.googletagmanager.com/ns.html?id=GTM-ABC123\" height=\"0\" width=\"0\" style=\"display:none;visibility:hidden\"></iframe></noscript>"
        );
        assert_eq!(t.body_start(Decision::OptOut), "");
        let csp = tracking(
            "[analytics]\ngoogle_tag_manager = \"GTM-ABC123\"\n[security]\ncsp = \"strict\"\n",
            Environment::Production,
        );
        assert!(csp.csp().unwrap().ends_with("; frame-src 'self' https://www.googletagmanager.com"));
    }

    #[test]
    fn every_provider_loads_its_script() {
        let html = head(
            r#"
[analytics]
google_ads = "AW-123456789"
microsoft_clarity = "abcd1234ef"
meta_pixel = "1234567890123456"
pinterest_tag = "2612345678901"
linkedin_insight = "7654321"
tiktok_pixel = "CABCDEFGHIJKLMNOPQRS"
hotjar = "424242"
cloudflare_web_analytics = "0123456789abcdef0123456789abcdef"
plausible = "example.com"
umami = "94db1cb1-74f4-4a40-ad6c-962362670409"
"#,
        );
        for needle in [
            "L(\"https://www.googletagmanager.com/gtag/js?id=AW-123456789\");g(\"js\",new Date);g(\"config\",\"AW-123456789\")})",
            "L(\"https://www.clarity.ms/tag/abcd1234ef\")",
            "w._hjSettings={hjid:424242,hjsv:6};L(\"https://static.hotjar.com/c/hotjar-424242.js?sv=6\")",
            "f.queue=[];L(\"https://connect.facebook.net/en_US/fbevents.js\");f(\"init\",\"1234567890123456\");f(\"track\",\"PageView\")",
            "L(\"https://s.pinimg.com/ct/core.js\")}w.pintrk(\"load\",\"2612345678901\");w.pintrk(\"page\")",
            "w._linkedin_partner_id=\"7654321\";(w._linkedin_data_partner_ids=w._linkedin_data_partner_ids||[]).push(\"7654321\")",
            "L(\"https://snap.licdn.com/li.lms-analytics/insight.min.js\")",
            "i=\"CABCDEFGHIJKLMNOPQRS\"",
            "L(u+\"?sdkid=\"+i+\"&lib=ttq\");t.page()",
            "L(\"https://plausible.io/js/script.js\",{\"data-domain\":\"example.com\"})",
            "L(\"https://cloud.umami.is/script.js\",{\"data-website-id\":\"94db1cb1-74f4-4a40-ad6c-962362670409\",\"data-do-not-track\":\"true\"})",
            "L(\"https://static.cloudflareinsights.com/beacon.min.js\",{\"data-cf-beacon\":\"{\\\"token\\\":\\\"0123456789abcdef0123456789abcdef\\\"}\"})",
            "w.addEventListener(\"nr:navigate\",function(){w.pintrk&&w.pintrk(\"page\")})",
        ] {
            assert!(html.contains(needle), "missing {needle}\nin {html}");
        }
        assert_eq!(html.matches("<script").count(), 1, "one inline script loads everything");
        assert!(!html.contains('\n'), "minified");
    }

    #[test]
    fn noscript_pixels_only_without_consent() {
        let toml = "[analytics]\nmeta_pixel = \"123456\"\nlinkedin_insight = \"77\"\npinterest_tag = \"88\"\n";
        let t = tracking(toml, Environment::Production);
        let body = t.body_start(Decision::Tags);
        assert!(body.contains("https://www.facebook.com/tr?id=123456&amp;ev=PageView&amp;noscript=1"));
        assert!(body.contains("https://px.ads.linkedin.com/collect/?pid=77&amp;fmt=img"));
        assert!(body.contains("https://ct.pinterest.com/v3/?tid=88&amp;noscript=1"));
        let t = tracking(&format!("{toml}consent = \"required\"\n"), Environment::Production);
        assert_eq!(t.body_start(Decision::Tags), "", "a pixel in <noscript> cannot wait for consent");
    }

    #[test]
    fn page_views_on_navigation_can_be_turned_off() {
        let on = head("[analytics]\nmeta_pixel = \"123456\"\npinterest_tag = \"88\"\n");
        assert!(on.contains("nr:navigate") && !on.contains("disablePushState"));
        let off =
            head("[analytics]\nmeta_pixel = \"123456\"\npinterest_tag = \"88\"\npage_view_on_navigation = false\n");
        assert!(!off.contains("nr:navigate"));
        assert!(off.contains("f.disablePushState=!0;"), "the pixel would follow the History API by itself");
    }

    #[test]
    fn consent_mode() {
        let html = head(
            "[analytics]\ngoogle_analytics = \"G-ABCD1234\"\nmeta_pixel = \"123456\"\nmicrosoft_clarity = \"abcd\"\nplausible = \"example.com\"\nconsent = \"required\"\n",
        );
        let default = html.find("g(\"consent\",\"default\",{ad_storage:\"denied\",ad_user_data:\"denied\",ad_personalization:\"denied\",analytics_storage:\"denied\",wait_for_update:500});").expect("defaults");
        let config = html.find("g(\"config\"").expect("config");
        assert!(default < config, "consent defaults come before the tag is configured");
        assert!(html.contains("n.consent={grant:function(a){S(a,1)},revoke:function(a){S(a,0)},state:"));
        assert!(
            html.contains("X(0,function(){L(\"https://www.googletagmanager.com/gtag/js"),
            "Google tags run in consent mode"
        );
        assert!(html.contains("X(\"ads\",function(){if(w.fbq)return;"), "the pixel waits for `ads`");
        assert!(html.contains("X(\"analytics\",function(){w.clarity="));
        assert!(html.contains("w.clarity(\"consentv2\",V());L("));
        assert!(
            html.contains(
                "function V(){var m=M();return{ad_Storage:m.ad_storage,analytics_Storage:m.analytics_storage}}"
            )
        );
        assert!(html.contains("g(\"consent\",\"update\",M());w.clarity&&w.clarity(\"consentv2\",V());R()"));
        assert!(html.contains("X(0,function(){w.plausible="), "cookieless tags do not wait");
        assert!(html.contains("function X(c,f){c?Q.push([c,f]):T&&f()}"));
        assert!(html.ends_with("R();}(window,document)</script>") || html.contains("R();T&&w.addEventListener"));
    }

    #[test]
    fn plausible_per_site_script_and_self_hosting() {
        let html = head("[analytics.plausible]\nscript = \"https://plausible.io/js/pa-AbC123.js\"\n");
        assert!(html.contains("w.plausible.init();L(\"https://plausible.io/js/pa-AbC123.js\")"));
        assert!(!html.contains("data-domain"));
        let t = tracking(
            "[analytics]\nplausible = { domain = \"example.com\", script = \"https://stats.example.com/js/script.js\" }\n[security]\ncsp = \"strict\"\n",
            Environment::Production,
        );
        let csp = t.csp().unwrap();
        assert!(csp.contains("'strict-dynamic' https://stats.example.com;"), "{csp}");
        assert!(csp.contains("connect-src 'self' https://stats.example.com;"), "{csp}");
    }

    #[test]
    fn strict_csp_allows_exactly_what_is_configured() {
        let plain = tracking("[security]\ncsp = \"strict\"\n", Environment::Production);
        assert_eq!(plain.csp(), Some(crate::middleware::STRICT_CSP));
        let custom = tracking(
            "[analytics]\ngoogle_analytics = \"G-ABCD1234\"\n[security]\ncsp = \"default-src 'self'\"\n",
            Environment::Production,
        );
        assert_eq!(custom.csp(), Some("default-src 'self'"), "a hand-written policy is the app's to maintain");
        let toml = "[analytics]\ngoogle_analytics = \"G-ABCD1234\"\n[security]\ncsp = \"strict\"\n";
        let dev = tracking(toml, Environment::Development);
        assert_eq!(dev.csp(), Some(crate::middleware::STRICT_CSP), "nothing is loaded in development");
        let csp = tracking(toml, Environment::Production).csp().unwrap().to_owned();
        assert_eq!(
            csp,
            "default-src 'self'; script-src 'self' 'nonce-{nonce}' 'strict-dynamic' https://www.googletagmanager.com; \
style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob: https://www.googletagmanager.com https://*.google-analytics.com; \
font-src 'self' data:; \
connect-src 'self' https://www.googletagmanager.com https://*.google-analytics.com https://*.google.com; \
media-src 'self' blob:; worker-src 'self' blob:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'"
        );
        assert!(!csp.contains("facebook") && !csp.contains("frame-src"));
    }

    #[test]
    fn exclusions_and_opt_outs() {
        let t = tracking(
            "[analytics]\ngoogle_analytics = \"G-ABCD1234\"\nexclude_paths = [\"/student/*\", \"/login\"]\nskip_noindex = true\n",
            Environment::Production,
        );
        assert_eq!(t.decide("/", None), Decision::Tags);
        assert_eq!(t.decide("/student/courses", None), Decision::Nothing);
        assert_eq!(t.decide("/login", Some(&Metadata::new())), Decision::Nothing);
        assert_eq!(t.decide("/about", Some(&Metadata::new().analytics(false))), Decision::OptOut);
        assert_eq!(t.decide("/about", Some(&Metadata::new().robots("noindex"))), Decision::OptOut);
        assert_eq!(t.decide("/about", Some(&Metadata::new().robots("noindex").analytics(true))), Decision::Tags);
        assert_eq!(t.decide("/about", Some(&Metadata::new().robots("index"))), Decision::Tags);
        assert_eq!(
            t.head(Decision::OptOut, "N0"),
            "<script nonce=\"N0\">addEventListener(\"nr:navigate\",function(){location.reload()})</script>"
        );
        let lenient = tracking("[analytics]\ngoogle_analytics = \"G-ABCD1234\"\n", Environment::Production);
        assert_eq!(lenient.decide("/x", Some(&Metadata::new().robots("noindex"))), Decision::Tags);
    }

    #[test]
    fn cached_html_is_prepared_for_a_navigation() {
        let t = tracking("[analytics]\ngoogle_tag_manager = \"GTM-ABC123\"\n", Environment::Production);
        let page = format!(
            "<head><title>t</title>{}</head><body>{}<main>x</main></body>",
            t.head(Decision::Tags, "N0"),
            t.body_start(Decision::Tags)
        );
        let clean = "<head><title>t</title></head><body><main>x</main></body>".to_owned();
        assert_eq!(t.for_navigation(page.clone()), Ok((clean.clone(), true)));
        // However the document was written (another nonce, no quotes).
        let rewritten = page.replace("nonce=\"N0\"", "nonce=abc").replace("src=\"https", "src=https");
        assert_eq!(t.for_navigation(rewritten), Ok((clean, true)));
        let opted_out = format!("<head>{}</head><body></body>", t.head(Decision::OptOut, "N0"));
        assert_eq!(t.for_navigation(opted_out), Err(()));
        assert_eq!(t.for_navigation("<head></head>".to_owned()), Ok(("<head></head>".to_owned(), false)));
        // The app's own <noscript> is not touched.
        let own = "<noscript><p>Enable JavaScript</p></noscript>";
        let page = format!("<head>{}</head><body>{own}</body>", t.head(Decision::Tags, "N0"));
        assert_eq!(t.for_navigation(page), Ok((format!("<head></head><body>{own}</body>"), true)));
    }

    #[test]
    fn values_cannot_break_out_of_the_script() {
        assert_eq!(js("</script><b>"), "\"\\u003c/script>\\u003cb>\"");
    }
}
