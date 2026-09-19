//! Browser behavior of the interactive components, served at `/_next-rust/ui.js`.
//!
//! It exists twice: [`UI_JS`], readable (served in development), and
//! [`UI_JS_MIN`], minified with Terser (production). After changing
//! `UI_JS`, regenerate `UI_JS_MIN` (see CONTRIBUTING.md) and update
//! `UI_JS_SOURCE_HASH` in the tests below.

/// Readable source of the component script.
pub const UI_JS: &str = r####"// Next Rust UI: browser behavior of the interactive components.
//
// Loaded as a module on pages that render an element with `data-nr-ui`.
// Components work without it (native controls); this adds the custom
// select, the calendar, the password toggle, `on_press`, the drawer, tab
// panels, modals and closable alerts.

const NR = () => window.nextRust || {};
const html = document.documentElement;
html.classList.add("nr-ui-js");

const CHEVRON_LEFT = '<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 18-6-6 6-6"/></svg>';
const CHEVRON_RIGHT = '<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>';
const CHEVRON_DOWN = '<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>';

// ---- Popovers ---------------------------------------------------------------

// Open popovers: component root → close function.
const open = new Map();

function closeAll(except) {
  for (const [root, close] of open) if (root !== except) close();
}

document.addEventListener(
  "pointerdown",
  (e) => {
    for (const [root, close] of open) if (!root.contains(e.target)) close();
  },
  true,
);

// Popovers are shown in the browser's top layer when it has one, so no
// `overflow: hidden` container (a card, a dialog) can clip them; they are
// then positioned against their anchor and follow it while scrolling.
const topLayer = typeof HTMLElement.prototype.showPopover === "function";
const anchors = new Map();

// Below the anchor, or above it when there is no room below; kept inside
// the viewport horizontally. `matchWidth`: as wide as the anchor.
function position(pop, anchor, matchWidth) {
  const r = anchor.getBoundingClientRect();
  const gap = 6;
  if (!topLayer) {
    pop.removeAttribute("data-placement");
    pop.removeAttribute("data-align");
    const h = pop.offsetHeight;
    if (r.bottom + h + gap * 2 > innerHeight && r.top - h - gap * 2 > 0) pop.dataset.placement = "top";
    if (r.left + pop.offsetWidth + 8 > innerWidth && r.right - pop.offsetWidth > 0) pop.dataset.align = "end";
    return;
  }
  const width = matchWidth ? r.width + 4 : pop.offsetWidth;
  const left = Math.max(8, Math.min(r.left - 2, innerWidth - width - 8));
  const h = pop.offsetHeight;
  const below = r.bottom + gap + h <= innerHeight - 8 || r.top - gap - h < 8;
  pop.dataset.placement = below ? "bottom" : "top";
  Object.assign(pop.style, {
    position: "fixed",
    inset: "auto",
    margin: "0",
    left: `${left}px`,
    top: `${below ? r.bottom + gap : r.top - gap - h}px`,
    width: matchWidth ? `${width}px` : "",
  });
}

function showPop(pop, anchor, matchWidth) {
  pop.hidden = false;
  if (topLayer) {
    pop.popover = "manual";
    pop.showPopover();
    anchors.set(pop, [anchor, matchWidth]);
  }
  position(pop, anchor, matchWidth);
}

function hidePop(pop) {
  if (topLayer && pop.matches(":popover-open")) pop.hidePopover();
  anchors.delete(pop);
  pop.hidden = true;
}

const follow = () => {
  for (const [pop, [anchor, matchWidth]] of anchors) position(pop, anchor, matchWidth);
};
addEventListener("scroll", follow, { capture: true, passive: true });
addEventListener("resize", follow, { passive: true });

// ---- Mounting ---------------------------------------------------------------

const mounted = new WeakSet();

function mount(el) {
  if (mounted.has(el)) return;
  mounted.add(el);
  const kind = el.dataset.nrUi;
  if (kind === "select") setupSelect(el);
  else if (kind === "datepicker") setupDatePicker(el);
  else if (kind === "avatar") setupAvatar(el);
  else if (kind === "tabs") setupTabs(el);
  else if (kind === "modal") setupModal(el);
}

function mountAll(scope) {
  if (scope.matches && scope.matches("[data-nr-ui]")) mount(scope);
  for (const el of scope.querySelectorAll("[data-nr-ui]")) mount(el);
}

const fire = (el, name, detail) => el.dispatchEvent(new CustomEvent(name, { bubbles: true, detail }));
const changed = (el) => {
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
};

// ---- on_press ---------------------------------------------------------------

document.addEventListener("click", async (e) => {
  const el = e.target.closest && e.target.closest("[data-nr-press]");
  if (!el || el.matches(':disabled, [aria-disabled="true"], [data-loading]')) return;
  const kind = el.dataset.nrPress;
  const target = el.dataset.nrPressTarget || "";
  if (kind === "navigate") {
    e.preventDefault();
    if (NR().navigate) NR().navigate(target);
    else location.href = target;
  } else if (kind === "modal") {
    e.preventDefault();
    openModal(document.getElementById(target));
  } else if (kind === "close-modal") {
    e.preventDefault();
    const dialog = el.closest("dialog");
    if (dialog) dialog.close();
  } else if (kind === "emit") {
    fire(el, "nr:" + target, null);
  } else if (kind === "action") {
    e.preventDefault();
    let input = null;
    try {
      input = JSON.parse(el.dataset.nrPressInput || "null");
    } catch {}
    el.setAttribute("data-loading", "");
    el.setAttribute("aria-busy", "true");
    try {
      const data = await NR().action(target, input);
      fire(el, "nr:success", data);
      if (el.dataset.nrPressRefresh !== "false" && NR().refresh) NR().refresh();
    } catch (err) {
      fire(el, "nr:error", err);
      if (!el.hasAttribute("data-nr-quiet")) console.error("[next-rust] action failed:", err.message);
    } finally {
      el.removeAttribute("data-loading");
      el.removeAttribute("aria-busy");
    }
  }
});

