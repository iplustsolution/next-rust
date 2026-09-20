//! The "configuration" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Configuration lives in ",
            code!["next-rust.toml"],
            ". ",
            code!["next-rust.json"],
            " with the same shape is also accepted. ",
            strong!["Every key is optional."],
            " A project without a config file uses the defaults below. Unknown keys are errors, so typos surface immediately.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                code!["next-rust.toml"],
                " is the settings screen of your app, written as a text file. Each line is one switch or one value: which port the site listens on, whether Tailwind is on, which security rules apply, which analytics id to use. You change a line, save, and the app behaves differently. No code is involved.",
            ],
            p![
                "You never have to fill it in. Every setting has a sensible default, so most projects keep a file of five to twenty lines and leave the rest alone.",
            ],
        ],
        h2![id("the-file"), a![class("anchor"), href("#the-file"), "The file at a glance"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["question"], th!["answer"]]],
                tbody![
                    tr![
                        td!["What is it?"],
                        td![
                            "A plain text file in TOML format: ",
                            code!["[section]"],
                            " headings, then ",
                            code!["key = value"],
                            " lines. Everything after a ",
                            code!["#"],
                            " is a comment.",
                        ],
                    ],
                    tr![
                        td!["Where does it live?"],
                        td![
                            "In the project's root folder, next to ",
                            code!["Cargo.toml"],
                            " and the ",
                            code!["app/"],
                            " folder. ",
                            code!["next-rust new"],
                            " creates it for you.",
                        ],
                    ],
                    tr![
                        td!["Who reads it?"],
                        td![
                            "The build script (",
                            code!["build.rs"],
                            ") when the project compiles, the ",
                            code!["next-rust"],
                            " commands, and the app itself when it starts.",
                        ],
                    ],
                    tr![
                        td!["What happens without it?"],
                        td!["Nothing breaks. The app runs with the defaults listed in the reference below."],
                    ],
                    tr![
                        td!["What happens in production?"],
                        td![
                            code!["next-rust build"],
                            " copies the settings into the finished program (the binary), so there is no settings file to upload to the server.",
                        ],
                    ],
                ],
            ],
        ],
        p![
            "A small, complete file looks like this. It goes in the project root; ",
            code!["next-rust new"],
            " writes a similar one, with Tailwind switched on:",
        ],
        pre![code![
            class("language-toml"),
            r##"# next-rust.toml
[app]
directory = "app"      # the folder that holds the pages

[server]
port = 3000            # the site answers on http://localhost:3000

[tailwind]
enabled = true         # generate CSS from the classes used in the pages"##,
        ]],
        h2![id("sections"), a![class("anchor"), href("#sections"), "What each section is for"]],
        p![
            "The file is divided into sections. Each one controls one part of the app. Read this table to find the section you need, then look up the exact key in the ",
            a![href("#reference"), "reference"],
            ".",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["section"], th!["in plain words"]]],
                tbody![
                    tr![
                        td![code!["[app]"]],
                        td![
                            "Where your pages are. Names the folder that holds the routes (",
                            code!["app/"],
                            "), the folder of files served as they are (",
                            code!["public/"],
                            "), and the language of the site.",
                        ],
                    ],
                    tr![
                        td![code!["[api]"]],
                        td!["Where JSON endpoints live and which URL prefix they get (", code!["/api"], ")."],
                    ],
                    tr![
                        td![code!["[server]"]],
                        td![
                            "How the running program answers the network: address, port, how large a request may be, how long it may take, and whether a proxy stands in front.",
                        ],
                    ],
                    tr![
                        td![code!["[build]"]],
                        td![
                            "How ",
                            code!["next-rust build"],
                            " makes the production program: smaller or faster, what is minified, which names must not be shortened.",
                        ],
                    ],
                    tr![
                        td![code!["[tailwind]"]],
                        td!["Turns Tailwind CSS on and configures it: theme colors, plugins, classes to keep."],
                    ],
                    tr![
                        td![code!["[assets]"]],
                        td![
                            "How stylesheets are trimmed and how long browsers may keep files from ",
                            code!["public/"],
                            "."
                        ],
                    ],
                    tr![
                        td![code!["[rendering]"]],
                        td![
                            "Whether pages are prepared ahead of time (static) or built for each visitor (dynamic), and how often a prepared page is refreshed.",
                        ],
                    ],
                    tr![
                        td![code!["[client]"]],
                        td![
                            "What the small browser script does, such as sending forms without reloading the page, and whether it is JavaScript or Rust compiled to WebAssembly.",
                        ],
                    ],
                    tr![
                        td![code!["[ui]"]],
                        td![
                            "Whether the shipped components' behaviour is JavaScript or Rust compiled to WebAssembly. See ",
                            a![href("/docs/wasm-islands"), "WebAssembly islands"],
                            ".",
                        ],
                    ],
                    tr![
                        td![code!["[animation]"], ", ", code!["[loader]"]],
                        td![
                            "The default motion: pages fading into each other, hover effects easing, and the loading bar along the top."
                        ],
                    ],
                    tr![td![code!["[images]"]], td!["How long browsers may cache images the framework serves."]],
                    tr![
                        td![code!["[security]"]],
                        td![
                            "The protective rules: security headers, the Content Security Policy, the check that forms come from your own site, HTTPS enforcement. See ",
                            a![href("/docs/security"), "security"],
                            ".",
                        ],
                    ],
                    tr![
                        td![code!["[env]"]],
                        td![
                            "Which environment variables browsers are allowed to see (only those with the public prefix)."
                        ],
                    ],
                    tr![
                        td![code!["[dev]"]],
                        td![
                            "How ",
                            code!["next-rust dev"],
                            " behaves: what it watches, the error overlay, starter code for new files."
                        ],
                    ],
                    tr![td![code!["[logging]"]], td!["How much the app prints, and in which format."]],
                    tr![
                        td![code!["[[redirects]]"], ", ", code!["[[headers]]"]],
                        td![
                            "Lists (note the double brackets; repeat the block for each entry). Send an old address to a new one, or add a response header to a group of URLs.",
                        ],
                    ],
                    tr![
                        td![code!["[verification]"], ", ", code!["[analytics]"]],
                        td![
                            "Prove to Google, Bing and others that the site is yours, and add visitor statistics. See ",
                            a![href("/docs/analytics"), "analytics & verification"],
                            ".",
                        ],
                    ],
                    tr![
                        td![code!["[plugins.<name>]"]],
                        td![
                            "Free-form settings for ",
                            a![href("/docs/plugins"), "plugins"],
                            ". The only place where keys of your own are allowed.",
                        ],
                    ],
                ],
            ],
        ],
        h2![id("discovery"), a![class("anchor"), href("#discovery"), "Discovery"]],
        p![
            "Discovery is how a command or the app finds its settings file. You rarely think about it: run commands from the project folder and the file is found. The exact order:",
        ],
        ol![
            li![code!["NEXT_RUST_CONFIG=/path/to/file"], " wins."],
            li![
                "Otherwise, the working directory and its ancestors are searched for ",
                code!["next-rust.toml"],
                " or ",
                code!["next-rust.json"],
                ". Having both in one directory is an error.",
            ],
            li![
                "A compiled app that can't find its configuration from the working directory falls back to the project root recorded at build time. This lets ",
                code!["./target/release/my-app"],
                " run from anywhere.",
            ],
        ],
        p![
            "Relative paths resolve against the ",
            strong!["directory containing the config file"],
            ", never the process working directory. ",
            code![".."],
            " is allowed.",
        ],
        h2![id("reference"), a![class("anchor"), href("#reference"), "Reference"]],
        p![
            "Every section and key, with its default value and a short note. Do not copy the block whole: put only the lines you want to change into ",
            code!["next-rust.toml"],
            " in the project root. A few lines show an example rather than a default, because the default is \"not set\": ",
            code!["[api] directory"],
            ", ",
            code!["[rendering] revalidate"],
            ", ",
            code!["[security] csp"],
            ", ",
            code!["[logging] level"],
            " and ",
            code!["requests"],
            ", the ",
            code!["[[redirects]]"],
            " and ",
            code!["[[headers]]"],
            " entries, and ",
            code!["[plugins.search]"],
            ".",
        ],
        pre![code![
            class("language-toml"),
            r##"[app]
directory = "app"            # routing directory: "src/web", "../shared/pages", "/abs/path"
public = "public"
base_path = ""               # mount everything under e.g. "/docs" (links and assets are not rewritten: include the prefix yourself)
trailing_slash = false       # true: /about → /about/
lang = "en"                  # <html lang>
html_precedence = "error"    # page.rs + page.html in one directory: error | rs | html
follow_symlinks = true       # symlink loops are detected either way

[api]
directory = "api"            # optional second tree with route.rs files only
prefix = "/api"

[server]
host = "0.0.0.0"
port = 3000                  # PORT / HOST environment variables override
body_limit = 2097152         # bytes
header_timeout = 10          # seconds to receive request headers
request_timeout = 60         # seconds until a handler is aborted with 504 (0 = off)
shutdown_timeout = 10        # graceful shutdown grace period
compression = true           # gzip
http2 = false                # h2c; needs the `http2` feature of next-rust
trust_proxy = false          # trust X-Forwarded-* headers
trusted_proxies = 1          # with trust_proxy: proxies in front; the client is that many X-Forwarded-For entries from the right

[build]
output = ".next-rust"
concurrency = 8              # reserved: static generation currently renders pages sequentially
optimize = "size"            # size (opt-level "z") | speed (opt-level 3)
panic = "unwind"             # unwind: a panic fails one request | abort: smaller, a panic stops the server
signature = true             # <meta name="generator"> on every page and a banner on the framework's scripts
minify_js = true             # release: minify and mangle the scripts in client/ and assets/
drop_unused_classes = true   # release: leave out generated-looking classes (sm:x, bg-[…]) no rule or script knows
inline_assets = false        # true: write scripts into the page (runtime, islands, asset! scripts) instead of linking them
minify_html = true           # release: HTML without what browsers do not need (quotes, ="", default attributes, optional end tags); text is never touched
minify_names = true          # release: short random names for data-* attributes, custom properties (--primary → --b3), @keyframes, @layer and generated ids
keep_names = []              # names minify_names leaves alone: "data-state", "--brand", a keyframes or layer name, an id; "data-chart-*" keeps a family
rename_ids = []              # ids minify_names may rename ("newsletter-email", "auth-*"); ids are public, so no author-written id is renamed by default
svg_precision = 3            # decimals kept in SVG path data (d, points); 0, or minify_html = false, leaves them as written
drop_console = true          # release: remove console.* calls from the scripts that are minified (debugger statements always go)
wasm_islands = false         # true: the islands/ crate contributes to the site's one WebAssembly module, for #[client(wasm = "…")] islands.
                             # `next-rust generate islands` writes the crate and sets this key; `next-rust doctor` checks the rest.
                             # Needs the wasm32-unknown-unknown target; a missing target is a warning (NR0303), never a failed build
wasm_opt = true              # shrink every wasm module with wasm-opt -Oz when it is on PATH (~10-15%); false ships what the linker wrote
wasm_keep_names = false      # true: keep the wasm name section, so functions carry their Rust names in a profiler (a few kB larger).
                             # Not the same key as keep_names above, which is about minify_names

[tailwind]
enabled = false              # generate Tailwind CSS from the classes in app/ and src/
preflight = true             # Tailwind's base styles
dark_mode = "media"          # media | class | attribute
plugins = []                 # "@tailwindcss/typography", "@tailwindcss/forms"
safelist = []                # classes to generate even if not found in the source
sources = []                 # more directories to scan
stylesheets = []             # CSS files compiled with Tailwind, in order (NR0008 if missing)
css = ""                     # extra Tailwind CSS (keyframes, @layer, …)
minify_classes = true        # release builds: short random class names (rounded-lg → k7)
keep_classes = []            # classes that keep their names (used by code outside the views)
# [tailwind.theme]           color-brand = "#f26b2a"  → @theme variables
# [tailwind.utilities]       btn = "rounded-lg px-4 py-2"  → custom classes
# [tailwind.variants]        hocus = "&:hover, &:focus"  → custom variants

[assets]
optimize = true              # reserved: CSS is always minified at compile time
public_max_age = 0
prune_css = true             # release builds leave out CSS rules for classes the app never uses
css_safelist = []            # names built at runtime that keep their rules: classes, ids, and per page attributes ("data-mode") and elements ("<dialog")
css_keep_vars = []           # custom properties ("--brand") every page keeps though nothing on it reads them
css_static_attributes = []   # attributes a script names but that only have the values the server renders ("data-theme"): rules for other values are left out

[rendering]
default = "auto"             # auto | static | dynamic ("server" = dynamic)
streaming = true
revalidate = 300             # default ISR interval for static pages (omit = never)
client_navigation = true     # Link! loads the client runtime

[client]
enhance_forms = true         # same-origin forms are sent without a page reload; false: only server actions
runtime = "js"               # "wasm": ship the framework's browser runtime as Rust compiled to WebAssembly instead of runtime.js.
                             # This one needs nothing from the project — no islands/ crate, no wasm target: the module is
                             # built into the framework and served from /_next-rust/runtime.wasm

[ui]
script = "js"                # "wasm": take the shipped components' behaviour from Rust compiled to WebAssembly.
                             # Needs the islands/ crate, with `start: next_rust_ui_wasm::boot` in its islands! —
                             # without it every component renders correctly and is inert (NR0310).
                             # `next-rust generate islands --ui` writes both

[animation]
enabled = true               # false leaves every default animation out
pages = true                 # pages fade into each other (view transitions)
hover = true                 # hover, focus and state changes ease instead of jumping
duration = 200               # ms, hover and state changes
page_duration = 250          # ms, the fade between two pages
easing = "cubic-bezier(0.2, 0, 0, 1)"

[loader]
enabled = true               # the bar along the top while the next page loads
color = "var(--nr-primary, #006fee)"   # any CSS color: "#f26b2a", "var(--color-brand)"
height = 4                   # px
shadow = true                # a glow, brightest under the leading edge
spinner = false              # a small spinner in the corner
delay = 100                  # ms before it shows; instant navigations never flash it
speed = 200                  # ms to fill, and as many to fade
easing = "ease"              # of the fill
z_index = 1600
at_bottom = false            # run along the bottom instead

[images]
sizes = [640, 750, 828, 1080, 1200, 1920, 2048, 3840]   # reserved for the image optimizer; Image! uses these defaults
quality = 75                 # reserved for the image optimizer
max_age = 2592000            # Cache-Control max-age of /_next-rust/image responses

[security]
headers = true               # nosniff, SAMEORIGIN, referrer policy, COOP, permissions policy
csp = "strict"               # the preset below, or your own policy with {nonce}
csrf = "origin"              # origin | token | off (server actions)
hsts_max_age = 0             # e.g. 31536000 when served only over HTTPS
allowed_origins = []         # extra origins allowed to call server actions
action_token_ttl = 43200     # seconds a server-action URL stays valid (secret: NEXT_RUST_SECRET)

[env]
public_prefix = "NEXT_RUST_PUBLIC_"   # a prefix under 4 characters is refused: an empty one
                                      # would publish the whole environment to browsers

[dev]
poll_interval = 250          # ms
overlay = true
watch = []                   # extra paths that trigger rebuilds
scaffold = true              # fill new, empty page.rs/layout.rs/route.rs/... with starter code

[logging]
format = "auto"              # auto (pretty in dev, JSON in prod) | pretty | json
level = "info"               # error | warn | info | debug | trace (default: info in development, error in production)
requests = true              # one line per request (default: on in development, off in production)

[[redirects]]
source = "/old/:slug"
destination = "/new/:slug"
permanent = true

[[headers]]
source = "/api/:path*"
headers = { "x-robots-tag" = "noindex" }

[verification]               # site-ownership proofs: see "Analytics & verification"
google = ""                  # <meta name="google-site-verification">; also bing, yandex, pinterest, facebook, baidu, naver, seznam
other = {}                   # { "meta-name" = "content" }
google_file = ""             # "google1234abcd.html", answered at /google1234abcd.html
bing_file = ""               # the code inside BingSiteAuth.xml, answered at /BingSiteAuth.xml
ads_txt = []                 # lines of /ads.txt; app_ads_txt for /app-ads.txt
assetlinks = ""              # JSON of /.well-known/assetlinks.json; apple_app_site_association likewise

[analytics]                  # tags written by the framework; empty = nothing
google_analytics = ""        # "G-XXXXXXXXXX"
google_tag_manager = ""      # "GTM-XXXXXXX"
google_ads = ""              # "AW-123456789"
microsoft_clarity = ""       # also meta_pixel, pinterest_tag, linkedin_insight, tiktok_pixel, hotjar,
cloudflare_web_analytics = "" #   plausible = "example.com", umami = "<website id>"
enabled_in_dev = false       # analytics are production-only; verification tags are always written
respect_dnt = true           # nothing loads for Do Not Track / Global Privacy Control
consent = "none"             # none | required (Consent Mode v2 denied until nextRust.consent.grant())
exclude_paths = []           # "/account/*": no analytics there; crossing the boundary is a full page load
skip_noindex = false         # true: none on noindex pages either
page_view_on_navigation = true
anonymize_ip = false

[plugins.search]             # free-form, read by plugins
index = "abc""##,
        ],],
        h3![
            id("production-renaming"),
            a![class("anchor"), href("#production-renaming"), "Names that change in production"],
        ],
        p![
            "To make pages smaller, a production build (",
            code!["next-rust build"],
            ") replaces long names with short random ones: Tailwind classes (",
            code!["[tailwind] minify_classes"],
            "), and ",
            code!["data-*"],
            " attributes, CSS custom properties, ",
            code!["@keyframes"],
            " and ",
            code!["@layer"],
            " names and generated ids (",
            code!["[build] minify_names"],
            "). ",
            code!["next-rust dev"],
            " never does this, which is why a problem here shows up in production only. If a name is known to code the build cannot read, list it in ",
            code!["[tailwind] keep_classes"],
            " or ",
            code!["[build] keep_names"],
            " and it keeps its spelling. This goes in ",
            code!["next-rust.toml"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            r##"[tailwind]
keep_classes = ["hidden", "opacity-0"]     # Tailwind classes a third-party script toggles by name

[build]
keep_names = ["data-state", "--brand", "data-chart-*"]   # "*" keeps a whole family"##,
        ]],
        p![
            code!["minify_names"],
            " rewrites a name everywhere the build can follow it: the HTML, every stylesheet in the page, inline ",
            code!["style"],
            " attributes, the framework's component script, and the scripts in ",
            code!["client/"],
            " and ",
            code!["assets/"],
            " (their string literals and ",
            code!["el.dataset.x"],
            " accesses). What refers to an id is rewritten with it: ",
            code!["for"],
            ", ",
            code!["form"],
            ", ",
            code!["list"],
            ", ",
            code!["popovertarget"],
            ", the ",
            code!["aria-*"],
            " relations, ",
            code!["headers"],
            ", ",
            code!["href=\"#id\""],
            " and ",
            code!["url(#id)"],
            " in SVG ",
            code!["fill"],
            "/",
            code!["stroke"],
            "/",
            code!["filter"],
            "/",
            code!["mask"],
            "/",
            code!["clip-path"],
            ". A name keeps its spelling when something the build cannot rewrite knows it: a file under ",
            code!["public/"],
            ", a name in a regular expression or ",
            code!["dataset[\"…\"]"],
            ", a name built at runtime (",
            code!["\"data-\" + key"],
            "), a selector or script inside a Rust string, any name the ",
            code!["islands/"],
            " crate uses (its strings are compiled into a WebAssembly module), everything under ",
            code!["data-nr-"],
            " (the framework's own protocol), and ",
            code!["keep_names"],
            ". The build report records what was kept and why. Short names make pages smaller and tedious to read; they hide nothing, so the rules on the security page still apply.",
        ],
        h3![id("csp"), a![class("anchor"), href("#csp"), "The Content Security Policy"]],
        p![
            "A Content Security Policy (CSP) is a list the server sends with every page that tells the browser which scripts, images and fonts it may load. Anything not on the list is blocked, so code an attacker manages to inject does not run. A ",
            em!["nonce"],
            " is a random value made fresh for every response: scripts that carry it are allowed, all others are not. No policy is sent unless you set ",
            code!["[security] csp"],
            ".",
        ],
        p![
            code!["{nonce}"],
            " in ",
            code!["security.csp"],
            " is replaced with the per-request nonce that the framework puts on its own scripts. ",
            code!["csp = \"strict\""],
            " stands for a ready-made policy (",
            code!["next_rust::STRICT_CSP"],
            "): scripts only with that nonce (",
            code!["'strict-dynamic'"],
            " lets them load modules and islands), images, fonts, media and connections from this origin only, no plugins, no framing by other sites, forms only to this origin. Inline ",
            code!["style"],
            " attributes stay allowed, because components set widths and custom properties with them.",
        ],
        p!["This is the exact policy ", code!["csp = \"strict\""], " sends, one rule per line:"],
        pre![code![class("language-text"), next_rust::STRICT_CSP.replace("; ", ";\n")]],
        h3![id("same-origin"), a![class("anchor"), href("#same-origin"), "Requests from your own pages only"]],
        p![
            "Cross-site request forgery (CSRF) is another website making a visitor's browser send a request to yours, for example submitting a form in their name. The defence is checking that the request really came from one of your own pages.",
        ],
        p![
            "Requests that change state should come from your own pages. Server actions verify that already; in an API route or a WebSocket handler, call ",
            code!["req.same_origin()"],
            " and answer 403 when it is false. It reads ",
            code!["Sec-Fetch-Site"],
            ", then ",
            code!["Origin"],
            " against the request host, and allows ",
            code!["security.allowed_origins"],
            ".",
        ],
        h2![id("environment-variables"), a![class("anchor"), href("#environment-variables"), "Environment variables"],],
        p![
            "An environment variable is a named value handed to a program from outside, such as ",
            code!["DATABASE_URL"],
            " or ",
            code!["PORT"],
            ". It is how you give the app passwords, API keys and per-server settings without writing them into the code. ",
            code!["next-rust.toml"],
            " is for settings that are the same everywhere and safe to publish; environment variables are for secrets and for what differs between your laptop and the server.",
        ],
        p![
            "A ",
            code![".env"],
            " file is a convenience for development: a text file in the project root with one ",
            code!["NAME=value"],
            " per line, read when the app starts. Without any ",
            code![".env"],
            " file nothing breaks; the app only sees the variables of the process. New projects get a ",
            code![".env.example"],
            " to copy from:",
        ],
        pre![code![
            class("language-sh"),
            r##"# .env.local (project root, never committed)
DATABASE_URL=postgres://localhost/app            # server only
NEXT_RUST_PUBLIC_SITE_NAME=My App                # the public prefix: browsers may see this one"##,
        ]],
        div![
            class("callout"),
            p![
                strong!["Keep secrets out of Git. "],
                "The ",
                code![".gitignore"],
                " of a new project ignores ",
                code![".env*.local"],
                ", so put real secrets in ",
                code![".env.local"],
                ". If you keep them in a plain ",
                code![".env"],
                ", add that to ",
                code![".gitignore"],
                " too. ",
                code!["next-rust doctor"],
                " warns when a ",
                code![".env"],
                " file exists and ",
                code![".gitignore"],
                " does not mention it. ",
                code![".env"],
                " files are not packed into the production binary: on the server, set the variables in the environment of the process.",
            ],
        ],
        p![code![".env"], " files are loaded at startup, from lowest to highest priority:"],
        ol![
            li![code![".env"]],
            li![code![".env.{development|production|test}"]],
            li![code![".env.local"], " (skipped in ", code!["test"], ")"],
            li![code![".env.{environment}.local"]],
        ],
        p![
            "Variables already set in the process environment always win over files. ",
            code!["NEXT_RUST_ENV"],
            " selects the environment. It defaults to ",
            code!["development"],
            " for debug builds and ",
            code!["production"],
            " for release builds, and ",
            code!["next-rust dev"],
            " / ",
            code!["start"],
            " set it for you.",
        ],
        p![
            strong!["Server-only vs public."],
            " All variables are available to server code via ",
            code!["std::env::var"],
            ". Only variables starting with the public prefix (",
            code!["NEXT_RUST_PUBLIC_"],
            ") are ever sent to browsers, and only on pages that render islands (as ",
            code!["window.nextRust.env"],
            "). A server secret never reaches the client unless you render it into HTML yourself.",
        ],
        p!["Framework variables:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["variable"], th!["effect"]]],
                tbody![
                    tr![
                        td![code!["NEXT_RUST_ENV"]],
                        td![code!["development"], ", ", code!["production"], ", ", code!["test"]],
                    ],
                    tr![td![code!["NEXT_RUST_CONFIG"]], td!["explicit config file"]],
                    tr![td![code!["PORT"], ", ", code!["HOST"]], td!["override ", code!["[server]"]]],
                    tr![td![code!["NO_COLOR"]], td!["disable ANSI colours"]],
                    tr![td![code!["NEXT_RUST_FRAMEWORK_PATH"]], td![code!["next-rust new"], " uses a local checkout"],],
                ],
            ],
        ],
        h2![
            id("monorepos-and-workspaces"),
            a![class("anchor"), href("#monorepos-and-workspaces"), "Monorepos and workspaces"],
        ],
        p![
            "A monorepo is one repository that holds several projects; a Cargo workspace is Rust's way of building them together. Skip this section if your repository is a single app.",
        ],
        p![
            "Each application package has its own ",
            code!["next-rust.toml"],
            ". The routing directory can live outside the package:",
        ],
        pre![code![
            class("language-text"),
            r#"repo/
├── Cargo.toml                 [workspace]
├── shared/pages/              routes shared by several apps
└── apps/
    ├── site/
    │   ├── Cargo.toml
    │   ├── build.rs
    │   └── next-rust.toml     [app] directory = "../../shared/pages"
    └── admin/
        └── next-rust.toml     [app] directory = "app""#,
        ],],
        p![
            "Workspace members build as usual (",
            code!["cargo build -p site"],
            "). Run the CLI commands from the app's directory.",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["A misspelled or invented key. "],
                "Unknown keys are hard errors: ",
                code!["prot = 3000"],
                " under ",
                code!["[server]"],
                " stops the build and names the key. That is deliberate, so a typo is never silently ignored. Only ",
                code!["[plugins.<name>]"],
                " accepts keys of your own.",
            ],
            li![
                strong!["A key in the wrong section. "],
                code!["port"],
                " belongs to ",
                code!["[server]"],
                ", ",
                code!["keep_classes"],
                " to ",
                code!["[tailwind]"],
                ", ",
                code!["keep_names"],
                " to ",
                code!["[build]"],
                ". A key under the wrong heading is an unknown key.",
            ],
            li![
                strong!["Styles or scripts that break in production only. "],
                "A class, ",
                code!["data-*"],
                " attribute or CSS variable was renamed and something the build cannot read still uses the old name. See ",
                a![href("#production-renaming"), "names that change in production"],
                ". Do not turn Tailwind or minification off to fix it.",
            ],
            li![
                strong!["Changing the release profile in Cargo.toml. "],
                code!["next-rust build"],
                " overrides ",
                code!["[profile.release]"],
                ". Use ",
                code!["[build] optimize"],
                " and ",
                code!["[build] panic"],
                " instead.",
            ],
            li![strong!["Both next-rust.toml and next-rust.json in one folder. "], "That is an error. Keep one.",],
            li![
                strong!["Secrets in next-rust.toml. "],
                "The file is committed and embedded in the binary. Secrets belong in environment variables.",
            ],
            li![
                strong!["Analytics pasted into a layout. "],
                "Tags and verification codes are settings, under ",
                code!["[analytics]"],
                " and ",
                code!["[verification]"],
                "; the framework writes them with the right CSP and consent handling.",
            ],
        ],
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        p!["Run these from the project folder after editing the file:"],
        pre![code![
            class("language-sh"),
            r"next-rust doctor            # reads the config and reports the file it found, plus toolchain, port and .env checks
next-rust check             # reads the config, validates routes and special files, then type-checks the project
next-rust routes --layouts  # shows every URL and whether it is static or dynamic",
        ]],
        p![
            "A config problem is printed with its code (",
            code!["NR0001"],
            " to ",
            code!["NR0011"],
            ", see ",
            a![href("/docs/diagnostics"), "diagnostics"],
            ") or, for a typo, with the file, line and key. ",
            code!["next-rust dev"],
            " watches the config file and rebuilds when it changes.",
        ],
    ]
}
