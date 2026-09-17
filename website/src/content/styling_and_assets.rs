//! The "styling-and-assets" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        h2![id("global-css"), a![class("anchor"), href("#global-css"), "Global CSS"]],
        pre![code![
            class("language-rust"),
            r#"// app/layout.rs
pub fn Layout(children: Children) -> impl View {
    div![global_css!("globals.css"), children]
}"#,
        ],],
        p![
            "The path is relative to the source file. At ",
            strong!["compile time"],
            " the stylesheet is read and minified, and a change to the CSS file triggers a rebuild.",
        ],
        h2![id("css-modules"), a![class("anchor"), href("#css-modules"), "CSS modules"]],
        pre![code![
            class("language-css"),
            r"/* app/components/card.module.css */
.card { border-radius: 8px; padding: 1rem; }
.card-title { font-weight: 600; }
.card:hover .card-title { text-decoration: underline; }
:global(.dark) .card { background: #111; }",
        ],],
        pre![code![
            class("language-rust"),
            r#"let styles = css_module!("card.module.css");
article![class(styles.card), h2![class(styles.card_title), "Hello"]]"#,
        ],],
        ul![
            li![
                "Class selectors are renamed to ",
                code!["card_1a2b3c4d"],
                ", a deterministic hash of the module path and class name, so styles never leak between modules.",
            ],
            li![
                "Each class is a typed field (",
                code!["card-title"],
                " → ",
                code!["card_title"],
                "). A misspelled class is a ",
                strong!["compile error"],
                ".",
            ],
            li![
                "Selectors inside ",
                code!["@media"],
                ", ",
                code!["@supports"],
                ", ",
                code!["@layer"],
                " and ",
                code!["@container"],
                " are scoped. ",
                code!["@keyframes"],
                " names, ",
                code!["url(..)"],
                " and strings are left untouched. ",
                code![":global(..)"],
                " opts out.",
            ],
            li!["CSS nesting (", code![".a { .b { } }"], ", ", code!["&:hover"], ") is supported."],
        ],
        h3![
            id("how-stylesheets-reach-the-page"),
            a![class("anchor"), href("#how-stylesheets-reach-the-page"), "How stylesheets reach the page"],
        ],
        p![
            "Stylesheets are part of the view tree. A page includes a module's CSS only if it renders a class from that module, or includes a ",
            code!["global_css!"],
            " stylesheet. Each stylesheet is emitted ",
            strong!["once per document"],
            " as ",
            code!["<style data-nr-css=\"<hash>\">"],
            ", hoisted into ",
            code!["<head>"],
            ". Stylesheets first used inside streamed content arrive with that chunk, so there's no flash of unstyled content. Client navigation adds new stylesheets and skips ones already present.",
        ],
        p![
            "Trade-off: inlining avoids extra requests and render-blocking fetches, but the CSS isn't cached separately from the HTML. For large design systems, put a stylesheet in ",
            code!["public/"],
            " and reference it with ",
            code!["Metadata::new().stylesheet(\"/styles.css\")"],
            ".",
        ],
        h2![id("public"), a![class("anchor"), href("#public"), code!["public/"]]],
        p!["Everything in ", code!["public/"], " is served from the site root:"],
        pre![code![
            class("language-text"),
            r"public/favicon.ico      → /favicon.ico
public/images/logo.png  → /images/logo.png
public/robots.txt       → /robots.txt",
        ],],
        ul![
            li!["Routes take precedence over files with the same path."],
            li![
                code!["ETag"],
                " and ",
                code!["Last-Modified"],
                " are sent, with ",
                code!["304"],
                " responses for conditional requests. Single-range requests (",
                code!["Range: bytes=…"],
                ") get ",
                code!["206"],
                ".",
            ],
            li!["Large files are streamed in 64 KB chunks."],
            li![
                "Icons set with ",
                code!["Metadata::icon"],
                " and ",
                code!["apple_touch_icon"],
                " that point into ",
                code!["public/"],
                " get a content version: ",
                code!["/logo.svg"],
                " is written as ",
                code!["/logo.svg?v=36f603f193"],
                " and served with ",
                code!["Cache-Control: public, max-age=31536000, immutable"],
                ". Browsers check the favicon again on their own (Chrome may do it when the URL changes); those checks are answered from the browser cache without a request. Changing the file changes the version.",
            ],
            li![
                code!["Cache-Control: public, max-age=<assets.public_max_age>"],
                ", which defaults to 0 because public URLs aren't fingerprinted.",
            ],
            li![
                "Hidden files (",
                code![".env"],
                ", ",
                code![".git"],
                "), ",
                code![".."],
                " segments, encoded traversal (",
                code!["%2e%2e"],
                "), backslashes, NUL bytes, and symbolic links pointing outside ",
                code!["public/"],
                " are all refused.",
            ],
        ],
        h2![
            id("fingerprinted-assets-asset"),
            a![class("anchor"), href("#fingerprinted-assets-asset"), "Fingerprinted assets: ", code!["asset!"],],
        ],
        p!["For long-term caching, put files in ", code!["assets/"], " and reference them with ", code!["asset!"], ":",],
        pre![code![
            class("language-rust"),
            r#"link![rel("preload"), href(asset!("fonts/inter.woff2")), as_("font")]
img![src(asset!("images/logo.svg")), alt("Acme")]"#,
        ],],
        p![
            code!["asset!"],
            " runs at compile time. It hashes the file content and returns ",
            code!["/_nr/assets/fonts/inter.<16-hex-hash>.woff2"],
            ". That URL is served with ",
            code!["Cache-Control: public, max-age=31536000, immutable"],
            ", and the URL changes whenever the content does. Editing the file triggers a rebuild.",
        ],
        h2![id("images"), a![class("anchor"), href("#images"), "Images"]],
        pre![code![
            class("language-rust"),
            r#"Image!(src = "/photos/lake.jpg", width = 1200, height = 800, alt = "A lake")"#,
        ],],
        p![
            "Generated markup: ",
            code!["srcset"],
            " with widths up to the declared width, ",
            code!["sizes"],
            ", ",
            code!["loading=\"lazy\""],
            " (or ",
            code!["fetchpriority=\"high\""],
            " with ",
            code!["priority = true"],
            "), ",
            code!["decoding=\"async\""],
            ", and width and height attributes.",
        ],
        p![
            "Local images go through ",
            code!["/_nr/image?url=…&w=…&q=…"],
            ", which only accepts local paths under ",
            code!["public/"],
            " (never remote URLs, so it can't be used for SSRF). It validates the width and serves the image with ",
            code!["Cache-Control: public, max-age=<images.max_age>"],
            ".",
        ],
        p![
            strong!["Status:"],
            " resizing and format conversion (WebP/AVIF) aren't implemented yet. The endpoint currently serves the original file (",
            code!["x-nr-image: original"],
            "). Because the markup and URLs are already final, adding an optimizer won't require changing your code. Until then, pre-size large images, or use ",
            code!["unoptimized = true"],
            " with an image CDN.",
        ],
        h2![id("fonts"), a![class("anchor"), href("#fonts"), "Fonts"]],
        pre![code![
            class("language-rust"),
            r#"use std::sync::LazyLock;

static INTER: LazyLock<LocalFont> = LazyLock::new(|| {
    LocalFont::new("Inter", asset!("fonts/inter-var.woff2")).weight("100 900").display("swap")
});

pub fn metadata() -> Metadata { INTER.metadata() }            // <link rel=preload as=font crossorigin>

pub fn Layout(children: Children) -> impl View {
    div![INTER.style_node(), style(format!("font-family: {}", INTER.family())), children]
}"#,
        ],],
        p![
            code!["LocalFont"],
            " sanitizes names and URLs placed in the generated ",
            code!["@font-face"],
            " rule. Font files served through ",
            code!["asset!"],
            " get immutable caching.",
        ],
        h2![id("compression"), a![class("anchor"), href("#compression"), "Compression"]],
        p![
            "Buffered responses larger than 1 KB, and all streamed responses, with compressible types (HTML, CSS, JS, JSON, SVG, WASM) are gzip-compressed when the client accepts it. Streamed HTML is compressed chunk by chunk with sync flushes, so it still arrives progressively. Disable with ",
            code!["[server] compression = false"],
            ", for example when a proxy compresses.",
        ],
    ]
}
