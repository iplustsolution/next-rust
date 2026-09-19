//! The "client" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Next Rust renders HTML on the server and ships ",
            strong!["no JavaScript by default"],
            ". Client-side behaviour is opt-in, per page and per component.",
        ],
        h2![id("execution-model"), a![class("anchor"), href("#execution-model"), "Execution model"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["where"], th!["what runs"], th!["how"]]],
                tbody![
                    tr![
                        td!["Server (native Rust)"],
                        td!["pages, layouts, loaders, API routes, middleware, actions"],
                        td!["compiled into the server binary"],
                    ],
                    tr![td!["Browser – HTML/CSS"], td!["the rendered document"], td!["no runtime needed"]],
                    tr![
                        td!["Browser – client runtime"],
                        td![
                            code!["Link!"],
                            " navigation, prefetch, form enhancement, declarative islands, streaming swaps",
                        ],
                        td![code!["/_next-rust/runtime.js"], ", a ~4 KB gzipped ES module, loaded only when used"],
                    ],
                    tr![
                        td!["Browser – island modules"],
                        td!["custom interactive widgets"],
                        td!["your ES module, e.g. ", code!["wasm-bindgen"], " output, loaded per island"],
                    ],
                ],
            ],
        ],
        p![
            "Rust doesn't run natively in browsers. It runs there only when compiled to WebAssembly. Next Rust's server never ships Rust code to the client unless you compile and serve a WASM module yourself (see ",
            a![href("#wasm-islands"), "WASM islands"],
            ").",
        ],
        p![
            "The runtime script is added to a page only when the render used a link to a page of the app (",
            code!["Link!"],
            " or a plain ",
            code!["a![href(\"/about\")]"],
            ", with ",
            code!["[rendering] client_navigation = true"],
            ", the default) or an island. Pages with neither ship zero JavaScript.",
        ],
        h2![
            id("islands-with-client"),
            a![class("anchor"), href("#islands-with-client"), "Islands with ", code!["#[client]"]],
        ],
        pre![code![
            class("language-rust"),
            r#"use next_rust::prelude::*;

#[client]
pub fn Counter(count: i32) -> impl View {
    div![
        button![on("click", "decrement:count"), aria("label", "Decrease"), "−"],
        output![data("nr-text", "count"), count],
        button![on("click", "increment:count"), aria("label", "Increase"), "+"],
    ]
}

// in a page
div![h1!["Stats"], Counter(3)]"#,
        ],],
        p![
            "On the server the component renders normally, wrapped in ",
            code!["<nr-island data-component=\"Counter\" data-props='{\"count\":3}'>"],
            ". The arguments must implement ",
            code!["Serialize"],
            " and become the island's ",
            strong!["state"],
            ". The rest of the page stays static HTML. That's partial hydration: only islands get behaviour attached.",
        ],
        h3![
            id("declarative-behaviour-no-custom-javascript-csp-safe"),
            a![
                class("anchor"),
                href("#declarative-behaviour-no-custom-javascript-csp-safe"),
                "Declarative behaviour (no custom JavaScript, CSP-safe)",
            ],
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["attribute"], th!["effect"]]],
                tbody![
                    tr![td![code!["data-nr-text=\"path\""]], td!["text content bound to state"]],
                    tr![
                        td![code!["data-nr-show=\"path\""], " / ", code!["\"!path\""]],
                        td!["toggles ", code!["hidden"]],
                    ],
                    tr![td![code!["data-nr-bind=\"path\""]], td!["two-way binding for inputs, checkboxes, selects"],],
                    tr![td![code!["data-nr-class-<name>=\"path\""]], td!["toggles a class"]],
                    tr![
                        td![code!["on(\"event\", \"ops\")"], " → ", code!["data-nr-on-<event>"]],
                        td!["runs operations, separated by ", code![";"]],
                    ],
                ],
            ],
        ],
        p![
            "Operations: ",
            code!["increment:path[,n]"],
            ", ",
            code!["decrement:path[,n]"],
            ", ",
            code!["toggle:path"],
            ", ",
            code!["set:path=<json>"],
            ", ",
            code!["prevent"],
            ", ",
            code!["navigate:/url"],
            ", and ",
            code!["action:<url>[->path]"],
            ", which calls a ",
            a![href("/docs/server-actions"), "server action"],
            " with the island state and stores the result.",
        ],
        p![
            "The runtime never evaluates strings as code, so islands work under a strict Content Security Policy without ",
            code!["unsafe-eval"],
            ".",
        ],
        h3![id("module-islands"), a![class("anchor"), href("#module-islands"), "Module islands"]],
        p!["For behaviour beyond the declarative operations, point an island at an ES module:"],
        pre![code![
            class("language-rust"),
            r#"#[client(module = "/_next-rust/client/islands/chart.js")]
pub fn Chart(points: Vec<(f64, f64)>) -> impl View {
    canvas![width(600), height(300), aria("label", "Chart")]
}"#,
        ],],
        pre![code![
            class("language-js"),
            r#"// client/islands/chart.js – served from /_next-rust/client/
export function hydrate(element, props) {
  const ctx = element.querySelector("canvas").getContext("2d");
  // draw props.points …
}"#,
        ],],
        p![
            "Files in the project's ",
            code!["client/"],
            " directory are served under ",
            code!["/_next-rust/client/"],
            " with the same path-traversal protection as ",
            code!["public/"],
            ".",
        ],
        h3![id("wasm-islands"), a![class("anchor"), href("#wasm-islands"), "WASM islands"]],
        p![
            "A module island can be a Rust crate compiled to WebAssembly. Build it with the standard toolchain and point the island at the generated JavaScript glue:",
        ],
        pre![code![
            class("language-sh"),
            r"rustup target add wasm32-unknown-unknown
cargo build -p my-widgets --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir client/islands target/wasm32-unknown-unknown/release/my_widgets.wasm",
        ],],
        pre![code![
            class("language-rust"),
            r"// my-widgets/src/lib.rs
#[wasm_bindgen]
pub fn hydrate(element: web_sys::Element, props: JsValue) { … }",
        ],],
        pre![code![
            class("language-rust"),
            r#"#[client(module = "/_next-rust/client/islands/my_widgets.js")]
pub fn Editor(doc: Document) -> impl View { … }"#,
        ],],
        p![
            strong!["Status:"],
            " the CLI doesn't yet automate this build step, and there's no Rust-side reactive DOM library. See ",
            a![href("/docs/status"), "Status & roadmap"],
            ". The protocol (",
            code!["hydrate(element, props)"],
            " plus JSON props) is stable, so a future ",
            code!["next-rust build --wasm"],
            " can fill that gap without changing components.",
        ],
        h2![id("serverclient-boundary"), a![class("anchor"), href("#serverclient-boundary"), "Server/client boundary"],],
        ul![
            li![
                p![
                    "Page, layout, loader, API and action code is compiled only into the server binary. HTML and serialized island props are the only things sent to the browser.",
                ],
                " ",
            ],
            li![
                p![
                    strong!["Island props are public."],
                    " Everything passed to a ",
                    code!["#[client]"],
                    " component is embedded in the HTML. Never pass secrets, tokens or internal records.",
                ],
                " ",
            ],
            li![
                p![
                    code!["#[server]"],
                    " marks an item as server-only. On ",
                    code!["wasm32"],
                    " targets it's compiled out, so accidental use from browser code fails to compile:",
                ],
                " ",
                pre![code![
                    class("language-rust"),
                    r"#[server]
pub async fn load_invoice(id: u64) -> Result<Invoice> { db::invoice(id).await }",
                ],],
                " ",
            ],
            li![
                p![
                    code!["#[server_action]"],
                    " functions are also compiled out on ",
                    code!["wasm32"],
                    ". Only their id constant remains, so client code can reference the action URL.",
                ],
                " ",
            ],
            li![
                p![
                    "Environment variables reach the browser only when they start with the public prefix (",
                    code!["NEXT_RUST_PUBLIC_"],
                    " by default), and only on pages with islands, as ",
                    code!["window.nextRust.env"],
                    ".",
                ],
                " ",
            ],
        ],
        h2![
            id("client-side-navigation"),
            a![class("anchor"), href("#client-side-navigation"), "Client-side navigation"],
        ],
        p![
            "With the runtime loaded, clicks on same-origin links (",
            code!["Link!"],
            " and plain ",
            code!["a![href(\"/about\"), \"About\"]"],
            " alike) don't reload the page. Instead they:",
        ],
        ol![
            li![
                "fetch the target HTML, unless it is already in memory (see below). Only the part below the layouts both pages share is requested (see ",
                a![href("/docs/routing#layouts-stay-on-screen"), "Layouts stay on screen"],
                ");",
            ],
            li![
                "swap ",
                code!["<body>"],
                ", ",
                code!["<title>"],
                " and managed ",
                code!["<meta>"],
                " / ",
                code!["<style data-nr-css>"],
                " elements;",
            ],
            li!["update history and scroll position, then hydrate islands in the new content."],
        ],
        h3![id("prefetching"), a![class("anchor"), href("#prefetching"), "Prefetching"]],
        p![
            "A page is fetched ahead of the click only when the mouse rests on its link for ",
            strong!["400 ms"],
            ". Moving or scrolling past links, pressing the button, and touch taps never fetch early, so a long list of cards doesn't turn into a burst of requests. Links to the page that is already open and same-page ",
            code!["#anchors"],
            " are never prefetched.",
        ],
        p![
            "Turn it off for a link with ",
            code!["prefetch = false"],
            h3![id("page-reuse"), a![class("anchor"), href("#page-reuse"), "Pages are fetched once"]],
            p![
                "Every page the runtime downloads, by prefetching or by navigating, is kept in memory for 30 seconds. Within that time a page is never requested twice: clicking a link that was prefetched, clicking it again, and the browser's back and forward buttons all reuse the copy. Clicking a link while its prefetch is still loading waits for that same request.",
            ],
            ul![
                li![
                    "After a server action or an enhanced form succeeds, every kept page is dropped, so the next navigation shows fresh data."
                ],
                li![code!["nextRust.refresh()"], " always fetches the current page again."],
                li![
                    "A page served by an intercepting route is only reused when navigating from the same page, because its HTML depends on where the navigation started."
                ],
            ],
            " on ",
            code!["Link!"],
            ", or ",
            code!["prefetch(false)"],
            " on a plain anchor."
        ],
        p![
            "A non-HTML response or network failure falls back to a full page load. Modifier-clicks, ",
            code!["target=_blank"],
            ", ",
            code!["download"],
            ", ",
            code!["rel=\"external\""],
            " and cross-origin links are left to the browser. To force a full page load for one link (for example a logout endpoint that sets cookies), add ",
            code!["reload(true)"],
            ":",
        ],
        pre![code![class("language-rust"), r#"a![href("/logout"), reload(true), "Log out"]"#]],
        p!["Imperative API:"],
        pre![code![
            class("language-js"),
            r#"nextRust.navigate("/dashboard");
nextRust.replace("/login");
nextRust.back();
nextRust.forward();
nextRust.refresh();   // re-fetch the current page without prefetch cache
nextRust.prefetch("/pricing");
window.addEventListener("nr:navigate", (e) => console.log(e.detail.url));"#,
        ],],
        p![
            "During navigation ",
            code!["<html>"],
            " has the ",
            code!["data-nr-navigating"],
            " attribute, for progress indicators.",
        ],
        h3![id("requests"), a![class("anchor"), href("#requests"), "Requests from the browser"]],
        p![
            "Island code that talks to this site goes through the runtime rather than a bare ",
            code!["fetch"],
            ": the request carries the page's cookies and CSRF token, refuses other origins (so those credentials can never leak), turns a non-2xx response into an error with ",
            code![".status"],
            " and ",
            code![".data"],
            ", and drops the prefetch cache after a write.",
        ],
        pre![code![
            class("language-js"),
            r#"const user = await nextRust.action(url, { email });            // a server action (url from action!(x).url())
const list = await nextRust.request("/api/items?page=2");       // GET, JSON out
await nextRust.request("/api/items", { method: "POST", body: { name } });  // JSON in (FormData/string pass through)
await nextRust.request("/api/slow", { timeout: 5000 });          // aborts after 5 s; or pass your own `signal`
const audio = await nextRust.request("/api/speak", { method: "POST", body: { text }, as: "blob" });  // as: "blob" | "text" | "response"

// Server-sent events (`Response::sse`) as an async iterator:
for await (const { event, data } of nextRust.stream("/api/chat", { method: "POST", body: { prompt } })) {
  if (event === "token") output.textContent += JSON.parse(data);
}"#,
        ],],
        p![
            "Navigation requests carry ",
            code!["x-nr-nav: 1"],
            " and ",
            code!["x-nr-from"],
            ", which power ",
            a![href("/docs/routing#intercepting-routes"), "intercepting routes"],
            ".",
        ],
        p![
            "Known limitation: inline ",
            code!["<script>"],
            " elements in the new page are re-executed. Under a strict nonce-based CSP they're blocked, because the nonce changes per response. Prefer module islands over inline scripts.",
        ],
    ]
}
