// Next Rust wasm shim. No dependencies, no eval (CSP friendly).
// Served at /_next-rust/runtime.js when [client] runtime = "wasm".
//
// This file decides nothing. It is a table indexed by the opcode the module
// passes, and every entry is one browser API call. Every rule the runtime has
// — which links are internal, when a prefetch fires, how the loading bar
// moves, what a form does, whether an island module is allowed to load — is
// in the WebAssembly module, in Rust. If a framework rule ever appears here,
// it is in the wrong file.
//
// The opcode order is the ABI: crates/next-rust-runtime/src/host.rs owns it.
// Append, never renumber.
const U = "/_next-rust/runtime.wasm";
const DEC = new TextDecoder();
const ENC = new TextEncoder();
// The handle table. 0 is "nothing"; every other index is an object the module
// holds, with a reference count beside it. Freed slots are reused.
const H = [0];
const RC = [0];
const FREE = [];
let X; // the module's exports
let MEM; // a Uint8Array over its memory, refreshed when the memory grows
let P = null; // the pending result string, as bytes
let SCRATCH = new DataView(new ArrayBuffer(8));

const mem = () => (MEM && MEM.buffer === X.memory.buffer ? MEM : (MEM = new Uint8Array(X.memory.buffer)));
// A (ptr, len) pair of the module's memory as a string.
const s = (p, l) => DEC.decode(mem().subarray(p, p + l));
// Keep an object and hand back its handle.
//
// `0` counts as nothing as well as `null`. Eighteen entries below are written
// `R(H[h] && H[h].something)`, and `H[h]` is the number `0` both for handle 0
// and for a slot that has been released — so without this the number `0`
// would be stored as a live handle, throw on the next call that used it, and
// leave the slot on the free list for an unrelated object to be handed out in.
//
// This is safe only because nothing here ever hands a meaningful `0` across:
// every value that reaches `R` is a DOM object, a response, a promise slot or
// a string, and the one number that crosses (`loader.start(0, …)`) means
// "no limit", which is what the JavaScript runtime's own `if (limit)` reads
// it as. **A generic value layer cannot do this** — `get("clientX")` on a
// click at the left edge, or `get("length")` on an empty collection, must
// survive as a handle holding `0`. The island shim in `next-rust-wasm`
// therefore guards at its call sites (`a ? R(expr) : 0`) and leaves `R`
// accepting falsy values. If these two shims are ever merged, the call-site
// guard is the one to keep; carrying this line into a generic layer would
// silently turn every legitimate zero into "nothing".
const R = (v) => {
  if (v === null || v === undefined || v === false || v === 0) return 0;
  const i = FREE.length ? FREE.pop() : H.length;
  H[i] = v;
  RC[i] = 1;
  return i;
};
// Put a string in the pending slot and report its byte length; null is 2^32-1.
const ret = (v) => {
  if (v === null || v === undefined) {
    P = null;
    return 0xffffffff;
  }
  P = ENC.encode(String(v));
  return P.length;
};
const num = (a, b) => {
  SCRATCH.setUint32(0, a, true);
  SCRATCH.setUint32(4, b, true);
  return SCRATCH.getFloat64(0, true);
};
const nr = () => (window.nextRust = window.nextRust || {});
const back = (t, h, f, v) => X.resume(t, h, f, ret(v));
// sessionStorage throws in a private window; the module treats a miss as absent.
const ss = (f) => {
  try {
    return f(sessionStorage);
  } catch {
    return null;
  }
};

