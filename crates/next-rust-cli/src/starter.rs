//! The starter site created by `next-rust new`: one centered hero with the
//! logo, a single line of text, a GitHub star button and a credit, plus a
//! matching 404 page. It is deliberately small, since it's meant to be replaced.

/// `app/layout.rs` (the app name is substituted)
pub const LAYOUT_RS: &str = r####"use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new()
        .title("__APP_NAME__")
        .title_template("%s · __APP_NAME__")
        .description("Built with Next Rust.")
        .icon("/favicon.svg")
        .theme_color("#0e0f12")
}

pub fn Layout(children: Children) -> impl View {
    fragment![global_css!("globals.css"), children]
}
"####;

/// `app/page.rs`
pub const PAGE_RS: &str = r####"use next_rust::prelude::*;

const STAR: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="M12 2.8l2.8 5.7 6.3.9-4.55 4.43 1.07 6.27L12 17.13 6.38 20.1l1.07-6.27L2.9 9.4l6.3-.9z"/></svg>"#;

pub fn Page() -> impl View {
    main![
        class("hero"),
        img![class("logo"), src("/favicon.svg"), alt("Next Rust logo"), width(96), height(96)],
        h1!["Next Rust"],
        p![class("tagline"), "Folders become routes, Rust becomes HTML. Start by editing ", code!["app/page.rs"], "."],
        a![
            class("star"),
            href("https://github.com/iplustsolution/next-rust"),
            target("_blank"),
            rel("noopener"),
            raw_html(STAR),
            "Star on GitHub",
        ],
        p![
            class("credit"),
            "Created by ",
            a![href("https://www.iplust.in/"), target("_blank"), rel("noopener"), "I Plus T Solution"],
        ],
    ]
}
"####;

/// `app/not-found.rs`
pub const NOT_FOUND_RS: &str = r####"use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    main![
        class("hero"),
        img![class("logo small"), src("/favicon.svg"), alt(""), width(64), height(64)],
        h1!["404"],
        p![class("tagline"), "There's nothing at this address."],
        Link!(href = "/", class = "star", "Back to home"),
    ]
}
"####;

/// `app/globals.css`
pub const GLOBALS_CSS: &str = r####":root {
  --bg: #0e0f12;
  --fg: #f4f4f5;
  --muted: #a1a1aa;
  --line: #26272d;
  --accent: #f26b2a;
  --accent-ink: #120a05;
  --sans: "IBM Plex Sans", ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  --mono: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  color-scheme: dark;
}

@media (prefers-color-scheme: light) {
  :root {
    --bg: #fafaf9;
    --fg: #18181b;
    --muted: #52525b;
    --line: #e4e4e7;
    color-scheme: light;
  }
}

*, *::before, *::after { box-sizing: border-box; }

body {
  margin: 0;
  min-height: 100vh;
  min-height: 100svh;
  display: grid;
  place-items: center;
  padding: 32px 20px;
  background-color: var(--bg);
  background-image: radial-gradient(var(--line) 1px, transparent 1px);
  background-size: 24px 24px;
  color: var(--fg);
  font-family: var(--sans);
  font-size: 16px;
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}

.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 34rem;
  animation: rise 500ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
}

.logo {
  width: 96px;
  height: 96px;
  border-radius: 24px;
}
.logo.small { width: 64px; height: 64px; border-radius: 16px; }

h1 {
  margin: 28px 0 10px;
  font-size: clamp(2.4rem, 7vw, 3.5rem);
  font-weight: 700;
  line-height: 1.05;
  letter-spacing: -0.035em;
}

.tagline {
  margin: 0;
  color: var(--muted);
  font-size: 1.05rem;
  text-wrap: balance;
}

code {
  font-family: var(--mono);
  font-size: 0.88em;
  color: var(--fg);
  padding: 0.12em 0.4em;
  border: 1px solid var(--line);
  border-radius: 6px;
  white-space: nowrap;
}

.star {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  margin-top: 36px;
  min-height: 46px;
  padding: 0 22px;
  border-radius: 10px;
  background: var(--accent);
  color: var(--accent-ink);
  font-weight: 600;
  text-decoration: none;
  cursor: pointer;
  transition: transform 200ms ease, background-color 200ms ease;
}
.star:hover { background: #ff7f41; transform: translateY(-1px); }
.star:active { transform: translateY(0); }

.credit {
  margin: 56px 0 0;
  font-family: var(--mono);
  font-size: 0.8rem;
  letter-spacing: 0.02em;
  color: var(--muted);
}
.credit a {
  color: var(--fg);
  text-decoration: none;
  border-bottom: 1px solid var(--accent);
  transition: color 200ms ease;
}
.credit a:hover { color: var(--accent); }

a:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
}

