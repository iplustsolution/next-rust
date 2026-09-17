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
//!
//! Two forms are embedded:
//!
//! * [`RUNTIME_JS`]: the readable source, served in development and the one
//!   to edit.
//! * [`RUNTIME_JS_MIN`]: the production build, minified with local names
//!   mangled. It is generated from the source with Terser (see
//!   CONTRIBUTING.md). A test fails if the source changes without the
//!   minified copy being regenerated.

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

/// Minified, name-mangled runtime served in production.
pub const RUNTIME_JS_MIN: &str = r####"const t=window.nextRust=window.nextRust||{},e=new Map;let r=location.pathname+location.search;const n=t=>{try{return new URL(t,location.href).origin===location.origin}catch{return!1}};async function a(t){const e=await fetch(t,{headers:{"x-nr-nav":"1","x-nr-from":location.pathname,accept:"text/html"},credentials:"same-origin"});if(!(e.headers.get("content-type")||"").includes("text/html"))throw new Error("not html");return{url:e.url,html:await e.text()}}function o(t){const r=e.get(t);if(r&&Date.now()-r.at<3e4)return r.promise;const n=a(t);return e.set(t,{at:Date.now(),promise:n}),n.catch(()=>e.delete(t)),n}async function s(t,n={}){const o=new URL(t,location.href);if(o.origin!==location.origin)return void(location.href=o.href);const s=o.pathname+o.search;if(!1!==n.history&&s===r&&o.hash)return void(location.hash=o.hash);const c=document.documentElement;c.setAttribute("data-nr-navigating","");try{const t=n.fresh?null:e.get(o.href),s=await(t?t.promise:a(o.href));e.delete(o.href),function(t){const e=(new DOMParser).parseFromString(t,"text/html");!function(t){for(const e of t.querySelectorAll('template[id^="nr-t"]')){const r=t.getElementById("nr-b"+e.id.slice(4));r&&r.replaceWith(e.content),e.remove()}}(e),document.title=e.title;const r='meta[name]:not([name="viewport"]),meta[property],link[rel="canonical"],style[data-nr-css]',n=new Set([...document.head.querySelectorAll("style[data-nr-css]")].map(t=>t.dataset.nrCss));document.head.querySelectorAll(r.replace(",style[data-nr-css]","")).forEach(t=>t.remove());for(const t of e.head.querySelectorAll(r))t.dataset&&t.dataset.nrCss&&n.has(t.dataset.nrCss)||document.head.appendChild(document.importNode(t,!0));document.body.replaceWith(document.importNode(e.body,!0));for(const t of document.body.querySelectorAll("script")){const e=t.getAttribute("src")||"";if("application/json"===t.type||e.startsWith("/_nr/runtime.js")||/^\$nr\(/.test(t.textContent))continue;const r=document.createElement("script");for(const e of t.attributes)r.setAttribute(e.name,e.value);r.textContent=t.textContent,t.replaceWith(r)}l(document)}(s.html);const c=new URL(s.url||o.href);if(c.hash||(c.hash=o.hash),!1!==n.history&&history[n.replace?"replaceState":"pushState"]({nr:1},"",c.href),r=c.pathname+c.search,!1!==n.scroll){const t=o.hash&&document.getElementById(decodeURIComponent(o.hash.slice(1)));t?t.scrollIntoView():window.scrollTo(0,0)}window.dispatchEvent(new CustomEvent("nr:navigate",{detail:{url:c.href}}))}catch{location.href=o.href}finally{c.removeAttribute("data-nr-navigating")}}t.navigate=t=>s(t),t.replace=t=>s(t,{replace:!0}),t.back=()=>history.back(),t.forward=()=>history.forward(),t.refresh=()=>s(location.href,{replace:!0,scroll:!1,fresh:!0}),t.prefetch=o,document.addEventListener("click",t=>{if(t.defaultPrevented||0!==t.button||t.metaKey||t.ctrlKey||t.shiftKey||t.altKey)return;const e=t.target.closest&&t.target.closest("a[data-nr-link]");!e||e.target&&"_self"!==e.target||e.hasAttribute("download")||!n(e.href)||(t.preventDefault(),s(e.href,{replace:e.hasAttribute("data-nr-replace"),scroll:"false"!==e.dataset.nrScroll}))});const c=t=>{const e=t.target.closest&&t.target.closest("a[data-nr-link]");e&&"false"!==e.dataset.nrPrefetch&&n(e.href)&&!new URL(e.href).hash&&o(e.href)};document.addEventListener("mouseover",c,{passive:!0}),document.addEventListener("touchstart",c,{passive:!0}),window.addEventListener("popstate",()=>{location.pathname+location.search!==r&&s(location.href,{history:!1,scroll:!1})}),t.action=async(t,e)=>{const r=await fetch(t,{method:"POST",headers:{"content-type":"application/json","x-nr-action":"1",accept:"application/json"},body:JSON.stringify(void 0===e?null:e),credentials:"same-origin"}),n=await r.json().catch(()=>({ok:!1,error:r.statusText}));if(!n.redirect){if(!n.ok){const t=new Error(n.error||"Validation failed");throw t.errors=n.errors,t.status=r.status,t}return n.data}s(n.redirect)},document.addEventListener("submit",async e=>{const r=e.target;if(!(r instanceof HTMLFormElement)||!r.dataset.nrAction||e.defaultPrevented)return;if("multipart/form-data"===r.enctype)return;e.preventDefault();const n=new URLSearchParams(new FormData(r,e.submitter));r.querySelectorAll("[data-nr-error]").forEach(t=>t.textContent=""),r.setAttribute("aria-busy","true");try{const e=await fetch(r.dataset.nrAction,{method:"POST",body:n,headers:{"x-nr-action":"1",accept:"application/json"},credentials:"same-origin"}),a=await e.json();if(a.redirect)return s(a.redirect);if(a.ok){r.dispatchEvent(new CustomEvent("nr:success",{detail:a.data,bubbles:!0}));const e=n.get("_redirect");return e?s(e):t.refresh()}const o=a.errors||{_form:a.error};for(const[t,e]of Object.entries(o)){const n=r.querySelector(`[data-nr-error="${CSS.escape(t)}"]`);n&&(n.textContent=e)}r.dispatchEvent(new CustomEvent("nr:error",{detail:a,bubbles:!0}))}catch{HTMLFormElement.prototype.submit.call(r)}finally{r.removeAttribute("aria-busy")}});try{t.env=JSON.parse((document.getElementById("__nr_env")||{}).textContent||"{}")}catch{t.env={}}const i=new WeakSet;async function l(t){for(const e of t.querySelectorAll("nr-island")){if(i.has(e))continue;i.add(e);let t={};try{t=JSON.parse(e.dataset.props||"{}")}catch{}if(e.dataset.module)try{const r=await(import(e.dataset.module));if("function"==typeof r.hydrate){await r.hydrate(e,t);continue}}catch(t){console.error(`[next-rust] island ${e.dataset.component} failed to load`,t)}d(e,t)}}function d(e,r){const n=t=>t.split(".").reduce((t,e)=>null==t?t:t[e],r),a=(t,e)=>{const n=t.split("."),a=n.pop();n.reduce((t,e)=>t[e]=t[e]&&"object"==typeof t[e]?t[e]:{},r)[a]=e},o=t=>t.startsWith("!")?!n(t.slice(1).trim()):!!n(t.trim()),c=t=>{try{return JSON.parse(t)}catch{return t}},i=()=>{for(const t of e.querySelectorAll("[data-nr-text]")){const e=n(t.dataset.nrText);t.textContent=null==e?"":"object"==typeof e?JSON.stringify(e):String(e)}for(const t of e.querySelectorAll("[data-nr-show]"))t.hidden=!o(t.dataset.nrShow);for(const t of e.querySelectorAll("[data-nr-bind]")){const e=n(t.dataset.nrBind);"checkbox"===t.type?t.checked=!!e:document.activeElement!==t&&(t.value=null==e?"":e)}for(const t of e.querySelectorAll("*"))for(const e of t.attributes)e.name.startsWith("data-nr-class-")&&t.classList.toggle(e.name.slice(14),o(e.value))},l={increment:(t,e)=>a(t,Number(n(t)||0)+Number(e||1)),decrement:(t,e)=>a(t,Number(n(t)||0)-Number(e||1)),toggle:t=>a(t,!n(t))},d=new Set;for(const t of e.querySelectorAll("*"))for(const e of t.attributes)e.name.startsWith("data-nr-on-")&&d.add(e.name.slice(11));for(const n of d)e.addEventListener(n,async o=>{const d=o.target.closest&&o.target.closest(`[data-nr-on-${n}]`);if(d&&e.contains(d)){for(const e of d.getAttribute(`data-nr-on-${n}`).split(";")){const n=e.indexOf(":"),i=(n<0?e:e.slice(0,n)).trim(),d=n<0?"":e.slice(n+1).trim();if("prevent"===i)o.preventDefault();else if("set"===i){const t=d.indexOf("=");a(d.slice(0,t).trim(),c(d.slice(t+1).trim()))}else if("action"===i){const[e,n]=d.split("->").map(t=>t.trim());try{const o=await t.action(e,r);n?a(n,o):o&&"object"==typeof o&&Object.assign(r,o)}catch(t){a("error",t.message)}}else if("navigate"===i)s(d);else if(l[i]){const[t,e]=d.split(",");l[i](t.trim(),e)}}i()}});e.addEventListener("input",t=>{const e=t.target;e.dataset&&e.dataset.nrBind&&(a(e.dataset.nrBind,"checkbox"===e.type?e.checked:"number"===e.type?Number(e.value):e.value),i())}),i()}l(document);"####;

#[cfg(test)]
mod tests {
    use super::*;

    /// Content hash of [`RUNTIME_JS`] that [`RUNTIME_JS_MIN`] was generated from.
    const RUNTIME_JS_SOURCE_HASH: &str = "fd84e1d8fb4daf48";

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
            "data-nr-link",
            "x-nr-nav",
            "x-nr-from",
            "x-nr-action",
            "hydrate",
            "nr-t",
            "nr-b",
        ] {
            assert!(RUNTIME_JS_MIN.contains(needle), "minified runtime lost `{needle}`");
        }
        assert!(RUNTIME_JS_MIN.len() < RUNTIME_JS.len() * 3 / 4);
        assert!(!RUNTIME_JS_MIN.contains('\n'), "the minified runtime is a single line");
    }
}
