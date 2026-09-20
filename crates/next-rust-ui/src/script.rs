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
// panels, modals, closable alerts, menu keys, the command palette, toasts,
// the combobox list, tags, carousel buttons and the copy button.

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
  // Fields have a 2px border the list lines up with; menus line up with their trigger.
  const align = pop.dataset.nrAlign;
  const start = align === "end" ? r.right - width : r.left - (align ? 0 : 2);
  const left = Math.max(8, Math.min(start, innerWidth - width - 8));
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
  else if (kind === "dropdown" || kind === "popover") setupPopover(el);
  else if (kind === "context") setupContext(el);
  else if (kind === "command") setupCommand(el);
  else if (kind === "toaster") showToaster(el);
  else if (kind === "combobox") setupCombobox(el);
  else if (kind === "tags") setupTags(el);
  else if (kind === "upload") setupUpload(el);
  else if (kind === "carousel") setupCarousel(el);
  else if (kind === "banner") setupBanner(el);
  else if (kind === "checkall") setupCheckAll(el);
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
  } else if (kind === "toast") {
    toast(target);
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
    } else if ((action === "step-up" || action === "step-down") && input) {
      try {
        if (action === "step-up") input.stepUp();
        else input.stepDown();
      } catch {}
      changed(input);
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
  // Toasts stay above the dialog.
  const toaster = document.querySelector('[data-nr-ui="toaster"]');
  if (toaster) showToaster(toaster);
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
  const alert = btn && btn.closest('[data-nr-ui="alert"], .nr-toast');
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

// ---- Menus, popovers, context menus -------------------------------------------

// With CSS anchor positioning the browser places a popover next to the
// button that opened it; elsewhere `position` does.
const anchored = typeof CSS !== "undefined" && CSS.supports("position-area: block-end");

function setupPopover(root) {
  const pop = root.querySelector("[popover]");
  const trigger = root.querySelector("[popovertarget]");
  if (!pop || !topLayer) return;
  const menu = pop.getAttribute("role") === "menu";
  const items = () => [...pop.querySelectorAll('[role="menuitem"]')].filter((i) => !i.matches(':disabled, [aria-disabled="true"]'));
  pop.addEventListener("beforetoggle", (e) => {
    if (e.newState !== "open" || anchored || !trigger) return;
    // Shown by the time the frame runs, not painted yet.
    requestAnimationFrame(() => {
      position(pop, trigger, false);
      anchors.set(pop, [trigger, false]);
    });
  });
  pop.addEventListener("toggle", (e) => {
    const on = e.newState === "open";
    if (trigger) trigger.setAttribute("aria-expanded", String(on));
    if (on && menu) (items()[0] || pop).focus({ preventScroll: true });
    if (!on) anchors.delete(pop);
    fire(root, on ? "nr:open" : "nr:close", null);
  });
  if (!menu) return;
  pop.addEventListener("keydown", (e) => {
    const all = items();
    const i = all.indexOf(document.activeElement);
    let to = { ArrowDown: i + 1, ArrowUp: i < 0 ? -1 : i - 1, Home: 0, End: all.length - 1 }[e.key];
    if (to === undefined && e.key.length === 1 && /\S/.test(e.key) && !e.ctrlKey && !e.metaKey) {
      // Type-ahead: the next item starting with the key.
      const k = e.key.toLowerCase();
      const order = [...all.slice(i + 1), ...all.slice(0, i + 1)];
      to = all.indexOf(order.find((o) => o.textContent.trim().toLowerCase().startsWith(k)));
      if (to < 0) return;
    }
    if (e.key === "Tab") pop.hidePopover();
    if (to === undefined || !all.length) return;
    e.preventDefault();
    all[(to + all.length) % all.length].focus({ preventScroll: true });
  });
  pop.addEventListener("click", (e) => {
    if (e.target.closest('[role="menuitem"]')) pop.hidePopover();
  });
  if (trigger) {
    trigger.addEventListener("keydown", (e) => {
      if ((e.key === "ArrowDown" || e.key === "ArrowUp") && !pop.matches(":popover-open")) {
        e.preventDefault();
        trigger.click();
      }
    });
  }
}

function setupContext(root) {
  setupPopover(root);
  const pop = root.querySelector("[popover]");
  if (!pop || !topLayer) return;
  // A manual popover: the pointer-up of the right click would dismiss an
  // automatic one at once. Closing is done here.
  const close = () => pop.matches(":popover-open") && pop.hidePopover();
  document.addEventListener("pointerdown", (e) => !pop.contains(e.target) && close(), true);
  addEventListener("scroll", close, { capture: true, passive: true });
  pop.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  });
  root.addEventListener("contextmenu", (e) => {
    if (pop.contains(e.target)) return;
    e.preventDefault();
    close();
    pop.showPopover();
    const x = Math.min(e.clientX, innerWidth - pop.offsetWidth - 8);
    const y = Math.min(e.clientY, innerHeight - pop.offsetHeight - 8);
    pop.style.left = `${Math.max(8, x)}px`;
    pop.style.top = `${Math.max(8, y)}px`;
  });
}

// ---- Command palette ----------------------------------------------------------------

function setupCommand(dialog) {
  setupModal(dialog);
  const input = dialog.querySelector(".nr-command-input");
  const empty = dialog.querySelector(".nr-command-empty");
  if (!input) return;
  const shown = () => [...dialog.querySelectorAll(".nr-command-item:not([hidden])")];
  let active = null;

  function setActive(el) {
    if (active) {
      active.removeAttribute("data-active");
      active.setAttribute("aria-selected", "false");
    }
    active = el || null;
    if (active) {
      active.setAttribute("data-active", "");
      active.setAttribute("aria-selected", "true");
      input.setAttribute("aria-activedescendant", active.id);
      active.scrollIntoView({ block: "nearest" });
    } else input.removeAttribute("aria-activedescendant");
  }

  function filter() {
    const words = input.value.toLowerCase().split(/\s+/).filter(Boolean);
    for (const item of dialog.querySelectorAll(".nr-command-item")) {
      const text = `${item.textContent} ${item.dataset.keywords || ""}`.toLowerCase();
      item.hidden = !words.every((w) => text.includes(w));
    }
    for (const g of dialog.querySelectorAll(".nr-command-group")) g.hidden = !g.querySelector(".nr-command-item:not([hidden])");
    const all = shown();
    if (empty) empty.hidden = all.length > 0;
    setActive(all[0]);
  }

  input.addEventListener("input", filter);
  input.addEventListener("keydown", (e) => {
    const all = shown();
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    if (step && all.length) {
      e.preventDefault();
      setActive(all[(all.indexOf(active) + step + all.length) % all.length]);
    } else if (e.key === "Enter" && active) {
      e.preventDefault();
      active.click();
    }
  });
  dialog.addEventListener("pointermove", (e) => {
    const item = e.target.closest(".nr-command-item");
    if (item && item !== active) setActive(item);
  });
  dialog.addEventListener("click", (e) => {
    if (e.target.closest(".nr-command-item")) dialog.close();
  });
  dialog.addEventListener("nr:open", () => {
    input.value = "";
    filter();
    input.focus();
  });
  const hotkey = dialog.dataset.hotkey;
  if (hotkey) {
    document.addEventListener("keydown", (e) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey || e.key.toLowerCase() !== hotkey || !dialog.isConnected) return;
      e.preventDefault();
      if (dialog.open) dialog.close();
      else openModal(dialog);
    });
  }
}