const O = [];
// -- results and lifetime -------------------------------------------------
O[1] = (p, l) => {
  if (P) mem().set(P.subarray(0, l), p);
  P = null;
  return 0;
};
O[2] = (h) => {
  if (h && RC[h] && --RC[h] <= 0) {
    H[h] = 0;
    RC[h] = 0;
    FREE.push(h);
  }
  return 0;
};
O[3] = (p, l) => (console.error(s(p, l)), 0);
O[4] = (h) => (h && RC[h]++, h);
// Two handles for the same object: `R` takes a fresh slot every crossing, so
// the module cannot tell by the numbers alone. Everywhere the JavaScript
// runtime writes `===` on a node, this is what answers it.
O[5] = (a, b) => (a && b && H[a] === H[b] ? 1 : 0);
// -- documents and traversal ----------------------------------------------
O[10] = () => R(document);
O[11] = () => R(document.body);
O[12] = () => R(document.head);
O[13] = () => R(document.documentElement);
O[14] = () => R(document.activeElement);
O[15] = (h) => (H[h] ? H[h].nodeType : 0);
O[16] = (h) => ret(H[h] && H[h].nodeName);
O[17] = (h) => (H[h] && H[h].isConnected ? 1 : 0);
O[18] = (h) => R(H[h] && H[h].parentNode);
O[19] = (h) => R(H[h] && H[h].firstChild);
O[20] = (h) => R(H[h] && H[h].nextSibling);
O[21] = (h) => R(H[h] && H[h].previousSibling);
O[22] = (h) => R(H[h] && [...H[h].childNodes]);
O[23] = (h) => (H[h] ? H[h].length : 0);
O[24] = (h, i) => R(H[h] && H[h][i]);
O[25] = (h, p, l) => R(H[h] && H[h].querySelector(s(p, l)));
O[26] = (h, p, l) => R(H[h] && [...H[h].querySelectorAll(s(p, l))]);
O[27] = (h, p, l) => R(H[h] && H[h].getElementById(s(p, l)));
O[28] = (p, l) => R([...document.getElementsByName(s(p, l))]);
O[29] = (h, p, l) => R(H[h] && H[h].closest && H[h].closest(s(p, l)));
O[30] = (h, o) => (H[h] && o && H[h].contains(H[o]) ? 1 : 0);
O[31] = (h, show) => R(H[h] && document.createTreeWalker(H[h], show));
O[32] = (h) => R(H[h] && H[h].nextNode());
O[33] = (h) => ret(H[h] && H[h].data);
O[34] = (h, p, l) => (H[h] && (H[h].data = s(p, l)), 0);
O[35] = (p, l) => R(document.createElement(s(p, l)));
O[36] = (_d, h, deep) => R(H[h] && document.importNode(H[h], !!deep));
O[37] = (h, c) => (H[h] && H[c] && H[h].appendChild(H[c]), 0);
O[38] = (h, c, b) => (H[h] && H[c] && H[h].insertBefore(H[c], b ? H[b] : null), 0);
O[39] = (h, n) => (H[h] && H[n] && H[h].replaceWith(H[n]), 0);
O[40] = (h) => (H[h] && H[h].remove(), 0);
O[41] = (h) => R(H[h] && H[h].content);
O[42] = (p, l) => R(new DOMParser().parseFromString(s(p, l), "text/html"));
O[43] = (h) => ret(H[h] && H[h].title);
O[44] = (p, l) => ((document.title = s(p, l)), 0);
// -- attributes and properties ----------------------------------------------
O[50] = (h, p, l) => ret(H[h] && H[h].getAttribute ? H[h].getAttribute(s(p, l)) : null);
O[51] = (h, p, l, q, m) => (H[h] && H[h].setAttribute && H[h].setAttribute(s(p, l), s(q, m)), 0);
O[52] = (h, p, l) => (H[h] && H[h].removeAttribute && H[h].removeAttribute(s(p, l)), 0);
O[53] = (h, p, l) => (H[h] && H[h].hasAttribute && H[h].hasAttribute(s(p, l)) ? 1 : 0);
O[54] = (h) => R(H[h] && H[h].attributes ? [...H[h].attributes].map((a) => a.name) : []);
O[55] = (h, i) => ret(H[h] && H[h][i]);
O[56] = (h, p, l, on) => (H[h] && H[h].classList && H[h].classList.toggle(s(p, l), !!on), 0);
O[57] = (h, p, l) => {
  const v = H[h] && H[h][s(p, l)];
  return ret(v === undefined || v === null ? null : v);
};
O[58] = (h, p, l, q, m) => (H[h] && (H[h][s(p, l)] = s(q, m)), 0);
O[59] = (h, p, l) => (H[h] && H[h][s(p, l)] ? 1 : 0);
O[60] = (h, p, l, on) => (H[h] && (H[h][s(p, l)] = !!on), 0);
O[62] = (h, p, l, a, b) => (H[h] && (H[h][s(p, l)] = num(a, b)), 0);
O[63] = (h, p, l) => (H[h] && s(p, l) in H[h] ? 1 : 0);
O[64] = (h, p, l) => (H[h] && typeof H[h][s(p, l)] === "string" ? 1 : 0);
O[65] = (h, p, l) => R(H[h] && H[h][s(p, l)]);
// -- events -------------------------------------------------------------------
// For listeners and for dispatch, handle 0 is `window`. There is no handle
// for it — nothing returns one — and the runtime needs it: `popstate`, the
// `submit` listener that has to run after every other one, and the
// `nr:navigate` event all live there.
const target = (h) => (h ? H[h] : window);
O[70] = (h, p, l, cb, flags) => {
  const t = target(h);
  if (!t) return 0;
  const type = s(p, l);
  const fn = (e) => {
    const eh = R(e);
    X.on_event(cb, eh);
    O[2](eh);
  };
  const opts = { passive: !!(flags & 1), capture: !!(flags & 2) };
  t.addEventListener(type, fn, opts);
  return R({ t, type, fn, opts });
};
O[71] = (h) => {
  const r = H[h];
  if (r) r.t.removeEventListener(r.type, r.fn, r.opts);
  return O[2](h);
};
O[72] = (h) => (H[h] && H[h].preventDefault(), 0);
// args: "<name>\0<detail JSON>"; kind 0 Event, 1 CustomEvent, 2 SubmitEvent.
O[73] = (h, kind, flags, p, l) => {
  const t = target(h);
  if (!t) return 0;
  const [name, detail] = s(p, l).split("\0");
  const init = { bubbles: !!(flags & 1), cancelable: !!(flags & 2) };
  let e;
  if (kind === 1) e = new CustomEvent(name, { ...init, detail: detail ? JSON.parse(detail) : undefined });
  else if (kind === 2) e = new SubmitEvent(name, init);
  else e = new Event(name, init);
  return t.dispatchEvent(e) ? 1 : 0;
};
// -- forms ----------------------------------------------------------------------
// The handle is the real FormData, because it is also what `fetch` sends as
// the body of a multipart form; its entries are cached beside it so the
// accessors below can index them without rebuilding the list each time.
const ENTRIES = new WeakMap();
const entries = (h) => {
  const d = H[h];
  if (!d) return null;
  let e = ENTRIES.get(d);
  if (!e) ENTRIES.set(d, (e = [...d]));
  return e;
};
O[80] = (h, sub) => {
  if (!H[h]) return 0;
  let d;
  try {
    d = new FormData(H[h], sub ? H[sub] : undefined);
  } catch {
    d = new FormData(H[h]);
  }
  return R(d);
};
O[81] = (h) => {
  const e = entries(h);
  return e ? e.length : 0;
};
O[82] = (h, i) => {
  const e = entries(h);
  return ret(e && e[i] && e[i][0]);
};
O[84] = (h, i) => {
  const e = entries(h);
  const v = e && e[i] && e[i][1];
  return ret(typeof v === "string" ? v : v && v.name);
};
O[85] = () => R(new URLSearchParams());
O[86] = (h, p, l, q, m) => (H[h] && H[h].append(s(p, l), s(q, m)), 0);
O[87] = (h, p, l) => ret(H[h] && H[h].get(s(p, l)));
O[88] = (h) => ret(H[h] && String(H[h]));
O[89] = (h) => (H[h] && H[h].reset(), 0);
O[90] = (h) => (H[h] && H[h].checkValidity() ? 1 : 0);
O[91] = (p, l) => ret(CSS.escape(s(p, l)));
O[92] = (h, ps) => (H[h] && H[h].focus && H[h].focus({ preventScroll: !!ps }), 0);
O[93] = (h, a, b, p, l) => {
  try {
    H[h].setSelectionRange(a, b, s(p, l));
  } catch {}
  return 0;
};
O[94] = (h) => (H[h] && H[h].scrollIntoView(), 0);
O[95] = (x, y) => (window.scrollTo(x, y), 0);
// The one named hook: `form.submit()` fires no submit event, so the module has
// to be able to see it and to call the original.
let NATIVE = HTMLFormElement.prototype.submit;
O[96] = (cb) => {
  HTMLFormElement.prototype.submit = function () {
    const h = R(this);
    X.on_event(cb, h);
    O[2](h);
  };
  return 0;
};
O[97] = (h) => (H[h] && NATIVE.call(H[h]), 0);
// -- styles --------------------------------------------------------------------------
O[100] = (h, p, l, q, m) => {
  if (!H[h]) return ret("");
  const pseudo = s(p, l);
  return ret(getComputedStyle(H[h], pseudo || undefined).getPropertyValue(s(q, m)));
};
O[101] = (h, p, l, q, m) => (H[h] && H[h].style.setProperty(s(p, l), s(q, m)), 0);
O[102] = (h, p, l) => (H[h] && H[h].style.removeProperty(s(p, l)), 0);
// -- fetch and streams ------------------------------------------------------------------
// args: u32 signal handle, u32 body length, then NUL-terminated url, method and
// header name/value pairs ending with an empty field, then the body bytes.
O[110] = (p, l, kind, body, token) => {
  const view = new DataView(mem().buffer, p, 8);
  const signal = view.getUint32(0, true);
  const blen = view.getUint32(4, true);
  const fields = s(p + 8, l - 8 - blen).split("\0");
  const init = { headers: {}, credentials: "same-origin" };
  const url = fields[0];
  init.method = fields[1] || "GET";
  for (let i = 2; i + 1 < fields.length && fields[i]; i += 2) init.headers[fields[i]] = fields[i + 1];
  if (kind === 1) init.body = mem().slice(p + l - blen, p + l);
  else if (kind) init.body = H[body];
  if (signal) init.signal = H[signal];
  fetch(url, init).then(
    (r) => X.resume(token, R(r), 0, 0xffffffff),
    (e) => back(token, 0, 1, (e && e.name) || "error"),
  );
  return 0;
};
O[111] = (h) => (H[h] ? H[h].status : 0);
O[112] = (h) => (H[h] && H[h].ok ? 1 : 0);
O[113] = (h) => ret(H[h] && H[h].url);
O[114] = (h) => ret(H[h] && H[h].statusText);
O[115] = (h, p, l) => ret(H[h] && H[h].headers.get(s(p, l)));
O[116] = (h, token) => (
  H[h].text().then(
    (t) => back(token, 0, 0, t),
    () => back(token, 0, 1, ""),
  ),
  0
);
O[117] = (h, token) => (
  H[h].blob().then(
    (b) => X.resume(token, R(b), 0, 0xffffffff),
    () => back(token, 0, 1, ""),
  ),
  0
);
O[118] = (h) => R(H[h] && H[h].body && H[h].body.getReader());
O[119] = (h, token) => (
  H[h].read().then(
    ({ value, done }) => back(token, 0, done ? 2 : 0, value ? DEC.decode(value, { stream: true }) : ""),
    () => back(token, 0, 1, ""),
  ),
  0
);
O[120] = (h) => (H[h] && H[h].cancel().catch(() => {}), 0);
O[121] = () => R(new AbortController());
O[122] = (h) => (H[h] && H[h].abort(), 0);
O[123] = (h) => R(H[h] && H[h].signal);
O[124] = (h) => (H[h] && H[h].aborted ? 1 : 0);
O[125] = (h) => ret(URL.createObjectURL(H[h]));
O[126] = (p, l) => (URL.revokeObjectURL(s(p, l)), 0);
// -- timers and time ----------------------------------------------------------------------
O[130] = (ms, cb) => setTimeout(() => X.on_event(cb, 0), ms);
O[131] = (id) => (clearTimeout(id), 0);
// -- location, history, storage, URL ----------------------------------------------------------
const PART = ["href", "origin", "protocol", "pathname", "search", "hash"];
O[140] = (w) => ret(location[PART[w]]);
O[141] = (p, l) => ((location.href = s(p, l)), 0);
O[142] = (p, l) => ((location.hash = s(p, l)), 0);
O[143] = (p, l) => (history.pushState({ nr: 1 }, "", s(p, l)), 0);
O[144] = (p, l) => (history.replaceState({ nr: 1 }, "", s(p, l)), 0);
O[145] = () => (history.back(), 0);
O[146] = () => (history.forward(), 0);
O[147] = (p, l) => ret(ss((x) => x.getItem(s(p, l))));
O[148] = (p, l, q, m) => (ss((x) => x.setItem(s(p, l), s(q, m))), 0);
O[149] = (p, l) => (ss((x) => x.removeItem(s(p, l))), 0);
O[150] = () => ret(document.cookie);
O[151] = (p, l, q, m) => {
  try {
    return R(new URL(s(p, l), m ? s(q, m) : undefined));
  } catch {
    return 0;
  }
};
O[152] = (h, w) => ret(H[h] && H[h][PART[w]]);
O[153] = (h, p, l) => (H[h] && (H[h].search = s(p, l)), 0);
O[154] = (h, p, l) => (H[h] && (H[h].hash = s(p, l)), 0);
// -- view transitions ---------------------------------------------------------------------------
O[160] = () => (document.startViewTransition ? 1 : 0);
O[161] = (cb, token) => {
  const t = document.startViewTransition(() => X.on_event(cb, 0));
  t.ready.catch(() => {});
  t.finished.catch(() => {});
  t.updateCallbackDone.then(
    () => back(token, 0, 0, null),
    () => back(token, 0, 1, null),
  );
  return 0;
};
// -- island modules -------------------------------------------------------------------------------
// The module decided this URL is allowed: it is a same-origin absolute path.
O[170] = (p, l, token) => (
  import(s(p, l)).then(
    (m) => X.resume(token, R(m), 0, 0xffffffff),
    (e) => back(token, 0, 1, String((e && e.message) || e)),
  ),
  0
);
O[171] = (p, l) => {
  const r = window.__nrm;
  return r && Object.hasOwn(r.f, s(p, l)) ? 1 : 0;
};
O[172] = (p, l, token) =>
  (Promise.resolve(window.__nrm.load(s(p, l))).then(
    (m) => X.resume(token, R(m), 0, 0xffffffff),
    (e) => back(token, 0, 1, String((e && e.message) || e)),
  ),
  0);
