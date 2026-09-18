// The component stylesheet, as Rust. `build.rs` minifies it at compile time
// (it `include!`s this file), and the result is `crate::UI_CSS`.
//
// No inner attributes or module docs here: build scripts include this file.

/// Source of the component stylesheet (see [`crate::UI_CSS`]).
pub const UI_CSS_SOURCE: &str = r####"/*
 * Next Rust UI components.
 *
 * Rules live in the `components` layer: anything in the `utilities` layer
 * (Tailwind) or outside layers (your own CSS) wins over them.
 *
 * Pages receive only the rules of the components they render: every
 * selector in `@layer components` starts with a class that appears in the
 * component's server-rendered HTML. Keep it that way when editing
 * (`tests/components.rs` checks it).
 */
@layer theme, base, components, utilities;

@layer theme {
  :root {
    --nr-font: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    --nr-bg: #fff;
    --nr-fg: #11181c;
    --nr-content: #fff;
    --nr-content-2: #f4f4f5;
    --nr-content-3: #e4e4e7;
    --nr-content-4: #d4d4d8;
    --nr-muted: #71717a;
    --nr-subtle: #a1a1aa;
    --nr-divider: rgba(17, 17, 17, 0.12);
    --nr-focus: #006fee;

    --nr-default: #d4d4d8;
    --nr-default-fg: #11181c;
    --nr-default-text: #3f3f46;
    --nr-primary: #006fee;
    --nr-primary-fg: #fff;
    --nr-primary-text: #005bc4;
    --nr-secondary: #7828c8;
    --nr-secondary-fg: #fff;
    --nr-secondary-text: #6020a0;
    --nr-success: #17c964;
    --nr-success-fg: #000;
    --nr-success-text: #12a150;
    --nr-warning: #f5a524;
    --nr-warning-fg: #000;
    --nr-warning-text: #c4841d;
    --nr-danger: #f31260;
    --nr-danger-fg: #fff;
    --nr-danger-text: #c20e4d;

    --nr-radius-sm: 8px;
    --nr-radius-md: 12px;
    --nr-radius-lg: 14px;
    --nr-shadow-sm: 0 0 5px 0 rgba(0, 0, 0, 0.02), 0 2px 10px 0 rgba(0, 0, 0, 0.06), 0 0 1px 0 rgba(0, 0, 0, 0.3);
    --nr-shadow-md: 0 0 15px 0 rgba(0, 0, 0, 0.03), 0 2px 30px 0 rgba(0, 0, 0, 0.08), 0 0 1px 0 rgba(0, 0, 0, 0.3);
    --nr-shadow-lg: 0 0 30px 0 rgba(0, 0, 0, 0.04), 0 30px 60px 0 rgba(0, 0, 0, 0.12), 0 0 1px 0 rgba(0, 0, 0, 0.3);
    --nr-ease: cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  @media (prefers-color-scheme: dark) {
    :root:not(.light):not([data-theme="light"]) {
      color-scheme: dark;
      --nr-bg: #000;
      --nr-fg: #ecedee;
      --nr-content: #18181b;
      --nr-content-2: #27272a;
      --nr-content-3: #3f3f46;
      --nr-content-4: #52525b;
      --nr-muted: #a1a1aa;
      --nr-subtle: #71717a;
      --nr-divider: rgba(255, 255, 255, 0.14);
      --nr-default: #3f3f46;
      --nr-default-fg: #fff;
      --nr-default-text: #d4d4d8;
      --nr-primary-text: #338ef7;
      --nr-secondary: #9353d3;
      --nr-secondary-text: #ae7ede;
      --nr-success-text: #45d483;
      --nr-warning-text: #f7b750;
      --nr-danger-text: #f54180;
      --nr-shadow-sm: 0 0 5px 0 rgba(0, 0, 0, 0.05), 0 2px 10px 0 rgba(0, 0, 0, 0.2), inset 0 0 1px 0 rgba(255, 255, 255, 0.15);
      --nr-shadow-md: 0 0 15px 0 rgba(0, 0, 0, 0.06), 0 2px 30px 0 rgba(0, 0, 0, 0.22), inset 0 0 1px 0 rgba(255, 255, 255, 0.15);
      --nr-shadow-lg: 0 0 30px 0 rgba(0, 0, 0, 0.07), 0 30px 60px 0 rgba(0, 0, 0, 0.26), inset 0 0 1px 0 rgba(255, 255, 255, 0.15);
    }
  }

  .dark,
  [data-theme="dark"] {
    color-scheme: dark;
    --nr-bg: #000;
    --nr-fg: #ecedee;
    --nr-content: #18181b;
    --nr-content-2: #27272a;
    --nr-content-3: #3f3f46;
    --nr-content-4: #52525b;
    --nr-muted: #a1a1aa;
    --nr-subtle: #71717a;
    --nr-divider: rgba(255, 255, 255, 0.14);
    --nr-default: #3f3f46;
    --nr-default-fg: #fff;
    --nr-default-text: #d4d4d8;
    --nr-primary-text: #338ef7;
    --nr-secondary: #9353d3;
    --nr-secondary-text: #ae7ede;
    --nr-success-text: #45d483;
    --nr-warning-text: #f7b750;
    --nr-danger-text: #f54180;
    --nr-shadow-sm: 0 0 5px 0 rgba(0, 0, 0, 0.05), 0 2px 10px 0 rgba(0, 0, 0, 0.2), inset 0 0 1px 0 rgba(255, 255, 255, 0.15);
    --nr-shadow-md: 0 0 15px 0 rgba(0, 0, 0, 0.06), 0 2px 30px 0 rgba(0, 0, 0, 0.22), inset 0 0 1px 0 rgba(255, 255, 255, 0.15);
    --nr-shadow-lg: 0 0 30px 0 rgba(0, 0, 0, 0.07), 0 30px 60px 0 rgba(0, 0, 0, 0.26), inset 0 0 1px 0 rgba(255, 255, 255, 0.15);
  }
}

@layer components {
  /* ---- Colors: every component reads --c, --c-fg (text on --c), --c-ink
     (the color as text) and --c-text (text on a soft tint). ------------- */
  .nr-c-default { --c: var(--nr-default); --c-fg: var(--nr-default-fg); --c-ink: var(--nr-fg); --c-text: var(--nr-default-text); }
  .nr-c-primary { --c: var(--nr-primary); --c-fg: var(--nr-primary-fg); --c-ink: var(--nr-primary); --c-text: var(--nr-primary-text); }
  .nr-c-secondary { --c: var(--nr-secondary); --c-fg: var(--nr-secondary-fg); --c-ink: var(--nr-secondary); --c-text: var(--nr-secondary-text); }
  .nr-c-success { --c: var(--nr-success); --c-fg: var(--nr-success-fg); --c-ink: var(--nr-success-text); --c-text: var(--nr-success-text); }
  .nr-c-warning { --c: var(--nr-warning); --c-fg: var(--nr-warning-fg); --c-ink: var(--nr-warning-text); --c-text: var(--nr-warning-text); }
  .nr-c-danger { --c: var(--nr-danger); --c-fg: var(--nr-danger-fg); --c-ink: var(--nr-danger); --c-text: var(--nr-danger-text); }

  .nr-icon { display: block; flex-shrink: 0; width: 1.25rem; height: 1.25rem; }

  /* ---- Spinner --------------------------------------------------------- */
  .nr-spinner { display: inline-flex; align-items: center; gap: 0.625rem; vertical-align: middle; }
  .nr-spinner[role="status"] { color: var(--c); }
  .nr-spinner-sm { --nr-sp: 1.25rem; }
  .nr-spinner-md { --nr-sp: 2rem; }
  .nr-spinner-lg { --nr-sp: 2.75rem; }
  .nr-spinner-ring {
    display: block;
    box-sizing: border-box;
    width: var(--nr-sp, 1.5rem);
    height: var(--nr-sp, 1.5rem);
    border: 3px solid color-mix(in oklab, currentColor 18%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: nr-spin 0.75s linear infinite;
  }
  .nr-spinner-label { font-size: 0.875rem; color: var(--nr-muted); }

  /* ---- Button ---------------------------------------------------------- */
  .nr-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    box-sizing: border-box;
    min-width: var(--nr-btn-min, 5rem);
    height: var(--nr-btn-h, 2.5rem);
    margin: 0;
    padding: 0 var(--nr-btn-px, 1rem);
    border: 2px solid transparent;
    border-radius: var(--r, var(--nr-radius-md));
    background: transparent;
    color: inherit;
    font-family: inherit;
    font-size: var(--nr-btn-fs, 0.875rem);
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    -webkit-tap-highlight-color: transparent;
    outline: none;
    overflow: hidden;
    isolation: isolate;
    transition: transform 0.15s var(--nr-ease), background-color 0.2s, color 0.2s, border-color 0.2s, box-shadow 0.2s, opacity 0.2s;
  }
  .nr-btn:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-btn:active:not(:disabled, [aria-disabled="true"]) { transform: scale(0.97); }
  .nr-btn:disabled, .nr-btn[aria-disabled="true"] { opacity: 0.5; cursor: not-allowed; }
  .nr-btn[aria-disabled="true"] { pointer-events: none; }
  .nr-btn[data-loading] { opacity: 1; cursor: progress; }
  .nr-btn .nr-icon { width: 1.15em; height: 1.15em; }
  .nr-btn-sm { --nr-btn-h: 2rem; --nr-btn-px: 0.75rem; --nr-btn-fs: 0.8125rem; --nr-btn-min: 4rem; --r: var(--nr-radius-sm); gap: 0.375rem; }
  .nr-btn-md { --nr-btn-h: 2.5rem; }
  .nr-btn-lg { --nr-btn-h: 3rem; --nr-btn-px: 1.5rem; --nr-btn-fs: 1rem; --nr-btn-min: 6rem; --r: var(--nr-radius-lg); gap: 0.75rem; }
  .nr-btn-full { width: 100%; }
  .nr-btn-icon { width: var(--nr-btn-h, 2.5rem); min-width: var(--nr-btn-h, 2.5rem); padding: 0; }

  .nr-btn-solid { background: var(--c); color: var(--c-fg); }
  .nr-btn-shadow { background: var(--c); color: var(--c-fg); box-shadow: 0 8px 22px -6px color-mix(in oklab, var(--c) 60%, transparent); }
  .nr-btn-bordered { border-color: var(--c); color: var(--c-ink); }
  .nr-btn-ghost { border-color: var(--c); color: var(--c-ink); }
  .nr-btn-light { color: var(--c-ink); }
  .nr-btn-flat { background: color-mix(in oklab, var(--c) 20%, transparent); color: var(--c-text); }
  .nr-btn-faded { background: var(--nr-content-2); border-color: var(--nr-content-3); color: var(--c-ink); }
  @media (hover: hover) {
    .nr-btn-solid:hover, .nr-btn-shadow:hover { opacity: 0.88; }
    .nr-btn-bordered:hover { background: color-mix(in oklab, var(--c) 10%, transparent); }
    .nr-btn-ghost:hover { background: var(--c); color: var(--c-fg); }
    .nr-btn-light:hover { background: color-mix(in oklab, var(--c) 16%, transparent); }
    .nr-btn-flat:hover { background: color-mix(in oklab, var(--c) 28%, transparent); }
    .nr-btn-faded:hover { background: var(--nr-content-3); }
  }

  .nr-btn-spinner { --nr-sp: 1.1em; }
  .nr-btn-spinner .nr-spinner-ring { border-width: 2px; }
  .nr-btn-pending { display: none; }
  .nr-btn[type="submit"]:where(form[aria-busy="true"] *) { pointer-events: none; cursor: progress; }
  .nr-btn[type="submit"]:where(form[aria-busy="true"] *) .nr-btn-pending, .nr-btn[data-loading] .nr-btn-pending { display: inline-flex; }

  .nr-btn-group { display: inline-flex; align-items: center; isolation: isolate; }
  .nr-btn-group > .nr-btn { min-width: 0; }
  .nr-btn-group > .nr-btn:not(:first-child) { border-start-start-radius: 0; border-end-start-radius: 0; }
  .nr-btn-group > .nr-btn:not(:last-child) { border-start-end-radius: 0; border-end-end-radius: 0; }
  .nr-btn-group > .nr-btn-bordered:not(:first-child), .nr-btn-group > .nr-btn-ghost:not(:first-child), .nr-btn-group > .nr-btn-faded:not(:first-child) { margin-inline-start: -2px; }
  .nr-btn-group > .nr-btn:focus-visible { z-index: 1; }
  .nr-btn-group.nr-btn-full { display: flex; }
  .nr-btn-group.nr-btn-full > .nr-btn { flex: 1; }

  /* ---- Fields: input, textarea, select, date picker --------------------- */
  .nr-field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    min-width: 0;
    font-size: 0.875rem;
    text-align: start;
  }
  .nr-field-full { width: 100%; }
  .nr-field-sm { --nr-field-h: 2rem; --nr-field-fs: 0.8125rem; --r: var(--nr-radius-sm); }
  .nr-field-md { --nr-field-h: 2.5rem; }
  .nr-field-lg { --nr-field-h: 3rem; --nr-field-fs: 1rem; --r: var(--nr-radius-lg); }
  .nr-label-inside.nr-field-sm { --nr-field-h: 3rem; }
  .nr-label-inside.nr-field-md { --nr-field-h: 3.5rem; }
  .nr-label-inside.nr-field-lg { --nr-field-h: 4rem; }
  .nr-label-left { flex-direction: row; align-items: flex-start; gap: 0.75rem; }
  .nr-label-left > .nr-field-label { display: flex; align-items: center; flex-shrink: 0; min-height: var(--nr-field-h, 2.5rem); }
  .nr-label-left > .nr-field-main { flex: 1; }
  .nr-field-main { position: relative; display: flex; flex-direction: column; gap: 0.375rem; min-width: 0; }

  .nr-field-label { display: block; color: var(--nr-fg); font-size: 0.875rem; font-weight: 500; line-height: 1.25; }
  .nr-field-required { margin-inline-start: 0.125rem; color: var(--nr-danger); }
  .nr-label-inside .nr-field-label {
    color: var(--nr-muted);
    font-size: 0.75rem;
    line-height: 1rem;
    cursor: inherit;
    transform-origin: left center;
    transition: transform 0.18s var(--nr-ease), color 0.15s;
    pointer-events: none;
  }
  .nr-label-inside .nr-field-input { padding-top: 0.125rem; }
  .nr-label-inside[data-float]:has(.nr-field-input:placeholder-shown:not(:focus)) .nr-field-label,
  .nr-label-inside[data-float]:has(.nr-select-trigger):not([data-filled]):not([data-open]) .nr-field-label {
    transform: translateY(0.625rem) scale(1.1667);
  }

  .nr-field-wrap {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    box-sizing: border-box;
    width: 100%;
    min-height: var(--nr-field-h, 2.5rem);
    padding: 0 0.75rem;
    border: 2px solid transparent;
    border-radius: var(--r, var(--nr-radius-md));
    cursor: text;
    transition: background-color 0.15s, border-color 0.15s, box-shadow 0.15s;
  }
  .nr-field-inner { position: relative; display: flex; flex: 1; flex-direction: column; justify-content: center; align-self: stretch; min-width: 0; }
  .nr-field-input {
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--nr-fg);
    font: inherit;
    font-size: var(--nr-field-fs, 0.875rem);
    line-height: 1.25rem;
    box-shadow: none;
    appearance: none;
    -webkit-appearance: none;
  }
  .nr-field-input::placeholder { color: var(--nr-subtle); opacity: 1; }
  .nr-field-input:-webkit-autofill { -webkit-text-fill-color: var(--nr-fg); transition: background-color 600000s 0s; }
  .nr-field-start, .nr-field-end { display: flex; flex-shrink: 0; align-items: center; gap: 0.25rem; color: var(--nr-muted); }
  .nr-field-start .nr-icon, .nr-field-end .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-field-action {
    display: inline-grid;
    place-items: center;
    width: 1.75rem;
    height: 1.75rem;
    margin: 0 -0.25rem 0 0;
    padding: 0;
    border: 0;
    border-radius: var(--nr-radius-sm);
    background: transparent;
    color: var(--nr-muted);
    cursor: pointer;
    transition: color 0.15s, background-color 0.15s;
  }
  .nr-field-action:hover { color: var(--nr-fg); background: var(--nr-content-3); }
  .nr-field-action:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 1px; }
  .nr-field-action:not(:where(.nr-ui-js *)) { display: none; }
  .nr-field-description { margin: 0; color: var(--nr-muted); font-size: 0.75rem; line-height: 1.25; }
  .nr-field-error { margin: 0; color: var(--nr-danger); font-size: 0.75rem; line-height: 1.25; }
  .nr-field-error:empty { display: none; }

  .nr-field-flat .nr-field-wrap { background: var(--nr-content-2); }
  .nr-field-faded .nr-field-wrap { background: var(--nr-content-2); border-color: var(--nr-content-3); }
  .nr-field-bordered .nr-field-wrap { border-color: var(--nr-content-3); }
  .nr-field-underlined .nr-field-wrap { padding-inline: 0.25rem; border-width: 0 0 2px; border-color: var(--nr-content-3); border-radius: 0; }
  .nr-field-underlined .nr-field-wrap::after {
    content: "";
    position: absolute;
    right: 50%;
    bottom: -2px;
    left: 50%;
    height: 2px;
    background: var(--c);
    transition: left 0.25s var(--nr-ease), right 0.25s var(--nr-ease);
  }
  @media (hover: hover) {
    .nr-field-flat .nr-field-wrap:hover { background: var(--nr-content-3); }
    .nr-field-faded .nr-field-wrap:hover, .nr-field-bordered .nr-field-wrap:hover { border-color: var(--nr-content-4); }
  }
  .nr-field-flat .nr-field-wrap:focus-within { background: var(--nr-content-2); box-shadow: 0 0 0 2px color-mix(in oklab, var(--c) 45%, transparent); }
  .nr-field-faded .nr-field-wrap:focus-within, .nr-field-bordered .nr-field-wrap:focus-within { border-color: var(--c); }
  .nr-field-underlined .nr-field-wrap:focus-within::after { right: 0; left: 0; }
  .nr-field:focus-within .nr-field-label { color: var(--c-ink); }

  .nr-field[data-invalid], .nr-field:has(> .nr-field-main > .nr-field-error:not(:empty)) {
    --c: var(--nr-danger);
    --c-ink: var(--nr-danger);
  }
  .nr-field[data-invalid] .nr-field-label, .nr-field:has(> .nr-field-main > .nr-field-error:not(:empty)) .nr-field-label { color: var(--nr-danger); }
  .nr-field-flat[data-invalid] .nr-field-wrap, .nr-field-flat:has(> .nr-field-main > .nr-field-error:not(:empty)) .nr-field-wrap {
    background: color-mix(in oklab, var(--nr-danger) 12%, transparent);
  }
  .nr-field-bordered[data-invalid] .nr-field-wrap, .nr-field-faded[data-invalid] .nr-field-wrap,
  .nr-field-underlined[data-invalid] .nr-field-wrap { border-color: var(--nr-danger); }
  .nr-field[data-disabled] { opacity: 0.5; pointer-events: none; }

  .nr-input:has(.nr-field-input:placeholder-shown) .nr-field-clear { visibility: hidden; }
  .nr-password-show, .nr-password-hide { display: inline-grid; }
  .nr-password-toggle .nr-password-hide { display: none; }
  .nr-password-toggle[aria-pressed="true"] .nr-password-show { display: none; }
  .nr-password-toggle[aria-pressed="true"] .nr-password-hide { display: inline-grid; }

  .nr-textarea .nr-field-wrap { align-items: flex-start; padding-block: 0.625rem; }
  .nr-textarea-input { min-height: 3.75rem; max-height: 22rem; resize: vertical; field-sizing: content; }

  /* ---- Select ---------------------------------------------------------- */
  .nr-select .nr-field-wrap { cursor: pointer; }
  .nr-select-native { cursor: pointer; }
  .nr-select-native:where(.nr-ui-js *) { display: none; }
  .nr-select-trigger { display: none; }
  .nr-select-trigger:where(.nr-ui-js *) {
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    min-height: 1.25rem;
    margin: 0;
    padding: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--nr-fg);
    font: inherit;
    font-size: var(--nr-field-fs, 0.875rem);
    text-align: start;
    cursor: pointer;
  }
  .nr-select-value { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nr-select-value[data-placeholder] { color: var(--nr-subtle); }
  .nr-select-chevron { display: flex; color: var(--nr-muted); pointer-events: none; transition: transform 0.2s var(--nr-ease); }
  .nr-select-chevron .nr-icon { width: 1rem; height: 1rem; }
  .nr-select[data-open] .nr-select-chevron { transform: rotate(180deg); }
  .nr-select-list {
    position: absolute;
    top: calc(100% + 0.375rem);
    right: -2px;
    left: -2px;
    z-index: 60;
    box-sizing: border-box;
    max-height: 18rem;
    margin: 0;
    padding: 0.25rem;
    overflow: auto;
    overscroll-behavior: contain;
    list-style: none;
    border: 1px solid var(--nr-divider);
    border-radius: var(--nr-radius-lg);
    background: var(--nr-content);
    color: var(--nr-fg);
    box-shadow: var(--nr-shadow-md);
    cursor: default;
    transform-origin: top;
    animation: nr-pop 0.16s var(--nr-ease);
  }
  .nr-select-list[hidden] { display: none; }
  .nr-select-list[data-placement="top"] { top: auto; bottom: calc(100% + 0.375rem); transform-origin: bottom; }
  .nr-select-option {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.4375rem 0.5rem;
    border-radius: var(--nr-radius-sm);
    font-size: 0.875rem;
    line-height: 1.25rem;
    cursor: pointer;
    outline: none;
    transition: background-color 0.1s;
  }
  .nr-select-option[data-active] { background: var(--nr-content-2); }
  .nr-select-option[aria-selected="true"] { color: var(--c-ink); }
  .nr-select-option[aria-disabled="true"] { opacity: 0.45; cursor: not-allowed; }
  .nr-select-option-start { display: flex; flex-shrink: 0; }
  .nr-select-option-text { display: flex; flex: 1; flex-direction: column; min-width: 0; }
  .nr-select-option-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nr-select-option-description { color: var(--nr-muted); font-size: 0.75rem; line-height: 1rem; }
  .nr-select-check { display: flex; visibility: hidden; color: var(--c-ink); }
  .nr-select-check .nr-icon { width: 1rem; height: 1rem; }
  .nr-select-option[aria-selected="true"] .nr-select-check { visibility: visible; }

  /* ---- Date picker ----------------------------------------------------- */
  .nr-datepicker .nr-field-wrap { cursor: pointer; }
  .nr-datepicker-input { cursor: pointer; }
  .nr-datepicker-input::-webkit-calendar-picker-indicator { cursor: pointer; opacity: 0.6; }
  .nr-calendar {
    position: absolute;
    top: calc(100% + 0.375rem);
    left: -2px;
    z-index: 60;
    box-sizing: border-box;
    width: 19rem;
    max-width: calc(100vw - 2rem);
    padding: 0.75rem;
    border: 1px solid var(--nr-divider);
    border-radius: var(--nr-radius-lg);
    background: var(--nr-content);
    color: var(--nr-fg);
    box-shadow: var(--nr-shadow-lg);
    font-size: 0.875rem;
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
    transform-origin: top left;
    animation: nr-pop 0.16s var(--nr-ease);
  }
  .nr-calendar[hidden] { display: none; }
  .nr-calendar[data-placement="top"] { top: auto; bottom: calc(100% + 0.375rem); transform-origin: bottom left; }
  .nr-calendar[data-align="end"] { right: -2px; left: auto; }
  .nr-calendar .nr-cal-header { display: flex; align-items: center; justify-content: space-between; gap: 0.25rem; margin-bottom: 0.5rem; }
  .nr-calendar .nr-cal-title, .nr-calendar .nr-cal-nav, .nr-calendar .nr-cal-day, .nr-calendar .nr-cal-month, .nr-calendar .nr-cal-link {
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
    transition: background-color 0.12s, color 0.12s, transform 0.1s;
  }
  .nr-calendar .nr-cal-title { display: inline-flex; align-items: center; gap: 0.25rem; padding: 0.375rem 0.625rem; border-radius: var(--nr-radius-sm); font-size: 0.9375rem; font-weight: 600; }
  .nr-calendar .nr-cal-title .nr-icon { width: 1rem; height: 1rem; color: var(--nr-muted); transition: transform 0.2s var(--nr-ease); }
  .nr-calendar[data-view="months"] .nr-cal-title .nr-icon { transform: rotate(180deg); }
  .nr-calendar .nr-cal-nav { display: grid; place-items: center; width: 2rem; height: 2rem; border-radius: var(--nr-radius-sm); color: var(--nr-muted); }
  .nr-calendar .nr-cal-nav .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-calendar .nr-cal-nav:disabled { opacity: 0.3; cursor: not-allowed; }
  .nr-calendar .nr-cal-grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 0.125rem; }
  .nr-calendar .nr-cal-weekday { padding: 0.25rem 0 0.375rem; color: var(--nr-muted); font-size: 0.75rem; font-weight: 500; text-align: center; }
  .nr-calendar .nr-cal-day { position: relative; display: grid; place-items: center; height: 2.375rem; border-radius: var(--nr-radius-md); font-variant-numeric: tabular-nums; }
  .nr-calendar .nr-cal-day[data-outside] { color: var(--nr-subtle); }
  .nr-calendar .nr-cal-day[data-today] { color: var(--c-ink); font-weight: 600; }
  .nr-calendar .nr-cal-day[data-today]::after { content: ""; position: absolute; bottom: 0.3rem; width: 0.25rem; height: 0.25rem; border-radius: 50%; background: currentColor; }
  .nr-calendar .nr-cal-months { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.375rem; }
  .nr-calendar .nr-cal-month { height: 2.75rem; border-radius: var(--nr-radius-md); }
  .nr-calendar .nr-cal-month[data-current] { color: var(--c-ink); font-weight: 600; }
  .nr-calendar .nr-cal-day[aria-selected="true"], .nr-calendar .nr-cal-month[aria-selected="true"] { background: var(--c); color: var(--c-fg); font-weight: 600; }
  .nr-calendar .nr-cal-day:disabled, .nr-calendar .nr-cal-month:disabled { opacity: 0.3; cursor: not-allowed; text-decoration: line-through; }
  .nr-calendar .nr-cal-day:focus-visible, .nr-calendar .nr-cal-month:focus-visible, .nr-calendar .nr-cal-nav:focus-visible,
  .nr-calendar .nr-cal-title:focus-visible, .nr-calendar .nr-cal-link:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 1px; }
  .nr-calendar .nr-cal-day:active:not(:disabled), .nr-calendar .nr-cal-month:active:not(:disabled) { transform: scale(0.93); }
  .nr-calendar .nr-cal-footer { display: flex; justify-content: space-between; margin-top: 0.625rem; padding-top: 0.625rem; border-top: 1px solid var(--nr-divider); }
  .nr-calendar .nr-cal-link { padding: 0.375rem 0.625rem; border-radius: var(--nr-radius-sm); color: var(--c-ink); font-size: 0.8125rem; font-weight: 500; }
  @media (hover: hover) {
    .nr-calendar .nr-cal-title:hover, .nr-calendar .nr-cal-nav:hover:not(:disabled), .nr-calendar .nr-cal-day:hover:not(:disabled, [aria-selected="true"]),
    .nr-calendar .nr-cal-month:hover:not(:disabled, [aria-selected="true"]) { background: var(--nr-content-2); color: var(--nr-fg); }
    .nr-calendar .nr-cal-link:hover { background: color-mix(in oklab, var(--c) 14%, transparent); }
  }

  /* ---- Checkbox, switch, radio ----------------------------------------- */
  .nr-checkbox, .nr-radio, .nr-switch {
    position: relative;
    display: inline-flex;
    align-items: flex-start;
    gap: 0.5rem;
    color: var(--nr-fg);
    font-size: 0.875rem;
    line-height: var(--nr-box, 1.25rem);
    vertical-align: middle;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    -webkit-tap-highlight-color: transparent;
  }
  .nr-checkbox[data-disabled], .nr-radio[data-disabled], .nr-switch[data-disabled] { opacity: 0.5; cursor: not-allowed; }
  .nr-checkbox-input, .nr-radio-input, .nr-switch-input { position: absolute; width: 1px; height: 1px; margin: 0; opacity: 0; pointer-events: none; }
  .nr-checkbox-text, .nr-radio-text, .nr-switch-text { display: flex; flex-direction: column; gap: 0.125rem; }
  .nr-toggle-description { color: var(--nr-muted); font-size: 0.75rem; line-height: 1.25; }

  .nr-checkbox-sm, .nr-radio-sm { --nr-box: 1rem; font-size: 0.8125rem; }
  .nr-checkbox-md, .nr-radio-md { --nr-box: 1.25rem; }
  .nr-checkbox-lg, .nr-radio-lg { --nr-box: 1.5rem; font-size: 1rem; }
  .nr-checkbox-box {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    box-sizing: border-box;
    width: var(--nr-box, 1.25rem);
    height: var(--nr-box, 1.25rem);
    border: 2px solid var(--nr-content-4);
    border-radius: calc(var(--r, var(--nr-radius-md)) * 0.5);
    color: var(--c-fg);
    transition: background-color 0.15s, border-color 0.15s, transform 0.1s;
  }
  .nr-checkbox-box .nr-icon { width: 72%; height: 72%; opacity: 0; stroke-dasharray: 22; stroke-dashoffset: 22; transition: stroke-dashoffset 0.25s var(--nr-ease), opacity 0.1s; }
  .nr-checkbox:hover .nr-checkbox-box { border-color: var(--nr-subtle); }
  .nr-checkbox:active .nr-checkbox-box { transform: scale(0.9); }
  .nr-checkbox:has(.nr-checkbox-input:checked) .nr-checkbox-box { border-color: var(--c); background: var(--c); }
  .nr-checkbox:has(.nr-checkbox-input:checked) .nr-checkbox-box .nr-icon { opacity: 1; stroke-dashoffset: 0; }
  .nr-checkbox:has(.nr-checkbox-input:focus-visible) .nr-checkbox-box { outline: 2px solid var(--nr-focus); outline-offset: 2px; }

  .nr-radio-circle {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    box-sizing: border-box;
    width: var(--nr-box, 1.25rem);
    height: var(--nr-box, 1.25rem);
    border: 2px solid var(--nr-content-4);
    border-radius: 50%;
    transition: border-color 0.15s, transform 0.1s;
  }
  .nr-radio-circle::after { content: ""; width: 50%; height: 50%; border-radius: 50%; background: var(--c); transform: scale(0); transition: transform 0.2s var(--nr-ease); }
  .nr-radio:hover .nr-radio-circle { border-color: var(--nr-subtle); }
  .nr-radio:active .nr-radio-circle { transform: scale(0.9); }
  .nr-radio:has(.nr-radio-input:checked) .nr-radio-circle { border-color: var(--c); }
  .nr-radio:has(.nr-radio-input:checked) .nr-radio-circle::after { transform: scale(1); }
  .nr-radio:has(.nr-radio-input:focus-visible) .nr-radio-circle { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-radio-group { display: flex; flex-direction: column; gap: 0.5rem; min-width: 0; margin: 0; padding: 0; border: 0; }
  .nr-radio-group > .nr-field-label { margin-bottom: 0.5rem; padding: 0; color: var(--nr-muted); }
  .nr-radio-group-items { display: flex; flex-direction: column; gap: 0.625rem; }
  .nr-radio-group-row .nr-radio-group-items { flex-flow: row wrap; gap: 0.75rem 1.25rem; }
  .nr-radio-group[data-invalid] .nr-radio-circle { border-color: var(--nr-danger); }

  .nr-switch { align-items: center; gap: 0.625rem; line-height: 1.25rem; }
  .nr-switch-sm { --nr-sw-w: 2.5rem; --nr-sw-h: 1.375rem; font-size: 0.8125rem; }
  .nr-switch-md { --nr-sw-w: 3rem; --nr-sw-h: 1.75rem; }
  .nr-switch-lg { --nr-sw-w: 3.5rem; --nr-sw-h: 2rem; font-size: 1rem; }
  .nr-switch-track {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    box-sizing: border-box;
    width: var(--nr-sw-w, 3rem);
    height: var(--nr-sw-h, 1.75rem);
    padding: 0.1875rem;
    border-radius: 9999px;
    background: var(--nr-content-3);
    transition: background-color 0.2s;
  }
  .nr-switch-thumb {
    width: calc(var(--nr-sw-h, 1.75rem) - 0.375rem);
    height: calc(var(--nr-sw-h, 1.75rem) - 0.375rem);
    border-radius: 9999px;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.22), 0 1px 2px rgba(0, 0, 0, 0.12);
    transition: transform 0.22s var(--nr-ease);
  }
  .nr-switch:has(.nr-switch-input:checked) .nr-switch-track { background: var(--c); }
  .nr-switch:has(.nr-switch-input:checked) .nr-switch-thumb { transform: translateX(calc(var(--nr-sw-w, 3rem) - var(--nr-sw-h, 1.75rem))); }
  .nr-switch:active .nr-switch-thumb { scale: 0.92; }
  .nr-switch:has(.nr-switch-input:focus-visible) .nr-switch-track { outline: 2px solid var(--nr-focus); outline-offset: 2px; }

  /* ---- Avatar ---------------------------------------------------------- */
  .nr-avatar {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    width: var(--nr-av, 2.5rem);
    height: var(--nr-av, 2.5rem);
    overflow: hidden;
    border-radius: var(--r, 9999px);
    background: var(--c);
    color: var(--c-fg);
    font-size: calc(var(--nr-av, 2.5rem) * 0.38);
    font-weight: 500;
    line-height: 1;
    vertical-align: middle;
    user-select: none;
    transition: transform 0.2s var(--nr-ease);
  }
  .nr-avatar-sm { --nr-av: 2rem; }
  .nr-avatar-md { --nr-av: 2.5rem; }
  .nr-avatar-lg { --nr-av: 3.5rem; }
  .nr-avatar-bordered { box-shadow: 0 0 0 2px var(--nr-bg), 0 0 0 4px var(--c); }
  /* Transparent text: a broken image never shows its alt text over the initials. */
  .nr-avatar-img { position: absolute; inset: 0; width: 100%; height: 100%; border-radius: inherit; object-fit: cover; color: transparent; }
  .nr-avatar-img[hidden] { display: none; }
  .nr-avatar-fallback { display: flex; align-items: center; justify-content: center; }
  .nr-avatar-fallback .nr-icon { width: 60%; height: 60%; }
  .nr-avatar-group { display: inline-flex; align-items: center; padding-inline-start: 0.5rem; }
  .nr-avatar-group > .nr-avatar { margin-inline-start: -0.5rem; box-shadow: 0 0 0 2px var(--nr-bg); }
  .nr-avatar-group > .nr-avatar:hover { z-index: 1; transform: translateY(-2px); }

  /* ---- Card ------------------------------------------------------------ */
  .nr-card {
    position: relative;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    min-width: 0;
    overflow: hidden;
    border-radius: var(--r, var(--nr-radius-lg));
    background: var(--nr-content);
    color: var(--nr-fg);
    text-decoration: none;
    outline: none;
    transition: transform 0.2s var(--nr-ease), box-shadow 0.2s;
  }
  .nr-card-shadow-sm { box-shadow: var(--nr-shadow-sm); }
  .nr-card-shadow-md { box-shadow: var(--nr-shadow-md); }
  .nr-card-shadow-lg { box-shadow: var(--nr-shadow-lg); }
  .nr-card-bordered { border: 1px solid var(--nr-divider); }
  .nr-card-blurred {
    background: color-mix(in oklab, var(--nr-content) 65%, transparent);
    -webkit-backdrop-filter: saturate(1.5) blur(18px);
    backdrop-filter: saturate(1.5) blur(18px);
  }
  @media (hover: hover) {
    .nr-card-hoverable:hover { transform: translateY(-2px); box-shadow: var(--nr-shadow-lg); }
  }
  .nr-card-pressable { cursor: pointer; }
  .nr-card-pressable:active { transform: scale(0.985); }
  .nr-card-pressable:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-card-header { display: flex; flex-shrink: 0; align-items: center; gap: 0.75rem; padding: 1rem 1rem 0.5rem; }
  .nr-card-body { display: flex; flex: 1; flex-direction: column; gap: 0.5rem; min-height: 0; padding: 0.75rem 1rem; }
  .nr-card-footer { display: flex; flex-shrink: 0; align-items: center; gap: 0.5rem; padding: 0.5rem 1rem 1rem; }
  .nr-card-header :is(h1, h2, h3, h4, h5, h6, p), .nr-card-body :is(h1, h2, h3, h4, h5, h6, p) { margin: 0; }

  /* ---- Chip ------------------------------------------------------------ */
  .nr-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.125rem;
    box-sizing: border-box;
    max-width: fit-content;
    height: var(--nr-chip-h, 1.75rem);
    min-width: 0;
    padding: 0 0.25rem;
    border: 2px solid transparent;
    border-radius: var(--r, 9999px);
    font-size: var(--nr-chip-fs, 0.875rem);
    line-height: 1;
    white-space: nowrap;
    vertical-align: middle;
  }
  .nr-chip-sm { --nr-chip-h: 1.5rem; --nr-chip-fs: 0.75rem; }
  .nr-chip-md { --nr-chip-h: 1.75rem; }
  .nr-chip-lg { --nr-chip-h: 2rem; --nr-chip-fs: 1rem; }
  .nr-chip .nr-icon { width: 1em; height: 1em; }
  .nr-chip-content { padding: 0 0.375rem; overflow: hidden; text-overflow: ellipsis; }
  .nr-chip-dot { flex-shrink: 0; width: 0.5rem; height: 0.5rem; margin-inline-start: 0.25rem; border-radius: 50%; background: var(--c); }
  .nr-chip-solid { background: var(--c); color: var(--c-fg); }
  .nr-chip-solid .nr-chip-dot { background: var(--c-fg); }
  .nr-chip-shadow { background: var(--c); color: var(--c-fg); box-shadow: 0 6px 16px -6px color-mix(in oklab, var(--c) 60%, transparent); }
  .nr-chip-bordered, .nr-chip-ghost { border-color: var(--c); color: var(--c-ink); }
  .nr-chip-light { color: var(--c-ink); }
  .nr-chip-flat { background: color-mix(in oklab, var(--c) 20%, transparent); color: var(--c-text); }
  .nr-chip-faded { background: var(--nr-content-2); border-color: var(--nr-content-3); color: var(--c-ink); }

  /* ---- Divider --------------------------------------------------------- */
  .nr-divider { flex-shrink: 0; width: 100%; height: 1px; margin: 0; border: 0; background: var(--nr-divider); }
  .nr-divider-v { align-self: stretch; width: 1px; height: auto; min-height: 1em; }

  /* ---- Layout ---------------------------------------------------------- */
  .nr-container { box-sizing: border-box; width: 100%; margin-inline: auto; padding-inline: 1rem; }
  @media (min-width: 640px) {
    .nr-container { padding-inline: 1.5rem; }
  }
  .nr-container-sm { max-width: 40rem; }
  .nr-container-md { max-width: 48rem; }
  .nr-container-lg { max-width: 64rem; }
  .nr-container-xl { max-width: 80rem; }
  .nr-container-2xl { max-width: 96rem; }
  .nr-container-full { max-width: none; }

  .nr-stack { display: flex; flex-direction: column; min-width: 0; }
  .nr-stack-row { flex-direction: row; }
  .nr-stack-wrap { flex-wrap: wrap; }
  .nr-gap-0 { gap: 0; }
  .nr-gap-1 { gap: 0.25rem; }
  .nr-gap-2 { gap: 0.5rem; }
  .nr-gap-3 { gap: 0.75rem; }
  .nr-gap-4 { gap: 1rem; }
  .nr-gap-5 { gap: 1.25rem; }
  .nr-gap-6 { gap: 1.5rem; }
  .nr-gap-8 { gap: 2rem; }
  .nr-gap-10 { gap: 2.5rem; }
  .nr-gap-12 { gap: 3rem; }
  .nr-gap-16 { gap: 4rem; }
  .nr-items-start { align-items: flex-start; }
  .nr-items-center { align-items: center; }
  .nr-items-end { align-items: flex-end; }
  .nr-items-stretch { align-items: stretch; }
  .nr-items-baseline { align-items: baseline; }
  .nr-justify-start { justify-content: flex-start; }
  .nr-justify-center { justify-content: center; }
  .nr-justify-end { justify-content: flex-end; }
  .nr-justify-between { justify-content: space-between; }
  .nr-justify-around { justify-content: space-around; }
  .nr-justify-evenly { justify-content: space-evenly; }

  .nr-grid { display: grid; grid-template-columns: repeat(var(--nr-cols, 1), minmax(0, 1fr)); }
  .nr-grid-fit { grid-template-columns: repeat(auto-fill, minmax(min(var(--nr-grid-min, 16rem), 100%), 1fr)); }
  @media (min-width: 640px) {
    .nr-cols-2, .nr-cols-3, .nr-cols-4, .nr-cols-5, .nr-cols-6, .nr-cols-7, .nr-cols-8, .nr-cols-9, .nr-cols-10, .nr-cols-11, .nr-cols-12 { --nr-cols: 2; }
  }
  @media (min-width: 1024px) {
    .nr-cols-3 { --nr-cols: 3; }
    .nr-cols-4 { --nr-cols: 4; }
    .nr-cols-5 { --nr-cols: 5; }
    .nr-cols-6 { --nr-cols: 6; }
    .nr-cols-7 { --nr-cols: 7; }
    .nr-cols-8 { --nr-cols: 8; }
    .nr-cols-9 { --nr-cols: 9; }
    .nr-cols-10 { --nr-cols: 10; }
    .nr-cols-11 { --nr-cols: 11; }
    .nr-cols-12 { --nr-cols: 12; }
  }

  .nr-navbar { position: relative; z-index: 40; box-sizing: border-box; width: 100%; background: var(--nr-bg); color: var(--nr-fg); }
  .nr-navbar-sticky { position: sticky; top: 0; }
  .nr-navbar-blurred {
    background: color-mix(in oklab, var(--nr-bg) 72%, transparent);
    -webkit-backdrop-filter: saturate(1.5) blur(16px);
    backdrop-filter: saturate(1.5) blur(16px);
  }
  .nr-navbar-bordered { border-bottom: 1px solid var(--nr-divider); }
  .nr-navbar-inner { display: flex; align-items: center; gap: 1rem; height: 4rem; }
  .nr-navbar-brand { display: flex; flex-shrink: 0; align-items: center; gap: 0.5rem; font-size: 1.0625rem; font-weight: 700; }
  .nr-navbar-brand a { color: inherit; text-decoration: none; }
  .nr-navbar-content { display: flex; flex: 1; align-items: center; justify-content: center; gap: 0.25rem; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .nr-navbar-end { display: flex; flex-shrink: 0; align-items: center; gap: 0.5rem; margin-inline-start: auto; }
  .nr-navbar-link {
    display: inline-flex;
    align-items: center;
    height: 2.25rem;
    padding: 0 0.75rem;
    border-radius: var(--nr-radius-sm);
    color: var(--nr-muted);
    font-size: 0.9375rem;
    font-weight: 500;
    white-space: nowrap;
    text-decoration: none;
    transition: color 0.15s, background-color 0.15s;
  }
  .nr-navbar-link:hover { color: var(--nr-fg); }
  .nr-navbar-link-active { color: var(--nr-fg); font-weight: 600; }
  .nr-navbar-menu {
    display: none;
    place-items: center;
    width: 2.5rem;
    height: 2.5rem;
    margin: 0 0 0 -0.5rem;
    padding: 0;
    border: 0;
    border-radius: var(--nr-radius-sm);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .nr-navbar-menu:hover { background: var(--nr-content-2); }
  @media (max-width: 1023px) {
    .nr-navbar-menu { display: inline-grid; }
    /* With a menu button, the links live in the drawer on small screens. */
    .nr-navbar[data-nr-ui="shell"] .nr-navbar-content { display: none; }
  }

  .nr-sidebar { display: flex; flex-direction: column; gap: 0.125rem; padding: 0.75rem; font-size: 0.875rem; }
  .nr-sidebar-title { margin: 1rem 0.75rem 0.375rem; color: var(--nr-subtle); font-size: 0.6875rem; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; }
  .nr-sidebar-title:first-child { margin-top: 0.25rem; }
  .nr-sidebar-link {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    height: 2.25rem;
    padding: 0 0.75rem;
    border-radius: var(--nr-radius-md);
    color: var(--nr-muted);
    font-weight: 500;
    text-decoration: none;
    transition: background-color 0.15s, color 0.15s;
  }
  .nr-sidebar-link:hover { background: var(--nr-content-2); color: var(--nr-fg); }
  .nr-sidebar-link-active, .nr-sidebar-link-active:hover { background: color-mix(in oklab, var(--nr-primary) 14%, transparent); color: var(--nr-primary-text); }
  .nr-sidebar-icon { display: flex; }
  .nr-sidebar-icon svg { width: 1.125rem; height: 1.125rem; }
  .nr-nav-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  body:has(> .nr-shell) { margin: 0; }
  .nr-shell { display: flex; flex-direction: column; min-height: 100dvh; background: var(--nr-bg); color: var(--nr-fg); font-family: var(--nr-font); }
  .nr-shell-body { display: flex; flex: 1; min-height: 0; }
  .nr-shell-sidebar {
    position: sticky;
    top: 4rem;
    flex-shrink: 0;
    box-sizing: border-box;
    width: 16rem;
    height: calc(100dvh - 4rem);
    overflow-y: auto;
    border-inline-end: 1px solid var(--nr-divider);
    background: var(--nr-bg);
  }
  .nr-shell-main { flex: 1; min-width: 0; padding: 1.5rem 1rem; }
  .nr-shell-scrim { display: none; }
  .nr-shell-footer { padding: 1.5rem; border-top: 1px solid var(--nr-divider); color: var(--nr-muted); font-size: 0.875rem; }
  @media (min-width: 1024px) {
    .nr-shell-main { padding: 2rem; }
  }
  @media (max-width: 1023px) {
    .nr-shell-sidebar { position: fixed; top: 4rem; bottom: 0; left: 0; z-index: 50; height: auto; box-shadow: var(--nr-shadow-lg); transform: translateX(-105%); transition: transform 0.25s var(--nr-ease); }
    .nr-shell[data-open] .nr-shell-sidebar { transform: none; }
    .nr-shell[data-open] .nr-shell-scrim { position: fixed; inset: 4rem 0 0; z-index: 45; display: block; background: rgba(0, 0, 0, 0.4); }
  }

  /* ---- Radius (last, so it wins over component defaults) --------------- */
  .nr-r-none { --r: 0px; }
  .nr-r-sm { --r: var(--nr-radius-sm); }
  .nr-r-md { --r: var(--nr-radius-md); }
  .nr-r-lg { --r: var(--nr-radius-lg); }
  .nr-r-full { --r: 9999px; }

  @media (prefers-reduced-motion: reduce) {
    .nr-select-list, .nr-calendar { animation: none; }
    .nr-spinner-ring { animation-duration: 1.5s; }
  }
}

@keyframes nr-spin {
  to { transform: rotate(1turn); }
}

@keyframes nr-pop {
  from { opacity: 0; transform: scale(0.96) translateY(-4px); }
}
"####;
