//! The icon browser (`/icons`): every Lucide icon of `next_rust::icons` in a
//! searchable grid, with a detail dialog to copy the Rust call or the SVG.
//!
//! The page is static and large (one inline `<svg>` per icon), so a cell is as
//! small as it can be: `<button title="name"><svg viewBox>…</svg><span>name</span></button>`
//! with no classes of its own. The grid styles its cells, and the stroke
//! attributes every Lucide icon shares are CSS there instead of five
//! attributes on each of the ~2100 `<svg>`s.
//!
//! The script finds elements by id, by tag and by the `title` attribute, and
//! shows and hides with the `hidden` attribute: release builds rename classes
//! and `data-*` attributes, but none of those.

use std::fmt::Write as _;

use next_rust::icons;
use next_rust::prelude::*;

/// The Lucide stroke style, as CSS on a container of bare `<svg viewBox>`s.
const STROKES: &str = "[&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-2 [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round]";

/// Rust-orange text, for the numbers that carry the page.
const FLAME: &str = "bg-[linear-gradient(100deg,var(--color-accent-fg)_0%,var(--color-accent)_48%,#f0930f_100%)] bg-clip-text text-transparent";

/// The ambient light behind the hero: warm on the left, cool on the right.
const GLOW: &str = concat!(
    "pointer-events-none absolute inset-x-0 -top-16 -z-10 h-[34rem] ",
    "bg-[radial-gradient(52%_62%_at_16%_0%,var(--color-accent-soft),transparent_70%),",
    "radial-gradient(46%_58%_at_84%_6%,#6366f11f,transparent_72%),",
    "radial-gradient(80%_46%_at_50%_0%,var(--color-soft),transparent_78%)]",
);

/// The same light, much weaker, under the grid: it only shows through the
/// gaps between the cells, which is exactly as much as it should.
const GLOW_LOW: &str = concat!(
    "pointer-events-none absolute inset-x-0 top-[30rem] bottom-0 -z-10 ",
    "bg-[radial-gradient(38%_22%_at_0%_10%,#e0561a1f,transparent_72%),",
    "radial-gradient(38%_24%_at_100%_42%,#6366f11f,transparent_72%),",
    "radial-gradient(52%_20%_at_50%_100%,#e0561a1a,transparent_74%)]",
);

/// The grid and, through it, every cell. Nothing is set on the cells
/// themselves: 1848 of them ship in the HTML, so every class here is a class
/// that is not repeated 1848 times.
const GRID: &str = concat!(
    "mt-6 grid grid-cols-[repeat(auto-fill,minmax(6rem,1fr))] gap-2.5 max-[640px]:grid-cols-[repeat(auto-fill,minmax(4.75rem,1fr))] max-[640px]:gap-1.5 ",
    // Cells off screen are not laid out or painted until they scroll in.
    "[&>button]:[content-visibility:auto] [&>button]:[contain-intrinsic-size:auto_5.5rem] ",
    "[&>button]:flex [&>button]:min-w-0 [&>button]:cursor-pointer [&>button]:flex-col [&>button]:items-center [&>button]:gap-2.5 ",
    "[&>button]:rounded-xl [&>button]:border [&>button]:border-line [&>button]:px-1.5 [&>button]:pt-4 [&>button]:pb-2.5 [&>button]:text-fg-soft ",
    // A lit surface: a warm corner fading into a top-to-bottom tint.
    "[&>button]:bg-[linear-gradient(160deg,var(--color-accent-soft)_0%,var(--color-raised)_46%,var(--color-soft)_100%)] ",
    "[&>button]:shadow-[inset_0_1px_0_0_#ffffff14,0_1px_2px_-1px_#0000000f] ",
    "[&>button]:transition-[transform,box-shadow,border-color,color] [&>button]:duration-150 ",
    // Hover lifts the cell, tints it and turns the icon rust-orange.
    "[&>button:hover]:-translate-y-[3px] [&>button:hover]:border-accent/60 [&>button:hover]:text-accent ",
    "[&>button:hover]:bg-[linear-gradient(160deg,#e0561a33_0%,#e0561a14_45%,var(--color-raised)_100%)] ",
    "[&>button:hover]:shadow-[0_12px_24px_-14px_var(--color-accent),0_0_0_1px_var(--color-accent-soft)] ",
    "[&>button:focus-visible]:border-accent [&>button:focus-visible]:text-accent ",
    "[&_svg]:size-6 [&_svg]:shrink-0 [&_svg]:transition-transform [&_svg]:duration-150 [&>button:hover_svg]:scale-115 ",
    "[&_span]:w-full [&_span]:truncate [&_span]:text-center [&_span]:text-[0.6875rem]/[1.4] [&_span]:text-muted ",
    "[&>button:hover_span]:text-fg-soft",
);

