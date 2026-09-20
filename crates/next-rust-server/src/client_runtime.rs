//! The browser runtime served at `/_next-rust/runtime.js`.
//!
//! Browsers can only execute JavaScript or WebAssembly, so the part of the
//! framework that runs in the browser is JavaScript source kept here as a
//! string. It has no dependencies and never uses `eval`, so it works under a
//! strict Content Security Policy. It is sent only to pages that render a
//! `Link!` with client navigation or an interactive island.
//!
//! Responsibilities: client-side navigation and prefetching, applying streamed
//! boundaries after navigation, server-action calls, sending every same-origin
//! form without a page reload (`[client] enhance_forms`), and island hydration (declarative bindings or `hydrate(element, props)` modules).
//!
//! Two forms are embedded:
//!
//! * [`RUNTIME_JS`]: the readable source, served in development and the one
//!   to edit.
//! * [`RUNTIME_JS_MIN`]: the production build, minified with local names
//!   mangled. It is generated from the source with Terser (see
//!   CONTRIBUTING.md). A test fails if the source changes without the
//!   minified copy being regenerated.

#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub const RUNTIME_JS: &str = r#"// Next Rust client runtime. No dependencies, no eval (CSP friendly).
// Served at /_next-rust/runtime.js and loaded only by pages that render an internal
// link (with client navigation enabled) or an interactive island.
const NR = (window.nextRust = window.nextRust || {});
// Pages fetched by prefetching or navigation, reused for PAGE_TTL ms so a
// page is never downloaded twice in a row: a click after a prefetch, a second
// click, or going back all use the copy already in memory.
const pages = new Map();
const PAGE_TTL = 30000;
let current = location.pathname + location.search;

function swapStreamed(doc) {
  for (const t of doc.querySelectorAll('template[id^="nr-t"]')) {
    const b = doc.getElementById("nr-b" + t.id.slice(4));
    if (b) b.replaceWith(t.content);
    t.remove();
  }
}

// ---- Layouts ---------------------------------------------------------------
// The server wraps the children of every reusable layout in comment markers,
// <!--nr-l:KEY--> ... <!--/nr-l:KEY-->. Navigations send the keys on screen,
// and the server answers with only what goes inside the deepest shared layout.

function layoutMarkers() {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
  const found = [];
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    if (n.data.startsWith("nr-l:")) found.push(n);
  }
  return found;
}

// The start and end markers of a layout region, if it is on screen.
function findRegion(key) {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
  let start = null;
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    if (n.data === "nr-l:" + key) start = n;
    else if (start && n.data === "/nr-l:" + key && n.parentNode === start.parentNode) return { start, end: n };
  }
  return null;
}

// `init`: the rest of the request, for a form (`method`, `body`, `signal`).
async function fetchPage(href, full, init) {
  const headers = { "x-nr-nav": "1", "x-nr-from": location.pathname, accept: "text/html" };
  if (init && init.body) withCsrf(headers);
  const keys = full ? [] : layoutMarkers().map((n) => n.data.slice(5));
  if (keys.length) headers["x-nr-layouts"] = keys.join(",");
  // Stylesheets (and utility rules) the page has are not sent again.
  const styles = [...document.querySelectorAll("style[data-nr-css]")].map((s) => s.dataset.nrCss);
  if (styles.length) headers["x-nr-styles"] = styles.join(",");
  // Inline delivery: scripts the document has are not sent again either.
  if (inlined.size) headers["x-nr-scripts"] = [...inlined].join(",");
  const res = await fetch(href, { headers, credentials: "same-origin", ...init });
  if (!(res.headers.get("content-type") || "").includes("text/html")) {
    // What a form posted to may answer with anything: the caller reads it.
    if (init && init.body) return { res, url: res.url };
    throw new Error("not html");
  }
  return {
    url: res.url,
    html: await res.text(),
    // An intercepted route renders differently depending on where the
    // navigation started, so that copy is only valid from the same page.
    from: res.headers.get("x-nr-intercepted") ? location.pathname : null,
    // Only the inside of this layout was sent.
    partial: res.headers.get("x-nr-partial"),
  };
}

// The page for `href`: the copy in memory if it is recent, otherwise one
// request, shared by everyone asking for the same page at the same time.
function loadPage(href, fresh, full, signal) {
  const now = Date.now();
  for (const [key, entry] of pages) if (now - entry.at >= PAGE_TTL) pages.delete(key);
  const hit = fresh ? null : pages.get(href);
  if (hit) return hit.promise;
  const promise = fetchPage(href, full, { signal });
  const entry = { at: now, promise };
  pages.set(href, entry);
  promise.catch(() => {
    if (pages.get(href) === entry) pages.delete(href);
  });
  return promise;
}

const prefetch = (href) => loadPage(href, false);

