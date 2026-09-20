//! A page is sent only the CSS it can use: rules for the elements, ids and
//! attributes it renders, the custom properties something reads, the fonts
//! and keyframes something names. Client navigations, which render only the
//! region that changes, still get what the new page needs.

use bytes::Bytes;
use next_rust_core::{Config, Environment};
use next_rust_server::app::LayoutFn;
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// Stands in for the generated Tailwind CSS with a project stylesheet
/// imported into it (minified, as the build writes it).
static TW: Stylesheet = Stylesheet {
    id: "tw-page",
    css: "@layer theme,base,utilities;\
@layer theme{:root{--color-primary:var(--primary);--color-unused:var(--chart);--spacing:.25rem;--font-mono:var(--mono);--animate-float:float 6s infinite}}\
@layer base{*{margin:0}table{border-collapse:collapse}code,pre{font-family:var(--font-mono)}input[type=checkbox]{width:1rem}details>summary{cursor:pointer}}\
@layer utilities{.p-4{padding:calc(var(--spacing)*4)}.text-primary{color:var(--color-primary)}.float-slow{animation:var(--animate-float)}.left{float:left}}\
@font-face{font-family:Mono;src:url(/mono.woff2)}@font-face{font-family:Sans;src:url(/sans.woff2)}\
:root{--primary:navy;--chart:teal;--mono:Mono,monospace;--sans:Sans,sans-serif;--bar:0}.dark{--primary:white;--chart:aqua}\
[data-theme=\"noir\"]{--primary:black}[data-theme=\"rose\"]{--primary:pink}body{font-family:var(--sans)}\
.prose table{width:100%}.bar{width:var(--bar)}[data-open]{display:block}\
:root{--kit-fg:navy;--kit-bg:white;--kit-unread:red}\
@keyframes float{to{translate:0 -4px}}@keyframes unused{to{opacity:0}}",
    per_class: Some(&["dark"]),
    // `assets/menu.js` sets `data-open` and `--bar`, and creates a
    // <details> with its <summary>.
    scripts: &[("assets/menu.js", &["data-open", "--bar", "<details", "<summary"])],
};
/// A second per-page sheet (the UI components' works like this): it reads
/// `--kit-fg`, which only the sheet above defines.
static KIT: Stylesheet = Stylesheet {
    id: "kit",
    css: ".kit-card{color:var(--kit-fg);background:var(--kit-bg)}",
    per_class: Some(&[]),
    scripts: &[],
};
static SHEETS: &[&Stylesheet] = &[&TW, &KIT];

fn root_layout(_ctx: Ctx, children: Children, _slots: Slots) -> R {
    Box::pin(async move { Ok(main![class("prose"), attr("data-theme", "rose"), children].into_node()) })
}
fn page_home(_ctx: Ctx) -> R {
    Box::pin(async { Ok(h1![class("p-4"), "Home"].into_node()) })
}
fn page_table(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(fragment![
            table![class("text-primary float-slow"), tr![td![code!["x"]]]],
            p![style("animation: unused 1s"), "inline"]
        ])
    })
}
fn page_script(_ctx: Ctx) -> R {
    Box::pin(async { Ok(fragment![p!["menu"], script![src("/_next-rust/assets/menu.js")]]) })
}
fn page_kit(_ctx: Ctx) -> R {
    Box::pin(async { Ok(fragment![&KIT, div![class("kit-card"), "card"]]) })
}
fn page_inline(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(fragment![
            p![class("left text-primary"), "theme switcher"],
            script![raw_html("document.documentElement.dataset.theme = localStorage.theme")]
        ])
    })
}

fn page(pattern: &'static str, body: app::RenderFn) -> PageDef {
    let layout: LayoutFn = root_layout;
    PageDef {
        pattern,
        source: "test",
        body: PageBody::Rust(body),
        segments: vec![SegmentDef {
            layout: Some(layout),
            layout_id: "app/layout.rs",
            layout_reusable: true,
            ..Default::default()
        }],
        metadata: None,
        rendering: Rendering::Dynamic,
        revalidate: None,
        generate_params: None,
        dynamic_params: true,
        tags: &[],
        intercept_from: None,
    }
}

fn app() -> App {
    let routes = Routes {
        pages: vec![
            page("/", page_home),
            page("/table", page_table),
            page("/script", page_script),
            page("/inline", page_inline),
            page("/kit", page_kit),
        ],
        stylesheets: SHEETS,
        ..Default::default()
    };
    let config = Config { root: std::env::temp_dir(), ..Config::default() };
    App::new(routes).config(config).environment(Environment::Test).build().unwrap()
}

/// The response and the CSS of its `tw-page` `<style>` elements, with their ids.
async fn get(app: &App, path: &str, nav: Option<(&str, &[String])>) -> (String, String, Vec<String>) {
    let mut req = http::Request::get(path);
    if let Some((layouts, styles)) = nav {
        req = req.header("x-nr-nav", "1").header("x-nr-layouts", layouts).header("x-nr-styles", styles.join(","));
    }
    let res = app.handle(Request::from_http(req.body(Bytes::new()).unwrap())).await;
    assert_eq!(res.status, 200);
    let html = res.into_text().await;
    let (mut css, mut ids) = (String::new(), Vec::new());
    for rest in html.split("<style data-nr-css=\"").skip(1) {
        let (id, rest) = rest.split_once("\">").unwrap();
        if id.starts_with("tw-page~") {
            ids.push(id.to_owned());
            css.push_str(rest.split("</style>").next().unwrap());
        }
    }
    (html, css, ids)
}

