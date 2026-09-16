# Styling & assets

## Global CSS

```rust
// app/layout.rs
pub fn Layout(children: Children) -> impl View {
    div![global_css!("globals.css"), children]
}
```

The path is relative to the source file. At **compile time** the stylesheet
is read and minified, and a change to the CSS file triggers a rebuild.

## CSS modules

```css
/* app/components/card.module.css */
.card { border-radius: 8px; padding: 1rem; }
.card-title { font-weight: 600; }
.card:hover .card-title { text-decoration: underline; }
:global(.dark) .card { background: #111; }
```

```rust
let styles = css_module!("card.module.css");
article![class(styles.card), h2![class(styles.card_title), "Hello"]]
```

- Class selectors are renamed to `card_1a2b3c4d`, a deterministic hash of the
  module path and class name, so styles never leak between modules.
- Each class is a typed field (`card-title` → `card_title`). A misspelled
  class is a **compile error**.
- Selectors inside `@media`, `@supports`, `@layer` and `@container` are
  scoped. `@keyframes` names, `url(..)` and strings are left untouched.
  `:global(..)` opts out.
- CSS nesting (`.a { .b { } }`, `&:hover`) is supported.

### How stylesheets reach the page

Stylesheets are part of the view tree. A page includes a module's CSS only if
it renders a class from that module, or includes a `global_css!` stylesheet.
Each stylesheet is emitted **once per document** as
`<style data-nr-css="<hash>">`, hoisted into `<head>`. Stylesheets first used
inside streamed content arrive with that chunk, so there's no flash of
unstyled content. Client navigation adds new stylesheets and skips ones
already present.

Trade-off: inlining avoids extra requests and render-blocking fetches, but
the CSS isn't cached separately from the HTML. For large design systems, put
a stylesheet in `public/` and reference it with
`Metadata::new().stylesheet("/styles.css")`.

## `public/`

Everything in `public/` is served from the site root:

```text
public/favicon.ico      → /favicon.ico
public/images/logo.png  → /images/logo.png
public/robots.txt       → /robots.txt
```

- Routes take precedence over files with the same path.
- `ETag` and `Last-Modified` are sent, with `304` responses for conditional
  requests. Single-range requests (`Range: bytes=…`) get `206`.
- Large files are streamed in 64 KB chunks.
- `Cache-Control: public, max-age=<assets.public_max_age>`, which defaults to 0
  because public URLs aren't fingerprinted.
- Hidden files (`.env`, `.git`), `..` segments, encoded traversal
  (`%2e%2e`), backslashes, NUL bytes, and symbolic links pointing outside
  `public/` are all refused.

## Fingerprinted assets: `asset!`

For long-term caching, put files in `assets/` and reference them with
`asset!`:

```rust
link![rel("preload"), href(asset!("fonts/inter.woff2")), as_("font")]
img![src(asset!("images/logo.svg")), alt("Acme")]
```

`asset!` runs at compile time. It hashes the file content and returns
`/_nr/assets/fonts/inter.<16-hex-hash>.woff2`. That URL is served with
`Cache-Control: public, max-age=31536000, immutable`, and the URL changes
whenever the content does. Editing the file triggers a rebuild.

## Images

```rust
Image!(src = "/photos/lake.jpg", width = 1200, height = 800, alt = "A lake")
```

Generated markup: `srcset` with widths up to the declared width, `sizes`,
`loading="lazy"` (or `fetchpriority="high"` with `priority = true`),
`decoding="async"`, and width and height attributes.

Local images go through `/_nr/image?url=…&w=…&q=…`, which only accepts local
paths under `public/` (never remote URLs, so it can't be used for SSRF). It
validates the width and serves the image with
`Cache-Control: public, max-age=<images.max_age>`.

**Status:** resizing and format conversion (WebP/AVIF) aren't implemented
yet. The endpoint currently serves the original file (`x-nr-image: original`).
Because the markup and URLs are already final, adding an optimizer won't
require changing your code. Until then, pre-size large images, or use
`unoptimized = true` with an image CDN.

## Fonts

```rust
use std::sync::LazyLock;

static INTER: LazyLock<LocalFont> = LazyLock::new(|| {
    LocalFont::new("Inter", asset!("fonts/inter-var.woff2")).weight("100 900").display("swap")
});

pub fn metadata() -> Metadata { INTER.metadata() }            // <link rel=preload as=font crossorigin>

pub fn Layout(children: Children) -> impl View {
    div![INTER.style_node(), style(format!("font-family: {}", INTER.family())), children]
}
```

`LocalFont` sanitizes names and URLs placed in the generated `@font-face`
rule. Font files served through `asset!` get immutable caching.

## Compression

Buffered responses larger than 1 KB, and all streamed responses, with compressible types (HTML, CSS, JS, JSON, SVG,
WASM) are gzip-compressed when the client accepts it. Streamed HTML is
compressed chunk by chunk with sync flushes, so it still arrives
progressively. Disable with `[server] compression = false`, for example when
a proxy compresses.