// `target`: a selector (`data-nr-target` on a form); when both the page and
// the response have that element, only its content changes.
function render(page, target) {
  const doc = new DOMParser().parseFromString(page.html, "text/html");
  swapStreamed(doc);
  document.title = doc.title;
  const managed = 'meta[name]:not([name="viewport"]),meta[property],link[rel="canonical"],style[data-nr-css]';
  // Streamed chunks put their styles in <body>; keep every style in <head>,
  // in order, so replacing the body never drops rules.
  for (const s of document.body.querySelectorAll("style[data-nr-css]")) document.head.appendChild(s);
  const existing = new Map([...document.head.querySelectorAll("style[data-nr-css]")].map((s) => [s.dataset.nrCss, s]));
  document.head.querySelectorAll(managed.replace(",style[data-nr-css]", "")).forEach((n) => n.remove());
  let restyled = false;
  for (const n of [...doc.head.querySelectorAll(managed), ...doc.body.querySelectorAll("style[data-nr-css]")]) {
    const id = n.dataset && n.dataset.nrCss;
    // Utility rules (`<sheet>~<rules>`) depend on their order: the newest copy goes last.
    if (id && id.includes("~")) restyled = true;
    if (id && existing.has(id)) {
      if (id.includes("~")) document.head.appendChild(existing.get(id));
    } else {
      document.head.appendChild(document.importNode(n, true));
    }
    if (n.parentNode !== doc.head) n.remove();
  }
  // Pages fetched earlier were styled for the rules the browser had then.
  if (restyled) pages.clear();
  const env = doc.getElementById("__next_rust_env");
  if (env) {
    try {
      NR.env = JSON.parse(env.textContent || "{}");
    } catch {}
  }
  // Inline delivery: a script runs once per document, like a module loaded
  // from a URL (classic page scripts ask to run again); module registrations
  // arrive in <head> and run from there, before the islands hydrate.
  for (const s of doc.querySelectorAll("script[data-nr-js]")) {
    const id = s.dataset.nrJs;
    if (inlined.has(id) && !s.hasAttribute("data-nr-rerun")) {
      s.remove();
      continue;
    }
    inlined.add(id);
    if (s.parentNode !== doc.head) continue;
    const copy = document.createElement("script");
    for (const a of s.attributes) copy.setAttribute(a.name, a.value);
    copy.textContent = s.textContent;
    document.head.appendChild(copy);
  }
  let scripts = [];
  const region = page.partial && findRegion(page.partial);
  const mine = target && document.querySelector(target);
  const theirs = mine && doc.querySelector(target);
  if (theirs) morph(mine, theirs);
  else if (region) {
    // Keep every shared layout; replace only what is inside the deepest one.
    const { start, end } = region;
    while (start.nextSibling && start.nextSibling !== end) start.nextSibling.remove();
    const added = [];
    for (const n of [...doc.body.childNodes]) {
      if (n.nodeType === 1 && (n.id === "__next_rust_env" || (n.getAttribute("src") || "").startsWith("/_next-rust/runtime.js"))) continue;
      const node = document.importNode(n, true);
      end.parentNode.insertBefore(node, end);
      added.push(node);
    }
    scripts = added.flatMap((n) => (n.nodeType !== 1 ? [] : n.tagName === "SCRIPT" ? [n] : [...n.querySelectorAll("script")]));
  } else {
    document.body.replaceWith(document.importNode(doc.body, true));
    scripts = [...document.body.querySelectorAll("script")];
  }
  for (const old of scripts) {
    const src = old.getAttribute("src") || "";
    if (old.type === "application/json" || src.startsWith("/_next-rust/runtime.js") || /^\$nr\(/.test(old.textContent)) continue;
    const s = document.createElement("script");
    for (const a of old.attributes) s.setAttribute(a.name, a.value);
    s.textContent = old.textContent;
    old.replaceWith(s);
  }
  hydrateIslands(document);
}

// Make the children of `from` those of `to`, changing what differs and keeping
// what does not: an element that stays keeps its focus, caret, scroll position
// and listeners. Elements pair up by tag, `id` and `data-nr-key`.
const alike = (a, b) =>
  a.nodeType === b.nodeType &&
  a.nodeName === b.nodeName &&
  (a.nodeType !== 1 || (a.id === b.id && a.getAttribute("data-nr-key") === b.getAttribute("data-nr-key")));

function morph(from, to) {
  let old = from.firstChild;
  for (const next of [...to.childNodes]) {
    let hit = old;
    while (hit && !alike(hit, next)) hit = hit.nextSibling;
    if (!hit) {
      from.insertBefore(document.importNode(next, true), old);
      continue;
    }
    while (old !== hit) {
      old = old.nextSibling;
      old.previousSibling.remove();
    }
    old = hit.nextSibling;
    if (hit.nodeType !== 1) {
      if (hit.data !== next.data) hit.data = next.data;
    } else if (hit.nodeName === "NR-ISLAND") {
      // A hydrated island no longer looks like its markup: new props, new island.
      if (hit.dataset.props !== next.dataset.props) hit.replaceWith(document.importNode(next, true));
    } else {
      for (const { name } of [...hit.attributes]) if (!next.hasAttribute(name)) hit.removeAttribute(name);
      for (const { name, value } of next.attributes) if (hit.getAttribute(name) !== value) hit.setAttribute(name, value);
      morph(hit, next);
      // What the visitor is typing into stays as typed.
      if (hit !== document.activeElement) {
        for (const p of ["value", "checked", "selected"]) if (p in hit && hit[p] !== next[p]) hit[p] = next[p];
      }
    }
  }
  while (old) {
    const gone = old;
    old = old.nextSibling;
    gone.remove();
  }
}

// The field being typed into, found again after the page around it was
// replaced (same `id`, or same `name` in the same position): focus, the text
// as typed and the caret all carry over.
function keepFocus() {
  const el = document.activeElement;
  if (!el || !(el.id || el.name)) return null;
  const { id, name, value, selectionStart, selectionEnd, selectionDirection } = el;
  const at = name ? [...document.getElementsByName(name)].indexOf(el) : 0;
  return () => {
    if (el.isConnected) return el === document.activeElement || el.focus({ preventScroll: true });
    const named = name ? document.getElementsByName(name) : [];
    const found = (id && document.getElementById(id)) || named[at] || named[0];
    if (!found) return;
    found.focus({ preventScroll: true });
    if (typeof selectionStart !== "number" || typeof found.value !== "string") return;
    try {
      found.value = value;
      found.setSelectionRange(selectionStart, selectionEnd, selectionDirection);
    } catch {}
  };
}

// Links marked with active_class / active_class_prefix follow the current URL.
function markActiveLinks() {
  const here = location.pathname.length > 1 ? location.pathname.replace(/\/+$/, "") : location.pathname;
  for (const a of document.querySelectorAll("a[data-nr-active],a[data-nr-active-prefix]")) {
    let path;
    try {
      path = new URL(a.href, location.href).pathname;
    } catch {
      continue;
    }
    if (path.length > 1) path = path.replace(/\/+$/, "");
    const exact = here === path;
    const below = exact || path === "/" || here.startsWith(path + "/");
    let current = false;
    for (const [attr, on] of [["nrActive", exact], ["nrActivePrefix", below]]) {
      const cls = a.dataset[attr];
      if (cls === undefined) continue;
      if (cls) a.classList.toggle(cls, on);
      current = current || on;
    }
    if (current) a.setAttribute("aria-current", "page");
    else if (a.getAttribute("aria-current") === "page") a.removeAttribute("aria-current");
  }
}

// ---- Page fades -------------------------------------------------------------
// The motion stylesheet ([animation] in next-rust.toml) sets --nr-page-fade on
// <html>: how long one page takes to fade into the next, 0 when off or when
// the visitor asked for reduced motion.
const pageFade = () => parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--nr-page-fade")) || 0;
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Run `swap` (which replaces the page) as a fade: a view transition where the
// browser has them, otherwise the body fades out, swaps and fades back in.
async function fade(swap) {
  const ms = pageFade();
  if (!ms) return swap();
  if (document.startViewTransition) {
    const transition = document.startViewTransition(swap);
    // The next navigation may cut this one short: the page is swapped all the same.
    transition.ready.catch(() => {});
    transition.finished.catch(() => {});
    return transition.updateCallbackDone;
  }
  const root = document.documentElement;
  root.setAttribute("data-nr-fade", "out");
  await pause(ms / 2);
  try {
    swap();
  } finally {
    root.setAttribute("data-nr-fade", "in");
    pause(ms / 2).then(() => root.getAttribute("data-nr-fade") === "in" && root.removeAttribute("data-nr-fade"));
  }
}

// ---- Loading bar ------------------------------------------------------------
// [loader] in next-rust.toml draws a bar along the top of the page while the
// next page is on its way: html[data-nr-loading]::before, slid in from the left
// --nr-load of the way by a transform. The stylesheet announces itself with
// --nr-loader and sets the timing; the bar appears only once a load has taken
// --nr-load-delay ms (a prefetched page never flashes it), moves quickly at
// first and ever slower while waiting, then fills and fades (data-nr-loading=
// "done") in --nr-load-speed ms each. `loader.start()` returns the function that
// ends that load; the bar stays while any load is open.
const loader = (() => {
  const root = document.documentElement;
  const css = (name) => getComputedStyle(root).getPropertyValue(name);
  const STEP = 200; // the stylesheet's linear transition lasts a little longer: the bar never rests
  const KEY = "nr-load";
  let open = 0;
  let shown = false;
  let progress = 0;
  let began = 0;
  let delay = 0;
  let creep = 0;
  let hide = 0;
  const set = (p) => {
    progress = p;
    root.style.setProperty("--nr-load", p * 100 + "%");
  };
  // Styles must be computed for a transition to have somewhere to start from.
  const flush = () => getComputedStyle(root, "::before").opacity;
  // Where the bar stands t ms in: a third within half a second, 95% never.
  const curve = (t) => 0.95 - 0.435 * (Math.exp(-t / 600) + Math.exp(-t / 6000));
  // Aim at where it should be by the next step; the transition takes it there.
  const crawl = () => {
    set(Math.max(progress, curve(performance.now() - began + STEP)));
    creep = setTimeout(crawl, STEP);
  };
  // `from`: where a bar taken over from the previous document stood.
  const show = (from) => {
    clearTimeout(hide);
    shown = true;
    // A bar still fading from the last load is dropped, not dragged back.
    root.removeAttribute("data-nr-loading");
    flush();
    set(from || 0);
    root.setAttribute("data-nr-loading", "");
    flush();
    began = performance.now();
    crawl();
  };
  const finish = () => {
    clearTimeout(delay);
    clearTimeout(creep);
    delay = 0;
    if (!shown) return;
    shown = false;
    root.setAttribute("data-nr-loading", "done");
    set(1);
    hide = setTimeout(
      () => {
        root.removeAttribute("data-nr-loading");
        root.style.removeProperty("--nr-load");
      },
      (parseFloat(css("--nr-load-speed")) || 0) * 2 + 50,
    );
  };
  // One load; `limit` ends it on its own after that many ms.
  const start = (limit, from) => {
    if (!css("--nr-loader")) return () => {};
    open++;
    if (from) show(from);
    else if (!shown && !delay) {
      delay = setTimeout(
        () => {
          delay = 0;
          if (open) show();
        },
        parseFloat(css("--nr-load-delay")) || 0,
      );
    }
    let ended = false;
    const done = () => {
      if (ended) return;
      ended = true;
      if (--open <= 0) {
        open = 0;
        finish();
      }
    };
    if (limit) setTimeout(done, limit);
    return done;
  };
  const reset = () => {
    open = 0;
    finish();
  };
  // A full page load: the bar the last document left with carries on in this
  // one from where it stood, and completes once the page has loaded.
  try {
    const left = (sessionStorage.getItem(KEY) || "").split(" ");
    sessionStorage.removeItem(KEY);
    if (left[1] > Date.now() - 35000 && left[0] > 0) {
      const done = start(30000, Math.min(+left[0], 0.95));
      if (document.readyState === "complete") setTimeout(done);
      else window.addEventListener("load", done);
    }
  } catch {}
  window.addEventListener("pagehide", () => {
    try {
      if (shown) sessionStorage.setItem(KEY, progress + " " + Date.now());
    } catch {}
  });
  return { start, reset };
})();
NR.loader = loader;
// A page restored from the back/forward cache still shows the bar it left with.
window.addEventListener("pageshow", (e) => e.persisted && loader.reset());

// A link the browser follows itself (another origin, data-nr-reload): the bar
// runs until the new page takes over, or gives up after a while.
function fullLoad(e) {
  const a = e.target && e.target.closest && e.target.closest("a[href]");
  if (!a || typeof a.href !== "string" || a.hasAttribute("download") || (a.target && a.target !== "_self")) return;
  let url;
  try {
    url = new URL(a.href, location.href);
  } catch {
    return;
  }
  if (!/^https?:$/.test(url.protocol)) return;
  if (url.hash && url.origin === location.origin && url.pathname + url.search === current) return;
  // Once every listener has had its say: a click someone cancels loads nothing.
  setTimeout(() => e.defaultPrevented || loader.start(30000));
}

let navigating = 0;

async function navigate(href, opts = {}) {
  const url = new URL(href, location.href);
  if (url.origin !== location.origin) {
    loader.start(30000);
    location.href = url.href;
    return;
  }
  const target = url.pathname + url.search;
  if (opts.history !== false && !opts.fresh && target === current && url.hash) {
    location.hash = url.hash;
    return;
  }
  const root = document.documentElement;
  const signal = opts.signal;
  // A live search drops the request of the keystroke before: nothing to show.
  const dropped = () => signal && signal.aborted;
  navigating++;
  root.setAttribute("data-nr-navigating", "");
  const loaded = loader.start();
  let page;
  let unsent;
  try {
    if (opts.post) {
      try {
        page = await fetchPage(url.href, false, opts.post);
      } catch (err) {
        unsent = err;
        throw err;
      }
      // What was posted may change what pages show: drop every page kept in memory.
      pages.clear();
      // Not a page (JSON, a file, nothing): the form shows it, unless the
      // server asked for a full page load of where the form led.
      if (page.res && page.res.headers.get("x-nr-reload")) throw 0;
      if (page.res) return page.res;
    } else {
      try {
        page = await loadPage(url.href, opts.fresh, false, signal);
      } catch {
        if (dropped()) return;
        page = await loadPage(url.href, true, false, signal); // a failed prefetch gets one more try
      }
      if (page.from && page.from !== location.pathname) page = await loadPage(url.href, true, false, signal);
    }
    // A partial page needs its layout on screen; if it is gone, get the full page.
    if (page.partial && !findRegion(page.partial)) page = await loadPage(page.url || url.href, true, true, signal);
    if (dropped()) return;
    const finalUrl = new URL(page.url || url.href);
    if (!finalUrl.hash) finalUrl.hash = url.hash;
    // A live search and a form that updates one element change the page in
    // place: no fade, and the page stays where it is scrolled.
    const quiet = opts.quiet || (opts.target && document.querySelector(opts.target));
    await (quiet ? (swap) => swap() : fade)(() => {
      const refocus = opts.focus && keepFocus();
      render(page, opts.target);
      // The same URL again (a search repeated) takes no new history entry.
      if (opts.history !== false) history[opts.replace || finalUrl.href === location.href ? "replaceState" : "pushState"]({ nr: 1 }, "", finalUrl.href);
      if (refocus) refocus();
      current = finalUrl.pathname + finalUrl.search;
      markActiveLinks();
      // A link inside a menu that stays on screen (an open <details>) closes it.
      for (let d = opts.link && opts.link.isConnected && opts.link.closest("details[open]"); d; d = d.parentElement && d.parentElement.closest("details[open]")) {
        d.open = false;
      }
      if (opts.scroll !== false && !quiet) {
        const anchor = url.hash && document.getElementById(decodeURIComponent(url.hash.slice(1)));
        anchor ? anchor.scrollIntoView() : window.scrollTo(0, 0);
      }
    });
    window.dispatchEvent(new CustomEvent("nr:navigate", { detail: { url: finalUrl.href } }));
  } catch (err) {
    if (dropped()) return;
    // A form that never reached the server is the browser's to send.
    if (unsent) throw err;
    location.href = (page && page.url) || url.href;
  } finally {
    if (!--navigating) root.removeAttribute("data-nr-navigating");
    loaded();
  }
}

NR.navigate = (href) => navigate(href);
NR.replace = (href) => navigate(href, { replace: true });
NR.back = () => history.back();
NR.forward = () => history.forward();
NR.refresh = () => navigate(location.href, { replace: true, scroll: false, fresh: true });
NR.prefetch = prefetch;

// Every same-origin link navigates without a full page load, whether it was
// written with Link! or as a plain a![href(..)]. The browser handles a link
// normally when it opens elsewhere (target, download, rel="external"), points
// to another origin or protocol, or opts out with data-nr-reload.
const internalLink = (target) => {
  const a = target && target.closest && target.closest("a[href]");
  if (!a || typeof a.href !== "string") return null;
  if (a.hasAttribute("data-nr-reload") || a.hasAttribute("download")) return null;
  if (a.target && a.target !== "_self") return null;
  if ((a.getAttribute("rel") || "").split(/\s+/).includes("external")) return null;
  let url;
  try {
    url = new URL(a.href, location.href);
  } catch {
    return null;
  }
  if (url.origin !== location.origin || !/^https?:$/.test(url.protocol)) return null;
  return a;
};

document.addEventListener("click", (e) => {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
  const a = internalLink(e.target);
  if (!a) return fullLoad(e);
  e.preventDefault();
  navigate(a.href, { replace: a.hasAttribute("data-nr-replace"), scroll: a.dataset.nrScroll !== "false", link: a });
});

// Prefetch only when the mouse rests on a link for PREFETCH_DELAY ms. Moving
// or scrolling past links, pressing a button and touch taps never fetch early;
// those pages load on click.
const PREFETCH_DELAY = 400;
let hoverLink = null;
let hoverTimer = 0;
const cancelIntent = () => {
  clearTimeout(hoverTimer);
  hoverTimer = 0;
  hoverLink = null;
};
const intent = (e) => {
  if (e.pointerType !== "mouse") return;
  const a = internalLink(e.target);
  if (a === hoverLink) return;
  cancelIntent();
  if (!a || a.dataset.nrPrefetch === "false") return;
  const url = new URL(a.href);
  // Same-page anchors and the page that is already open need no fetch.
  if (url.hash || url.pathname + url.search === current) return;
  hoverLink = a;
  hoverTimer = setTimeout(() => {
    hoverTimer = 0;
    if (hoverLink === a && a.isConnected) prefetch(a.href);
  }, PREFETCH_DELAY);
};
document.addEventListener("pointerover", intent, { passive: true });
document.addEventListener("pointermove", intent, { passive: true });
document.addEventListener("pointerout", (e) => {
  if (hoverLink && !hoverLink.contains(e.relatedTarget)) cancelIntent();
}, { passive: true });
document.addEventListener("pointerdown", cancelIntent, { passive: true });
document.addEventListener("scroll", cancelIntent, { capture: true, passive: true });

window.addEventListener("popstate", () => {
  if (location.pathname + location.search !== current) navigate(location.href, { history: false, scroll: false });
});

// ---- Server actions -------------------------------------------------------

// With `[security] csrf = "token"` the server also expects the `next_rust_csrf`
// cookie echoed in a header.
function withCsrf(headers) {
  const csrf = document.cookie.match(/(?:^|;\s*)next_rust_csrf=([^;]+)/);
  if (csrf) headers["x-csrf-token"] = csrf[1];
  return headers;
}
const actionHeaders = (extra) => withCsrf({ "x-nr-action": "1", accept: "application/json", ...extra });

NR.action = async (url, input) => {
  const res = await fetch(url, {
    method: "POST",
    headers: actionHeaders({ "content-type": "application/json" }),
    body: JSON.stringify(input === undefined ? null : input),
    credentials: "same-origin",
  });
  const body = await res.json().catch(() => ({ ok: false, error: res.statusText }));
  // An action may change what pages show: drop every page kept in memory.
  if (body.ok || body.redirect) pages.clear();
  if (body.redirect) {
    navigate(body.redirect);
    return undefined;
  }
  if (!body.ok) {
    const err = new Error(body.error || "Validation failed");
    err.errors = body.errors;
    err.status = res.status;
    // What the page branches on: an action that refuses for a reason the page
    // can act on (a quota reached, a plan to upgrade, a mode that is blocked)
    // says so in `code`, with whatever goes with it in `data`. `request` and
    // `stream` both carry `data` already; without these two an action's caller
    // has nothing but a sentence of English to match against.
    err.code = body.code;
    err.data = body.data;
    throw err;
  }
  return body.data;
};

// ---- Requests -------------------------------------------------------------

// Same-origin only: the page's cookies and CSRF token travel with every
// request, so they must never reach another site. Use `fetch` for those.
const sameOrigin = (url) => {
  const u = new URL(url, location.href);
  if (u.origin !== location.origin) throw new Error("[next-rust] same-origin URLs only: " + u.origin);
  return u.href;
};

const requestInit = (options) => {
  const { method = "GET", body, headers, signal } = options;
  const init = { method: method.toUpperCase(), headers: actionHeaders(headers || {}), credentials: "same-origin", signal };
  delete init.headers["x-nr-action"];
  if (body !== undefined && body !== null) {
    const raw =
      typeof body === "string" ||
      body instanceof FormData ||
      body instanceof URLSearchParams ||
      body instanceof Blob ||
      body instanceof ArrayBuffer;
    if (raw) init.body = body;
    else {
      init.headers["content-type"] = "application/json";
      init.body = JSON.stringify(body);
    }
  }
  return init;
};

// A request to this site: JSON in, JSON (or text) out, an Error with
// `.status` and `.data` when the response is not 2xx. `timeout` in ms;
// `as: "blob" | "text" | "response"` for other kinds of body.
NR.request = async (url, options = {}) => {
  const init = requestInit(options);
  let timer = 0;
  if (options.timeout && !options.signal) {
    const controller = new AbortController();
    init.signal = controller.signal;
    timer = setTimeout(() => controller.abort(), options.timeout);
  }
  try {
    const res = await fetch(sameOrigin(url), init);
    if (init.method !== "GET" && init.method !== "HEAD") pages.clear();
    const type = res.headers.get("content-type") || "";
    const as = res.ok ? options.as : undefined;
    if (as === "response") return res;
    const data =
      res.status === 204
        ? null
        : as === "blob"
          ? await res.blob()
          : as !== "text" && type.includes("json")
            ? await res.json().catch(() => null)
            : await res.text();
    if (!res.ok) {
      const message = (data && typeof data === "object" && (data.error || data.message)) || res.statusText || "Request failed";
      const err = new Error(message);
      err.status = res.status;
      err.data = data;
      throw err;
    }
    return data;
  } finally {
    clearTimeout(timer);
  }
};

// Server-sent events from this site (`Response::sse`), as an async iterator
// of `{ event, data, id }`; `data` is the raw text of the event.
//   for await (const ev of nextRust.stream(url, { method: "POST", body })) { … }
NR.stream = async function* (url, options = {}) {
  const init = requestInit(options);
  init.headers.accept = "text/event-stream";
  const res = await fetch(sameOrigin(url), init);
  if (!res.ok || !res.body) {
    const err = new Error(res.statusText || "Stream failed");
    err.status = res.status;
    err.data = await res.text().catch(() => "");
    throw err;
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    for (;;) {
      const { value, done } = await reader.read();
      buffer += done ? "\n\n" : decoder.decode(value, { stream: true });
      let at;
      while ((at = buffer.search(/\r?\n\r?\n/)) >= 0) {
        const block = buffer.slice(0, at);
        buffer = buffer.slice(at).replace(/^\r?\n\r?\n/, "");
        const ev = { event: "message", data: [], id: undefined };
        for (const line of block.split(/\r?\n/)) {
          if (!line || line.startsWith(":")) continue;
          const colon = line.indexOf(":");
          const field = colon < 0 ? line : line.slice(0, colon);
          const value = colon < 0 ? "" : line.slice(colon + 1).replace(/^ /, "");
          if (field === "event") ev.event = value;
          else if (field === "data") ev.data.push(value);
          else if (field === "id") ev.id = value;
        }
        if (ev.data.length) yield { event: ev.event, data: ev.data.join("\n"), id: ev.id };
      }
      if (done) return;
    }
  } finally {
    reader.cancel().catch(() => {});
  }
};

// ---- Forms ----------------------------------------------------------------
// No form reloads the page. A form that goes to this site is sent from here:
// a GET form (a search box, filters, a picker) becomes a navigation to the URL
// it builds, a POST form is posted and the answer shown (the page it redirects
// to, the page it renders, or its JSON through the same events and
// `data-nr-error` elements as a server action). The browser keeps the forms it
// alone can do: another origin, `target`, `method="dialog"`, `text/plain`,
// and any form or button marked `data-nr-reload` / `data-nr-native`.
// `[client] enhance_forms = false` leaves every form but server actions alone.
const enhanceForms = !document.getElementById("__next_rust_forms");
const nativeSubmit = HTMLFormElement.prototype.submit;
const sending = new WeakMap(); // form -> the AbortController of what it is sending
const liveTimers = new WeakMap();
let live = false;

// An attribute of the form that its submit button may override (`formaction`, ...).
const formAttr = (form, submitter, name) => (submitter && submitter.getAttribute("form" + name)) || form.getAttribute(name) || "";

// What sending `form` means, or null when the browser should do it.
function formRequest(form, submitter) {
  for (const el of [form, submitter]) {
    if (el && ["data-nr-reload", "data-nr-native", "download"].some((a) => el.hasAttribute(a))) return null;
  }
  const action = form.dataset.nrAction;
  const target = formAttr(form, submitter, "target");
  const method = formAttr(form, submitter, "method").toLowerCase();
  const enctype = formAttr(form, submitter, "enctype").toLowerCase();
  let url;
  if (action) {
    // Server actions take no files.
    if (form.enctype === "multipart/form-data") return null;
  } else {
    if (!enhanceForms || (target && target !== "_self") || method === "dialog" || enctype === "text/plain") return null;
    try {
      url = new URL(formAttr(form, submitter, "action") || location.href, location.href);
    } catch {
      return null;
    }
    if (url.origin !== location.origin || !/^https?:$/.test(url.protocol)) return null;
  }
  // Only now: building the data tells the page (`formdata`), once per submission.
  let data;
  try {
    data = new FormData(form, submitter);
  } catch {
    data = new FormData(form);
  }
  // A file in a form that cannot carry files goes by its name, as browsers do.
  const fields = new URLSearchParams([...data].map(([k, v]) => [k, typeof v === "string" ? v : v.name]));
  if (action) return { action, body: fields };
  if (method === "post") return { url, body: enctype === "multipart/form-data" ? data : fields };
  url.search = fields;
  return { url };
}

function formSucceeded(form, data, next) {
  form.dataset.nrState = "success";
  form.dispatchEvent(new CustomEvent("nr:success", { detail: data, bubbles: true }));
  // `data-nr-stay`: show the result in place; `_redirect` is then only
  // where browsers without JavaScript go.
  if (form.hasAttribute("data-nr-stay")) {
    for (const el of form.querySelectorAll("[data-nr-result]")) {
      const key = el.dataset.nrResult;
      const value = key && data && typeof data === "object" ? data[key] : data;
      el.textContent = value == null || typeof value === "object" ? "" : String(value);
    }
    if (form.hasAttribute("data-nr-reset")) form.reset();
    return;
  }
  return next ? navigate(next) : NR.refresh();
}

function formFailed(form, body) {
  form.dataset.nrState = "error";
  const errors = body.errors || { _form: body.error };
  for (const [field, message] of Object.entries(errors)) {
    const el = form.querySelector(`[data-nr-error="${CSS.escape(field)}"]`);
    if (el) el.textContent = message;
  }
  form.dispatchEvent(new CustomEvent("nr:error", { detail: body, bubbles: true }));
}

// The answer to a posted form that is not a page: a file is saved, anything
// else reaches the form the way a server action's result does.
async function formAnswer(form, res, next) {
  const disposition = res.headers.get("content-disposition") || "";
  if (/attachment/i.test(disposition)) {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(await res.blob());
    a.download = (disposition.match(/filename="?([^";]+)/i) || [])[1] || "";
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 60000);
    return;
  }
  const text = await res.text();
  let data = text;
  try {
    data = JSON.parse(text);
  } catch {}
  if (res.ok) return formSucceeded(form, text ? data : null, next);
  const body = data && typeof data === "object" ? data : { error: text };
  body.error = body.error || body.message || res.statusText || "Request failed";
  body.status = res.status;
  formFailed(form, body);
}

