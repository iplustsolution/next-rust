use std::time::Duration;

use futures_util::StreamExt;
use next_rust_view::*;

#[allow(non_snake_case)]
fn Button(label: &str) -> impl View {
    button![class("btn"), label]
}

#[test]
fn concise_syntax() {
    let page = div![
        class("container"),
        h1!["Dashboard"],
        p!["Welcome back."],
        Button("Save"),
        br![],
        img![src("/a.png"), alt("A")],
    ];
    assert_eq!(
        render_static(page),
        r#"<div class="container"><h1>Dashboard</h1><p>Welcome back.</p><button class="btn">Save</button><br><img src="/a.png" alt="A"></div>"#
    );
}

#[test]
fn escaping_by_default() {
    let evil = "<script>alert('x')</script>";
    let html = render_static(div![title(evil), evil]);
    assert_eq!(
        html,
        "<div title=\"&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;\">&lt;script&gt;alert('x')&lt;/script&gt;</div>"
    );
    assert_eq!(render_static(div![raw_html("<b>trusted</b>")]), "<div><b>trusted</b></div>");
}

#[test]
fn dangerous_attributes_are_neutralized() {
    let html = render_static(a![href("javascript:alert(1)"), attr("onclick", "steal()"), "x"]);
    assert_eq!(html, r##"<a href="#">x</a>"##);
    let html = render_static(button![raw_attr("onclick", "go()"), "x"]);
    assert_eq!(html, r#"<button onclick="go()">x</button>"#);
    let html = render_static(div![attr("x\" onmouseover=\"y", "1")]);
    assert_eq!(html, "<div></div>");
    let html = render_static(script!["var s = '</script><script>alert(1)'"]);
    assert_eq!(html, "<script>var s = '<\\/script><script>alert(1)'</script>");
}

#[test]
fn class_merging_and_conditionals() {
    let active = true;
    let html = render_static(div![class("a"), class([("active", active), ("hidden", !active)]), class(None::<&str>)]);
    assert_eq!(html, r#"<div class="a active"></div>"#);
    let html = render_static(input![disabled(false), checked(true), attr_if(false, id("x"))]);
    assert_eq!(html, "<input checked>");
}

#[test]
fn conditional_and_list_rendering() {
    let items = vec!["one", "two"];
    let logged_in = false;
    let html = render_static(div![
        when(logged_in, || p!["Hi"]),
        if items.is_empty() { Some(p!["Empty"]) } else { None },
        ul![each(&items, |i| li![key(*i), *i])],
        items.iter().map(|i| span![*i]).collect::<Vec<_>>(),
        (1, " ", 2.5),
        fragment![b!["x"], i!["y"]],
    ]);
    assert_eq!(
        html,
        r#"<div><ul><li data-nr-key="one">one</li><li data-nr-key="two">two</li></ul><span>one</span><span>two</span>1 2.5<b>x</b><i>y</i></div>"#
    );
}

#[test]
fn links_and_images() {
    let html = render_static(nav![Link!(href = "/", "Home"), Link!(href = "/about", prefetch = false, span!["About"])]);
    assert_eq!(
        html,
        r#"<nav><a href="/" data-nr-link="">Home</a><a href="/about" data-nr-prefetch="false" data-nr-link=""><span>About</span></a></nav>"#
    );
    let html =
        render_static(Image!(src = "https://cdn.example.com/x.png", alt = "remote", width = 10, priority = true));
    assert_eq!(
        html,
        r#"<img src="https://cdn.example.com/x.png" alt="remote" width="10" fetchpriority="high" decoding="async">"#
    );
    let html = render_static(Image!(src = "/hero.jpg", width = 800, height = 400, alt = "Hero"));
    assert!(html.contains("srcset=\"/_next-rust/image?url=%2Fhero.jpg&amp;w=640&amp;q=75 640w, /_next-rust/image?url=%2Fhero.jpg&amp;w=750&amp;q=75 750w, /_next-rust/image?url=%2Fhero.jpg&amp;w=800&amp;q=75 800w\""), "{html}");
}

static SHEET: Stylesheet = Stylesheet { id: "abc", css: ".card_1{color:red}", per_class: None, scripts: &[] };
static GLOBAL: Stylesheet = Stylesheet { id: "glob", css: "body{margin:0}", per_class: None, scripts: &[] };

#[test]
fn stylesheets_are_deduplicated() {
    let card = CssClass::new("card_1", &SHEET);
    let html = render_static(div![&GLOBAL, p![class(card)], p![class(card)]]);
    assert_eq!(
        html,
        r#"<div><style data-nr-css="glob">body{margin:0}</style><style data-nr-css="abc">.card_1{color:red}</style><p class="card_1"></p><p class="card_1"></p></div>"#
    );
}

#[test]
fn layouts_children_and_slots() {
    #[allow(non_snake_case)]
    fn Layout(children: Children, mut slots: Slots) -> impl View {
        div![header!["H"], main![children], aside![slots.take("side")], slots.take("missing")]
    }
    let mut slots = Slots::new();
    slots.insert("side", "S");
    let html = render_static(Layout(Children::new(p!["page"]), slots));
    assert_eq!(html, "<div><header>H</header><main><p>page</p></main><aside>S</aside></div>");
}

async fn slow(ms: u64, label: &'static str) -> impl View {
    futures_timer(ms).await;
    span![label]
}

/// Tiny timer without depending on an async runtime.
async fn futures_timer(ms: u64) {
    let (tx, rx) = futures_channel_like();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        let _ = tx.send(());
    });
    let _ = rx.await;
}

fn futures_channel_like() -> (std::sync::mpsc::Sender<()>, impl std::future::Future<Output = ()>) {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let fut = futures_util::future::poll_fn(move |cx| match rx.try_recv() {
        Ok(()) => std::task::Poll::Ready(()),
        Err(_) => {
            let waker = cx.waker().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(2));
                waker.wake();
            });
            std::task::Poll::Pending
        }
    });
    (tx, fut)
}

