//! The browser runtime served at `/_nr/runtime.js`.
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

pub const RUNTIME_JS: &str = r#"// Next Rust client runtime. No dependencies, no eval (CSP friendly).
// Served at /_nr/runtime.js and loaded only by pages that render a Link! with
// client navigation or an interactive island.
const NR = (window.nextRust = window.nextRust || {});
const prefetched = new Map();
const PREFETCH_TTL = 30000;
let current = location.pathname + location.search;

const sameOrigin = (href) => {
  try {
    return new URL(href, location.href).origin === location.origin;
  } catch {
    return false;
  }
};

function swapStreamed(doc) {
  for (const t of doc.querySelectorAll('template[id^="nr-t"]')) {
    const b = doc.getElementById("nr-b" + t.id.slice(4));
    if (b) b.replaceWith(t.content);
    t.remove();
  }
}

async function fetchPage(href) {
  const res = await fetch(href, {
    headers: { "x-nr-nav": "1", "x-nr-from": location.pathname, accept: "text/html" },
    credentials: "same-origin",
  });
  if (!(res.headers.get("content-type") || "").includes("text/html")) throw new Error("not html");
  return { url: res.url, html: await res.text() };
}

function prefetch(href) {
  const hit = prefetched.get(href);
  if (hit && Date.now() - hit.at < PREFETCH_TTL) return hit.promise;
  const promise = fetchPage(href);
  prefetched.set(href, { at: Date.now(), promise });
  promise.catch(() => prefetched.delete(href));
  return promise;
}

function render(html) {
  const doc = new DOMParser().parseFromString(html, "text/html");
  swapStreamed(doc);
  document.title = doc.title;
  const managed = 'meta[name]:not([name="viewport"]),meta[property],link[rel="canonical"],style[data-nr-css]';
  const existing = new Set([...document.head.querySelectorAll("style[data-nr-css]")].map((s) => s.dataset.nrCss));
  document.head.querySelectorAll(managed.replace(",style[data-nr-css]", "")).forEach((n) => n.remove());
  for (const n of doc.head.querySelectorAll(managed)) {
    if (n.dataset && n.dataset.nrCss && existing.has(n.dataset.nrCss)) continue;
    document.head.appendChild(document.importNode(n, true));
  }
  document.body.replaceWith(document.importNode(doc.body, true));
  for (const old of document.body.querySelectorAll("script")) {
    const src = old.getAttribute("src") || "";
    if (old.type === "application/json" || src.startsWith("/_nr/runtime.js") || /^\$nr\(/.test(old.textContent)) continue;
    const s = document.createElement("script");
    for (const a of old.attributes) s.setAttribute(a.name, a.value);
    s.textContent = old.textContent;
    old.replaceWith(s);
  }
  hydrateIslands(document);
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
    const cached = opts.fresh ? null : prefetched.get(url.href);
    const page = await (cached ? cached.promise : fetchPage(url.href));
    prefetched.delete(url.href);
    render(page.html);
    const finalUrl = new URL(page.url || url.href);
    if (!finalUrl.hash) finalUrl.hash = url.hash;
    if (opts.history !== false) history[opts.replace ? "replaceState" : "pushState"]({ nr: 1 }, "", finalUrl.href);
    current = finalUrl.pathname + finalUrl.search;
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

document.addEventListener("click", (e) => {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
  const a = e.target.closest && e.target.closest("a[data-nr-link]");
  if (!a || (a.target && a.target !== "_self") || a.hasAttribute("download") || !sameOrigin(a.href)) return;
  e.preventDefault();
  navigate(a.href, { replace: a.hasAttribute("data-nr-replace"), scroll: a.dataset.nrScroll !== "false" });
});

const intent = (e) => {
  const a = e.target.closest && e.target.closest("a[data-nr-link]");
  if (a && a.dataset.nrPrefetch !== "false" && sameOrigin(a.href) && !new URL(a.href).hash) prefetch(a.href);
};
document.addEventListener("mouseover", intent, { passive: true });
document.addEventListener("touchstart", intent, { passive: true });

window.addEventListener("popstate", () => {
  if (location.pathname + location.search !== current) navigate(location.href, { history: false, scroll: false });
});

// ---- Server actions -------------------------------------------------------

NR.action = async (url, input) => {
  const res = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json", "x-nr-action": "1", accept: "application/json" },
    body: JSON.stringify(input === undefined ? null : input),
    credentials: "same-origin",
  });
  const body = await res.json().catch(() => ({ ok: false, error: res.statusText }));
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

document.addEventListener("submit", async (e) => {
  const form = e.target;
  if (!(form instanceof HTMLFormElement) || !form.dataset.nrAction || e.defaultPrevented) return;
  if (form.enctype === "multipart/form-data") return;
  e.preventDefault();
  const data = new URLSearchParams(new FormData(form, e.submitter));
  form.querySelectorAll("[data-nr-error]").forEach((el) => (el.textContent = ""));
  form.setAttribute("aria-busy", "true");
  try {
    const res = await fetch(form.dataset.nrAction, {
      method: "POST",
      body: data,
      headers: { "x-nr-action": "1", accept: "application/json" },
      credentials: "same-origin",
    });
    const body = await res.json();
    if (body.redirect) return navigate(body.redirect);
    if (body.ok) {
      form.dispatchEvent(new CustomEvent("nr:success", { detail: body.data, bubbles: true }));
      const next = data.get("_redirect");
      return next ? navigate(next) : NR.refresh();
    }
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
  NR.env = JSON.parse((document.getElementById("__nr_env") || {}).textContent || "{}");
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

hydrateIslands(document);
"#;