async function sendForm(form, submitter, req, quiet) {
  const before = sending.get(form);
  if (before) {
    // Sent twice (a double click, Enter held down): once is enough. Only a
    // live search gives way, to the newer text.
    if (!before.live) return;
    before.abort();
  }
  const controller = new AbortController();
  controller.live = quiet;
  sending.set(form, controller);
  clearTimeout(liveTimers.get(form));
  form.querySelectorAll("[data-nr-error],[data-nr-result]").forEach((el) => (el.textContent = ""));
  delete form.dataset.nrState;
  form.setAttribute("aria-busy", "true");
  const held = submitter && !submitter.disabled && submitter === document.activeElement;
  const disabled = submitter && !submitter.disabled && (submitter.disabled = true);
  const next = req.body && !(req.body instanceof FormData) && req.body.get("_redirect");
  const loaded = req.action ? loader.start() : () => {};
  try {
    if (req.action) {
      const res = await fetch(req.action, { method: "POST", body: req.body, headers: actionHeaders(), credentials: "same-origin" });
      const body = await res.json();
      if (body.ok || body.redirect) pages.clear();
      if (body.redirect) return navigate(body.redirect);
      if (body.ok) return formSucceeded(form, body.data, next);
      return formFailed(form, body);
    }
    const scroll = [form, submitter].every((el) => !el || el.dataset.nrScroll !== "false");
    const res = await navigate(req.url.href, {
      post: req.body && { method: "POST", body: req.body, signal: controller.signal },
      signal: controller.signal,
      fresh: true,
      focus: !req.body,
      replace: quiet || form.hasAttribute("data-nr-replace"),
      scroll,
      quiet,
      target: form.dataset.nrTarget,
    });
    if (res) await formAnswer(form, res, next);
  } catch {
    if (!controller.signal.aborted) nativeSubmit.call(form);
  } finally {
    if (sending.get(form) === controller) {
      sending.delete(form);
      form.removeAttribute("aria-busy");
    }
    if (disabled) {
      submitter.disabled = false;
      if (held && submitter.isConnected && document.activeElement === document.body) submitter.focus({ preventScroll: true });
    }
    loaded();
  }
}

