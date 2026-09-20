//! Release builds write HTML with short names and compact markup
//! (`[build] minify_names`, `minify_html`): the last step of a response,
//! whatever produced it.

use bytes::Bytes;
use futures_util::StreamExt;
use next_rust_cache::MemoryStore;
use next_rust_core::{Config, Environment};
use next_rust_server::markup::{Finisher, Markup, NameTable};
use next_rust_server::response::Body;
use next_rust_server::*;
use next_rust_view::*;

type R = BoxFuture<Result<Node>>;

/// What a build would hand the app: sorted by key.
static NAMES: NameTable = &[
    ("#auth-*", "w-"),
    ("--brand", "--a"),
    ("@k:fade", "c"),
    ("@l:components", "d"),
    ("data-component", ""),
    ("data-open", "data-k"),
    ("data-strength-for", "data-m"),
];

static SHEET: Stylesheet = Stylesheet {
    id: "sheet-1",
    css: "@layer components{[data-open]{color:var(--brand);animation:fade 1s}}@keyframes fade{to{opacity:1}}#auth-email{top:0}",
    per_class: None,
    scripts: &[],
};

fn form(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(div![
            &SHEET,
            data("open", ""),
            style("--brand:#f00"),
            label![r#for("auth-email"), "Email"],
            input![
                id("auth-email"),
                name("email"),
                r#type("email"),
                value(""),
                aria("describedby", "auth-email-error")
            ],
            p![id("auth-email-error"), data("nr-error", "email")],
            div![data("strength-for", "auth-email")],
            ul![li!["One"], li![a![href("#auth-email"), "Two"]]],
            island("Chart", "{\"tries\":3,\"label\":\"a b\"}".into(), fragment![]),
            div![data("note", "{\"it's\":1}")],
            pre!["  spaced   text\n"],
            script!["document.querySelector('[data-open]')"],
        ]
        .into_node())
    })
}

fn slow(_ctx: Ctx) -> R {
    Box::pin(async {
        Ok(div![
            h1!["Shell"],
            suspense(p!["Loading"], async {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                ul![li![data("open", ""), "Late"], li!["Later"]]
            }),
        ]
        .into_node())
    })
}

fn json(_req: Request) -> BoxFuture<Response> {
    Box::pin(async { Response::json(&serde_json::json!({ "html": "<p data-open=\"\">x</p>" })) })
}

fn page(pattern: &'static str, body: fn(Ctx) -> R) -> PageDef {
    PageDef {
        pattern,
        source: "test",
        body: PageBody::Rust(body),
        segments: vec![SegmentDef::default()],
        metadata: None,
        rendering: Rendering::Dynamic,
        revalidate: None,
        generate_params: None,
        dynamic_params: true,
        tags: &[],
        intercept_from: None,
    }
}

fn build(names: NameTable, markup: Markup) -> App {
    let routes = Routes {
        pages: vec![page("/form", form), page("/slow", slow)],
        apis: vec![ApiDef {
            pattern: "/api/data",
            source: "test",
            handlers: vec![(http::Method::GET, json)],
            middleware: vec![],
        }],
        names,
        markup,
        ..Default::default()
    };
    App::new(routes)
        .config(Config { root: std::env::temp_dir().join("nr-markup-test-root"), ..Config::default() })
        .environment(Environment::Production)
        .page_store(MemoryStore::new(10))
        .build()
        .unwrap()
}

async fn get(app: &App, uri: &str) -> Response {
    app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await
}

