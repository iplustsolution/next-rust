//! The "security" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "This page describes the framework's defaults and reports the pre-1.0 audit of each area named in the project requirements. \"Tested\" means an automated test in this repository covers the behaviour.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "A website is a building open to strangers. Most break-ins use a handful of well-known tricks: slipping a script into a comment, faking a form from another site, asking for a file that was never meant to be public. Next Rust locks those doors by default, so a new project is protected before you have configured anything. This page lists each lock, how it was tested, and the few things only you can do.",
            ],
        ],
        h2![id("glossary"), a![class("anchor"), href("#glossary"), "The words used on this page"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["term"], th!["meaning"]]],
                tbody![
                    tr![
                        td![strong!["XSS (cross-site scripting)"]],
                        td![
                            "An attacker gets their own script to run inside your page, for example by typing HTML into a comment field. The script can then act as the visitor."
                        ]
                    ],
                    tr![
                        td![strong!["Escaping"]],
                        td![
                            "Turning characters such as < into harmless text (&lt;) so that what a visitor typed is shown, never executed."
                        ]
                    ],
                    tr![
                        td![strong!["CSRF (cross-site request forgery)"]],
                        td![
                            "Another website makes a visitor's browser send a request to yours, such as submitting a form in their name. The defence is checking the form really came from your own site."
                        ]
                    ],
                    tr![
                        td![strong!["CSP (Content Security Policy)"]],
                        td![
                            "A list the server sends with each page that tells the browser which scripts, images and fonts it may load. Everything else is blocked."
                        ]
                    ],
                    tr![
                        td![strong!["Nonce"]],
                        td![
                            "A random value made fresh for every response. Scripts that carry it are allowed by the CSP; injected ones cannot know it."
                        ]
                    ],
                    tr![
                        td![strong!["Cookie"]],
                        td![
                            "A small named value the browser stores for your site and sends back with every request. It is how a site recognises a returning browser."
                        ]
                    ],
                    tr![
                        td![strong!["Session"]],
                        td![
                            "The server's memory of one signed-in visitor. The browser holds only a random id in a cookie; the data stays on the server."
                        ]
                    ],
                    tr![
                        td![strong!["TLS / HTTPS"]],
                        td![
                            "The encryption between browser and server: the padlock. HSTS is a header that tells browsers to use HTTPS for your site from now on."
                        ]
                    ],
                    tr![
                        td![strong!["Reverse proxy"]],
                        td![
                            "A program (Caddy, nginx, a cloud load balancer) that stands between the internet and the app, handles TLS and passes requests on."
                        ]
                    ],
                    tr![
                        td![strong!["Path traversal"]],
                        td!["Asking for a file outside the allowed folder with tricks like ../../etc/passwd."]
                    ],
                    tr![
                        td![strong!["Open redirect"]],
                        td![
                            "A link to your site that forwards the visitor to an attacker's site, borrowing your good name."
                        ]
                    ],
                    tr![
                        td![strong!["SSRF (server-side request forgery)"]],
                        td![
                            "Tricking the server into fetching a URL of the attacker's choice, often one inside your private network."
                        ]
                    ],
                    tr![
                        td![strong!["Header injection"]],
                        td!["Smuggling a line break into a value so that it becomes an extra response header."]
                    ],
                    tr![
                        td![strong!["Denial of service"]],
                        td!["Making the site unavailable by exhausting memory, connections or processor time."]
                    ],
                    tr![
                        td![strong!["Signed (HMAC)"]],
                        td!["A value with a checksum only the server can compute, so any change to it is detected."]
                    ],
                ],
            ],
        ],
        h2![id("your-part"), a![class("anchor"), href("#your-part"), "Your part: a launch checklist"]],
        p!["The defaults cover the framework. These are the decisions it cannot make for you:"],
        ul![
            li![
                strong!["Set NEXT_RUST_SECRET on the server. "],
                "32 or more random bytes (",
                code!["openssl rand -hex 32"],
                "), the same on every instance. It signs form URLs. Without it a random one is used per process: safe, but open forms break at every restart. It is an environment variable, never a line in ",
                code!["next-rust.toml"],
                ".",
            ],
            li![
                strong!["Keep secrets out of Git and out of the page. "],
                "Put them in ",
                code![".env.local"],
                " (ignored by the generated ",
                code![".gitignore"],
                ") during development and in the server's environment in production. ",
                code!["next-rust doctor"],
                " warns when a ",
                code![".env"],
                " file is not git-ignored. Only variables starting with ",
                code!["NEXT_RUST_PUBLIC_"],
                " can reach browsers; never give a secret that prefix, and never pass one to a ",
                code!["#[client]"],
                " island.",
            ],
            li![
                strong!["Check who the visitor is in every action and API route. "],
                "The framework proves a request came from one of your pages, not that the person may do what they ask. See ",
                a![href("/docs/auth"), "auth & rate limits"],
                ".",
            ],
            li![
                strong!["Never hand visitor input to raw_html or raw_attr. "],
                "Everything else is escaped for you; these two are the explicit way out.",
            ],
            li![
                strong!["Treat route parameters as visitor input. "],
                "Look them up in a list or a database; never join them into a file path.",
            ],
            li![
                strong!["Serve the site over HTTPS "],
                "through a reverse proxy, and turn on the production switches below."
            ],
        ],
        p![
            "These lines go in ",
            code!["next-rust.toml"],
            " in the project root. All three are off by default because the framework cannot know whether a proxy and HTTPS are in place:",
        ],
        pre![code![
            class("language-toml"),
            r##"[security]
csp = "strict"              # send the ready-made Content Security Policy
hsts_max_age = 31536000     # only once the site is HTTPS-only: browsers will refuse plain HTTP for a year

[server]
trust_proxy = true          # only when a proxy is in front: read the visitor's address from X-Forwarded-*"##,
        ]],
        h3![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        p!["Ask the running site for its response headers. Run this in a terminal against your own address:"],
        pre![code![
            class("language-sh"),
            r"curl -sI https://example.com/ | grep -iE 'content-security|strict-transport|x-frame|x-content-type|referrer'",
        ]],
        p![
            "You should see ",
            code!["x-content-type-options: nosniff"],
            ", ",
            code!["x-frame-options: SAMEORIGIN"],
            " and ",
            code!["referrer-policy"],
            " on any project, plus ",
            code!["content-security-policy"],
            " (with a different ",
            code!["nonce-…"],
            " on every request) and ",
            code!["strict-transport-security"],
            " once you set the keys above. In the browser, the developer tools' console reports anything the policy blocked.",
        ],
        h2![id("defaults-at-a-glance"), a![class("anchor"), href("#defaults-at-a-glance"), "Defaults at a glance"],],
        p!["What a new project does without any configuration:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["area"], th!["default"]]],
                tbody![
                    tr![
                        td!["HTML output"],
                        td!["all text and attributes escaped; ", code!["raw_html"], " is explicit"],
                    ],
                    tr![
                        td!["URL attributes"],
                        td![
                            code!["javascript:"],
                            ", ",
                            code!["vbscript:"],
                            ", non-image ",
                            code!["data:"],
                            " neutralized",
                        ],
                    ],
                    tr![td!["Event handler attributes"], td!["dropped unless ", code!["raw_attr"]]],
                    tr![
                        td!["Response headers"],
                        td![
                            code!["X-Content-Type-Options: nosniff"],
                            ", ",
                            code!["X-Frame-Options: SAMEORIGIN"],
                            ", ",
                            code!["Referrer-Policy: strict-origin-when-cross-origin"],
                            ", ",
                            code!["Cross-Origin-Opener-Policy: same-origin"],
                            ", restrictive ",
                            code!["Permissions-Policy"],
                        ],
                    ],
                    tr![
                        td!["CSP"],
                        td![
                            "opt-in via ",
                            code!["[security] csp"],
                            "; framework inline scripts always carry a per-request nonce",
                        ],
                    ],
                    tr![
                        td!["Cookies"],
                        td![
                            code!["HttpOnly"],
                            ", ",
                            code!["SameSite=Lax"],
                            ", ",
                            code!["Path=/"],
                            ", ",
                            code!["Secure"],
                            " outside development",
                        ],
                    ],
                    tr![
                        td!["Server actions"],
                        td![
                            "POST-only, cross-site requests rejected, per-visitor signed and expiring URLs (HMAC-SHA256, bound to an ",
                            code!["HttpOnly"],
                            " ",
                            code!["SameSite=Strict"],
                            " cookie)",
                        ],
                    ],
                    tr![td!["Request bodies"], td!["2 MiB limit; header read timeout 10 s; request timeout 60 s"],],
                    tr![td!["Errors"], td!["details only in development; production shows a digest"]],
                    tr![td!["Static files"], td!["traversal-safe resolver, no dotfiles, no directory listings"],],
                    tr![td!["Secrets"], td!["only ", code!["NEXT_RUST_PUBLIC_*"], " variables can reach clients"],],
                ],
            ],
        ],
        h2![id("audit"), a![class("anchor"), href("#audit"), "Audit"]],
        p![
            "Each heading names a class of attack and the verdict. The first sentence says what the attack is; the rest says how the framework answers it and which test proves it.",
        ],
        h3![
            id("path-traversal-directory-traversal-static-file-serving-pass-tested"),
            a![
                class("anchor"),
                href("#path-traversal-directory-traversal-static-file-serving-pass-tested"),
                "Path traversal / directory traversal / static file serving — ",
                strong!["pass (tested)"],
            ],
        ],
        p![em![
            "The attack: asking for /../../etc/passwd, or an encoded variant, to read a file outside the public folder."
        ]],
        p![
            code!["static_files::resolve_safe"],
            " percent-decodes each segment. It rejects ",
            code!["."],
            ", ",
            code![".."],
            ", empty segments, segments starting with ",
            code!["."],
            ", backslashes, NUL bytes and ",
            code![":"],
            ", before touching the filesystem. It then canonicalizes the result and requires it to stay under the canonical root, which defeats symlink escapes. Only regular files are served.",
        ],
        p![
            "Tests cover ",
            code!["/../"],
            ", ",
            code!["%2e%2e"],
            ", ",
            code!["..%2F"],
            ", backslash, NUL, dotfiles, directories and symlinks pointing outside the root (",
            code!["static_files::tests"],
            ", ",
            code!["tests/pipeline.rs::public_files"],
            "). The same resolver serves ",
            code!["/_next-rust/client/"],
            ", ",
            code!["/_next-rust/assets/"],
            " and ",
            code!["/_next-rust/image"],
            ".",
        ],
        h3![
            id("html-escaping-template-injection-xss-pass-tested"),
            a![
                class("anchor"),
                href("#html-escaping-template-injection-xss-pass-tested"),
                "HTML escaping, template injection, XSS — ",
                strong!["pass (tested)"],
            ],
        ],
        p![em![
            "The attack: a visitor submits text containing a script, and the site shows it to other visitors as live HTML."
        ]],
        ul![
            li![
                "The DSL escapes text (",
                code!["& < >"],
                ") and attribute values (",
                code!["& < > \" '"],
                "). Templates are Rust code, not strings, so there is no template language to inject into.",
            ],
            li![
                "URL attributes are checked by ",
                code!["is_safe_url"],
                ", which ignores control characters and whitespace so ",
                code!["java\\tscript:"],
                " is caught.",
            ],
            li![
                code!["attr(\"onclick\", ..)"],
                " is dropped. Attribute names with quotes, spaces, ",
                code!["="],
                ", ",
                code!["/"],
                " or ",
                code![">"],
                " are dropped.",
            ],
            li![
                "Text inside ",
                code!["<script>"],
                "/",
                code!["<style>"],
                " is escaped against ",
                code!["</script"],
                " breakouts.",
            ],
            li!["Metadata values are escaped. Unsafe canonical/link URLs become ", code!["#"], "."],
            li!["Island props are JSON inside an attribute, escaped as attribute text."],
            li![
                "Streaming redirect scripts embed the location via JSON encoding with every ",
                code!["<"],
                " written as ",
                code!["\\u003c"],
                " (as is the public environment JSON). A ",
                code!["javascript:"],
                ", ",
                code!["vbscript:"],
                " or ",
                code!["data:"],
                " redirect target becomes ",
                code!["/"],
                " wherever script would follow it: streamed redirects and the redirect an action returns to the client runtime.",
            ],
        ],
        p![
            "Residual risk: ",
            code!["raw_html"],
            ", ",
            code!["raw_attr"],
            ", plugin ",
            code!["head()"],
            " markup and the contents of ",
            code!["script!"],
            " elements you write are trusted by design.",
        ],
        h3![
            id("csrf-pass-for-server-actions-tested-opt-in-elsewhere"),
            a![
                class("anchor"),
                href("#csrf-pass-for-server-actions-tested-opt-in-elsewhere"),
                "CSRF — ",
                strong!["pass for server actions (tested); opt-in elsewhere"],
            ],
        ],
        p![em![
            "The attack: a page on another site silently submits a form to yours while the visitor is signed in. The defence is checking the form really came from your own site."
        ]],
        p![
            "Server actions check ",
            code!["Sec-Fetch-Site"],
            "/",
            code!["Origin"],
            " against ",
            code!["Host"],
            ", and every action URL is a token signed with ",
            code!["NEXT_RUST_SECRET"],
            ", valid for one browser (bound to a ",
            code!["SameSite=Strict"],
            " cookie) and for a limited time. Copied, expired, tampered and guessed URLs are rejected (",
            code!["action_token::tests"],
            ", ",
            code!["tests/pipeline.rs::server_actions"],
            "). Plain API routes aren't automatically CSRF-protected, because many are called cross-origin by design. Use ",
            code!["csrf()"],
            " middleware (double-submit token, constant-time comparison) or SameSite cookies as appropriate.",
        ],
        h3![id("ssrf-pass"), a![class("anchor"), href("#ssrf-pass"), "SSRF — ", strong!["pass"]]],
        p![em![
            "The attack: making your server fetch a URL the attacker chose, such as an address inside your private network."
        ]],
        p![
            "The framework makes no outbound requests on behalf of clients. The image endpoint accepts only local paths starting with ",
            code!["/"],
            " (not ",
            code!["//"],
            ") and resolves them inside ",
            code!["public/"],
            ". Remote ",
            code!["Image!"],
            " sources are emitted as plain ",
            code!["src"],
            " attributes and never fetched by the server.",
        ],
        h3![
            id("request-smuggling-delegated-to-hyper"),
            a![
                class("anchor"),
                href("#request-smuggling-delegated-to-hyper"),
                "Request smuggling — ",
                strong!["delegated to hyper"],
            ],
        ],
        p![em![
            "The attack: a request written so that a proxy and the app disagree on where it ends, hiding a second request inside the first."
        ]],
        p![
            "HTTP parsing, ",
            code!["Content-Length"],
            "/",
            code!["Transfer-Encoding"],
            " handling and connection management are done by hyper 1.x, which rejects ambiguous framing. The framework doesn't parse HTTP itself. Deploy behind proxies that normalize requests, the standard advice for any origin server.",
        ],
        h3![
            id("open-redirects-pass-tested"),
            a![class("anchor"), href("#open-redirects-pass-tested"), "Open redirects — ", strong!["pass (tested)"],],
        ],
        p![em![
            "The attack: a link to your site (…/login?next=//evil.example) that forwards the visitor to another site after they trust yours."
        ]],
        p![
            "Redirect targets taken from the request (the ",
            code!["_redirect"],
            " form field, the ",
            code!["Referer"],
            " after a plain form post, the ",
            code!["next"],
            " parameter written by ",
            code!["protected"],
            ") must pass ",
            code!["is_local_path"],
            ": one leading ",
            code!["/"],
            ", no backslash, whitespace or control character. Browsers read ",
            code!["//host"],
            " and ",
            code!["/\\host"],
            " as another site, and remove tabs and newlines first, so ",
            code!["/%09/host"],
            " is refused too. The trailing-slash redirect collapses leading slashes, so ",
            code!["//evil.example/"],
            " never redirects to ",
            code!["//evil.example"],
            ". ",
            code!["Response::redirect"],
            " removes tabs from the location. ",
            code!["next_rust::is_local_path"],
            " is public: use it before calling ",
            code!["redirect(next)"],
            " with a value from a query string (",
            code!["response::tests"],
            ", ",
            code!["tests/pipeline.rs"],
            ").",
        ],
        h3![
            id("header-injection-pass-tested"),
            a![class("anchor"), href("#header-injection-pass-tested"), "Header injection — ", strong!["pass (tested)"],],
        ],
        p![em![
            "The attack: a line break inside a value that ends up in a response header, which adds a header of the attacker's choosing."
        ]],
        p![
            "All header writes go through ",
            code!["http::HeaderValue"],
            ", which rejects CR/LF. ",
            code!["Response::set_header"],
            " drops invalid values and logs a warning. Redirect locations use the same validation. Cookie names, values and attributes are validated (",
            code!["cookies::tests"],
            "). SSE event names and ids have newlines stripped.",
        ],
        h3![
            id("cookie-security-pass-tested"),
            a![class("anchor"), href("#cookie-security-pass-tested"), "Cookie security — ", strong!["pass (tested)"],],
        ],
        p![em![
            "The risk: a cookie read by a script, sent over plain HTTP, or a session id that can be guessed or planted."
        ]],
        p![
            "Defaults are ",
            code!["HttpOnly; SameSite=Lax; Path=/"],
            ", plus ",
            code!["Secure"],
            " outside development. ",
            code!["SameSite=None"],
            " always adds ",
            code!["Secure"],
            ". Session ids are 256 bits from the OS CSPRNG (",
            code!["getrandom"],
            "). Session data is server-side, and ",
            code!["Session::regenerate"],
            " exists to prevent fixation; the in-memory store sweeps expired sessions. Cookies named ",
            code!["__Host-…"],
            " are always sent ",
            code!["Secure; Path=/"],
            " without ",
            code!["Domain"],
            ", and ",
            code!["__Secure-…"],
            " always ",
            code!["Secure"],
            ". Flash cookies used for form errors expire after 60 seconds, never contain password-, token-, secret- or card-like fields, and always fit the browser's cookie size limit (submitted values are dropped first). ",
            code!["Request::is_secure"],
            " reads ",
            code!["X-Forwarded-Proto"],
            " only with ",
            code!["[server] trust_proxy"],
            ".",
        ],
        h3![
            id("secret-exposure-client-bundle-leakage-pass-with-caveats"),
            a![
                class("anchor"),
                href("#secret-exposure-client-bundle-leakage-pass-with-caveats"),
                "Secret exposure / client bundle leakage — ",
                strong!["pass, with caveats"],
            ],
        ],
        p![em!["The risk: a password, API key or piece of server code ending up in what the browser downloads."]],
        ul![
            li![
                "Server code is compiled only into the server binary. The framework doesn't build client bundles from application code.",
            ],
            li![
                code!["#[server]"],
                " and ",
                code!["#[server_action]"],
                " items are compiled out on ",
                code!["wasm32"],
                ", so WASM client crates can't accidentally include them.",
            ],
            li![
                "Environment variables reach HTML only if prefixed with ",
                code!["NEXT_RUST_PUBLIC_"],
                ", and only on pages with islands.",
            ],
            li![
                "Caveats: ",
                code!["#[client]"],
                " component props are serialized into the page, so never pass secrets as props. Anything you render into HTML is public.",
            ],
            li![
                "Static pages that read request data fail to render (",
                code!["DynamicUsage"],
                ") instead of caching one user's data for everyone (tested).",
            ],
        ],
        h3![
            id("minified-client-output-not-a-security-boundary"),
            a![
                class("anchor"),
                href("#minified-client-output-not-a-security-boundary"),
                "Minified client output — ",
                strong!["not a security boundary"],
            ],
        ],
        p![
            "Release builds write compact HTML (",
            code!["[build] minify_html"],
            "), give ",
            code!["data-*"],
            " attributes, custom properties, ",
            code!["@keyframes"],
            "/",
            code!["@layer"],
            " names and generated ids short random names (",
            code!["minify_names"],
            "), shorten class names (",
            code!["[tailwind] minify_classes"],
            ") and minify the scripts in ",
            code!["client/"],
            " and ",
            code!["assets/"],
            " (",
            code!["minify_js"],
            "). This makes pages smaller and tedious to read. It hides nothing: everything a browser needs to render and run the page is in the page, so a determined reader gets it back.",
        ],
        p![
            "Never put a secret, a private URL, an internal identifier or anything else that must stay unknown into markup, a stylesheet, a script or island props, whether or not these switches are on. Secrets stay on the server and reach the browser only as the results a handler chooses to render.",
        ],
        p![
            "Release builds emit and serve no source maps (there is no bundler, so none are produced at all), and no build path appears in the output: ",
            code!["next-rust build"],
            " strips symbols and remaps path prefixes, so the binary carries no names or directories from the machine that built it.",
        ],
        h3![
            id("file-uploads-limited-by-design"),
            a![
                class("anchor"),
                href("#file-uploads-limited-by-design"),
                "File uploads — ",
                strong!["limited by design"],
            ],
        ],
        p![em!["The risk: huge or malicious files filling the disk, or being served back to other visitors as pages."]],
        p![
            "Bodies are capped (",
            code!["body_limit"],
            ", ",
            code!["Content-Length"],
            " checked early, streaming limit enforced during reads). Server actions reject multipart. No upload parser is bundled. Use ",
            code!["req.take_body()"],
            " with a vetted multipart crate in an API route, store files outside ",
            code!["public/"],
            ", and validate type and size.",
        ],
        h3![
            id("denial-of-service-basic-protections"),
            a![
                class("anchor"),
                href("#denial-of-service-basic-protections"),
                "Denial of service — ",
                strong!["basic protections"],
            ],
        ],
        p![em!["The attack: using up the server's memory, connections or time so real visitors get no answer."]],
        ul![
            li![
                "header read timeout (slowloris), also for a connection that never sends a byte; request timeout; graceful shutdown (",
                code!["tests/fuzz.rs::the_server_limits_headers_and_closes_silent_connections"],
                ");",
            ],
            li![
                "HTTP/1: at most 100 headers and about 400 KB of head per request (hyper), answered ",
                code!["431"],
                "; HTTP/2: 128 streams per connection, 64 KB of headers, keep-alive pings with a timeout, and hyper's limits on reset streams (rapid reset, CVE-2023-44487);",
            ],
            li![
                "WebSocket messages are limited to 1 MiB (",
                code!["ws::upgrade_with_limit"],
                " for another limit); request bodies are never decompressed, so there is no zip bomb to defuse; server-sent events are pulled by the connection, so a slow reader slows its own stream only;",
            ],
            li!["body size limits;"],
            li![
                "bounded caches (",
                code!["MemoryStore"],
                " LRU) and a bounded rate-limit map: above 100,000 buckets new clients share one bucket, and the table is pruned at most once a second;",
            ],
            li![
                "rate limits count an IPv6 client by its /64 network, so rotating addresses inside one allocation does not reset the limit;"
            ],
            li![
                "date headers from clients (",
                code!["If-Modified-Since"],
                ") are range-checked before any arithmetic, and a panic in a handler answers 500 instead of dropping the connection — in ",
                code!["App::handle"],
                ", so every adapter and not only the built-in server;",
            ],
            li!["ISR regeneration deduplicated per path;"],
            li!["the dev-only endpoints (", code!["/_next-rust/dev/*"], ") are disabled outside development (tested).",],
        ],
        p![
            "Not provided: connection limits (total or per IP) and a write timeout for clients that stop reading a response. Use a proxy or CDN in front; for rate limits shared by several instances, the Redis limiter of ",
            code!["next_rust::auth"],
            ".",
        ],
        h3![
            id("client-addresses-behind-a-proxy"),
            a![class("anchor"), href("#client-addresses-behind-a-proxy"), "Client addresses behind a proxy"],
        ],
        p![
            "Proxies append the address they saw to ",
            code!["X-Forwarded-For"],
            "; whatever the client sent stays at the left. With ",
            code!["[server] trust_proxy"],
            ", ",
            code!["client_ip()"],
            " therefore reads the list from the right: the last entry with one proxy, the n-th from the right with ",
            code!["[server] trusted_proxies = n"],
            " (2 for a CDN in front of a load balancer). Entries further left are never used, so a client cannot choose its own address or its rate-limit bucket. Make sure the application port is reachable only through the proxy: a client that connects directly can send any header.",
        ],
        h3![
            id("error-information-leakage-pass-tested"),
            a![
                class("anchor"),
                href("#error-information-leakage-pass-tested"),
                "Error information leakage — ",
                strong!["pass (tested)"],
            ],
        ],
        p![em!["The risk: an error page that shows file paths, queries or messages an attacker can learn from."]],
        p![
            "In production, internal errors render a generic page with a random digest that is also logged. Details (messages, source file) appear only in development (",
            code!["tests/pipeline.rs::error_boundaries_and_global_errors"],
            ").",
        ],
        h2![
            id("second-audit"),
            a![class("anchor"), href("#second-audit"), "Second audit: new code and generated input"]
        ],
        p![
            "A second pass covered what was added after the first (inline scripts, partial prerendering, background jobs, the auth kit) and ran generated hostile input, with fixed seeds, against everything a request reaches before application code. What it found and fixed:",
        ],
        ul![
            li![
                strong!["Spoofed client address (medium)."],
                " The first ",
                code!["X-Forwarded-For"],
                " entry was trusted, which the client controls behind an appending proxy: a new rate-limit bucket per request. See above.",
            ],
            li![
                strong!["Page-cache poisoning through intercepting routes (medium)."],
                " A statically rendered intercepting route shared the cache entry of the page at the same URL; a request with ",
                code!["x-nr-nav"],
                " and ",
                code!["x-nr-from"],
                " chose which one later visitors got. Intercepting routes are rendered per request (",
                code!["tests/security.rs"],
                ").",
            ],
            li![
                strong!["Connections that never speak (medium)."],
                " They were held open without limit; the first byte now has to arrive within ",
                code!["[server] header_timeout"],
                ".",
            ],
            li![strong!["WebSocket memory (medium)."], " 64 MiB per message was accepted; the limit is 1 MiB.",],
            li![
                strong!["A streamed page with a form in a shared cache (medium)."],
                " Action markers are replaced with a token bound to the visitor's cookie, and a complete body is then \
                 marked ",
                code!["private, no-store"],
                ". A streamed body is read after the head is sent, so it kept whatever the application had set: a \
                 streamed, publicly cacheable page with a form handed one visitor's token and binding cookie to a \
                 shared cache. Streamed HTML carries the same ",
                code!["private, no-cache, no-store, max-age=0, must-revalidate"],
                " as every other response that must not be kept.",
            ],
            li![
                strong!["A panic outside the built-in server (medium)."],
                " Only the hyper server turned a panic in application code into a 500. ",
                code!["App::handle"],
                ", which every other adapter uses (serverless, tests, an embedding), let it unwind into the caller — \
                 with ",
                code!["[build] panic = \"abort\""],
                " that ends the process. It is caught there now, and cookies set before the panic are dropped with the \
                 half-finished response.",
            ],
            li![
                strong!["Inline scripts and shared caches (low)."],
                " Client navigations with ",
                code!["[build] inline_assets"],
                " leave out scripts the browser has and relied on ",
                code!["Vary"],
                ", which several CDNs ignore; they are ",
                code!["private, no-store"],
                " now. The HTML rewriter is tested with visitor text around real scripts and a chunk boundary at every byte: it never turns text into a script, and it no longer writes module registrations into the middle of a chunk.",
            ],
            li![
                strong!["Script injection through a file name (high)."],
                " With ",
                code!["[build] inline_assets"],
                ", the registry key of an inlined module went into JavaScript unquoted, so a file named ",
                code![r#"client/x"];alert(1);r.f["y.js"#],
                " ran its own code on every page carrying that island, and one containing ",
                code!["</script>"],
                " ended the element. Keys are JSON-encoded and escaped, the build refuses a name that cannot be \
                 written into a page, and a script declared in Rust can no longer name a path outside the build \
                 directory (the new diagnostic ",
                code!["NR0214"],
                "). The same pass fixed an inlined script being cut open by page code containing the text ",
                code![r#""</head>""#],
                ".",
            ],
            li![
                strong!["An island module from another origin (high)."],
                " An island's ",
                code!["data-module"],
                " went to ",
                code!["import()"],
                " as written, so a ",
                code!["#[client(module = ..)]"],
                " path built from application data — a CMS field, a database row — could name ",
                code!["https://evil.example/x.js"],
                " and the browser ran that code in the page. The runtime resolves the attribute against the \
                 page's origin and imports it only when it is a path on that origin (one leading ",
                code!["/"],
                ", no ",
                code!["//"],
                ", no backslash, no scheme); anything else is refused with a console error and the island keeps \
                 its server-rendered markup and declarative behaviour. The module registry of ",
                code!["[build] inline_assets"],
                " has null-prototype tables, so a key like ",
                code!["constructor"],
                " cannot find a member of ",
                code!["Object.prototype"],
                " in place of a factory.",
            ],
            li![
                strong!["A page reading the request's CSP nonce (medium)."],
                " Cached static HTML carries a placeholder where the nonce goes; it was a fixed literal, replaced \
                 anywhere in the document, so a static page that rendered that literal as ordinary text — a CMS \
                 field, a route parameter, a file name — was handed the live nonce of every request. The \
                 placeholder is random per process.",
            ],
            li![
                strong!["Open redirect through a configured redirect (high)."],
                " ",
                code!["http::Uri"],
                " accepts a backslash in a path and browsers read it as a slash, so a rule ",
                code![r#"source = "/go/:to", destination = "/:to""#],
                " answered ",
                code![r"/go/\evil.example"],
                " with a Location the browser resolves to another origin. It was the only place a piece of the \
                 request path reached a Location header.",
            ],
            li![
                strong!["XML injection into feeds (medium)."],
                " An entry date that did not parse was written into ",
                code!["<updated>"],
                ", ",
                code!["<published>"],
                " and ",
                code!["<pubDate>"],
                " unescaped, and dates routinely come from a CMS. Everything is escaped now, characters XML cannot \
                 carry are dropped, and a ",
                code!["NaN"],
                " sitemap priority is left out. ",
                code![r#"<link rel="preload">"#],
                " was also the one metadata URL that skipped the script-scheme check.",
            ],
            li![
                strong!["Parsers (low)."],
                " A non-ASCII attribute name panicked the renderer, ",
                code!["%+f"],
                " was read as a percent escape, and ",
                code!["%00"],
                " reached route parameters. All three are fixed.",
            ],
            li![
                strong!["Jobs and auth."],
                " Payload, name and error-size limits, panic isolation of hooks, corrupt Redis records, cron overflow; a password length limit before argon2, bounded parallel hashing, bounds on stored hash parameters, a limiter that stays a limiter when its table is full, sign-out of the whole session family. The changelog lists each one.",
            ],
        ],
        p![
            "Confirmed without findings: the shell of a partially prerendered page is rendered without a request, so it cannot hold cookies, headers or query values, and responses with holes are never stored by shared caches; layout keys in ",
            code!["x-nr-layouts"],
            " are hashes the server computes, so the header cannot cut a document anywhere else; ",
            code!["x-nr-scripts"],
            " only removes scripts from the sender's own response; Redis scripts take every key and value as arguments; sealed tokens bind their purpose in the key and the associated data; hyper refuses conflicting ",
            code!["Content-Length"],
            "/",
            code!["Transfer-Encoding"],
            " (tested over a socket); ",
            code!["cargo audit"],
            " reports no advisory for the lockfile.",
        ],
        p![
            strong!["A route parameter is visitor input."],
            " It is percent-decoded, so ",
            code!["[slug]"],
            " can be ",
            code!["../../etc/passwd"],
            " (from ",
            code!["..%2F..%2F"],
            "). Never join one into a filesystem path or a query string; look it up in a list of known values, or use ",
            code!["static_files::resolve_safe"],
            ".",
        ],
        p![
            "Known limits, stated plainly: no connection cap and no write timeout (a proxy's job); a static page with ",
            code!["dynamic_params"],
            " that answers 200 for any parameter fills the page cache up to its bound (10,000 entries in memory; a custom ",
            code!["FileStore"],
            " is not bounded); pages cached by a CDN carry the CSP nonce they were rendered with, so put ",
            code!["[security] csp"],
            " pages behind ",
            code!["private"],
            " caching or accept a shared nonce; ",
            code!["style-src"],
            " of the strict preset allows inline styles; an explicit ",
            code!["RENDERING = Rendering::Static"],
            " on a page that reads the request is not caught by a build diagnostic (only ",
            code!["Partial"],
            " is, as ",
            code!["NR0212"],
            ") — it fails closed when the page is rendered, as a 500 and a failed ",
            code!["next-rust build"],
            ", except for a parallel ",
            code!["@slot"],
            ", which is logged and left out of the page instead. Property tests raise confidence, they do not prove \
             absence of bugs.",
        ],
        h3![
            id("what-the-defaults-do-not-do"),
            a![class("anchor"), href("#what-the-defaults-do-not-do"), "What the defaults do not do"],
        ],
        ul![
            li![
                strong!["Your headers win. "],
                "A ",
                code!["[[headers]]"],
                " rule, or ",
                code!["set_header"],
                " in middleware or a handler, replaces a default for that response — the defaults only fill in what \
                 is missing. That is deliberate (a site may want ",
                code!["DENY"],
                " or its own policy), but it also means a rule can weaken one. Check yours.",
            ],
            li![
                strong!["HSTS is off. "],
                code!["[security] hsts_max_age = 0"],
                " by default, because the binary cannot know it is behind TLS and a stray ",
                code!["max-age"],
                " on a host later served over plain HTTP is not something a browser lets you take back. Set it \
                 (31536000) once the site is HTTPS-only; it is sent with ",
                code!["includeSubDomains"],
                ".",
            ],
            li![
                strong!["No CORP, no COEP. "],
                "Another origin may embed your images, fonts and JSON as subresources. Add ",
                code!["Cross-Origin-Resource-Policy: same-origin"],
                " with a ",
                code!["[[headers]]"],
                " rule if nothing of yours is meant to be loaded from elsewhere.",
            ],
            li![
                strong!["The framework names itself. "],
                "Every document carries ",
                code![r#"<meta name="generator" content="Next Rust x.y.z">"#],
                " and the scripts carry a banner comment, so anyone can read the exact version off a 404. ",
                code!["[build] signature = false"],
                " removes both.",
            ],
            li![
                strong!["Body and query errors quote the parser. "],
                code!["invalid JSON body: EOF while parsing a value at line 1 column 13"],
                " is returned to the client in production as well. It describes the request the client just sent, \
                 not the server, but for a typed target serde also names the expected fields — write your own ",
                code!["Error::http(400, …)"],
                " where that matters.",
            ],
            li![
                strong!["An error without a boundary is a 500. "],
                "A 400 chosen by an extractor is served as 500 unless a ",
                code!["error.rs"],
                " above it catches it and keeps the status.",
            ],
            li![
                strong!["The page and fragment store is a trust boundary. "],
                "Cached HTML is written back into a document as markup. With the default in-memory store it is only \
                 ever this process's own output; a ",
                code!["FileStore"],
                " directory that someone else can write is a markup-injection sink, so treat it like the binary.",
            ],
            li![
                strong!["A full rate-limit table limits jointly. "],
                "Above 100,000 keys the in-memory limiter counts every new key of a quota together, so a flood of \
                 made-up keys can lock out accounts it never touched. It fails strict rather than open; use the \
                 Redis limiter across instances.",
            ],
        ],
        h2![
            id("reporting-vulnerabilities"),
            a![class("anchor"), href("#reporting-vulnerabilities"), "Reporting vulnerabilities"],
        ],
        p![
            "Please report security issues privately to the maintainers before disclosure. Until a dedicated address exists, open a GitHub security advisory on the repository.",
        ],
    ]
}
