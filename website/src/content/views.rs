//! The "views" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        h2![id("elements"), a![class("anchor"), href("#elements"), "Elements"]],
        p![
            "Every HTML element has a macro. Its arguments are ",
            strong!["parts"],
            ": attributes and children, in any order, separated by commas.",
        ],
        pre![code![
            class("language-rust"),
            r#"div![
    class("card"),
    h2!["Title"],
    p![class("muted"), "Body text"],
    img![src("/cat.jpg"), alt("A cat")],
]"#,
        ],],
        p!["renders"],
        pre![code![
            class("language-html"),
            r#"<div class="card"><h2>Title</h2><p class="muted">Body text</p><img src="/cat.jpg" alt="A cat"></div>"#,
        ],],
        p![
            "Void elements (",
            code!["img"],
            ", ",
            code!["input"],
            ", ",
            code!["br"],
            ", ",
            code!["meta"],
            ", …) never render children. The full list is ",
            code!["next_rust::TAGS"],
            ", and includes common SVG elements (",
            code!["svg"],
            ", ",
            code!["path"],
            ", ",
            code!["circle"],
            ", …).",
        ],
        h2![id("attributes"), a![class("anchor"), href("#attributes"), "Attributes"]],
        pre![code![
            class("language-rust"),
            r#"input![r#type("email"), name("email"), required(true), placeholder("you@example.com")]
a![href("/docs"), target("_blank"), rel("noopener")]
div![data("id", "42"), aria("label", "Close"), role("dialog")]
button![disabled(is_busy), "Save"]          // boolean attributes
div![attr("hx-get", "/fragment")]           // any attribute
div![attr_if(selected, class("selected"))]  // conditional attribute"#,
        ],],
        p![
            code!["class"],
            " accepts strings, CSS-module classes, arrays, ",
            code!["Option"],
            "s and ",
            code!["(value, condition)"],
            " pairs. Several ",
            code!["class"],
            " parts are merged:",
        ],
        pre![code![
            class("language-rust"),
            r#"div![class("tab"), class([("active", is_active), ("disabled", !enabled)])]"#,
        ],],
        h2![id("text-and-escaping"), a![class("anchor"), href("#text-and-escaping"), "Text and escaping"]],
        p![
            "Anything implementing ",
            code!["View"],
            " can be a child: ",
            code!["&str"],
            ", ",
            code!["String"],
            ", numbers, ",
            code!["char"],
            ", ",
            code!["Option<V>"],
            ", ",
            code!["Vec<V>"],
            ", arrays, tuples, ",
            code!["Node"],
            ", ",
            code!["Element"],
            ".",
        ],
        p![strong!["All text and attribute values are escaped."]],
        ul![
            li![
                code!["<"],
                ", ",
                code![">"],
                ", ",
                code!["&"],
                " in text; also ",
                code!["\""],
                " and ",
                code!["'"],
                " in attributes.",
            ],
            li![
                "URL attributes (",
                code!["href"],
                ", ",
                code!["src"],
                ", ",
                code!["action"],
                ", ",
                code!["formaction"],
                ", ",
                code!["poster"],
                ", …) that start with ",
                code!["javascript:"],
                ", ",
                code!["vbscript:"],
                " or non-image ",
                code!["data:"],
                " are replaced by ",
                code!["#"],
                ". The check ignores control characters and whitespace, so ",
                code!["java\\tscript:"],
                " is caught too.",
            ],
            li![
                "Event-handler attributes (",
                code!["onclick"],
                ", …) passed to ",
                code!["attr()"],
                " are dropped. Use ",
                code!["raw_attr()"],
                " if you really need one.",
            ],
            li![
                "Text inside ",
                code!["<script>"],
                " and ",
                code!["<style>"],
                " has ",
                code!["</"],
                " sequences neutralized.",
            ],
            li![
                "Attribute names containing quotes, spaces, ",
                code!["="],
                ", ",
                code![">"],
                " or ",
                code!["/"],
                " are dropped.",
            ],
        ],
        p!["To output trusted HTML, opt in explicitly:"],
        pre![
            code![class("language-rust"), r"div![raw_html(markdown_to_html(&post.body))]   // never pass user input",],
        ],
        h2![id("components"), a![class("anchor"), href("#components"), "Components"]],
        p!["Components are plain functions:"],
        pre![code![
            class("language-rust"),
            r#"pub fn Button(label: &str) -> impl View {
    button![class("btn"), label]
}

pub fn Card(title: &str, children: impl View) -> impl View {
    article![class("card"), h3![title], children]
}

// usage
Card("Hello", fragment![p!["One"], Button("Go")])"#,
        ],],
        p![
            "Return ",
            code!["impl View"],
            ", ",
            code!["Element"],
            " or ",
            code!["Node"],
            ". Use ",
            code!["Node"],
            " when branches produce different types:",
        ],
        pre![code![
            class("language-rust"),
            r#"fn Status(ok: bool) -> Node {
    if ok { span!["OK"].into_node() } else { strong!["Error"].into_node() }
}"#,
        ],],
        h2![id("conditionals"), a![class("anchor"), href("#conditionals"), "Conditionals"]],
        pre![code![
            class("language-rust"),
            r#"div![
    when(user.is_admin, || AdminPanel()),                 // lazily built
    if items.is_empty() { Some(p!["Nothing yet"]) } else { None },
    user.avatar.as_ref().map(|url| img![src(url.clone()), alt("")]),
]"#,
        ],],
        h2![id("lists"), a![class("anchor"), href("#lists"), "Lists"]],
        pre![code![
            class("language-rust"),
            r"ul![each(&todos, |t| li![key(t.id), t.title.clone()])]
ul![todos.iter().map(|t| li![t.title.clone()]).collect::<Vec<_>>()]",
        ],],
        p![
            code!["key(..)"],
            " renders ",
            code!["data-nr-key"],
            ". Interactive islands use it to keep list items' identity stable.",
        ],
        h2![id("fragments"), a![class("anchor"), href("#fragments"), "Fragments"]],
        pre![code![class("language-rust"), r#"fragment![h1!["Title"], p!["Intro"]]   // no wrapper element"#]],
        h2![id("async-content"), a![class("anchor"), href("#async-content"), "Async content"]],
        pre![code![
            class("language-rust"),
            r#"async fn Recommendations(user: u64) -> impl View { … }

div![suspense(p!["Loading…"], Recommendations(user.id))]"#,
        ],],
        p!["See ", a![href("/docs/rendering#streaming"), "streaming"], "."],
        h2![id("links"), a![class("anchor"), href("#links"), "Links"]],
        pre![code![
            class("language-rust"),
            r#"Link!(href = "/pricing", "Pricing")
Link!(href = "/docs", class = "nav", prefetch = false, span!["Docs"])
Link!(href = "/step/2", replace = true, scroll = false, "Next")"#,
        ],],
        p![
            code!["Link!"],
            " renders a normal ",
            code!["<a href>"],
            " that works without JavaScript. Plain anchors get the same treatment: ",
            code!["a![href(\"/about\"), \"About\"]"],
            " also navigates without a page refresh and prefetches on hover. Use ",
            code!["reload(true)"],
            " on a link that must do a full page load. See ",
            a![href("/docs/client"), "client.md"],
            ".",
        ],
        h2![id("images"), a![class("anchor"), href("#images"), "Images"]],
        pre![code![
            class("language-rust"),
            r#"Image!(src = "/photos/lake.jpg", width = 1200, height = 800, alt = "A lake at dawn")
Image!(src = "/hero.jpg", width = 1920, height = 1080, alt = "", priority = true, sizes = "100vw")"#,
        ],],
        p![
            "This produces ",
            code!["srcset"],
            "/",
            code!["sizes"],
            ", ",
            code!["loading=\"lazy\""],
            " (eager with ",
            code!["priority = true"],
            "), ",
            code!["decoding=\"async\""],
            " and explicit dimensions to avoid layout shift. In debug builds, a missing ",
            code!["alt"],
            " is flagged with ",
            code!["data-nr-warning"],
            ". See ",
            a![href("/docs/styling-and-assets#images"), "styling-and-assets.md"],
            " for how images are served.",
        ],
        h2![id("accessibility"), a![class("anchor"), href("#accessibility"), "Accessibility"]],
        p!["Next Rust doesn't try to make markup accessible automatically. It gives you the tools:"],
        ul![
            li![
                "semantic element macros (",
                code!["main!"],
                ", ",
                code!["nav!"],
                ", ",
                code!["header!"],
                ", ",
                code!["article!"],
                ", ",
                code!["dialog!"],
                ", …);",
            ],
            li![code!["aria(..)"], ", ", code!["role(..)"], ", ", code!["r#for(..)"], ", ", code!["tabindex(..)"], ";",],
            li![code!["Image!"], " requires you to decide on ", code!["alt"], " and warns in development;"],
            li![
                "generated error boundaries use ",
                code!["role=\"alert\""],
                " and loading fallbacks use ",
                code!["aria-busy"],
                ".",
            ],
        ],
        h2![
            id("rendering-outside-requests"),
            a![class("anchor"), href("#rendering-outside-requests"), "Rendering outside requests"],
        ],
        pre![code![
            class("language-rust"),
            r#"let html = next_rust::render_static(Card("Hi", p!["x"]));          // sync, suspense → fallback
let html = next_rust::render_to_string(page_view).await;            // resolves suspense"#,
        ],],
        p!["Use these for emails, tests or custom endpoints."],
    ]
}