@keyframes rise {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: none; }
}

@media (prefers-reduced-motion: reduce) {
  .hero { animation: none; }
  .star { transition: none; }
}
"####;

/// `app/layout.rs` for Tailwind projects: no CSS file at all.
pub const TW_LAYOUT_RS: &str = r####"use next_rust::prelude::*;

pub fn metadata() -> Metadata {
    Metadata::new()
        .title("__APP_NAME__")
        .title_template("%s · __APP_NAME__")
        .description("Built with Next Rust.")
        .icon("/favicon.svg")
        .theme_color("#0e0f12")
}

// Styled with Tailwind CSS. The theme (colors, the `bg-dots` and `btn-brand`
// utilities, the `rise` animation) is set in next-rust.toml under [tailwind].
pub fn Layout(children: Children) -> impl View {
    div![
        class("grid min-h-svh place-items-center bg-stone-50 bg-dots px-5 py-8 font-sans text-zinc-900 antialiased"),
        class("dark:bg-night dark:text-zinc-100 dark:[--dot:var(--color-zinc-800)]"),
        children,
    ]
}
"####;

/// `app/page.rs` for Tailwind projects
pub const TW_PAGE_RS: &str = r####"use next_rust::prelude::*;

const STAR: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="M12 2.8l2.8 5.7 6.3.9-4.55 4.43 1.07 6.27L12 17.13 6.38 20.1l1.07-6.27L2.9 9.4l6.3-.9z"/></svg>"#;

pub fn Page() -> impl View {
    main![
        class("flex max-w-xl flex-col items-center text-center motion-safe:animate-rise"),
        img![class("size-24 rounded-3xl"), src("/favicon.svg"), alt("Next Rust logo"), width(96), height(96)],
        h1![class("mt-7 mb-2.5 text-[clamp(2.4rem,7vw,3.5rem)] leading-[1.05] font-bold tracking-tighter"), "Next Rust"],
        p![
            class("text-lg text-balance text-zinc-600 dark:text-zinc-400"),
            "Folders become routes, Rust becomes HTML. Start by editing ",
            code![
                class("rounded-md border border-zinc-300 px-1.5 py-0.5 font-mono text-[0.88em] whitespace-nowrap text-zinc-900 dark:border-zinc-800 dark:text-zinc-100"),
                "app/page.rs"
            ],
            ".",
        ],
        a![
            class("btn-brand mt-9"),
            href("https://github.com/iplustsolution/next-rust"),
            target("_blank"),
            rel("noopener"),
            raw_html(STAR),
            "Star on GitHub",
        ],
        p![
            class("mt-14 font-mono text-xs tracking-wide text-zinc-500"),
            "Created by ",
            a![
                class("border-b border-brand text-zinc-900 transition-colors hover:text-brand dark:text-zinc-100"),
                href("https://www.iplust.in/"),
                target("_blank"),
                rel("noopener"),
                "I Plus T Solution",
            ],
        ],
    ]
}
"####;

/// `app/not-found.rs` for Tailwind projects
pub const TW_NOT_FOUND_RS: &str = r####"use next_rust::prelude::*;

pub fn NotFound() -> impl View {
    main![
        class("flex flex-col items-center text-center motion-safe:animate-rise"),
        img![class("size-16 rounded-2xl"), src("/favicon.svg"), alt(""), width(64), height(64)],
        h1![class("mt-6 mb-2 text-6xl font-bold tracking-tighter"), "404"],
        p![class("text-zinc-600 dark:text-zinc-400"), "There's nothing at this address."],
        Link!(href = "/", class = "btn-brand mt-9", "Back to home"),
    ]
}
"####;

/// The `[tailwind]` section of `next-rust.toml` for Tailwind projects.
pub const TW_CONFIG: &str = r####"
# Tailwind CSS: write classes in your views and the CSS is generated from the
# classes you use. No CSS files, no Node.js. Classes: https://tailwindcss.com/docs
[tailwind]
enabled = true
# dark_mode = "media"       # media (system setting) | class (.dark) | attribute ([data-theme=dark])
# plugins = ["@tailwindcss/typography", "@tailwindcss/forms"]
# safelist = ["bg-red-500"] # classes your code builds from pieces at runtime
# Anything else Tailwind supports in CSS (keyframes, @layer, …):
css = """
@keyframes rise {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: none; }
}
"""

# Theme variables: color-* → bg-*/text-*/border-*, font-*, breakpoint-*, radius-*, animate-*, …
[tailwind.theme]
color-brand = "#f26b2a"
color-brand-hover = "#ff7f41"
color-brand-ink = "#120a05"
color-night = "#0e0f12"
animate-rise = "rise 500ms cubic-bezier(0.2, 0.8, 0.2, 1) both"