#[tokio::test]
async fn pages_leave_with_short_names() {
    let app = build(NAMES, Markup::default());
    let html = get(&app, "/form").await.into_text().await;
    // Attributes, ids and what refers to them.
    assert!(html.contains("<div data-k=\"\" style=\"--a:#f00\">"), "{html}");
    assert!(
        html.contains("<label for=\"w-email\">") && html.contains("<input id=\"w-email\" name=\"email\""),
        "{html}"
    );
    assert!(html.contains("aria-describedby=\"w-email-error\"") && html.contains("<p id=\"w-email-error\""), "{html}");
    assert!(html.contains("<div data-m=\"w-email\">") && html.contains("<a href=\"#w-email\""), "{html}");
    // The stylesheet of the page.
    assert!(
        html.contains("@layer d{[data-k]{color:var(--a);animation:c 1s}}@keyframes c{to{opacity:1}}#w-email{top:0}"),
        "{html}"
    );
    // Field names, the framework's own attributes, text and scripts stay.
    assert!(html.contains("name=\"email\"") && html.contains("data-nr-error=\"email\""), "{html}");
    assert!(html.contains("<pre>  spaced   text\n</pre>"), "{html}");
    assert!(html.contains("<script>document.querySelector('[data-open]')</script>"), "{html}");
    assert!(!html.contains("content-length"), "{html}");
}

#[tokio::test]
async fn compact_markup_is_the_same_document() {
    let app = build(&[], Markup { compact: true, svg_precision: 3 });
    let html = get(&app, "/form").await.into_text().await;
    assert!(html.contains("<label for=auth-email>Email</label><input id=auth-email name=email type=email value aria-describedby=auth-email-error>"), "{html}");
    assert!(html.contains("<ul><li>One<li><a href=#auth-email>Two</a></ul>"), "{html}");
    assert!(html.contains("<div data-open style=--brand:#f00>"), "{html}");
    assert!(html.contains("<pre>  spaced   text\n</pre>"), "{html}");
}

/// Island props are JSON in an attribute, where every `"` costs six bytes
/// as `&quot;`. A single-quoted value carries the quotes as themselves, and
/// a parser reads back the same string.
#[tokio::test]
async fn island_props_lose_their_quote_entities() {
    let app = build(&[], Markup { compact: true, svg_precision: 3 });
    let html = get(&app, "/form").await.into_text().await;
    assert!(html.contains(r#"<nr-island data-component=Chart data-props='{"tries":3,"label":"a b"}'>"#), "{html}");
    assert_eq!(html.matches("&quot;").count(), 2, "only the value that cannot lose them keeps them: {html}");
    // A value that also holds an apostrophe keeps its entities: single
    // quotes would end it early.
    assert!(html.contains(r#"data-note={&quot;it&#39;s&quot;:1}"#), "{html}");
}

#[tokio::test]
async fn development_builds_are_left_alone() {
    let app = build(&[], Markup::default());
    let html = get(&app, "/form").await.into_text().await;
    assert!(html.contains("<div data-open=\"\" style=\"--brand:#f00\">") && html.contains("</li></ul>"), "{html}");
}

#[tokio::test]
async fn streamed_pages_are_rewritten_chunk_by_chunk() {
    let app = build(NAMES, Markup { compact: true, svg_precision: 3 });
    let res = get(&app, "/slow").await;
    let Body::Stream(mut stream) = res.body else { panic!("expected a stream") };
    let first = String::from_utf8(stream.next().await.unwrap().unwrap().to_vec()).unwrap();
    // The shell goes out at once, boundary and all.
    assert!(first.contains("<h1>Shell</h1><nr-b id=nr-b1><p>Loading</p></nr-b>"), "{first}");
    let mut rest = String::new();
    while let Some(chunk) = stream.next().await {
        rest.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
    }
    assert!(rest.contains("<template id=nr-t1><ul><li data-k>Late<li>Later</ul></template>"), "{rest}");
    assert!(rest.contains("$nr(1)</script>") && rest.ends_with("</body></html>"), "{rest}");
}

#[tokio::test]
async fn only_html_is_rewritten() {
    let app = build(NAMES, Markup { compact: true, svg_precision: 3 });
    let res = get(&app, "/api/data").await;
    assert_eq!(res.into_text().await, r#"{"html":"<p data-open=\"\">x</p>"}"#);
}

#[test]
fn html_sent_another_way_can_be_finished_by_hand() {
    let finisher = Finisher::new(NAMES, Markup::default()).unwrap();
    assert_eq!(finisher.finish("<p data-open=\"\" id=\"auth-x\">x</p>"), "<p data-k=\"\" id=\"w-x\">x</p>");
}
