//! The browser runtime served at `/_next-rust/runtime.js`.
//!
//! Browsers can only execute JavaScript or WebAssembly, so the part of the
//! framework that runs in the browser is JavaScript source kept here as a
//! string. It has no dependencies and never uses `eval`, so it works under a
//! strict Content Security Policy. It is sent only to pages that render a
//! `Link!` with client navigation or an interactive island.
//!
//! Responsibilities: client-side navigation and prefetching, applying streamed
//! boundaries after navigation, server-action calls and form enhancement, and
//! island hydration (declarative bindings or `hydrate(element, props)` modules).
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

async function fetchPage(href, full) {
  const headers = { "x-nr-nav": "1", "x-nr-from": location.pathname, accept: "text/html" };
  const keys = full ? [] : layoutMarkers().map((n) => n.data.slice(5));
  if (keys.length) headers["x-nr-layouts"] = keys.join(",");
  // Stylesheets (and utility rules) the page has are not sent again.
  const styles = [...document.querySelectorAll("style[data-nr-css]")].map((s) => s.dataset.nrCss);
  if (styles.length) headers["x-nr-styles"] = styles.join(",");
  const res = await fetch(href, { headers, credentials: "same-origin" });
  if (!(res.headers.get("content-type") || "").includes("text/html")) throw new Error("not html");
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
function loadPage(href, fresh, full) {
  const now = Date.now();
  for (const [key, entry] of pages) if (now - entry.at >= PAGE_TTL) pages.delete(key);
  const hit = fresh ? null : pages.get(href);
  if (hit) return hit.promise;
  const promise = fetchPage(href, full);
  const entry = { at: now, promise };
  pages.set(href, entry);
  promise.catch(() => {
    if (pages.get(href) === entry) pages.delete(href);
  });
  return promise;
}

const prefetch = (href) => loadPage(href, false);

function render(page) {
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
  let scripts;
  const region = page.partial && findRegion(page.partial);
  if (region) {
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

async function navigate(href, opts = {}) {
  const url = new URL(href, location.href);
  if (url.origin !== location.origin) {
    location.href = url.href;
    return;
  }
  const target = url.pathname + url.search;
  if (opts.history !== false && target === current && url.hash) {
    location.hash = url.hash;
    return;
  }
  const root = document.documentElement;
  root.setAttribute("data-nr-navigating", "");
  try {
    let page;
    try {
      page = await loadPage(url.href, opts.fresh);
    } catch {
      page = await loadPage(url.href, true); // a failed prefetch gets one more try
    }
    if (page.from && page.from !== location.pathname) page = await loadPage(url.href, true);
    // A partial page needs its layout on screen; if it is gone, get the full page.
    if (page.partial && !findRegion(page.partial)) page = await loadPage(url.href, true, true);
    render(page);
    const finalUrl = new URL(page.url || url.href);
    if (!finalUrl.hash) finalUrl.hash = url.hash;
    if (opts.history !== false) history[opts.replace ? "replaceState" : "pushState"]({ nr: 1 }, "", finalUrl.href);
    current = finalUrl.pathname + finalUrl.search;
    markActiveLinks();
    // A link inside a menu that stays on screen (an open <details>) closes it.
    for (let d = opts.link && opts.link.isConnected && opts.link.closest("details[open]"); d; d = d.parentElement && d.parentElement.closest("details[open]")) {
      d.open = false;
    }
    if (opts.scroll !== false) {
      const anchor = url.hash && document.getElementById(decodeURIComponent(url.hash.slice(1)));
      anchor ? anchor.scrollIntoView() : window.scrollTo(0, 0);
    }
    window.dispatchEvent(new CustomEvent("nr:navigate", { detail: { url: finalUrl.href } }));
  } catch {
    location.href = url.href;
  } finally {
    root.removeAttribute("data-nr-navigating");
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
  if (!a) return;
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
const actionHeaders = (extra) => {
  const headers = { "x-nr-action": "1", accept: "application/json", ...extra };
  const csrf = document.cookie.match(/(?:^|;\s*)next_rust_csrf=([^;]+)/);
  if (csrf) headers["x-csrf-token"] = csrf[1];
  return headers;
};

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

document.addEventListener("submit", async (e) => {
  const form = e.target;
  if (!(form instanceof HTMLFormElement) || !form.dataset.nrAction || e.defaultPrevented) return;
  if (form.enctype === "multipart/form-data") return;
  e.preventDefault();
  const data = new URLSearchParams(new FormData(form, e.submitter));
  form.querySelectorAll("[data-nr-error],[data-nr-result]").forEach((el) => (el.textContent = ""));
  delete form.dataset.nrState;
  form.setAttribute("aria-busy", "true");
  try {
    const res = await fetch(form.dataset.nrAction, {
      method: "POST",
      body: data,
      headers: actionHeaders(),
      credentials: "same-origin",
    });
    const body = await res.json();
    if (body.ok || body.redirect) pages.clear();
    if (body.redirect) return navigate(body.redirect);
    if (body.ok) {
      form.dataset.nrState = "success";
      form.dispatchEvent(new CustomEvent("nr:success", { detail: body.data, bubbles: true }));
      // `data-nr-stay`: show the result in place; `_redirect` is then only
      // where browsers without JavaScript go.
      if (form.hasAttribute("data-nr-stay")) {
        for (const el of form.querySelectorAll("[data-nr-result]")) {
          const key = el.dataset.nrResult;
          const value = key && body.data && typeof body.data === "object" ? body.data[key] : body.data;
          el.textContent = value == null || typeof value === "object" ? "" : String(value);
        }
        if (form.hasAttribute("data-nr-reset")) form.reset();
        return;
      }
      const next = data.get("_redirect");
      return next ? navigate(next) : NR.refresh();
    }
    form.dataset.nrState = "error";
    const errors = body.errors || { _form: body.error };
    for (const [field, message] of Object.entries(errors)) {
      const el = form.querySelector(`[data-nr-error="${CSS.escape(field)}"]`);
      if (el) el.textContent = message;
    }
    form.dispatchEvent(new CustomEvent("nr:error", { detail: body, bubbles: true }));
  } catch {
    HTMLFormElement.prototype.submit.call(form);
  } finally {
    form.removeAttribute("aria-busy");
  }
});

// ---- Islands --------------------------------------------------------------

try {
  NR.env = JSON.parse((document.getElementById("__next_rust_env") || {}).textContent || "{}");
} catch {
  NR.env = {};
}

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
        const mod = await import(el.dataset.module);
        if (typeof mod.hydrate === "function") {
          await mod.hydrate(el, props);
          continue;
        }
      } catch (err) {
        console.error(`[next-rust] island ${el.dataset.component} failed to load`, err);
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
pub const RUNTIME_JS_MIN: &str = r####"const t=window.nextRust=window.nextRust||{},e=new Map;let n=location.pathname+location.search;function r(t){const e=document.createTreeWalker(document.body,NodeFilter.SHOW_COMMENT);let n=null;for(let r=e.nextNode();r;r=e.nextNode())if(r.data==="nr-l:"+t)n=r;else if(n&&r.data==="/nr-l:"+t&&r.parentNode===n.parentNode)return{start:n,end:r};return null}function a(t,n,r){const a=Date.now();for(const[t,n]of e)a-n.at>=3e4&&e.delete(t);const o=n?null:e.get(t);if(o)return o.promise;const s=async function(t,e){const n={"x-nr-nav":"1","x-nr-from":location.pathname,accept:"text/html"},r=e?[]:function(){const t=document.createTreeWalker(document.body,NodeFilter.SHOW_COMMENT),e=[];for(let n=t.nextNode();n;n=t.nextNode())n.data.startsWith("nr-l:")&&e.push(n);return e}().map(t=>t.data.slice(5));r.length&&(n["x-nr-layouts"]=r.join(","));const a=[...document.querySelectorAll("style[data-nr-css]")].map(t=>t.dataset.nrCss);a.length&&(n["x-nr-styles"]=a.join(","));const o=await fetch(t,{headers:n,credentials:"same-origin"});if(!(o.headers.get("content-type")||"").includes("text/html"))throw new Error("not html");return{url:o.url,html:await o.text(),from:o.headers.get("x-nr-intercepted")?location.pathname:null,partial:o.headers.get("x-nr-partial")}}(t,r),c={at:a,promise:s};return e.set(t,c),s.catch(()=>{e.get(t)===c&&e.delete(t)}),s}const o=t=>a(t,!1);function s(){const t=location.pathname.length>1?location.pathname.replace(/\/+$/,""):location.pathname;for(const e of document.querySelectorAll("a[data-nr-active],a[data-nr-active-prefix]")){let n;try{n=new URL(e.href,location.href).pathname}catch{continue}n.length>1&&(n=n.replace(/\/+$/,""));const r=t===n,a=r||"/"===n||t.startsWith(n+"/");let o=!1;for(const[t,n]of[["nrActive",r],["nrActivePrefix",a]]){const r=e.dataset[t];void 0!==r&&(r&&e.classList.toggle(r,n),o=o||n)}o?e.setAttribute("aria-current","page"):"page"===e.getAttribute("aria-current")&&e.removeAttribute("aria-current")}}async function c(o,c={}){const i=new URL(o,location.href);if(i.origin!==location.origin)return void(location.href=i.href);const l=i.pathname+i.search;if(!1!==c.history&&l===n&&i.hash)return void(location.hash=i.hash);const d=document.documentElement;d.setAttribute("data-nr-navigating","");try{let o;try{o=await a(i.href,c.fresh)}catch{o=await a(i.href,!0)}o.from&&o.from!==location.pathname&&(o=await a(i.href,!0)),o.partial&&!r(o.partial)&&(o=await a(i.href,!0,!0)),function(n){const a=(new DOMParser).parseFromString(n.html,"text/html");!function(t){for(const e of t.querySelectorAll('template[id^="nr-t"]')){const n=t.getElementById("nr-b"+e.id.slice(4));n&&n.replaceWith(e.content),e.remove()}}(a),document.title=a.title;const o='meta[name]:not([name="viewport"]),meta[property],link[rel="canonical"],style[data-nr-css]';for(const t of document.body.querySelectorAll("style[data-nr-css]"))document.head.appendChild(t);const s=new Map([...document.head.querySelectorAll("style[data-nr-css]")].map(t=>[t.dataset.nrCss,t]));document.head.querySelectorAll(o.replace(",style[data-nr-css]","")).forEach(t=>t.remove());let c=!1;for(const t of[...a.head.querySelectorAll(o),...a.body.querySelectorAll("style[data-nr-css]")]){const e=t.dataset&&t.dataset.nrCss;e&&e.includes("~")&&(c=!0),e&&s.has(e)?e.includes("~")&&document.head.appendChild(s.get(e)):document.head.appendChild(document.importNode(t,!0)),t.parentNode!==a.head&&t.remove()}c&&e.clear();const i=a.getElementById("__next_rust_env");if(i)try{t.env=JSON.parse(i.textContent||"{}")}catch{}let l;const d=n.partial&&r(n.partial);if(d){const{start:t,end:e}=d;for(;t.nextSibling&&t.nextSibling!==e;)t.nextSibling.remove();const n=[];for(const t of[...a.body.childNodes]){if(1===t.nodeType&&("__next_rust_env"===t.id||(t.getAttribute("src")||"").startsWith("/_next-rust/runtime.js")))continue;const r=document.importNode(t,!0);e.parentNode.insertBefore(r,e),n.push(r)}l=n.flatMap(t=>1!==t.nodeType?[]:"SCRIPT"===t.tagName?[t]:[...t.querySelectorAll("script")])}else document.body.replaceWith(document.importNode(a.body,!0)),l=[...document.body.querySelectorAll("script")];for(const t of l){const e=t.getAttribute("src")||"";if("application/json"===t.type||e.startsWith("/_next-rust/runtime.js")||/^\$nr\(/.test(t.textContent))continue;const n=document.createElement("script");for(const e of t.attributes)n.setAttribute(e.name,e.value);n.textContent=t.textContent,t.replaceWith(n)}g(document)}(o);const l=new URL(o.url||i.href);l.hash||(l.hash=i.hash),!1!==c.history&&history[c.replace?"replaceState":"pushState"]({nr:1},"",l.href),n=l.pathname+l.search,s();for(let t=c.link&&c.link.isConnected&&c.link.closest("details[open]");t;t=t.parentElement&&t.parentElement.closest("details[open]"))t.open=!1;if(!1!==c.scroll){const t=i.hash&&document.getElementById(decodeURIComponent(i.hash.slice(1)));t?t.scrollIntoView():window.scrollTo(0,0)}window.dispatchEvent(new CustomEvent("nr:navigate",{detail:{url:l.href}}))}catch{location.href=i.href}finally{d.removeAttribute("data-nr-navigating")}}t.navigate=t=>c(t),t.replace=t=>c(t,{replace:!0}),t.back=()=>history.back(),t.forward=()=>history.forward(),t.refresh=()=>c(location.href,{replace:!0,scroll:!1,fresh:!0}),t.prefetch=o;const i=t=>{const e=t&&t.closest&&t.closest("a[href]");if(!e||"string"!=typeof e.href)return null;if(e.hasAttribute("data-nr-reload")||e.hasAttribute("download"))return null;if(e.target&&"_self"!==e.target)return null;if((e.getAttribute("rel")||"").split(/\s+/).includes("external"))return null;let n;try{n=new URL(e.href,location.href)}catch{return null}return n.origin===location.origin&&/^https?:$/.test(n.protocol)?e:null};document.addEventListener("click",t=>{if(t.defaultPrevented||0!==t.button||t.metaKey||t.ctrlKey||t.shiftKey||t.altKey)return;const e=i(t.target);e&&(t.preventDefault(),c(e.href,{replace:e.hasAttribute("data-nr-replace"),scroll:"false"!==e.dataset.nrScroll,link:e}))});let l=null,d=0;const u=()=>{clearTimeout(d),d=0,l=null},f=t=>{if("mouse"!==t.pointerType)return;const e=i(t.target);if(e===l)return;if(u(),!e||"false"===e.dataset.nrPrefetch)return;const r=new URL(e.href);r.hash||r.pathname+r.search===n||(l=e,d=setTimeout(()=>{d=0,l===e&&e.isConnected&&o(e.href)},400))};document.addEventListener("pointerover",f,{passive:!0}),document.addEventListener("pointermove",f,{passive:!0}),document.addEventListener("pointerout",t=>{l&&!l.contains(t.relatedTarget)&&u()},{passive:!0}),document.addEventListener("pointerdown",u,{passive:!0}),document.addEventListener("scroll",u,{capture:!0,passive:!0}),window.addEventListener("popstate",()=>{location.pathname+location.search!==n&&c(location.href,{history:!1,scroll:!1})});const h=t=>{const e={"x-nr-action":"1",accept:"application/json",...t},n=document.cookie.match(/(?:^|;\s*)next_rust_csrf=([^;]+)/);return n&&(e["x-csrf-token"]=n[1]),e};t.action=async(t,n)=>{const r=await fetch(t,{method:"POST",headers:h({"content-type":"application/json"}),body:JSON.stringify(void 0===n?null:n),credentials:"same-origin"}),a=await r.json().catch(()=>({ok:!1,error:r.statusText}));if((a.ok||a.redirect)&&e.clear(),!a.redirect){if(!a.ok){const t=new Error(a.error||"Validation failed");throw t.errors=a.errors,t.status=r.status,t}return a.data}c(a.redirect)};const m=t=>{const e=new URL(t,location.href);if(e.origin!==location.origin)throw new Error("[next-rust] same-origin URLs only: "+e.origin);return e.href},p=t=>{const{method:e="GET",body:n,headers:r,signal:a}=t,o={method:e.toUpperCase(),headers:h(r||{}),credentials:"same-origin",signal:a};return delete o.headers["x-nr-action"],null!=n&&("string"==typeof n||n instanceof FormData||n instanceof URLSearchParams||n instanceof Blob||n instanceof ArrayBuffer?o.body=n:(o.headers["content-type"]="application/json",o.body=JSON.stringify(n))),o};t.request=async(t,n={})=>{const r=p(n);let a=0;if(n.timeout&&!n.signal){const t=new AbortController;r.signal=t.signal,a=setTimeout(()=>t.abort(),n.timeout)}try{const a=await fetch(m(t),r);"GET"!==r.method&&"HEAD"!==r.method&&e.clear();const o=a.headers.get("content-type")||"",s=a.ok?n.as:void 0;if("response"===s)return a;const c=204===a.status?null:"blob"===s?await a.blob():"text"!==s&&o.includes("json")?await a.json().catch(()=>null):await a.text();if(!a.ok){const t=c&&"object"==typeof c&&(c.error||c.message)||a.statusText||"Request failed",e=new Error(t);throw e.status=a.status,e.data=c,e}return c}finally{clearTimeout(a)}},t.stream=async function*(t,e={}){const n=p(e);n.headers.accept="text/event-stream";const r=await fetch(m(t),n);if(!r.ok||!r.body){const t=new Error(r.statusText||"Stream failed");throw t.status=r.status,t.data=await r.text().catch(()=>""),t}const a=r.body.getReader(),o=new TextDecoder;let s="";try{for(;;){const{value:t,done:e}=await a.read();let n;for(s+=e?"\n\n":o.decode(t,{stream:!0});(n=s.search(/\r?\n\r?\n/))>=0;){const t=s.slice(0,n);s=s.slice(n).replace(/^\r?\n\r?\n/,"");const e={event:"message",data:[],id:void 0};for(const n of t.split(/\r?\n/)){if(!n||n.startsWith(":"))continue;const t=n.indexOf(":"),r=t<0?n:n.slice(0,t),a=t<0?"":n.slice(t+1).replace(/^ /,"");"event"===r?e.event=a:"data"===r?e.data.push(a):"id"===r&&(e.id=a)}e.data.length&&(yield{event:e.event,data:e.data.join("\n"),id:e.id})}if(e)return}}finally{a.cancel().catch(()=>{})}},document.addEventListener("submit",async n=>{const r=n.target;if(!(r instanceof HTMLFormElement)||!r.dataset.nrAction||n.defaultPrevented)return;if("multipart/form-data"===r.enctype)return;n.preventDefault();const a=new URLSearchParams(new FormData(r,n.submitter));r.querySelectorAll("[data-nr-error],[data-nr-result]").forEach(t=>t.textContent=""),delete r.dataset.nrState,r.setAttribute("aria-busy","true");try{const n=await fetch(r.dataset.nrAction,{method:"POST",body:a,headers:h(),credentials:"same-origin"}),o=await n.json();if((o.ok||o.redirect)&&e.clear(),o.redirect)return c(o.redirect);if(o.ok){if(r.dataset.nrState="success",r.dispatchEvent(new CustomEvent("nr:success",{detail:o.data,bubbles:!0})),r.hasAttribute("data-nr-stay")){for(const t of r.querySelectorAll("[data-nr-result]")){const e=t.dataset.nrResult,n=e&&o.data&&"object"==typeof o.data?o.data[e]:o.data;t.textContent=null==n||"object"==typeof n?"":String(n)}return void(r.hasAttribute("data-nr-reset")&&r.reset())}const e=a.get("_redirect");return e?c(e):t.refresh()}r.dataset.nrState="error";const s=o.errors||{_form:o.error};for(const[t,e]of Object.entries(s)){const n=r.querySelector(`[data-nr-error="${CSS.escape(t)}"]`);n&&(n.textContent=e)}r.dispatchEvent(new CustomEvent("nr:error",{detail:o,bubbles:!0}))}catch{HTMLFormElement.prototype.submit.call(r)}finally{r.removeAttribute("aria-busy")}});try{t.env=JSON.parse((document.getElementById("__next_rust_env")||{}).textContent||"{}")}catch{t.env={}}const y=new WeakSet;async function g(t){for(const e of t.querySelectorAll("nr-island")){if(y.has(e))continue;y.add(e);let t={};try{t=JSON.parse(e.dataset.props||"{}")}catch{}if(e.dataset.module)try{const n=await(import(e.dataset.module));if("function"==typeof n.hydrate){await n.hydrate(e,t);continue}}catch(t){console.error(`[next-rust] island ${e.dataset.component} failed to load`,t)}b(e,t)}}function b(e,n){const r=t=>t.split(".").reduce((t,e)=>null==t?t:t[e],n),a=(t,e)=>{const r=t.split("."),a=r.pop();r.reduce((t,e)=>t[e]=t[e]&&"object"==typeof t[e]?t[e]:{},n)[a]=e},o=t=>t.startsWith("!")?!r(t.slice(1).trim()):!!r(t.trim()),s=t=>{try{return JSON.parse(t)}catch{return t}},i=()=>{for(const t of e.querySelectorAll("[data-nr-text]")){const e=r(t.dataset.nrText);t.textContent=null==e?"":"object"==typeof e?JSON.stringify(e):String(e)}for(const t of e.querySelectorAll("[data-nr-show]"))t.hidden=!o(t.dataset.nrShow);for(const t of e.querySelectorAll("[data-nr-bind]")){const e=r(t.dataset.nrBind);"checkbox"===t.type?t.checked=!!e:document.activeElement!==t&&(t.value=null==e?"":e)}for(const t of e.querySelectorAll("*"))for(const e of t.attributes)e.name.startsWith("data-nr-class-")&&t.classList.toggle(e.name.slice(14),o(e.value))},l={increment:(t,e)=>a(t,Number(r(t)||0)+Number(e||1)),decrement:(t,e)=>a(t,Number(r(t)||0)-Number(e||1)),toggle:t=>a(t,!r(t))},d=new Set;for(const t of e.querySelectorAll("*"))for(const e of t.attributes)e.name.startsWith("data-nr-on-")&&d.add(e.name.slice(11));for(const r of d)e.addEventListener(r,async o=>{const d=o.target.closest&&o.target.closest(`[data-nr-on-${r}]`);if(d&&e.contains(d)){for(const e of d.getAttribute(`data-nr-on-${r}`).split(";")){const r=e.indexOf(":"),i=(r<0?e:e.slice(0,r)).trim(),d=r<0?"":e.slice(r+1).trim();if("prevent"===i)o.preventDefault();else if("set"===i){const t=d.indexOf("=");a(d.slice(0,t).trim(),s(d.slice(t+1).trim()))}else if("action"===i){const[e,r]=d.split("->").map(t=>t.trim());try{const o=await t.action(e,n);r?a(r,o):o&&"object"==typeof o&&Object.assign(n,o)}catch(t){a("error",t.message)}}else if("navigate"===i)c(d);else if(l[i]){const[t,e]=d.split(",");l[i](t.trim(),e)}}i()}});e.addEventListener("input",t=>{const e=t.target;e.dataset&&e.dataset.nrBind&&(a(e.dataset.nrBind,"checkbox"===e.type?e.checked:"number"===e.type?Number(e.value):e.value),i())}),i()}s(),g(document);"####;

#[cfg(test)]
mod tests {
    use super::*;

    /// Content hash of [`RUNTIME_JS`] that [`RUNTIME_JS_MIN`] was generated from.
    const RUNTIME_JS_SOURCE_HASH: &str = "46b498f49ffe1728";

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
        ] {
            assert!(RUNTIME_JS_MIN.contains(needle), "minified runtime lost `{needle}`");
        }
        assert!(RUNTIME_JS_MIN.len() < RUNTIME_JS.len() * 3 / 4);
        assert!(!RUNTIME_JS_MIN.contains('\n'), "the minified runtime is a single line");
    }
}
