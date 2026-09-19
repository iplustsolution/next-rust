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
        h2![id("configuration-nr00xx"), a![class("anchor"), href("#configuration-nr00xx"), "Configuration (NR00xx)"],],
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
                ],
            ],
        ],
        p!["Parse errors (bad TOML/JSON, unknown keys) report the file, line and key."],
        h2![id("routing-nr01xx"), a![class("anchor"), href("#routing-nr01xx"), "Routing (NR01xx)"]],
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
    ]
}
