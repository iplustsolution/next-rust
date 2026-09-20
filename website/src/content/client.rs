//! The "client" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "A Next Rust page arrives in the browser finished, like a printed poster: nothing has to run for it to \
             be readable. Most of a site needs no more than that. Where a part should react without asking the \
             server (a counter, a menu, a chart), you mark just that part as an island: a small interactive patch \
             in an otherwise still page. The framework also ships one small script of its own, only to pages that \
             need it, which makes links and forms feel instant by swapping the page instead of reloading it."
        ),
        p![
            "Next Rust renders HTML on the server and ships ",
            strong!["no JavaScript by default"],
            ". Client-side behaviour is opt-in, per page and per component.",
        ],
        p!["Words used on this page:"],
        ul![
            li![strong!["Client"], ": the visitor's browser. ", strong!["Server"], ": your compiled Rust program.",],
            li![
                strong!["Client runtime"],
                ": the framework's own small JavaScript file, ",
                code!["/_next-rust/runtime.js"],
                ". You never edit it.",
            ],
            li![
                strong!["Island"],
                ": one component that gets behaviour in the browser while the page around it stays plain HTML.",
            ],
            li![
                strong!["Hydration"],
                ": attaching behaviour to HTML the server already rendered, rather than building it again in the \
                 browser.",
            ],
            li![
                strong!["Props"],
                ": the arguments passed to an island, copied into the HTML so the browser side can read them.",
            ],
            li![
                strong!["ES module"],
                ": a standard JavaScript file that exports functions, loaded with ",
                code!["<script type=\"module\">"],
                " or ",
                code!["import"],
                ".",
            ],
            li![
                strong!["WebAssembly (WASM)"],
                ": a compact binary format browsers can run. It is how languages other than JavaScript, Rust \
                 included, run in a browser.",
            ],
            li![
                strong!["CSP"],
                " (content security policy): a response header that tells the browser which scripts it may run, the \
                 main defence against injected scripts.",
            ],
            li![strong!["Prefetch"], ": downloading a page shortly before it is needed, so the click feels instant.",],
        ],
        h2![id("execution-model"), a![class("anchor"), href("#execution-model"), "Execution model"]],
        p!["Four kinds of code exist in an app. This table says where each runs and how it gets there:"],
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
            "Rust doesn't run natively in browsers. It runs there only when compiled to WebAssembly. Next Rust's server never ships Rust code to the client unless you ask for it: either by compiling and serving a WASM module yourself (see ",
            a![href("#wasm-islands"), "WASM islands"],
            "), or with ",
            code!["[build] wasm_islands = true"],
            " (see ",
            a![href("#webassembly-islands"), "WebAssembly islands"],
            ").",
        ],
        p![
            "The runtime script is added to a page only when the render used a link to a page of the app (",
            code!["Link!"],
            " or a plain ",
            code!["a![href(\"/about\")]"],
            ", with ",
            code!["[rendering] client_navigation = true"],
            ", the default), a form (with ",
            code!["[client] enhance_forms = true"],
            ", the default), an interactive UI component, or an island. Pages with none of these ship zero JavaScript. Navigations swap the page inside a view transition, so one page fades into the next (see ",
            a![href("/docs/styling-and-assets#animation"), "Animation"],
            ").",
        ],
        h2![
            id("islands-with-client"),
            a![class("anchor"), href("#islands-with-client"), "Islands with ", code!["#[client]"]],
        ],
        p![
            "An island is an ordinary component function with ",
            code!["#[client]"],
            " on top. It can live in any Rust file of the project, for example ",
            code!["src/components/counter.rs"],
            ", and is used in a page like any other component. The example is a counter with a minus button, the \
             number, and a plus button. Note that there is no JavaScript in it: the ",
            code!["on(..)"],
            " and ",
            code!["data(..)"],
            " attributes describe the behaviour, and the runtime carries it out.",
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
        p![
            "An island's state is a small JSON object that starts as its props (",
            code!["{\"count\": 3}"],
            "). These attributes tie elements to that state; a ",
            code!["path"],
            " is the name of a value in it, such as ",
            code!["count"],
            " or ",
            code!["user.name"],
            ". When the state changes, every bound element updates.",
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
        p!["For behaviour beyond the declarative operations (drawing on a canvas, using a browser API, wrapping a \
             third-party widget), point an island at an ES module. Two files are involved. The first is the Rust \
             component, anywhere in the project; it renders the HTML on the server and names the module:",],
        pre![code![
            class("language-rust"),
            r#"#[client(module = "/_next-rust/client/islands/chart.js")]
pub fn Chart(points: Vec<(f64, f64)>) -> impl View {
    canvas![width(600), height(300), aria("label", "Chart")]
}"#,
        ],],
        p![
            "The second is the module itself, here ",
            code!["client/islands/chart.js"],
            ". It must export a function called ",
            code!["hydrate"],
            "; the runtime calls it once for every such island on the page, with the island's element and its props:",
        ],
        pre![code![
            class("language-js"),
            r#"// client/islands/chart.js – served from /_next-rust/client/
export function hydrate(element, props) {
  const ctx = element.querySelector("canvas").getContext("2d");
  // draw props.points …
}"#,
        ],],
        p![
            code!["client/"],
            " is an optional folder in the project root, next to ",
            code!["app/"],
            " and ",
            code!["src/"],
            ". It exists for exactly this purpose: browser scripts that belong to islands. A project with no module \
             islands has no ",
            code!["client/"],
            " folder and loses nothing. (Scripts for the whole site, loaded with a ",
            code!["<script>"],
            " tag, go in ",
            code!["assets/"],
            " instead; see ",
            a![href("/docs/styling-and-assets"), "Styling & assets"],
            ".) Files in the project's ",
            code!["client/"],
            " directory are served under ",
            code!["/_next-rust/client/"],
            " with the same path-traversal protection as ",
            code!["public/"],
            ".",
        ],
        p![
            "An island module is only ever imported from this origin: the runtime resolves ",
            code!["data-module"],
            " against the page's origin and refuses anything that is not a path on it (one leading ",
            code!["/"],
            ", no ",
            code!["//"],
            ", no scheme), with a console error and the island left with its declarative behaviour. The \
             attribute can be built from application data, and a foreign module would run in the page. Host \
             the module yourself — a copy under ",
            code!["client/"],
            " — rather than pointing an island at a CDN.",
        ],
        p![
            "The module does not have to be a file: ",
            code!["next_rust::client_script!(\"islands/chart.js\", r#\"…\"#);"],
            " in any Rust file of the project declares it, and ",
            code!["#[client(script = \"islands/chart.js\")]"],
            " is short for the ",
            code!["module"],
            " URL above. With ",
            code!["[build] inline_assets = true"],
            " the modules of a page are written into the document and nothing is fetched. See ",
            a![href("/docs/styling-and-assets#scripts-and-styles-in-rust"), "Scripts and styles in Rust"],
            ".",
        ],
        h3![id("wasm-islands"), a![class("anchor"), href("#wasm-islands"), "WASM islands"]],
        p!["There are two ways to run Rust in the browser with Next Rust, and they are easy to confuse. This \
             table is the whole story, including what does not exist yet:",],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["route"], th!["what it is"], th!["state today"]]],
                tbody![
                    tr![
                        td![strong!["Bring your own module"], " (this section)"],
                        td![
                            "You compile any Rust crate with ",
                            code!["wasm-bindgen"],
                            " yourself, put the output in ",
                            code!["client/"],
                            " and load it as a module island. Full ",
                            code!["web_sys"],
                            " access, any crates you like."
                        ],
                        td![
                            "Works. The build is manual: the CLI does not run ",
                            code!["cargo"],
                            " or ",
                            code!["wasm-bindgen"],
                            " for you, and you rerun them after every change."
                        ],
                    ],
                    tr![
                        td![
                            strong![code!["[build] wasm_islands"]],
                            " (",
                            a![href("#webassembly-islands"), "next section"],
                            ")"
                        ],
                        td![
                            "The build compiles the project's ",
                            code!["islands/"],
                            " crate into one small shared module. No ",
                            code!["wasm-bindgen"],
                            ", no dependencies, a fixed list of DOM operations."
                        ],
                        td![
                            "Works, and ",
                            code!["next-rust generate islands"],
                            " sets it up in one command. Still low-level: you write against a fixed DOM API, not a reactive framework."
                        ],
                    ],
                    tr![
                        td![strong!["Not available"]],
                        td![
                            "A Rust-side reactive UI library (signals, components that re-render in the browser), \
                             automated ",
                            code!["wasm-bindgen"],
                            " builds, running pages or server code in the browser."
                        ],
                        td!["Planned or undecided; see ", a![href("/docs/status"), "Status & roadmap"], "."],
                    ],
                ],
            ],
        ],
        p!["If you only want a widget to work, write the declarative island or a JavaScript module above: both are \
             smaller and simpler than either WebAssembly route. Reach for WebAssembly when you have real Rust logic \
             to reuse in the browser (a parser, a simulation, an editor core).",],
        p![
            "For the first route: a module island can be a Rust crate compiled to WebAssembly. Build it with the standard toolchain and point the island at the generated JavaScript glue. These terminal commands install the browser target, compile a crate called ",
            code!["my-widgets"],
            " and let ",
            code!["wasm-bindgen"],
            " write the JavaScript wrapper into ",
            code!["client/islands/"],
            ":",
        ],
        pre![code![
            class("language-sh"),
            r"rustup target add wasm32-unknown-unknown
cargo build -p my-widgets --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir client/islands target/wasm32-unknown-unknown/release/my_widgets.wasm",
        ],],
        p![
            "The crate (a separate one, here ",
            code!["my-widgets/src/lib.rs"],
            ") exports the same ",
            code!["hydrate"],
            " function a JavaScript module would:",
        ],
        pre![code![
            class("language-rust"),
            r"// my-widgets/src/lib.rs
#[wasm_bindgen]
pub fn hydrate(element: web_sys::Element, props: JsValue) { … }",
        ],],
        p!["And the component in your app points at the generated wrapper, exactly like a JavaScript module island:"],
        pre![code![
            class("language-rust"),
            r#"#[client(module = "/_next-rust/client/islands/my_widgets.js")]
pub fn Editor(doc: Document) -> impl View { … }"#,
        ],],
        p![
            strong!["Status:"],
            " the CLI doesn't automate this ",
            code!["wasm-bindgen"],
            " build, and there's no Rust-side reactive DOM library. See ",
            a![href("/docs/status"), "Status & roadmap"],
            ". The protocol (",
            code!["hydrate(element, props)"],
            " plus JSON props) is stable, so such a module keeps working unchanged. For islands the framework compiles and serves itself, see ",
            a![href("#webassembly-islands"), "WebAssembly islands"],
            " below.",
        ],
        h2![id("webassembly-islands"), a![class("anchor"), href("#webassembly-islands"), "WebAssembly islands"],],
        p![
            "This is the second route from the table above, switched on in the ",
            code!["[build]"],
            " section of ",
            code!["next-rust.toml"],
            ". It is off by default, and nothing else on this page depends on it. With ",
            code!["[build] wasm_islands = true"],
            ", island behaviour is written in Rust instead of JavaScript: the project's ",
            code!["islands/"],
            " crate is compiled to one WebAssembly module for the whole site, and ",
            code!["#[client(wasm = \"AuthSwitch\")]"],
            " points a component at the ",
            code!["hydrate_AuthSwitch"],
            " export in it. Nothing else about the component changes — it is rendered on the server as before, \
             and its props are still serialized into the HTML.",
        ],
        p![
            "The island itself depends on one crate, ",
            code!["next-rust-wasm"],
            ", and writes no boilerplate: ",
            code!["next_rust_wasm::islands! { hydrate_AuthSwitch => crate::auth_switch::hydrate }"],
            " emits the allocator, the panic handler and every export the shim calls. An island is a ",
            code!["fn(Ctx)"],
            " with elements, lists, strings read back out of the DOM, listeners as ordinary Rust closures, ",
            code!["async"],
            " over fetch, actions and server-sent events, timers, observers, storage, history and ",
            code!["Intl"],
            " formatting done by the browser rather than by linking a copy of ICU. Because a listener is a \
             closure, two instances of the same island on one page each keep their own state.",
        ],
        p!["Props arrive as a compact binary blob rather than JSON. Field names are hashed at compile time, so \
             no field name appears in the module and nothing in the browser parses JSON — a JSON parser in an \
             island measured 23 times the island's own size and put every field name back in the payload.",],
        h3![id("turning-wasm-on"), a![class("anchor"), href("#turning-wasm-on"), "Turning it on"]],
        p![
            "Two commands. The first is once per machine; the second writes the ",
            code!["islands/"],
            " crate, turns the key on in ",
            code!["next-rust.toml"],
            " and leaves a working island at ",
            code!["/wasm"],
            " to copy from. A brand new project can have both at once with ",
            code!["next-rust new my-app --wasm"],
            ".",
        ],
        pre![code![
            class("language-sh"),
            r"rustup target add wasm32-unknown-unknown   # once per machine
next-rust generate islands                 # add --ui for the shipped components too
next-rust doctor                           # checks every prerequisite
next-rust dev                              # http://localhost:3000/wasm",
        ],],
        p![
            "After that an island is two lines that have to agree. On the server, the component: ",
            code![r#"#[client(wasm = "Counter")]"#],
            ". In ",
            code!["islands/src/lib.rs"],
            ", the export: ",
            code!["hydrate_Counter => crate::counter"],
            ". The name is matched exactly and case-sensitively, and the build says so when the two drift apart \
             (",
            code!["NR0313"],
            ") — which matters more here than elsewhere, because the browser's own failure mode is silence.",
        ],
        pre![code![
            class("language-rust"),
            r#"// islands/src/lib.rs
#![no_std]
extern crate alloc;
use next_rust_wasm::prelude::*;

next_rust_wasm::islands! {
    hydrate_Counter => crate::counter,
}

pub fn counter(cx: Ctx) {
    let output = cx.root.query("[data-count]");
    let mut count = cx.props.i64(next_rust_wasm::k!("count")).unwrap_or(0);
    cx.root.query("[data-step]").on("click", move |_| {
        count += 1;
        output.set_text(&num::number_to_string(count as f64));
    });
}"#,
        ],],
        h3![id("what-wasm-costs"), a![class("anchor"), href("#what-wasm-costs"), "What it costs"]],
        p![
            "Measured on the project ",
            code!["next-rust new --wasm"],
            " generates — the counter above, nothing else — built with ",
            code!["next-rust build"],
            " and ",
            code!["wasm-opt -Oz"],
            ". Over the wire means brotli, which is how the module is served:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["what is in the module"], th!["raw"], th!["over the wire"]]],
                tbody![
                    tr![td!["one island, nothing else"], td!["6.2 kB"], td!["3.0 kB"]],
                    tr![
                        td!["the same island, plus the shipped components (", code!["--ui"], ")"],
                        td!["84.3 kB"],
                        td!["32.1 kB"],
                    ],
                    tr![
                        td!["the loader beside it (", code!["__nrw.js"], ", minified, one per site)"],
                        td!["15.7 kB"],
                        td!["5.0 kB"],
                    ],
                ],
            ],
        ],
        p![
            "So the first island on a site costs about 8 kB over the wire and each one after it costs its own \
             code — the counter itself is 3.5 kB of that, and the rest is the allocator, the callback table and \
             the string machinery every island shares. Turning the components on with ",
            code!["--ui"],
            " adds roughly 29 kB brotli in exchange for the component script it replaces. ",
            strong!["WebAssembly is not a size optimisation here"],
            ": it is a way to write browser behaviour in Rust, and the ",
            a![href("/docs/wasm-islands#sizes"), "sizes page"],
            " has the full comparison.",
        ],
        div![
            class("callout"),
            p![
                strong!["This is obfuscation, not invisibility, and not a security boundary."],
                " JavaScript the user cannot see is not possible, and this feature does not provide it. A \
                 readable JavaScript shim always remains, the ",
                code![".wasm"],
                " is downloadable and disassemblable, and every string literal in it sits in the data section in \
                 plain text. Props are still public and secrets still belong on the server.",
            ],
        ],
        div![
            class("callout"),
            p![
                strong!["Nothing here fails a build, so read the warnings."],
                " A missing target, a missing crate, an island the module does not export: each is a ",
                code!["cargo:warning"],
                " with an ",
                code!["NR03xx"],
                " code and the line to add, and the build then produces a working binary with the behaviour left \
                 out. That is deliberate — a machine without the wasm target should still be able to build the \
                 app — but it means the symptom in the browser is a component that renders, looks finished and \
                 does nothing. ",
                code!["next-rust doctor"],
                " repeats every one of those checks on demand.",
            ],
        ],
        p![
            a![href("/docs/wasm-islands"), strong!["WebAssembly islands"]],
            " has the whole story: the three configuration switches, how to write one, the ABI, what the build \
             does, how the module is delivered and cached, the CSP, measured sizes against the JavaScript it \
             replaces, and what a visitor gets when it does not run.",
        ],
        h2![id("serverclient-boundary"), a![class("anchor"), href("#serverclient-boundary"), "Server/client boundary"],],
        p!["The boundary is the line between what stays on your server and what a visitor can see. The rule of \
             thumb: anything that reaches the browser is public, whatever language it was written in.",],
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
        p!["Normally a click on a link throws the whole page away and loads a new one: a white flash, scroll \
             position lost, every script started again. Client-side navigation keeps the page alive and replaces \
             only what differs. You write nothing for it, and without JavaScript the same links work as ordinary links.",],
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
            " on ",
            code!["Link!"],
            ", or ",
            code!["prefetch(false)"],
            " on a plain anchor.",
        ],
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
        p![
            "Imperative API, for your own scripts in ",
            code!["client/"],
            " or ",
            code!["assets/"],
            ". ",
            code!["window.nextRust"],
            " exists on every page that has the runtime:",
        ],
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
        h3![id("forms"), a![class("anchor"), href("#forms"), "Forms never reload the page"]],
        p![
            "The same goes for forms, with no code: pressing Enter in a search box, picking a filter, posting to a ",
            code!["route.rs"],
            ". The runtime sends every form that goes to this site itself, and the browser only takes over where it has to. Without JavaScript each of them is still an ordinary HTML form.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["form"], th!["what happens"]]],
                tbody![
                    tr![
                        td![code!["method=\"get\""], " (or none), same origin"],
                        td![
                            "A client navigation to the URL the form builds: the fields are serialized as the browser would (the button that was pressed with its ",
                            code!["name"],
                            "/",
                            code!["value"],
                            ", ",
                            code!["formaction"],
                            " and ",
                            code!["formmethod"],
                            ", no disabled, unchecked or nameless fields, every option of a multiple select). History gets a new entry (the same URL again replaces it), shared layouts stay, the loading bar runs, the page is always fetched fresh, and the field that had focus keeps it, with the text and caret as typed."
                        ],
                    ],
                    tr![
                        td![code!["method=\"post\""], " to a ", code!["route.rs"]],
                        td![
                            "Posted with ",
                            code!["fetch"],
                            " (urlencoded, or multipart when the form says so, files included). A redirect is followed and shown as a navigation; an HTML answer, whatever its status, becomes the page; a ",
                            code!["Content-Disposition: attachment"],
                            " answer is saved as a file. Anything else (JSON, text, 204) reaches the form the way a server action's result does: ",
                            code!["data-nr-state"],
                            ", the ",
                            code!["nr:success"],
                            " / ",
                            code!["nr:error"],
                            " events (",
                            code!["detail.status"],
                            " on errors), ",
                            code!["data-nr-error"],
                            " elements filled from ",
                            code!["{\"errors\": {…}}"],
                            " or ",
                            code!["{\"error\": \"…\"}"],
                            ", and ",
                            code!["stay_on_success"],
                            " / ",
                            code!["action_result"],
                            " to show a result in place (the page refreshes otherwise). The CSRF token travels in ",
                            code!["x-csrf-token"],
                            ". If the request cannot be sent at all, the browser submits the form."
                        ],
                    ],
                    tr![
                        td![code!["action!(…)"]],
                        td![
                            "A server action, as described in ",
                            a![href("/docs/server-actions#forms"), "Server actions"],
                            "."
                        ],
                    ],
                    tr![
                        td![
                            "another origin, ",
                            code!["target"],
                            " other than ",
                            code!["_self"],
                            ", ",
                            code!["method=\"dialog\""],
                            ", ",
                            code!["enctype=\"text/plain\""],
                            ", ",
                            code!["reload(true)"],
                        ],
                        td![
                            "Left to the browser. ",
                            code!["reload(true)"],
                            " (",
                            code!["data-nr-reload"],
                            ", or ",
                            code!["data-nr-native"],
                            ") works on the form or on one submit button."
                        ],
                    ],
                ],
            ],
        ],
        p![
            "While a form is being sent it has ",
            code!["aria-busy=\"true\""],
            " and the button that sent it is disabled; submitting it again in that time (a double click, Enter held down) does nothing. Native validation (",
            code!["required"],
            ", ",
            code!["type=\"email\""],
            ") runs first, as always, unless the form has ",
            code!["novalidate"],
            ". A form with a single field and no button submits on Enter like any other, and a script calling ",
            code!["form.submit()"],
            " or ",
            code!["form.requestSubmit()"],
            " goes the same way. ",
            code!["replace(true)"],
            " and ",
            code!["scroll(false)"],
            " mean what they mean on a link.",
        ],
        p![
            "Two search forms as they would appear in a page such as ",
            code!["app/search/page.rs"],
            ": a plain one, and one that searches while the visitor types (debounced: it waits for a short pause \
             in typing instead of sending a request per keystroke):",
        ],
        pre![code![
            class("language-rust"),
            r##"// A search page. Enter searches without a reload; nothing else to write.
form![action("/search"), input![name("q"), r#type("search"), value(q)], button!["Search"]]

// Search as you type: sent 250 ms after the last keystroke, the request before is
// dropped, history is replaced, and only #results changes, so the input keeps
// its focus and caret.
form![action("/search"), live_search(250), swap_target("#results"),
    input![name("q"), r#type("search"), value(q)]]
div![id("results"), /* … */]"##,
        ],],
        p![
            code!["swap_target(\"#results\")"],
            " (",
            code!["data-nr-target"],
            ") works on any form: when the answer arrives, only that element is updated, in place: elements pair up by tag, ",
            code!["id"],
            " and ",
            code!["key(…)"],
            ", and what stays keeps its focus, caret, scroll position and listeners. If the element is missing on either side, the page is swapped as usual. ",
            code!["live_search"],
            " (",
            code!["data-nr-live"],
            ", on the form or on one field; ",
            code!["data-nr-debounce"],
            " overrides the delay) is for GET forms only.",
        ],
        p![
            "What is not changed: a ",
            code!["<button>"],
            " inside a form is a submit button unless it says ",
            code!["r#type(\"button\")"],
            ", as in HTML. Such a button no longer reloads the page, but it does submit the form. To leave every plain form to the browser, set ",
            code!["[client] enhance_forms = false"],
            " in ",
            code!["next-rust.toml"],
            "; server-action forms are not affected.",
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
        p!["The calls, as used inside an island module under ", code!["client/"], ":"],
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
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        ul![
            li![
                strong!["Is the runtime on the page?"],
                " View the page source and look for ",
                code!["/_next-rust/runtime.js"],
                ". A page with no internal link, form, UI component or island correctly has none.",
            ],
            li![
                strong!["Is navigation client-side?"],
                " Open the browser's developer tools, Network tab, and click an internal link. You should see one ",
                code!["fetch"],
                " request for the new page instead of a full document load, and the tab's loading spinner never starts.",
            ],
            li![
                strong!["Did the island hydrate?"],
                " Inspect it: the ",
                code!["<nr-island>"],
                " element carries ",
                code!["data-component"],
                " and ",
                code!["data-props"],
                ". If clicking does nothing, the Console tab shows why (a module that failed to load, a missing ",
                code!["hydrate"],
                " export).",
            ],
            li![
                strong!["Did the WebAssembly module build?"],
                " The Network tab shows a ",
                code![".wasm"],
                " file loading on a page that uses a ",
                code!["wasm"],
                " island. A ",
                code!["CompileError"],
                " in the console points at a hand-written CSP without ",
                code!["'wasm-unsafe-eval'"],
                ".",
            ],
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["Secrets in props."],
                " Island arguments are printed into the HTML. Pass ids and display values, never tokens or whole \
                 database records.",
            ],
            li![
                strong!["An island that is async, generic or a method."],
                " ",
                code!["#[client]"],
                " takes a plain, synchronous function whose arguments are simple names of ",
                code!["Serialize"],
                " types. Fetch data in the page and pass it in.",
            ],
            li![
                strong!["Pointing ", code!["module"], " at a CDN."],
                " The runtime refuses modules from other origins. Copy the file into ",
                code!["client/"],
                ".",
            ],
            li![
                strong!["Writing JavaScript to intercept form submits."],
                " Forms are already sent without a reload. Listen for ",
                code!["nr:success"],
                " / ",
                code!["nr:error"],
                " instead, or opt one form out with ",
                code!["reload(true)"],
                ".",
            ],
            li![
                strong!["A class or ", code!["data-*"], " name that only breaks in production."],
                " Release builds shorten class names, ",
                code!["data-*"],
                " attributes and CSS variables, and rewrite your scripts to match where they can see the name. A \
                 name assembled at runtime (",
                code!["\"data-\" + key"],
                ", a class from a template string) needs ",
                code!["[tailwind] keep_classes"],
                " or ",
                code!["[build] keep_names"],
                ". See ",
                a![href("/docs/tailwind#dynamic-classes"), "Classes built at runtime"],
                ".",
            ],
            li![
                strong!["Expecting WebAssembly to hide logic."],
                " It obscures, it does not protect. Anything a visitor must not do has to be refused by the server.",
            ],
            li![
                strong!["Logout links that seem to do nothing."],
                " A link to an endpoint that sets cookies and redirects should be a full load: ",
                code!["reload(true)"],
                ".",
            ],
        ],
    ]
}