# Your own classes: combine Tailwind classes, or write CSS declarations.
[tailwind.utilities]
btn-brand = "inline-flex min-h-11 items-center gap-2.5 rounded-lg bg-brand px-5 font-semibold text-brand-ink transition hover:-translate-y-px hover:bg-brand-hover focus-visible:outline-2 focus-visible:outline-offset-3 focus-visible:outline-brand"
bg-dots = { background-image = "radial-gradient(var(--dot, var(--color-zinc-300)) 1px, transparent 1px)", background-size = "24px 24px" }
"####;

/// `public/favicon.svg (the Next Rust logo)`
pub const FAVICON_SVG: &str = r####"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" role="img" aria-label="Next Rust logo">
  <title>Next Rust</title>
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#e5e5ec"/>
      <stop offset="1" stop-color="#e0e0ef"/>
    </linearGradient>
    <linearGradient id="rust" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ff9c2a"/>
      <stop offset="0.55" stop-color="#f25c1f"/>
      <stop offset="1" stop-color="#b3260c"/>
    </linearGradient>
    <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#f25c1f"/>
      <stop offset="1" stop-color="#f25c1f" stop-opacity="0"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#f25c1f" stop-opacity="0.35"/>
      <stop offset="1" stop-color="#f25c1f" stop-opacity="0"/>
    </radialGradient>
    <clipPath id="inner">
      <circle cx="256" cy="256" r="126"/>
    </clipPath>
  </defs>

  <rect width="512" height="512" rx="112" fill="url(#bg)"/>
  <circle cx="256" cy="256" r="230" fill="url(#glow)"/>

  <!-- gear: the Rust half -->
  <g fill="url(#rust)">
    <g id="tooth"><rect x="238" y="70" width="36" height="72" rx="8"/></g>
    <use href="#tooth" transform="rotate(30 256 256)"/>
    <use href="#tooth" transform="rotate(60 256 256)"/>
    <use href="#tooth" transform="rotate(90 256 256)"/>
    <use href="#tooth" transform="rotate(120 256 256)"/>
    <use href="#tooth" transform="rotate(150 256 256)"/>
    <use href="#tooth" transform="rotate(180 256 256)"/>
    <use href="#tooth" transform="rotate(210 256 256)"/>
    <use href="#tooth" transform="rotate(240 256 256)"/>
    <use href="#tooth" transform="rotate(270 256 256)"/>
    <use href="#tooth" transform="rotate(300 256 256)"/>
    <use href="#tooth" transform="rotate(330 256 256)"/>
  </g>
  <circle cx="256" cy="256" r="146" fill="none" stroke="url(#rust)" stroke-width="40"/>
  <circle cx="256" cy="256" r="126" fill="#f6f6f9"/>

  <!-- the "N": the Next half, its stroke racing out of the gear -->
  <g clip-path="url(#inner)">
    <rect x="190" y="186" width="30" height="140" rx="4" fill="#f25c1f"/>
    <polygon points="190,186 222,186 350,360 318,360" fill="url(#fade)"/>
    <rect x="292" y="186" width="30" height="92" rx="4" fill="url(#fade)"/>
  </g>
</svg>
"####;

pub const MAIN_RS: &str = "next_rust::app!();\n";

pub fn layout_rs(app_name: &str) -> String {
    LAYOUT_RS.replace("__APP_NAME__", app_name)
}

pub fn tw_layout_rs(app_name: &str) -> String {
    TW_LAYOUT_RS.replace("__APP_NAME__", app_name)
}

#[cfg(test)]
mod tests {
    #[test]
    fn tailwind_config_template_parses() {
        let text = format!("{}{}", crate::templates::CONFIG, super::TW_CONFIG);
        let config = next_rust_core::Config::from_toml_str(&text).expect("valid next-rust.toml");
        assert!(config.tailwind.enabled);
        assert_eq!(config.tailwind.theme["color-brand"], "#f26b2a");
        assert!(config.tailwind.utilities.contains_key("btn-brand") && config.tailwind.css.contains("@keyframes rise"));
    }

    /// `website/public/logo.svg` is the source of the logo. The CLI keeps its
    /// own copy (it is published without the website), so check they match.
    #[test]
    fn starter_favicon_matches_the_project_logo() {
        let logo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../website/public/logo.svg");
        if let Ok(source) = std::fs::read_to_string(logo) {
            // Git on Windows may check the file out with CRLF line endings,
            // while string literals in Rust source always use LF.
            let source = source.replace("\r\n", "\n");
            assert_eq!(super::FAVICON_SVG, source, "copy website/public/logo.svg into FAVICON_SVG");
        }
    }
}
