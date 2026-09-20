//! The "diagnostics" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Build-time problems are reported with a stable code, the files involved, an explanation and a suggested fix. Errors fail ",
            code!["next-rust build"],
            ", ",
            code!["check"],
            " and ",
            code!["cargo build"],
            " (from ",
            code!["build.rs"],
            "). Warnings are printed and don't block the build.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "A diagnostic is the framework telling you about a mistake before the site ever runs, the way a spell checker underlines a word before you send the letter. Each kind of mistake has a fixed number (",
                code!["NR0201"],
                ") so you can look it up here or search for it. Most frameworks discover these problems when a visitor opens the page; here a misnamed file or a bad setting stops the build instead.",
            ],
        ],
        h2![id("reading-a-diagnostic"), a![class("anchor"), href("#reading-a-diagnostic"), "Reading a diagnostic"]],
        p![
            "Every message has the same four parts. This is what the terminal shows for ",
            code!["base_path = \"docs/\""],
            " in ",
            code!["next-rust.toml"],
            ":",
        ],
        pre![code![
            class("language-text"),
            r#"error[NR0005]: Invalid base path

  `app.base_path = "docs/"`

  help: use a leading slash and no trailing slash, e.g. `/docs`"#,
        ]],
        ul![
            li![
                strong!["Level and code: "],
                code!["error"],
                " stops the build, ",
                code!["warning"],
                " does not. The code is stable across versions."
            ],
            li![
                strong!["Location: "],
                "a ",
                code!["-->"],
                " line with the file (and line, when known) for problems in a file."
            ],
            li![strong!["Message: "], "what was found."],
            li![strong!["help: "], "the fix. Read this line first; it usually names the exact change."],
        ],
        p![
            "The first two digits tell you where to look: ",
            code!["NR00xx"],
            " is the settings file (",
            code!["next-rust.toml"],
            "), ",
            code!["NR01xx"],
            " is the names and arrangement of folders under ",
            code!["app/"],
            ", ",
            code!["NR02xx"],
            " is the Rust code inside the special files (",
            code!["page.rs"],
            ", ",
            code!["layout.rs"],
            ", ",
            code!["route.rs"],
            ", …), and ",
            code!["NR03xx"],
            " is WebAssembly — the only family that can be true of a build that succeeded.",
        ],
        p!["Where diagnostics appear:"],
        ul![
            li![
                code!["next-rust dev"],
                ": in the terminal and as an overlay in the browser; the last working version keeps serving."
            ],
            li![code!["next-rust check"], " and ", code!["next-rust doctor"], ": on demand, without building."],
            li![
                code!["next-rust build"],
                " and plain ",
                code!["cargo build"],
                ": the build script (",
                code!["build.rs"],
                ") prints them and an error fails the build."
            ],
        ],
        h2![id("configuration-nr00xx"), a![class("anchor"), href("#configuration-nr00xx"), "Configuration (NR00xx)"],],
        p![
            "Problems in ",
            code!["next-rust.toml"],
            " (or ",
            code!["next-rust.json"],
            "), the settings file in the project root. The fix is always an edit to that file, or creating the folder it names.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["code"], th!["level"], th!["meaning"]]],
                tbody![
                    tr![td!["NR0001"], td!["error"], td!["the configured app directory does not exist"]],
                    tr![td!["NR0002"], td!["error"], td!["the app path is not a directory"]],
                    tr![td!["NR0003"], td!["error"], td![code!["[api] directory"], " does not exist"]],
                    tr![td!["NR0004"], td!["error"], td![code!["[api] prefix"], " does not start with ", code!["/"]],],
                    tr![
                        td!["NR0005"],
                        td!["error"],
                        td!["invalid ", code!["[app] base_path"], " (needs a leading and no trailing slash)"],
                    ],
                    tr![
                        td!["NR0006"],
                        td!["error"],
                        td!["a redirect ", code!["source"], " does not start with ", code!["/"]],
                    ],
                    tr![td!["NR0007"], td!["warning"], td!["unknown ", code!["[logging] level"]]],
                    tr![
                        td!["NR0008"],
                        td!["error"],
                        td!["a file in ", code!["[tailwind] stylesheets"], " does not exist"],
                    ],
                    tr![
                        td!["NR0009"],
                        td!["error"],
                        td![
                            "a Rust file in ",
                            code!["[tailwind] stylesheets"],
                            " has no ",
                            code!["stylesheet!(…)"],
                            " at module level"
                        ],
                    ],
                    tr![
                        td!["NR0010"],
                        td!["error"],
                        td![
                            "a ",
                            code!["[verification]"],
                            " value is malformed: a whole ",
                            code!["<meta>"],
                            " tag instead of its ",
                            code!["content"],
                            ", a ",
                            code!["google_file"],
                            " that is not ",
                            code!["google….html"],
                            ", a bad ",
                            code!["ads.txt"],
                            " record, app-link JSON that does not parse"
                        ],
                    ],
                    tr![
                        td!["NR0011"],
                        td!["error"],
                        td![
                            "an ",
                            code!["[analytics]"],
                            " id has the wrong shape: a Universal Analytics ",
                            code!["UA-…"],
                            " id (use the GA4 ",
                            code!["G-…"],
                            " id), an id under the wrong key, a script URL that is not ",
                            code!["https://"],
                            ", an ",
                            code!["exclude_paths"],
                            " pattern without a leading ",
                            code!["/"]
                        ],
                    ],
                ],
            ],
        ],
        p!["Parse errors (bad TOML/JSON, unknown keys) report the file, line and key."],
        p![
            "An unknown key is always an error, never a warning: a misspelled ",
            code!["prot = 3000"],
            " would otherwise be ignored silently and the site would start on the wrong port. It has no NR code because the file could not be read at all. Check the spelling and the section against the ",
            a![href("/docs/configuration#reference"), "configuration reference"],
            ".",
        ],
        h2![id("routing-nr01xx"), a![class("anchor"), href("#routing-nr01xx"), "Routing (NR01xx)"]],
        p![
            "Problems with the folders and file names under ",
            code!["app/"],
            ", where each folder is a piece of the URL (",
            a![href("/docs/routing"), "routing"],
            "). The fix is renaming, moving or deleting a folder or file; no code changes. ",
            code!["next-rust routes --tree"],
            " shows the tree as the framework sees it.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["code"], th!["level"], th!["meaning"]]],
                tbody![
                    tr![
                        td!["NR0101"],
                        td!["error / warning"],
                        td![
                            code!["page.rs"],
                            " and ",
                            code!["page.html"],
                            " in one directory (a warning when ",
                            code!["html_precedence"],
                            " picks one)",
                        ],
                    ],
                    tr![td!["NR0102"], td!["error"], td!["two routes resolve to the same URL"]],
                    tr![td!["NR0103"], td!["error"], td!["different parameter names at the same position"]],
                    tr![td!["NR0104"], td!["error"], td!["a page and an API route resolve to the same URL"]],
                    tr![td!["NR0105"], td!["error"], td!["catch-all segment is not the last segment"]],
                    tr![td!["NR0106"], td!["error"], td!["optional catch-all overlaps a route at its parent path"],],
                    tr![td!["NR0107"], td!["error"], td!["duplicate parameter name within one route"]],
                    tr![
                        td!["NR0108"],
                        td!["error"],
                        td![
                            "invalid directory name (",
                            code!["user[id]"],
                            ", ",
                            code!["[a-b]"],
                            ", ",
                            code!["()"],
                            ", ",
                            code!["@a-b"],
                            ", …)",
                        ],
                    ],
                    tr![td!["NR0109"], td!["error"], td!["a directory could not be read (permissions)"]],
                    tr![td!["NR0110"], td!["warning"], td!["symbolic link loop skipped"]],
                    tr![
                        td!["NR0111"],
                        td!["error"],
                        td![
                            "root-only file (",
                            code!["global-error.rs"],
                            ", ",
                            code!["sitemap.rs"],
                            ", ",
                            code!["robots.rs"],
                            ") in a subdirectory",
                        ],
                    ],
                    tr![td!["NR0112"], td!["warning"], td!["non-route special file inside ", code!["[api] directory"]],],
                    tr![td!["NR0113"], td!["warning"], td!["non-UTF-8 file or directory name skipped"]],
                    tr![td!["NR0114"], td!["warning"], td!["broken symbolic link"]],
                    tr![td!["NR0115"], td!["error"], td!["intercepting route reaches above the root, or is nested"],],
                    tr![
                        td!["NR0116"],
                        td!["error"],
                        td![code!["[...x]"], " and ", code!["[[...x]]"], " at the same position"],
                    ],
                    tr![
                        td!["NR0117"],
                        td!["warning"],
                        td!["slot directory with no ", code!["page.rs"], "/", code!["default.rs"]],
                    ],
                    tr![td!["NR0118"], td!["warning"], td!["no routes found"]],
                    tr![
                        td!["NR0119"],
                        td!["warning"],
                        td![code!["route.rs"], " inside an intercepting route (ignored)"],
                    ],
                    tr![
                        td!["NR0120"],
                        td!["warning"],
                        td!["file name differs from a special file only by case (", code!["Page.rs"], ")"],
                    ],
                    tr![
                        td!["NR0121"],
                        td!["warning"],
                        td![
                            "a Next.js-style file (",
                            code!["page.tsx"],
                            ", ",
                            code!["layout.js"],
                            ", …) — use ",
                            code![".rs"],
                        ],
                    ],
                ],
            ],
        ],
        h2![
            id("source-analysis-nr02xx"),
            a![class("anchor"), href("#source-analysis-nr02xx"), "Source analysis (NR02xx)"],
        ],
        p![
            "Problems inside a special file: the build script reads the Rust source and checks that the file offers what its name promises. A ",
            code!["page.rs"],
            " must contain a public function named ",
            code!["Page"],
            ", a ",
            code!["route.rs"],
            " at least one HTTP method such as ",
            code!["GET"],
            ", and so on. The names are fixed and case-sensitive.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["code"], th!["level"], th!["meaning"]]],
                tbody![
                    tr![
                        td!["NR0200"],
                        td!["error"],
                        td!["syntax error in a special file (with ", code!["file:line:column"], ") or unreadable file",],
                    ],
                    tr![
                        td!["NR0201"],
                        td!["error"],
                        td![
                            "a special file lacks its required ",
                            code!["pub fn"],
                            " (e.g. ",
                            code!["Page"],
                            ", ",
                            code!["Layout"],
                            ")",
                        ],
                    ],
                    tr![td!["NR0203"], td!["error"], td![code!["route.rs"], " exports no HTTP method handler"],],
                    tr![
                        td!["NR0204"],
                        td!["error"],
                        td![
                            code!["Loading"],
                            ", ",
                            code!["ErrorBoundary"],
                            " or ",
                            code!["GlobalError"],
                            " is ",
                            code!["async"],
                        ],
                    ],
                    tr![td!["NR0205"], td!["error"], td!["invalid signature for an API handler or middleware"],],
                    tr![
                        td!["NR0206"],
                        td!["error"],
                        td![
                            "an argument that cannot be supplied there (",
                            code!["Children"],
                            " in a page, ",
                            code!["Data<T>"],
                            " without ",
                            code!["load"],
                            ", …)",
                        ],
                    ],
                    tr![td!["NR0210"], td!["error"], td!["a server action with more than two arguments"]],
                    tr![
                        td!["NR0211"],
                        td!["error"],
                        td!["a private module declared by a route file holds server actions"]
                    ],
                    tr![
                        td!["NR0212"],
                        td!["error"],
                        td![
                            code!["Rendering::Partial"],
                            " on a page whose shell cannot be static: a request-bound argument outside a ",
                            code!["dynamic"],
                            " hole, or an intercepting route"
                        ]
                    ],
                    tr![
                        td!["NR0213"],
                        td!["error"],
                        td![
                            "two ",
                            code!["client_script!"],
                            " (or ",
                            code!["asset_script!"],
                            ") declarations have the same path"
                        ]
                    ],
                    tr![
                        td!["NR0214"],
                        td!["error"],
                        td![
                            "a ",
                            code!["client_script!"],
                            " / ",
                            code!["asset_script!"],
                            " path that cannot be a file name, a URL and a JavaScript registry key at once (",
                            code![".."],
                            ", an absolute path, quotes, angle brackets, control characters)"
                        ]
                    ],
                ],
            ],
        ],
        h2![id("webassembly-nr03xx"), a![class("anchor"), href("#webassembly-nr03xx"), "WebAssembly (NR03xx)"]],
        p![
            "These are different from every other family in one way that matters: ",
            strong!["none of them fails a build"],
            ". A missing toolchain component is the machine's problem, not the code's, so the build produces a \
             working binary with the WebAssembly behaviour left out — and the symptom in the browser is a \
             component that renders, looks finished and does nothing at all. There is no error in the console \
             and no failed request to find it by. That is why each of these carries the key that is on, what it \
             produced, and the exact line to write.",
        ],
        p![
            "They appear as ",
            code!["cargo:warning"],
            " lines during a build, flattened onto one line each, and in full from ",
            code!["next-rust doctor"],
            ", which repeats every check that needs no compiler. Run the doctor when a page looks right and does \
             nothing.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["code"], th!["level"], th!["meaning"]]],
                tbody![
                    tr![
                        td!["NR0301"],
                        td!["warning"],
                        td![
                            "a switch that needs the ",
                            code!["islands/"],
                            " crate is on and there is none — run ",
                            code!["next-rust generate islands"]
                        ],
                    ],
                    tr![
                        td!["NR0302"],
                        td!["warning"],
                        td![
                            "the islands crate has no ",
                            code![r#"[lib] crate-type = ["cdylib"]"#],
                            ", so cargo builds an rlib and there is no module"
                        ],
                    ],
                    tr![
                        td!["NR0303"],
                        td!["warning"],
                        td![
                            "the ",
                            code!["wasm32-unknown-unknown"],
                            " target is not installed (",
                            code!["rustup target add wasm32-unknown-unknown"],
                            ")"
                        ],
                    ],
                    tr![
                        td!["NR0304"],
                        td!["warning"],
                        td![
                            "a framework WebAssembly crate has no source beside this build — depend on next-rust \
                             by path, or point ",
                            code!["NEXT_RUST_WASM_*"],
                            " at a prebuilt module"
                        ],
                    ],
                    tr![
                        td!["NR0305"],
                        td!["warning"],
                        td![
                            "a ",
                            code!["NEXT_RUST_WASM_*"],
                            " override points at neither a ",
                            code![".wasm"],
                            " file nor a crate directory"
                        ],
                    ],
                    tr![
                        td!["NR0306"],
                        td!["warning"],
                        td![
                            "the islands crate is a workspace member, so cargo ignores its ",
                            code!["[profile.release]"],
                            " and the module is about twice the size — give it an empty ",
                            code!["[workspace]"],
                            " table"
                        ],
                    ],
                    tr![
                        td!["NR0307"],
                        td!["warning"],
                        td![
                            code!["wasm-opt"],
                            " is not on PATH, so the module ships as the linker wrote it (size only)"
                        ],
                    ],
                    tr![
                        td!["NR0308"],
                        td!["warning"],
                        td![
                            code!["wasm-opt"],
                            " ran and produced nothing smaller — usually a version older than the features rustc \
                             emits"
                        ],
                    ],
                    tr![
                        td!["NR0310"],
                        td!["warning"],
                        td![
                            code![r#"[ui] script = "wasm""#],
                            " is on and the module exports no ",
                            code!["start"],
                            ": every component renders correctly and is inert. Add ",
                            code!["start: next_rust_ui_wasm::boot,"],
                            " to the ",
                            code!["islands!"]
                        ],
                    ],
                    tr![
                        td!["NR0311"],
                        td!["warning"],
                        td![
                            code!["[build] wasm_islands"],
                            " is on and the module exports no ",
                            code!["hydrate_<Island>"],
                            " at all"
                        ],
                    ],
                    tr![
                        td!["NR0312"],
                        td!["warning"],
                        td![
                            "a component is written ",
                            code![r#"#[client(wasm = "…")]"#],
                            " while ",
                            code!["[build] wasm_islands"],
                            " is off: the glue module is never written, the browser gets a 404, and the island \
                             keeps its server markup"
                        ],
                    ],
                    tr![
                        td!["NR0313"],
                        td!["warning"],
                        td![
                            "a ",
                            code![r#"#[client(wasm = "Name")]"#],
                            " has no ",
                            code!["hydrate_Name"],
                            " behind it — a typo, or an island that was written and never listed. The names are \
                             case-sensitive"
                        ],
                    ],
                    tr![
                        td!["NR0314"],
                        td!["warning"],
                        td!["the module exports an island no component asks for: bytes in every visitor's \
                             download that nothing can reach"],
                    ],
                    tr![
                        td!["NR0315"],
                        td!["warning"],
                        td![
                            "the islands crate invokes no ",
                            code!["next_rust_wasm::islands! { … }"],
                            ", which is what emits the allocator, the panic handler and every export the loader \
                             calls"
                        ],
                    ],
                    tr![
                        td!["NR0316"],
                        td!["warning"],
                        td![
                            "the module is missing a shared export the loader calls (",
                            code!["alloc"],
                            ", ",
                            code!["on_event"],
                            ") — an island then runs with no props, or with no events"
                        ],
                    ],
                ],
            ],
        ],
        h2![id("usual-fixes"), a![class("anchor"), href("#usual-fixes"), "The frequent ones and their fixes"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["code"], th!["usual cause"], th!["fix"]]],
                tbody![
                    tr![
                        td!["NR0001"],
                        td!["no ", code!["app/"], " folder, or the command runs in the wrong folder"],
                        td![
                            "create ",
                            code!["app/"],
                            ", ",
                            code!["cd"],
                            " into the project, or correct ",
                            code!["[app] directory"]
                        ],
                    ],
                    tr![
                        td!["NR0011"],
                        td!["an old ", code!["UA-…"], " Google Analytics id, or an id under the wrong key"],
                        td!["use the GA4 ", code!["G-…"], " id; the help line shows the expected form"],
                    ],
                    tr![
                        td!["NR0102"],
                        td![
                            "two route groups contain the same path: ",
                            code!["(a)/about"],
                            " and ",
                            code!["(b)/about"]
                        ],
                        td!["delete or rename one; groups never appear in the URL"],
                    ],
                    tr![
                        td!["NR0103"],
                        td![code!["[id]"], " and ", code!["[slug]"], " as sibling folders"],
                        td!["use the same parameter name at the same position"],
                    ],
                    tr![
                        td!["NR0104"],
                        td!["a ", code!["page.rs"], " and a ", code!["route.rs"], " answer the same URL"],
                        td!["move the API route, usually under ", code!["app/api/"]],
                    ],
                    tr![
                        td!["NR0108"],
                        td![code!["[user-id]"], ": a parameter name with a hyphen"],
                        td!["parameter names are Rust identifiers: ", code!["[user_id]"]],
                    ],
                    tr![
                        td!["NR0120"],
                        td![code!["Page.rs"], ", ", code!["Layout.rs"], ", ", code!["not_found.rs"]],
                        td![code!["page.rs"], ", ", code!["layout.rs"], ", ", code!["not-found.rs"]],
                    ],
                    tr![
                        td!["NR0201"],
                        td![
                            "wrong function name or a missing ",
                            code!["pub"],
                            "; ",
                            code!["error.rs"],
                            " exporting ",
                            code!["Error"]
                        ],
                        td![
                            code!["pub fn Page"],
                            ", ",
                            code!["pub fn Layout"],
                            ", ",
                            code!["pub fn NotFound"],
                            ", and ",
                            code!["pub fn ErrorBoundary"],
                            " in ",
                            code!["error.rs"],
                        ],
                    ],
                    tr![
                        td!["NR0204"],
                        td![code!["async fn Loading"], " or ", code!["async fn ErrorBoundary"]],
                        td!["remove ", code!["async"], ": these render instantly while something else is pending"],
                    ],
                    tr![
                        td!["NR0205"],
                        td!["an API handler with extractor arguments"],
                        td![
                            "handlers take nothing or exactly one ",
                            code!["Request"],
                            "; ",
                            code!["middleware"],
                            " takes ",
                            code!["(Request, Next)"]
                        ],
                    ],
                    tr![
                        td!["NR0206"],
                        td![code!["Data<T>"], " in a page whose file has no ", code!["load"]],
                        td!["put ", code!["pub async fn load"], " in the same file"],
                    ],
                    tr![
                        td!["NR0212"],
                        td![
                            "a ",
                            code!["Rendering::Partial"],
                            " page that takes ",
                            code!["Cookies"],
                            ", ",
                            code!["Headers"],
                            " or ",
                            code!["Query"]
                        ],
                        td![
                            "read them inside a ",
                            code!["dynamic(..)"],
                            " hole (",
                            a![href("/docs/rendering"), "rendering"],
                            ")"
                        ],
                    ],
                ],
            ],
        ],
        h2![
            id("compile-errors-in-generated-code"),
            a![class("anchor"), href("#compile-errors-in-generated-code"), "Compile errors in generated code"],
        ],
        p![
            "The generated route file (",
            code!["$OUT_DIR/next_rust_routes.rs"],
            ") calls your functions directly, so some mistakes show up as normal Rust type errors that point into it:",
        ],
        ul![
            li![
                code!["the trait bound X: FromContext is not satisfied"],
                ": a page argument whose type isn't an extractor. Check the argument list in ",
                a![href("/docs/routing#pages"), "routing"],
                ".",
            ],
            li![
                code!["the trait bound X: View is not satisfied"],
                ": a page returns something that can't be rendered.",
            ],
            li![
                code!["cannot find value __NR_ACTION_ID_…"],
                ": ",
                code!["action!(path)"],
                " points at a function without ",
                code!["#[server_action]"],
                ".",
            ],
        ],
        p!["The generated file is plain, readable Rust. Open it to see exactly how your function is called."],
        p![
            code!["$OUT_DIR"],
            " is a folder Cargo creates for the build script's output, deep inside ",
            code!["target/"],
            " (",
            code!["target/debug/build/<your-app>-<hash>/out/"],
            "). Never edit the file: it is rewritten on every build. Fix your own source instead.",
        ],
        h2![
            id("problems-without-a-code"),
            a![class("anchor"), href("#problems-without-a-code"), "Problems without a code"]
        ],
        ul![
            li![
                strong!["A new file under app/ is not routed. "],
                "The build script did not run again. Build again; ",
                code!["next-rust dev"],
                " does this automatically.",
            ],
            li![
                strong!["load returns a bare value. "],
                "The generated code applies ",
                code!["?"],
                ", so ",
                code!["load"],
                " must return ",
                code!["Result<T>"],
                ", and it must live in the same file as the ",
                code!["Page"],
                " that takes ",
                code!["Data<T>"],
                "; a ",
                code!["load"],
                " in another module is never called.",
            ],
            li![
                strong!["A page you expected to be static is dynamic. "],
                "The response carries ",
                code!["x-nr-cache: BYPASS"],
                ". Run ",
                code!["next-rust routes --layouts"],
                " to see which argument or layout made it dynamic.",
            ],
            li![
                strong!["Styles missing in production only. "],
                "A class name that exists only outside the views was renamed or its rule removed. Add it to ",
                code!["[tailwind] keep_classes"],
                " or ",
                code!["[assets] css_safelist"],
                "; for ",
                code!["data-*"],
                " attributes and CSS variables, ",
                code!["[build] keep_names"],
                ".",
            ],
            li![
                strong!["\"No space left on device\" during a build. "],
                code!["target/"],
                " filled the disk. Delete ",
                code!["target/debug/incremental"],
                " first, or run ",
                code!["cargo clean"],
                ".",
            ],
            li![
                strong!["Nothing here matches. "],
                "Run ",
                code!["next-rust doctor"],
                ": it checks the Rust version (1.89 or newer), the settings file, ",
                code!["build.rs"],
                ", ",
                code!["src/main.rs"],
                ", whether ",
                code![".env"],
                " is git-ignored, whether the port is free, and the routes.",
            ],
        ],
    ]
}