const COPY_BUTTON: &str = concat!(
    "inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-[0.625rem] border border-line-strong ",
    "bg-[linear-gradient(160deg,var(--color-raised),var(--color-soft))] px-3.5 text-[0.8125rem] font-semibold text-fg ",
    "shadow-[0_1px_2px_-1px_#00000014] transition hover:border-accent/55 hover:text-accent-fg active:translate-y-px",
);

const COPY_BUTTON_PRIMARY: &str = concat!(
    "inline-flex h-10 cursor-pointer items-center justify-center gap-2 rounded-[0.625rem] border border-accent/70 ",
    "bg-[linear-gradient(135deg,var(--color-accent)_0%,var(--color-accent-fg)_100%)] px-4 text-[0.8125rem] font-semibold text-on-accent ",
    "shadow-[0_10px_22px_-12px_var(--color-accent)] transition hover:brightness-110 active:translate-y-px",
);

/// One number of the stat row under the headline.
fn stat(value: String, unit: &'static str, note: &'static str) -> impl View {
    div![
        class(
            "relative overflow-hidden rounded-2xl border border-line bg-[linear-gradient(155deg,var(--color-raised)_0%,var(--color-soft)_100%)] px-5 py-4 shadow-[0_1px_2px_-1px_#0000000f] max-[860px]:px-4 max-[860px]:py-3 max-[640px]:rounded-xl max-[640px]:px-3 max-[640px]:py-2.5"
        ),
        span![
            aria("hidden", "true"),
            class(
                "pointer-events-none absolute inset-x-0 top-0 h-px bg-[linear-gradient(90deg,transparent,var(--color-accent),transparent)]"
            )
        ],
        p![
            class(
                "flex flex-wrap items-baseline gap-x-1.5 text-[1.75rem]/[1.15] font-bold tracking-[-0.03em] max-[860px]:text-[1.375rem] max-[640px]:text-[1.125rem]"
            ),
            span![class(FLAME), value],
            span![class("text-[0.9375rem] font-semibold text-fg-soft max-[640px]:text-[0.75rem]"), unit],
        ],
        p![class("mt-1 text-[0.8125rem]/[1.5] text-muted max-[640px]:hidden"), note],
    ]
}

