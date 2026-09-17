# Views & components

## Elements

Every HTML element has a macro. Its arguments are **parts**: attributes and
children, in any order, separated by commas.

```rust
div![
    class("card"),
    h2!["Title"],
    p![class("muted"), "Body text"],
    img![src("/cat.jpg"), alt("A cat")],
]
```

renders

```html
<div class="card"><h2>Title</h2><p class="muted">Body text</p><img src="/cat.jpg" alt="A cat"></div>
```

Void elements (`img`, `input`, `br`, `meta`, …) never render children. The
full list is `next_rust::TAGS`, and includes common SVG elements (`svg`,
`path`, `circle`, …).

## Attributes

```rust
input![r#type("email"), name("email"), required(true), placeholder("you@example.com")]
a![href("/docs"), target("_blank"), rel("noopener")]
div![data("id", "42"), aria("label", "Close"), role("dialog")]
button![disabled(is_busy), "Save"]          // boolean attributes
div![attr("hx-get", "/fragment")]           // any attribute
div![attr_if(selected, class("selected"))]  // conditional attribute
```

`class` accepts strings, CSS-module classes, arrays, `Option`s and
`(value, condition)` pairs. Several `class` parts are merged:

```rust
div![class("tab"), class([("active", is_active), ("disabled", !enabled)])]
```

## Text and escaping

Anything implementing `View` can be a child: `&str`, `String`, numbers,
`char`, `Option<V>`, `Vec<V>`, arrays, tuples, `Node`, `Element`.

**All text and attribute values are escaped.**

- `<`, `>`, `&` in text; also `"` and `'` in attributes.
- URL attributes (`href`, `src`, `action`, `formaction`, `poster`, …) that
  start with `javascript:`, `vbscript:` or non-image `data:` are replaced by
  `#`. The check ignores control characters and whitespace, so
  `java\tscript:` is caught too.
- Event-handler attributes (`onclick`, …) passed to `attr()` are dropped.
  Use `raw_attr()` if you really need one.
- Text inside `<script>` and `<style>` has `</` sequences neutralized.
- Attribute names containing quotes, spaces, `=`, `>` or `/` are dropped.

To output trusted HTML, opt in explicitly:

```rust
div![raw_html(markdown_to_html(&post.body))]   // never pass user input
```

## Components

Components are plain functions:

```rust
pub fn Button(label: &str) -> impl View {
    button![class("btn"), label]
}

pub fn Card(title: &str, children: impl View) -> impl View {
    article![class("card"), h3![title], children]
}

// usage
Card("Hello", fragment![p!["One"], Button("Go")])
```

Return `impl View`, `Element` or `Node`. Use `Node` when branches produce
different types:

```rust
fn Status(ok: bool) -> Node {
    if ok { span!["OK"].into_node() } else { strong!["Error"].into_node() }
}
```

## Conditionals

```rust
div![
    when(user.is_admin, || AdminPanel()),                 // lazily built
    if items.is_empty() { Some(p!["Nothing yet"]) } else { None },
    user.avatar.as_ref().map(|url| img![src(url.clone()), alt("")]),
]
```

## Lists

```rust
ul![each(&todos, |t| li![key(t.id), t.title.clone()])]
ul![todos.iter().map(|t| li![t.title.clone()]).collect::<Vec<_>>()]
```

`key(..)` renders `data-nr-key`. Interactive islands use it to keep list
items' identity stable.

## Fragments

```rust
fragment![h1!["Title"], p!["Intro"]]   // no wrapper element
```

## Async content

```rust
async fn Recommendations(user: u64) -> impl View { … }

div![suspense(p!["Loading…"], Recommendations(user.id))]
```

See [streaming](rendering.md#streaming).

## Links

```rust
Link!(href = "/pricing", "Pricing")
Link!(href = "/docs", class = "nav", prefetch = false, span!["Docs"])
Link!(href = "/step/2", replace = true, scroll = false, "Next")
```

`Link!` renders a normal `<a href>` that works without JavaScript. Plain
anchors get the same treatment: `a![href("/about"), "About"]` also navigates
without a page refresh and prefetches on hover. Use `reload(true)` on a link
that must do a full page load. See [client.md](client.md).

## Images

```rust
Image!(src = "/photos/lake.jpg", width = 1200, height = 800, alt = "A lake at dawn")
Image!(src = "/hero.jpg", width = 1920, height = 1080, alt = "", priority = true, sizes = "100vw")
```

This produces `srcset`/`sizes`, `loading="lazy"` (eager with
`priority = true`), `decoding="async"` and explicit dimensions to avoid
layout shift. In debug builds, a missing `alt` is flagged with
`data-nr-warning`. See [styling-and-assets.md](styling-and-assets.md#images)
for how images are served.

## Accessibility

Next Rust doesn't try to make markup accessible automatically. It gives you
the tools:

- semantic element macros (`main!`, `nav!`, `header!`, `article!`, `dialog!`, …);
- `aria(..)`, `role(..)`, `r#for(..)`, `tabindex(..)`;
- `Image!` requires you to decide on `alt` and warns in development;
- generated error boundaries use `role="alert"` and loading fallbacks use `aria-busy`.

## Rendering outside requests

```rust
let html = next_rust::render_static(Card("Hi", p!["x"]));          // sync, suspense → fallback
let html = next_rust::render_to_string(page_view).await;            // resolves suspense
```

Use these for emails, tests or custom endpoints.