// Pressable elements that are not buttons (cards) react to Enter and Space.
document.addEventListener("keydown", (e) => {
  const el = e.target;
  if ((e.key === "Enter" || e.key === " ") && el.matches && el.matches('[data-nr-press][role="button"]')) {
    e.preventDefault();
    el.click();
  }
});

// ---- Text fields --------------------------------------------------------------

document.addEventListener("click", (e) => {
  const btn = e.target.closest && e.target.closest("[data-nr-field-action]");
  if (btn) {
    const input = btn.closest(".nr-field")?.querySelector("input.nr-field-input, textarea.nr-field-input");
    const action = btn.dataset.nrFieldAction;
    if (action === "toggle-password" && input) {
      const show = input.type === "password";
      input.type = show ? "text" : "password";
      btn.setAttribute("aria-pressed", String(show));
      btn.setAttribute("aria-label", show ? "Hide password" : "Show password");
      input.focus({ preventScroll: true });
      const end = input.value.length;
      input.setSelectionRange(end, end);
    } else if (action === "clear" && input) {
      input.value = "";
      changed(input);
      input.focus();
    }
    return;
  }
  // A click anywhere in a text field's box focuses the input.
  const wrap = e.target.closest && e.target.closest(".nr-input .nr-field-wrap, .nr-textarea .nr-field-wrap");
  if (wrap && !e.target.closest("input, textarea, button, a, select")) {
    wrap.querySelector(".nr-field-input")?.focus();
  }
});

// ---- App shell drawer ---------------------------------------------------------

function setShell(openIt) {
  const shell = document.querySelector(".nr-shell");
  if (!shell) return;
  shell.toggleAttribute("data-open", openIt);
  for (const b of document.querySelectorAll(".nr-navbar-menu")) b.setAttribute("aria-expanded", String(openIt));
}

document.addEventListener("click", (e) => {
  if (e.target.closest && e.target.closest("[data-nr-shell-toggle]")) {
    setShell(!document.querySelector(".nr-shell[data-open]"));
  } else if (e.target.closest && e.target.closest(".nr-shell-sidebar a[href]")) {
    setShell(false);
  }
});
window.addEventListener("nr:navigate", () => setShell(false));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && document.querySelector(".nr-shell[data-open]")) setShell(false);
});

// ---- Tabs ---------------------------------------------------------------------

function setupTabs(root) {
  const list = root.querySelector('[role="tablist"]');
  if (!list) return;
  const tabs = () => [...list.querySelectorAll('[role="tab"]')];
  function select(tab, focus) {
    if (!tab || tab.disabled) return;
    for (const t of tabs()) {
      const on = t === tab;
      t.classList.toggle("nr-tab-selected", on);
      t.setAttribute("aria-selected", String(on));
      t.tabIndex = on ? 0 : -1;
      const panel = t.getAttribute("aria-controls") && document.getElementById(t.getAttribute("aria-controls"));
      if (panel) panel.hidden = !on;
    }
    if (focus) tab.focus({ preventScroll: true });
    fire(root, "nr:change", { key: tab.dataset.key });
  }
  list.addEventListener("click", (e) => {
    const tab = e.target.closest('[role="tab"]');
    if (tab && list.contains(tab)) select(tab);
  });
  list.addEventListener("keydown", (e) => {
    const all = tabs().filter((t) => !t.disabled);
    const i = all.indexOf(e.target.closest('[role="tab"]'));
    if (i < 0) return;
    const to = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: all.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    select(all[(to + all.length) % all.length], true);
  });
}

// ---- Modal --------------------------------------------------------------------

function openModal(dialog) {
  if (!dialog || dialog.open) return;
  dialog.showModal();
  fire(dialog, "nr:open", null);
}

function setupModal(dialog) {
  // Shown on load with `open`: reopen as a real modal (inert page, focus kept).
  if (dialog.open) {
    dialog.close();
    openModal(dialog);
  }
  dialog.addEventListener("click", (e) => {
    // A click on the backdrop lands on the dialog itself, not on its box.
    if (e.target === dialog && dialog.hasAttribute("data-dismissable")) dialog.close();
  });
  dialog.addEventListener("close", () => fire(dialog, "nr:close", null));
}

// ---- Closable alerts ------------------------------------------------------------

document.addEventListener("click", (e) => {
  const btn = e.target.closest && e.target.closest("[data-nr-dismiss]");
  const alert = btn && btn.closest('[data-nr-ui="alert"]');
  if (alert) {
    fire(alert, "nr:dismiss", null);
    alert.remove();
  }
});

// ---- Avatar -------------------------------------------------------------------

function setupAvatar(el) {
  const img = el.querySelector(".nr-avatar-img");
  if (!img) return;
  const fail = () => (img.hidden = true);
  if (img.complete && img.naturalWidth === 0 && img.getAttribute("src")) fail();
  else img.addEventListener("error", fail, { once: true });
}

// ---- Select -------------------------------------------------------------------