pub fn page() -> impl View {
    let total = icons::all().len();
    main![
        id("content"),
        class("relative isolate mx-auto max-w-[100rem] px-6 pt-12 pb-24 max-[640px]:px-4"),
        div![aria("hidden", "true"), class(GLOW)],
        div![aria("hidden", "true"), class(GLOW_LOW)],
        p![
            class(
                "inline-flex items-center gap-2 rounded-full border border-accent/30 bg-accent-soft px-3 py-1 text-[0.8125rem] font-semibold text-accent-fg"
            ),
            icons::Sparkles().size("0.9375rem"),
            "Icon library",
        ],
        h1![
            class("mt-5 text-[clamp(2.25rem,5.2vw,3.5rem)]/[1.06] font-bold tracking-[-0.035em]"),
            span![class(FLAME), class("text-[1.1em]"), total.to_string()],
            " icons, one function each",
        ],
        p![
            class("mt-4 max-w-[48rem] text-lg/[1.6] text-muted"),
            "Every ",
            a![
                class("text-accent-fg underline underline-offset-3"),
                href("https://lucide.dev"),
                target("_blank"),
                rel("noopener"),
                "Lucide"
            ],
            " icon ships as a Rust function: ",
            code![class("font-mono text-[0.9em] text-fg"), "icons::ArrowRight().size(16)"],
            ". It renders as an inline SVG that follows the text color, and icons you never call are left out of the binary.",
        ],
        div![
            class("mt-7 grid max-w-[54rem] grid-cols-3 gap-3 max-[640px]:gap-2"),
            stat(total.to_string(), "icons", "Every Lucide glyph, as a Rust function call."),
            stat(icons::LUCIDE_VERSION.to_owned(), "Lucide", "ISC licensed, refreshed with each release."),
            stat("0 KB".into(), "runtime", "Tree-shaken: unused icons never reach the binary."),
        ],
        p![
            class("mt-4 text-sm/[1.6] text-muted"),
            "Click an icon to copy it · ",
            a![class("text-accent-fg underline underline-offset-3"), href("/docs/components#icons"), "Icon properties"],
        ],
        Browser(),
    ]
}

/// Search box, grid and detail dialog: one island, so the script is loaded
/// with the page and wired up again after every client navigation to it.
#[client(script = "icons.js")]
fn Browser() -> impl View {
    let all = icons::all();
    let total = all.len();
    // One string for the whole grid: names are `[a-z0-9-]` and the bodies are
    // generated from Lucide's drawings, so there is nothing to escape.
    let mut cells = String::with_capacity(total * 420);
    for icon in all {
        let _ = write!(
            cells,
            r#"<button type="button" title="{name}"><svg viewBox="0 0 24 24" aria-hidden="true">{body}</svg><span>{name}</span></button>"#,
            name = icon.name,
            body = icon.body,
        );
    }
    fragment![
        div![
            class(
                "relative sticky top-16 z-40 -mx-6 mt-8 border-b border-line bg-header px-6 py-3.5 backdrop-blur-xl backdrop-saturate-180 max-[640px]:-mx-4 max-[640px]:px-4"
            ),
            role("search"),
            span![
                aria("hidden", "true"),
                class(
                    "pointer-events-none absolute inset-x-0 -bottom-px h-px bg-[linear-gradient(90deg,transparent,var(--color-accent),transparent)] opacity-60"
                )
            ],
            // The gradient hairline is the wrapper; the input sits on an
            // opaque surface one pixel inside it.
            div![
                class(
                    "rounded-2xl bg-[linear-gradient(110deg,var(--color-accent)_0%,#f0930f_45%,var(--color-accent-fg)_100%)] p-px shadow-[0_6px_20px_-14px_var(--color-accent)] transition-shadow focus-within:shadow-[0_0_0_4px_var(--color-accent-soft),0_14px_34px_-16px_var(--color-accent)]"
                ),
                div![
                    class("relative flex items-center rounded-[0.9375rem] bg-bg"),
                    span![
                        class("pointer-events-none absolute left-4 text-accent max-[640px]:left-3.5"),
                        icons::Search().size("1.125rem")
                    ],
                    input![
                        id("icons-search"),
                        class(
                            "h-14 w-full min-w-0 rounded-[0.9375rem] bg-transparent pr-36 pl-12 text-base text-fg placeholder:text-muted placeholder:[text-overflow:ellipsis] focus:outline-none max-[640px]:h-12 max-[640px]:pr-3 max-[640px]:pl-10.5 max-[640px]:placeholder:text-[0.875rem]"
                        ),
                        r#type("search"),
                        name("q"),
                        placeholder(format!("Search {total} icons, e.g. arrow right")),
                        aria("label", "Search icons"),
                        aria("controls", "icons-grid"),
                        autocomplete("off"),
                        spellcheck("false"),
                        enterkeyhint("search"),
                    ],
                    span![
                        class(
                            "pointer-events-none absolute right-3 flex items-center gap-2 text-[0.8125rem] text-muted max-[860px]:hidden"
                        ),
                        span![
                            id("icons-count"),
                            role("status"),
                            class(
                                "rounded-full border border-line bg-soft px-2.5 py-1 font-medium tabular-nums text-fg-soft"
                            ),
                            format!("{total} icons")
                        ],
                        kbd![
                            class(
                                "rounded-md border border-line-strong bg-raised px-1.5 py-0.5 font-mono text-[0.75rem]/[1.5]"
                            ),
                            "/"
                        ],
                    ],
                ],
            ],
        ],
        noscript![p![
            class("mt-4 rounded-[0.625rem] border border-line bg-soft px-4 py-3 text-sm text-fg-soft"),
            "Search and the copy buttons need JavaScript. Every icon is on this page with its name, so your browser's find-in-page works.",
        ]],
        div![id("icons-grid"), class(GRID), class(STROKES), raw_html(cells)],
        div![
            id("icons-empty"),
            hidden(true),
            class(
                "mt-6 overflow-hidden rounded-2xl border border-dashed border-line-strong bg-[radial-gradient(58%_80%_at_50%_0%,var(--color-accent-soft),transparent_72%)] px-6 py-14 text-center"
            ),
            span![
                class(
                    "mx-auto mb-4 grid size-14 place-items-center rounded-2xl border border-line bg-raised text-accent shadow-[0_10px_24px_-14px_var(--color-accent)]"
                ),
                icons::SearchX().size("1.5rem")
            ],
            p![class("text-lg font-semibold text-fg"), "No icon matches ", q![id("icons-empty-query")], "."],
            p![
                class("mt-2 text-sm text-muted"),
                "Try fewer or different words. Names are the ones on lucide.dev: ",
                code![class("font-mono text-fg-soft"), "house"],
                ", not home.",
            ],
        ],
        detail(),
    ]
}

