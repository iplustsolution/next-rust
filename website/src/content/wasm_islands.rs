//! The "WebAssembly islands" documentation page.
//!
//! The honest version: what a wasm island is, how to write one, what the ABI
//! is, what it costs in real bytes, how it is delivered and cached, and the
//! things it is repeatedly mistaken for. The sizes on this page are measured,
//! not estimated; when you change the pipeline, remeasure them.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "Normally the little bits of behaviour on a page — a menu that opens, a toggle that switches — are \
             written in JavaScript. This page is about writing them in Rust instead and shipping them as \
             WebAssembly: a compact binary the browser runs. The page still arrives finished from the server; \
             only the language of the interactive patches changes. A small piece of JavaScript always remains, \
             because WebAssembly cannot touch the page on its own.",
        ),
        p![
            "Islands, props and hydration are explained on ",
            a![href("/docs/client"), "Client & navigation"],
            ". This page is only about the WebAssembly route: switching it on, writing one, what the build does \
             with it, how it reaches the browser, and what it does and does not buy you.",
        ],
        div![
            class("callout"),
            p![
                strong!["Read this before you switch it on."],
                " A WebAssembly island is ",
                strong!["obfuscation, not invisibility, and not a security boundary"],
                ". JavaScript the user cannot see is not possible, and this feature does not provide it.",
            ],
            ul![
                li![
                    "WebAssembly has no access to the DOM. A JavaScript shim is therefore ",
                    strong!["always"],
                    " present and always readable. There is no configuration that removes it.",
                ],
                li![
                    "The ",
                    code![".wasm"],
                    " file is downloaded by the browser, so it can be saved and disassembled. ",
                    code!["wasm2wat"],
                    " (part of wabt) is enough, and it is a one-line command.",
                ],
                li![
                    "Every string literal in the module — selectors, class names, user-facing copy, URLs, error \
                     messages — sits in the data section ",
                    strong!["in plain text"],
                    ". ",
                    code!["strings islands.wasm"],
                    " prints them.",
                ],
                li!["Island props are serialized into the HTML exactly as before. They are public.",],
            ],
            p![
                strong!["Secrets belong on the server."],
                " Anything a visitor must not do, the server must refuse. Write the module as if the reader had \
                 the source, because in every way that matters they do.",
            ],
        ],
        h2![id("switches"), a![class("anchor"), href("#switches"), "The switches"]],
        p![
            "Three separate pieces of browser logic can be Rust instead of JavaScript, and each has its own key. \
             All three default to the JavaScript they have always shipped, so an existing project that upgrades \
             changes nothing until you edit ",
            code!["next-rust.toml"],
            ". They are independent: you can put your own islands in Rust while the framework runtime stays \
             JavaScript, or the other way round. The full reference is on ",
            a![href("/docs/configuration"), "Configuration"],
            ".",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["key"], th!["default"], th!["what it does"]]],
                tbody![
                    tr![
                        td![code!["[build] wasm_islands"]],
                        td![code!["false"]],
                        td![
                            "Compile the project's own ",
                            code!["islands/"],
                            " crate and serve ",
                            code!["#[client(wasm = \"…\")]"],
                            " islands from it.",
                        ],
                    ],
                    tr![
                        td![code!["[client] runtime"]],
                        td![code!["\"js\""]],
                        td![
                            code!["\"wasm\""],
                            " ships the framework's own browser runtime — client navigation, prefetching, the \
                             loading bar, form enhancement, hydration — as Rust compiled to WebAssembly instead \
                             of ",
                            code!["/_next-rust/runtime.js"],
                            ". What it does is identical either way.",
                        ],
                    ],
                    tr![
                        td![code!["[ui] script"]],
                        td![code!["\"js\""]],
                        td![
                            code!["\"wasm\""],
                            " takes the behaviour of the shipped components (select, date picker, toggles, \
                             dialogs) from Rust. Markup, stylesheet and keyboard contract are unchanged.",
                        ],
                    ],
                    tr![
                        td![code!["[build] wasm_opt"]],
                        td![code!["true"]],
                        td![
                            "Shrink every module with ",
                            code!["wasm-opt -Oz"],
                            " when it is on ",
                            code!["PATH"],
                            ". ",
                            code!["false"],
                            " ships what the linker wrote: faster builds, easier disassembly while debugging.",
                        ],
                    ],
                    tr![
                        td![code!["[build] wasm_keep_names"]],
                        td![code!["false"]],
                        td![
                            "Keep the wasm name section, so functions carry their Rust names in a profiler and \
                             in a stack trace. Costs a few kilobytes and undoes most of the obfuscation. Not to \
                             be confused with ",
                            code!["[build] keep_names"],
                            ", which is about ",
                            code!["minify_names"],
                            " and has nothing to do with WebAssembly.",
                        ],
                    ],
                ],
            ],
        ],
        p![
            "Any one of the first three being on is what makes a page compile a module, and therefore what \
             relaxes the Content Security Policy — see ",
            a![href("#csp"), "CSP"],
            " below.",
        ],
        h2![id("writing-one"), a![class("anchor"), href("#writing-one"), "Writing one"]],
        p![
            code!["islands/"],
            " is a folder you create yourself; ",
            code!["next-rust new"],
            " does not make one. With ",
            code!["wasm_islands = true"],
            " and no such folder, the build prints a warning and builds no WebAssembly — the site still works. \
             It is a separate crate because it is compiled for a different machine (the browser) than the rest \
             of your app (the server). It is ",
            code!["no_std"],
            ", has no dependencies, and builds as a ",
            code!["cdylib"],
            ":",
        ],
        pre![code![
            class("language-toml"),
            r#"# islands/Cargo.toml
[package]
name = "islands"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "z"
panic = "abort"
lto = true
strip = true"#,
        ],],
        p![
            "The file below is ",
            code!["islands/src/lib.rs"],
            ", complete. It toggles the class ",
            code!["is-open"],
            " on the island when it is clicked. Read it as: declare the one imported function, name the two \
             operations used, register a click listener when the island is hydrated, react when the event comes \
             back.",
        ],
        pre![code![
            class("language-rust"),
            r#"// islands/src/lib.rs
#![no_std]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// The shim's opcode table: the module's only import.
#[link(wasm_import_module = "e")]
unsafe extern "C" {
    fn h(op: u32, a: u32, b: u32, c: u32, d: u32, f: u32) -> u32;
}

const LISTEN: u32 = 1; // listen on a node; the callback id comes back to on_event
const TOGGLE: u32 = 2; // classList.toggle(node, class, on)

fn call(op: u32, node: u32, text: &str, d: u32) -> u32 {
    unsafe { h(op, node, text.as_ptr() as u32, text.len() as u32, d, 0) }
}

const CLICKED: u32 = 1;
static mut ROOT: u32 = 0;

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn hydrate_AuthSwitch(root: u32) {
    unsafe { ROOT = root };
    call(LISTEN, root, "click", CLICKED);
}

#[unsafe(no_mangle)]
pub extern "C" fn on_event(cb: u32, _detail: u32) {
    if cb == CLICKED {
        call(TOGGLE, unsafe { ROOT }, "is-open", 1);
    }
}"#,
        ],],
        p![
            "The call site is an ordinary island in your app's own code (for example ",
            code!["src/components/auth_switch.rs"],
            "); only the attribute differs:",
        ],
        pre![code![
            class("language-rust"),
            r#"#[client(wasm = "AuthSwitch")]
pub fn AuthSwitch(signed_in: bool) -> impl View {
    nav![
        class("auth"),
        a![class(if signed_in { "is-on" } else { "is-off" }), href("/account"), "Account"],
        a![href("/login"), "Log in"],
    ]
}"#,
        ],],
        p![
            "The name must be an identifier, and it names the export: ",
            code!["wasm = \"AuthSwitch\""],
            " binds to ",
            code!["hydrate_AuthSwitch"],
            ". You declare the island list once, in Rust — the build reads the names back out of the compiled \
             module's export section, so there is no manifest to keep in sync.",
        ],
        p!["One-time setup, then build as usual:"],
        pre![code![
            class("language-sh"),
            r"rustup target add wasm32-unknown-unknown   # required
brew install binaryen                      # optional: provides wasm-opt
next-rust build",
        ],],
        h2![id("abi"), a![class("anchor"), href("#abi"), "The ABI"]],
        p![
            "The module imports exactly one function, ",
            code!["e.h(op, a, b, c, d, f)"],
            ", and exports ",
            code!["memory"],
            ", ",
            code!["alloc(len) -> ptr"],
            ", optionally ",
            code!["set_prop(i, ptr, len)"],
            ", one ",
            code!["hydrate_<Name>(root)"],
            " per island, and one shared ",
            code!["on_event(cb_id, detail)"],
            ". Strings are passed as a pointer and a length into the module's linear memory; node handles are \
             small integers, and ",
            code!["0"],
            " means \"nothing\".",
        ],
        p![
            "The shim is a switch statement over opcodes and nothing else, deliberately: no island behaviour is \
             readable in it. ",
            strong!["The order of the table is the ABI"],
            " — the module calls operations by index. Append, never reorder.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["op"], th!["operation"]]],
                tbody![
                    tr![
                        td!["1"],
                        td![code!["addEventListener"], " (the callback id returns through ", code!["on_event"], ")"]
                    ],
                    tr![td!["2"], td![code!["classList.toggle"]]],
                    tr![td!["3"], td![code!["querySelector"]]],
                    tr![td!["4"], td![code!["setAttribute"]]],
                    tr![td!["5"], td![code!["textContent"]]],
                    tr![td!["6"], td![code!["nextRust.action"]]],
                    tr![td!["7"], td![code!["nextRust.navigate"]]],
                    tr![td!["8"], td![code!["closest"]]],
                    tr![td!["9"], td![code!["closest"], " from the current event's target"]],
                    tr![td!["10"], td!["the event's modifier and button flags"]],
                    tr![td!["11"], td!["compare ", code!["event.key"]]],
                    tr![td!["12"], td![code!["preventDefault"]]],
                    tr![td!["13"], td![code!["focus"]]],
                    tr![td!["14"], td!["set an IDL boolean (", code!["hidden"], ", ", code!["inert"], ")"]],
                    tr![td!["15"], td![code!["history.replaceState"], " to an element's ", code!["href"]]],
                ],
            ],
        ],
        p![
            "Because ",
            code!["nextRust.action"],
            " and ",
            code!["navigate"],
            " are in the table, an island can submit a server action or navigate without any JavaScript of its \
             own. Props reach the island as plain strings, pushed in by the shim with ",
            code!["set_prop"],
            " before ",
            code!["hydrate_<Name>"],
            " is called; the island never parses JSON. That is a deliberate size decision — see ",
            a![href("#sizes"), "What it costs"],
            ".",
        ],
        h2![id("the-build"), a![class("anchor"), href("#the-build"), "What the build does"]],
        p![
            "Compilation happens in the project's build script, so ",
            code!["next-rust dev"],
            " and a plain ",
            code!["cargo build"],
            " do it too — there is no separate wasm step to remember and none to forget in CI.",
        ],
        p![
            strong!["Your islands and the UI kit compile into one module"],
            " — ",
            code!["/_next-rust/client/__nrw/islands.wasm"],
            " — not one per crate. Your ",
            code!["islands/"],
            " crate and the UI kit link against the same base crate, and separate modules would duplicate the \
             allocator, the callback table and the string machinery in every copy. One module means one \
             download, one compile and one cache entry, and it is why the per-island cost below is a number in \
             a report rather than a file on the network.",
        ],
        p![
            "The browser runtime is the exception. ",
            code![r#"[client] runtime = "wasm""#],
            " is served from ",
            code!["/_next-rust/runtime.wasm"],
            ", a module the framework ships already built, so a page that turns on both downloads two modules \
             and the machinery they share twice. It carries its own allocator and panic handler, which is why \
             it cannot be linked into the crate above. Turn it on for what it is — the browser runtime in Rust \
             — and not to save a request.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["switch"], th!["what it adds to the module"]]],
                tbody![
                    tr![td![code!["[build] wasm_islands"]], td![code!["<project>/islands/"], ", your own crate"],],
                    tr![
                        td![code!["[client] runtime = \"wasm\""]],
                        td![code!["next-rust-runtime"], ", shipped with the framework"],
                    ],
                    tr![
                        td![code!["[ui] script = \"wasm\""]],
                        td![code!["next-rust-ui"], "'s component logic, shipped with the framework"],
                    ],
                ],
            ],
        ],
        p![
            "The framework's contributions come from the framework itself, so there is nothing to add to your \
             workspace. Your own ",
            code!["islands/"],
            " crate is yours: a compile error in it is a hard build error, exactly like a compile error anywhere \
             else in your code.",
        ],
        p![strong!["Everything else about WebAssembly degrades to a warning rather than failing your build:"]],
        ul![
            li![
                code!["wasm32-unknown-unknown"],
                " is not installed → a warning naming the exact ",
                code!["rustup target add wasm32-unknown-unknown"],
                " to run, and that module is skipped. Pages render and are usable; the islands in it have no \
                 behaviour. Your build still produces a binary.",
            ],
            li![
                code!["wasm-opt"],
                " is missing or fails → a warning with the unoptimized size and how to install binaryen. The \
                 unoptimized module works; it is roughly 10–15% larger. With ",
                code!["[build] wasm_opt = false"],
                " the step is skipped silently, because then you asked for it.",
            ],
            li![
                code!["wasm_islands = true"],
                " with no ",
                code!["islands/"],
                " crate → a warning naming the path it looked at and the ",
                code!["crate-type"],
                " it needs.",
            ],
        ],
        p!["The reasoning is that WebAssembly is an optimisation, not a correctness requirement. A machine \
             without the target — a fresh CI runner, a colleague's laptop — should still produce a working \
             deployment and tell you what it left out, rather than stopping.",],
        h3![id("size-report"), a![class("anchor"), href("#size-report"), "The size report"]],
        p![
            code!["next-rust build"],
            " prints a WebAssembly section: the module raw and brotli-compressed, the shim's own raw, minified \
             and brotli sizes beside it, and then a line per island. Each island gets two numbers — ",
            strong!["exclusive"],
            ", the code only that island reaches, and ",
            strong!["reachable"],
            ", including the code it shares with the others. Exclusive is what you save by deleting an island; \
             reachable is what the first island on a page costs. The same figures go into the JSON build report \
             under ",
            code!["wasm"],
            ", so CI can fail a pull request that pushes the module past a budget. Watch the brotli column: that \
             is what crosses the network.",
        ],
        h3![id("dev"), a![class("anchor"), href("#dev"), "In development"]],
        p![
            code!["next-rust dev"],
            " watches the islands crate and rebuilds the module when it changes, like any other source file. A \
             wasm rebuild is a full ",
            code!["--release"],
            " compile of that crate, so it is slower than a page edit — keep the crate small and dependency-free \
             and it stays in the low seconds. Modules are served ",
            code!["no-cache"],
            " in development, so a reload always gets the new one.",
        ],
        h2![id("delivery"), a![class("anchor"), href("#delivery"), "How it reaches the browser"]],
        p!["The module is a fetch, not a script, and it is on the critical path for hydration. So:"],
        ul![
            li![
                "It is served as ",
                code!["application/wasm"],
                ". This is not cosmetic: a browser refuses to stream-compile a module that arrives as anything \
                 else, and silently falls back to downloading it whole first.",
            ],
            li![
                "It is precompressed. Brotli and gzip copies are built at build time for release binaries and \
                 made once per process in development, with ",
                code!["vary: accept-encoding"],
                ". A ",
                code![".wasm"],
                " compresses well — roughly 2.5–3× — so this is the single biggest delivery win.",
            ],
            li![
                "It carries the build's content hash in the URL (",
                code!["?v=…"],
                ") and is then served ",
                code!["public, max-age=31536000, immutable"],
                ". A new build changes the URL, so nothing ever has to revalidate. Requested without the hash it \
                 falls back to an ",
                code!["ETag"],
                " and an hour of freshness.",
            ],
            li![
                "The page emits ",
                code!["<link rel=\"preload\" as=\"fetch\" crossorigin>"],
                " for the module it will use, so the download starts while the HTML is still being parsed \
                 instead of after the shim has run. ",
                code!["crossorigin"],
                " is required even same-origin here: without it the preload and the shim's own ",
                code!["fetch"],
                " use different caches and the module is downloaded twice.",
            ],
        ],
        div![
            class("callout"),
            p![
                strong![code!["[build] inline_assets = true"], " and WebAssembly."],
                " Inlining writes scripts into the document so a page loads with no ",
                code![".js"],
                " request. The shim is a script, so it is inlined like any other. The ",
                code![".wasm"],
                " is ",
                strong!["not"],
                " inlined and stays a separate fetch: a base64 data URI would be about a third larger, could not \
                 be stream-compiled, and would be re-downloaded with every HTML response instead of being cached \
                 forever. The preload link matters more, not less, when the shim is inline.",
            ],
        ],
        h2![id("csp"), a![class("anchor"), href("#csp"), "CSP"]],
        p![
            "Browsers refuse to compile a WebAssembly module unless ",
            code!["script-src"],
            " includes ",
            code!["'wasm-unsafe-eval'"],
            ". Next Rust adds that one keyword automatically, and only when one of the three switches above is \
             on. It permits ",
            code!["WebAssembly.instantiate"],
            " and nothing else: it does not bring back ",
            code!["eval"],
            ", and it does not permit inline scripts. Nonces, ",
            code!["'strict-dynamic'"],
            " and every other directive are untouched.",
        ],
        p![
            "It composes with ",
            code!["[analytics]"],
            ", which widens the same directive with the tag origins: the origins are added while the policy is \
             built, the keyword when the response is written, and neither undoes the other. A policy you wrote \
             yourself with no ",
            code!["script-src"],
            " is left exactly as you wrote it. With the ",
            code!["strict"],
            " preset and Tag Manager configured, the result is:",
        ],
        pre![code![
            class("language-text"),
            "script-src 'self' 'nonce-…' 'strict-dynamic' https://www.googletagmanager.com 'wasm-unsafe-eval';",
        ],],
        h2![id("fallback"), a![class("anchor"), href("#fallback"), "What happens when it does not run"]],
        p!["Every one of these leaves the server-rendered markup in place. That is the whole point of rendering \
             on the server first: WebAssembly adds behaviour to a page that already exists, so losing the \
             behaviour costs the behaviour and nothing else.",],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["situation"], th!["what the visitor gets"]]],
                tbody![
                    tr![
                        td!["A browser with no WebAssembly support"],
                        td!["The shim throws while booting, the hydrate promise rejects, the markup stays. Links \
                             and forms still work, because those are the framework runtime's job, not the \
                             island's.",],
                    ],
                    tr![
                        td![
                            "The module 404s, or is served with the wrong media type, or fails to compile \
                             (missing ",
                            code!["'wasm-unsafe-eval'"],
                            ")",
                        ],
                        td!["A clean rejection. Nothing is torn down, no markup is removed, and the browser \
                             console says why.",],
                    ],
                    tr![
                        td!["The named export is missing (a typo in ", code!["wasm = \"…\""], ")"],
                        td!["The shim finds no function and returns. The component renders and does nothing."],
                    ],
                    tr![
                        td![strong!["A visitor with JavaScript switched off entirely"]],
                        td![
                            "The server-rendered HTML, its CSS, and the declarative bindings. Navigation is full \
                             page loads, and forms post and redirect — which is why server actions are built to \
                             work without JavaScript, and why the ",
                            code!["_redirect"],
                            " field on an action form matters.",
                        ],
                    ],
                ],
            ],
        ],
        p![
            "So the floor is the same floor the framework always had: ",
            strong!["a page that works as HTML"],
            ". Design islands as an enhancement of something already usable, and none of the rows above is an \
             outage. Design a page that only works once the module has hydrated, and every row above is.",
        ],
        h2![id("sizes"), a![class("anchor"), href("#sizes"), "What it costs"]],
        p![
            "Measured on Apple silicon with rustc 1.98, wasm-opt 132 and Chromium, on this site's own ",
            code!["AuthSwitch"],
            " island. These are real numbers from a real build, not estimates:",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["what"], th!["size"]]],
                tbody![
                    tr![
                        td!["minimal \"toggle a class\" island, ", code!["no_std"]],
                        td!["281 B raw / 253 B gzip — the floor"],
                    ],
                    tr![
                        td![code!["AuthSwitch"], " ported from JS to Rust, after ", code!["wasm-opt -Oz"]],
                        td!["1647 B raw / 865 B brotli"],
                    ],
                    tr![td!["the same island as it ships in JavaScript"], td!["1202 B brotli"]],
                    tr![
                        td!["the shared JS shim (a switch over opcodes, no business logic)"],
                        td!["1657 B raw / 720 B brotli, once per page"],
                    ],
                    tr![
                        td!["three simple islands compiled into one shared module"],
                        td!["2058 B raw / 1037 B brotli — about 100 B brotli per island after the first"],
                    ],
                    tr![
                        td![strong!["one small island: wasm + shim against the JS it replaces"]],
                        td![strong!["865 + 720 ≈ 1585 B brotli against 1202 B — wasm is larger"]],
                    ],
                    tr![td![code!["wasm-opt -Oz"], " on the real module"], td!["1844 B → 1596 B, about 13%"]],
                    tr![td!["hydration in Chromium"], td!["4.0 ms from calling hydrate to the island being live"]],
                ],
            ],
        ],
        p![
            "That sixth row is the honest trade-off, and it is the one worth arguing with. ",
            strong!["For a page with one small island, WebAssembly is bigger than the JavaScript it replaces."],
            " It starts to win around the second or third island on a page, because the 720 B shim and the \
             module's shared Rust machinery are paid once. If your reason for turning this on is payload size, \
             measure your own pages first — the size report exists for exactly that.",
        ],
        div![
            class("callout"),
            p![
                strong!["Keep the crate empty."],
                " Linking ",
                code!["serde_json"],
                " into an island to deserialize props cost ",
                strong!["38,109 B raw / 18,563 B gzip"],
                " — larger than every JavaScript island on this site put together — and it leaked the prop \
                 struct's name and every field name into the binary. That is why props are parsed by the shim \
                 and pushed in as plain strings, and why the islands crate is ",
                code!["no_std"],
                " with a bump allocator and no dependencies. A dependency in this crate is bytes every visitor \
                 downloads.",
            ],
        ],
        h2![id("what-ships"), a![class("anchor"), href("#what-ships"), "What is actually in the module"]],
        p![
            code!["wasm2wat"],
            " and ",
            code!["strings"],
            " on the shipped module show no source paths, no crate or symbol names, and no ",
            code!["name"],
            " or ",
            code!["producers"],
            " custom sections (",
            code!["strip = true"],
            " plus ",
            code!["wasm-opt --strip-*"],
            "). What they ",
            em!["do"],
            " show is the intended exports — ",
            code!["alloc"],
            ", ",
            code!["hydrate_<Name>"],
            ", ",
            code!["on_event"],
            ", ",
            code!["memory"],
            " — and every string literal, in full. Turning on ",
            code!["[build] wasm_keep_names"],
            " adds the Rust function names back on top of that, which is useful in a profiler and unhelpful \
             everywhere else.",
        ],
        p![
            "Existing ",
            code!["#[client(script = …)]"],
            " and ",
            code!["#[client(module = …)]"],
            " JavaScript islands keep working unchanged alongside all of this. Nothing here is a migration you \
             have to finish.",
        ],
    ]
    .into_node()
}
