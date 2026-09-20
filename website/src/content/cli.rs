//! The "cli" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            code!["next-rust"],
            " is the command-line tool (CLI: a program you run by typing in a terminal) that creates a project, runs it while you work, builds the production program and checks that everything is in order.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "Think of it as the remote control of your project. ",
                code!["next-rust new"],
                " makes the project folder, ",
                code!["next-rust dev"],
                " shows the site on your own computer and refreshes it as you type, ",
                code!["next-rust build"],
                " turns the project into one finished program, and ",
                code!["next-rust doctor"],
                " tells you what is wrong when something does not work.",
            ],
        ],
        p!["Every command has the same shape, and ", code!["--help"], " after any command explains it:"],
        pre![code![class("language-text"), r"next-rust <COMMAND> [OPTIONS]"]],
        p!["A normal working day uses four of them, typed in the project folder:"],
        pre![code![
            class("language-sh"),
            r"next-rust new my-app     # once: create the project folder my-app/
cd my-app
next-rust dev            # while you work: http://localhost:3000, rebuilds on every save
next-rust build          # when you are done: one production program in .next-rust/
next-rust start          # try that production program locally",
        ]],
        h2![id("commands-at-a-glance"), a![class("anchor"), href("#commands-at-a-glance"), "Commands at a glance"]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["command"], th!["what it is for"]]],
                tbody![
                    tr![td![a![href("#new-name"), code!["new"]]], td!["create a new project"]],
                    tr![
                        td![a![href("#dev"), code!["dev"]]],
                        td!["run the site on your computer and rebuild on every change"]
                    ],
                    tr![td![a![href("#build"), code!["build"]]], td!["make the production program (one file)"]],
                    tr![td![a![href("#start"), code!["start"]]], td!["run the program that ", code!["build"], " made"]],
                    tr![
                        td![a![href("#check"), code!["check"]]],
                        td!["find mistakes without building or running anything"]
                    ],
                    tr![
                        td![a![href("#routes"), code!["routes"]]],
                        td!["list every URL of the site and how it is rendered"]
                    ],
                    tr![
                        td![a![href("#analyze"), code!["analyze"]]],
                        td!["report page sizes and the size of the last build"]
                    ],
                    tr![
                        td![a![href("#generate-kind-route"), code!["generate"]]],
                        td!["write a starter file for a page, layout, API route, …"]
                    ],
                    tr![
                        td![a![href("#doctor"), code!["doctor"]]],
                        td!["check the computer and the project for common problems"]
                    ],
                    tr![
                        td![a![href("#docker"), code!["docker"]]],
                        td!["write the files needed to package the app as a container"]
                    ],
                    tr![td![a![href("#editor"), code!["editor"]]], td!["add VS Code snippets and settings"]],
                    tr![td![a![href("#upgrade"), code!["upgrade"]]], td!["update the framework and this tool"]],
                    tr![td![a![href("#clean"), code!["clean"]]], td!["delete the build output folder"]],
                ],
            ],
        ],
        h2![
            id("files-and-folders"),
            a![class("anchor"), href("#files-and-folders"), "Files and folders the CLI works with"]
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["path"], th!["what it is"], th!["safe to delete?"]]],
                tbody![
                    tr![
                        td![code!["next-rust.toml"]],
                        td![
                            "the project's settings, in the project root (",
                            a![href("/docs/configuration"), "configuration"],
                            ")"
                        ],
                        td!["yes, the defaults apply; but it is yours, keep it in Git"],
                    ],
                    tr![
                        td![code!["build.rs"]],
                        td![
                            "the build script: a tiny Rust file in the project root that Cargo runs before compiling. It reads ",
                            code!["app/"],
                            " and generates the route table",
                        ],
                        td!["no: without it no page is part of the program (", code!["doctor"], " reports it)"],
                    ],
                    tr![
                        td![code!["src/main.rs"]],
                        td!["the program's entry point; one line, ", code!["next_rust::app!();"]],
                        td!["no"],
                    ],
                    tr![
                        td![code!["target/"]],
                        td![
                            "Cargo's working folder: compiled pieces, reused so the next build is faster. Can grow to several gigabytes"
                        ],
                        td!["yes; the next build recreates it, slowly. Never commit it"],
                    ],
                    tr![
                        td![code!["target/next-rust/"]],
                        td![
                            "reports of the last build (",
                            code!["routes.json"],
                            ", ",
                            code!["build.json"],
                            ") that ",
                            code!["analyze"],
                            " reads"
                        ],
                        td!["yes"],
                    ],
                    tr![
                        td![code![".next-rust/"]],
                        td![
                            "the output folder (",
                            code!["[build] output"],
                            "): the finished production binary ",
                            code![".next-rust/<name>"],
                            ", and ",
                            code![".next-rust/dev/"],
                            " with the dev server's status",
                        ],
                        td!["yes, ", code!["next-rust clean"], " does exactly that. Never commit it"],
                    ],
                    tr![
                        td![code!["Dockerfile"], ", ", code![".dockerignore"]],
                        td!["the recipe for a container image, written by ", code!["next-rust docker"]],
                        td!["yes, if you do not deploy with Docker"],
                    ],
                    tr![
                        td![code![".vscode/"]],
                        td!["editor snippets and settings, written by ", code!["new"], " and ", code!["editor"]],
                        td!["yes"],
                    ],
                    tr![
                        td![code!["~/.next-rust/"], ", ", code!["~/.cache/next-rust"]],
                        td![
                            "in your home folder, not the project: the time of the last update check, and the downloaded Tailwind engine"
                        ],
                        td!["yes; the engine is downloaded again when needed"],
                    ],
                ],
            ],
        ],
        p![
            "A ",
            em!["binary"],
            " is a finished program file the computer runs directly, like an app you double-click. Rust code is compiled into one; nothing else has to be installed to run it.",
        ],
        p![
            "The CLI depends only on the standard library and the framework's own crates (no argument-parsing library, no async runtime). It drives ",
            code!["cargo"],
            " for compilation.",
        ],
        h2![id("new-name"), a![class("anchor"), href("#new-name"), code!["new <name>"]]],
        p![
            "Creates a project: ",
            code!["Cargo.toml"],
            ", ",
            code!["build.rs"],
            ", ",
            code!["src/main.rs"],
            ", ",
            code!["next-rust.toml"],
            ", ",
            code!["app/"],
            " with a layout, two pages, a 404 page and an API route, plus ",
            code!["public/"],
            ", ",
            code![".gitignore"],
            " and ",
            code![".env.example"],
            ".",
        ],
        p![
            "The name may contain ASCII letters, digits, ",
            code!["-"],
            " and ",
            code!["_"],
            ", must not start with a digit, and the folder must be empty or not exist yet. The project is styled with Tailwind CSS by default.",
        ],
        p![
            "The project depends on ",
            code!["next-rust"],
            " from crates.io (the public Rust package registry) when this CLI's version is published there; otherwise, or with ",
            code!["--git"],
            ", on ",
            code!["https://github.com/iplustsolution/next-rust"],
            ".",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["option"], th![]]],
                tbody![
                    tr![
                        td![code!["--framework-path <dir>"]],
                        td!["depend on a local Next Rust checkout (path dependencies) instead"],
                    ],
                    tr![
                        td![code!["--git"]],
                        td!["depend on the GitHub repository even when a crates.io release exists"]
                    ],
                    tr![
                        td![code!["--no-tailwind"]],
                        td!["use a plain CSS file (", code!["app/globals.css"], ") instead of Tailwind"],
                    ],
                ],
            ],
        ],
        h2![id("dev"), a![class("anchor"), href("#dev"), code!["dev"]]],
        p![
            "The development server: it runs the site on your own computer (",
            code!["http://localhost:3000"],
            ") and keeps it up to date while you edit. Nothing here is visible to the internet. It:",
        ],
        ul![
            li!["validates routes on every change and prints readable diagnostics;"],
            li![
                "rebuilds incrementally with ",
                code!["cargo build"],
                ". While it compiles, and after a compile error, ",
                strong!["the previous server keeps serving"],
                ";",
            ],
            li!["restarts the server after a successful build. Connected browsers reconnect and reload automatically;",],
            li![
                "sends compile errors to the browser as an overlay (",
                code![".next-rust/dev/status.json"],
                ", streamed via ",
                code!["/_next-rust/dev/events"],
                ");",
            ],
            li!["reloads the browser without a rebuild when ", code!["public/"], " or ", code!["client/"], " change;",],
            li!["prints ", code!["route added"], " / ", code!["route removed"], " when files appear or disappear;",],
            li![
                strong!["fills new empty special files with starter code."],
                " Create an empty ",
                code!["app/about/page.rs"],
                " and it becomes a working ",
                code!["/about"],
                " page within a moment. The same goes for ",
                code!["layout.rs"],
                ", ",
                code!["route.rs"],
                ", ",
                code!["loading.rs"],
                ", ",
                code!["error.rs"],
                ", ",
                code!["not-found.rs"],
                ", ",
                code!["middleware.rs"],
                ", ",
                code!["template.rs"],
                ", ",
                code!["metadata.rs"],
                " and ",
                code!["default.rs"],
                ". Only brand-new, empty files are filled: files with content, files you empty later, and files in ",
                code!["_private"],
                " folders are never touched. A new file is filled only after it has stayed empty for a moment \
                 (0.8 s), so a file an editor or tool is still writing is never overwritten. Turn it off with ",
                code!["[dev] scaffold = false"],
                ".",
            ],
        ],
        p![
            "Watched for rebuilds: the app directory, ",
            code!["src/"],
            ", ",
            code!["assets/"],
            ", ",
            code!["build.rs"],
            ", ",
            code!["Cargo.toml"],
            ", the config file, ",
            code!["[api] directory"],
            " and ",
            code!["[dev] watch"],
            ". Watching uses polling (",
            code!["[dev] poll_interval"],
            "), which works the same on every platform, over network filesystems and in containers.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["option"], th![]]],
                tbody![tr![td![code!["--port, -p <port>"]], td!["port (default from config)"]]],
            ],
        ],
        h2![id("build"), a![class("anchor"), href("#build"), code!["build"]]],
        p![
            "Makes the production version: one program file that contains the whole site and is ready to copy to a server (",
            a![href("/docs/deployment"), "deployment"],
            "). It takes longer than a development build because the compiler spends its time making the file small and fast. Step by step, it:",
        ],
        ol![
            li!["validates routes (fails on errors);"],
            li![
                code!["cargo build --release"],
                " with ",
                code!["next-rust.toml"],
                ", ",
                code!["public/"],
                ", ",
                code!["assets/"],
                " and ",
                code!["client/"],
                " embedded (only the files in ",
                code!["assets/"],
                " that ",
                code!["asset!"],
                " references), ",
                code!["opt-level = \"z\""],
                ", fat LTO, one codegen unit, stripped symbols and every build path rewritten. These settings override ",
                code!["[profile.release]"],
                "; change them with ",
                code!["[build] optimize"],
                " and ",
                code!["[build] panic"],
                ";",
            ],
            li!["copies the binary to ", code![".next-rust/<name>"], ", the only file the build writes there;",],
            li![
                "runs it from an empty directory with ",
                code!["--export"],
                ", which renders every static page in memory, so rendering errors and missing files fail the build;",
            ],
            li![
                "writes the route manifest and render report for ",
                code!["next-rust analyze"],
                " to ",
                code!["target/next-rust/"],
                ";",
            ],
            li![
                "shows each step as it happens: routes, compiling with a live progress bar (percentage, units done, what is compiling now, then link-time optimization), embedded files, the pre-render check and the output. In CI logs, where nothing can be redrawn, it prints a line at every 25%. Then it prints a summary:",
            ],
        ],
        p!["The summary lists each route, the size of its pre-rendered HTML and how it is rendered:"],
        pre![code![
            class("language-text"),
            r"Route                                              Size
○ /                                               705 B  1 page
○ /blog/:slug                                     2.1 kB  12 pages · revalidate 3600s
◐ /shop/:slug                                     1.8 kB  40 pages · revalidate 3600s · shell + dynamic holes
ƒ /api/hello                                             api
λ /products/:id                                          dynamic segment without generate_params

○  static (pre-rendered)   ◐  partial (pre-rendered shell, per-request holes)   λ  dynamic (server-rendered)   ƒ  API",
        ],],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["option"], th![]]],
                tbody![tr![td![code!["--no-check"]], td!["skip rendering static pages after compiling"]]],
            ],
        ],
        h2![id("start"), a![class("anchor"), href("#start"), code!["start"]]],
        p![
            "Runs ",
            code![".next-rust/<name>"],
            " with ",
            code!["NEXT_RUST_ENV=production"],
            ". ",
            code!["--port"],
            " overrides the port. It fails with a clear message when there is no build yet: run ",
            code!["next-rust build"],
            " first. On a server you do not need the CLI at all; you run the binary itself.",
        ],
        h2![id("check"), a![class("anchor"), href("#check"), code!["check"]]],
        p![
            "Validates routes and special-file exports, then runs ",
            code!["cargo check"],
            " (Rust's type check, which finds compile errors without producing a program). It is the quickest way to ask \"is anything broken?\", and a good step for CI (continuous integration: the automated checks that run on every push).",
        ],
        h2![id("routes"), a![class("anchor"), href("#routes"), code!["routes"]]],
        p![
            "Prints the route table: every URL the site answers, the file it comes from, and whether it is static (prepared ahead) or dynamic (built per request). Use it to confirm the framework understood your folder names.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["option"], th![]]],
                tbody![
                    tr![td![code!["--layouts"]], td!["layout chain and the reason a route is dynamic"]],
                    tr![td![code!["--tree"]], td!["the scanned directory tree with special files"]],
                    tr![td![code!["--json"]], td!["the route manifest (same format as ", code!["routes.json"], ")"],],
                ],
            ],
        ],
        h2![id("analyze"), a![class("anchor"), href("#analyze"), code!["analyze"]]],
        p![
            "Reads the last build's reports from ",
            code!["target/next-rust/"],
            ": static/dynamic/API counts, pre-rendered page sizes (largest first) with render times, on-demand routes, and binary size.",
        ],
        h2![
            id("generate-kind-route"),
            a![class("anchor"), href("#generate-kind-route"), code!["generate <kind> <route>"]],
        ],
        p![
            "Scaffolds a special file. Kinds: ",
            code!["page"],
            ", ",
            code!["layout"],
            ", ",
            code!["template"],
            ", ",
            code!["loading"],
            ", ",
            code!["error"],
            ", ",
            code!["not-found"],
            ", ",
            code!["api"],
            ", ",
            code!["middleware"],
            ". It refuses to overwrite a file that exists. ",
            code!["g"],
            " is an alias. Run it from the project folder; the route is the URL, written the way the folder is named:",
        ],
        pre![code![
            class("language-sh"),
            r"next-rust generate page /products/[id]
next-rust generate api /api/orders
next-rust generate loading /dashboard",
        ],],
        h2![id("editor"), a![class("anchor"), href("#editor"), code!["editor"]]],
        p!["Adds editor support for VS Code:"],
        ul![
            li![
                code![".vscode/next-rust.code-snippets"],
                ": type a prefix and pick the suggestion. ",
                code!["nrpage"],
                ", ",
                code!["nrparams"],
                ", ",
                code!["nrlayout"],
                ", ",
                code!["nrroute"],
                ", ",
                code!["nrget"],
                ", ",
                code!["nrpost"],
                ", ",
                code!["nrmeta"],
                ", ",
                code!["nrload"],
                ", ",
                code!["nraction"],
                ", ",
                code!["nrform"],
                ", ",
                code!["nrloading"],
                ", ",
                code!["nrerror"],
                ", ",
                code!["nrnotfound"],
                ", ",
                code!["nrmiddleware"],
                ", ",
                code!["nrcomponent"],
                ", ",
                code!["nrlink"],
                ", ",
                code!["nrimage"],
                ", ",
                code!["nrcss"],
                ".",
            ],
            li![
                code![".vscode/extensions.json"],
                ": recommends rust-analyzer, which provides completion for everything in ",
                code!["next_rust::prelude"],
                " (element macros, attributes, extractors, ",
                code!["Response"],
                ", …).",
            ],
            li![
                code![".vscode/settings.json"],
                ": re-runs build scripts on save, so a page you just created is part of the crate, and gets completions, right away.",
            ],
        ],
        p!["New projects include these files. Existing files are kept unless ", code!["--force"], "."],
        h2![id("doctor"), a![class("anchor"), href("#doctor"), code!["doctor"]]],
        p![
            "Checks the toolchain (rustc ≥ 1.89), configuration, ",
            code!["build.rs"],
            " and ",
            code!["src/main.rs"],
            " wiring, the ",
            code!["next-rust-build"],
            " dependency, ",
            code![".env"],
            " git-ignore, port availability, and route diagnostics.",
        ],
        p![
            "Run it first whenever something fails for a reason that does not look like your code. Each check prints a green line or a red one that names the fix, and the command exits with ",
            code!["1"],
            " when it found a problem. This is the output of a healthy project (versions and counts differ):",
        ],
        pre![code![
            class("language-text"),
            r"✓ rustc: rustc 1.89.0
✓ cargo: cargo 1.89.0
✓ Rust version supports Next Rust (≥ 1.89)
✓ configuration: /home/you/my-app/next-rust.toml
✓ Cargo.toml found
✓ build.rs found
✓ src/main.rs includes the generated routes
✓ next-rust-build is a build dependency
✓ .env files are git-ignored
✓ port 3000 is free
✓ 4 routes, no route errors

Everything looks good.",
        ]],
        h2![id("docker"), a![class("anchor"), href("#docker"), code!["docker"]]],
        p![
            "Docker packages a program with everything it needs into an image that runs the same on any server. A ",
            code!["Dockerfile"],
            " is the recipe for that image; ",
            code![".dockerignore"],
            " lists what to leave out of it. Both go in the project root.",
        ],
        p![
            "Writes a multi-stage ",
            code!["Dockerfile"],
            " and ",
            code![".dockerignore"],
            ". See ",
            a![href("/docs/deployment"), "deployment"],
            ". ",
            code!["--force"],
            " overwrites existing files.",
        ],
        h2![id("upgrade"), a![class("anchor"), href("#upgrade"), code!["upgrade"]]],
        p!["Updates to the latest Next Rust on GitHub."],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["option"], th![]]],
                tbody![
                    tr![td![em!["(none)"]], td!["both of the below"]],
                    tr![
                        td![code!["--project"]],
                        td![
                            code!["cargo update -p next-rust -p next-rust-build"],
                            " in the current project, reporting the old and new commit",
                        ],
                    ],
                    tr![
                        td![code!["--cli"]],
                        td![code![
                            "cargo install --git https://github.com/iplustsolution/next-rust next-rust-cli --force",
                        ],],
                    ],
                ],
            ],
        ],
        p![
            "Projects that use ",
            code!["--framework-path"],
            " are updated with ",
            code!["git pull"],
            " in that checkout. ",
            code!["upgrade"],
            " says so instead of changing anything.",
        ],
        h3![id("update-notice"), a![class("anchor"), href("#update-notice"), "Update notice"]],
        p![
            "Once every 24 hours, ",
            code!["dev"],
            " and ",
            code!["new"],
            " check GitHub for a newer version: the workspace version in ",
            code!["Cargo.toml"],
            ", and the latest commit compared with your ",
            code!["Cargo.lock"],
            ". If something is newer, they print a note suggesting ",
            code!["next-rust upgrade"],
            ". The check:",
        ],
        ul![
            li!["never installs or changes anything;"],
            li![
                "sends nothing about you or your project (it downloads one public file and runs ",
                code!["git ls-remote"],
                ");",
            ],
            li!["gives up silently after about 5 seconds or on any error;"],
            li!["is skipped when ", code!["CI"], " or ", code!["NEXT_RUST_NO_UPDATE_CHECK"], " is set."],
        ],
        p!["The time of the last check is stored in ", code!["~/.next-rust/last-update-check"], "."],
        h2![id("clean"), a![class("anchor"), href("#clean"), code!["clean"]]],
        p![
            "Removes the build output directory (",
            code![".next-rust/"],
            " unless ",
            code!["[build] output"],
            " says otherwise). It refuses to delete a directory outside the project. It does not touch ",
            code!["target/"],
            "; ",
            code!["cargo clean"],
            " empties that one.",
        ],
        h2![id("exit-codes"), a![class("anchor"), href("#exit-codes"), "Exit codes"]],
        p![
            "An exit code is the number a command hands back when it finishes; scripts and CI use it to decide whether to continue. ",
            code!["0"],
            " success, ",
            code!["1"],
            " failure (diagnostics were printed).",
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["A misspelled flag. "],
                "Unknown flags are ignored silently, so ",
                code!["--prot 8080"],
                " starts on the default port without complaint. Flags accept both ",
                code!["--port 3000"],
                " and ",
                code!["--port=3000"],
                ".",
            ],
            li![
                strong!["Running a command outside the project. "],
                "Commands look for ",
                code!["next-rust.toml"],
                " and ",
                code!["Cargo.toml"],
                " from the current folder upwards. ",
                code!["cd"],
                " into the app first (in a workspace: into the app's own folder).",
            ],
            li![strong!["start or analyze before build. "], "Both read what ", code!["build"], " wrote.",],
            li![
                strong!["A new page that does not appear under plain cargo run. "],
                "The route table is generated by ",
                code!["build.rs"],
                ", so a new file under ",
                code!["app/"],
                " needs a rebuild. ",
                code!["next-rust dev"],
                " does that for you.",
            ],
            li![
                strong!["Port already in use. "],
                "Another program (often a forgotten dev server) holds the port. Stop it, or pass ",
                code!["--port"],
                "; ",
                code!["doctor"],
                " reports this.",
            ],
            li![
                strong!["Committing build output. "],
                code!["target/"],
                " and ",
                code![".next-rust/"],
                " belong in ",
                code![".gitignore"],
                "; new projects have them there already.",
            ],
        ],
    ]
}
