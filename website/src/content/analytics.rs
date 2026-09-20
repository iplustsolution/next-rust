//! The "analytics & verification" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Search-engine verification and analytics tags are part of the framework. You put an id into ",
            code!["next-rust.toml"],
            " and every page carries the right tag, in ",
            code!["next-rust dev"],
            " and in builds. There is no snippet to paste into a layout, no component to import and no script to write. A key you leave out (or leave empty) writes nothing.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "Two separate jobs. ",
                strong!["Verification"],
                " is proving to Google, Bing and others that the site is yours, like a letter sent to your address with a code only the resident can read back. It unlocks their webmaster tools. ",
                strong!["Analytics"],
                " is the visitor counter: how many people came, from where, to which pages. Both normally mean pasting code from the vendor into your pages. Here each is one line in the settings file, and the framework does the pasting correctly.",
            ],
        ],
        p![
            "A ",
            em!["tag"],
            " is the small piece of HTML or script a service asks you to put on your pages. An ",
            em!["id"],
            " is the code that tells the service which account the numbers belong to (",
            code!["G-XXXXXXXXXX"],
            " for Google Analytics). You copy the id from the service's dashboard; the framework writes the tag.",
        ],
        p![
            "Both sections go in ",
            code!["next-rust.toml"],
            " in the project root. This is a complete set-up: the site is verified with Google and counted by Google Analytics, except for the pages under ",
            code!["/account"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            r##"[verification]
google = "abc123…"                  # Search Console → Settings → Ownership → HTML tag → the content value

[analytics]
google_analytics = "G-XXXXXXXXXX"
exclude_paths = ["/account/*"]"##,
        ]],
        div![
            class("callout"),
            p![
                strong!["Settings, not code. "],
                "Do not paste a vendor's snippet into ",
                code!["app/layout.rs"],
                " and do not write a route for ",
                code!["ads.txt"],
                ". A pasted snippet has no CSP nonce (a strict policy blocks it), ignores consent and Do Not Track, runs in development too and can miss page views on client-side navigations. The config keys handle all of that.",
            ],
        ],
        h2![id("verification"), a![class("anchor"), href("#verification"), "Site verification"]],
        p![
            "The service shows you a tag such as ",
            code!["<meta name=\"google-site-verification\" content=\"abc123…\">"],
            ". Copy only what is between the quotes of ",
            code!["content"],
            " into the matching key, deploy, then press \"Verify\" in the service's dashboard.",
        ],
        p![
            "Each key is the ",
            code!["content"],
            " value of the tag the service shows you, not the whole tag. The framework writes a real ",
            code!["<meta>"],
            " into the ",
            code!["<head>"],
            " of every document, in every environment (services such as Seznam and Naver reject tags added by script).",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["key"], th!["service"], th!["tag"]]],
                tbody![
                    tr![
                        td![code!["google"]],
                        td!["Google Search Console (also claims the site for Merchant Center)"],
                        td![code!["google-site-verification"]]
                    ],
                    tr![td![code!["bing"]], td!["Bing Webmaster Tools"], td![code!["msvalidate.01"]]],
                    tr![td![code!["yandex"]], td!["Yandex Webmaster"], td![code!["yandex-verification"]]],
                    tr![td![code!["pinterest"]], td!["Pinterest (claim your website)"], td![code!["p:domain_verify"]]],
                    tr![
                        td![code!["facebook"]],
                        td!["Meta Business domain verification"],
                        td![code!["facebook-domain-verification"]]
                    ],
                    tr![
                        td![code!["baidu"]],
                        td!["Baidu Search Resource Platform"],
                        td![code!["baidu-site-verification"]]
                    ],
                    tr![td![code!["naver"]], td!["Naver Search Advisor"], td![code!["naver-site-verification"]]],
                    tr![td![code!["seznam"]], td!["Seznam Webmaster"], td![code!["seznam-wmt"]]],
                    tr![td![code!["other"]], td!["anything else"], td![code!["other = { \"name\" = \"content\" }"]]],
                ],
            ],
        ],
        p![
            "Some services verify with a file instead. The framework answers these URLs itself, so nothing goes into ",
            code!["public/"],
            ":"
        ],
        p!["Each key is optional; add the ones you need to ", code!["next-rust.toml"], ":"],
        pre![code![
            class("language-toml"),
            r##"[verification]
google_file = "google1234abcd5678.html"   # the file name Search Console offers; served at /google1234abcd5678.html
bing_file = "0123456789ABCDEF"            # the code inside <user>…</user> of BingSiteAuth.xml; served at /BingSiteAuth.xml
ads_txt = ["google.com, pub-0000000000000000, DIRECT, f08c47fec0942fa0"]   # /ads.txt, one record per entry
app_ads_txt = []                          # /app-ads.txt"##,
        ]],
        p![
            "Google Merchant Center has no tag of its own: it accepts a site that is verified in Search Console (",
            code!["google"],
            " above), or through Tag Manager or Analytics.",
        ],
        p![
            code!["ads.txt"],
            " is a public list of who may sell advertising on your site; ad networks read it at ",
            code!["/ads.txt"],
            ". App links are files that let a phone open your app instead of the browser for links to your site (",
            code!["assetlinks.json"],
            " for Android, ",
            code!["apple-app-site-association"],
            " for iOS).",
        ],
        p![
            "App links live under ",
            code!["/.well-known/"],
            ", which ",
            code!["public/"],
            " never serves (dot-directories are hidden). Put the JSON into the config instead; it is answered as ",
            code!["application/json"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            r##"[verification]
assetlinks = '''[{ "relation": ["delegate_permission/common.handle_all_urls"], "target": { … } }]'''   # /.well-known/assetlinks.json
apple_app_site_association = '''{ "applinks": { … } }'''   # /.well-known/apple-app-site-association and /apple-app-site-association"##,
        ]],
        h2![id("analytics"), a![class("anchor"), href("#analytics"), "Analytics"]],
        p![
            "One key per service. Set the ones you use; every other vendor's code stays out of the page entirely. The last column matters only with ",
            code!["consent = \"required\""],
            " (see ",
            a![href("#consent"), "consent"],
            "): it names the permission a tag waits for.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["key"], th!["value"], th!["waits for consent"]]],
                tbody![
                    tr![
                        td![code!["google_analytics"]],
                        td!["GA4 measurement id, ", code!["G-XXXXXXXXXX"]],
                        td!["runs in Consent Mode"]
                    ],
                    tr![
                        td![code!["google_tag_manager"]],
                        td!["container id, ", code!["GTM-XXXXXXX"]],
                        td!["runs in Consent Mode"]
                    ],
                    tr![
                        td![code!["google_ads"]],
                        td!["conversion id, ", code!["AW-123456789"]],
                        td!["runs in Consent Mode"]
                    ],
                    tr![td![code!["microsoft_clarity"]], td!["project id"], td![code!["analytics"]]],
                    tr![td![code!["hotjar"]], td!["site id"], td![code!["analytics"]]],
                    tr![td![code!["meta_pixel"]], td!["pixel id"], td![code!["ads"]]],
                    tr![td![code!["pinterest_tag"]], td!["tag id"], td![code!["ads"]]],
                    tr![td![code!["linkedin_insight"]], td!["partner id"], td![code!["ads"]]],
                    tr![td![code!["tiktok_pixel"]], td!["pixel id"], td![code!["ads"]]],
                    tr![
                        td![code!["plausible"]],
                        td!["the site's domain, or ", code!["{ domain, script }"]],
                        td!["no (cookieless)"]
                    ],
                    tr![
                        td![code!["umami"]],
                        td!["website id, or ", code!["{ website_id, script }"]],
                        td!["no (cookieless)"]
                    ],
                    tr![td![code!["cloudflare_web_analytics"]], td!["beacon token"], td!["no (cookieless)"]],
                ],
            ],
        ],
        p![
            "Ids are checked when the project builds (",
            a![href("/docs/diagnostics"), code!["NR0011"]],
            "): a Universal Analytics id (",
            code!["UA-…"],
            ") is refused with a pointer to GA4, a ",
            code!["GTM-"],
            " id in ",
            code!["google_analytics"],
            " is pointed at the right key, and so on.",
        ],
        p![
            "Plausible and Umami are privacy-friendly counters that set no cookies. They take either a plain value or, when you host them yourself, a table with the script's URL. In ",
            code!["next-rust.toml"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            r##"[analytics]
plausible = "example.com"
# self-hosted, proxied, or the per-site script Plausible now hands out:
# plausible = { domain = "example.com", script = "https://stats.example.com/js/script.js" }
# plausible = { script = "https://plausible.io/js/pa-XXXXXXXX.js" }
umami = { website_id = "94db1cb1-74f4-4a40-ad6c-962362670409", script = "https://stats.example.com/script.js" }"##,
        ]],
        h3![id("what-is-written"), a![class("anchor"), href("#what-is-written"), "What is written into the page"]],
        ul![
            li![
                "One small inline script in ",
                code!["<head>"],
                ", minified, with the request's CSP nonce. It defines the usual globals (",
                code!["dataLayer"],
                ", ",
                code!["gtag"],
                ", ",
                code!["fbq"],
                ", ",
                code!["clarity"],
                ", ",
                code!["pintrk"],
                ", ",
                code!["ttq"],
                ", ",
                code!["plausible"],
                ", …) and loads each vendor script ",
                code!["async"],
                ", passing the nonce on. Your own code can call ",
                code!["gtag(\"event\", …)"],
                " or ",
                code!["fbq(\"track\", …)"],
                " as the vendors document.",
            ],
            li![code!["preconnect"], " hints for the script origins."],
            li![
                "Tag Manager's ",
                code!["<noscript>"],
                " frame right after ",
                code!["<body>"],
                ", and ",
                code!["<noscript>"],
                " pixels for Meta, LinkedIn and Pinterest (left out when consent is required: a pixel cannot wait).",
            ],
        ],
        p![
            "Every kind of document gets them exactly once: dynamic, streamed, static and cached, partially prerendered, error and 404 pages, and full-document ",
            code!["page.html"],
            " files. Responses to client-side navigations carry none, because the document the visitor started in already has them.",
        ],
        h3![id("options"), a![class("anchor"), href("#options"), "Options"]],
        p![
            "These keys sit in the same ",
            code!["[analytics]"],
            " section, next to the ids. The values shown are the defaults, so add a line only to change it. Do Not Track and Global Privacy Control are browser settings with which a visitor asks not to be tracked.",
        ],
        pre![code![
            class("language-toml"),
            r##"[analytics]
enabled_in_dev = false           # analytics load in production only; verification tags are always written
respect_dnt = true               # load nothing (not even a preconnect) for Do Not Track / Global Privacy Control
consent = "none"                 # none | required
exclude_paths = []               # "/account" (that page), "/student/*" (it and everything below), "/blog/*/edit"
skip_noindex = false             # true: no analytics on pages whose robots metadata says noindex (404 and error pages too)
page_view_on_navigation = true   # page views on client-side navigations
anonymize_ip = false             # adds anonymize_ip to the Google tag config (GA4 never stores IPs; it ignores this)"##,
        ]],
        h3![id("navigation"), a![class("anchor"), href("#navigation"), "Client-side navigations"]],
        p![
            "Links navigate without a reload, so a page view has to be reported for each navigation. Google Analytics (enhanced measurement, on by default), the Meta Pixel, TikTok, Clarity, Hotjar, Plausible, Umami and Cloudflare follow the History API themselves. For the rest the framework listens to its own ",
            code!["nr:navigate"],
            " event: Pinterest gets ",
            code!["pintrk(\"page\")"],
            " and Tag Manager a ",
            code!["dataLayer"],
            " event named ",
            code!["nr.page_view"],
            " (with ",
            code!["page_location"],
            " and ",
            code!["page_title"],
            ") to use as a trigger. ",
            code!["page_view_on_navigation = false"],
            " turns those off, and the Meta Pixel's own tracking with them. The LinkedIn Insight Tag has no page-view call, so it counts full page loads only.",
        ],
        h3![id("excluding-pages"), a![class("anchor"), href("#excluding-pages"), "Pages without analytics"]],
        p![
            code!["exclude_paths"],
            " keeps whole areas clean, typically what is behind a login. A single page (or a layout, for everything below it) opts out in its metadata; verification tags are not affected:",
        ],
        p![
            "This goes in the page's own file (or a ",
            code!["layout.rs"],
            "), as its ",
            code!["metadata"],
            " function:",
        ],
        pre![code![
            class("language-rust"),
            r##"pub fn metadata() -> Metadata {
    Metadata::new().title("Invoice").analytics(false)
}"##,
        ]],
        p![
            "A tag that is loaded keeps watching the address bar, so moving between a page with analytics and one without has to be a full page load, and the framework makes it one: the server answers such a client navigation with ",
            code!["204"],
            " and ",
            code!["x-nr-reload: 1"],
            ", and the browser loads the URL normally. Navigations inside either area stay client-side. Nothing about an excluded page ever reaches a vendor.",
        ],
        h3![id("consent"), a![class("anchor"), href("#consent"), "Consent"]],
        p![
            "In the EU, the UK and elsewhere, tags that set cookies may only run after the visitor agreed, which is what a cookie banner is for. The framework does not draw the banner (that is your design); it holds the tags back until your banner reports the answer.",
        ],
        p![
            "With ",
            code!["consent = \"required\""],
            " Google tags start in Consent Mode v2 with ",
            code!["ad_storage"],
            ", ",
            code!["ad_user_data"],
            ", ",
            code!["ad_personalization"],
            " and ",
            code!["analytics_storage"],
            " set to ",
            code!["denied"],
            " before anything loads, and every other tag that sets cookies is not loaded at all until its category is granted. Your cookie banner calls these from its buttons (JavaScript in the browser, for example in a script under ",
            code!["client/"],
            "):",
        ],
        pre![code![
            class("language-js"),
            r##"nextRust.consent.grant();              // everything
nextRust.consent.grant(["analytics"]); // "analytics" and/or "ads"
nextRust.consent.revoke(["ads"]);
nextRust.consent.state();              // { analytics: true, ads: false }"##,
        ]],
        p![
            "The choice is kept in ",
            code!["localStorage"],
            " (",
            code!["nr-consent"],
            ") and applied on the next page before the tags run. Granting updates Consent Mode, tells Clarity (",
            code!["consentv2"],
            ") and loads what was waiting. Revoking updates Consent Mode at once; scripts that are already loaded stay until the next page load. Plausible, Umami and Cloudflare Web Analytics set no cookies and do not wait.",
        ],
        h2![id("csp"), a![class("anchor"), href("#csp"), "Content Security Policy"]],
        p![
            "A Content Security Policy (CSP) is the list of addresses a page may load scripts and images from; the browser blocks everything else (",
            a![href("/docs/security"), "security"],
            "). Analytics scripts come from the vendors' servers, so they have to be on that list. You do not maintain it:",
        ],
        p![
            "With ",
            code!["[security] csp = \"strict\""],
            " the policy allows exactly the origins of the tags you configured, nothing for the ones you did not: ",
            code!["script-src"],
            ", ",
            code!["connect-src"],
            ", ",
            code!["img-src"],
            ", and ",
            code!["frame-src"],
            " for Tag Manager and Google Ads. The inline script carries the nonce, and ",
            code!["'strict-dynamic'"],
            " lets the vendor scripts it loads run. For Google Analytics that is:",
        ],
        pre![code![
            class("language-text"),
            r##"script-src  … https://www.googletagmanager.com
img-src     … https://www.googletagmanager.com https://*.google-analytics.com
connect-src … https://www.googletagmanager.com https://*.google-analytics.com https://*.google.com"##,
        ]],
        p![
            "A policy you wrote yourself is yours to extend. Two things Google documents that no static policy can cover: Google Signals and Ads remarketing also contact country domains (",
            code!["https://www.google.<tld>"],
            "), which cannot be wildcarded, and Custom HTML tags inside a Tag Manager container need the nonce passed on in the container.",
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        ul![
            li![
                strong!["Verification: "],
                "open any page, choose \"View page source\" and search for the service's name (",
                code!["google-site-verification"],
                "). It is there in ",
                code!["next-rust dev"],
                " too. For file methods, open ",
                code!["/ads.txt"],
                " or ",
                code!["/google….html"],
                " in the browser.",
            ],
            li![
                strong!["Analytics: "],
                "not in ",
                code!["next-rust dev"],
                " unless ",
                code!["enabled_in_dev = true"],
                ". Run ",
                code!["next-rust build"],
                " and ",
                code!["next-rust start"],
                ", open the site and watch the vendor's real-time view, or the Network tab of the developer tools for a request to the vendor.",
            ],
            li![
                strong!["Nothing arrives? "],
                "Check that your own browser does not send Do Not Track or Global Privacy Control (",
                code!["respect_dnt"],
                "), that no ad blocker is active, that the page is not under ",
                code!["exclude_paths"],
                ", and with ",
                code!["consent = \"required\""],
                " that consent was granted.",
            ],
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["Pasting the whole <meta> tag. "],
                "The key takes the ",
                code!["content"],
                " value only; the whole tag is ",
                a![href("/docs/diagnostics"), code!["NR0010"]],
                ".",
            ],
            li![
                strong!["An old UA-… id. "],
                "Universal Analytics was shut down; use the GA4 ",
                code!["G-…"],
                " id (",
                code!["NR0011"],
                ").",
            ],
            li![
                strong!["An id under the wrong key. "],
                code!["GTM-…"],
                " belongs to ",
                code!["google_tag_manager"],
                ", ",
                code!["AW-…"],
                " to ",
                code!["google_ads"],
                ". The build says so.",
            ],
            li![
                strong!["A misspelled key. "],
                code!["google_analytic = …"],
                " is a hard error, like every unknown key in ",
                code!["next-rust.toml"],
                ".",
            ],
            li![strong!["A Plausible domain with https://. "], "It is the bare domain: ", code!["example.com"], ".",],
            li![strong!["Expecting numbers from localhost. "], "Analytics load in production only by default.",],
            li![
                strong!["Google Analytics and the same id inside Tag Manager. "],
                "Every page view is counted twice. Configure the id in one place.",
            ],
            li![
                strong!["A snippet in a layout as well as the key. "],
                "Same double counting, and the snippet ignores CSP, consent and exclusions.",
            ],
        ],
        h2![id("less-client-work"), a![class("anchor"), href("#less-client-work"), "Cost"]],
        p![
            "The framework's part is a few hundred bytes per configured tag, inline, and no request. Everything else is the vendor's script, loaded ",
            code!["async"],
            " so it never blocks rendering. Pages under ",
            code!["exclude_paths"],
            " get nothing at all.",
        ],
    ]
}