function setupSelect(root) {
  const native = root.querySelector(".nr-select-native");
  const trigger = root.querySelector(".nr-select-trigger");
  const list = root.querySelector(".nr-select-list");
  const valueEl = root.querySelector(".nr-select-value");
  const wrap = root.querySelector(".nr-field-wrap");
  if (!native || !trigger || !list || !valueEl) return;
  const multiple = native.multiple;
  const blank = [...native.options].find((o) => o.value === "");
  const placeholder = blank ? blank.textContent : "";
  const options = () => [...list.querySelectorAll('[role="option"]')];
  const usable = () => options().filter((o) => o.getAttribute("aria-disabled") !== "true");
  let active = null;
  let typed = "";
  let typedAt = 0;

  function sync() {
    const values = new Set([...native.selectedOptions].map((o) => o.value).filter((v) => v !== ""));
    const labels = [];
    for (const o of options()) {
      const on = values.has(o.dataset.value);
      o.setAttribute("aria-selected", String(on));
      if (on) labels.push(o.querySelector(".nr-select-option-label").textContent);
    }
    valueEl.textContent = labels.length ? labels.join(", ") : placeholder;
    valueEl.toggleAttribute("data-placeholder", !labels.length);
    root.toggleAttribute("data-filled", labels.length > 0);
  }

  function setActive(o, scroll = true) {
    if (active) active.removeAttribute("data-active");
    active = o || null;
    if (active) {
      active.setAttribute("data-active", "");
      trigger.setAttribute("aria-activedescendant", active.id);
      if (scroll) active.scrollIntoView({ block: "nearest" });
    } else {
      trigger.removeAttribute("aria-activedescendant");
    }
  }

  function show() {
    if (!list.hidden || native.disabled) return;
    closeAll(root);
    showPop(list, wrap, true);
    root.setAttribute("data-open", "");
    trigger.setAttribute("aria-expanded", "true");
    setActive(usable().find((o) => o.getAttribute("aria-selected") === "true") || usable()[0]);
    open.set(root, hide);
  }

  function hide(focus) {
    if (list.hidden) return;
    hidePop(list);
    root.removeAttribute("data-open");
    trigger.setAttribute("aria-expanded", "false");
    setActive(null);
    open.delete(root);
    if (focus === true) trigger.focus();
  }

  function choose(o) {
    if (!o || o.getAttribute("aria-disabled") === "true") return;
    const option = [...native.options].find((x) => x.value === o.dataset.value);
    if (!option) return;
    option.selected = multiple ? !option.selected : true;
    changed(native);
    sync();
    if (!multiple) hide(true);
  }

  trigger.addEventListener("click", () => (list.hidden ? show() : hide()));
  wrap.addEventListener("click", (e) => {
    if (e.target.closest(".nr-select-list, .nr-select-trigger")) return;
    trigger.focus();
    if (list.hidden) show();
    else hide();
  });
  list.addEventListener("mousedown", (e) => e.preventDefault());
  list.addEventListener("pointermove", (e) => {
    const o = e.target.closest('[role="option"]');
    if (o && o !== active && o.getAttribute("aria-disabled") !== "true") setActive(o, false);
  });
  list.addEventListener("click", (e) => {
    e.stopPropagation();
    choose(e.target.closest('[role="option"]'));
  });
  // Space activates a button on key up: that would reopen the list.
  trigger.addEventListener("keyup", (e) => {
    if (e.key === " ") e.preventDefault();
  });
  trigger.addEventListener("keydown", (e) => {
    const items = usable();
    const i = items.indexOf(active);
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (list.hidden) show();
        else setActive(items[Math.min(i + 1, items.length - 1)] || items[0]);
        break;
      case "ArrowUp":
        e.preventDefault();
        if (list.hidden) show();
        else setActive(items[Math.max(i - 1, 0)]);
        break;
      case "Home":
      case "End":
        if (!list.hidden) {
          e.preventDefault();
          setActive(e.key === "Home" ? items[0] : items[items.length - 1]);
        }
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        if (list.hidden) show();
        else choose(active);
        break;
      case "Escape":
        if (!list.hidden) {
          e.preventDefault();
          hide(true);
        }
        break;
      case "Tab":
        hide();
        break;
      default:
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
          const now = Date.now();
          typed = now - typedAt > 700 ? e.key : typed + e.key;
          typedAt = now;
          const query = typed.toLowerCase();
          const match = items.find((o) => o.textContent.trim().toLowerCase().startsWith(query));
          if (match) {
            if (list.hidden) show();
            setActive(match);
          }
        }
    }
  });
  root.querySelector(".nr-field-label")?.addEventListener("click", (e) => {
    e.preventDefault();
    trigger.focus();
  });
  native.addEventListener("change", sync);
  if (native.form) native.form.addEventListener("reset", () => setTimeout(sync));
  sync();
}

// ---- Date picker --------------------------------------------------------------