#[test]
fn render_to_string_resolves_suspense() {
    let view =
        div![suspense(p!["Loading"], slow(5, "done")), suspense("…", async { suspense("inner…", slow(1, "nested")) })];
    let html = futures_executor::block_on(render_to_string(view));
    assert_eq!(html, "<div><span>done</span><span>nested</span></div>");
    // Synchronous render shows fallbacks.
    assert_eq!(render_static(suspense(p!["Loading"], slow(0, "x"))), "<p>Loading</p>");
}

#[test]
fn streaming_document_sends_shell_first() {
    let body = div![
        h1!["Shell"],
        suspense(p!["Loading slow"], slow(250, "slow")),
        suspense(p!["Loading fast"], async { div![slow(1, "fast").await, suspense("inner…", slow(1, "inner"))] }),
    ];
    let mut parts = DocumentParts::new(body);
    parts.head = Metadata::new().title("T").render_head();
    parts.nonce = Some("n0nce".into());
    parts.tail = Box::new(|flags| format!("<!--streamed={}-->", flags.streamed));
    let chunks: Vec<String> = futures_executor::block_on(stream_document(parts, true).collect());
    assert!(chunks.len() >= 4, "{chunks:#?}");
    let shell = &chunks[0];
    assert!(shell.starts_with("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">"));
    assert!(shell.contains("<title>T</title>"));
    assert!(shell.contains("<script nonce=\"n0nce\">function $nr("));
    assert!(shell.contains(
        "<h1>Shell</h1><nr-b id=\"nr-b1\"><p>Loading slow</p></nr-b><nr-b id=\"nr-b2\"><p>Loading fast</p></nr-b>"
    ));
    // The fast boundary arrives before the slow one.
    assert!(chunks[1].starts_with("<template id=\"nr-t2\"><div><span>fast</span><nr-b id=\"nr-b3\">inner…</nr-b></div></template><script nonce=\"n0nce\">$nr(2)</script>"), "{}", chunks[1]);
    let all = chunks.concat();
    let pos_inner = all.find("<template id=\"nr-t3\">").unwrap();
    let pos_slow = all.find("<template id=\"nr-t1\">").unwrap();
    assert!(pos_inner < pos_slow);
    assert!(all.ends_with("<!--streamed=true--></body></html>"));
}

#[test]
fn non_streaming_document_is_one_chunk() {
    let parts = DocumentParts::new(div![&SHEET, suspense("…", slow(1, "resolved"))]);
    let chunks: Vec<String> = futures_executor::block_on(stream_document(parts, false).collect());
    assert_eq!(chunks.len(), 1);
    assert_eq!(
        chunks[0],
        r#"<!DOCTYPE html><html lang="en"><head><style data-nr-css="abc">.card_1{color:red}</style></head><body><div><span>resolved</span></div></body></html>"#
    );
}

#[test]
fn active_links_match_exactly_or_by_prefix() {
    assert!(link_is_active("/docs", "/docs", false));
    assert!(link_is_active("/docs/", "/docs", false));
    assert!(link_is_active("/docs?x=1#top", "/docs", false));
    assert!(!link_is_active("/docs", "/docs/routing", false));
    assert!(link_is_active("/docs", "/docs/routing", true));
    assert!(!link_is_active("/doc", "/docs", true), "prefix matches whole segments");
    assert!(link_is_active("/", "/anything", true));
    assert!(!link_is_active("https://example.com/docs", "/docs", true));

    let mut node = nav![
        a![href("/docs"), class("link"), active_class("on"), "Docs"],
        a![href("/blog"), active_class("on"), "Blog"],
        a![href("/docs"), active_class_prefix("section"), "Section"],
    ]
    .into_node();
    mark_active_links(&mut node, "/docs");
    let html = render_static(node);
    assert!(html.contains(r#"class="link on" data-nr-active="on" aria-current="page">Docs"#), "{html}");
    assert!(html.contains(r#"<a href="/blog" data-nr-active="on">Blog"#), "{html}");
    assert!(html.contains(r#"class="section" aria-current="page">Section"#), "{html}");
}

#[test]
fn plain_internal_anchors_enable_client_navigation() {
    let flags_for = |view: Node| {
        let parts = DocumentParts { tail: Box::new(|f| format!("[links={}]", f.links)), ..DocumentParts::new(view) };
        futures_executor::block_on(stream_document(parts, false).collect::<Vec<_>>()).concat()
    };
    assert!(flags_for(a![href("/about"), "About"].into_node()).contains("[links=true]"));
    assert!(flags_for(a![href("https://example.com"), "x"].into_node()).contains("[links=false]"));
    assert!(flags_for(a![href("//cdn.example.com/x"), "x"].into_node()).contains("[links=false]"));
    // A form posting to a server action needs the runtime too, or it would
    // submit with a full page load.
    assert!(
        flags_for(form![data("nr-action", "/_next-rust/action/abc"), button!["Send"]].into_node())
            .contains("[links=true]")
    );
    assert!(flags_for(form![action("/elsewhere"), button!["Send"]].into_node()).contains("[links=false]"));
    let html = render_static(a![href("/logout"), reload(true), "Log out"]);
    assert_eq!(html, r#"<a href="/logout" data-nr-reload="">Log out</a>"#);
}

#[test]
fn flags_detect_links_and_islands() {
    let parts = DocumentParts {
        tail: Box::new(|f| format!("{}{}", f.links, f.islands)),
        ..DocumentParts::new(div![Link!(href = "/", "x"), island("Counter", "{}".into(), button!["0"])])
    };
    let html = futures_executor::block_on(stream_document(parts, false).collect::<Vec<_>>()).concat();
    assert!(html.contains(r#"<nr-island data-component="Counter" data-props="{}"><button>0</button></nr-island>"#));
    assert!(html.ends_with("truetrue</body></html>"));
}

#[test]
fn document_html_attributes_are_safe_and_styled() {
    static UTILS: Stylesheet = Stylesheet {
        id: "tw",
        css: "@layer utilities{.dark{color:red}.unused{color:blue}}",
        per_class: Some(&[]),
        scripts: &[],
    };
    let parts = DocumentParts {
        html_attributes: vec![
            ("data-theme".into(), "pass\"point".into()),
            ("class".into(), "dark".into()),
            ("onload".into(), "alert(1)".into()),
            ("bad name".into(), "x".into()),
            ("lang".into(), "de".into()),
        ],
        ..DocumentParts::new(fragment![&UTILS, p!["hi"]])
    };
    let html = futures_executor::block_on(stream_document(parts, false).collect::<Vec<_>>()).concat();
    assert!(
        html.starts_with(r#"<!DOCTYPE html><html lang="de" data-theme="pass&quot;point" class="dark"><head>"#),
        "{html}"
    );
    assert!(!html.contains("onload") && !html.contains("bad name"), "{html}");
    // The class is only on <html>, yet its rule is sent; unused rules are not.
    assert!(html.contains(".dark{color:red}") && !html.contains(".unused"), "{html}");
}

#[test]
fn svg_text_filters_and_foreign_objects() {
    let html = render_static(svg![
        view_box("0 0 10 10"),
        defs![filter![
            id("glow"),
            feGaussianBlur![attr("stdDeviation", "2")],
            feMerge![feMergeNode![], feMergeNode![]]
        ]],
        text![attr("x", "1"), tspan!["a < b"]],
        foreignObject![attr("width", "10"), div!["html"]],
    ]);
    assert_eq!(
        html,
        r#"<svg viewBox="0 0 10 10"><defs><filter id="glow"><feGaussianBlur stdDeviation="2"></feGaussianBlur><feMerge><feMergeNode></feMergeNode><feMergeNode></feMergeNode></feMerge></filter></defs><text x="1"><tspan>a &lt; b</tspan></text><foreignObject width="10"><div>html</div></foreignObject></svg>"#
    );
    for name in ["text", "tspan", "textPath", "foreignObject", "mask", "pattern", "marker", "filter", "feDropShadow"] {
        assert!(TAGS.contains(&name), "{name} is listed in TAGS");
    }
}

/// A hole that renders the `String` it is given as context.
fn name_hole(fallback: &'static str) -> Node {
    hole(p![fallback], |cx| {
        Box::pin(async move {
            let name = cx.downcast_ref::<String>().cloned().unwrap_or_default();
            p![class("who"), Link!(href = "/me", name)].into_node()
        })
    })
}

#[test]
fn holes_show_their_fallback_without_a_context() {
    assert_eq!(render_static(div![name_hole("…")]), "<div><p>…</p></div>");
    let html = futures_executor::block_on(render_to_string(div![name_hole("…")]));
    assert_eq!(html, "<div><p>…</p></div>");
    // With a context, a document renders them like suspense: streamed, or in place.
    let cx: HoleContext = std::sync::Arc::new("Ada".to_owned());
    let parts =
        |cx: &HoleContext| DocumentParts { hole_context: Some(cx.clone()), ..DocumentParts::new(name_hole("…")) };
    let chunks = futures_executor::block_on(stream_document(parts(&cx), true).collect::<Vec<_>>());
    assert!(chunks[0].contains(r#"<nr-b id="nr-b1"><p>…</p></nr-b>"#), "{}", chunks[0]);
    assert!(chunks[1].starts_with(r#"<template id="nr-t1"><p class="who">"#), "{}", chunks[1]);
    let whole = futures_executor::block_on(stream_document(parts(&cx), false).collect::<Vec<_>>()).concat();
    assert!(whole.contains(r#"<body><p class="who"><a href="/me" data-nr-link="">Ada</a></p></body>"#), "{whole}");
}

#[test]
fn a_shell_is_rendered_once_and_resumed_for_each_request() {
    let view = main![
        h1!["Shop"],
        // Suspense belongs to the shell: it is awaited once.
        suspense(p!["…"], async { p!["catalogue"] }),
        name_hole("who?"),
        // A hole inside a fallback is not one: there is nothing to swap it into.
        hole(name_hole("nested fallback"), |_| Box::pin(async { Node::Empty })),
    ];
    let parts = DocumentParts { tail: Box::new(|f| format!("[links={}]", f.links)), ..DocumentParts::new(view) };
    let (html, shell) = futures_executor::block_on(render_shell(parts));
    let shell = std::sync::Arc::new(shell.expect("a document with holes has a shell"));
    assert_eq!(shell.holes(), 2);
    assert!(html.contains("function $nr("), "the swap script is part of the shell: {html}");
    assert!(
        html.ends_with(
            r#"<body><main><h1>Shop</h1><p>catalogue</p><nr-b id="nr-b1"><p>who?</p></nr-b><nr-b id="nr-b2"><p>nested fallback</p></nr-b></main>"#
        ),
        "no tail, no closing tags: {html}"
    );

    let resume = |name: &str, streaming: bool| Resume {
        html: html.clone(),
        shell: shell.clone(),
        context: std::sync::Arc::new(name.to_owned()),
        nonce: Some("n1".into()),
        tail: Box::new(|f| format!("[links={}]", f.links)),
        streaming,
        known_styles: Vec::new(),
    };
    for name in ["Ada", "Bob"] {
        let chunks = futures_executor::block_on(resume_document(resume(name, true)).collect::<Vec<_>>());
        assert_eq!(chunks[0], html, "the shell goes out as it is");
        let rest = chunks[1..].concat();
        assert!(
            rest.contains(&format!(r#"<template id="nr-t1"><p class="who"><a href="/me" data-nr-link="">{name}</a>"#))
        );
        assert!(rest.contains(r#"<script nonce="n1">$nr(1)</script>"#) && rest.contains(r#"<template id="nr-t2">"#));
        assert!(rest.ends_with("[links=true]</body></html>"), "flags of the holes reach the tail: {rest}");

        let whole = futures_executor::block_on(resume_document(resume(name, false)).collect::<Vec<_>>());
        assert_eq!(whole.len(), 1);
        assert!(whole[0].contains(&format!(
            r#"<p>catalogue</p><p class="who"><a href="/me" data-nr-link="">{name}</a></p></main>"#
        )));
        assert!(!whole[0].contains("<nr-b") && !whole[0].contains("<template"), "{}", whole[0]);
    }

    // Holes that are not in the HTML being sent (a region of the shell) are not run.
    let mut cut = resume("Ada", true);
    cut.html = html.replace(r#"<nr-b id="nr-b1"><p>who?</p></nr-b>"#, "");
    let rest = futures_executor::block_on(resume_document(cut).collect::<Vec<_>>())[1..].concat();
    assert!(!rest.contains("nr-t1") && rest.contains("nr-t2"), "{rest}");

    // No holes: the document is complete and there is nothing to resume.
    let (html, shell) = futures_executor::block_on(render_shell(DocumentParts::new(p!["plain"])));
    assert!(shell.is_none() && html.ends_with("<p>plain</p></body></html>"), "{html}");
}

#[test]
fn rendered_fragments_carry_their_styles_and_flags() {
    static CARD: Stylesheet = Stylesheet { id: "card", css: ".card{color:red}", per_class: None, scripts: &[] };
    let fragment = futures_executor::block_on(render_fragment(div![
        class("card"),
        &CARD,
        suspense((), async { Link!(href = "/x", "more") })
    ]));
    assert_eq!(fragment.html, r#"<div class="card"><a href="/x" data-nr-link="">more</a></div>"#);
    assert!(fragment.flags.links && fragment.classes == ["card"] && fragment.sheets.len() == 1);
    assert!(!fragment.flags.forms);
    // A form asks for the runtime too: it is sent without a page reload.
    let search = futures_executor::block_on(render_fragment(form![action("/search"), input![name("q")]]));
    assert!(search.flags.forms && !search.flags.links);

    let twice = fragment![Node::Rendered(Box::new(fragment.clone())), Node::Rendered(Box::new(fragment))];
    let parts = DocumentParts { tail: Box::new(|f| format!("[links={}]", f.links)), ..DocumentParts::new(twice) };
    let html = futures_executor::block_on(stream_document(parts, false).collect::<Vec<_>>()).concat();
    assert_eq!(html.matches(r#"<style data-nr-css="card">"#).count(), 1, "hoisted once: {html}");
    assert_eq!(html.matches(r#"<div class="card">"#).count(), 2);
    assert!(html.ends_with("[links=true]</body></html>"));
}