/// The one dialog every cell opens; the script fills it in. None of the
/// elements the script writes into may have children of their own: it sets
/// `textContent`, which would drop them.
fn detail() -> impl View {
    let snippet = |label: &'static str, code_id: &'static str, button_id: &'static str, sample: &'static str| {
        div![
            class("mt-4"),
            p![class("mb-1.5 text-[0.75rem] font-semibold tracking-[0.04em] text-muted uppercase"), label],
            div![
                class(
                    "flex items-stretch overflow-hidden rounded-xl border border-line bg-[linear-gradient(125deg,var(--color-code)_0%,var(--color-soft)_100%)] shadow-[inset_0_1px_0_#ffffff0f]"
                ),
                pre![
                    class(
                        "min-w-0 flex-1 overflow-x-auto px-3.5 py-2.5 font-mono text-[0.78125rem]/[1.7] text-fg-soft max-[640px]:px-3 max-[640px]:text-[0.75rem]"
                    ),
                    code![id(code_id), sample]
                ],
                button![
                    id(button_id),
                    r#type("button"),
                    class(
                        "cursor-pointer border-l border-line px-3.5 text-[0.8125rem] font-semibold text-muted transition-colors hover:bg-accent-soft hover:text-accent-fg"
                    ),
                    "Copy"
                ],
            ],
        ]
    };
    dialog![
        id("icon-dialog"),
        aria("labelledby", "icon-title"),
        class(
            "m-auto w-[min(34rem,calc(100vw-2rem))] max-w-none overflow-hidden rounded-[1.25rem] border border-line-strong bg-raised p-0 text-fg shadow-[0_36px_80px_-24px_#00000099] backdrop:bg-black/55 backdrop:backdrop-blur-[3px]"
        ),
        div![
            class(
                "relative bg-[linear-gradient(180deg,var(--color-soft)_0%,var(--color-raised)_42%)] p-6 max-[640px]:p-4"
            ),
            span![
                aria("hidden", "true"),
                class(
                    "pointer-events-none absolute inset-x-0 top-0 h-px bg-[linear-gradient(90deg,transparent,var(--color-accent),transparent)]"
                )
            ],
            div![
                class("flex items-start gap-5 max-[640px]:gap-4"),
                div![
                    id("icon-preview"),
                    class(
                        "grid size-32 shrink-0 place-items-center rounded-2xl border border-line text-fg shadow-[inset_0_1px_0_#ffffff14,0_12px_30px_-18px_var(--color-accent)] max-[640px]:size-24 [&_svg]:size-20 max-[640px]:[&_svg]:size-14"
                    ),
                    class(
                        "[background-image:radial-gradient(var(--color-line-strong)_1px,transparent_1px),radial-gradient(circle_at_28%_18%,var(--color-accent-soft),transparent_66%),linear-gradient(155deg,var(--color-raised),var(--color-soft))] [background-size:0.5rem_0.5rem,100%_100%,100%_100%]"
                    ),
                    class(STROKES),
                ],
                div![
                    class("min-w-0 flex-1"),
                    h2![
                        id("icon-title"),
                        class("truncate text-[1.5rem]/[1.2] font-bold tracking-[-0.025em]"),
                        "ArrowRight"
                    ],
                    p![
                        id("icon-name"),
                        class(
                            "mt-2 inline-block max-w-full truncate rounded-md border border-line bg-soft px-2 py-0.5 font-mono text-[0.8125rem] text-muted"
                        ),
                        "arrow-right"
                    ],
                    a![
                        id("icon-link"),
                        class(
                            "mt-3 flex items-center gap-1.5 text-sm font-medium text-accent-fg underline underline-offset-3"
                        ),
                        href("https://lucide.dev/icons/arrow-right"),
                        target("_blank"),
                        rel("noopener"),
                        "Open on lucide.dev",
                        icons::ExternalLink().size("0.875rem"),
                    ],
                ],
                form![
                    method("dialog"),
                    button![
                        class(
                            "grid size-9 cursor-pointer place-items-center rounded-[0.5625rem] border border-transparent text-muted transition-colors hover:border-line hover:bg-soft hover:text-fg"
                        ),
                        r#type("submit"),
                        aria("label", "Close"),
                        icons::X().size("1.125rem"),
                    ],
                ],
            ],
            snippet("Use it", "icon-use", "icon-copy-use", "icons::ArrowRight()"),
            snippet(
                "Customised",
                "icon-custom",
                "icon-copy-custom",
                "icons::ArrowRight().size(20).color(\"#e0561a\").stroke_width(1.5)",
            ),
            div![
                class("mt-5 flex flex-wrap gap-2.5 border-t border-line pt-5"),
                button![id("icon-copy-svg"), r#type("button"), class(COPY_BUTTON_PRIMARY), "Copy SVG"],
                button![id("icon-copy-name"), r#type("button"), class(COPY_BUTTON), "Copy name"],
            ],
        ],
    ]
}

next_rust::client_script!(
    "icons.js",
    r##"
// The icon browser of /icons. Elements are found by id, tag and `title`, and
// shown or hidden with the `hidden` attribute: none of that is renamed in
// release builds (classes and data-* attributes are).
const SVG_OPEN = '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">';

// arrow-down-0-1 -> ArrowDown01, the name of the Rust function.
const pascal = (name) => name.split("-").map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join("");

// "ArrowRight", "arrow-right", "arrow right" and "right arrow" all become
// the words ["arrow", "right"].
const words = (query) =>
  query
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);

async function copy(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    (document.querySelector("dialog[open]") || document.body).appendChild(area);
    area.select();
    let done = false;
    try {
      done = document.execCommand("copy");
    } catch {}
    area.remove();
    return done;
  }
}

export function hydrate(root) {
  const byId = (id) => root.querySelector("#" + id);
  const input = byId("icons-search");
  const grid = byId("icons-grid");
  const count = byId("icons-count");
  const empty = byId("icons-empty");
  const emptyQuery = byId("icons-empty-query");
  const dialog = byId("icon-dialog");
  if (!input || !grid || !dialog) return;

  const cells = [...grid.children].map((el) => {
    const name = el.getAttribute("title") || el.textContent.trim();
    return { el, name, compact: name.replaceAll("-", "") };
  });
  const total = cells.length;

  let frame = 0;
  const filter = () => {
    frame = 0;
    const query = words(input.value);
    let shown = 0;
    for (const cell of cells) {
      const match = query.every((word) => cell.name.includes(word) || cell.compact.includes(word));
      if (cell.el.hidden === match) cell.el.hidden = !match;
      if (match) shown++;
    }
    if (count) count.textContent = query.length ? `${shown} of ${total} icons` : `${total} icons`;
    if (empty) empty.hidden = shown > 0;
    if (emptyQuery) emptyQuery.textContent = input.value.trim();
    grid.hidden = shown === 0;
  };
  input.addEventListener("input", () => {
    if (!frame) frame = requestAnimationFrame(filter);
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && input.value) {
      input.value = "";
      filter();
    } else if (event.key === "Enter") {
      const first = cells.find((cell) => !cell.el.hidden);
      if (first && input.value.trim()) open(first);
    }
  });

  // "/" focuses the search box. The listener leaves with the page.
  const onKey = (event) => {
    if (!input.isConnected) return document.removeEventListener("keydown", onKey);
    const target = event.target;
    const typing = target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));
    if (event.key === "/" && !typing && !dialog.open && !event.metaKey && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      input.focus();
      input.select();
    }
  };
  document.addEventListener("keydown", onKey);

  let current = null;
  const set = (id, text) => {
    const el = byId(id);
    if (el) el.textContent = text;
  };
  function open(cell) {
    current = cell;
    const fn = pascal(cell.name);
    set("icon-title", fn);
    set("icon-name", cell.name);
    set("icon-use", `icons::${fn}()`);
    set("icon-custom", `icons::${fn}().size(20).color("#e0561a").stroke_width(1.5)`);
    const link = byId("icon-link");
    if (link) link.href = "https://lucide.dev/icons/" + cell.name;
    const preview = byId("icon-preview");
    const svg = cell.el.querySelector("svg");
    if (preview && svg) preview.replaceChildren(svg.cloneNode(true));
    if (typeof dialog.showModal === "function") {
      if (!dialog.open) dialog.showModal();
    } else {
      dialog.setAttribute("open", "");
    }
  }
  grid.addEventListener("click", (event) => {
    const el = event.target instanceof Element && event.target.closest("button");
    const cell = el && cells.find((c) => c.el === el);
    if (cell) open(cell);
  });
  // A click on the backdrop closes the dialog.
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });

  const svgSource = (cell) => {
    const svg = cell.el.querySelector("svg");
    const body = svg ? svg.innerHTML.replace(/><\/[a-z]+>/g, "/>") : "";
    return SVG_OPEN + body + "</svg>";
  };
  const copies = {
    "icon-copy-use": () => byId("icon-use").textContent,
    "icon-copy-custom": () => byId("icon-custom").textContent,
    "icon-copy-svg": () => svgSource(current),
    "icon-copy-name": () => current.name,
  };
  for (const [id, text] of Object.entries(copies)) {
    const button = byId(id);
    if (!button) continue;
    const label = button.textContent;
    let timer = 0;
    button.addEventListener("click", async () => {
      if (!current) return;
      const done = await copy(text());
      button.textContent = done ? "Copied" : "Press Ctrl+C";
      clearTimeout(timer);
      timer = setTimeout(() => (button.textContent = label), 1400);
    });
  }

  // /icons?q=arrow opens filtered.
  const initial = new URLSearchParams(location.search).get("q");
  if (initial) input.value = initial;
  if (input.value) filter();
}
"##
);
