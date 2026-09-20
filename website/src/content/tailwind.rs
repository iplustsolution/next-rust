//! The "tailwind" documentation page.

use next_rust::prelude::*;

/// The box that opens the page: the feature without the jargon.
fn plain_words(text: impl View) -> Node {
    div![class("callout"), p![strong!["In plain words. "], text]].into_node()
}

pub fn content() -> Node {
    fragment![
        plain_words(
            "Tailwind is a way of styling a page with small, ready-made labels instead of writing a stylesheet. \
             Rather than inventing a name like \"hero-box\" and describing it in a separate CSS file, you put the \
             look directly on the element: \"padding 6, rounded corners, blue background\". Each label (a utility \
             class) does exactly one thing, like LEGO bricks. Next Rust reads the labels you used and produces the \
             matching CSS by itself, so there is no extra tool to install or run."
        ),
        p![
            "Next Rust has Tailwind CSS built in. Write Tailwind classes in your views and the framework generates the CSS for exactly the classes you use. There is nothing to install and no CSS file to write: no Node.js, no npm, no ",
            code!["tailwind.config.js"],
            ".",
        ],
        p![
            "A page styled this way, for example ",
            code!["app/page.rs"],
            ". Read the classes as plain instructions: ",
            code!["mx-auto"],
            " centers the block, ",
            code!["max-w-2xl"],
            " limits its width, ",
            code!["px-6 py-24"],
            " add padding left/right and top/bottom, ",
            code!["text-4xl font-bold"],
            " make a large bold heading, and a prefix such as ",
            code!["hover:"],
            " or ",
            code!["dark:"],
            " (a variant) applies the class only in that situation:",
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
        p![
            "Tailwind is controlled from the ",
            code!["[tailwind]"],
            " section of ",
            code!["next-rust.toml"],
            ", the settings file in the project root. Without that section (or with ",
            code!["enabled = false"],
            ") Tailwind classes are just unknown words that style nothing, and you style with ",
            a![href("/docs/styling-and-assets"), "your own CSS"],
            " instead. In an existing project, add this to ",
            code!["next-rust.toml"],
            ":",
        ],
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
            "A theme is your brand's vocabulary: your colors, fonts, corner radius. Define a value once and Tailwind \
             creates classes for it, so ",
            code!["bg-brand"],
            " means your orange everywhere and changing the brand color is a one-line edit. Theme variables go in ",
            code!["[tailwind.theme]"],
            " in ",
            code!["next-rust.toml"],
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
            "When the same long list of classes appears on every button, give the list a name. ",
            code!["[tailwind.utilities]"],
            " in ",
            code!["next-rust.toml"],
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
        p![
            "Classes prefixed with ",
            code!["dark:"],
            " apply only in dark mode. This setting in ",
            code!["next-rust.toml"],
            " decides what \"dark mode\" means: the visitor's system setting (the default), or a class or attribute \
             your own theme switch puts on the page. For a switch, render the class on the server with ",
            a![href("/docs/rendering#html-attributes"), code!["Metadata::html_attribute"]],
            " so the page never flashes the wrong colors.",
        ],
        pre![code![
            class("language-toml"),
            "[tailwind]\ndark_mode = \"media\"   # dark: follows the system setting (default)\n# dark_mode = \"class\"     # dark: applies inside an element with class=\"dark\"\n# dark_mode = \"attribute\" # dark: applies inside data-theme=\"dark\"",
        ],],
        h2![id("variants"), a![class("anchor"), href("#variants"), "Custom variants"]],
        p![
            "A variant is the prefix before the colon (",
            code!["hover:"],
            ", ",
            code!["md:"],
            ") that says when a class applies. ",
            code!["[tailwind.variants]"],
            " adds your own: the name on the left, and on the right a CSS selector in which ",
            code!["&"],
            " stands for the element.",
        ],
        pre![code![
            class("language-toml"),
            r#"[tailwind.variants]
theme-midnight = "&:where([data-theme=midnight] *)"   # theme-midnight:bg-black
hocus = "&:hover, &:focus"                            # hocus:underline"#,
        ],],
        h2![id("plugins"), a![class("anchor"), href("#plugins"), "Plugins"]],
        p![
            "The official plugins are bundled with the engine, so listing them in ",
            code!["next-rust.toml"],
            " is all it takes (no npm install). Typography styles long-form text; forms gives form controls a \
             sensible base look:",
        ],
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
            ") or list them in ",
            code!["safelist"],
            ". A safelisted class is always generated, whether or not the build found it:",
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
        h2![id("stylesheets"), a![class("anchor"), href("#stylesheets"), "Stylesheet files"]],
        p![
            "A whole design system (color tokens, dark-mode palettes, component classes) does not belong in a TOML string. ",
            code!["stylesheets"],
            " lists CSS files that are compiled together with your classes. Each one is a full Tailwind input, so ",
            code!["@theme"],
            ", ",
            code!["@layer"],
            ", ",
            code!["@utility"],
            ", ",
            code!["@apply"],
            " (with every variant, including ",
            code!["dark:"],
            " and your own) and ",
            code!["@keyframes"],
            " all work. First tell ",
            code!["next-rust.toml"],
            " which files to compile. The folder name ",
            code!["styles/"],
            " is only an example; a Rust file with a ",
            code!["stylesheet!"],
            " works too (see ",
            a![href("/docs/styling-and-assets#scripts-and-styles-in-rust"), "Scripts and styles in Rust"],
            "):",
        ],
        pre![code![
            class("language-toml"),
            r#"[tailwind]
enabled = true
dark_mode = "class"
stylesheets = ["styles/theme.css", "styles/components.css"]   # relative to next-rust.toml"#,
        ],],
        p![
            "Then the stylesheet itself, ",
            code!["styles/theme.css"],
            ". It defines two colors that change in dark mode, exposes them as Tailwind colors (",
            code!["bg-primary"],
            ", ",
            code!["bg-bg"],
            ") and builds a ",
            code![".card"],
            " class out of utilities:",
        ],
        pre![code![
            class("language-css"),
            r#"/* styles/theme.css */
:root { --primary: #f26b2a; --bg: #fafaf7; }
.dark { --bg: #0d1b2a; }
@theme inline { --color-primary: var(--primary); --color-bg: var(--bg); }

@layer components {
  .card { @apply rounded-2xl border bg-bg p-6 dark:border-white/10; }
}"#,
        ],],
        p![
            "Files are imported in the order listed, right after Tailwind itself and before the other options, so a ",
            code!["css"],
            " string can still override them. Rules outside ",
            code!["@layer utilities"],
            " reach every page; utilities are sent to the pages that use them, like any other. A missing file stops the build with ",
            code!["NR0008"],
            ".",
        ],
        p![
            "Only the listed files are watched for changes, so list every file rather than ",
            code!["@import"],
            "ing one from another. A class that only a script adds (",
            code!["document.documentElement.classList.add(\"dark\")"],
            ") keeps its rules as long as the script lives in ",
            code!["client/"],
            " or ",
            code!["public/"],
            ", or the class is in ",
            code!["keep_classes"],
            ".",
        ],
        h2![
            id("short-class-names"),
            a![class("anchor"), href("#short-class-names"), "Short class names in production"]
        ],
        p![
            "Release builds (",
            code!["next-rust build"],
            ") rename every Tailwind class to a short name, in the CSS and in the HTML. What you write stays readable; what the browser downloads is small:",
        ],
        p!["The same element in the HTML the browser receives, in development and in a production build:"],
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
                ": the same class gets a different name next time. Classes you use most get the shortest names: one or two characters for the first thousand, three beyond that. A short name never equals a string in your Rust or script source, so it can't collide with a class those refer to.",
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
                "Scripts in ",
                code!["client/"],
                " and ",
                code!["assets/"],
                " are rewritten with the short names too, wherever a string can only be a class list: ",
                code!["el(\"div\", \"flex items-center gap-2\")"],
                ", ",
                code!["classList.add(\"hidden\")"],
                ", ",
                code!["className = …"],
                ", ",
                code!["class: …"],
                " in an object, ",
                code!["class=\"…\""],
                " in HTML strings, ",
                code![".card"],
                " in selectors. A lone word that could also be a CSS value (",
                code!["\"block\""],
                ", ",
                code!["\"absolute\""],
                ") keeps its name everywhere, as does a class named in ",
                code!["public/"],
                " files or mixed into other text. For classes built in ways the build can't see (regular expressions, concatenated pieces), list them in ",
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
        p![
            "The two settings, in ",
            code!["next-rust.toml"],
            ". ",
            code!["keep_classes"],
            " is the list of class names that must survive unchanged because something the build cannot read \
             (a third-party script, HTML assembled in a string) refers to them by their original name:",
        ],
        pre![code![
            class("language-toml"),
            "[tailwind]\nminify_classes = true          # default\nkeep_classes = [\"hidden\", \"is-open\"]",
        ],],
        p![
            "The same shortening is applied to ",
            code!["data-*"],
            " attributes, CSS custom properties, ",
            code!["@keyframes"],
            " and ",
            code!["@layer"],
            " names and the ids the UI components generate (",
            code!["[build] minify_names"],
            "). The matching escape hatch there is ",
            code!["[build] keep_names"],
            "; your own ids keep their names unless you list them in ",
            code!["[build] rename_ids"],
            ". See ",
            a![href("/docs/configuration"), "Configuration"],
            ".",
        ],
        h2![id("unused-classes"), a![class("anchor"), href("#unused-classes"), "Classes that style nothing"]],
        p![
            "A class in your code that Tailwind never generated (a typo in a variant, an arbitrary value it rejects, ",
            code!["sm:flexx"],
            ", ",
            code!["bg-[linear-gradient(…)]"],
            ") is dead weight in every page that renders it. Release builds leave such classes out of the HTML when they look generated (a variant, ",
            code!["[…]"],
            ", a fraction, ",
            code!["!"],
            ") and no stylesheet, script or static file of the project names them; plain words like ",
            code!["g-recaptcha"],
            " are always kept, since a third-party script may want them. ",
            code!["next-rust build"],
            " lists the classes it found, so you can fix them. ",
            code!["[build] drop_unused_classes = false"],
            " turns this off.",
        ],
        h2![id("per-page-css"), a![class("anchor"), href("#per-page-css"), "Only the CSS a page uses"]],
        p![
            "The stylesheet holds every rule the app uses, but each page is sent only the rules for the classes it renders. That covers Tailwind's utilities and any rule whose selectors start with a class, wherever it comes from: the CSS you import with ",
            code!["stylesheets"],
            ", a component library's ",
            code!["@layer components"],
            ", a ",
            code!["global_css!"],
            " sheet. Write ten classes in a stylesheet and render one, and the page receives one rule. The rest is cut down the same way: a theme variable (",
            code!["--color-primary"],
            ", ",
            code!["--spacing"],
            ") is sent when something sent reads it, directly or through another variable; a base or element rule (",
            code!["table"],
            ", ",
            code!["input[type=checkbox]"],
            ", ",
            code!["[data-theme=\"noir\"]"],
            ") when the page has that element or attribute, or loads a script that adds it; ",
            code!["@font-face"],
            ", ",
            code!["@property"],
            " and ",
            code!["@keyframes"],
            " once something sent names them (",
            a![href("/docs/styling-and-assets#per-page"), "the exact rules and the settings that override them"],
            "). On a client-side navigation the browser reports what it has, and the server sends only the missing rules (for cached static pages too). Content streamed in later brings its own rules along. The CSS is minified in development as well.",
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
                "Classes that only a script adds still need their rules, and they travel with the script: a page gets the classes of the ",
                code!["client/"],
                " modules its islands load, of the ",
                code!["assets/"],
                " and ",
                code!["public/"],
                " scripts it includes, and of the modules those import. Classes in ",
                code!["keep_classes"],
                ", ",
                code!["safelist"],
                " and extra Tailwind sources are sent with every page. A class your code assembles at runtime in a way the build can't see belongs in ",
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
            ". ",
            code!["build.rs"],
            " is the small program in the project root that Cargo runs before compiling your app; in a new \
             project it is the single line ",
            code!["next_rust_build::generate();"],
            ". Replacing that line with a ",
            code!["Generator"],
            " lets you set options in code. Useful when the theme comes from Rust data, or to keep one list of colors for light and dark mode:",
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
        h2![id("check-it-worked"), a![class("anchor"), href("#check-it-worked"), "How to check it worked"]],
        ul![
            li![
                "Run ",
                code!["next-rust dev"],
                ", add ",
                code!["class(\"bg-red-500\")"],
                " to any element and save. The element turns red; nothing else to do.",
            ],
            li![
                "In the browser's developer tools the element shows the class names you wrote, and ",
                code!["<head>"],
                " contains a ",
                code!["<style data-nr-css>"],
                " block with their rules.",
            ],
            li![
                "Before deploying, run ",
                code!["next-rust build"],
                " and ",
                code!["next-rust start"],
                " and click through the site once. Class names are now short and random; everything should look \
                 the same as in development. The build output lists classes that style nothing.",
            ],
        ],
        h2![id("common-mistakes"), a![class("anchor"), href("#common-mistakes"), "Common mistakes"]],
        ul![
            li![
                strong!["It looks right in dev and wrong in production."],
                " Almost always a class that reaches the page in a way the build cannot see: built with ",
                code!["format!"],
                ", stored in a database, written inside a JavaScript string in Rust, matched by a regular \
                 expression, or expected by a third-party script. Add it to ",
                code!["keep_classes"],
                " (and ",
                code!["safelist"],
                " if Tailwind never saw it whole). Do not turn Tailwind or ",
                code!["minify_classes"],
                " off to fix one class.",
            ],
            li![
                strong!["Classes assembled from pieces."],
                " ",
                code!["format!(\"text-{color}-600\")"],
                " generates nothing. Write whole names in both branches of a condition.",
            ],
            li![
                strong!["Selecting elements by Tailwind class from outside."],
                " A browser test or analytics script looking for ",
                code![".rounded-xl"],
                " finds nothing in production. Select by ",
                code!["id"],
                " or a ",
                code!["data-*"],
                " attribute listed in ",
                code!["[build] keep_names"],
                ".",
            ],
            li![
                strong!["Different class names on different servers."],
                " Names are random per build. When several builds serve the same site behind one load balancer, set ",
                code!["NEXT_RUST_CLASS_SEED"],
                " to the same number for all of them.",
            ],
            li![
                strong!["A build that cannot download the engine."],
                " Offline or locked-down machines need ",
                code!["NEXT_RUST_TAILWIND_BIN"],
                " (see above).",
            ],
            li![
                strong!["Adding transitions by hand."],
                " Hover and state changes already ease by default (",
                a![href("/docs/styling-and-assets#animation"), "Animation"],
                "). Your own ",
                code!["transition-*"],
                " classes still win when you want something different.",
            ],
        ],
    ]
}