// ---- Toasts -----------------------------------------------------------------------

const TOAST_ICONS = {
  default: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
  success: '<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',
  warning: '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  danger: '<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>',
};
const svg = (body) => `<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

// Above modal dialogs: the top layer, entered after them.
function showToaster(host) {
  if (!topLayer || !host.children.length) return;
  try {
    if (host.matches(":popover-open")) {
      if (!document.querySelector("dialog:modal")) return;
      host.hidePopover();
    }
    host.popover = "manual";
    host.showPopover();
  } catch {}
}

// `nextRust.toast("Saved")`, or `{ title, message, color, duration }`
// (`color`: success, warning, danger, primary, secondary; `duration` in ms, 0 to stay).
function toast(options) {
  const host = document.querySelector('[data-nr-ui="toaster"]');
  if (!host) return null;
  const o = typeof options === "string" ? { message: options } : options || {};
  const color = o.color || "default";
  const el = document.createElement("div");
  el.className = o.duration === 0 ? "nr-toast nr-toast-sticky" : "nr-toast";
  el.dataset.color = color;
  el.setAttribute("role", color === "danger" || color === "warning" ? "alert" : "status");
  if (o.duration > 0) el.style.setProperty("--nr-toast-d", `${o.duration}ms`);
  el.innerHTML = `<span class="nr-toast-icon">${svg(TOAST_ICONS[color] || TOAST_ICONS.default)}</span><div class="nr-toast-body"></div><button type="button" class="nr-toast-close" aria-label="Dismiss" data-nr-dismiss>${svg('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>')}</button>`;
  const body = el.children[1];
  for (const [tag, cls, text] of [["strong", "nr-toast-title", o.title], ["div", "nr-toast-text", o.message]]) {
    if (text == null || text === "") continue;
    const part = document.createElement(tag);
    part.className = cls;
    part.textContent = text;
    body.append(part);
  }
  host.append(el);
  showToaster(host);
  return el;
}

(window.nextRust = window.nextRust || {}).toast = toast;
addEventListener("nr:toast", (e) => toast(e.detail));
// A toast that has faded out (the stylesheet times it) leaves the page.
document.addEventListener("animationend", (e) => {
  const el = e.target;
  if (el.matches && el.matches(".nr-toast") && getComputedStyle(el).visibility === "hidden") el.remove();
});

// ---- Combobox ----------------------------------------------------------------------

function setupCombobox(root) {
  const input = root.querySelector(".nr-field-input");
  const list = root.querySelector(".nr-combobox-list");
  const wrap = root.querySelector(".nr-field-wrap");
  if (!input || !list || !input.list) return;
  const values = [...input.list.options].map((o) => o.value);
  input.removeAttribute("list");
  input.setAttribute("role", "combobox");
  input.setAttribute("aria-autocomplete", "list");
  input.setAttribute("aria-expanded", "false");
  input.setAttribute("aria-controls", list.id);
  let active = null;

  function setActive(li) {
    if (active) active.removeAttribute("data-active");
    active = li || null;
    if (active) {
      active.setAttribute("data-active", "");
      input.setAttribute("aria-activedescendant", active.id);
      active.scrollIntoView({ block: "nearest" });
    } else input.removeAttribute("aria-activedescendant");
  }

  function render() {
    const q = input.value.trim().toLowerCase();
    const matches = values.filter((v) => v.toLowerCase().includes(q));
    list.textContent = "";
    matches.forEach((v, i) => {
      const li = document.createElement("li");
      li.id = `${list.id}-${i}`;
      li.setAttribute("role", "option");
      li.setAttribute("aria-selected", String(v === input.value));
      li.textContent = v;
      list.append(li);
    });
    if (!matches.length) {
      const li = document.createElement("li");
      li.setAttribute("role", "presentation");
      li.textContent = "No matches";
      list.append(li);
    }
    setActive(null);
  }

  function show() {
    if (input.disabled || input.readOnly) return;
    render();
    if (!list.hidden) return position(list, wrap, true);
    closeAll(root);
    showPop(list, wrap, true);
    input.setAttribute("aria-expanded", "true");
    open.set(root, hide);
  }

  function hide() {
    if (list.hidden) return;
    hidePop(list);
    input.setAttribute("aria-expanded", "false");
    setActive(null);
    open.delete(root);
  }

  function choose(li) {
    input.value = li.textContent;
    changed(input);
    hide();
  }

  input.addEventListener("input", show);
  input.addEventListener("click", show);
  input.addEventListener("keydown", (e) => {
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    if (step) {
      e.preventDefault();
      if (list.hidden) show();
      const all = [...list.querySelectorAll('[role="option"]')];
      if (all.length) setActive(all[(all.indexOf(active) + step + all.length) % all.length]);
    } else if (e.key === "Enter" && active) {
      e.preventDefault();
      choose(active);
    } else if (e.key === "Escape" && !list.hidden) {
      e.preventDefault();
      hide();
    }
  });
  // Keep the focus in the input while choosing with the pointer.
  list.addEventListener("pointerdown", (e) => e.preventDefault());
  list.addEventListener("click", (e) => {
    const li = e.target.closest('[role="option"]');
    if (li) choose(li);
  });
  input.addEventListener("blur", hide);
}

// ---- Tags input ----------------------------------------------------------------------

function setupTags(root) {
  const input = root.querySelector(".nr-field-input");
  if (!input) return;
  // The form receives the tags from a hidden input, "a, b, c": the same
  // string the plain text field submits without this script.
  const hidden = document.createElement("input");
  hidden.type = "hidden";
  hidden.name = input.name;
  input.removeAttribute("name");
  const row = document.createElement("div");
  row.className = "nr-tags-row";
  input.before(row);
  row.append(input, hidden);
  const max = +root.dataset.max || Infinity;
  const floats = root.hasAttribute("data-float");
  const required = input.required;
  const initial = input.value;
  let tags = [];

  function sync() {
    hidden.value = tags.join(", ");
    for (const chip of row.querySelectorAll(".nr-tags-chip")) chip.remove();
    tags.forEach((tag, i) => {
      const chip = document.createElement("span");
      chip.className = "nr-tags-chip";
      chip.textContent = tag;
      const remove = document.createElement("button");
      remove.type = "button";
      remove.textContent = "×";
      remove.dataset.index = i;
      remove.setAttribute("aria-label", `Remove ${tag}`);
      chip.append(remove);
      input.before(chip);
    });
    root.toggleAttribute("data-float", floats && !tags.length);
    input.required = required && !tags.length;
  }

  function add(text, quiet) {
    const before = tags.length;
    for (const part of text.split(",")) {
      const tag = part.trim();
      if (tag && !tags.includes(tag) && tags.length < max) tags.push(tag);
    }
    input.value = "";
    sync();
    if (!quiet && tags.length !== before) fire(root, "nr:change", { values: [...tags] });
  }

  add(initial, true);
  input.addEventListener("keydown", (e) => {
    if ((e.key === "Enter" || e.key === ",") && input.value.trim()) {
      e.preventDefault();
      add(input.value);
    } else if (e.key === "," ) {
      e.preventDefault();
    } else if (e.key === "Backspace" && !input.value && tags.length) {
      tags.pop();
      sync();
      fire(root, "nr:change", { values: [...tags] });
    }
  });
  input.addEventListener("change", () => add(input.value));
  input.addEventListener("blur", () => add(input.value));
  row.addEventListener("click", (e) => {
    const remove = e.target.closest("button[data-index]");
    if (!remove) return;
    tags.splice(+remove.dataset.index, 1);
    sync();
    fire(root, "nr:change", { values: [...tags] });
    input.focus();
  });
  if (input.form) {
    input.form.addEventListener("reset", () =>
      setTimeout(() => {
        tags = [];
        add(initial, true);
      }),
    );
  }
}

// ---- File upload ---------------------------------------------------------------------

function setupUpload(root) {
  const zone = root.querySelector(".nr-dropzone");
  const input = root.querySelector(".nr-dropzone-input");
  const files = root.querySelector(".nr-dropzone-files");
  if (!zone || !input) return;
  for (const name of ["dragenter", "dragover"]) zone.addEventListener(name, () => zone.setAttribute("data-drag", ""));
  for (const name of ["dragleave", "drop"]) zone.addEventListener(name, () => zone.removeAttribute("data-drag"));
  input.addEventListener("change", () => {
    if (!files) return;
    files.textContent = "";
    for (const f of input.files) {
      const li = document.createElement("li");
      li.textContent = `${f.name} (${f.size < 1e6 ? `${Math.ceil(f.size / 1e3)} KB` : `${(f.size / 1e6).toFixed(1)} MB`})`;
      files.append(li);
    }
    files.hidden = !input.files.length;
  });
}

// ---- Slider and color value, copy button ---------------------------------------------

document.addEventListener("input", (e) => {
  const el = e.target;
  if (!el.matches || !el.matches(".nr-slider-input, .nr-color-input")) return;
  const root = el.closest("[data-nr-ui]");
  if (!root) return;
  if (el.type === "range") {
    const min = +el.min || 0;
    const max = el.max === "" ? 100 : +el.max;
    root.style.setProperty("--nr-v", `${((el.value - min) / (max - min || 1)) * 100}%`);
  }
  const out = root.querySelector("[data-nr-echo]");
  if (out) out.textContent = el.value + (out.dataset.unit || "");
});

document.addEventListener("click", async (e) => {
  const btn = e.target.closest && e.target.closest("[data-nr-copy]");
  const pre = btn && btn.closest(".nr-codeblock")?.querySelector("pre");
  if (!pre) return;
  try {
    await navigator.clipboard.writeText(pre.textContent);
  } catch {
    return;
  }
  btn.setAttribute("data-copied", "");
  btn.setAttribute("aria-label", "Copied");
  fire(btn, "nr:copy", null);
  setTimeout(() => {
    btn.removeAttribute("data-copied");
    btn.setAttribute("aria-label", "Copy code");
  }, 1500);
});

// ---- Carousel ------------------------------------------------------------------------

function setupCarousel(root) {
  const view = root.querySelector(".nr-carousel-viewport");
  const prev = root.querySelector(".nr-carousel-prev");
  const next = root.querySelector(".nr-carousel-next");
  if (!view || !prev || !next) return;
  const rtl = getComputedStyle(view).direction === "rtl";
  const update = () => {
    const x = Math.abs(view.scrollLeft);
    prev.setAttribute("aria-disabled", String(x < 2));
    next.setAttribute("aria-disabled", String(x > view.scrollWidth - view.clientWidth - 2));
  };
  root.addEventListener("click", (e) => {
    const btn = e.target.closest("[data-nr-slide]");
    if (!btn || btn.getAttribute("aria-disabled") === "true") return;
    const slide = view.querySelector(".nr-carousel-item");
    const gap = parseFloat(getComputedStyle(view).columnGap) || 0;
    // One slide at a time; a page of them when several show at once.
    const per = slide ? Math.max(1, Math.round(view.clientWidth / (slide.offsetWidth + gap))) : 1;
    const by = slide ? (slide.offsetWidth + gap) * per : view.clientWidth;
    view.scrollBy({ left: +btn.dataset.nrSlide * by * (rtl ? -1 : 1) });
  });
  view.addEventListener("scroll", update, { passive: true });
  new ResizeObserver(update).observe(view);
  update();
}

// ---- Banner, table selection -------------------------------------------------------------

function setupBanner(el) {
  const key = `nr-banner:${el.dataset.nrPersist}`;
  try {
    if (localStorage.getItem(key)) return el.remove();
  } catch {}
  el.addEventListener("change", () => {
    try {
      localStorage.setItem(key, "1");
    } catch {}
    fire(el, "nr:dismiss", null);
  });
}

function setupCheckAll(all) {
  const table = all.closest("table");
  if (!table) return;
  const rows = () => [...table.querySelectorAll("tbody .nr-table-checkbox")];
  const sync = () => {
    const r = rows();
    const n = r.filter((c) => c.checked).length;
    all.checked = n > 0 && n === r.length;
    all.indeterminate = n > 0 && n < r.length;
  };
  table.addEventListener("change", (e) => {
    if (e.target === all) for (const c of rows()) c.checked = all.checked;
    else if (!e.target.matches(".nr-table-checkbox")) return;
    sync();
    fire(table, "nr:change", { selected: rows().filter((c) => c.checked).map((c) => c.value) });
  });
  sync();
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
pub const UI_JS_MIN: &str = r####"const e=()=>window.nextRust||{},t=document.documentElement;t.classList.add("nr-ui-js");const n=new Map;function r(e){for(const[t,r]of n)t!==e&&r()}document.addEventListener("pointerdown",e=>{for(const[t,r]of n)t.contains(e.target)||r()},!0);const a="function"==typeof HTMLElement.prototype.showPopover,o=new Map;function s(e,t,n){const r=t.getBoundingClientRect();if(!a){e.removeAttribute("data-placement"),e.removeAttribute("data-align");const t=e.offsetHeight;return r.bottom+t+12>innerHeight&&r.top-t-12>0&&(e.dataset.placement="top"),void(r.left+e.offsetWidth+8>innerWidth&&r.right-e.offsetWidth>0&&(e.dataset.align="end"))}const o=n?r.width+4:e.offsetWidth,s=e.dataset.nrAlign,i="end"===s?r.right-o:r.left-(s?0:2),l=Math.max(8,Math.min(i,innerWidth-o-8)),d=e.offsetHeight,c=r.bottom+6+d<=innerHeight-8||r.top-6-d<8;e.dataset.placement=c?"bottom":"top",Object.assign(e.style,{position:"fixed",inset:"auto",margin:"0",left:`${l}px`,top:`${c?r.bottom+6:r.top-6-d}px`,width:n?`${o}px`:""})}function i(e,t,n){e.hidden=!1,a&&(e.popover="manual",e.showPopover(),o.set(e,[t,n])),s(e,t,n)}function l(e){a&&e.matches(":popover-open")&&e.hidePopover(),o.delete(e),e.hidden=!0}const d=()=>{for(const[e,[t,n]]of o)s(e,t,n)};addEventListener("scroll",d,{capture:!0,passive:!0}),addEventListener("resize",d,{passive:!0});const c=new WeakSet;function u(e){if(c.has(e))return;c.add(e);const o=e.dataset.nrUi;"select"===o?function(e){const t=e.querySelector(".nr-select-native"),a=e.querySelector(".nr-select-trigger"),o=e.querySelector(".nr-select-list"),s=e.querySelector(".nr-select-value"),d=e.querySelector(".nr-field-wrap");if(!(t&&a&&o&&s))return;const c=t.multiple,u=[...t.options].find(e=>""===e.value),p=u?u.textContent:"",f=()=>[...o.querySelectorAll('[role="option"]')],v=()=>f().filter(e=>"true"!==e.getAttribute("aria-disabled"));let g=null,h="",y=0;function b(){const n=new Set([...t.selectedOptions].map(e=>e.value).filter(e=>""!==e)),r=[];for(const e of f()){const t=n.has(e.dataset.value);e.setAttribute("aria-selected",String(t)),t&&r.push(e.querySelector(".nr-select-option-label").textContent)}s.textContent=r.length?r.join(", "):p,s.toggleAttribute("data-placeholder",!r.length),e.toggleAttribute("data-filled",r.length>0)}function w(e,t=!0){g&&g.removeAttribute("data-active"),g=e||null,g?(g.setAttribute("data-active",""),a.setAttribute("aria-activedescendant",g.id),t&&g.scrollIntoView({block:"nearest"})):a.removeAttribute("aria-activedescendant")}function k(){o.hidden&&!t.disabled&&(r(e),i(o,d,!0),e.setAttribute("data-open",""),a.setAttribute("aria-expanded","true"),w(v().find(e=>"true"===e.getAttribute("aria-selected"))||v()[0]),n.set(e,E))}function E(t){o.hidden||(l(o),e.removeAttribute("data-open"),a.setAttribute("aria-expanded","false"),w(null),n.delete(e),!0===t&&a.focus())}function A(e){if(!e||"true"===e.getAttribute("aria-disabled"))return;const n=[...t.options].find(t=>t.value===e.dataset.value);n&&(n.selected=!c||!n.selected,m(t),b(),c||E(!0))}a.addEventListener("click",()=>o.hidden?k():E()),d.addEventListener("click",e=>{e.target.closest(".nr-select-list, .nr-select-trigger")||(a.focus(),o.hidden?k():E())}),o.addEventListener("mousedown",e=>e.preventDefault()),o.addEventListener("pointermove",e=>{const t=e.target.closest('[role="option"]');t&&t!==g&&"true"!==t.getAttribute("aria-disabled")&&w(t,!1)}),o.addEventListener("click",e=>{e.stopPropagation(),A(e.target.closest('[role="option"]'))}),a.addEventListener("keyup",e=>{" "===e.key&&e.preventDefault()}),a.addEventListener("keydown",e=>{const t=v(),n=t.indexOf(g);switch(e.key){case"ArrowDown":e.preventDefault(),o.hidden?k():w(t[Math.min(n+1,t.length-1)]||t[0]);break;case"ArrowUp":e.preventDefault(),o.hidden?k():w(t[Math.max(n-1,0)]);break;case"Home":case"End":o.hidden||(e.preventDefault(),w("Home"===e.key?t[0]:t[t.length-1]));break;case"Enter":case" ":e.preventDefault(),o.hidden?k():A(g);break;case"Escape":o.hidden||(e.preventDefault(),E(!0));break;case"Tab":E();break;default:if(1===e.key.length&&!e.ctrlKey&&!e.metaKey&&!e.altKey){const n=Date.now();h=n-y>700?e.key:h+e.key,y=n;const r=h.toLowerCase(),a=t.find(e=>e.textContent.trim().toLowerCase().startsWith(r));a&&(o.hidden&&k(),w(a))}}}),e.querySelector(".nr-field-label")?.addEventListener("click",e=>{e.preventDefault(),a.focus()}),t.addEventListener("change",b),t.form&&t.form.addEventListener("reset",()=>setTimeout(b)),b()}(e):"datepicker"===o?function(e){const a=e.querySelector(".nr-datepicker-input"),o=e.querySelector(".nr-calendar"),d=e.querySelector(".nr-field-wrap");if(!a||!o||!d)return;const c=e.dataset.locale||t.lang||void 0,u=+(e.dataset.firstDay||0),p=A(a.min),f=A(a.max),v=()=>{const e=new Date;return new Date(e.getFullYear(),e.getMonth(),e.getDate())},g=e=>(!p||e>=p)&&(!f||e<=f),h=new Intl.DateTimeFormat(c,{dateStyle:"medium"}),y=new Intl.DateTimeFormat(c,{dateStyle:"full"}),x=new Intl.DateTimeFormat(c,{month:"long",year:"numeric"}),L=new Intl.DateTimeFormat(c,{month:"short"}),D=new Intl.DateTimeFormat(c,{weekday:"short"}),q=document.createElement("input");q.type="hidden",q.name=a.name,a.removeAttribute("name"),a.after(q);let M=A(a.value);a.type="text",a.autocomplete="off",a.setAttribute("inputmode","none"),a.setAttribute("aria-haspopup","dialog"),a.addEventListener("beforeinput",e=>e.preventDefault());const C=()=>{a.value=M?h.format(M):"",q.value=M?b(M):""};C();let F="days",$=M||v();function P(e,t,n){const r=document.createElement("button");r.type="button",r.className=e;for(const[e,n]of Object.entries(t))!1!==n&&null!=n&&r.setAttribute(e,!0===n?"":n);return r.innerHTML=n,r}function T(e){o.textContent="",o.dataset.view=F;const t=document.createElement("div");t.className="nr-cal-header";const n="days"===F,r=n?S(E($),-1):new Date($.getFullYear()-1,11,31),a=n?S(E($),1):new Date($.getFullYear()+1,0,1),i=n?new Date(r.getFullYear(),r.getMonth()+1,0):r;if(t.append(P("nr-cal-nav",{"data-cal":"prev","aria-label":n?"Previous month":"Previous year",disabled:!!p&&i<p},'<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 18-6-6 6-6"/></svg>'),P("nr-cal-title",{"data-cal":"view","aria-live":"polite"},`<span>${n?x.format($):$.getFullYear()}</span><svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>`),P("nr-cal-nav",{"data-cal":"next","aria-label":n?"Next month":"Next year",disabled:!!f&&a>f},'<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"/></svg>')),o.append(t),n){const e=document.createElement("div");e.className="nr-cal-grid",e.setAttribute("role","grid");for(let t=0;t<7;t++){const n=document.createElement("div");n.className="nr-cal-weekday",n.setAttribute("role","columnheader"),n.textContent=D.format(new Date(2023,0,1+(t+u)%7)),e.append(n)}const t=E($),n=(t.getDay()-u+7)%7,r=v();for(let a=0;a<42;a++){const o=k(t,a-n);e.append(P("nr-cal-day",{role:"gridcell","data-date":b(o),"aria-label":y.format(o),"aria-selected":w(o,M)?"true":"false","data-today":w(o,r),"data-outside":o.getMonth()!==$.getMonth(),tabindex:w(o,$)?"0":"-1",disabled:!g(o)},String(o.getDate())))}o.append(e)}else{const e=document.createElement("div");e.className="nr-cal-months";for(let t=0;t<12;t++){const n=new Date($.getFullYear(),t,1),r=new Date($.getFullYear(),t+1,0);e.append(P("nr-cal-month",{"data-month":String(t),"aria-selected":M&&M.getFullYear()===$.getFullYear()&&M.getMonth()===t?"true":"false","data-current":t===$.getMonth(),tabindex:t===$.getMonth()?"0":"-1",disabled:!!p&&r<p||!!f&&n>f},L.format(n)))}o.append(e)}const l=document.createElement("div");l.className="nr-cal-footer",l.append(P("nr-cal-link",{"data-cal":"today",disabled:!g(v())},"Today"),P("nr-cal-link",{"data-cal":"clear"},"Clear")),o.append(l),o.hidden||s(o,d,!1),e&&o.querySelector('[tabindex="0"]:not(:disabled)')?.focus({preventScroll:!0})}function Y(){!o.hidden||a.disabled||a.readOnly||(r(e),F="days",$=M||(g(v())?v():p||f||v()),T(!1),i(o,d,!1),e.setAttribute("data-open",""),e.querySelector(".nr-datepicker-toggle")?.setAttribute("aria-expanded","true"),o.querySelector('[tabindex="0"]:not(:disabled)')?.focus({preventScroll:!0}),n.set(e,W))}function W(t){o.hidden||(l(o),e.removeAttribute("data-open"),e.querySelector(".nr-datepicker-toggle")?.setAttribute("aria-expanded","false"),n.delete(e),!0===t&&a.focus({preventScroll:!0}))}function H(e){M=e,C(),m(q),W(!0)}function N(e){p&&e<p&&(e=p),f&&e>f&&(e=f),$=e,T(!0)}o.addEventListener("click",e=>{const t=e.target.closest("button");t&&!t.disabled&&(t.dataset.date?H(A(t.dataset.date)):t.dataset.month?($=new Date($.getFullYear(),+t.dataset.month,Math.min($.getDate(),28)),F="days",T(!0)):"prev"===t.dataset.cal?($="days"===F?S($,-1):new Date($.getFullYear()-1,$.getMonth(),1),T(!1)):"next"===t.dataset.cal?($="days"===F?S($,1):new Date($.getFullYear()+1,$.getMonth(),1),T(!1)):"view"===t.dataset.cal?(F="days"===F?"months":"days",T(!0)):"today"===t.dataset.cal?H(v()):"clear"===t.dataset.cal&&(M=null,C(),m(q),W(!0)))}),o.addEventListener("keydown",e=>{if("Escape"===e.key)return e.preventDefault(),void W(!0);const t=e.target.dataset&&e.target.dataset.date,n=e.target.dataset&&null!=e.target.dataset.month;if(!t&&!n)return;const r={ArrowLeft:-1,ArrowRight:1,ArrowUp:t?-7:-3,ArrowDown:t?7:3}[e.key];if(null!=r)if(e.preventDefault(),t)N(k($,r));else{const e=Math.min(11,Math.max(0,$.getMonth()+r));$=new Date($.getFullYear(),e,1),T(!0)}else if(!t||"PageUp"!==e.key&&"PageDown"!==e.key){if(t&&("Home"===e.key||"End"===e.key)){e.preventDefault();const t=($.getDay()-u+7)%7;N(k($,"Home"===e.key?-t:6-t))}}else e.preventDefault(),N(S($,("PageUp"===e.key?-1:1)*(e.shiftKey?12:1)))}),e.addEventListener("focusout",t=>{o.hidden||!t.relatedTarget||e.contains(t.relatedTarget)||W()}),a.addEventListener("click",Y),a.addEventListener("keydown",e=>{"Enter"===e.key||" "===e.key||"ArrowDown"===e.key?(e.preventDefault(),Y()):"Backspace"!==e.key&&"Delete"!==e.key||!M||a.required||(e.preventDefault(),M=null,C(),m(q))}),d.addEventListener("click",e=>{e.target.closest(".nr-calendar")||(e.target.closest(".nr-datepicker-toggle")&&!o.hidden?W(!0):Y())}),a.form&&a.form.addEventListener("reset",()=>setTimeout(()=>{M=A(a.defaultValue),C()}))}(e):"avatar"===o?function(e){const t=e.querySelector(".nr-avatar-img");if(!t)return;const n=()=>t.hidden=!0;t.complete&&0===t.naturalWidth&&t.getAttribute("src")?n():t.addEventListener("error",n,{once:!0})}(e):"tabs"===o?function(e){const t=e.querySelector('[role="tablist"]');if(!t)return;const n=()=>[...t.querySelectorAll('[role="tab"]')];function r(t,r){if(t&&!t.disabled){for(const e of n()){const n=e===t;e.classList.toggle("nr-tab-selected",n),e.setAttribute("aria-selected",String(n)),e.tabIndex=n?0:-1;const r=e.getAttribute("aria-controls")&&document.getElementById(e.getAttribute("aria-controls"));r&&(r.hidden=!n)}r&&t.focus({preventScroll:!0}),f(e,"nr:change",{key:t.dataset.key})}}t.addEventListener("click",e=>{const n=e.target.closest('[role="tab"]');n&&t.contains(n)&&r(n)}),t.addEventListener("keydown",e=>{const t=n().filter(e=>!e.disabled),a=t.indexOf(e.target.closest('[role="tab"]'));if(a<0)return;const o={ArrowRight:a+1,ArrowLeft:a-1,Home:0,End:t.length-1}[e.key];void 0!==o&&(e.preventDefault(),r(t[(o+t.length)%t.length],!0))})}(e):"modal"===o?h(e):"dropdown"===o||"popover"===o?L(e):"context"===o?function(e){L(e);const t=e.querySelector("[popover]");if(!t||!a)return;const n=()=>t.matches(":popover-open")&&t.hidePopover();document.addEventListener("pointerdown",e=>!t.contains(e.target)&&n(),!0),addEventListener("scroll",n,{capture:!0,passive:!0}),t.addEventListener("keydown",e=>{"Escape"===e.key&&(e.preventDefault(),n())}),e.addEventListener("contextmenu",e=>{if(t.contains(e.target))return;e.preventDefault(),n(),t.showPopover();const r=Math.min(e.clientX,innerWidth-t.offsetWidth-8),a=Math.min(e.clientY,innerHeight-t.offsetHeight-8);t.style.left=`${Math.max(8,r)}px`,t.style.top=`${Math.max(8,a)}px`})}(e):"command"===o?function(e){h(e);const t=e.querySelector(".nr-command-input"),n=e.querySelector(".nr-command-empty");if(!t)return;const r=()=>[...e.querySelectorAll(".nr-command-item:not([hidden])")];let a=null;function o(e){a&&(a.removeAttribute("data-active"),a.setAttribute("aria-selected","false")),a=e||null,a?(a.setAttribute("data-active",""),a.setAttribute("aria-selected","true"),t.setAttribute("aria-activedescendant",a.id),a.scrollIntoView({block:"nearest"})):t.removeAttribute("aria-activedescendant")}function s(){const a=t.value.toLowerCase().split(/\s+/).filter(Boolean);for(const t of e.querySelectorAll(".nr-command-item")){const e=`${t.textContent} ${t.dataset.keywords||""}`.toLowerCase();t.hidden=!a.every(t=>e.includes(t))}for(const t of e.querySelectorAll(".nr-command-group"))t.hidden=!t.querySelector(".nr-command-item:not([hidden])");const s=r();n&&(n.hidden=s.length>0),o(s[0])}t.addEventListener("input",s),t.addEventListener("keydown",e=>{const t=r(),n={ArrowDown:1,ArrowUp:-1}[e.key];n&&t.length?(e.preventDefault(),o(t[(t.indexOf(a)+n+t.length)%t.length])):"Enter"===e.key&&a&&(e.preventDefault(),a.click())}),e.addEventListener("pointermove",e=>{const t=e.target.closest(".nr-command-item");t&&t!==a&&o(t)}),e.addEventListener("click",t=>{t.target.closest(".nr-command-item")&&e.close()}),e.addEventListener("nr:open",()=>{t.value="",s(),t.focus()});const i=e.dataset.hotkey;i&&document.addEventListener("keydown",t=>{(t.metaKey||t.ctrlKey)&&!t.altKey&&t.key.toLowerCase()===i&&e.isConnected&&(t.preventDefault(),e.open?e.close():g(e))})}(e):"toaster"===o?M(e):"combobox"===o?function(e){const t=e.querySelector(".nr-field-input"),a=e.querySelector(".nr-combobox-list"),o=e.querySelector(".nr-field-wrap");if(!t||!a||!t.list)return;const d=[...t.list.options].map(e=>e.value);t.removeAttribute("list"),t.setAttribute("role","combobox"),t.setAttribute("aria-autocomplete","list"),t.setAttribute("aria-expanded","false"),t.setAttribute("aria-controls",a.id);let c=null;function u(e){c&&c.removeAttribute("data-active"),c=e||null,c?(c.setAttribute("data-active",""),t.setAttribute("aria-activedescendant",c.id),c.scrollIntoView({block:"nearest"})):t.removeAttribute("aria-activedescendant")}function p(){if(!t.disabled&&!t.readOnly){if(function(){const e=t.value.trim().toLowerCase(),n=d.filter(t=>t.toLowerCase().includes(e));if(a.textContent="",n.forEach((e,n)=>{const r=document.createElement("li");r.id=`${a.id}-${n}`,r.setAttribute("role","option"),r.setAttribute("aria-selected",String(e===t.value)),r.textContent=e,a.append(r)}),!n.length){const e=document.createElement("li");e.setAttribute("role","presentation"),e.textContent="No matches",a.append(e)}u(null)}(),!a.hidden)return s(a,o,!0);r(e),i(a,o,!0),t.setAttribute("aria-expanded","true"),n.set(e,f)}}function f(){a.hidden||(l(a),t.setAttribute("aria-expanded","false"),u(null),n.delete(e))}function v(e){t.value=e.textContent,m(t),f()}t.addEventListener("input",p),t.addEventListener("click",p),t.addEventListener("keydown",e=>{const t={ArrowDown:1,ArrowUp:-1}[e.key];if(t){e.preventDefault(),a.hidden&&p();const n=[...a.querySelectorAll('[role="option"]')];n.length&&u(n[(n.indexOf(c)+t+n.length)%n.length])}else"Enter"===e.key&&c?(e.preventDefault(),v(c)):"Escape"!==e.key||a.hidden||(e.preventDefault(),f())}),a.addEventListener("pointerdown",e=>e.preventDefault()),a.addEventListener("click",e=>{const t=e.target.closest('[role="option"]');t&&v(t)}),t.addEventListener("blur",f)}(e):"tags"===o?function(e){const t=e.querySelector(".nr-field-input");if(!t)return;const n=document.createElement("input");n.type="hidden",n.name=t.name,t.removeAttribute("name");const r=document.createElement("div");r.className="nr-tags-row",t.before(r),r.append(t,n);const a=+e.dataset.max||1/0,o=e.hasAttribute("data-float"),s=t.required,i=t.value;let l=[];function d(){n.value=l.join(", ");for(const e of r.querySelectorAll(".nr-tags-chip"))e.remove();l.forEach((e,n)=>{const r=document.createElement("span");r.className="nr-tags-chip",r.textContent=e;const a=document.createElement("button");a.type="button",a.textContent="×",a.dataset.index=n,a.setAttribute("aria-label",`Remove ${e}`),r.append(a),t.before(r)}),e.toggleAttribute("data-float",o&&!l.length),t.required=s&&!l.length}function c(n,r){const o=l.length;for(const e of n.split(",")){const t=e.trim();t&&!l.includes(t)&&l.length<a&&l.push(t)}t.value="",d(),r||l.length===o||f(e,"nr:change",{values:[...l]})}c(i,!0),t.addEventListener("keydown",n=>{"Enter"!==n.key&&","!==n.key||!t.value.trim()?","===n.key?n.preventDefault():"Backspace"===n.key&&!t.value&&l.length&&(l.pop(),d(),f(e,"nr:change",{values:[...l]})):(n.preventDefault(),c(t.value))}),t.addEventListener("change",()=>c(t.value)),t.addEventListener("blur",()=>c(t.value)),r.addEventListener("click",n=>{const r=n.target.closest("button[data-index]");r&&(l.splice(+r.dataset.index,1),d(),f(e,"nr:change",{values:[...l]}),t.focus())}),t.form&&t.form.addEventListener("reset",()=>setTimeout(()=>{l=[],c(i,!0)}))}(e):"upload"===o?function(e){const t=e.querySelector(".nr-dropzone"),n=e.querySelector(".nr-dropzone-input"),r=e.querySelector(".nr-dropzone-files");if(t&&n){for(const e of["dragenter","dragover"])t.addEventListener(e,()=>t.setAttribute("data-drag",""));for(const e of["dragleave","drop"])t.addEventListener(e,()=>t.removeAttribute("data-drag"));n.addEventListener("change",()=>{if(r){r.textContent="";for(const e of n.files){const t=document.createElement("li");t.textContent=`${e.name} (${e.size<1e6?`${Math.ceil(e.size/1e3)} KB`:`${(e.size/1e6).toFixed(1)} MB`})`,r.append(t)}r.hidden=!n.files.length}})}}(e):"carousel"===o?function(e){const t=e.querySelector(".nr-carousel-viewport"),n=e.querySelector(".nr-carousel-prev"),r=e.querySelector(".nr-carousel-next");if(!t||!n||!r)return;const a="rtl"===getComputedStyle(t).direction,o=()=>{const e=Math.abs(t.scrollLeft);n.setAttribute("aria-disabled",String(e<2)),r.setAttribute("aria-disabled",String(e>t.scrollWidth-t.clientWidth-2))};e.addEventListener("click",e=>{const n=e.target.closest("[data-nr-slide]");if(!n||"true"===n.getAttribute("aria-disabled"))return;const r=t.querySelector(".nr-carousel-item"),o=parseFloat(getComputedStyle(t).columnGap)||0,s=r?Math.max(1,Math.round(t.clientWidth/(r.offsetWidth+o))):1,i=r?(r.offsetWidth+o)*s:t.clientWidth;t.scrollBy({left:+n.dataset.nrSlide*i*(a?-1:1)})}),t.addEventListener("scroll",o,{passive:!0}),new ResizeObserver(o).observe(t),o()}(e):"banner"===o?function(e){const t=`nr-banner:${e.dataset.nrPersist}`;try{if(localStorage.getItem(t))return e.remove()}catch{}e.addEventListener("change",()=>{try{localStorage.setItem(t,"1")}catch{}f(e,"nr:dismiss",null)})}(e):"checkall"===o&&function(e){const t=e.closest("table");if(!t)return;const n=()=>[...t.querySelectorAll("tbody .nr-table-checkbox")],r=()=>{const t=n(),r=t.filter(e=>e.checked).length;e.checked=r>0&&r===t.length,e.indeterminate=r>0&&r<t.length};t.addEventListener("change",a=>{if(a.target===e)for(const t of n())t.checked=e.checked;else if(!a.target.matches(".nr-table-checkbox"))return;r(),f(t,"nr:change",{selected:n().filter(e=>e.checked).map(e=>e.value)})}),r()}(e)}function p(e){e.matches&&e.matches("[data-nr-ui]")&&u(e);for(const t of e.querySelectorAll("[data-nr-ui]"))u(t)}const f=(e,t,n)=>e.dispatchEvent(new CustomEvent(t,{bubbles:!0,detail:n})),m=e=>{e.dispatchEvent(new Event("input",{bubbles:!0})),e.dispatchEvent(new Event("change",{bubbles:!0}))};function v(e){const t=document.querySelector(".nr-shell");if(t){t.toggleAttribute("data-open",e);for(const t of document.querySelectorAll(".nr-navbar-menu"))t.setAttribute("aria-expanded",String(e))}}function g(e){if(!e||e.open)return;e.showModal();const t=document.querySelector('[data-nr-ui="toaster"]');t&&M(t),f(e,"nr:open",null)}function h(e){e.open&&(e.close(),g(e)),e.addEventListener("click",t=>{t.target===e&&e.hasAttribute("data-dismissable")&&e.close()}),e.addEventListener("close",()=>f(e,"nr:close",null))}document.addEventListener("click",async t=>{const n=t.target.closest&&t.target.closest("[data-nr-press]");if(!n||n.matches(':disabled, [aria-disabled="true"], [data-loading]'))return;const r=n.dataset.nrPress,a=n.dataset.nrPressTarget||"";if("navigate"===r)t.preventDefault(),e().navigate?e().navigate(a):location.href=a;else if("modal"===r)t.preventDefault(),g(document.getElementById(a));else if("close-modal"===r){t.preventDefault();const e=n.closest("dialog");e&&e.close()}else if("toast"===r)C(a);else if("emit"===r)f(n,"nr:"+a,null);else if("action"===r){t.preventDefault();let r=null;try{r=JSON.parse(n.dataset.nrPressInput||"null")}catch{}n.setAttribute("data-loading",""),n.setAttribute("aria-busy","true");try{const t=await e().action(a,r);f(n,"nr:success",t),"false"!==n.dataset.nrPressRefresh&&e().refresh&&e().refresh()}catch(e){f(n,"nr:error",e),n.hasAttribute("data-nr-quiet")||console.error("[next-rust] action failed:",e.message)}finally{n.removeAttribute("data-loading"),n.removeAttribute("aria-busy")}}}),document.addEventListener("keydown",e=>{const t=e.target;("Enter"===e.key||" "===e.key)&&t.matches&&t.matches('[data-nr-press][role="button"]')&&(e.preventDefault(),t.click())}),document.addEventListener("click",e=>{const t=e.target.closest&&e.target.closest("[data-nr-field-action]");if(t){const e=t.closest(".nr-field")?.querySelector("input.nr-field-input, textarea.nr-field-input"),n=t.dataset.nrFieldAction;if("toggle-password"===n&&e){const n="password"===e.type;e.type=n?"text":"password",t.setAttribute("aria-pressed",String(n)),t.setAttribute("aria-label",n?"Hide password":"Show password"),e.focus({preventScroll:!0});const r=e.value.length;e.setSelectionRange(r,r)}else if("clear"===n&&e)e.value="",m(e),e.focus();else if(("step-up"===n||"step-down"===n)&&e){try{"step-up"===n?e.stepUp():e.stepDown()}catch{}m(e)}return}const n=e.target.closest&&e.target.closest(".nr-input .nr-field-wrap, .nr-textarea .nr-field-wrap");n&&!e.target.closest("input, textarea, button, a, select")&&n.querySelector(".nr-field-input")?.focus()}),document.addEventListener("click",e=>{e.target.closest&&e.target.closest("[data-nr-shell-toggle]")?v(!document.querySelector(".nr-shell[data-open]")):e.target.closest&&e.target.closest(".nr-shell-sidebar a[href]")&&v(!1)}),window.addEventListener("nr:navigate",()=>v(!1)),document.addEventListener("keydown",e=>{"Escape"===e.key&&document.querySelector(".nr-shell[data-open]")&&v(!1)}),document.addEventListener("click",e=>{const t=e.target.closest&&e.target.closest("[data-nr-dismiss]"),n=t&&t.closest('[data-nr-ui="alert"], .nr-toast');n&&(f(n,"nr:dismiss",null),n.remove())});const y=(e,t=2)=>String(e).padStart(t,"0"),b=e=>`${y(e.getFullYear(),4)}-${y(e.getMonth()+1)}-${y(e.getDate())}`,w=(e,t)=>!!e&&!!t&&b(e)===b(t),k=(e,t)=>new Date(e.getFullYear(),e.getMonth(),e.getDate()+t),E=e=>new Date(e.getFullYear(),e.getMonth(),1);function A(e){const t=/^(\d{4})-(\d{2})-(\d{2})$/.exec(e||"");if(!t)return null;const n=new Date(+t[1],+t[2]-1,+t[3]);return n.getMonth()===+t[2]-1?n:null}function S(e,t){const n=new Date(e.getFullYear(),e.getMonth()+t,1),r=new Date(n.getFullYear(),n.getMonth()+1,0).getDate();return new Date(n.getFullYear(),n.getMonth(),Math.min(e.getDate(),r))}const x="undefined"!=typeof CSS&&CSS.supports("position-area: block-end");function L(e){const t=e.querySelector("[popover]"),n=e.querySelector("[popovertarget]");if(!t||!a)return;const r="menu"===t.getAttribute("role"),i=()=>[...t.querySelectorAll('[role="menuitem"]')].filter(e=>!e.matches(':disabled, [aria-disabled="true"]'));t.addEventListener("beforetoggle",e=>{"open"===e.newState&&!x&&n&&requestAnimationFrame(()=>{s(t,n,!1),o.set(t,[n,!1])})}),t.addEventListener("toggle",a=>{const s="open"===a.newState;n&&n.setAttribute("aria-expanded",String(s)),s&&r&&(i()[0]||t).focus({preventScroll:!0}),s||o.delete(t),f(e,s?"nr:open":"nr:close",null)}),r&&(t.addEventListener("keydown",e=>{const n=i(),r=n.indexOf(document.activeElement);let a={ArrowDown:r+1,ArrowUp:r<0?-1:r-1,Home:0,End:n.length-1}[e.key];if(void 0===a&&1===e.key.length&&/\S/.test(e.key)&&!e.ctrlKey&&!e.metaKey){const t=e.key.toLowerCase(),o=[...n.slice(r+1),...n.slice(0,r+1)];if(a=n.indexOf(o.find(e=>e.textContent.trim().toLowerCase().startsWith(t))),a<0)return}"Tab"===e.key&&t.hidePopover(),void 0!==a&&n.length&&(e.preventDefault(),n[(a+n.length)%n.length].focus({preventScroll:!0}))}),t.addEventListener("click",e=>{e.target.closest('[role="menuitem"]')&&t.hidePopover()}),n&&n.addEventListener("keydown",e=>{"ArrowDown"!==e.key&&"ArrowUp"!==e.key||t.matches(":popover-open")||(e.preventDefault(),n.click())}))}const D={default:'<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',success:'<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',warning:'<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',danger:'<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>'},q=e=>`<svg class="nr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${e}</svg>`;function M(e){if(a&&e.children.length)try{if(e.matches(":popover-open")){if(!document.querySelector("dialog:modal"))return;e.hidePopover()}e.popover="manual",e.showPopover()}catch{}}function C(e){const t=document.querySelector('[data-nr-ui="toaster"]');if(!t)return null;const n="string"==typeof e?{message:e}:e||{},r=n.color||"default",a=document.createElement("div");a.className=0===n.duration?"nr-toast nr-toast-sticky":"nr-toast",a.dataset.color=r,a.setAttribute("role","danger"===r||"warning"===r?"alert":"status"),n.duration>0&&a.style.setProperty("--nr-toast-d",`${n.duration}ms`),a.innerHTML=`<span class="nr-toast-icon">${q(D[r]||D.default)}</span><div class="nr-toast-body"></div><button type="button" class="nr-toast-close" aria-label="Dismiss" data-nr-dismiss>${q('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>')}</button>`;const o=a.children[1];for(const[e,t,r]of[["strong","nr-toast-title",n.title],["div","nr-toast-text",n.message]]){if(null==r||""===r)continue;const n=document.createElement(e);n.className=t,n.textContent=r,o.append(n)}return t.append(a),M(t),a}(window.nextRust=window.nextRust||{}).toast=C,addEventListener("nr:toast",e=>C(e.detail)),document.addEventListener("animationend",e=>{const t=e.target;t.matches&&t.matches(".nr-toast")&&"hidden"===getComputedStyle(t).visibility&&t.remove()}),document.addEventListener("input",e=>{const t=e.target;if(!t.matches||!t.matches(".nr-slider-input, .nr-color-input"))return;const n=t.closest("[data-nr-ui]");if(!n)return;if("range"===t.type){const e=+t.min||0,r=""===t.max?100:+t.max;n.style.setProperty("--nr-v",(t.value-e)/(r-e||1)*100+"%")}const r=n.querySelector("[data-nr-echo]");r&&(r.textContent=t.value+(r.dataset.unit||""))}),document.addEventListener("click",async e=>{const t=e.target.closest&&e.target.closest("[data-nr-copy]"),n=t&&t.closest(".nr-codeblock")?.querySelector("pre");if(n){try{await navigator.clipboard.writeText(n.textContent)}catch{return}t.setAttribute("data-copied",""),t.setAttribute("aria-label","Copied"),f(t,"nr:copy",null),setTimeout(()=>{t.removeAttribute("data-copied"),t.setAttribute("aria-label","Copy code")},1500)}}),p(document),new MutationObserver(e=>{for(const t of e)for(const e of t.addedNodes)1===e.nodeType&&p(e)}).observe(t,{childList:!0,subtree:!0});"####;

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
    const UI_JS_SOURCE_HASH: &str = "69d89da5e77b266d";

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