#[tokio::test]
async fn a_page_gets_the_css_it_can_use_and_nothing_else() {
    let app = app();
    let (_, css, _) = get(&app, "/", None).await;
    assert_eq!(
        css,
        "@layer theme,base,utilities;@layer theme{:root{--spacing:.25rem}}@layer base{*{margin:0}}\
@layer utilities{.p-4{padding:calc(var(--spacing)*4)}}@font-face{font-family:Sans;src:url(/sans.woff2)}\
:root{--sans:Sans,sans-serif}body{font-family:var(--sans)}:root{--kit-fg:navy;--kit-bg:white}",
        "no table, code, checkbox or <details> rule; no theme block but the page's; one font; no keyframes. \
`--kit-fg`/`--kit-bg` stay because the document's other stylesheet reads them; `--kit-unread`, which no sheet \
reads, does not"
    );

    let (_, css, _) = get(&app, "/table", None).await;
    for sent in [
        "table{border-collapse:collapse}",
        "code,pre{font-family:var(--font-mono)}",
        "--font-mono:var(--mono)",
        "--mono:Mono,monospace",
        "@font-face{font-family:Mono;",
        "--color-primary:var(--primary)",
        ":root{--primary:navy",
        ".dark{--primary:white}",
        "[data-theme=\"rose\"]{--primary:pink}",
        ".prose table{width:100%}",
        "--animate-float:float 6s infinite",
        "@keyframes float{",
        "@keyframes unused{",
    ] {
        assert!(css.contains(sent), "{sent} missing from {css}");
    }
    for left_out in ["noir", "--chart", "--color-unused", "checkbox", "summary", "--bar", "[data-open]", ".left"] {
        assert!(!css.contains(left_out), "{left_out} in {css}");
    }
}

#[tokio::test]
async fn scripts_bring_the_names_they_set() {
    let app = app();
    let (_, css, _) = get(&app, "/script", None).await;
    assert!(css.contains("[data-open]{display:block}") && css.contains("details>summary"), "{css}");
    assert!(css.contains("--bar:0"), "a custom property the script sets: {css}");
    assert!(!css.contains("noir"), "{css}");
    // An inline script that switches themes keeps every theme block.
    let (_, css, _) = get(&app, "/inline", None).await;
    assert!(css.contains("noir") && css.contains("pink") && css.contains(".left{float:left}"), "{css}");
    assert!(!css.contains("@keyframes float"), "`float:left` is not the animation: {css}");
}

#[tokio::test]
async fn a_client_navigation_gets_what_the_new_page_needs() {
    let app = app();
    let (home, _, ids) = get(&app, "/", None).await;
    let layouts = next_rust_server::testing::layout_keys(&home).join(",");
    // Only the region inside the layout is rendered: `.prose` and
    // `data-theme="rose"` stay on screen, outside of it.
    let (html, css, more) = get(&app, "/table", Some((&layouts, &ids))).await;
    assert!(!html.contains("class=\"prose\""), "a partial response: {html}");
    assert!(css.contains(".prose table{width:100%}"), "the owner class is outside the region: {css}");
    assert!(css.contains("[data-theme=\"rose\"]{--primary:pink}") && !css.contains("noir"), "{css}");
    assert!(css.contains(":root{--primary:navy") && css.contains(".dark{--primary:white}"), "{css}");
    assert!(css.contains("@font-face{font-family:Mono;") && css.contains("@keyframes float{"), "{css}");
    assert!(!css.contains("*{margin:0}") && !css.contains("family:Sans"), "nothing twice: {css}");
    // Back on a page whose CSS the browser has: nothing is sent.
    let known: Vec<String> = ids.into_iter().chain(more).collect();
    let (_, css, _) = get(&app, "/", Some((&layouts, &known))).await;
    assert_eq!(css, "");
    let (_, css, _) = get(&app, "/table", Some((&layouts, &known))).await;
    assert_eq!(css, "");
}

#[tokio::test]
async fn a_variable_another_stylesheet_reads_is_kept() {
    let app = app();
    // `.kit-card` is in the second sheet; `--kit-fg`/`--kit-bg` are defined
    // in the first one, which knows nothing about that rule.
    let (html, css, _) = get(&app, "/kit", None).await;
    assert!(css.contains("--kit-fg:navy") && css.contains("--kit-bg:white"), "{css}");
    assert!(!css.contains("--kit-unread"), "a variable no sheet reads is still left out: {css}");
    assert!(html.contains(".kit-card{color:var(--kit-fg)"), "{html}");
}

static UI_SHEETS: &[&Stylesheet] = &[&next_rust_ui::UI_CSS];

/// The UI components' own stylesheet: its system-dark block sits behind
/// `@media (prefers-color-scheme: dark)` and `:root:not(.light)`, so nothing
/// in the markup can match it. A page that renders a component must still
/// get it, or the kit stops following the system setting.
#[tokio::test]
async fn the_ui_kit_keeps_its_system_dark_block() {
    let css = next_rust_ui::UI_CSS.css;
    assert!(css.contains("prefers-color-scheme"), "the kit's sheet has a system-dark block to keep");
    let routes = Routes { pages: vec![page("/", page_kit)], stylesheets: UI_SHEETS, ..Default::default() };
    let config = Config { root: std::env::temp_dir(), ..Config::default() };
    let app = App::new(routes).config(config).environment(Environment::Test).build().unwrap();
    let res = app.handle(Request::from_http(http::Request::get("/").body(Bytes::new()).unwrap())).await;
    let html = res.into_text().await;
    assert!(html.contains("prefers-color-scheme"), "the system-dark block was dropped: {html}");
}