const pad = (n, w = 2) => String(n).padStart(w, "0");
const iso = (d) => `${pad(d.getFullYear(), 4)}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const sameDay = (a, b) => !!a && !!b && iso(a) === iso(b);
const addDays = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
const monthStart = (d) => new Date(d.getFullYear(), d.getMonth(), 1);

function parseDate(s) {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s || "");
  if (!m) return null;
  const d = new Date(+m[1], +m[2] - 1, +m[3]);
  return d.getMonth() === +m[2] - 1 ? d : null;
}

// Same month and year as `d`, on the closest valid day.
function addMonths(d, n) {
  const target = new Date(d.getFullYear(), d.getMonth() + n, 1);
  const last = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate();
  return new Date(target.getFullYear(), target.getMonth(), Math.min(d.getDate(), last));
}

function setupDatePicker(root) {
  const input = root.querySelector(".nr-datepicker-input");
  const cal = root.querySelector(".nr-calendar");
  const wrap = root.querySelector(".nr-field-wrap");
  if (!input || !cal || !wrap) return;
  const locale = root.dataset.locale || html.lang || undefined;
  const firstDay = +(root.dataset.firstDay || 0);
  const min = parseDate(input.min);
  const max = parseDate(input.max);
  const today = () => {
    const n = new Date();
    return new Date(n.getFullYear(), n.getMonth(), n.getDate());
  };
  const allowed = (d) => (!min || d >= min) && (!max || d <= max);
  const dateFmt = new Intl.DateTimeFormat(locale, { dateStyle: "medium" });
  const longFmt = new Intl.DateTimeFormat(locale, { dateStyle: "full" });
  const titleFmt = new Intl.DateTimeFormat(locale, { month: "long", year: "numeric" });
  const monthFmt = new Intl.DateTimeFormat(locale, { month: "short" });
  const weekdayFmt = new Intl.DateTimeFormat(locale, { weekday: "short" });

  // The visible input shows the date in the visitor's locale; a hidden
  // input submits it as YYYY-MM-DD under the field's name.
  const hidden = document.createElement("input");
  hidden.type = "hidden";
  hidden.name = input.name;
  input.removeAttribute("name");
  input.after(hidden);
  let value = parseDate(input.value);
  input.type = "text";
  input.autocomplete = "off";
  input.setAttribute("inputmode", "none");
  input.setAttribute("aria-haspopup", "dialog");
  input.addEventListener("beforeinput", (e) => e.preventDefault());

  const write = () => {
    input.value = value ? dateFmt.format(value) : "";
    hidden.value = value ? iso(value) : "";
  };
  write();

  let view = "days";
  let focus = value || today();

  function button(cls, attrs, content) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = cls;
    for (const [k, v] of Object.entries(attrs)) if (v !== false && v != null) b.setAttribute(k, v === true ? "" : v);
    b.innerHTML = content;
    return b;
  }

  function render(moveFocus) {
    cal.textContent = "";
    cal.dataset.view = view;
    const header = document.createElement("div");
    header.className = "nr-cal-header";
    const inDays = view === "days";
    const prevTarget = inDays ? addMonths(monthStart(focus), -1) : new Date(focus.getFullYear() - 1, 11, 31);
    const nextTarget = inDays ? addMonths(monthStart(focus), 1) : new Date(focus.getFullYear() + 1, 0, 1);
    const prevLast = inDays ? new Date(prevTarget.getFullYear(), prevTarget.getMonth() + 1, 0) : prevTarget;
    header.append(
      button("nr-cal-nav", { "data-cal": "prev", "aria-label": inDays ? "Previous month" : "Previous year", disabled: !!min && prevLast < min }, CHEVRON_LEFT),
      button("nr-cal-title", { "data-cal": "view", "aria-live": "polite" }, `<span>${inDays ? titleFmt.format(focus) : focus.getFullYear()}</span>${CHEVRON_DOWN}`),
      button("nr-cal-nav", { "data-cal": "next", "aria-label": inDays ? "Next month" : "Next year", disabled: !!max && nextTarget > max }, CHEVRON_RIGHT),
    );
    cal.append(header);

    if (inDays) {
      const grid = document.createElement("div");
      grid.className = "nr-cal-grid";
      grid.setAttribute("role", "grid");
      for (let i = 0; i < 7; i++) {
        const w = document.createElement("div");
        w.className = "nr-cal-weekday";
        w.setAttribute("role", "columnheader");
        w.textContent = weekdayFmt.format(new Date(2023, 0, 1 + ((i + firstDay) % 7)));
        grid.append(w);
      }
      const start = monthStart(focus);
      const lead = (start.getDay() - firstDay + 7) % 7;
      const now = today();
      for (let i = 0; i < 42; i++) {
        const d = addDays(start, i - lead);
        grid.append(
          button(
            "nr-cal-day",
            {
              role: "gridcell",
              "data-date": iso(d),
              "aria-label": longFmt.format(d),
              "aria-selected": sameDay(d, value) ? "true" : "false",
              "data-today": sameDay(d, now),
              "data-outside": d.getMonth() !== focus.getMonth(),
              tabindex: sameDay(d, focus) ? "0" : "-1",
              disabled: !allowed(d),
            },
            String(d.getDate()),
          ),
        );
      }
      cal.append(grid);
    } else {
      const months = document.createElement("div");
      months.className = "nr-cal-months";
      for (let m = 0; m < 12; m++) {
        const first = new Date(focus.getFullYear(), m, 1);
        const last = new Date(focus.getFullYear(), m + 1, 0);
        months.append(
          button(
            "nr-cal-month",
            {
              "data-month": String(m),
              "aria-selected": value && value.getFullYear() === focus.getFullYear() && value.getMonth() === m ? "true" : "false",
              "data-current": m === focus.getMonth(),
              tabindex: m === focus.getMonth() ? "0" : "-1",
              disabled: (!!min && last < min) || (!!max && first > max),
            },
            monthFmt.format(first),
          ),
        );
      }
      cal.append(months);
    }

    const footer = document.createElement("div");
    footer.className = "nr-cal-footer";
    footer.append(
      button("nr-cal-link", { "data-cal": "today", disabled: !allowed(today()) }, "Today"),
      button("nr-cal-link", { "data-cal": "clear" }, "Clear"),
    );
    cal.append(footer);
    // Month and day views differ in height.
    if (!cal.hidden) position(cal, wrap, false);
    if (moveFocus) cal.querySelector('[tabindex="0"]:not(:disabled)')?.focus({ preventScroll: true });
  }

  function show() {
    if (!cal.hidden || input.disabled || input.readOnly) return;
    closeAll(root);
    view = "days";
    focus = value || (allowed(today()) ? today() : min || max || today());
    render(false);
    showPop(cal, wrap, false);
    root.setAttribute("data-open", "");
    root.querySelector(".nr-datepicker-toggle")?.setAttribute("aria-expanded", "true");
    cal.querySelector('[tabindex="0"]:not(:disabled)')?.focus({ preventScroll: true });
    open.set(root, hide);
  }

  function hide(returnFocus) {
    if (cal.hidden) return;
    hidePop(cal);
    root.removeAttribute("data-open");
    root.querySelector(".nr-datepicker-toggle")?.setAttribute("aria-expanded", "false");
    open.delete(root);
    if (returnFocus === true) input.focus({ preventScroll: true });
  }

  function select(d) {
    value = d;
    write();
    changed(hidden);
    hide(true);
  }

  function move(d) {
    if (min && d < min) d = min;
    if (max && d > max) d = max;
    focus = d;
    render(true);
  }

  cal.addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b || b.disabled) return;
    if (b.dataset.date) select(parseDate(b.dataset.date));
    else if (b.dataset.month) {
      focus = new Date(focus.getFullYear(), +b.dataset.month, Math.min(focus.getDate(), 28));
      view = "days";
      render(true);
    } else if (b.dataset.cal === "prev") {
      focus = view === "days" ? addMonths(focus, -1) : new Date(focus.getFullYear() - 1, focus.getMonth(), 1);
      render(false);
    } else if (b.dataset.cal === "next") {
      focus = view === "days" ? addMonths(focus, 1) : new Date(focus.getFullYear() + 1, focus.getMonth(), 1);
      render(false);
    } else if (b.dataset.cal === "view") {
      view = view === "days" ? "months" : "days";
      render(true);
    } else if (b.dataset.cal === "today") select(today());
    else if (b.dataset.cal === "clear") {
      value = null;
      write();
      changed(hidden);
      hide(true);
    }
  });

  cal.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      hide(true);
      return;
    }
    const onDay = e.target.dataset && e.target.dataset.date;
    const onMonth = e.target.dataset && e.target.dataset.month != null;
    if (!onDay && !onMonth) return;
    // Days: a week per row. Months: three per row.
    const step = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: onDay ? -7 : -3, ArrowDown: onDay ? 7 : 3 }[e.key];
    if (step != null) {
      e.preventDefault();
      if (onDay) move(addDays(focus, step));
      else {
        const m = Math.min(11, Math.max(0, focus.getMonth() + step));
        focus = new Date(focus.getFullYear(), m, 1);
        render(true);
      }
    } else if (onDay && (e.key === "PageUp" || e.key === "PageDown")) {
      e.preventDefault();
      move(addMonths(focus, (e.key === "PageUp" ? -1 : 1) * (e.shiftKey ? 12 : 1)));
    } else if (onDay && (e.key === "Home" || e.key === "End")) {
      e.preventDefault();
      const offset = (focus.getDay() - firstDay + 7) % 7;
      move(addDays(focus, e.key === "Home" ? -offset : 6 - offset));
    }
  });

  // Leaving the picker with Tab closes it.
  root.addEventListener("focusout", (e) => {
    if (!cal.hidden && e.relatedTarget && !root.contains(e.relatedTarget)) hide();
  });

  input.addEventListener("click", show);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown") {
      e.preventDefault();
      show();
    } else if ((e.key === "Backspace" || e.key === "Delete") && value && !input.required) {
      e.preventDefault();
      value = null;
      write();
      changed(hidden);
    }
  });
  wrap.addEventListener("click", (e) => {
    if (e.target.closest(".nr-calendar")) return;
    if (e.target.closest(".nr-datepicker-toggle") && !cal.hidden) hide(true);
    else show();
  });
  if (input.form) {
    input.form.addEventListener("reset", () =>
      setTimeout(() => {
        value = parseDate(input.defaultValue);
        write();
      }),
    );
  }
}

// ---- Start ------------------------------------------------------------------

// Last, so every helper above is initialized.
mountAll(document);
// Streamed content and client-side navigations add components later.
new MutationObserver((records) => {
  for (const r of records) for (const n of r.addedNodes) if (n.nodeType === 1) mountAll(n);
}).observe(html, { childList: true, subtree: true });
"####;

/// [`UI_JS`], minified.
pub const UI_JS_MIN: &str = r####"const e=()=>window.nextRust||{},t=document.documentElement;t.classList.add("nr-ui-js");const n=new Map;function a(e){for(const[t,a]of n)t!==e&&a()}document.addEventListener("pointerdown",e=>{for(const[t,a]of n)t.contains(e.target)||a()},!0);const r="function"==typeof HTMLElement.prototype.showPopover,o=new Map;function s(e,t,n){const a=t.getBoundingClientRect();if(!r){e.removeAttribute("data-placement"),e.removeAttribute("data-align");const t=e.offsetHeight;return a.bottom+t+12>innerHeight&&a.top-t-12>0&&(e.dataset.placement="top"),void(a.left+e.offsetWidth+8>innerWidth&&a.right-e.offsetWidth>0&&(e.dataset.align="end"))}const o=n?a.width+4:e.offsetWidth,s=Math.max(8,Math.min(a.left-2,innerWidth-o-8)),l=e.offsetHeight,i=a.bottom+6+l<=innerHeight-8||a.top-6-l<8;e.dataset.placement=i?"bottom":"top",Object.assign(e.style,{position:"fixed",inset:"auto",margin:"0",left:`${s}px`,top:`${i?a.bottom+6:a.top-6-l}px`,width:n?`${o}px`:""})}function l(e,t,n){e.hidden=!1,r&&(e.popover="manual",e.showPopover(),o.set(e,[t,n])),s(e,t,n)}function i(e){r&&e.matches(":popover-open")&&e.hidePopover(),o.delete(e),e.hidden=!0}const d=()=>{for(const[e,[t,n]]of o)s(e,t,n)};addEventListener("scroll",d,{capture:!0,passive:!0}),addEventListener("resize",d,{passive:!0});const c=new WeakSet;function u(e){if(c.has(e))return;c.add(e);const r=e.dataset.nrUi;var o;"select"===r?function(e){const t=e.querySelector(".nr-select-native"),r=e.querySelector(".nr-select-trigger"),o=e.querySelector(".nr-select-list"),s=e.querySelector(".nr-select-value"),d=e.querySelector(".nr-field-wrap");if(!(t&&r&&o&&s))return;const c=t.multiple,u=[...t.options].find(e=>""===e.value),f=u?u.textContent:"",p=()=>[...o.querySelectorAll('[role="option"]')],m=()=>p().filter(e=>"true"!==e.getAttribute("aria-disabled"));let v=null,h="",b=0;function y(){const n=new Set([...t.selectedOptions].map(e=>e.value).filter(e=>""!==e)),a=[];for(const e of p()){const t=n.has(e.dataset.value);e.setAttribute("aria-selected",String(t)),t&&a.push(e.querySelector(".nr-select-option-label").textContent)}s.textContent=a.length?a.join(", "):f,s.toggleAttribute("data-placeholder",!a.length),e.toggleAttribute("data-filled",a.length>0)}function w(e,t=!0){v&&v.removeAttribute("data-active"),v=e||null,v?(v.setAttribute("data-active",""),r.setAttribute("aria-activedescendant",v.id),t&&v.scrollIntoView({block:"nearest"})):r.removeAttribute("aria-activedescendant")}function k(){o.hidden&&!t.disabled&&(a(e),l(o,d,!0),e.setAttribute("data-open",""),r.setAttribute("aria-expanded","true"),w(m().find(e=>"true"===e.getAttribute("aria-selected"))||m()[0]),n.set(e,D))}function D(t){o.hidden||(i(o),e.removeAttribute("data-open"),r.setAttribute("aria-expanded","false"),w(null),n.delete(e),!0===t&&r.focus())}function E(e){if(!e||"true"===e.getAttribute("aria-disabled"))return;const n=[...t.options].find(t=>t.value===e.dataset.value);n&&(n.selected=!c||!n.selected,g(t),y(),c||D(!0))}r.addEventListener("click",()=>o.hidden?k():D()),d.addEventListener("click",e=>{e.target.closest(".nr-select-list, .nr-select-trigger")||(r.focus(),o.hidden?k():D())}),o.addEventListener("mousedown",e=>e.preventDefault()),o.addEventListener("pointermove",e=>{const t=e.target.closest('[role="option"]');t&&t!==v&&"true"!==t.getAttribute("aria-disabled")&&w(t,!1)}),o.addEventListener("click",e=>{e.stopPropagation(),E(e.target.closest('[role="option"]'))}),r.addEventListener("keyup",e=>{" "===e.key&&e.preventDefault()}),r.addEventListener("keydown",e=>{const t=m(),n=t.indexOf(v);switch(e.key){case"ArrowDown":e.preventDefault(),o.hidden?k():w(t[Math.min(n+1,t.length-1)]||t[0]);break;case"ArrowUp":e.preventDefault(),o.hidden?k():w(t[Math.max(n-1,0)]);break;case"Home":case"End":o.hidden||(e.preventDefault(),w("Home"===e.key?t[0]:t[t.length-1]));break;case"Enter":case" ":e.preventDefault(),o.hidden?k():E(v);break;case"Escape":o.hidden||(e.preventDefault(),D(!0));break;case"Tab":D();break;default:if(1===e.key.length&&!e.ctrlKey&&!e.metaKey&&!e.altKey){const n=Date.now();h=n-b>700?e.key:h+e.key,b=n;const a=h.toLowerCase(),r=t.find(e=>e.textContent.trim().toLowerCase().startsWith(a));r&&(o.hidden&&k(),w(r))}}}),e.querySelector(".nr-field-label")?.addEventListener("click",e=>{e.preventDefault(),r.focus()}),t.addEventListener("change",y),t.form&&t.form.addEventListener("reset",()=>setTimeout(y)),y()}(e):"datepicker"===r?function(e){const r=e.querySelector(".nr-datepicker-input"),o=e.querySelector(".nr-calendar"),d=e.querySelector(".nr-field-wrap");if(!r||!o||!d)return;const c=e.dataset.locale||t.lang||void 0,u=+(e.dataset.firstDay||0),f=D(r.min),p=D(r.max),m=()=>{const e=new Date;return new Date(e.getFullYear(),e.getMonth(),e.getDate())},v=e=>(!f||e>=f)&&(!p||e<=p),h=new Intl.DateTimeFormat(c,{dateStyle:"medium"}),A=new Intl.DateTimeFormat(c,{dateStyle:"full"}),S=new Intl.DateTimeFormat(c,{month:"long",year:"numeric"}),L=new Intl.DateTimeFormat(c,{month:"short"}),x=new Intl.DateTimeFormat(c,{weekday:"short"}),M=document.createElement("input");M.type="hidden",M.name=r.name,r.removeAttribute("name"),r.after(M);let q=D(r.value);r.type="text",r.autocomplete="off",r.setAttribute("inputmode","none"),r.setAttribute("aria-haspopup","dialog"),r.addEventListener("beforeinput",e=>e.preventDefault());const F=()=>{r.value=q?h.format(q):"",M.value=q?b(q):""};F();let Y="days",T=q||m();function C(e,t,n){const a=document.createElement("button");a.type="button",a.className=e;for(const[e,n]of Object.entries(t))!1!==n&&null!=n&&a.setAttribute(e,!0===n?"":n);return a.innerHTML=n,a}function P(e){o.textContent="",o.dataset.view=Y;const t=document.createElement("div");t.className="nr-cal-header";const n="days"===Y,a=n?E(k(T),-1):new Date(T.getFullYear()-1,11,31),r=n?E(k(T),1):new Date(T.getFullYear()+1,0,1),l=n?new Date(a.getFullYear(),a.getMonth()+1,0):a;if(t.append(C("nr-cal-nav",{"data-cal":"prev","aria-label":n?"Previous month":"Previous year",disabled:!!f&&l<f},'<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 18-6-6 6-6"/></svg>'),C("nr-cal-title",{"data-cal":"view","aria-live":"polite"},`<span>${n?S.format(T):T.getFullYear()}</span><svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>`),C("nr-cal-nav",{"data-cal":"next","aria-label":n?"Next month":"Next year",disabled:!!p&&r>p},'<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>')),o.append(t),n){const e=document.createElement("div");e.className="nr-cal-grid",e.setAttribute("role","grid");for(let t=0;t<7;t++){const n=document.createElement("div");n.className="nr-cal-weekday",n.setAttribute("role","columnheader"),n.textContent=x.format(new Date(2023,0,1+(t+u)%7)),e.append(n)}const t=k(T),n=(t.getDay()-u+7)%7,a=m();for(let r=0;r<42;r++){const o=w(t,r-n);e.append(C("nr-cal-day",{role:"gridcell","data-date":b(o),"aria-label":A.format(o),"aria-selected":y(o,q)?"true":"false","data-today":y(o,a),"data-outside":o.getMonth()!==T.getMonth(),tabindex:y(o,T)?"0":"-1",disabled:!v(o)},String(o.getDate())))}o.append(e)}else{const e=document.createElement("div");e.className="nr-cal-months";for(let t=0;t<12;t++){const n=new Date(T.getFullYear(),t,1),a=new Date(T.getFullYear(),t+1,0);e.append(C("nr-cal-month",{"data-month":String(t),"aria-selected":q&&q.getFullYear()===T.getFullYear()&&q.getMonth()===t?"true":"false","data-current":t===T.getMonth(),tabindex:t===T.getMonth()?"0":"-1",disabled:!!f&&a<f||!!p&&n>p},L.format(n)))}o.append(e)}const i=document.createElement("div");i.className="nr-cal-footer",i.append(C("nr-cal-link",{"data-cal":"today",disabled:!v(m())},"Today"),C("nr-cal-link",{"data-cal":"clear"},"Clear")),o.append(i),o.hidden||s(o,d,!1),e&&o.querySelector('[tabindex="0"]:not(:disabled)')?.focus({preventScroll:!0})}function H(){!o.hidden||r.disabled||r.readOnly||(a(e),Y="days",T=q||(v(m())?m():f||p||m()),P(!1),l(o,d,!1),e.setAttribute("data-open",""),e.querySelector(".nr-datepicker-toggle")?.setAttribute("aria-expanded","true"),o.querySelector('[tabindex="0"]:not(:disabled)')?.focus({preventScroll:!0}),n.set(e,I))}function I(t){o.hidden||(i(o),e.removeAttribute("data-open"),e.querySelector(".nr-datepicker-toggle")?.setAttribute("aria-expanded","false"),n.delete(e),!0===t&&r.focus({preventScroll:!0}))}function N(e){q=e,F(),g(M),I(!0)}function O(e){f&&e<f&&(e=f),p&&e>p&&(e=p),T=e,P(!0)}o.addEventListener("click",e=>{const t=e.target.closest("button");t&&!t.disabled&&(t.dataset.date?N(D(t.dataset.date)):t.dataset.month?(T=new Date(T.getFullYear(),+t.dataset.month,Math.min(T.getDate(),28)),Y="days",P(!0)):"prev"===t.dataset.cal?(T="days"===Y?E(T,-1):new Date(T.getFullYear()-1,T.getMonth(),1),P(!1)):"next"===t.dataset.cal?(T="days"===Y?E(T,1):new Date(T.getFullYear()+1,T.getMonth(),1),P(!1)):"view"===t.dataset.cal?(Y="days"===Y?"months":"days",P(!0)):"today"===t.dataset.cal?N(m()):"clear"===t.dataset.cal&&(q=null,F(),g(M),I(!0)))}),o.addEventListener("keydown",e=>{if("Escape"===e.key)return e.preventDefault(),void I(!0);const t=e.target.dataset&&e.target.dataset.date,n=e.target.dataset&&null!=e.target.dataset.month;if(!t&&!n)return;const a={ArrowLeft:-1,ArrowRight:1,ArrowUp:t?-7:-3,ArrowDown:t?7:3}[e.key];if(null!=a)if(e.preventDefault(),t)O(w(T,a));else{const e=Math.min(11,Math.max(0,T.getMonth()+a));T=new Date(T.getFullYear(),e,1),P(!0)}else if(!t||"PageUp"!==e.key&&"PageDown"!==e.key){if(t&&("Home"===e.key||"End"===e.key)){e.preventDefault();const t=(T.getDay()-u+7)%7;O(w(T,"Home"===e.key?-t:6-t))}}else e.preventDefault(),O(E(T,("PageUp"===e.key?-1:1)*(e.shiftKey?12:1)))}),e.addEventListener("focusout",t=>{o.hidden||!t.relatedTarget||e.contains(t.relatedTarget)||I()}),r.addEventListener("click",H),r.addEventListener("keydown",e=>{"Enter"===e.key||" "===e.key||"ArrowDown"===e.key?(e.preventDefault(),H()):"Backspace"!==e.key&&"Delete"!==e.key||!q||r.required||(e.preventDefault(),q=null,F(),g(M))}),d.addEventListener("click",e=>{e.target.closest(".nr-calendar")||(e.target.closest(".nr-datepicker-toggle")&&!o.hidden?I(!0):H())}),r.form&&r.form.addEventListener("reset",()=>setTimeout(()=>{q=D(r.defaultValue),F()}))}(e):"avatar"===r?function(e){const t=e.querySelector(".nr-avatar-img");if(!t)return;const n=()=>t.hidden=!0;t.complete&&0===t.naturalWidth&&t.getAttribute("src")?n():t.addEventListener("error",n,{once:!0})}(e):"tabs"===r?function(e){const t=e.querySelector('[role="tablist"]');if(!t)return;const n=()=>[...t.querySelectorAll('[role="tab"]')];function a(t,a){if(t&&!t.disabled){for(const e of n()){const n=e===t;e.classList.toggle("nr-tab-selected",n),e.setAttribute("aria-selected",String(n)),e.tabIndex=n?0:-1;const a=e.getAttribute("aria-controls")&&document.getElementById(e.getAttribute("aria-controls"));a&&(a.hidden=!n)}a&&t.focus({preventScroll:!0}),p(e,"nr:change",{key:t.dataset.key})}}t.addEventListener("click",e=>{const n=e.target.closest('[role="tab"]');n&&t.contains(n)&&a(n)}),t.addEventListener("keydown",e=>{const t=n().filter(e=>!e.disabled),r=t.indexOf(e.target.closest('[role="tab"]'));if(r<0)return;const o={ArrowRight:r+1,ArrowLeft:r-1,Home:0,End:t.length-1}[e.key];void 0!==o&&(e.preventDefault(),a(t[(o+t.length)%t.length],!0))})}(e):"modal"===r&&((o=e).open&&(o.close(),v(o)),o.addEventListener("click",e=>{e.target===o&&o.hasAttribute("data-dismissable")&&o.close()}),o.addEventListener("close",()=>p(o,"nr:close",null)))}function f(e){e.matches&&e.matches("[data-nr-ui]")&&u(e);for(const t of e.querySelectorAll("[data-nr-ui]"))u(t)}const p=(e,t,n)=>e.dispatchEvent(new CustomEvent(t,{bubbles:!0,detail:n})),g=e=>{e.dispatchEvent(new Event("input",{bubbles:!0})),e.dispatchEvent(new Event("change",{bubbles:!0}))};function m(e){const t=document.querySelector(".nr-shell");if(t){t.toggleAttribute("data-open",e);for(const t of document.querySelectorAll(".nr-navbar-menu"))t.setAttribute("aria-expanded",String(e))}}function v(e){e&&!e.open&&(e.showModal(),p(e,"nr:open",null))}document.addEventListener("click",async t=>{const n=t.target.closest&&t.target.closest("[data-nr-press]");if(!n||n.matches(':disabled, [aria-disabled="true"], [data-loading]'))return;const a=n.dataset.nrPress,r=n.dataset.nrPressTarget||"";if("navigate"===a)t.preventDefault(),e().navigate?e().navigate(r):location.href=r;else if("modal"===a)t.preventDefault(),v(document.getElementById(r));else if("close-modal"===a){t.preventDefault();const e=n.closest("dialog");e&&e.close()}else if("emit"===a)p(n,"nr:"+r,null);else if("action"===a){t.preventDefault();let a=null;try{a=JSON.parse(n.dataset.nrPressInput||"null")}catch{}n.setAttribute("data-loading",""),n.setAttribute("aria-busy","true");try{const t=await e().action(r,a);p(n,"nr:success",t),"false"!==n.dataset.nrPressRefresh&&e().refresh&&e().refresh()}catch(e){p(n,"nr:error",e),n.hasAttribute("data-nr-quiet")||console.error("[next-rust] action failed:",e.message)}finally{n.removeAttribute("data-loading"),n.removeAttribute("aria-busy")}}}),document.addEventListener("keydown",e=>{const t=e.target;("Enter"===e.key||" "===e.key)&&t.matches&&t.matches('[data-nr-press][role="button"]')&&(e.preventDefault(),t.click())}),document.addEventListener("click",e=>{const t=e.target.closest&&e.target.closest("[data-nr-field-action]");if(t){const e=t.closest(".nr-field")?.querySelector("input.nr-field-input, textarea.nr-field-input"),n=t.dataset.nrFieldAction;if("toggle-password"===n&&e){const n="password"===e.type;e.type=n?"text":"password",t.setAttribute("aria-pressed",String(n)),t.setAttribute("aria-label",n?"Hide password":"Show password"),e.focus({preventScroll:!0});const a=e.value.length;e.setSelectionRange(a,a)}else"clear"===n&&e&&(e.value="",g(e),e.focus());return}const n=e.target.closest&&e.target.closest(".nr-input .nr-field-wrap, .nr-textarea .nr-field-wrap");n&&!e.target.closest("input, textarea, button, a, select")&&n.querySelector(".nr-field-input")?.focus()}),document.addEventListener("click",e=>{e.target.closest&&e.target.closest("[data-nr-shell-toggle]")?m(!document.querySelector(".nr-shell[data-open]")):e.target.closest&&e.target.closest(".nr-shell-sidebar a[href]")&&m(!1)}),window.addEventListener("nr:navigate",()=>m(!1)),document.addEventListener("keydown",e=>{"Escape"===e.key&&document.querySelector(".nr-shell[data-open]")&&m(!1)}),document.addEventListener("click",e=>{const t=e.target.closest&&e.target.closest("[data-nr-dismiss]"),n=t&&t.closest('[data-nr-ui="alert"]');n&&(p(n,"nr:dismiss",null),n.remove())});const h=(e,t=2)=>String(e).padStart(t,"0"),b=e=>`${h(e.getFullYear(),4)}-${h(e.getMonth()+1)}-${h(e.getDate())}`,y=(e,t)=>!!e&&!!t&&b(e)===b(t),w=(e,t)=>new Date(e.getFullYear(),e.getMonth(),e.getDate()+t),k=e=>new Date(e.getFullYear(),e.getMonth(),1);function D(e){const t=/^(\d{4})-(\d{2})-(\d{2})$/.exec(e||"");if(!t)return null;const n=new Date(+t[1],+t[2]-1,+t[3]);return n.getMonth()===+t[2]-1?n:null}function E(e,t){const n=new Date(e.getFullYear(),e.getMonth()+t,1),a=new Date(n.getFullYear(),n.getMonth()+1,0).getDate();return new Date(n.getFullYear(),n.getMonth(),Math.min(e.getDate(),a))}f(document),new MutationObserver(e=>{for(const t of e)for(const e of t.addedNodes)1===e.nodeType&&f(e)}).observe(t,{childList:!0,subtree:!0});"####;

/// `script` with every `nr-*` class name replaced through `rename`, for
/// release builds that give the component classes short names. A class name
/// is a whole `nr-…` token (so `data-nr-ui` is left alone).
pub fn rename_classes(script: &str, rename: &dyn Fn(&str) -> Option<String>) -> String {
    let part = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
    let bytes = script.as_bytes();
    let mut out = String::with_capacity(script.len());
    let mut last = 0;
    let mut i = 0;
    while let Some(at) = script[i..].find("nr-").map(|at| i + at) {
        let end =
            at + bytes[at..].iter().take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || **b == b'-').count();
        if at > 0 && part(bytes[at - 1]) {
            i = end;
            continue;
        }
        if let Some(short) = rename(&script[at..end]) {
            out.push_str(&script[last..at]);
            out.push_str(&short);
            last = end;
        }
        i = end;
    }
    out.push_str(&script[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Content hash of [`UI_JS`] that [`UI_JS_MIN`] was generated from.
    const UI_JS_SOURCE_HASH: &str = "7faa71b0c48d8972";

    #[test]
    fn minified_script_matches_source() {
        assert_eq!(
            next_rust_assets::content_hash(UI_JS.as_bytes()),
            UI_JS_SOURCE_HASH,
            "UI_JS changed: regenerate UI_JS_MIN and UI_JS_SOURCE_HASH (see CONTRIBUTING.md)"
        );
        assert!(!UI_JS_MIN.contains('\n'), "minified to one line");
        assert!(UI_JS_MIN.len() < UI_JS.len() * 2 / 3);
    }

    #[test]
    fn class_names_are_renamed_as_whole_tokens() {
        let rename = |c: &str| match c {
            "nr-field" => Some("a".to_owned()),
            "nr-field-wrap" => Some("b".to_owned()),
            "nr-ui-js" => Some("c".to_owned()),
            _ => None,
        };
        let js = r#"x.querySelector(".nr-field .nr-field-wrap");h.classList.add("nr-ui-js");e.dataset.nrUi;"[data-nr-ui]";"nr-other""#;
        assert_eq!(
            rename_classes(js, &rename),
            r#"x.querySelector(".a .b");h.classList.add("c");e.dataset.nrUi;"[data-nr-ui]";"nr-other""#
        );
    }

    #[test]
    fn minified_script_keeps_public_contract() {
        for needle in
            ["nr-ui-js", "data-nr-ui", "data-nr-press", "data-nr-field-action", "data-nr-shell-toggle", "nr:navigate"]
        {
            assert!(UI_JS_MIN.contains(needle), "minified script lost `{needle}`");
        }
    }
}