// 2 means "this module has no hydrate export": the module falls back to the
// declarative bindings, exactly as the JavaScript runtime does.
O[173] = (m, el, p, l, token) => {
  const mod = H[m];
  if (!mod || typeof mod.hydrate !== "function") return back(token, 0, 2, null), 0;
  Promise.resolve(mod.hydrate(H[el], l ? JSON.parse(s(p, l)) : {})).then(
    () => back(token, 0, 0, null),
    (e) => back(token, 0, 1, String((e && e.message) || e)),
  );
  return 0;
};
// -- the public API on window.nextRust -------------------------------------------------------------
// Assigned onto whatever is already there: `toast` and `consent` come from the
// analytics layer, not from here.
O[180] = (p, l) => {
  const N = nr();
  // `api_call` takes four words. Arguments the caller passed are wrapped as
  // handles and released again; a token or a sink is ALREADY a handle and
  // must be passed through untouched, or the module settles a handle that
  // holds a number instead of the promise behind it.
  const send = (which, args, extra) => {
    const a = args.map(R);
    const all = extra === undefined ? a : [...a, extra];
    const out = X.api_call(which, all[0] | 0, all[1] | 0, all[2] | 0, all[3] | 0);
    a.forEach(O[2]);
    return out;
  };
  const call = (which, ...args) => {
    const out = send(which, args);
    if (!out) return undefined;
    // The module handed the value over; the table must not keep holding it.
    const value = H[out];
    O[2](out);
    return value;
  };
  const promise = (which, ...args) =>
    new Promise((res, rej) => {
      send(which, args, R({ res, rej }));
    });
  N.navigate = (href) => promise(1, String(href));
  N.replace = (href) => promise(2, String(href));
  N.back = () => call(3);
  N.forward = () => call(4);
  N.refresh = () => promise(5);
  N.prefetch = (href) => call(6, String(href));
  N.action = (url, input) => promise(7, String(url), JSON.stringify(input === undefined ? null : input));
  N.request = (url, options) => promise(8, String(url), options || {});
  N.stream = (url, options) => {
    const q = [];
    let wake = null;
    let ended = null;
    const sink = R({
      push: (v) => (q.push(v), wake && wake()),
      end: (e) => ((ended = e || true), wake && wake()),
    });
    send(9, [String(url), options || {}], sink);
    return (async function* () {
      for (;;) {
        while (q.length) yield q.shift();
        if (ended) {
          if (ended !== true) throw ended;
          return;
        }
        await new Promise((r) => (wake = r));
        wake = null;
      }
    })();
  };
  N.loader = { start: (limit, from) => call(10, limit, from), reset: () => call(11) };
  // The module read the environment out of the document and hands over the
  // text: which element holds it is the framework's business, not the shim's.
  try {
    N.env = JSON.parse(l ? s(p, l) : "{}");
  } catch {
    N.env = {};
  }
  return 0;
};
O[181] = (sink, p, l) => {
  const [event, data, id] = s(p, l).split("\0");
  H[sink].push({ event, data, id: id || undefined });
  return 0;
};
// An Error the module encoded as JSON: its `message`, plus every other member
// it wrote beside it. Which members those are is the module's business; this
// only puts them back on the Error.
const E = (t) => {
  const { message, ...rest } = JSON.parse(t);
  return Object.assign(new Error(message), rest);
};
O[182] = (sink, p, l) => (H[sink].end(l ? E(s(p, l)) : null), 0);
// Settle a promise the shim handed out. The module cannot build a JavaScript
// value, so the kinds it can settle with are: 0 a handle, 1 a string, 2 an
// Error, 3 a JSON value, 4 an Error whose other members are JSON (the
// `.errors` and `.status` a failed action rejects with).
O[183] = (t, kind, a, b) => {
  const slot = H[t];
  if (!slot) return 0;
  if (kind === 2) slot.rej(new Error(s(a, b)));
  else if (kind === 4) slot.rej(E(s(a, b)));
  else if (kind === 3) slot.res(JSON.parse(s(a, b)));
  else slot.res(kind === 1 ? s(a, b) : a ? H[a] : undefined);
  O[2](t);
  return 0;
};
// A value a caller passed in, as JSON. `nextRust.request(url, options)` takes
// a plain object, and the table has no way to walk one.
O[184] = (h) => ret(H[h] === undefined ? null : JSON.stringify(H[h]));
// A JavaScript function that re-enters the module. `nextRust.loader.start()`
// answers with the function that ends that load, and nothing else in the
// table makes one.
O[185] = (cb) =>
  R(() => {
    X.on_event(cb, 0);
  });

// Numbers come back through their own import so the integer table stays u32.
const F = [];
F[61] = (h, p, l) => {
  const v = H[h] && H[h][s(p, l)];
  return typeof v === "number" ? v : NaN;
};
F[132] = () => Date.now();
F[133] = () => performance.now();

const IMPORTS = {
  e: {
    h: (o, a, b, c, d, f) => (O[o] ? O[o](a, b, c, d, f) | 0 : 0),
    f: (o, a, b, c, d, f) => (F[o] ? F[o](a, b, c, d, f) : 0),
  },
};

(WebAssembly.instantiateStreaming
  ? WebAssembly.instantiateStreaming(fetch(U), IMPORTS)
  : fetch(U)
      .then((r) => r.arrayBuffer())
      .then((b) => WebAssembly.instantiate(b, IMPORTS))
)
  .then((r) => {
    X = r.instance.exports;
    X.start();
  })
  .catch((e) => console.error("[next-rust] the runtime module failed to load", e));