// On `window`, so every listener on the form or the document has had its say
// by now: a submit the page cancels (to handle it itself) is left alone.
window.addEventListener("submit", (e) => {
  const form = e.target;
  if (!(form instanceof HTMLFormElement) || e.defaultPrevented) return;
  const req = formRequest(form, e.submitter);
  if (req) {
    e.preventDefault();
    if (!live || !req.body) sendForm(form, e.submitter, req, live);
  } else if (!live) {
    // The browser submits this one itself: a full page load, with the bar.
    const target = formAttr(form, e.submitter, "target");
    if ((!target || target === "_self") && formAttr(form, e.submitter, "method").toLowerCase() !== "dialog") {
      setTimeout(() => e.defaultPrevented || loader.start(30000));
    }
  }
});

// `form.submit()` fires no submit event; it still must not reload the page.
HTMLFormElement.prototype.submit = function () {
  const req = formRequest(this);
  req ? sendForm(this, null, req) : nativeSubmit.call(this);
};

// Live search: a GET form marked `data-nr-live` (or the field itself) is sent
// as the visitor types, `data-nr-debounce` (or `data-nr-live="ms"`, else 250)
// ms after the last change.
// The previous request is dropped, history is replaced, focus stays.
document.addEventListener("input", (e) => {
  const el = e.target;
  const form = el && el.form;
  if (!form || e.isComposing || !(form.hasAttribute("data-nr-live") || el.hasAttribute("data-nr-live"))) return;
  clearTimeout(liveTimers.get(form));
  liveTimers.set(
    form,
    setTimeout(() => {
      if (!form.isConnected || !(form.noValidate || form.checkValidity())) return;
      live = true;
      try {
        // A submit event, so listeners that validate or cancel still get their say.
        form.dispatchEvent(new SubmitEvent("submit", { bubbles: true, cancelable: true }));
      } finally {
        live = false;
      }
    }, +(el.dataset.nrDebounce || form.dataset.nrDebounce || el.dataset.nrLive || form.dataset.nrLive || 250)),
  );
});

// ---- Islands --------------------------------------------------------------

try {
  NR.env = JSON.parse((document.getElementById("__next_rust_env") || {}).textContent || "{}");
} catch {
  NR.env = {};
}

