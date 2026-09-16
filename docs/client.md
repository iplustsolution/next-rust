# Client components, hydration & navigation

Next Rust renders HTML on the server and ships **no JavaScript by default**.
Client-side behaviour is opt-in, per page and per component.

## Execution model

| where | what runs | how |
|---|---|---|
| Server (native Rust) | pages, layouts, loaders, API routes, middleware, actions | compiled into the server binary |
| Browser – HTML/CSS | the rendered document | no runtime needed |
| Browser – client runtime | `Link!` navigation, prefetch, form enhancement, declarative islands, streaming swaps | `/_nr/runtime.js`, a ~4 KB gzipped ES module, loaded only when used |
| Browser – island modules | custom interactive widgets | your ES module, e.g. `wasm-bindgen` output, loaded per island |

Rust doesn't run natively in browsers. It runs there only when compiled to
WebAssembly. Next Rust's server never ships Rust code to the client unless
you compile and serve a WASM module yourself (see
[WASM islands](#wasm-islands)).

The runtime script is added to a page only when the render used a `Link!`
(with `[rendering] client_navigation = true`, the default) or an island.
Pages with neither ship zero JavaScript.

## Islands with `#[client]`

```rust
use next_rust::prelude::*;

#[client]
pub fn Counter(count: i32) -> impl View {
    div![
        button![on("click", "decrement:count"), aria("label", "Decrease"), "−"],
        output![data("nr-text", "count"), count],
        button![on("click", "increment:count"), aria("label", "Increase"), "+"],
    ]
}

// in a page
div![h1!["Stats"], Counter(3)]
```

On the server the component renders normally, wrapped in
`<nr-island data-component="Counter" data-props='{"count":3}'>`. The
arguments must implement `Serialize` and become the island's **state**. The
rest of the page stays static HTML. That's partial hydration: only islands
get behaviour attached.

### Declarative behaviour (no custom JavaScript, CSP-safe)

| attribute | effect |
|---|---|
| `data-nr-text="path"` | text content bound to state |
| `data-nr-show="path"` / `"!path"` | toggles `hidden` |
| `data-nr-bind="path"` | two-way binding for inputs, checkboxes, selects |
| `data-nr-class-<name>="path"` | toggles a class |
| `on("event", "ops")` → `data-nr-on-<event>` | runs operations, separated by `;` |

Operations: `increment:path[,n]`, `decrement:path[,n]`, `toggle:path`,
`set:path=<json>`, `prevent`, `navigate:/url`, and `action:<url>[->path]`,
which calls a [server action](server-actions.md) with the island state and
stores the result.

The runtime never evaluates strings as code, so islands work under a strict
Content Security Policy without `unsafe-eval`.

### Module islands

For behaviour beyond the declarative operations, point an island at an ES
module:

```rust
#[client(module = "/_nr/client/islands/chart.js")]
pub fn Chart(points: Vec<(f64, f64)>) -> impl View {
    canvas![width(600), height(300), aria("label", "Chart")]
}
```

```js
// client/islands/chart.js – served from /_nr/client/
export function hydrate(element, props) {
  const ctx = element.querySelector("canvas").getContext("2d");
  // draw props.points …
}
```

Files in the project's `client/` directory are served under `/_nr/client/`
with the same path-traversal protection as `public/`.

### WASM islands

A module island can be a Rust crate compiled to WebAssembly. Build it with
the standard toolchain and point the island at the generated JavaScript glue:

```sh
rustup target add wasm32-unknown-unknown
cargo build -p my-widgets --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir client/islands target/wasm32-unknown-unknown/release/my_widgets.wasm
```

```rust
// my-widgets/src/lib.rs
#[wasm_bindgen]
pub fn hydrate(element: web_sys::Element, props: JsValue) { … }
```

```rust
#[client(module = "/_nr/client/islands/my_widgets.js")]
pub fn Editor(doc: Document) -> impl View { … }
```

**Status:** the CLI doesn't yet automate this build step, and there's no
Rust-side reactive DOM library. See [status.md](status.md). The protocol
(`hydrate(element, props)` plus JSON props) is stable, so a future
`next-rust build --wasm` can fill that gap without changing components.

## Server/client boundary

- Page, layout, loader, API and action code is compiled only into the server
  binary. HTML and serialized island props are the only things sent to the
  browser.
- **Island props are public.** Everything passed to a `#[client]` component
  is embedded in the HTML. Never pass secrets, tokens or internal records.
- `#[server]` marks an item as server-only. On `wasm32` targets it's
  compiled out, so accidental use from browser code fails to compile:

  ```rust
  #[server]
  pub async fn load_invoice(id: u64) -> Result<Invoice> { db::invoice(id).await }
  ```

- `#[server_action]` functions are also compiled out on `wasm32`. Only their
  id constant remains, so client code can reference the action URL.
- Environment variables reach the browser only when they start with the
  public prefix (`NEXT_RUST_PUBLIC_` by default), and only on pages with
  islands, as `window.nextRust.env`.

## Client-side navigation

With the runtime loaded, `Link!` clicks:

1. fetch the target HTML (reusing a prefetch from hover or touch, cached for 30 seconds);
2. swap `<body>`, `<title>` and managed `<meta>` / `<style data-nr-css>` elements;
3. update history and scroll position, then hydrate islands in the new content.

A non-HTML response or network failure falls back to a full page load.
Modifier-clicks, `target=_blank`, `download` and cross-origin links are left
to the browser.

Imperative API:

```js
nextRust.navigate("/dashboard");
nextRust.replace("/login");
nextRust.back();
nextRust.forward();
nextRust.refresh();   // re-fetch the current page without prefetch cache
nextRust.prefetch("/pricing");
window.addEventListener("nr:navigate", (e) => console.log(e.detail.url));
```

During navigation `<html>` has the `data-nr-navigating` attribute, for
progress indicators.

Navigation requests carry `x-nr-nav: 1` and `x-nr-from`, which power
[intercepting routes](routing.md#intercepting-routes).

Known limitation: inline `<script>` elements in the new page are re-executed.
Under a strict nonce-based CSP they're blocked, because the nonce changes per
response. Prefer module islands over inline scripts.
