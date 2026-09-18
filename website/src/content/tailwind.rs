//! The "tailwind" documentation page.

use next_rust::prelude::*;

pub fn content() -> Node {
    fragment![
        p![
            "Next Rust has Tailwind CSS built in. Write Tailwind classes in your views and the framework generates the CSS for exactly the classes you use. There is nothing to install and no CSS file to write: no Node.js, no npm, no ",
            code!["tailwind.config.js"],
            ".",
        ],
        pre![code![
            class("language-rust"),
            r#"pub fn Page() -> impl View {
    main![
        class("mx-auto max-w-2xl px-6 py-24"),
        h1![class("text-4xl font-bold tracking-tight text-zinc-900 dark:text-white"), "Hello"],
        a![class("mt-8 inline-flex rounded-lg bg-brand px-4 py-2 font-semibold text-white hover:bg-brand/90"), href("/docs"), "Read the docs"],
    ]
}"#,
        ],],
        p![
            "New projects created with ",
            code!["next-rust new"],
            " use Tailwind already. The engine is the official Tailwind CSS ",
            strong!["v4.3.3"],
            ", so every class, variant and feature in the ",
            a![href("https://tailwindcss.com/docs"), target("_blank"), rel("noopener"), "Tailwind documentation"],
            " works as described there.",
        ],
        h2![id("turn-it-on"), a![class("anchor"), href("#turn-it-on"), "Turn it on"]],
        p!["In an existing project, add this to ", code!["next-rust.toml"], ":"],
        pre![code![class("language-toml"), "[tailwind]\nenabled = true"],],
        p!["That's all. Every page, layout and 404 page gets the stylesheet automatically."],
        h2![id("how-it-works"), a![class("anchor"), href("#how-it-works"), "How it works"]],
        ol![
            li![
                "The build scans the Rust files in ",
                code!["app/"],
                " and ",
                code!["src/"],
                " for class names. Any string works: ",
                code!["class(\"…\")"],
                ", both branches of an ",
                code!["if"],
                ", arrays, constants.",
            ],
            li![
                "Tailwind generates CSS for those classes only. Development builds get readable CSS; release builds get it minified."
            ],
            li![
                "The CSS is compiled into the app and added to the ",
                code!["<head>"],
                " of every page, before your own stylesheets. Partial navigations don't send it again.",
            ],
        ],
        p![
            "The Tailwind engine (the official standalone executable for your operating system) is downloaded the first time a Tailwind project builds, checked against the SHA-256 checksum published with the release, and cached for every project on the machine. ",
            code!["next-rust dev"],
            " and ",
            code!["next-rust build"],
            " show the download with a progress bar. It needs ",
            code!["curl"],
            ", which macOS, Windows 10+ and most Linux systems include.",
        ],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["system"], th!["cache"]]],
                tbody![
                    tr![
                        td!["macOS, Linux"],
                        td![code!["~/.cache/next-rust"], " (or ", code!["$XDG_CACHE_HOME/next-rust"], ")"]
                    ],
                    tr![td!["Windows"], td![code!["%LOCALAPPDATA%\\next-rust"]]],
                ],
            ],
        ],
        div![
            class("callout"),
            p![
                "Offline machine or locked-down CI? Download the executable for your platform from the ",
                a![
                    href("https://github.com/tailwindlabs/tailwindcss/releases/tag/v4.3.3"),
                    target("_blank"),
                    rel("noopener"),
                    "Tailwind v4.3.3 release"
                ],
                " and set ",
                code!["NEXT_RUST_TAILWIND_BIN=/path/to/tailwindcss"],
                ". ",
                code!["NEXT_RUST_CACHE_DIR"],
                " moves the cache.",
            ],
        ],
        h2![id("theme"), a![class("anchor"), href("#theme"), "Your theme"]],
        p![
            "Theme variables go in ",
            code!["[tailwind.theme]"],
            ". Each one becomes a Tailwind v4 ",
            code!["@theme"],
            " variable and creates the matching classes:",
        ],
        pre![code![
            class("language-toml"),
            r##"[tailwind.theme]
color-brand = "#f26b2a"            # bg-brand, text-brand, border-brand, ring-brand/50, …
color-ink = "oklch(21% 0.006 285)" # bg-ink, text-ink, …
font-display = "Inter, sans-serif" # font-display
breakpoint-3xl = "120rem"          # 3xl:flex
radius-card = "1.25rem"            # rounded-card
shadow-soft = "0 8px 30px rgb(0 0 0 / 0.08)"  # shadow-soft
animate-rise = "rise 500ms ease-out both"     # animate-rise"##,
        ],],
        p![
            "The names follow Tailwind's ",
            a![
                href("https://tailwindcss.com/docs/theme#theme-variable-namespaces"),
                target("_blank"),
                rel("noopener"),
                "theme variable namespaces"
            ],
            " (",
            code!["color-*"],
            ", ",
            code!["font-*"],
            ", ",
            code!["text-*"],
            ", ",
            code!["spacing"],
            ", ",
            code!["breakpoint-*"],
            ", ",
            code!["radius-*"],
            ", ",
            code!["shadow-*"],
            ", ",
            code!["animate-*"],
            ", …). The leading ",
            code!["--"],
            " is optional. To replace a whole namespace, set it to ",
            code!["initial"],
            " first, for example ",
            code!["color-* = \"initial\""],
            ".",
        ],
        h2![id("custom-classes"), a![class("anchor"), href("#custom-classes"), "Your own classes"]],
        p![
            code!["[tailwind.utilities]"],
            " defines new classes, as Tailwind classes to combine or as CSS declarations. They work with every variant (",
            code!["hover:btn"],
            ", ",
            code!["md:btn"],
            ", …):",
        ],
        pre![code![
            class("language-toml"),
            r#"[tailwind.utilities]
btn = "inline-flex items-center rounded-lg bg-brand px-4 py-2 font-semibold text-white hover:bg-brand/90"
card = "rounded-card border border-zinc-200 bg-white p-6 shadow-soft dark:border-zinc-800 dark:bg-zinc-900"
content-auto = { content-visibility = "auto" }"#,
        ],],
        h2![id("dark-mode"), a![class("anchor"), href("#dark-mode"), "Dark mode"]],
        pre![code![
            class("language-toml"),
            "[tailwind]\ndark_mode = \"media\"   # dark: follows the system setting (default)\n# dark_mode = \"class\"     # dark: applies inside an element with class=\"dark\"\n# dark_mode = \"attribute\" # dark: applies inside data-theme=\"dark\"",
        ],],
        h2![id("variants"), a![class("anchor"), href("#variants"), "Custom variants"]],
        pre![code![
            class("language-toml"),
            r#"[tailwind.variants]
theme-midnight = "&:where([data-theme=midnight] *)"   # theme-midnight:bg-black
hocus = "&:hover, &:focus"                            # hocus:underline"#,
        ],],
        h2![id("plugins"), a![class("anchor"), href("#plugins"), "Plugins"]],
        p!["The official plugins are bundled with the engine:"],
        pre![code![
            class("language-toml"),
            "[tailwind]\nplugins = [\"@tailwindcss/typography\", \"@tailwindcss/forms\"]"
        ],],
        p![
            "With typography, ",
            code!["article![class(\"prose dark:prose-invert\"), …]"],
            " styles rendered Markdown or CMS content.",
        ],
        h2![id("dynamic-classes"), a![class("anchor"), href("#dynamic-classes"), "Classes built at runtime"]],
        p![
            "Tailwind finds complete class names in your source. A class assembled from pieces, like ",
            code!["format!(\"bg-{color}-500\")"],
            ", never appears whole, so write the full names (",
            code!["if error { \"bg-red-500\" } else { \"bg-green-500\" }"],
            ") or list them:",
        ],
        pre![code![
            class("language-toml"),
            "[tailwind]\nsafelist = [\"bg-red-500\", \"bg-green-500\"]\nsources = [\"templates\"]   # more directories to scan"
        ],],
        h2![id("anything-else"), a![class("anchor"), href("#anything-else"), "Anything else"]],
        p![
            "For what the options above don't cover (keyframes, ",
            code!["@layer base"],
            " rules, ",
            code!["@source not"],
            ", …), ",
            code!["css"],
            " takes Tailwind CSS directly. It is added after everything else:",
        ],
        pre![code![
            class("language-toml"),
            r#"[tailwind]
enabled = true
css = """
@keyframes rise {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: none; }
}
@layer base {
  h1 { @apply text-balance; }
}
"""
# preflight = false   # leave out Tailwind's base styles"#,
        ],],
        h2![
            id("short-class-names"),
            a![class("anchor"), href("#short-class-names"), "Short class names in production"]
        ],
        p![
            "Release builds (",
            code!["next-rust build"],
            ") rename every Tailwind class to a short name, in the CSS and in the HTML. What you write stays readable; what the browser downloads is small:",
        ],
        pre![code![
            class("language-html"),
            r#"<!-- next-rust dev -->
<div class="mt-4 rounded-xl border border-zinc-200 px-6 py-4 hover:bg-zinc-50">

<!-- next-rust build -->
<div class="dx ce g e7 k2 q9 ty">"#,
        ],],
        ul![
            li![
                "Names are ",
                strong!["random for every build"],
                ": the same class gets a different name next time. Classes you use most get the shortest names.",
            ],
            li![
                "Development builds keep the names you wrote, so the browser's developer tools show ",
                code!["rounded-xl"],
                ", not ",
                code!["ce"],
                ".",
            ],
            li![
                "Everything the renderer writes is covered: ",
                code!["class(…)"],
                ", ",
                code!["active_class"],
                ", ",
                code!["data-nr-class-<name>"],
                " and ",
                code!["class=\"…\""],
                " inside ",
                code!["raw_html"],
                ". Classes that only give context to other classes (",
                code!["group"],
                ", ",
                code!["peer"],
                ", the ",
                code![".tk"],
                " in ",
                code!["[&_.tk]:text-red-500"],
                ") keep their names.",
            ],
        ],
        p!["On this site the HTML of a docs page went from 97 KB to 71 KB, and the CSS from 47 KB to 38 KB.",],
        div![
            class("callout"),
            p![
                "Classes that appear in ",
                code!["public/"],
                " or ",
                code!["client/"],
                " files (a script calling ",
                code!["classList.add(\"hidden\")"],
                ") keep their names automatically. For other code that refers to a class by name, list it in ",
                code!["keep_classes"],
                ", or turn renaming off with ",
                code!["minify_classes = false"],
                ". ",
                code!["NEXT_RUST_CLASS_SEED=<number>"],
                " gives the same names on every build (reproducible builds, or several servers running different builds behind one load balancer); ",
                code!["NEXT_RUST_MINIFY_CLASSES=1"],
                " renames in a development build too.",
            ],
        ],
        pre![code![
            class("language-toml"),
            "[tailwind]\nminify_classes = true          # default\nkeep_classes = [\"hidden\", \"is-open\"]",
        ],],
        h2![id("per-page-css"), a![class("anchor"), href("#per-page-css"), "Only the CSS a page uses"]],
        p![
            "The stylesheet holds every utility the app uses, but each page is sent only the rules for the classes it renders. Theme variables and the base styles are always sent; ",
            code!["@property"],
            " registrations and ",
            code!["@keyframes"],
            " are sent once a rule that needs them is. On a client-side navigation the browser reports what it has, and the server sends only the missing rules (for cached static pages too). Content streamed in later brings its own rules along. The CSS is minified in development as well.",
        ],
        p![
            "Tailwind utilities depend on their order (",
            code!["px-2"],
            " must come after ",
            code!["p-4"],
            " to win). When a new rule arrives, rules the browser already has that set the same property and come after it in Tailwind's order are sent again, so the result always matches the full stylesheet.",
        ],
        div![
            class("callout"),
            p![
                "Classes that only code outside the view tree adds still need their rules. Classes that appear in ",
                code!["public/"],
                ", ",
                code!["client/"],
                " or extra sources (",
                code!["classList.add(\"hidden\")"],
                "), and those in ",
                code!["keep_classes"],
                " or ",
                code!["safelist"],
                ", are sent with every page. A class your code assembles at runtime anywhere else belongs in ",
                code!["keep_classes"],
                ".",
            ],
        ],
        h2![id("build-rs"), a![class("anchor"), href("#build-rs"), "Configure it in Rust"]],
        p![
            "Every option above can also be set in ",
            code!["build.rs"],
            ", on top of ",
            code!["next-rust.toml"],
            ". Useful when the theme comes from Rust data, or to keep one list of colors for light and dark mode:",
        ],
        pre![code![
            class("language-rust"),
            r##"// build.rs
fn main() {
    next_rust_build::Generator::new()
        .tailwind(|tw| {
            tw.enabled = true;
            for (name, light, dark) in [("bg", "#ffffff", "#0b0b0d"), ("fg", "#111113", "#ededef")] {
                tw.theme.insert(format!("color-{name}"), format!("light-dark({light}, {dark})"));
            }
            tw.utilities.insert("btn".into(), "rounded-lg bg-fg px-4 py-2 text-bg".into());
        })
        .run();
}"##,
        ],],
        p![
            "This website is built that way: see its ",
            a![
                href("https://github.com/iplustsolution/next-rust/blob/main/website/build.rs"),
                target("_blank"),
                rel("noopener"),
                code!["build.rs"]
            ],
            ".",
        ],
        h2![id("deployment"), a![class("anchor"), href("#deployment"), "Deployment"]],
        p![
            "The generated CSS is part of the binary, so production servers need nothing. Only the machine that builds needs the engine: ",
            code!["next-rust docker"],
            " writes a Dockerfile that installs ",
            code!["curl"],
            " and caches the engine between image builds.",
        ],
    ]
}