// Inline delivery (`[build] inline_assets`): the ids of the scripts written
// into the document, and island modules taken from the page's registry
// (`window.__nrm`, filled by those scripts) before the network.
const inlined = new Set([...document.querySelectorAll("script[data-nr-js]")].map((s) => s.dataset.nrJs));
// `data-module` may be built from application data, so a module is imported
// only when it is a path on this origin: another origin (or "//host", or a
// scheme) would run someone else's code in the page.
const moduleUrl = (src) => {
  try {
    const u = new URL(src, location.origin);
    if (/^\/[^/\\]/.test(src) && u.origin === location.origin) return u.href;
  } catch {}
  console.error(`[next-rust] island module refused: ${src} is not a same-origin path starting with "/"`);
  return null;
};
const loadModule = (src) => {
  const url = moduleUrl(src);
  if (!url) return null;
  // The registry has a null prototype; the own-property check keeps a key like
  // "constructor" from resolving to a member of Object.prototype regardless.
  const registry = window.__nrm;
  const key = src.replace(/^\/_next-rust\//, "").split("?")[0];
  return registry && Object.hasOwn(registry.f, key) ? registry.load(key) : import(url);
};

const hydrated = new WeakSet();

async function hydrateIslands(root) {
  for (const el of root.querySelectorAll("nr-island")) {
    if (hydrated.has(el)) continue;
    hydrated.add(el);
    let props = {};
    try {
      props = JSON.parse(el.dataset.props || "{}");
    } catch {}
    if (el.dataset.module) {
      try {
        const mod = await loadModule(el.dataset.module);
        if (mod && typeof mod.hydrate === "function") {
          await mod.hydrate(el, props);
          continue;
        }
      } catch (err) {
        console.error(`[next-rust] island ${el.dataset.component || el.dataset.module} failed to load`, err);
      }
    }
    bindDeclarative(el, props);
  }
}

// Declarative behaviour: data-nr-text, data-nr-show, data-nr-bind,
// data-nr-class-<name>, data-nr-on-<event>="increment:count; set:open=false".
function bindDeclarative(root, state) {
  const read = (path) => path.split(".").reduce((o, k) => (o == null ? o : o[k]), state);
  const write = (path, value) => {
    const keys = path.split(".");
    const last = keys.pop();
    keys.reduce((o, k) => (o[k] = o[k] && typeof o[k] === "object" ? o[k] : {}), state)[last] = value;
  };
  const truthy = (expr) => (expr.startsWith("!") ? !read(expr.slice(1).trim()) : !!read(expr.trim()));
  const parse = (v) => {
    try {
      return JSON.parse(v);
    } catch {
      return v;
    }
  };
  const update = () => {
    for (const el of root.querySelectorAll("[data-nr-text]")) {
      const v = read(el.dataset.nrText);
      el.textContent = v == null ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    }
    for (const el of root.querySelectorAll("[data-nr-show]")) el.hidden = !truthy(el.dataset.nrShow);
    for (const el of root.querySelectorAll("[data-nr-bind]")) {
      const v = read(el.dataset.nrBind);
      if (el.type === "checkbox") el.checked = !!v;
      else if (document.activeElement !== el) el.value = v == null ? "" : v;
    }
    for (const el of root.querySelectorAll("*"))
      for (const a of el.attributes) if (a.name.startsWith("data-nr-class-")) el.classList.toggle(a.name.slice(14), truthy(a.value));
  };
  const ops = {
    increment: (k, n) => write(k, Number(read(k) || 0) + Number(n || 1)),
    decrement: (k, n) => write(k, Number(read(k) || 0) - Number(n || 1)),
    toggle: (k) => write(k, !read(k)),
  };
  const types = new Set();
  for (const el of root.querySelectorAll("*"))
    for (const a of el.attributes) if (a.name.startsWith("data-nr-on-")) types.add(a.name.slice(11));
  for (const type of types) {
    root.addEventListener(type, async (e) => {
      const el = e.target.closest && e.target.closest(`[data-nr-on-${type}]`);
      if (!el || !root.contains(el)) return;
      for (const stmt of el.getAttribute(`data-nr-on-${type}`).split(";")) {
        const i = stmt.indexOf(":");
        const op = (i < 0 ? stmt : stmt.slice(0, i)).trim();
        const arg = i < 0 ? "" : stmt.slice(i + 1).trim();
        if (op === "prevent") e.preventDefault();
        else if (op === "set") {
          const j = arg.indexOf("=");
          write(arg.slice(0, j).trim(), parse(arg.slice(j + 1).trim()));
        } else if (op === "action") {
          const [url, target] = arg.split("->").map((s) => s.trim());
          try {
            const result = await NR.action(url, state);
            if (target) write(target, result);
            else if (result && typeof result === "object") Object.assign(state, result);
          } catch (err) {
            write("error", err.message);
          }
        } else if (op === "navigate") navigate(arg);
        else if (ops[op]) {
          const [k, n] = arg.split(",");
          ops[op](k.trim(), n);
        }
      }
      update();
    });
  }
  root.addEventListener("input", (e) => {
    const el = e.target;
    if (!el.dataset || !el.dataset.nrBind) return;
    write(el.dataset.nrBind, el.type === "checkbox" ? el.checked : el.type === "number" ? Number(el.value) : el.value);
    update();
  });
  update();
}

markActiveLinks();
hydrateIslands(document);
"#;

/// Minified, name-mangled runtime served in production.
pub const RUNTIME_JS_MIN: &str = r####"const t=window.nextRust=window.nextRust||{},e=new Map;let n=location.pathname+location.search;function r(t){const e=document.createTreeWalker(document.body,NodeFilter.SHOW_COMMENT);let n=null;for(let r=e.nextNode();r;r=e.nextNode())if(r.data==="nr-l:"+t)n=r;else if(n&&r.data==="/nr-l:"+t&&r.parentNode===n.parentNode)return{start:n,end:r};return null}async function o(t,e,n){const r={"x-nr-nav":"1","x-nr-from":location.pathname,accept:"text/html"};n&&n.body&&x(r);const o=e?[]:function(){const t=document.createTreeWalker(document.body,NodeFilter.SHOW_COMMENT),e=[];for(let n=t.nextNode();n;n=t.nextNode())n.data.startsWith("nr-l:")&&e.push(n);return e}().map(t=>t.data.slice(5));o.length&&(r["x-nr-layouts"]=o.join(","));const a=[...document.querySelectorAll("style[data-nr-css]")].map(t=>t.dataset.nrCss);a.length&&(r["x-nr-styles"]=a.join(",")),D.size&&(r["x-nr-scripts"]=[...D].join(","));const s=await fetch(t,{headers:r,credentials:"same-origin",...n});if(!(s.headers.get("content-type")||"").includes("text/html")){if(n&&n.body)return{res:s,url:s.url};throw new Error("not html")}return{url:s.url,html:await s.text(),from:s.headers.get("x-nr-intercepted")?location.pathname:null,partial:s.headers.get("x-nr-partial")}}function a(t,n,r,a){const s=Date.now();for(const[t,n]of e)s-n.at>=3e4&&e.delete(t);const i=n?null:e.get(t);if(i)return i.promise;const c=o(t,r,{signal:a}),l={at:s,promise:c};return e.set(t,l),c.catch(()=>{e.get(t)===l&&e.delete(t)}),c}const s=t=>a(t,!1);function i(n,o){const a=(new DOMParser).parseFromString(n.html,"text/html");!function(t){for(const e of t.querySelectorAll('template[id^="nr-t"]')){const n=t.getElementById("nr-b"+e.id.slice(4));n&&n.replaceWith(e.content),e.remove()}}(a),document.title=a.title;const s='meta[name]:not([name="viewport"]),meta[property],link[rel="canonical"],style[data-nr-css]';for(const t of document.body.querySelectorAll("style[data-nr-css]"))document.head.appendChild(t);const i=new Map([...document.head.querySelectorAll("style[data-nr-css]")].map(t=>[t.dataset.nrCss,t]));document.head.querySelectorAll(s.replace(",style[data-nr-css]","")).forEach(t=>t.remove());let c=!1;for(const t of[...a.head.querySelectorAll(s),...a.body.querySelectorAll("style[data-nr-css]")]){const e=t.dataset&&t.dataset.nrCss;e&&e.includes("~")&&(c=!0),e&&i.has(e)?e.includes("~")&&document.head.appendChild(i.get(e)):document.head.appendChild(document.importNode(t,!0)),t.parentNode!==a.head&&t.remove()}c&&e.clear();const d=a.getElementById("__next_rust_env");if(d)try{t.env=JSON.parse(d.textContent||"{}")}catch{}for(const t of a.querySelectorAll("script[data-nr-js]")){const e=t.dataset.nrJs;if(D.has(e)&&!t.hasAttribute("data-nr-rerun")){t.remove();continue}if(D.add(e),t.parentNode!==a.head)continue;const n=document.createElement("script");for(const e of t.attributes)n.setAttribute(e.name,e.value);n.textContent=t.textContent,document.head.appendChild(n)}let u=[];const f=n.partial&&r(n.partial),m=o&&document.querySelector(o),h=m&&a.querySelector(o);if(h)l(m,h);else if(f){const{start:t,end:e}=f;for(;t.nextSibling&&t.nextSibling!==e;)t.nextSibling.remove();const n=[];for(const t of[...a.body.childNodes]){if(1===t.nodeType&&("__next_rust_env"===t.id||(t.getAttribute("src")||"").startsWith("/_next-rust/runtime.js")))continue;const r=document.importNode(t,!0);e.parentNode.insertBefore(r,e),n.push(r)}u=n.flatMap(t=>1!==t.nodeType?[]:"SCRIPT"===t.tagName?[t]:[...t.querySelectorAll("script")])}else document.body.replaceWith(document.importNode(a.body,!0)),u=[...document.body.querySelectorAll("script")];for(const t of u){const e=t.getAttribute("src")||"";if("application/json"===t.type||e.startsWith("/_next-rust/runtime.js")||/^\$nr\(/.test(t.textContent))continue;const n=document.createElement("script");for(const e of t.attributes)n.setAttribute(e.name,e.value);n.textContent=t.textContent,t.replaceWith(n)}M(document)}const c=(t,e)=>t.nodeType===e.nodeType&&t.nodeName===e.nodeName&&(1!==t.nodeType||t.id===e.id&&t.getAttribute("data-nr-key")===e.getAttribute("data-nr-key"));function l(t,e){let n=t.firstChild;for(const r of[...e.childNodes]){let e=n;for(;e&&!c(e,r);)e=e.nextSibling;if(e){for(;n!==e;)n=n.nextSibling,n.previousSibling.remove();if(n=e.nextSibling,1!==e.nodeType)e.data!==r.data&&(e.data=r.data);else if("NR-ISLAND"===e.nodeName)e.dataset.props!==r.dataset.props&&e.replaceWith(document.importNode(r,!0));else{for(const{name:t}of[...e.attributes])r.hasAttribute(t)||e.removeAttribute(t);for(const{name:t,value:n}of r.attributes)e.getAttribute(t)!==n&&e.setAttribute(t,n);if(l(e,r),e!==document.activeElement)for(const t of["value","checked","selected"])t in e&&e[t]!==r[t]&&(e[t]=r[t])}}else t.insertBefore(document.importNode(r,!0),n)}for(;n;){const t=n;n=n.nextSibling,t.remove()}}function d(){const t=location.pathname.length>1?location.pathname.replace(/\/+$/,""):location.pathname;for(const e of document.querySelectorAll("a[data-nr-active],a[data-nr-active-prefix]")){let n;try{n=new URL(e.href,location.href).pathname}catch{continue}n.length>1&&(n=n.replace(/\/+$/,""));const r=t===n,o=r||"/"===n||t.startsWith(n+"/");let a=!1;for(const[t,n]of[["nrActive",r],["nrActivePrefix",o]]){const r=e.dataset[t];void 0!==r&&(r&&e.classList.toggle(r,n),a=a||n)}a?e.setAttribute("aria-current","page"):"page"===e.getAttribute("aria-current")&&e.removeAttribute("aria-current")}}const u=()=>parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--nr-page-fade"))||0,f=t=>new Promise(e=>setTimeout(e,t));async function m(t){const e=u();if(!e)return t();if(document.startViewTransition){const e=document.startViewTransition(t);return e.ready.catch(()=>{}),e.finished.catch(()=>{}),e.updateCallbackDone}const n=document.documentElement;n.setAttribute("data-nr-fade","out"),await f(e/2);try{t()}finally{n.setAttribute("data-nr-fade","in"),f(e/2).then(()=>"in"===n.getAttribute("data-nr-fade")&&n.removeAttribute("data-nr-fade"))}}const h=(()=>{const t=document.documentElement,e=e=>getComputedStyle(t).getPropertyValue(e),n="nr-load";let r=0,o=!1,a=0,s=0,i=0,c=0,l=0;const d=e=>{a=e,t.style.setProperty("--nr-load",100*e+"%")},u=()=>getComputedStyle(t,"::before").opacity,f=()=>{var t;d(Math.max(a,(t=performance.now()-s+200,.95-.435*(Math.exp(-t/600)+Math.exp(-t/6e3))))),c=setTimeout(f,200)},m=e=>{clearTimeout(l),o=!0,t.removeAttribute("data-nr-loading"),u(),d(e||0),t.setAttribute("data-nr-loading",""),u(),s=performance.now(),f()},h=()=>{clearTimeout(i),clearTimeout(c),i=0,o&&(o=!1,t.setAttribute("data-nr-loading","done"),d(1),l=setTimeout(()=>{t.removeAttribute("data-nr-loading"),t.style.removeProperty("--nr-load")},2*(parseFloat(e("--nr-load-speed"))||0)+50))},p=(t,n)=>{if(!e("--nr-loader"))return()=>{};r++,n?m(n):o||i||(i=setTimeout(()=>{i=0,r&&m()},parseFloat(e("--nr-load-delay"))||0));let a=!1;const s=()=>{a||(a=!0,--r<=0&&(r=0,h()))};return t&&setTimeout(s,t),s};try{const t=(sessionStorage.getItem(n)||"").split(" ");if(sessionStorage.removeItem(n),t[1]>Date.now()-35e3&&t[0]>0){const e=p(3e4,Math.min(+t[0],.95));"complete"===document.readyState?setTimeout(e):window.addEventListener("load",e)}}catch{}return window.addEventListener("pagehide",()=>{try{o&&sessionStorage.setItem(n,a+" "+Date.now())}catch{}}),{start:p,reset:()=>{r=0,h()}}})();t.loader=h,window.addEventListener("pageshow",t=>t.persisted&&h.reset());let p=0;async function y(t,s={}){const c=new URL(t,location.href);if(c.origin!==location.origin)return h.start(3e4),void(location.href=c.href);const l=c.pathname+c.search;if(!1!==s.history&&!s.fresh&&l===n&&c.hash)return void(location.hash=c.hash);const u=document.documentElement,f=s.signal,y=()=>f&&f.aborted;p++,u.setAttribute("data-nr-navigating","");const g=h.start();let b,v;try{if(s.post){try{b=await o(c.href,!1,s.post)}catch(t){throw v=t,t}if(e.clear(),b.res&&b.res.headers.get("x-nr-reload"))throw 0;if(b.res)return b.res}else{try{b=await a(c.href,s.fresh,!1,f)}catch{if(y())return;b=await a(c.href,!0,!1,f)}b.from&&b.from!==location.pathname&&(b=await a(c.href,!0,!1,f))}if(b.partial&&!r(b.partial)&&(b=await a(b.url||c.href,!0,!0,f)),y())return;const t=new URL(b.url||c.href);t.hash||(t.hash=c.hash);const l=s.quiet||s.target&&document.querySelector(s.target);await(l?t=>t():m)(()=>{const e=s.focus&&function(){const t=document.activeElement;if(!t||!t.id&&!t.name)return null;const{id:e,name:n,value:r,selectionStart:o,selectionEnd:a,selectionDirection:s}=t,i=n?[...document.getElementsByName(n)].indexOf(t):0;return()=>{if(t.isConnected)return t===document.activeElement||t.focus({preventScroll:!0});const c=n?document.getElementsByName(n):[],l=e&&document.getElementById(e)||c[i]||c[0];if(l&&(l.focus({preventScroll:!0}),"number"==typeof o&&"string"==typeof l.value))try{l.value=r,l.setSelectionRange(o,a,s)}catch{}}}();i(b,s.target),!1!==s.history&&history[s.replace||t.href===location.href?"replaceState":"pushState"]({nr:1},"",t.href),e&&e(),n=t.pathname+t.search,d();for(let t=s.link&&s.link.isConnected&&s.link.closest("details[open]");t;t=t.parentElement&&t.parentElement.closest("details[open]"))t.open=!1;if(!1!==s.scroll&&!l){const t=c.hash&&document.getElementById(decodeURIComponent(c.hash.slice(1)));t?t.scrollIntoView():window.scrollTo(0,0)}}),window.dispatchEvent(new CustomEvent("nr:navigate",{detail:{url:t.href}}))}catch(t){if(y())return;if(v)throw t;location.href=b&&b.url||c.href}finally{--p||u.removeAttribute("data-nr-navigating"),g()}}t.navigate=t=>y(t),t.replace=t=>y(t,{replace:!0}),t.back=()=>history.back(),t.forward=()=>history.forward(),t.refresh=()=>y(location.href,{replace:!0,scroll:!1,fresh:!0}),t.prefetch=s;const g=t=>{const e=t&&t.closest&&t.closest("a[href]");if(!e||"string"!=typeof e.href)return null;if(e.hasAttribute("data-nr-reload")||e.hasAttribute("download"))return null;if(e.target&&"_self"!==e.target)return null;if((e.getAttribute("rel")||"").split(/\s+/).includes("external"))return null;let n;try{n=new URL(e.href,location.href)}catch{return null}return n.origin===location.origin&&/^https?:$/.test(n.protocol)?e:null};document.addEventListener("click",t=>{if(t.defaultPrevented||0!==t.button||t.metaKey||t.ctrlKey||t.shiftKey||t.altKey)return;const e=g(t.target);if(!e)return function(t){const e=t.target&&t.target.closest&&t.target.closest("a[href]");if(!e||"string"!=typeof e.href||e.hasAttribute("download")||e.target&&"_self"!==e.target)return;let r;try{r=new URL(e.href,location.href)}catch{return}/^https?:$/.test(r.protocol)&&(r.hash&&r.origin===location.origin&&r.pathname+r.search===n||setTimeout(()=>t.defaultPrevented||h.start(3e4)))}(t);t.preventDefault(),y(e.href,{replace:e.hasAttribute("data-nr-replace"),scroll:"false"!==e.dataset.nrScroll,link:e})});let b=null,v=0;const w=()=>{clearTimeout(v),v=0,b=null},S=t=>{if("mouse"!==t.pointerType)return;const e=g(t.target);if(e===b)return;if(w(),!e||"false"===e.dataset.nrPrefetch)return;const r=new URL(e.href);r.hash||r.pathname+r.search===n||(b=e,v=setTimeout(()=>{v=0,b===e&&e.isConnected&&s(e.href)},400))};function x(t){const e=document.cookie.match(/(?:^|;\s*)next_rust_csrf=([^;]+)/);return e&&(t["x-csrf-token"]=e[1]),t}document.addEventListener("pointerover",S,{passive:!0}),document.addEventListener("pointermove",S,{passive:!0}),document.addEventListener("pointerout",t=>{b&&!b.contains(t.relatedTarget)&&w()},{passive:!0}),document.addEventListener("pointerdown",w,{passive:!0}),document.addEventListener("scroll",w,{capture:!0,passive:!0}),window.addEventListener("popstate",()=>{location.pathname+location.search!==n&&y(location.href,{history:!1,scroll:!1})});const A=t=>x({"x-nr-action":"1",accept:"application/json",...t});t.action=async(t,n)=>{const r=await fetch(t,{method:"POST",headers:A({"content-type":"application/json"}),body:JSON.stringify(void 0===n?null:n),credentials:"same-origin"}),o=await r.json().catch(()=>({ok:!1,error:r.statusText}));if((o.ok||o.redirect)&&e.clear(),!o.redirect){if(!o.ok){const t=new Error(o.error||"Validation failed");throw t.errors=o.errors,t.status=r.status,t.code=o.code,t.data=o.data,t}return o.data}y(o.redirect)};const E=t=>{const e=new URL(t,location.href);if(e.origin!==location.origin)throw new Error("[next-rust] same-origin URLs only: "+e.origin);return e.href},T=t=>{const{method:e="GET",body:n,headers:r,signal:o}=t,a={method:e.toUpperCase(),headers:A(r||{}),credentials:"same-origin",signal:o};return delete a.headers["x-nr-action"],null!=n&&("string"==typeof n||n instanceof FormData||n instanceof URLSearchParams||n instanceof Blob||n instanceof ArrayBuffer?a.body=n:(a.headers["content-type"]="application/json",a.body=JSON.stringify(n))),a};t.request=async(t,n={})=>{const r=T(n);let o=0;if(n.timeout&&!n.signal){const t=new AbortController;r.signal=t.signal,o=setTimeout(()=>t.abort(),n.timeout)}try{const o=await fetch(E(t),r);"GET"!==r.method&&"HEAD"!==r.method&&e.clear();const a=o.headers.get("content-type")||"",s=o.ok?n.as:void 0;if("response"===s)return o;const i=204===o.status?null:"blob"===s?await o.blob():"text"!==s&&a.includes("json")?await o.json().catch(()=>null):await o.text();if(!o.ok){const t=i&&"object"==typeof i&&(i.error||i.message)||o.statusText||"Request failed",e=new Error(t);throw e.status=o.status,e.data=i,e}return i}finally{clearTimeout(o)}},t.stream=async function*(t,e={}){const n=T(e);n.headers.accept="text/event-stream";const r=await fetch(E(t),n);if(!r.ok||!r.body){const t=new Error(r.statusText||"Stream failed");throw t.status=r.status,t.data=await r.text().catch(()=>""),t}const o=r.body.getReader(),a=new TextDecoder;let s="";try{for(;;){const{value:t,done:e}=await o.read();let n;for(s+=e?"\n\n":a.decode(t,{stream:!0});(n=s.search(/\r?\n\r?\n/))>=0;){const t=s.slice(0,n);s=s.slice(n).replace(/^\r?\n\r?\n/,"");const e={event:"message",data:[],id:void 0};for(const n of t.split(/\r?\n/)){if(!n||n.startsWith(":"))continue;const t=n.indexOf(":"),r=t<0?n:n.slice(0,t),o=t<0?"":n.slice(t+1).replace(/^ /,"");"event"===r?e.event=o:"data"===r?e.data.push(o):"id"===r&&(e.id=o)}e.data.length&&(yield{event:e.event,data:e.data.join("\n"),id:e.id})}if(e)return}}finally{o.cancel().catch(()=>{})}};const C=!document.getElementById("__next_rust_forms"),L=HTMLFormElement.prototype.submit,N=new WeakMap,k=new WeakMap;let _=!1;const q=(t,e,n)=>e&&e.getAttribute("form"+n)||t.getAttribute(n)||"";function j(t,e){for(const n of[t,e])if(n&&["data-nr-reload","data-nr-native","download"].some(t=>n.hasAttribute(t)))return null;const n=t.dataset.nrAction,r=q(t,e,"target"),o=q(t,e,"method").toLowerCase(),a=q(t,e,"enctype").toLowerCase();let s,i;if(n){if("multipart/form-data"===t.enctype)return null}else{if(!C||r&&"_self"!==r||"dialog"===o||"text/plain"===a)return null;try{s=new URL(q(t,e,"action")||location.href,location.href)}catch{return null}if(s.origin!==location.origin||!/^https?:$/.test(s.protocol))return null}try{i=new FormData(t,e)}catch{i=new FormData(t)}const c=new URLSearchParams([...i].map(([t,e])=>[t,"string"==typeof e?e:e.name]));return n?{action:n,body:c}:"post"===o?{url:s,body:"multipart/form-data"===a?i:c}:(s.search=c,{url:s})}function O(e,n,r){if(e.dataset.nrState="success",e.dispatchEvent(new CustomEvent("nr:success",{detail:n,bubbles:!0})),!e.hasAttribute("data-nr-stay"))return r?y(r):t.refresh();for(const t of e.querySelectorAll("[data-nr-result]")){const e=t.dataset.nrResult,r=e&&n&&"object"==typeof n?n[e]:n;t.textContent=null==r||"object"==typeof r?"":String(r)}e.hasAttribute("data-nr-reset")&&e.reset()}function R(t,e){t.dataset.nrState="error";const n=e.errors||{_form:e.error};for(const[e,r]of Object.entries(n)){const n=t.querySelector(`[data-nr-error="${CSS.escape(e)}"]`);n&&(n.textContent=r)}t.dispatchEvent(new CustomEvent("nr:error",{detail:e,bubbles:!0}))}async function W(t,n,r,o){const a=N.get(t);if(a){if(!a.live)return;a.abort()}const s=new AbortController;s.live=o,N.set(t,s),clearTimeout(k.get(t)),t.querySelectorAll("[data-nr-error],[data-nr-result]").forEach(t=>t.textContent=""),delete t.dataset.nrState,t.setAttribute("aria-busy","true");const i=n&&!n.disabled&&n===document.activeElement,c=n&&!n.disabled&&(n.disabled=!0),l=r.body&&!(r.body instanceof FormData)&&r.body.get("_redirect"),d=r.action?h.start():()=>{};try{if(r.action){const n=await fetch(r.action,{method:"POST",body:r.body,headers:A(),credentials:"same-origin"}),o=await n.json();return(o.ok||o.redirect)&&e.clear(),o.redirect?y(o.redirect):o.ok?O(t,o.data,l):R(t,o)}const a=[t,n].every(t=>!t||"false"!==t.dataset.nrScroll),i=await y(r.url.href,{post:r.body&&{method:"POST",body:r.body,signal:s.signal},signal:s.signal,fresh:!0,focus:!r.body,replace:o||t.hasAttribute("data-nr-replace"),scroll:a,quiet:o,target:t.dataset.nrTarget});i&&await async function(t,e,n){const r=e.headers.get("content-disposition")||"";if(/attachment/i.test(r)){const t=document.createElement("a");return t.href=URL.createObjectURL(await e.blob()),t.download=(r.match(/filename="?([^";]+)/i)||[])[1]||"",t.click(),void setTimeout(()=>URL.revokeObjectURL(t.href),6e4)}const o=await e.text();let a=o;try{a=JSON.parse(o)}catch{}if(e.ok)return O(t,o?a:null,n);const s=a&&"object"==typeof a?a:{error:o};s.error=s.error||s.message||e.statusText||"Request failed",s.status=e.status,R(t,s)}(t,i,l)}catch{s.signal.aborted||L.call(t)}finally{N.get(t)===s&&(N.delete(t),t.removeAttribute("aria-busy")),c&&(n.disabled=!1,i&&n.isConnected&&document.activeElement===document.body&&n.focus({preventScroll:!0})),d()}}window.addEventListener("submit",t=>{const e=t.target;if(!(e instanceof HTMLFormElement)||t.defaultPrevented)return;const n=j(e,t.submitter);if(n)t.preventDefault(),_&&n.body||W(e,t.submitter,n,_);else if(!_){const n=q(e,t.submitter,"target");n&&"_self"!==n||"dialog"===q(e,t.submitter,"method").toLowerCase()||setTimeout(()=>t.defaultPrevented||h.start(3e4))}}),HTMLFormElement.prototype.submit=function(){const t=j(this);t?W(this,null,t):L.call(this)},document.addEventListener("input",t=>{const e=t.target,n=e&&e.form;n&&!t.isComposing&&(n.hasAttribute("data-nr-live")||e.hasAttribute("data-nr-live"))&&(clearTimeout(k.get(n)),k.set(n,setTimeout(()=>{if(n.isConnected&&(n.noValidate||n.checkValidity())){_=!0;try{n.dispatchEvent(new SubmitEvent("submit",{bubbles:!0,cancelable:!0}))}finally{_=!1}}},+(e.dataset.nrDebounce||n.dataset.nrDebounce||e.dataset.nrLive||n.dataset.nrLive||250))))});try{t.env=JSON.parse((document.getElementById("__next_rust_env")||{}).textContent||"{}")}catch{t.env={}}const D=new Set([...document.querySelectorAll("script[data-nr-js]")].map(t=>t.dataset.nrJs)),P=t=>{const e=(t=>{try{const e=new URL(t,location.origin);if(/^\/[^/\\]/.test(t)&&e.origin===location.origin)return e.href}catch{}return console.error(`[next-rust] island module refused: ${t} is not a same-origin path starting with "/"`),null})(t);if(!e)return null;const n=window.__nrm,r=t.replace(/^\/_next-rust\//,"").split("?")[0];return n&&Object.hasOwn(n.f,r)?n.load(r):import(e)},U=new WeakSet;async function M(t){for(const e of t.querySelectorAll("nr-island")){if(U.has(e))continue;U.add(e);let t={};try{t=JSON.parse(e.dataset.props||"{}")}catch{}if(e.dataset.module)try{const n=await P(e.dataset.module);if(n&&"function"==typeof n.hydrate){await n.hydrate(e,t);continue}}catch(t){console.error(`[next-rust] island ${e.dataset.component||e.dataset.module} failed to load`,t)}B(e,t)}}function B(e,n){const r=t=>t.split(".").reduce((t,e)=>null==t?t:t[e],n),o=(t,e)=>{const r=t.split("."),o=r.pop();r.reduce((t,e)=>t[e]=t[e]&&"object"==typeof t[e]?t[e]:{},n)[o]=e},a=t=>t.startsWith("!")?!r(t.slice(1).trim()):!!r(t.trim()),s=t=>{try{return JSON.parse(t)}catch{return t}},i=()=>{for(const t of e.querySelectorAll("[data-nr-text]")){const e=r(t.dataset.nrText);t.textContent=null==e?"":"object"==typeof e?JSON.stringify(e):String(e)}for(const t of e.querySelectorAll("[data-nr-show]"))t.hidden=!a(t.dataset.nrShow);for(const t of e.querySelectorAll("[data-nr-bind]")){const e=r(t.dataset.nrBind);"checkbox"===t.type?t.checked=!!e:document.activeElement!==t&&(t.value=null==e?"":e)}for(const t of e.querySelectorAll("*"))for(const e of t.attributes)e.name.startsWith("data-nr-class-")&&t.classList.toggle(e.name.slice(14),a(e.value))},c={increment:(t,e)=>o(t,Number(r(t)||0)+Number(e||1)),decrement:(t,e)=>o(t,Number(r(t)||0)-Number(e||1)),toggle:t=>o(t,!r(t))},l=new Set;for(const t of e.querySelectorAll("*"))for(const e of t.attributes)e.name.startsWith("data-nr-on-")&&l.add(e.name.slice(11));for(const r of l)e.addEventListener(r,async a=>{const l=a.target.closest&&a.target.closest(`[data-nr-on-${r}]`);if(l&&e.contains(l)){for(const e of l.getAttribute(`data-nr-on-${r}`).split(";")){const r=e.indexOf(":"),i=(r<0?e:e.slice(0,r)).trim(),l=r<0?"":e.slice(r+1).trim();if("prevent"===i)a.preventDefault();else if("set"===i){const t=l.indexOf("=");o(l.slice(0,t).trim(),s(l.slice(t+1).trim()))}else if("action"===i){const[e,r]=l.split("->").map(t=>t.trim());try{const a=await t.action(e,n);r?o(r,a):a&&"object"==typeof a&&Object.assign(n,a)}catch(t){o("error",t.message)}}else if("navigate"===i)y(l);else if(c[i]){const[t,e]=l.split(",");c[i](t.trim(),e)}}i()}});e.addEventListener("input",t=>{const e=t.target;e.dataset&&e.dataset.nrBind&&(o(e.dataset.nrBind,"checkbox"===e.type?e.checked:"number"===e.type?Number(e.value):e.value),i())}),i()}d(),M(document);"####;

// ---- The WebAssembly runtime ------------------------------------------------
//
// `[client] runtime = "wasm"` serves the same behaviour written in Rust
// (`crates/next-rust-runtime`) and compiled to WebAssembly, behind a shim that
// is a table indexed by opcode and nothing else. Both runtimes are kept: an
// application switches with one config key, and falls back the same way.
//
// The three files below are generated by `crates/next-rust-runtime/regenerate.sh`
// and checked in, exactly as [`RUNTIME_JS_MIN`] is, because building them needs
// the `wasm32-unknown-unknown` target and `wasm-opt`, which an application
// building against this crate must not be asked for.

/// The readable shim, served in development.
///
/// Release builds never reach it (`shim_js` returns the minified copy under
/// `cfg(not(debug_assertions))`), exactly as with [`RUNTIME_JS`] above.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub const RUNTIME_SHIM_JS: &str = include_str!("wasm_runtime/shim.js");

/// The minified shim, served in production.
pub const RUNTIME_SHIM_JS_MIN: &str = include_str!("wasm_runtime/shim.min.js");

/// The runtime itself: `crates/next-rust-runtime`, built for
/// `wasm32-unknown-unknown`, `-Oz`'d and stripped.
pub const RUNTIME_WASM: &[u8] = include_bytes!("wasm_runtime/runtime.wasm");

/// The shim as served: readable in development, minified otherwise.
pub(crate) fn shim_js(dev: bool) -> &'static str {
    #[cfg(debug_assertions)]
    if dev {
        return RUNTIME_SHIM_JS;
    }
    let _ = dev;
    RUNTIME_SHIM_JS_MIN
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Content hash of [`RUNTIME_JS`] that [`RUNTIME_JS_MIN`] was generated from.
    const RUNTIME_JS_SOURCE_HASH: &str = "41bda0ddd8f089e9";

    #[test]
    fn minified_runtime_matches_source() {
        assert_eq!(
            next_rust_assets::content_hash(RUNTIME_JS.as_bytes()),
            RUNTIME_JS_SOURCE_HASH,
            "RUNTIME_JS changed: regenerate RUNTIME_JS_MIN and RUNTIME_JS_SOURCE_HASH (see CONTRIBUTING.md)"
        );
    }

    #[test]
    fn minified_runtime_keeps_public_contract() {
        for needle in [
            "window.nextRust",
            "nr-island",
            "data-nr-reload",
            "pointerover",
            "x-nr-nav",
            "x-nr-from",
            "x-nr-action",
            "startViewTransition",
            "--nr-page-fade",
            "data-nr-fade",
            "data-nr-loading",
            "--nr-load",
            "--nr-loader",
            "--nr-load-delay",
            "--nr-load-speed",
            "loader",
            "pageshow",
            "hydrate",
            "nr-t",
            "nr-b",
            "data-nr-stay",
            "data-nr-result",
            "data-nr-reset",
            "nrState",
            "request",
            "stream",
            "text/event-stream",
            "__next_rust_forms",
            "data-nr-native",
            "data-nr-live",
            "nrDebounce",
            "nrTarget",
            "data-nr-key",
            "x-nr-reload",
            "aria-busy",
        ] {
            assert!(RUNTIME_JS_MIN.contains(needle), "minified runtime lost `{needle}`");
        }
        assert!(RUNTIME_JS_MIN.len() < RUNTIME_JS.len() * 3 / 4);
        assert!(!RUNTIME_JS_MIN.contains('\n'), "the minified runtime is a single line");
    }

    /// An action that refuses for a reason the page can act on says so in
    /// `body.code`, with whatever goes with it in `body.data`. Both have to
    /// survive onto the thrown `Error`, or every caller is left matching on a
    /// sentence of English: a learner over quota gets the raw server string
    /// instead of the upgrade button, because the branch that would have
    /// shown it is keyed off `err.code`.
    ///
    /// `request` and `stream` have always carried `data`; `action` is the one
    /// that did not, and this test is why it now does.
    #[test]
    fn a_failed_action_carries_its_code_and_data() {
        for (name, runtime) in [("RUNTIME_JS", RUNTIME_JS), ("RUNTIME_JS_MIN", RUNTIME_JS_MIN)] {
            for needle in [".code=", ".data="] {
                let squashed = runtime.replace(" = ", "=");
                assert!(squashed.contains(needle), "{name} drops `{needle}` from a failed action");
            }
        }
        // And the readable copy reads them off the body, not from anywhere else.
        assert!(RUNTIME_JS.contains("err.code = body.code"), "RUNTIME_JS does not take `code` from the body");
        assert!(RUNTIME_JS.contains("err.data = body.data"), "RUNTIME_JS does not take `data` from the body");
    }

    /// `data-module` can be built from application data, so the loader imports
    /// it only after resolving it against this origin, and looks it up in the
    /// module registry with an own-property check (`constructor`, `__proto__`).
    #[test]
    fn the_island_loader_guards_the_module_url() {
        for (name, runtime) in [("RUNTIME_JS", RUNTIME_JS), ("RUNTIME_JS_MIN", RUNTIME_JS_MIN)] {
            for needle in [
                // Resolve and compare origins rather than trust the attribute.
                "location.origin",
                // A single leading slash, no backslash: no "//host", no scheme.
                r"/^\/[^/\\]/",
                "island module refused",
                "Object.hasOwn",
            ] {
                assert!(runtime.contains(needle), "{name} lost the island module guard `{needle}`");
            }
        }
        // The readable copy must not import the raw attribute value anywhere.
        assert!(!RUNTIME_JS.contains("import(src)"), "the island loader imports an unchecked `data-module`");
    }

    /// Content hash of [`RUNTIME_SHIM_JS`] that [`RUNTIME_SHIM_JS_MIN`] was
    /// generated from, the way [`RUNTIME_JS_SOURCE_HASH`] works for the
    /// JavaScript runtime. `crates/next-rust-runtime/regenerate.sh` rebuilds
    /// both copies.
    const RUNTIME_SHIM_SOURCE_HASH: &str = "7bf1082c60be0e15";

    #[test]
    fn minified_shim_matches_source() {
        assert_eq!(
            next_rust_assets::content_hash(RUNTIME_SHIM_JS.as_bytes()),
            RUNTIME_SHIM_SOURCE_HASH,
            "shim.js changed: run crates/next-rust-runtime/regenerate.sh and update RUNTIME_SHIM_SOURCE_HASH"
        );
        assert!(RUNTIME_SHIM_JS_MIN.len() < RUNTIME_SHIM_JS.len() * 3 / 4);
        assert!(!RUNTIME_SHIM_JS_MIN.contains('\n'), "the minified shim is a single line");
    }

    /// The point of the WebAssembly runtime is that the JavaScript left in the
    /// page decides nothing. The shim is allowed to call browser APIs and to
    /// publish `window.nextRust`; it is not allowed to know any of the
    /// framework's own vocabulary, because knowing a name means implementing a
    /// rule about it. If one of these ever appears here, the rule it belongs to
    /// has leaked out of `next-rust-runtime` and back into the browser.
    #[test]
    fn the_shim_carries_no_framework_behaviour() {
        for (name, shim) in [("shim.js", RUNTIME_SHIM_JS), ("shim.min.js", RUNTIME_SHIM_JS_MIN)] {
            for needle in [
                "data-nr",
                "x-nr-",
                "nr-island",
                "nr-l:",
                "nr-t",
                "nr-b",
                "__next_rust_env",
                "__next_rust_forms",
                "next_rust_csrf",
                "text/event-stream",
                "multipart/form-data",
                "content-disposition",
            ] {
                assert!(!shim.contains(needle), "{name} knows about `{needle}`; that rule belongs in Rust");
            }
            // No `eval` and no `new Function`: the shim runs under a strict CSP.
            assert!(!shim.contains("eval("), "{name} uses eval");
            assert!(!shim.contains("new Function"), "{name} uses new Function");
            // It must still do the two things only JavaScript can.
            assert!(shim.contains("WebAssembly.instantiate"), "{name} does not instantiate the module");
            assert!(shim.contains("window.nextRust"), "{name} does not publish the public API");
        }
    }

    /// The shim calls five exports by name. A module rebuilt without one of
    /// them loads and then does nothing, which is a hard failure to read from
    /// the browser, so it is checked here instead.
    #[test]
    fn the_wasm_runtime_exports_what_the_shim_calls() {
        assert!(RUNTIME_WASM.starts_with(b"\0asm\x01\0\0\0"), "runtime.wasm is not a WebAssembly module");
        for export in ["start", "alloc", "dealloc", "on_event", "resume"] {
            assert!(
                RUNTIME_WASM.windows(export.len()).any(|w| w == export.as_bytes()),
                "runtime.wasm does not export `{export}`: rerun crates/next-rust-runtime/regenerate.sh"
            );
        }
    }
}
