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

    /* Series colors of charts, in order. */
    --nr-chart-1: #2a78d6;
    --nr-chart-2: #eb6834;
    --nr-chart-3: #1baf7a;
    --nr-chart-4: #eda100;
    --nr-chart-5: #e87ba4;
    --nr-chart-6: #008300;
    --nr-chart-7: #4a3aa7;
    --nr-chart-8: #e34948;
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
      --nr-chart-1: #3987e5;
      --nr-chart-2: #d95926;
      --nr-chart-3: #199e70;
      --nr-chart-4: #c98500;
      --nr-chart-5: #d55181;
      --nr-chart-7: #9085e9;
      --nr-chart-8: #e66767;
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
    --nr-chart-1: #3987e5;
    --nr-chart-2: #d95926;
    --nr-chart-3: #199e70;
    --nr-chart-4: #c98500;
    --nr-chart-5: #d55181;
    --nr-chart-7: #9085e9;
    --nr-chart-8: #e66767;
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
    /* Fade only. Its own transition also keeps the app's blanket motion off
       left/top/width, which the script sets when it lifts the list into the
       top layer (else it would slide there from its CSS spot). `display` and
       `overlay` hold it on screen while it fades out. */
    transition: opacity 0.15s var(--nr-ease), display 0.15s allow-discrete, overlay 0.15s allow-discrete;
  }
  .nr-select-list[hidden] { display: none; opacity: 0; }
  @starting-style { .nr-select-list:not([hidden]) { opacity: 0; } }
  .nr-select-list[data-placement="top"] { top: auto; bottom: calc(100% + 0.375rem); }
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
    /* Fade only, like .nr-select-list. */
    transition: opacity 0.15s var(--nr-ease), display 0.15s allow-discrete, overlay 0.15s allow-discrete;
  }
  .nr-calendar[hidden] { display: none; opacity: 0; }
  @starting-style { .nr-calendar:not([hidden]) { opacity: 0; } }
  .nr-calendar[data-placement="top"] { top: auto; bottom: calc(100% + 0.375rem); }
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

  /* ---- Alert ------------------------------------------------------------- */
  .nr-alert {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    box-sizing: border-box;
    width: 100%;
    padding: 0.75rem 1rem;
    border: 1px solid transparent;
    border-radius: var(--r, var(--nr-radius-md));
    font-family: var(--nr-font);
    font-size: 0.875rem;
    line-height: 1.4;
  }
  .nr-alert-icon { display: flex; flex-shrink: 0; margin-top: 0.0625rem; }
  .nr-alert-icon .nr-icon { width: 1.25rem; height: 1.25rem; }
  .nr-alert-body { flex: 1; min-width: 0; }
  .nr-alert-title { display: block; font-weight: 600; }
  .nr-alert-title + .nr-alert-text { margin-top: 0.125rem; }
  .nr-alert-text > :first-child { margin-top: 0; }
  .nr-alert-text > :last-child { margin-bottom: 0; }
  .nr-alert-end { display: flex; flex-shrink: 0; align-items: center; gap: 0.5rem; }
  .nr-alert-close {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: 1.75rem;
    height: 1.75rem;
    margin: -0.25rem -0.375rem -0.25rem 0;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
  }
  .nr-alert-close:hover { background: color-mix(in oklab, currentColor 12%, transparent); opacity: 1; }
  .nr-alert-close .nr-icon { width: 1rem; height: 1rem; }
  .nr-alert-flat { background: color-mix(in oklab, var(--c) 14%, transparent); color: var(--c-text); }
  .nr-alert-flat .nr-alert-title { color: var(--c-text); }
  .nr-alert-solid { background: var(--c); color: var(--c-fg); }
  .nr-alert-bordered { border-color: var(--c); color: var(--c-text); }
  .nr-alert-faded { border-color: color-mix(in oklab, var(--c) 35%, transparent); background: color-mix(in oklab, var(--c) 8%, transparent); color: var(--c-text); }
  .nr-alert.nr-c-default { --c-text: var(--nr-fg); }
  .nr-alert.nr-c-default.nr-alert-flat, .nr-alert.nr-c-default.nr-alert-faded { background: var(--nr-content-2); }

  /* ---- Badge ------------------------------------------------------------- */
  .nr-badge { position: relative; display: inline-flex; flex-shrink: 0; vertical-align: middle; }
  .nr-badge-content {
    position: absolute;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    min-width: var(--nr-badge-h, 1.25rem);
    height: var(--nr-badge-h, 1.25rem);
    padding: 0 0.3125rem;
    border: 2px solid var(--nr-bg);
    border-radius: 999px;
    background: var(--c);
    color: var(--c-fg);
    font-family: var(--nr-font);
    font-size: var(--nr-badge-fs, 0.6875rem);
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    pointer-events: none;
  }
  .nr-badge-sm { --nr-badge-h: 1rem; --nr-badge-fs: 0.625rem; }
  .nr-badge-md { --nr-badge-h: 1.25rem; }
  .nr-badge-lg { --nr-badge-h: 1.5rem; --nr-badge-fs: 0.75rem; }
  .nr-badge-dot { min-width: 0.625rem; height: 0.625rem; padding: 0; }
  .nr-badge-flat { background: color-mix(in oklab, var(--c) 20%, var(--nr-bg)); color: var(--c-text); }
  .nr-badge-faded { border-color: var(--nr-content-3); background: var(--nr-content-2); color: var(--c-text); }
  .nr-badge-tr { top: 0; right: 0; transform: translate(35%, -35%); }
  .nr-badge-tl { top: 0; left: 0; transform: translate(-35%, -35%); }
  .nr-badge-br { right: 0; bottom: 0; transform: translate(35%, 35%); }
  .nr-badge-bl { bottom: 0; left: 0; transform: translate(-35%, 35%); }

  /* ---- Progress ---------------------------------------------------------- */
  .nr-progress { display: flex; flex-direction: column; gap: 0.375rem; width: 100%; font-family: var(--nr-font); font-size: 0.875rem; }
  .nr-progress-label { display: flex; justify-content: space-between; gap: 1rem; color: var(--nr-fg); }
  .nr-progress-value { color: var(--nr-muted); font-variant-numeric: tabular-nums; }
  .nr-progress-track { overflow: hidden; height: var(--nr-progress-h, 0.5rem); border-radius: var(--r, 999px); background: var(--nr-content-3); }
  .nr-progress-bar { width: 0; height: 100%; border-radius: inherit; background: var(--c); transition: width 0.4s var(--nr-ease); }
  .nr-progress-sm { --nr-progress-h: 0.25rem; }
  .nr-progress-md { --nr-progress-h: 0.5rem; }
  .nr-progress-lg { --nr-progress-h: 0.75rem; }
  .nr-progress-striped .nr-progress-bar {
    background-image: linear-gradient(45deg, rgba(255, 255, 255, 0.25) 25%, transparent 25%, transparent 50%, rgba(255, 255, 255, 0.25) 50%, rgba(255, 255, 255, 0.25) 75%, transparent 75%);
    background-size: 1rem 1rem;
  }
  .nr-progress-indeterminate .nr-progress-bar { width: 40%; animation: nr-slide 1.4s var(--nr-ease) infinite; }

  .nr-ring { display: inline-flex; flex-direction: column; align-items: center; gap: 0.375rem; font-family: var(--nr-font); font-size: 0.875rem; }
  .nr-ring-face { position: relative; display: grid; width: var(--nr-ring-d, 3rem); height: var(--nr-ring-d, 3rem); place-items: center; }
  .nr-ring-svg { position: absolute; inset: 0; width: 100%; height: 100%; transform: rotate(-90deg); }
  .nr-ring-track { stroke: var(--nr-content-3); }
  .nr-ring-fill {
    stroke: var(--c);
    stroke-linecap: round;
    stroke-dasharray: 100.53;
    stroke-dashoffset: calc(100.53 - 100.53 * var(--p, 0) / 100);
    transition: stroke-dashoffset 0.4s var(--nr-ease);
  }
  .nr-ring-value { position: relative; color: var(--nr-fg); font-size: var(--nr-ring-fs, 0.75rem); font-weight: 600; font-variant-numeric: tabular-nums; }
  .nr-ring-label { color: var(--nr-muted); }
  .nr-ring-sm { --nr-ring-d: 2.25rem; --nr-ring-fs: 0.625rem; }
  .nr-ring-md { --nr-ring-d: 3rem; }
  .nr-ring-lg { --nr-ring-d: 4rem; --nr-ring-fs: 0.875rem; }
  .nr-ring-indeterminate .nr-ring-svg { animation: nr-spin 1s linear infinite; }
  .nr-ring-indeterminate .nr-ring-fill { stroke-dashoffset: 75; }

  /* ---- Skeleton ---------------------------------------------------------- */
  .nr-skeleton {
    display: block;
    width: 100%;
    min-height: 1rem;
    border-radius: var(--r, var(--nr-radius-sm));
    background: linear-gradient(90deg, var(--nr-content-2) 25%, var(--nr-content-3) 50%, var(--nr-content-2) 75%);
    background-size: 200% 100%;
    animation: nr-shimmer 1.6s linear infinite;
  }
  .nr-skeleton-lines { display: flex; flex-direction: column; gap: 0.5rem; min-height: 0; background: none; animation: none; }
  .nr-skeleton-line {
    display: block;
    height: 0.875rem;
    border-radius: var(--nr-radius-sm);
    background: linear-gradient(90deg, var(--nr-content-2) 25%, var(--nr-content-3) 50%, var(--nr-content-2) 75%);
    background-size: 200% 100%;
    animation: nr-shimmer 1.6s linear infinite;
  }
  .nr-skeleton-line:last-child:not(:first-child) { width: 60%; }

  /* ---- Stat -------------------------------------------------------------- */
  .nr-stat { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; font-family: var(--nr-font); }
  .nr-stat-body { display: flex; flex-direction: column; gap: 0.25rem; min-width: 0; }
  .nr-stat-label { color: var(--nr-muted); font-size: 0.8125rem; font-weight: 500; }
  .nr-stat-line { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0.5rem; }
  .nr-stat-value { color: var(--nr-fg); font-size: 1.75rem; font-weight: 700; line-height: 1.1; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .nr-stat-delta { display: inline-flex; align-items: center; gap: 0.125rem; color: var(--nr-muted); font-size: 0.8125rem; font-weight: 600; }
  .nr-stat-delta .nr-icon { width: 0.875rem; height: 0.875rem; }
  .nr-stat-up { color: var(--nr-success-text); }
  .nr-stat-down { color: var(--nr-danger-text); }
  .nr-stat-flat { color: var(--nr-muted); }
  .nr-stat-description { color: var(--nr-subtle); font-size: 0.75rem; }
  .nr-stat-icon {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: var(--nr-radius-md);
    background: color-mix(in oklab, var(--c, var(--nr-default)) 16%, transparent);
    color: var(--c-text, var(--nr-fg));
  }
  .nr-stat-icon .nr-icon { width: 1.25rem; height: 1.25rem; }

  /* ---- Kbd --------------------------------------------------------------- */
  .nr-kbd { display: inline-flex; align-items: center; gap: 0.25rem; font-family: var(--nr-font); }
  .nr-kbd kbd {
    display: inline-flex;
    align-items: center;
    height: 1.5rem;
    padding: 0 0.375rem;
    border: 1px solid var(--nr-content-3);
    border-bottom-width: 2px;
    border-radius: 0.375rem;
    background: var(--nr-content-2);
    color: var(--nr-muted);
    font-family: inherit;
    font-size: 0.75rem;
    line-height: 1;
  }

  /* ---- Empty state ------------------------------------------------------- */
  .nr-empty { display: flex; flex-direction: column; align-items: center; padding: 3rem 1.5rem; color: var(--nr-muted); font-family: var(--nr-font); text-align: center; }
  .nr-empty-compact { padding: 1.5rem 1rem; }
  .nr-empty-icon { display: grid; place-items: center; width: 3rem; height: 3rem; margin-bottom: 0.75rem; border-radius: 999px; background: var(--nr-content-2); color: var(--nr-subtle); }
  .nr-empty-icon .nr-icon { width: 1.5rem; height: 1.5rem; }
  .nr-empty-title { margin: 0; color: var(--nr-fg); font-size: 1rem; font-weight: 600; }
  .nr-empty-description { max-width: 28rem; margin: 0.25rem 0 0; font-size: 0.875rem; }
  .nr-empty-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.5rem; margin-top: 1rem; }

  /* ---- Tooltip ----------------------------------------------------------- */
  .nr-tooltip-anchor { position: relative; display: inline-flex; }
  .nr-tooltip {
    position: absolute;
    z-index: 60;
    max-width: 16rem;
    padding: 0.375rem 0.625rem;
    border-radius: var(--nr-radius-sm);
    background: var(--nr-fg);
    color: var(--nr-bg);
    font-family: var(--nr-font);
    font-size: 0.75rem;
    line-height: 1.3;
    white-space: normal;
    width: max-content;
    box-shadow: var(--nr-shadow-sm);
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.15s, visibility 0.15s, transform 0.15s var(--nr-ease);
    pointer-events: none;
  }
  .nr-tooltip:where(.nr-c-primary, .nr-c-secondary, .nr-c-success, .nr-c-warning, .nr-c-danger) { background: var(--c); color: var(--c-fg); }
  .nr-tooltip-anchor:hover > .nr-tooltip, .nr-tooltip-anchor:focus-within > .nr-tooltip { opacity: 1; visibility: visible; transform: none; }
  .nr-tooltip-top { bottom: calc(100% + 0.375rem); left: 50%; transform: translate(-50%, 4px); }
  .nr-tooltip-top:where(.nr-tooltip-anchor:hover > *, .nr-tooltip-anchor:focus-within > *) { transform: translateX(-50%); }
  .nr-tooltip-bottom { top: calc(100% + 0.375rem); left: 50%; transform: translate(-50%, -4px); }
  .nr-tooltip-bottom:where(.nr-tooltip-anchor:hover > *, .nr-tooltip-anchor:focus-within > *) { transform: translateX(-50%); }
  .nr-tooltip-left { top: 50%; right: calc(100% + 0.375rem); transform: translate(4px, -50%); }
  .nr-tooltip-left:where(.nr-tooltip-anchor:hover > *, .nr-tooltip-anchor:focus-within > *) { transform: translateY(-50%); }
  .nr-tooltip-right { top: 50%; left: calc(100% + 0.375rem); transform: translate(-4px, -50%); }
  .nr-tooltip-right:where(.nr-tooltip-anchor:hover > *, .nr-tooltip-anchor:focus-within > *) { transform: translateY(-50%); }

  /* ---- Tabs -------------------------------------------------------------- */
  .nr-tabs { display: flex; flex-direction: column; gap: 0.75rem; font-family: var(--nr-font); }
  .nr-tabs-list { display: inline-flex; align-items: center; gap: 0.125rem; max-width: 100%; overflow-x: auto; scrollbar-width: none; }
  .nr-tabs-list::-webkit-scrollbar { display: none; }
  .nr-tab {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    height: var(--nr-tab-h, 2.25rem);
    padding: 0 var(--nr-tab-px, 0.75rem);
    border: 0;
    border-radius: var(--r, var(--nr-radius-sm));
    background: transparent;
    color: var(--nr-muted);
    font-family: inherit;
    font-size: var(--nr-tab-fs, 0.875rem);
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
    transition: color 0.15s, background-color 0.15s, box-shadow 0.15s;
  }
  .nr-tab:hover { color: var(--nr-fg); }
  .nr-tab:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-tab:disabled, .nr-tab[aria-disabled="true"] { opacity: 0.5; cursor: not-allowed; pointer-events: none; }
  .nr-tab .nr-icon { width: 1.125em; height: 1.125em; }
  .nr-tab-selected { color: var(--nr-fg); }
  .nr-tabs-sm { --nr-tab-h: 1.875rem; --nr-tab-px: 0.625rem; --nr-tab-fs: 0.8125rem; }
  .nr-tabs-md { --nr-tab-h: 2.25rem; }
  .nr-tabs-lg { --nr-tab-h: 2.75rem; --nr-tab-px: 1rem; --nr-tab-fs: 1rem; }
  .nr-tabs-full .nr-tabs-list { display: flex; }
  .nr-tabs-full .nr-tab { flex: 1; }
  .nr-tabs-solid .nr-tabs-list, .nr-tabs-bordered .nr-tabs-list { padding: 0.25rem; border-radius: calc(var(--r, var(--nr-radius-sm)) + 0.25rem); }
  .nr-tabs-solid .nr-tabs-list { background: var(--nr-content-2); }
  .nr-tabs-bordered .nr-tabs-list { border: 2px solid var(--nr-content-3); }
  .nr-tabs-solid .nr-tab-selected, .nr-tabs-bordered .nr-tab-selected { background: var(--nr-content); box-shadow: var(--nr-shadow-sm); }
  .nr-tabs-solid:where(.nr-c-primary, .nr-c-secondary, .nr-c-success, .nr-c-warning, .nr-c-danger) .nr-tab-selected { background: var(--c); color: var(--c-fg); }
  .nr-tabs-light .nr-tab-selected { background: color-mix(in oklab, var(--c, var(--nr-default)) 16%, transparent); color: var(--c-text, var(--nr-fg)); }
  .nr-tabs-underlined .nr-tabs-list { gap: 0.5rem; border-bottom: 1px solid var(--nr-divider); }
  .nr-tabs-underlined .nr-tab { border-radius: 0; }
  .nr-tabs-underlined .nr-tab::after { content: ""; position: absolute; right: 0; bottom: -1px; left: 0; height: 2px; border-radius: 2px; background: transparent; transition: background-color 0.15s; }
  .nr-tabs-underlined .nr-tab-selected { color: var(--c-text, var(--nr-fg)); }
  .nr-tabs-underlined .nr-tab-selected::after { background: var(--c, var(--nr-fg)); }
  .nr-tabs.nr-c-default { --c: var(--nr-fg); --c-text: var(--nr-fg); }
  .nr-tab-panel { outline: none; }
  .nr-tab-panel:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; border-radius: var(--nr-radius-sm); }
  .nr-tab-panel > :first-child { margin-top: 0; }

  /* ---- Breadcrumbs ------------------------------------------------------- */
  .nr-breadcrumbs { font-family: var(--nr-font); font-size: var(--nr-crumb-fs, 0.875rem); }
  .nr-breadcrumbs-sm { --nr-crumb-fs: 0.75rem; }
  .nr-breadcrumbs-md { --nr-crumb-fs: 0.875rem; }
  .nr-breadcrumbs-lg { --nr-crumb-fs: 1rem; }
  .nr-breadcrumbs-list { display: flex; flex-wrap: wrap; align-items: center; margin: 0; padding: 0; list-style: none; }
  .nr-breadcrumb { display: inline-flex; align-items: center; }
  .nr-breadcrumb-sep { display: inline-flex; margin: 0 0.375rem; color: var(--nr-subtle); }
  .nr-breadcrumb-sep .nr-icon { width: 1em; height: 1em; }
  .nr-breadcrumb-link { display: inline-flex; align-items: center; gap: 0.25rem; color: var(--nr-muted); text-decoration: none; }
  .nr-breadcrumb-link .nr-icon { width: 1em; height: 1em; }
  .nr-breadcrumb-link[href]:hover { color: var(--nr-fg); text-decoration: underline; }
  .nr-breadcrumb-link[aria-disabled="true"] { opacity: 0.5; }
  .nr-breadcrumb-current { color: var(--nr-fg); font-weight: 500; }

  /* ---- Steps ------------------------------------------------------------- */
  .nr-steps { display: flex; gap: 0; margin: 0; padding: 0; list-style: none; font-family: var(--nr-font); font-size: var(--nr-step-fs, 0.875rem); }
  .nr-steps-sm { --nr-step-fs: 0.8125rem; --nr-step-d: 1.5rem; }
  .nr-steps-md { --nr-step-fs: 0.875rem; --nr-step-d: 1.875rem; }
  .nr-steps-lg { --nr-step-fs: 1rem; --nr-step-d: 2.25rem; }
  .nr-step { position: relative; display: flex; flex: 1; min-width: 0; }
  .nr-step-inner { display: flex; flex: 1; align-items: flex-start; gap: 0.625rem; min-width: 0; padding: 0.25rem 0; color: inherit; text-decoration: none; }
  .nr-step-inner[href]:hover .nr-step-title { color: var(--c-text, var(--nr-fg)); }
  .nr-step-marker {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    box-sizing: border-box;
    width: var(--nr-step-d, 1.875rem);
    height: var(--nr-step-d, 1.875rem);
    border: 2px solid var(--nr-content-3);
    border-radius: 999px;
    background: var(--nr-content);
    color: var(--nr-muted);
    font-size: 0.8em;
    font-weight: 600;
    transition: background-color 0.15s, border-color 0.15s, color 0.15s;
  }
  .nr-step-marker .nr-icon { width: 1em; height: 1em; }
  .nr-step-text { display: flex; flex-direction: column; gap: 0.125rem; min-width: 0; padding-top: calc((var(--nr-step-d, 1.875rem) - 1.25em) / 2); }
  .nr-step-title { color: var(--nr-muted); font-weight: 600; line-height: 1.25; }
  .nr-step-description { color: var(--nr-subtle); font-size: 0.85em; line-height: 1.3; }
  .nr-step:not(:last-child) .nr-step-inner::after { content: ""; flex: 1; height: 2px; margin: calc(var(--nr-step-d, 1.875rem) / 2 - 1px) 0.75rem 0; border-radius: 2px; background: var(--nr-content-3); }
  .nr-step-done .nr-step-marker { border-color: var(--c); background: var(--c); color: var(--c-fg); }
  .nr-step-done .nr-step-inner::after { background: var(--c); }
  .nr-step-done .nr-step-title, .nr-step-current .nr-step-title { color: var(--nr-fg); }
  .nr-step-current .nr-step-marker { border-color: var(--c); color: var(--c-text); box-shadow: 0 0 0 4px color-mix(in oklab, var(--c) 20%, transparent); }
  .nr-step-error .nr-step-marker { border-color: var(--nr-danger); background: var(--nr-danger); color: var(--nr-danger-fg); }
  .nr-step-error .nr-step-title { color: var(--nr-danger-text); }
  .nr-step-upcoming .nr-step-marker { color: var(--nr-subtle); }
  .nr-steps-vertical { flex-direction: column; }
  .nr-steps-vertical .nr-step { flex: none; }
  .nr-steps-vertical .nr-step-inner { padding: 0 0 1.25rem; }
  .nr-steps-vertical .nr-step:last-child .nr-step-inner { padding-bottom: 0; }
  .nr-steps-vertical .nr-step:not(:last-child) .nr-step-inner::after { position: absolute; top: calc(var(--nr-step-d, 1.875rem) + 0.25rem); bottom: 0.25rem; left: calc(var(--nr-step-d, 1.875rem) / 2 - 1px); flex: none; width: 2px; height: auto; margin: 0; }

  /* ---- Accordion --------------------------------------------------------- */
  .nr-accordion { display: flex; flex-direction: column; font-family: var(--nr-font); }
  .nr-accordion-item { border-bottom: 1px solid var(--nr-divider); }
  .nr-accordion-light .nr-accordion-item:last-child { border-bottom: 0; }
  .nr-accordion-bordered { border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-md); }
  .nr-accordion-bordered .nr-accordion-item, .nr-accordion-shadow .nr-accordion-item { padding: 0 1rem; }
  .nr-accordion-bordered .nr-accordion-item:last-child, .nr-accordion-shadow .nr-accordion-item:last-child { border-bottom: 0; }
  .nr-accordion-shadow { border-radius: var(--nr-radius-md); background: var(--nr-content); box-shadow: var(--nr-shadow-md); }
  .nr-accordion-splitted { gap: 0.5rem; }
  .nr-accordion-splitted .nr-accordion-item { padding: 0 1rem; border: 0; border-radius: var(--nr-radius-md); background: var(--nr-content); box-shadow: var(--nr-shadow-sm); }
  .nr-accordion-trigger {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 0;
    color: var(--nr-fg);
    cursor: pointer;
    list-style: none;
    user-select: none;
  }
  .nr-accordion-trigger::-webkit-details-marker { display: none; }
  .nr-accordion-trigger:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; border-radius: var(--nr-radius-sm); }
  .nr-accordion-trigger[aria-disabled="true"] { opacity: 0.5; cursor: not-allowed; pointer-events: none; }
  .nr-accordion-compact .nr-accordion-trigger { padding: 0.625rem 0; }
  .nr-accordion-heading { display: flex; flex: 1; flex-direction: column; gap: 0.125rem; min-width: 0; }
  .nr-accordion-title { font-size: 1rem; font-weight: 500; }
  .nr-accordion-subtitle { color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-accordion-indicator { display: flex; flex-shrink: 0; color: var(--nr-muted); transition: transform 0.2s var(--nr-ease); }
  .nr-accordion-indicator .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-accordion-item[open] > .nr-accordion-trigger .nr-accordion-indicator { transform: rotate(180deg); }
  .nr-accordion-content { padding: 0 0 1rem; color: var(--nr-muted); font-size: 0.9375rem; line-height: 1.5; }
  .nr-accordion-compact .nr-accordion-content { padding-bottom: 0.625rem; }
  .nr-accordion-content > :first-child { margin-top: 0; }
  .nr-accordion-content > :last-child { margin-bottom: 0; }

  /* ---- Modal ------------------------------------------------------------- */
  .nr-modal {
    box-sizing: border-box;
    width: min(var(--nr-modal-w, 32rem), calc(100vw - 2rem));
    max-width: none;
    max-height: calc(100dvh - 2rem);
    margin: auto;
    padding: 0;
    border: 0;
    border-radius: var(--r, var(--nr-radius-lg));
    background: var(--nr-content);
    color: var(--nr-fg);
    font-family: var(--nr-font);
    box-shadow: var(--nr-shadow-lg);
  }
  .nr-modal:not([open]) { display: none; }
  .nr-modal::backdrop { background: rgba(0, 0, 0, 0.5); backdrop-filter: blur(2px); }
  .nr-modal[open] { animation: nr-pop 0.2s var(--nr-ease); }
  .nr-modal-sm { --nr-modal-w: 24rem; }
  .nr-modal-md { --nr-modal-w: 32rem; }
  .nr-modal-lg { --nr-modal-w: 40rem; }
  .nr-modal-xl { --nr-modal-w: 48rem; }
  .nr-modal-2xl { --nr-modal-w: 64rem; }
  .nr-modal-full { width: 100vw; height: 100dvh; max-height: none; border-radius: 0; }
  .nr-modal-top { margin-top: 2rem; }
  .nr-modal-bottom { margin-bottom: 0; border-bottom-right-radius: 0; border-bottom-left-radius: 0; }
  .nr-modal-box { display: flex; flex-direction: column; max-height: inherit; }
  .nr-modal-scroll .nr-modal-box { max-height: calc(100dvh - 2rem); }
  .nr-modal-scroll .nr-modal-body { flex: 1; min-height: 0; overflow-y: auto; }
  .nr-modal-header { display: flex; align-items: flex-start; gap: 1rem; padding: 1.25rem 1.5rem 0; }
  .nr-modal-title { flex: 1; margin: 0; font-size: 1.125rem; font-weight: 600; line-height: 1.3; }
  .nr-modal-closer { display: contents; }
  .nr-modal-close {
    display: grid;
    flex-shrink: 0;
    place-items: center;
    width: 2rem;
    height: 2rem;
    margin: -0.375rem -0.5rem -0.375rem auto;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--nr-muted);
    cursor: pointer;
  }
  .nr-modal-close:hover { background: var(--nr-content-2); color: var(--nr-fg); }
  .nr-modal-close:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-modal-close .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-modal-body { padding: 1rem 1.5rem; font-size: 0.9375rem; line-height: 1.5; }
  .nr-modal-body > :first-child { margin-top: 0; }
  .nr-modal-body > :last-child { margin-bottom: 0; }
  .nr-modal-footer { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 0.5rem; padding: 0 1.5rem 1.25rem; }

  /* ---- Table ------------------------------------------------------------- */
  .nr-table-wrap { overflow-x: auto; border-radius: var(--r, var(--nr-radius-md)); font-family: var(--nr-font); }
  .nr-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  .nr-table-caption { padding: 0.75rem 1rem; color: var(--nr-muted); font-size: 0.8125rem; text-align: start; caption-side: top; }
  .nr-table th, .nr-table td { padding: 0.75rem 1rem; text-align: start; vertical-align: middle; }
  .nr-table th { background: var(--nr-content-2); color: var(--nr-muted); font-size: 0.75rem; font-weight: 600; letter-spacing: 0.02em; text-transform: uppercase; white-space: nowrap; }
  .nr-table thead tr:first-child th:first-child { border-start-start-radius: var(--r, var(--nr-radius-md)); }
  .nr-table thead tr:first-child th:last-child { border-start-end-radius: var(--r, var(--nr-radius-md)); }
  .nr-table td { border-top: 1px solid var(--nr-divider); color: var(--nr-fg); }
  .nr-table tbody tr:first-child td { border-top: 0; }
  .nr-table-striped tbody tr:nth-child(even) td { background: color-mix(in oklab, var(--nr-content-2) 60%, transparent); }
  .nr-table-compact th, .nr-table-compact td { padding: 0.5rem 0.75rem; }
  .nr-table-bordered th, .nr-table-bordered td { border: 1px solid var(--nr-divider); }
  .nr-table-hoverable tbody tr:hover td { background: var(--nr-content-2); }
  .nr-table-sticky { max-height: var(--nr-table-h, 32rem); overflow-y: auto; }
  .nr-table-sticky th { position: sticky; top: 0; z-index: 1; }
  .nr-table-empty { padding: 0; }

  /* ---- Screen-reader-only text ------------------------------------------- */
  .nr-sr { position: absolute; width: 1px; height: 1px; margin: -1px; padding: 0; overflow: hidden; clip-path: inset(50%); border: 0; white-space: nowrap; }

  /* ---- Popover panels: dropdown menus, popovers, context menus ----------- */
  .nr-dropdown { position: relative; display: inline-block; }
  .nr-popover-anchor { position: relative; display: inline-block; }
  .nr-pop {
    box-sizing: border-box;
    max-width: calc(100vw - 1rem);
    padding: 0.25rem;
    border: 1px solid var(--nr-divider);
    border-radius: var(--nr-radius-lg);
    background: var(--nr-content);
    color: var(--nr-fg);
    font-family: var(--nr-font);
    font-size: 0.875rem;
    box-shadow: var(--nr-shadow-md);
    /* Its own transition keeps the app's blanket motion off the position. */
    transition: opacity 0.15s;
  }
  .nr-pop:popover-open { animation: nr-pop 0.15s var(--nr-ease); }
  /* Next to the trigger (the popover's implicit anchor); centered on screen
     where anchor positioning is missing and the script did not run. */
  @supports (position-area: block-end) {
    .nr-pop { position: fixed; inset: auto; margin: 0.375rem 0; position-area: block-end span-inline-end; position-try-fallbacks: flip-block, flip-inline, flip-block flip-inline; }
    .nr-pop-end { position-area: block-end span-inline-start; }
    .nr-pop-top { position-area: block-start span-inline-end; }
    .nr-pop-top.nr-pop-end { position-area: block-start span-inline-start; }
    .nr-pop-left { margin: 0 0.375rem; position-area: inline-start span-block-end; }
    .nr-pop-right { margin: 0 0.375rem; position-area: inline-end span-block-end; }
  }
  .nr-pop-free { position: fixed; inset: auto; margin: 0; position-area: none; }
  .nr-menu { min-width: 12rem; max-height: min(24rem, calc(100dvh - 2rem)); overflow-y: auto; overscroll-behavior: contain; }
  .nr-menu-item {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    box-sizing: border-box;
    width: 100%;
    margin: 0;
    padding: 0.4375rem 0.5rem;
    border: 0;
    border-radius: var(--nr-radius-sm);
    outline: none;
    background: transparent;
    color: var(--c-text);
    font: inherit;
    line-height: 1.25rem;
    text-align: start;
    text-decoration: none;
    cursor: pointer;
  }
  .nr-menu-item:hover, .nr-menu-item:focus-visible { background: color-mix(in oklab, var(--c) 18%, transparent); }
  .nr-menu-item.nr-c-default { color: var(--nr-fg); }
  .nr-menu-item.nr-c-default:hover, .nr-menu-item.nr-c-default:focus-visible { background: color-mix(in oklab, var(--nr-fg) 8%, transparent); }
  .nr-menu-item:focus-visible { box-shadow: inset 0 0 0 2px var(--nr-focus); }
  .nr-menu-item:disabled, .nr-menu-item[aria-disabled="true"] { opacity: 0.5; pointer-events: none; }
  .nr-menu-item .nr-icon { width: 1rem; height: 1rem; }
  .nr-menu-start { display: flex; flex-shrink: 0; color: var(--nr-muted); }
  .nr-menu-start:where(.nr-c-primary *, .nr-c-secondary *, .nr-c-success *, .nr-c-warning *, .nr-c-danger *) { color: inherit; }
  .nr-menu-label { display: flex; flex: 1; flex-direction: column; min-width: 0; }
  .nr-menu-description { color: var(--nr-muted); font-size: 0.75rem; line-height: 1rem; }
  .nr-menu-end { display: flex; flex-shrink: 0; color: var(--nr-muted); }
  .nr-menu-shortcut { flex-shrink: 0; padding: 0 0.3125rem; border: 1px solid var(--nr-divider); border-radius: 0.375rem; color: var(--nr-muted); font-family: inherit; font-size: 0.6875rem; line-height: 1.125rem; }
  .nr-menu-section + .nr-menu-section, .nr-menu-item + .nr-menu-section, .nr-menu-section + .nr-menu-item { margin-top: 0.25rem; border-top: 1px solid var(--nr-divider); }
  .nr-menu-section + .nr-menu-section, .nr-menu-item + .nr-menu-section { padding-top: 0.25rem; }
  .nr-menu-title { padding: 0.375rem 0.5rem 0.25rem; color: var(--nr-muted); font-size: 0.75rem; font-weight: 600; }
  .nr-popover { width: max-content; max-width: min(20rem, calc(100vw - 1rem)); padding: 0.875rem 1rem; line-height: 1.5; }
  .nr-popover > :first-child { margin-top: 0; }
  .nr-popover > :last-child { margin-bottom: 0; }
  .nr-popover-title { display: block; margin-bottom: 0.25rem; font-size: 0.9375rem; font-weight: 600; }
  .nr-context { display: contents; }

  /* ---- Drawer, confirm dialog, loading overlay --------------------------- */
  .nr-drawer { --nr-drawer-from: translateX(100%); width: min(var(--nr-drawer-w, 24rem), calc(100vw - 3rem)); height: 100dvh; max-height: none; margin-block: 0; border-radius: 0; }
  .nr-drawer[open] { animation: nr-drawer-in 0.25s var(--nr-ease); }
  .nr-drawer-right { margin-inline: auto 0; }
  .nr-drawer-right:dir(rtl) { --nr-drawer-from: translateX(-100%); }
  .nr-drawer-left { --nr-drawer-from: translateX(-100%); margin-inline: 0 auto; }
  .nr-drawer-left:dir(rtl) { --nr-drawer-from: translateX(100%); }
  .nr-drawer-top, .nr-drawer-bottom { width: 100vw; height: auto; max-height: min(var(--nr-drawer-w, 24rem), calc(100dvh - 3rem)); margin-inline: 0; }
  .nr-drawer-top { --nr-drawer-from: translateY(-100%); margin-block: 0 auto; }
  .nr-drawer-bottom { --nr-drawer-from: translateY(100%); margin-block: auto 0; border-start-start-radius: var(--nr-radius-lg); border-start-end-radius: var(--nr-radius-lg); }
  .nr-drawer-sm { --nr-drawer-w: 18rem; }
  .nr-drawer-md { --nr-drawer-w: 24rem; }
  .nr-drawer-lg { --nr-drawer-w: 32rem; }
  .nr-drawer-xl { --nr-drawer-w: 40rem; }
  .nr-drawer-2xl { --nr-drawer-w: 48rem; }
  .nr-drawer-full { --nr-drawer-w: 100vmax; width: 100vw; }
  .nr-drawer .nr-modal-box { height: 100%; max-height: inherit; }
  .nr-drawer .nr-modal-body { flex: 1; min-height: 0; overflow-y: auto; }
  .nr-confirm .nr-modal-body { color: var(--nr-muted); }
  .nr-loading { position: relative; }
  .nr-loading-veil { position: absolute; inset: 0; z-index: 5; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.75rem; border-radius: inherit; background: color-mix(in oklab, var(--nr-content) 70%, transparent); color: var(--c-ink); animation: nr-fade 0.2s; }
  .nr-loading-blur { backdrop-filter: blur(3px); }
  .nr-loading-spinner { --nr-sp: 2rem; }
  .nr-loading-label { color: var(--nr-fg); font-family: var(--nr-font); font-size: 0.875rem; font-weight: 500; }

  /* ---- Toasts ------------------------------------------------------------ */
  .nr-toaster { position: fixed; inset: auto; z-index: 100; display: flex; flex-direction: column; gap: 0.5rem; box-sizing: border-box; width: min(24rem, 100vw); max-height: 100dvh; margin: 0; padding: 1rem; border: 0; overflow: visible; background: transparent; font-family: var(--nr-font); pointer-events: none; }
  .nr-toaster-tl { inset-block-start: 0; inset-inline-start: 0; flex-direction: column-reverse; }
  .nr-toaster-tc { inset-block-start: 0; inset-inline: 0; flex-direction: column-reverse; margin-inline: auto; }
  .nr-toaster-tr { inset-block-start: 0; inset-inline-end: 0; flex-direction: column-reverse; }
  .nr-toaster-bl { inset-block-end: 0; inset-inline-start: 0; }
  .nr-toaster-bc { inset-block-end: 0; inset-inline: 0; margin-inline: auto; }
  .nr-toaster-br { inset-block-end: 0; inset-inline-end: 0; }
  /* Keyed on the toaster: the script adds toasts to pages that rendered none. */
  .nr-toaster .nr-toast {
    display: flex;
    flex-shrink: 0;
    align-items: flex-start;
    gap: 0.625rem;
    box-sizing: border-box;
    padding: 0.75rem 0.875rem;
    overflow: hidden;
    border: 1px solid var(--nr-divider);
    border-radius: var(--nr-radius-md);
    background: var(--nr-content);
    color: var(--nr-fg);
    font-size: 0.875rem;
    line-height: 1.4;
    box-shadow: var(--nr-shadow-md);
    pointer-events: auto;
    /* Leaves on its own, script or not; hovering it holds it. */
    animation: nr-toast-in 0.25s var(--nr-ease), nr-toast-out 0.3s ease-in var(--nr-toast-d, 5s) forwards;
  }
  .nr-toaster .nr-toast:hover, .nr-toaster .nr-toast:focus-within { animation-play-state: paused; }
  .nr-toaster .nr-toast-sticky { animation: nr-toast-in 0.25s var(--nr-ease); }
  .nr-toaster .nr-toast-icon { display: flex; flex-shrink: 0; margin-top: 0.0625rem; color: var(--nr-muted); }
  .nr-toaster [data-color="primary"] .nr-toast-icon { color: var(--nr-primary-text); }
  .nr-toaster [data-color="secondary"] .nr-toast-icon { color: var(--nr-secondary-text); }
  .nr-toaster [data-color="success"] .nr-toast-icon { color: var(--nr-success-text); }
  .nr-toaster [data-color="warning"] .nr-toast-icon { color: var(--nr-warning-text); }
  .nr-toaster [data-color="danger"] .nr-toast-icon { color: var(--nr-danger-text); }
  .nr-toaster .nr-toast-icon .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-toaster .nr-toast-body { flex: 1; min-width: 0; }
  .nr-toaster .nr-toast-title { display: block; font-weight: 600; }
  .nr-toaster .nr-toast-text { color: var(--nr-muted); }
  .nr-toaster .nr-toast-text:first-child { color: var(--nr-fg); }
  .nr-toaster .nr-toast-close { display: grid; flex-shrink: 0; place-items: center; width: 1.5rem; height: 1.5rem; margin: -0.125rem -0.25rem 0 0; padding: 0; border: 0; border-radius: 999px; background: transparent; color: var(--nr-muted); cursor: pointer; }
  .nr-toaster .nr-toast-close:not(:where(.nr-ui-js *)) { display: none; }
  .nr-toaster .nr-toast-close:hover { background: var(--nr-content-2); color: var(--nr-fg); }
  .nr-toaster .nr-toast-close:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 1px; }
  .nr-toaster .nr-toast-close .nr-icon { width: 0.875rem; height: 0.875rem; }

  /* ---- Command palette --------------------------------------------------- */
  .nr-command { --nr-modal-w: 36rem; margin-top: min(12vh, 6rem); overflow: hidden; }
  .nr-command-box { display: flex; flex-direction: column; max-height: min(28rem, calc(100dvh - 8rem)); }
  .nr-command-search { display: flex; align-items: center; gap: 0.625rem; padding: 0.75rem 1rem; border-bottom: 1px solid var(--nr-divider); color: var(--nr-muted); }
  .nr-command-search:focus-within { box-shadow: inset 0 -2px 0 var(--nr-focus); }
  .nr-command-input { flex: 1; min-width: 0; margin: 0; padding: 0; border: 0; outline: 0; background: transparent; color: var(--nr-fg); font: inherit; font-size: 1rem; appearance: none; -webkit-appearance: none; }
  .nr-command-input::placeholder { color: var(--nr-subtle); opacity: 1; }
  .nr-command-input::-webkit-search-cancel-button { display: none; }
  .nr-command-input:not(:where(.nr-ui-js *)) { visibility: hidden; }
  .nr-command-esc { padding: 0.125rem 0.375rem; border: 1px solid var(--nr-divider); border-radius: 0.375rem; background: var(--nr-content-2); color: var(--nr-muted); font: inherit; font-size: 0.6875rem; cursor: pointer; }
  .nr-command-esc:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 1px; }
  .nr-command-list { flex: 1; min-height: 0; padding: 0.375rem; overflow-y: auto; overscroll-behavior: contain; }
  .nr-command-group[hidden] { display: none; }
  .nr-command-heading { padding: 0.5rem 0.625rem 0.25rem; color: var(--nr-muted); font-size: 0.75rem; font-weight: 600; }
  .nr-command-item { display: flex; align-items: center; gap: 0.625rem; box-sizing: border-box; width: 100%; margin: 0; padding: 0.5rem 0.625rem; border: 0; border-radius: var(--nr-radius-sm); outline: none; background: transparent; color: var(--nr-fg); font: inherit; font-size: 0.875rem; text-align: start; text-decoration: none; cursor: pointer; }
  .nr-command-item[hidden] { display: none; }
  .nr-command-item:focus-visible, .nr-command-item[data-active] { background: color-mix(in oklab, var(--nr-fg) 8%, transparent); }
  .nr-command-item:focus-visible { box-shadow: inset 0 0 0 2px var(--nr-focus); }
  .nr-command-start { display: flex; flex-shrink: 0; color: var(--nr-muted); }
  .nr-command-start .nr-icon { width: 1rem; height: 1rem; }
  .nr-command-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nr-command-shortcut { flex-shrink: 0; color: var(--nr-muted); font-family: inherit; font-size: 0.75rem; }
  .nr-command-empty { margin: 0; padding: 2rem 1rem; color: var(--nr-muted); font-size: 0.875rem; text-align: center; }
  .nr-command-empty[hidden] { display: none; }

  /* ---- Search, number, time, combobox, tags ------------------------------ */
  .nr-search-form { display: block; margin: 0; }
  .nr-search .nr-field-input::-webkit-search-cancel-button { display: none; }
  .nr-search-shortcut { padding: 0 0.3125rem; border: 1px solid var(--nr-divider); border-radius: 0.375rem; color: var(--nr-muted); font-family: inherit; font-size: 0.6875rem; line-height: 1.125rem; white-space: nowrap; }
  .nr-number .nr-field-input:where(.nr-ui-js *) { appearance: textfield; -moz-appearance: textfield; }
  .nr-number .nr-field-input:where(.nr-ui-js *)::-webkit-inner-spin-button { -webkit-appearance: none; }
  .nr-number .nr-field-input:not(:where(.nr-ui-js *)) { appearance: auto; -moz-appearance: auto; }
  .nr-number-steps { display: flex; flex-direction: column; margin-inline-end: -0.25rem; }
  .nr-number-step { width: 1.5rem; height: 0.9375rem; margin: 0; border-radius: 0.3125rem; }
  .nr-number-step .nr-icon { width: 0.875rem; height: 0.875rem; }
  .nr-time .nr-field-input { min-height: 1.25rem; }
  .nr-time .nr-field-input::-webkit-calendar-picker-indicator { opacity: 0.55; cursor: pointer; }
  .nr-combobox-chevron { display: flex; color: var(--nr-muted); pointer-events: none; }
  .nr-combobox-chevron .nr-icon { width: 1rem; height: 1rem; }
  .nr-combobox-list { position: absolute; top: calc(100% + 0.375rem); right: -2px; left: -2px; z-index: 60; box-sizing: border-box; max-height: 16rem; margin: 0; padding: 0.25rem; overflow: auto; overscroll-behavior: contain; list-style: none; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-lg); background: var(--nr-content); color: var(--nr-fg); font-size: 0.875rem; box-shadow: var(--nr-shadow-md); cursor: default; }
  .nr-combobox-list[hidden] { display: none; }
  .nr-combobox-list[data-placement="top"] { top: auto; bottom: calc(100% + 0.375rem); }
  .nr-combobox-list li { padding: 0.4375rem 0.5rem; border-radius: var(--nr-radius-sm); cursor: pointer; }
  .nr-combobox-list li[data-active], .nr-combobox-list li[role="option"]:hover { background: color-mix(in oklab, var(--nr-fg) 8%, transparent); }
  .nr-combobox-list li[aria-selected="true"] { font-weight: 600; }
  .nr-combobox-list li[role="presentation"] { color: var(--nr-muted); cursor: default; }
  .nr-tags .nr-tags-row { display: flex; flex-wrap: wrap; align-items: center; gap: 0.25rem; padding-block: 0.125rem; }
  .nr-tags .nr-tags-row .nr-field-input { flex: 1; width: auto; min-width: 5rem; }
  .nr-tags .nr-tags-chip { display: inline-flex; align-items: center; gap: 0.125rem; height: 1.5rem; padding-inline: 0.5rem 0.125rem; border-radius: 999px; background: color-mix(in oklab, var(--c) 20%, transparent); color: var(--c-text); font-size: 0.8125rem; white-space: nowrap; }
  .nr-tags .nr-tags-chip button { display: grid; place-items: center; width: 1.125rem; height: 1.125rem; padding: 0; border: 0; border-radius: 50%; background: transparent; color: inherit; font-size: 0.875rem; line-height: 1; cursor: pointer; opacity: 0.7; }
  .nr-tags .nr-tags-chip button:hover { background: color-mix(in oklab, var(--c) 24%, transparent); opacity: 1; }
  .nr-tags .nr-tags-chip button:focus-visible { outline: 2px solid var(--nr-focus); opacity: 1; }

  /* ---- Slider ------------------------------------------------------------ */
  .nr-slider { display: flex; flex-direction: column; gap: 0.5rem; min-width: 0; font-family: var(--nr-font); font-size: 0.875rem; }
  .nr-slider[data-disabled] { opacity: 0.5; pointer-events: none; }
  .nr-slider-sm { --nr-sl: 0.25rem; --nr-th: 1rem; }
  .nr-slider-md { --nr-sl: 0.375rem; --nr-th: 1.25rem; }
  .nr-slider-lg { --nr-sl: 0.5rem; --nr-th: 1.5rem; }
  .nr-slider-head { display: flex; align-items: baseline; justify-content: space-between; gap: 1rem; }
  .nr-slider-value { color: var(--nr-muted); font-variant-numeric: tabular-nums; }
  .nr-slider-value:not(:where(.nr-ui-js *)) { display: none; }
  .nr-slider-input { width: 100%; height: var(--nr-th, 1.25rem); margin: 0; outline: none; background: transparent; cursor: pointer; appearance: none; -webkit-appearance: none; }
  .nr-slider-input::-webkit-slider-runnable-track { height: var(--nr-sl, 0.375rem); border-radius: 999px; background: var(--nr-content-3); }
  /* The filled part needs the script to follow the thumb in WebKit and Blink. */
  .nr-slider-input:where(.nr-ui-js *)::-webkit-slider-runnable-track { background: linear-gradient(to right, var(--c) var(--nr-v, 0%), var(--nr-content-3) var(--nr-v, 0%)); }
  .nr-slider-input:where(.nr-ui-js *):dir(rtl)::-webkit-slider-runnable-track { background: linear-gradient(to left, var(--c) var(--nr-v, 0%), var(--nr-content-3) var(--nr-v, 0%)); }
  .nr-slider-input::-webkit-slider-thumb { box-sizing: border-box; width: var(--nr-th, 1.25rem); height: var(--nr-th, 1.25rem); margin-top: calc((var(--nr-sl, 0.375rem) - var(--nr-th, 1.25rem)) / 2); border: 2px solid var(--c); border-radius: 50%; background: #fff; box-shadow: var(--nr-shadow-sm); transition: transform 0.15s var(--nr-ease); -webkit-appearance: none; }
  .nr-slider-input::-moz-range-track { height: var(--nr-sl, 0.375rem); border-radius: 999px; background: var(--nr-content-3); }
  .nr-slider-input::-moz-range-progress { height: var(--nr-sl, 0.375rem); border-radius: 999px; background: var(--c); }
  .nr-slider-input::-moz-range-thumb { box-sizing: border-box; width: var(--nr-th, 1.25rem); height: var(--nr-th, 1.25rem); border: 2px solid var(--c); border-radius: 50%; background: #fff; box-shadow: var(--nr-shadow-sm); }
  .nr-slider-input:active::-webkit-slider-thumb { transform: scale(1.12); }
  .nr-slider-input:focus-visible::-webkit-slider-thumb { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-slider-input:focus-visible::-moz-range-thumb { outline: 2px solid var(--nr-focus); outline-offset: 2px; }

  /* ---- Pin input --------------------------------------------------------- */
  .nr-pin { display: flex; flex-direction: column; gap: 0.375rem; min-width: 0; font-family: var(--nr-font); }
  .nr-pin-sm { --nr-pin-w: 2.25rem; --nr-pin-fs: 1rem; }
  .nr-pin-md { --nr-pin-w: 2.75rem; --nr-pin-fs: 1.25rem; }
  .nr-pin-lg { --nr-pin-w: 3.25rem; --nr-pin-fs: 1.5rem; }
  /* A fixed cell width, smaller on phones: the box must not depend on its
     own content width (a container query here would feed back into itself). */
  .nr-pin-box { position: relative; width: calc(var(--nr-pin-n, 6) * (var(--nr-pin-w, 2.75rem) + 0.5rem) - 0.5rem); max-width: 100%; height: calc(var(--nr-pin-w, 2.75rem) * 1.15); overflow-x: clip; }
  .nr-pin-cells { position: absolute; inset: 0; display: flex; gap: 0.5rem; pointer-events: none; }
  .nr-pin-cells i { box-sizing: border-box; flex: none; width: var(--nr-pin-w, 2.75rem); border: 2px solid var(--nr-content-3); border-radius: var(--nr-radius-md); background: var(--nr-content-2); transition: border-color 0.15s; }
  .nr-pin-input { position: absolute; inset-block: 0; inset-inline-start: 0; box-sizing: border-box; width: calc(100% + var(--nr-pin-w, 2.75rem) + 0.5rem); margin: 0; padding: 0; padding-inline-start: calc((var(--nr-pin-w, 2.75rem) - 1ch) / 2); border: 0; outline: 0; background: transparent; color: var(--nr-fg); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: var(--nr-pin-fs, 1.25rem); letter-spacing: calc(var(--nr-pin-w, 2.75rem) + 0.5rem - 1ch); caret-color: var(--c); }
  @media (max-width: 26rem) {
    .nr-pin-md { --nr-pin-w: 2.25rem; --nr-pin-fs: 1.125rem; }
    .nr-pin-lg { --nr-pin-w: 2.5rem; --nr-pin-fs: 1.25rem; }
  }
  .nr-pin-box:focus-within .nr-pin-cells i { border-color: var(--c); }
  .nr-pin[data-invalid] .nr-pin-cells i { border-color: var(--nr-danger); }
  .nr-pin-input:disabled { opacity: 0.5; }

  /* ---- Rating ------------------------------------------------------------ */
  .nr-rating { display: inline-flex; flex-direction: column; gap: 0.375rem; min-width: 0; margin: 0; padding: 0; border: 0; font-family: var(--nr-font); vertical-align: middle; }
  .nr-rating:disabled { opacity: 0.5; pointer-events: none; }
  .nr-rating-sm { --nr-star: 1.125rem; }
  .nr-rating-md { --nr-star: 1.5rem; }
  .nr-rating-lg { --nr-star: 2rem; }
  .nr-rating .nr-icon { width: var(--nr-star, 1.5rem); height: var(--nr-star, 1.5rem); fill: currentColor; }
  .nr-rating-legend { padding: 0; color: var(--nr-fg); font-size: 0.875rem; font-weight: 500; }
  .nr-rating-items { display: inline-flex; align-self: flex-start; }
  .nr-rating-item { position: relative; display: flex; padding: 0.125rem; border-radius: var(--nr-radius-sm); color: var(--nr-content-4); cursor: pointer; transition: color 0.12s, transform 0.12s var(--nr-ease); }
  .nr-rating-input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: inherit; }
  .nr-rating-item:where(:has(.nr-rating-input:checked), :has(~ .nr-rating-item .nr-rating-input:checked)) { color: var(--c); }
  .nr-rating-item:has(.nr-rating-input:focus-visible) { outline: 2px solid var(--nr-focus); outline-offset: 1px; }
  @media (hover: hover) {
    .nr-rating-items:hover .nr-rating-item { color: var(--nr-content-4); }
    .nr-rating-items:hover .nr-rating-item:hover, .nr-rating-items:hover .nr-rating-item:has(~ .nr-rating-item:hover) { color: var(--c); }
    .nr-rating-item:hover { transform: scale(1.12); }
  }
  .nr-rating-stars { position: relative; display: inline-flex; align-self: flex-start; color: var(--nr-content-4); }
  .nr-rating-fill { position: absolute; inset-block: 0; inset-inline-start: 0; display: flex; overflow: hidden; color: var(--c); }

  /* ---- Segmented control ------------------------------------------------- */
  .nr-segmented { display: inline-flex; gap: 0.125rem; box-sizing: border-box; width: fit-content; max-width: 100%; padding: 0.25rem; overflow-x: auto; border-radius: var(--r, var(--nr-radius-md)); background: var(--nr-content-2); font-family: var(--nr-font); scrollbar-width: none; }
  .nr-segmented-full { display: flex; width: auto; }
  .nr-segmented-full .nr-segment { flex: 1; }
  .nr-segmented-sm { --nr-seg-h: 1.625rem; --nr-seg-fs: 0.8125rem; }
  .nr-segmented-md { --nr-seg-h: 2rem; }
  .nr-segmented-lg { --nr-seg-h: 2.5rem; --nr-seg-fs: 1rem; }
  .nr-segment { position: relative; display: flex; flex-shrink: 0; }
  .nr-segment-input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .nr-segment-input:disabled { cursor: not-allowed; }
  .nr-segment-label { display: flex; align-items: center; justify-content: center; gap: 0.375rem; box-sizing: border-box; width: 100%; height: var(--nr-seg-h, 2rem); padding: 0 0.875rem; border-radius: calc(var(--r, var(--nr-radius-md)) - 0.25rem); color: var(--nr-muted); font-size: var(--nr-seg-fs, 0.875rem); font-weight: 500; white-space: nowrap; transition: background-color 0.15s, color 0.15s, box-shadow 0.15s; }
  .nr-segment-label .nr-icon { width: 1rem; height: 1rem; }
  .nr-segment-input:hover + .nr-segment-label { color: var(--nr-fg); }
  .nr-segment-input:checked + .nr-segment-label { background: var(--nr-content); color: var(--c-text); box-shadow: var(--nr-shadow-sm); }
  .nr-segment-input:focus-visible + .nr-segment-label { outline: 2px solid var(--nr-focus); outline-offset: -2px; }
  .nr-segment-input:disabled + .nr-segment-label { opacity: 0.5; }

  /* ---- File upload, color picker, date range ----------------------------- */
  .nr-upload { display: flex; flex-direction: column; gap: 0.375rem; min-width: 0; font-family: var(--nr-font); }
  .nr-upload[data-disabled] { opacity: 0.5; pointer-events: none; }
  .nr-dropzone { position: relative; display: flex; flex-direction: column; align-items: center; gap: 0.25rem; box-sizing: border-box; padding: 1.75rem 1rem; border: 2px dashed var(--nr-content-4); border-radius: var(--nr-radius-lg); color: var(--nr-fg); text-align: center; transition: border-color 0.15s, background-color 0.15s; }
  .nr-dropzone:hover, .nr-dropzone[data-drag] { border-color: var(--c); background: color-mix(in oklab, var(--c) 8%, transparent); }
  .nr-dropzone:has(.nr-dropzone-input:focus-visible) { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-dropzone-icon { display: flex; margin-bottom: 0.25rem; color: var(--nr-muted); }
  .nr-dropzone-icon .nr-icon { width: 2rem; height: 2rem; }
  .nr-dropzone-title { font-size: 0.875rem; font-weight: 500; }
  .nr-dropzone-hint { color: var(--nr-muted); font-size: 0.75rem; }
  .nr-dropzone-input { max-width: 100%; margin-top: 0.625rem; color: var(--nr-muted); font: inherit; font-size: 0.8125rem; }
  .nr-dropzone-input:where(.nr-ui-js *) { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .nr-dropzone-files { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.25rem; margin: 0.625rem 0 0; padding: 0; list-style: none; }
  .nr-dropzone-files[hidden] { display: none; }
  .nr-dropzone-files li { max-width: 100%; padding: 0.125rem 0.5rem; overflow: hidden; border-radius: 999px; background: var(--nr-content-2); color: var(--nr-fg); font-size: 0.75rem; text-overflow: ellipsis; white-space: nowrap; }
  .nr-color { display: inline-flex; align-items: center; gap: 0.75rem; font-family: var(--nr-font); cursor: pointer; }
  .nr-color[data-disabled] { opacity: 0.5; pointer-events: none; }
  .nr-color-sm { --nr-sw: 2rem; }
  .nr-color-md { --nr-sw: 2.5rem; }
  .nr-color-lg { --nr-sw: 3rem; }
  .nr-color-input { box-sizing: border-box; flex-shrink: 0; width: var(--nr-sw, 2.5rem); height: var(--nr-sw, 2.5rem); margin: 0; padding: 0; overflow: hidden; border: 2px solid var(--nr-content-3); border-radius: var(--nr-radius-md); background: none; cursor: pointer; appearance: none; -webkit-appearance: none; }
  .nr-color-input::-webkit-color-swatch-wrapper { padding: 0; }
  .nr-color-input::-webkit-color-swatch { border: 0; }
  .nr-color-input::-moz-color-swatch { border: 0; }
  .nr-color-input:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-color-text { display: flex; flex-direction: column; gap: 0.125rem; }
  .nr-color-label { color: var(--nr-fg); font-size: 0.875rem; font-weight: 500; }
  .nr-color-value { color: var(--nr-muted); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.75rem; text-transform: uppercase; }
  .nr-color-value:not(:where(.nr-ui-js *)) { display: none; }
  .nr-daterange { display: flex; flex-direction: column; gap: 0.375rem; min-width: 0; font-family: var(--nr-font); }
  .nr-daterange-fields { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: center; gap: 0.5rem; }
  .nr-daterange-sep { display: flex; color: var(--nr-muted); }
  .nr-daterange-sep .nr-icon { width: 1rem; height: 1rem; }
  .nr-daterange-sep:dir(rtl) { transform: scaleX(-1); }
  @media (max-width: 30rem) {
    .nr-daterange-fields { grid-template-columns: minmax(0, 1fr); }
    .nr-daterange-sep { display: none; }
  }

  /* ---- Form layout ------------------------------------------------------- */
  .nr-formfield { display: flex; flex-direction: column; gap: 0.375rem; min-width: 0; font-family: var(--nr-font); }
  .nr-formfield[data-invalid] > .nr-field-label { color: var(--nr-danger); }
  .nr-formfield-main { display: flex; flex-direction: column; gap: 0.375rem; min-width: 0; }
  @media (min-width: 40rem) {
    .nr-formfield-row { display: grid; grid-template-columns: minmax(0, 12rem) minmax(0, 1fr); gap: 1rem; align-items: start; }
    .nr-formfield-row > .nr-field-label { padding-top: 0.625rem; }
  }
  .nr-formsection { display: grid; gap: 1.25rem; padding-block: 2rem; border-top: 1px solid var(--nr-divider); font-family: var(--nr-font); }
  .nr-formsection:first-child { padding-top: 0; border-top: 0; }
  .nr-formsection-title { margin: 0; color: var(--nr-fg); font-size: 1rem; font-weight: 600; }
  .nr-formsection-description { margin: 0.25rem 0 0; color: var(--nr-muted); font-size: 0.875rem; line-height: 1.5; }
  .nr-formsection-body { display: flex; flex-direction: column; gap: 1rem; min-width: 0; }
  @media (min-width: 48rem) {
    .nr-formsection { grid-template-columns: minmax(0, 1fr) minmax(0, 2fr); gap: 2.5rem; }
  }
  .nr-formactions { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0.75rem; }
  .nr-formactions-start { justify-content: flex-start; }
  .nr-formactions-center { justify-content: center; }
  .nr-formactions-between { justify-content: space-between; }
  .nr-formactions-bordered { padding-top: 1.25rem; border-top: 1px solid var(--nr-divider); }

  /* ---- Hero and sections ------------------------------------------------- */
  .nr-hero { position: relative; isolation: isolate; box-sizing: border-box; padding: clamp(3rem, 8vw, 6rem) 1rem; overflow: hidden; color: var(--nr-fg); font-family: var(--nr-font); text-align: center; }
  .nr-hero-glow::before { content: ""; position: absolute; top: -35%; right: 10%; left: 10%; z-index: -1; height: 75%; background: radial-gradient(closest-side, color-mix(in oklab, var(--c) 30%, transparent), transparent); filter: blur(40px); pointer-events: none; }
  .nr-hero-inner { display: flex; flex-direction: column; align-items: center; gap: clamp(2rem, 5vw, 3.5rem); max-width: 72rem; margin-inline: auto; }
  .nr-hero-text { display: flex; flex-direction: column; align-items: center; gap: 1.25rem; max-width: 48rem; }
  .nr-hero-eyebrow { margin: 0; padding: 0.25rem 0.75rem; border-radius: 999px; background: color-mix(in oklab, var(--c) 14%, transparent); color: var(--c-text); font-size: 0.8125rem; font-weight: 600; }
  .nr-hero-title { margin: 0; font-size: clamp(2.25rem, 6vw, 4rem); font-weight: 800; line-height: 1.05; letter-spacing: -0.03em; text-wrap: balance; }
  .nr-hero-subtitle { max-width: 40rem; margin: 0; color: var(--nr-muted); font-size: clamp(1.0625rem, 2vw, 1.25rem); line-height: 1.6; text-wrap: pretty; }
  .nr-hero-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.75rem; margin-top: 0.5rem; }
  .nr-hero-media { width: 100%; min-width: 0; }
  .nr-hero-media > img { display: block; max-width: 100%; height: auto; margin-inline: auto; border-radius: var(--nr-radius-lg); }
  .nr-hero-start { text-align: start; }
  .nr-hero-start .nr-hero-inner, .nr-hero-start .nr-hero-text { align-items: flex-start; }
  .nr-hero-start .nr-hero-actions { justify-content: flex-start; }
  @media (min-width: 64rem) {
    .nr-hero-split .nr-hero-inner { flex-direction: row; align-items: center; }
    .nr-hero-split .nr-hero-text, .nr-hero-split .nr-hero-media { flex: 1; }
  }
  .nr-section { box-sizing: border-box; padding-block: clamp(3rem, 7vw, 5.5rem); color: var(--nr-fg); font-family: var(--nr-font); }
  .nr-section-compact { padding-block: clamp(2rem, 4vw, 3rem); }
  .nr-section-muted { background: var(--nr-content-2); }
  .nr-section-head { display: flex; flex-direction: column; align-items: center; gap: 0.75rem; max-width: 44rem; margin: 0 auto clamp(2rem, 4vw, 3.5rem); text-align: center; }
  .nr-section-start .nr-section-head { align-items: flex-start; margin-inline: 0; text-align: start; }
  .nr-section-eyebrow { margin: 0; color: var(--c-text); font-size: 0.875rem; font-weight: 600; letter-spacing: 0.02em; }
  .nr-section-title { margin: 0; font-size: clamp(1.75rem, 4vw, 2.5rem); font-weight: 700; line-height: 1.15; letter-spacing: -0.02em; text-wrap: balance; }
  .nr-section-subtitle { margin: 0; color: var(--nr-muted); font-size: 1.0625rem; line-height: 1.6; text-wrap: pretty; }

  /* ---- Features, pricing, testimonials, logos, call to action ------------ */
  .nr-features { gap: 2rem; font-family: var(--nr-font); }
  .nr-feature { display: flex; flex-direction: column; align-items: flex-start; gap: 0.625rem; min-width: 0; color: var(--nr-fg); text-decoration: none; }
  .nr-feature-card { padding: 1.5rem; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-lg); background: var(--nr-content); transition: border-color 0.2s, box-shadow 0.2s, transform 0.2s var(--nr-ease); }
  .nr-feature-card[href]:hover { border-color: var(--c); box-shadow: var(--nr-shadow-md); transform: translateY(-2px); }
  .nr-feature[href]:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 4px; border-radius: var(--nr-radius-lg); }
  .nr-feature-icon { display: grid; place-items: center; width: 2.75rem; height: 2.75rem; margin-bottom: 0.25rem; border-radius: var(--nr-radius-md); background: color-mix(in oklab, var(--c) 14%, transparent); color: var(--c-text); font-size: 1.25rem; }
  .nr-feature-title { margin: 0; font-size: 1.0625rem; font-weight: 600; line-height: 1.3; }
  .nr-feature-text { color: var(--nr-muted); font-size: 0.9375rem; line-height: 1.6; }
  .nr-feature-text > :first-child { margin-top: 0; }
  .nr-feature-text > :last-child { margin-bottom: 0; }
  .nr-pricing { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(16rem, 100%), 1fr)); gap: 1.5rem; align-items: stretch; font-family: var(--nr-font); }
  .nr-price-card { position: relative; display: flex; flex-direction: column; gap: 1rem; box-sizing: border-box; min-width: 0; padding: 1.75rem; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-lg); background: var(--nr-content); color: var(--nr-fg); font-family: var(--nr-font); }
  .nr-price-card-featured { border-color: var(--c); box-shadow: 0 0 0 1px var(--c), 0 18px 40px -18px color-mix(in oklab, var(--c) 55%, transparent); }
  .nr-price-head { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
  .nr-price-name { margin: 0; font-size: 1.0625rem; font-weight: 600; }
  .nr-price-badge { padding: 0.125rem 0.625rem; border-radius: 999px; background: color-mix(in oklab, var(--c) 16%, transparent); color: var(--c-text); font-size: 0.75rem; font-weight: 600; }
  .nr-price-amount { display: flex; align-items: baseline; gap: 0.25rem; margin: 0; }
  .nr-price-value { font-size: 2.5rem; font-weight: 800; line-height: 1; letter-spacing: -0.03em; }
  .nr-price-period { color: var(--nr-muted); font-size: 0.9375rem; }
  .nr-price-description { margin: 0; color: var(--nr-muted); font-size: 0.9375rem; line-height: 1.5; }
  .nr-price-features { display: flex; flex-direction: column; gap: 0.625rem; margin: 0.25rem 0 0; padding: 1.25rem 0 0; border-top: 1px solid var(--nr-divider); list-style: none; font-size: 0.9375rem; }
  .nr-price-feature { display: flex; align-items: flex-start; gap: 0.625rem; line-height: 1.4; }
  .nr-price-feature .nr-icon { width: 1.125rem; height: 1.125rem; margin-top: 0.0625rem; color: var(--c-ink); }
  .nr-price-feature-off { color: var(--nr-muted); }
  .nr-price-feature-off .nr-icon { color: var(--nr-subtle); }
  .nr-testimonial { display: flex; flex-direction: column; gap: 1rem; box-sizing: border-box; min-width: 0; margin: 0; color: var(--nr-fg); font-family: var(--nr-font); }
  .nr-testimonial-card { padding: 1.5rem; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-lg); background: var(--nr-content); }
  .nr-testimonial-quote { flex: 1; margin: 0; font-size: 1rem; line-height: 1.65; text-wrap: pretty; }
  .nr-testimonial-quote > :first-child { margin-top: 0; }
  .nr-testimonial-quote > :last-child { margin-bottom: 0; }
  .nr-testimonial-author { display: flex; align-items: center; gap: 0.75rem; }
  .nr-testimonial-who { display: flex; flex: 1; flex-direction: column; min-width: 0; font-size: 0.875rem; line-height: 1.35; }
  .nr-testimonial-name { font-weight: 600; }
  .nr-testimonial-role { color: var(--nr-muted); }
  .nr-testimonial-logo { display: flex; flex-shrink: 0; color: var(--nr-muted); }
  .nr-logos { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; font-family: var(--nr-font); }
  .nr-logos-title { margin: 0; color: var(--nr-muted); font-size: 0.875rem; font-weight: 500; text-align: center; }
  .nr-logos-list { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 1.5rem 3rem; margin: 0; padding: 0; list-style: none; }
  .nr-logos-item { display: flex; align-items: center; color: var(--nr-muted); font-size: 1.25rem; font-weight: 700; letter-spacing: -0.01em; opacity: 0.75; filter: grayscale(1); transition: opacity 0.2s, filter 0.2s, color 0.2s; }
  .nr-logos-item:hover { color: var(--nr-fg); opacity: 1; filter: none; }
  .nr-logos-item img, .nr-logos-item svg { display: block; width: auto; max-width: 9rem; height: 2rem; }
  .nr-cta { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; box-sizing: border-box; padding: clamp(2rem, 5vw, 3.5rem); border-radius: calc(var(--nr-radius-lg) * 1.5); font-family: var(--nr-font); text-align: center; }
  .nr-cta-solid { background: linear-gradient(135deg, var(--c), color-mix(in oklab, var(--c) 72%, #000)); color: var(--c-fg); }
  .nr-cta-solid.nr-c-default { background: var(--nr-fg); color: var(--nr-bg); }
  .nr-cta-soft { background: color-mix(in oklab, var(--c) 12%, transparent); color: var(--nr-fg); }
  .nr-cta-solid .nr-btn-solid.nr-c-default { background: var(--nr-bg); color: var(--nr-fg); }
  .nr-cta-text { display: flex; flex-direction: column; gap: 0.625rem; max-width: 40rem; }
  .nr-cta-title { margin: 0; font-size: clamp(1.5rem, 3.5vw, 2.25rem); font-weight: 700; line-height: 1.15; letter-spacing: -0.02em; text-wrap: balance; }
  .nr-cta-description { margin: 0; font-size: 1.0625rem; line-height: 1.6; opacity: 0.85; }
  .nr-cta-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.75rem; }
  @media (min-width: 48rem) {
    .nr-cta-split { flex-direction: row; justify-content: space-between; text-align: start; }
    .nr-cta-split .nr-cta-actions { flex-shrink: 0; }
  }

  /* ---- Timeline ---------------------------------------------------------- */
  .nr-timeline { display: flex; flex-direction: column; margin: 0; padding: 0; list-style: none; font-family: var(--nr-font); }
  .nr-timeline-item { position: relative; display: flex; gap: 0.875rem; padding-bottom: 1.5rem; }
  .nr-timeline-item:last-child { padding-bottom: 0; }
  .nr-timeline-item:not(:last-child)::before { content: ""; position: absolute; top: 1.5rem; bottom: 0.25rem; inset-inline-start: calc(0.625rem - 1px); width: 2px; border-radius: 1px; background: var(--nr-content-3); }
  .nr-timeline-marker { display: grid; flex-shrink: 0; place-items: center; width: 1.25rem; height: 1.25rem; }
  .nr-timeline-marker::after { content: ""; width: 0.625rem; height: 0.625rem; border-radius: 50%; background: var(--c); box-shadow: 0 0 0 3px color-mix(in oklab, var(--c) 25%, transparent); }
  .nr-timeline-marker[data-icon] { border-radius: 50%; background: color-mix(in oklab, var(--c) 18%, transparent); color: var(--c-text); }
  .nr-timeline-marker[data-icon]::after { content: none; }
  .nr-timeline-marker .nr-icon, .nr-timeline-marker svg { width: 0.75rem; height: 0.75rem; }
  .nr-timeline-body { flex: 1; min-width: 0; }
  .nr-timeline-head { display: flex; flex-wrap: wrap; align-items: baseline; justify-content: space-between; gap: 0.125rem 0.75rem; line-height: 1.25rem; }
  .nr-timeline-title { color: var(--nr-fg); font-size: 0.9375rem; font-weight: 600; }
  .nr-timeline-time { color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-timeline-text { margin-top: 0.25rem; color: var(--nr-muted); font-size: 0.875rem; line-height: 1.55; }
  .nr-timeline-text > :first-child { margin-top: 0; }
  .nr-timeline-text > :last-child { margin-bottom: 0; }

  /* ---- Carousel and marquee ---------------------------------------------- */
  .nr-carousel { --nr-per: 1; position: relative; min-width: 0; font-family: var(--nr-font); }
  @media (min-width: 48rem) {
    .nr-carousel-2, .nr-carousel-3, .nr-carousel-4 { --nr-per: 2; }
  }
  @media (min-width: 64rem) {
    .nr-carousel-3 { --nr-per: 3; }
    .nr-carousel-4 { --nr-per: 4; }
  }
  .nr-carousel-viewport { display: flex; gap: var(--nr-carousel-gap, 1rem); overflow-x: auto; border-radius: var(--nr-radius-lg); outline: none; scroll-behavior: smooth; scroll-snap-type: x mandatory; overscroll-behavior-x: contain; scrollbar-width: none; }
  .nr-carousel-viewport::-webkit-scrollbar { display: none; }
  .nr-carousel-viewport:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-carousel-item { display: flex; flex-direction: column; box-sizing: border-box; flex: 0 0 calc((100% - (var(--nr-per) - 1) * var(--nr-carousel-gap, 1rem)) / var(--nr-per)); min-width: 0; scroll-snap-align: start; }
  .nr-carousel-item > * { flex: 1; }
  .nr-carousel-peek .nr-carousel-item { flex-basis: calc((100% - (var(--nr-per) - 1) * var(--nr-carousel-gap, 1rem)) / var(--nr-per) * 0.88); }
  .nr-carousel-item > img { display: block; width: 100%; height: auto; border-radius: var(--nr-radius-lg); }
  .nr-carousel-btn { position: absolute; top: 50%; z-index: 1; display: grid; place-items: center; width: 2.5rem; height: 2.5rem; padding: 0; border: 1px solid var(--nr-divider); border-radius: 50%; background: var(--nr-content); color: var(--nr-fg); box-shadow: var(--nr-shadow-sm); cursor: pointer; transform: translateY(-50%); transition: opacity 0.2s, background-color 0.2s; }
  .nr-carousel-btn:not(:where(.nr-ui-js *)) { display: none; }
  .nr-carousel-btn:hover { background: var(--nr-content-2); }
  .nr-carousel-btn:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-carousel-btn[aria-disabled="true"] { opacity: 0.35; cursor: default; }
  .nr-carousel-btn:dir(rtl) .nr-icon { transform: scaleX(-1); }
  /* Phones swipe. */
  @media (max-width: 39.99rem) { .nr-carousel-btn { display: none; } }
  .nr-carousel-prev { inset-inline-start: 0.5rem; }
  .nr-carousel-next { inset-inline-end: 0.5rem; }
  .nr-marquee { display: flex; gap: var(--nr-marquee-gap, 3rem); overflow: hidden; direction: ltr; font-family: var(--nr-font); }
  .nr-marquee-fade { mask-image: linear-gradient(to right, transparent, #000 8%, #000 92%, transparent); -webkit-mask-image: linear-gradient(to right, transparent, #000 8%, #000 92%, transparent); }
  .nr-marquee-track { display: flex; flex-shrink: 0; align-items: center; justify-content: space-around; gap: var(--nr-marquee-gap, 3rem); min-width: 100%; animation: nr-marquee var(--nr-marquee-d, 40s) linear infinite; }
  .nr-marquee-reverse .nr-marquee-track { animation-direction: reverse; }
  .nr-marquee:hover .nr-marquee-track, .nr-marquee:focus-within .nr-marquee-track { animation-play-state: paused; }

  /* ---- Bento ------------------------------------------------------------- */
  .nr-bento { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1rem; font-family: var(--nr-font); }
  @media (min-width: 48rem) {
    .nr-bento { grid-auto-rows: minmax(11rem, auto); grid-auto-flow: dense; grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .nr-bento-c2, .nr-bento-c3, .nr-bento-c4 { grid-column: span 2; }
    .nr-bento-r2 { grid-row: span 2; }
    .nr-bento-r3 { grid-row: span 3; }
  }
  @media (min-width: 64rem) {
    .nr-bento-3 { grid-template-columns: repeat(3, minmax(0, 1fr)); }
    .nr-bento-4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
    .nr-bento-3 > .nr-bento-c3, .nr-bento-3 > .nr-bento-c4 { grid-column: span 3; }
    .nr-bento-4 > .nr-bento-c3 { grid-column: span 3; }
    .nr-bento-4 > .nr-bento-c4 { grid-column: span 4; }
  }
  .nr-bento-item { display: flex; flex-direction: column; gap: 1rem; box-sizing: border-box; min-width: 0; padding: 1.5rem; overflow: hidden; border: 1px solid var(--nr-divider); border-radius: calc(var(--nr-radius-lg) * 1.25); background: var(--nr-content); color: var(--nr-fg); text-decoration: none; transition: border-color 0.2s, box-shadow 0.2s, transform 0.2s var(--nr-ease); }
  .nr-bento-item[href]:hover { border-color: var(--nr-content-4); box-shadow: var(--nr-shadow-md); transform: translateY(-2px); }
  .nr-bento-item[href]:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-bento-title { margin: 0; font-size: 1.0625rem; font-weight: 600; line-height: 1.3; }
  .nr-bento-description { margin: 0.25rem 0 0; color: var(--nr-muted); font-size: 0.9375rem; line-height: 1.55; }
  .nr-bento-content { display: flex; flex: 1; flex-direction: column; justify-content: flex-end; min-width: 0; min-height: 0; }

  /* ---- Footer ------------------------------------------------------------ */
  .nr-footer { box-sizing: border-box; padding: 3rem 1rem 2rem; border-top: 1px solid var(--nr-divider); color: var(--nr-fg); font-family: var(--nr-font); font-size: 0.875rem; }
  .nr-footer-top { display: grid; gap: 2.5rem; max-width: 72rem; margin-inline: auto; }
  @media (min-width: 48rem) {
    .nr-footer { padding-inline: 1.5rem; }
    .nr-footer-top:has(> .nr-footer-about) { grid-template-columns: minmax(0, 1.2fr) minmax(0, 2.8fr); gap: 4rem; }
  }
  .nr-footer-about { display: flex; flex-direction: column; gap: 0.75rem; max-width: 22rem; }
  .nr-footer-brand { font-size: 1.125rem; font-weight: 700; }
  .nr-footer-brand a { color: inherit; text-decoration: none; }
  .nr-footer-description { margin: 0; color: var(--nr-muted); line-height: 1.6; }
  .nr-footer-social { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.25rem; }
  .nr-footer-columns { display: grid; grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr)); gap: 2rem 1.5rem; }
  .nr-footer-heading { margin: 0 0 0.875rem; color: var(--nr-fg); font-size: 0.8125rem; font-weight: 600; }
  .nr-footer-links { display: flex; flex-direction: column; gap: 0.625rem; margin: 0; padding: 0; list-style: none; }
  .nr-footer-link { color: var(--nr-muted); text-decoration: none; transition: color 0.15s; }
  .nr-footer-link:hover { color: var(--nr-fg); }
  .nr-footer-link:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; border-radius: 0.25rem; }
  .nr-footer-bottom { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 0.75rem 1.5rem; max-width: 72rem; margin: 2.5rem auto 0; padding-top: 1.5rem; border-top: 1px solid var(--nr-divider); color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-footer-end { display: flex; flex-wrap: wrap; align-items: center; gap: 1rem; }
  .nr-footer-end a { color: inherit; }

  /* ---- Pagination -------------------------------------------------------- */
  .nr-pagination { font-family: var(--nr-font); }
  .nr-pagination-sm { --nr-pg: 2rem; --nr-pg-fs: 0.8125rem; }
  .nr-pagination-md { --nr-pg: 2.25rem; }
  .nr-pagination-lg { --nr-pg: 2.75rem; --nr-pg-fs: 1rem; }
  .nr-pagination-list { display: flex; flex-wrap: wrap; align-items: center; gap: 0.25rem; margin: 0; padding: 0; list-style: none; }
  .nr-page { display: inline-grid; place-items: center; box-sizing: border-box; min-width: var(--nr-pg, 2.25rem); height: var(--nr-pg, 2.25rem); padding: 0 0.5rem; border-radius: var(--nr-radius-sm); color: var(--nr-fg); font-size: var(--nr-pg-fs, 0.875rem); font-weight: 500; font-variant-numeric: tabular-nums; text-decoration: none; transition: background-color 0.15s; }
  .nr-page[href]:hover { background: var(--nr-content-2); }
  .nr-page:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }
  .nr-page-current, .nr-page-current[href]:hover { background: var(--c); color: var(--c-fg); }
  .nr-page-step { padding: 0; }
  .nr-page-step[aria-disabled] { opacity: 0.4; }
  .nr-page-step .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-page-step:dir(rtl) .nr-icon { transform: scaleX(-1); }
  .nr-page-gap { min-width: 1.5rem; padding: 0; color: var(--nr-muted); }
  .nr-page-status { padding: 0 0.75rem; color: var(--nr-muted); font-size: 0.875rem; }

  /* ---- Layout helpers ---------------------------------------------------- */
  .nr-aspect { position: relative; width: 100%; overflow: hidden; border-radius: var(--r, 0); }
  .nr-aspect > * { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; object-fit: cover; }
  .nr-scroll { overflow: hidden auto; overscroll-behavior: contain; scrollbar-width: thin; scrollbar-color: var(--nr-content-4) transparent; }
  .nr-scroll-x { overflow: auto hidden; }
  .nr-scroll:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; border-radius: var(--nr-radius-sm); }
  .nr-spacer { flex: 1 1 var(--nr-space, 0px); align-self: stretch; min-width: var(--nr-space, 0px); min-height: var(--nr-space, 0px); }
  .nr-spacer[style] { flex: 0 0 var(--nr-space); }
  .nr-show-sm, .nr-show-md, .nr-show-lg, .nr-show-xl, .nr-hide-sm, .nr-hide-md, .nr-hide-lg, .nr-hide-xl { display: contents; }
  @media (max-width: 39.99rem) { .nr-show-sm { display: none; } }
  @media (max-width: 47.99rem) { .nr-show-md { display: none; } }
  @media (max-width: 63.99rem) { .nr-show-lg { display: none; } }
  @media (max-width: 79.99rem) { .nr-show-xl { display: none; } }
  @media (min-width: 40rem) { .nr-hide-sm { display: none; } }
  @media (min-width: 48rem) { .nr-hide-md { display: none; } }
  @media (min-width: 64rem) { .nr-hide-lg { display: none; } }
  @media (min-width: 80rem) { .nr-hide-xl { display: none; } }
  .nr-sticky { position: sticky; top: var(--nr-sticky-top, 0px); z-index: 40; background: var(--nr-bg); }
  .nr-sticky-blur { background: color-mix(in oklab, var(--nr-bg) 78%, transparent); backdrop-filter: saturate(1.6) blur(12px); -webkit-backdrop-filter: saturate(1.6) blur(12px); }
  .nr-sticky-border { border-bottom: 1px solid var(--nr-divider); }

  /* ---- Table helpers ----------------------------------------------------- */
  .nr-th-sort-link { display: inline-flex; align-items: center; gap: 0.25rem; color: inherit; text-decoration: none; }
  .nr-th-sort-link:hover { color: var(--nr-fg); }
  .nr-th-sort-link:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; border-radius: 0.25rem; }
  .nr-th-sort-link .nr-icon { width: 0.875rem; height: 0.875rem; opacity: 0.6; }
  .nr-th-sort:not([aria-sort="none"]) .nr-th-sort-link { color: var(--nr-fg); }
  .nr-th-sort:not([aria-sort="none"]) .nr-icon { opacity: 1; }
  .nr-table-check { width: 1%; padding-inline-end: 0; }
  .nr-table-checkbox { display: block; width: 1rem; height: 1rem; margin: 0; accent-color: var(--nr-primary); cursor: pointer; }
  .nr-table-checkbox:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 2px; }

  /* ---- List, description list, chip group -------------------------------- */
  .nr-list { display: flex; flex-direction: column; margin: 0; padding: 0; list-style: none; font-family: var(--nr-font); }
  .nr-list-bordered { overflow: hidden; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-lg); background: var(--nr-content); }
  .nr-list-divided > .nr-list-item + .nr-list-item { border-top: 1px solid var(--nr-divider); }
  .nr-list-row { display: flex; align-items: center; gap: 0.875rem; box-sizing: border-box; padding: 0.75rem 0; color: var(--nr-fg); text-decoration: none; }
  .nr-list-row:where(.nr-list-bordered *) { padding-inline: 1rem; }
  .nr-list-row:where(.nr-list-compact *) { padding-block: 0.5rem; }
  .nr-list-row[href] { transition: background-color 0.15s; }
  .nr-list-row[href]:hover { background: var(--nr-content-2); }
  .nr-list-row[href]:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: -2px; }
  .nr-list-start { display: flex; flex-shrink: 0; color: var(--nr-muted); }
  .nr-list-main { display: flex; flex: 1; flex-direction: column; gap: 0.0625rem; min-width: 0; }
  .nr-list-title { overflow: hidden; font-size: 0.9375rem; font-weight: 500; text-overflow: ellipsis; white-space: nowrap; }
  .nr-list-description { overflow: hidden; color: var(--nr-muted); font-size: 0.8125rem; text-overflow: ellipsis; white-space: nowrap; }
  .nr-list-end { display: flex; flex-shrink: 0; align-items: center; gap: 0.5rem; color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-dl { display: grid; gap: 1rem 2rem; margin: 0; font-family: var(--nr-font); font-size: 0.9375rem; }
  .nr-dl-item { display: flex; flex-direction: column; gap: 0.125rem; min-width: 0; }
  .nr-dl-term { color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-dl-value { margin: 0; color: var(--nr-fg); font-weight: 500; overflow-wrap: anywhere; }
  .nr-dl-row { gap: 0; }
  .nr-dl-row > .nr-dl-item { flex-direction: row; justify-content: space-between; gap: 1.5rem; padding-block: 0.75rem; border-top: 1px solid var(--nr-divider); }
  .nr-dl-row > .nr-dl-item:first-child { border-top: 0; }
  .nr-dl-row .nr-dl-term { font-size: inherit; }
  .nr-dl-row .nr-dl-value { text-align: end; }
  @media (min-width: 40rem) {
    .nr-dl-2, .nr-dl-3, .nr-dl-4 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (min-width: 64rem) {
    .nr-dl-3 { grid-template-columns: repeat(3, minmax(0, 1fr)); }
    .nr-dl-4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  }
  .nr-chip-group { display: flex; flex-wrap: wrap; align-items: center; gap: 0.5rem; }

  /* ---- Code -------------------------------------------------------------- */
  .nr-code { padding: 0.125em 0.375em; border: 1px solid var(--nr-divider); border-radius: 0.375rem; background: var(--nr-content-2); color: var(--nr-fg); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.875em; }
  .nr-codeblock { margin: 0; min-width: 0; overflow: hidden; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-lg); background: var(--nr-content-2); color: var(--nr-fg); font-family: var(--nr-font); }
  .nr-codeblock-head { display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem 1rem; border-bottom: 1px solid var(--nr-divider); color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-codeblock-title { flex: 1; min-width: 0; overflow: hidden; color: var(--nr-fg); font-weight: 500; text-overflow: ellipsis; white-space: nowrap; }
  .nr-codeblock-lang { margin-inline-start: auto; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.04em; }
  .nr-codeblock-body { position: relative; }
  .nr-codeblock-pre { margin: 0; padding: 1rem; overflow-x: auto; outline: none; background: transparent; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.8125rem; line-height: 1.65; tab-size: 4; scrollbar-width: thin; }
  .nr-codeblock-pre:focus-visible { box-shadow: inset 0 0 0 2px var(--nr-focus); }
  .nr-codeblock-pre code { padding: 0; border: 0; background: none; font: inherit; }
  .nr-codeblock-wrap .nr-codeblock-pre { white-space: pre-wrap; overflow-wrap: anywhere; }
  .nr-codeblock-copy { position: absolute; top: 0.5rem; inset-inline-end: 0.5rem; display: grid; place-items: center; width: 2rem; height: 2rem; padding: 0; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-sm); background: var(--nr-content); color: var(--nr-muted); cursor: pointer; opacity: 0; transition: opacity 0.15s, color 0.15s; }
  .nr-codeblock-copy:not(:where(.nr-ui-js *)) { display: none; }
  .nr-codeblock-copy:where(.nr-codeblock:hover *, .nr-codeblock:focus-within *), .nr-codeblock-copy[data-copied] { opacity: 1; }
  @media (hover: none) { .nr-codeblock-copy { opacity: 1; } }
  .nr-codeblock-copy:hover { color: var(--nr-fg); }
  .nr-codeblock-copy:focus-visible { outline: 2px solid var(--nr-focus); outline-offset: 1px; }
  .nr-codeblock-copy .nr-icon { width: 1rem; height: 1rem; }
  .nr-codeblock-copy-done { display: none; color: var(--nr-success-text); }
  .nr-codeblock-copy[data-copied] .nr-codeblock-copy-idle { display: none; }
  .nr-codeblock-copy[data-copied] .nr-codeblock-copy-done { display: block; }

  /* ---- Callout, banner --------------------------------------------------- */
  .nr-callout { box-sizing: border-box; margin: 0; padding: 0.875rem 1rem; border: 1px solid color-mix(in oklab, var(--c) 35%, transparent); border-inline-start: 4px solid var(--c); border-radius: var(--nr-radius-md); background: color-mix(in oklab, var(--c) 8%, transparent); color: var(--nr-fg); font-family: var(--nr-font); font-size: 0.9375rem; line-height: 1.6; }
  .nr-callout-title { display: flex; align-items: center; gap: 0.5rem; margin: 0 0 0.25rem; color: var(--c-text); font-weight: 600; }
  .nr-callout-title .nr-icon { width: 1.125rem; height: 1.125rem; }
  .nr-callout-text > :first-child { margin-top: 0; }
  .nr-callout-text > :last-child { margin-bottom: 0; }
  .nr-banner { position: relative; display: flex; align-items: center; justify-content: center; gap: 0.75rem; box-sizing: border-box; padding: 0.625rem 3rem; font-family: var(--nr-font); font-size: 0.875rem; line-height: 1.4; text-align: center; }
  .nr-banner:has(.nr-banner-x:checked) { display: none; }
  .nr-banner-solid { background: var(--c); color: var(--c-fg); }
  .nr-banner-solid.nr-c-default { background: var(--nr-fg); color: var(--nr-bg); }
  .nr-banner-soft { background: color-mix(in oklab, var(--c) 14%, transparent); color: var(--nr-fg); }
  .nr-banner-text { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 0.25rem 0.75rem; margin: 0; }
  .nr-banner-text .nr-icon { width: 1rem; height: 1rem; }
  .nr-banner-link { display: inline-flex; align-items: center; gap: 0.25rem; color: inherit; font-weight: 600; text-underline-offset: 3px; }
  .nr-banner-link:focus-visible { outline: 2px solid currentColor; outline-offset: 2px; border-radius: 0.25rem; }
  .nr-banner-link:dir(rtl) .nr-icon { transform: scaleX(-1); }
  .nr-banner-close { position: absolute; top: 50%; inset-inline-end: 0.5rem; display: grid; place-items: center; width: 2rem; height: 2rem; border-radius: 50%; cursor: pointer; opacity: 0.8; transform: translateY(-50%); }
  .nr-banner-close:hover { background: color-mix(in oklab, currentColor 16%, transparent); opacity: 1; }
  .nr-banner-close:has(.nr-banner-x:focus-visible) { outline: 2px solid currentColor; outline-offset: -2px; opacity: 1; }
  .nr-banner-x { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }

  /* ---- Meter ------------------------------------------------------------- */
  .nr-meter { display: flex; flex-direction: column; gap: 0.5rem; min-width: 0; font-family: var(--nr-font); font-size: 0.875rem; }
  .nr-meter-sm { --nr-meter-h: 0.375rem; }
  .nr-meter-md { --nr-meter-h: 0.625rem; }
  .nr-meter-lg { --nr-meter-h: 1rem; }
  .nr-meter-head { display: flex; align-items: baseline; justify-content: space-between; gap: 1rem; }
  .nr-meter-label { color: var(--nr-fg); font-weight: 500; }
  .nr-meter-value { color: var(--nr-muted); font-variant-numeric: tabular-nums; }
  .nr-meter-track { height: var(--nr-meter-h, 0.625rem); overflow: hidden; border-radius: 999px; background: var(--nr-content-3); }
  .nr-meter-bar { height: 100%; border-radius: inherit; background: var(--c); transition: width 0.4s var(--nr-ease); }
  .nr-meter-segmented { border-radius: 0.1875rem; mask-image: repeating-linear-gradient(to right, #000 0 calc(100% / var(--nr-meter-n, 5) - 3px), transparent 0 calc(100% / var(--nr-meter-n, 5))); -webkit-mask-image: repeating-linear-gradient(to right, #000 0 calc(100% / var(--nr-meter-n, 5) - 3px), transparent 0 calc(100% / var(--nr-meter-n, 5))); }
  .nr-meter-segmented .nr-meter-bar { border-radius: 0; }

  /* ---- Figure, video ----------------------------------------------------- */
  .nr-figure { display: flex; flex-direction: column; gap: 0.625rem; margin: 0; font-family: var(--nr-font); }
  .nr-figure-media { overflow: hidden; border-radius: var(--r, var(--nr-radius-lg)); background: var(--nr-content-2); }
  .nr-figure-bordered .nr-figure-media { border: 1px solid var(--nr-divider); }
  .nr-figure-img { display: block; width: 100%; height: auto; }
  .nr-figure-media[style] > * { display: block; width: 100%; height: 100%; object-fit: cover; }
  .nr-figure-caption { color: var(--nr-muted); font-size: 0.8125rem; line-height: 1.5; text-align: center; }
  .nr-video { position: relative; width: 100%; overflow: hidden; border-radius: var(--r, var(--nr-radius-lg)); background: #000; }
  .nr-video-frame { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; }

  /* ---- Result ------------------------------------------------------------ */
  .nr-result { display: flex; flex-direction: column; align-items: center; gap: 0.75rem; box-sizing: border-box; padding: clamp(2rem, 6vw, 4rem) 1.5rem; color: var(--nr-fg); font-family: var(--nr-font); text-align: center; }
  .nr-result-icon { display: grid; place-items: center; width: 4.5rem; height: 4.5rem; margin-bottom: 0.5rem; border-radius: 50%; background: color-mix(in oklab, var(--c) 15%, transparent); color: var(--c-ink); }
  .nr-result-icon .nr-icon { width: 2.25rem; height: 2.25rem; }
  .nr-result-code { margin: 0; color: var(--c-ink); font-size: clamp(3.5rem, 10vw, 6rem); font-weight: 800; line-height: 1; letter-spacing: -0.04em; }
  .nr-result-title { margin: 0; font-size: 1.5rem; font-weight: 700; line-height: 1.25; text-wrap: balance; }
  .nr-result-description { max-width: 32rem; margin: 0; color: var(--nr-muted); font-size: 1rem; line-height: 1.6; text-wrap: pretty; }
  .nr-result-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.75rem; margin-top: 1rem; }

  /* ---- Avatar status ----------------------------------------------------- */
  .nr-avatar-with-status { overflow: visible; }
  .nr-avatar-status { position: absolute; inset-block-end: 0; inset-inline-end: 0; box-sizing: border-box; width: 28%; min-width: 0.625rem; height: 28%; min-height: 0.625rem; border: 2px solid var(--nr-bg); border-radius: 50%; background: var(--c); }
  .nr-avatar-status.nr-c-default { background: var(--nr-subtle); }

  /* ---- Charts ------------------------------------------------------------ */
  .nr-series-1 { --s: var(--nr-chart-1); }
  .nr-series-2 { --s: var(--nr-chart-2); }
  .nr-series-3 { --s: var(--nr-chart-3); }
  .nr-series-4 { --s: var(--nr-chart-4); }
  .nr-series-5 { --s: var(--nr-chart-5); }
  .nr-series-6 { --s: var(--nr-chart-6); }
  .nr-series-7 { --s: var(--nr-chart-7); }
  .nr-series-8 { --s: var(--nr-chart-8); }
  .nr-sparkline { position: relative; display: inline-block; max-width: 100%; vertical-align: middle; }
  .nr-sparkline svg { display: block; width: 100%; height: 100%; overflow: visible; }
  .nr-sparkline-line { fill: none; stroke: var(--c-ink); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  .nr-sparkline-area { fill: color-mix(in oklab, var(--c) 16%, transparent); stroke: none; }
  .nr-sparkline-dot { position: absolute; inset-inline-end: 0; width: 0.5rem; height: 0.5rem; margin: -0.25rem -0.25rem 0 0; border-radius: 50%; background: var(--c-ink); box-shadow: 0 0 0 2px var(--nr-content); }
  .nr-chart { --nr-plot: var(--c-ink); display: flex; flex-direction: column; gap: 0.75rem; min-width: 0; margin: 0; color: var(--nr-fg); font-family: var(--nr-font); }
  .nr-chart-title { font-size: 0.9375rem; font-weight: 600; }
  .nr-chart-legend { display: flex; flex-wrap: wrap; gap: 0.375rem 1rem; margin: 0; padding: 0; list-style: none; color: var(--nr-muted); font-size: 0.8125rem; }
  .nr-chart-legend li { display: inline-flex; align-items: center; gap: 0.375rem; }
  .nr-chart-legend strong { color: var(--nr-fg); font-weight: 600; font-variant-numeric: tabular-nums; }
  .nr-chart-swatch { flex-shrink: 0; width: 0.625rem; height: 0.625rem; border-radius: 0.1875rem; background: var(--s, var(--nr-plot)); }
  .nr-chart-plot { position: relative; height: var(--nr-chart-h, 12rem); margin-block: 0.5rem 1.5rem; margin-inline-start: 2.5rem; }
  .nr-chart-grid span { position: absolute; right: 0; left: 0; height: 0; border-top: 1px solid var(--nr-divider); }
  .nr-chart-grid i { position: absolute; top: -0.5rem; inset-inline-end: calc(100% + 0.5rem); color: var(--nr-muted); font-size: 0.6875rem; font-style: normal; line-height: 1rem; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .nr-bars { position: absolute; inset: 0; display: flex; align-items: flex-end; gap: 2px; margin: 0; padding: 0; list-style: none; }
  .nr-bar-slot { position: relative; display: flex; flex: 1; align-items: flex-end; justify-content: center; min-width: 0; height: 100%; }
  .nr-bar { position: relative; width: min(100%, 1.5rem); min-height: 1px; border-radius: 4px 4px 0 0; background: var(--nr-plot); transition: filter 0.15s; }
  .nr-bar-slot:hover .nr-bar { filter: brightness(1.12); }
  .nr-bar-label { position: absolute; top: calc(100% + 0.375rem); right: 0; left: 0; overflow: hidden; color: var(--nr-muted); font-size: 0.6875rem; line-height: 1rem; text-align: center; text-overflow: ellipsis; white-space: nowrap; }
  .nr-bar-tip { position: absolute; bottom: calc(100% + 0.375rem); left: 50%; z-index: 2; padding: 0.25rem 0.5rem; border-radius: 0.375rem; background: var(--nr-fg); color: var(--nr-bg); font-size: 0.75rem; line-height: 1.3; white-space: nowrap; opacity: 0; transform: translateX(-50%); transition: opacity 0.12s; pointer-events: none; }
  .nr-bar-slot:hover .nr-bar-tip { opacity: 1; }
  .nr-bars-values .nr-bar-tip { padding: 0; background: none; color: var(--nr-muted); font-size: 0.6875rem; opacity: 1; }
  .nr-line-svg { position: absolute; inset: 0; width: 100%; height: 100%; overflow: visible; }
  .nr-line { fill: none; stroke: var(--s, var(--nr-plot)); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  .nr-line-area { fill: color-mix(in oklab, var(--nr-plot) 12%, transparent); stroke: none; }
  .nr-line-cols { position: absolute; inset: 0; margin: 0; padding: 0; list-style: none; }
  .nr-line-col { position: absolute; top: 0; bottom: 0; transform: translateX(-50%); }
  .nr-line-col::before { content: ""; position: absolute; top: 0; bottom: 0; left: 50%; border-inline-start: 1px solid var(--nr-content-4); opacity: 0; }
  .nr-line-col:hover::before { opacity: 1; }
  .nr-line-dot { position: absolute; left: 50%; width: 0.5rem; height: 0.5rem; margin: 0 0 -0.25rem -0.25rem; border-radius: 50%; background: var(--s, var(--nr-plot)); box-shadow: 0 0 0 2px var(--nr-content); opacity: 0; }
  .nr-line-col:hover .nr-line-dot { opacity: 1; }
  .nr-line-tip { position: absolute; top: 0; left: calc(50% + 0.625rem); z-index: 2; display: flex; flex-direction: column; gap: 0.125rem; padding: 0.375rem 0.625rem; border: 1px solid var(--nr-divider); border-radius: var(--nr-radius-sm); background: var(--nr-content); color: var(--nr-fg); font-size: 0.75rem; line-height: 1.35; white-space: nowrap; box-shadow: var(--nr-shadow-md); opacity: 0; pointer-events: none; }
  .nr-line-tip span { display: flex; align-items: center; gap: 0.375rem; color: var(--nr-muted); }
  .nr-line-col:hover .nr-line-tip { opacity: 1; }
  .nr-line-col:nth-last-child(-n + 3):not(:nth-child(-n + 2)) .nr-line-tip { right: calc(50% + 0.625rem); left: auto; }
  .nr-line-label { position: absolute; top: calc(100% + 0.375rem); left: 50%; color: var(--nr-muted); font-size: 0.6875rem; line-height: 1rem; white-space: nowrap; transform: translateX(-50%); }
  .nr-line-col-first .nr-line-label { transform: none; }
  .nr-line-col-last .nr-line-label { transform: translateX(-100%); }
  .nr-donut-body { display: flex; flex-wrap: wrap; align-items: center; gap: 1.25rem 2rem; }
  .nr-donut-body .nr-chart-legend { flex: 1; flex-direction: column; min-width: 9rem; max-width: 16rem; }
  .nr-donut-body .nr-chart-legend li { display: flex; }
  .nr-donut-body .nr-chart-legend strong { margin-inline-start: auto; padding-inline-start: 1rem; }
  .nr-donut-ring { position: relative; flex-shrink: 0; width: var(--nr-donut-d, 10rem); height: var(--nr-donut-d, 10rem); }
  .nr-donut-ring svg { display: block; width: 100%; height: 100%; transform: rotate(-90deg); }
  .nr-donut-track { fill: none; stroke: var(--nr-content-2); stroke-width: 5; }
  .nr-donut-seg { fill: none; stroke: var(--s); stroke-width: 5; transition: stroke-width 0.15s; }
  .nr-donut-seg:hover { stroke-width: 6; }
  .nr-donut-center { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--nr-muted); font-size: 0.75rem; line-height: 1.3; pointer-events: none; }
  .nr-donut-total { color: var(--nr-fg); font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

  /* ---- Radius (last, so it wins over component defaults) --------------- */
  .nr-r-none { --r: 0px; }
  .nr-r-sm { --r: var(--nr-radius-sm); }
  .nr-r-md { --r: var(--nr-radius-md); }
  .nr-r-lg { --r: var(--nr-radius-lg); }
  .nr-r-full { --r: 9999px; }

  @media (prefers-reduced-motion: reduce) {
    .nr-select-list, .nr-calendar { transition: none; }
    .nr-modal[open] { animation: none; }
    .nr-spinner-ring { animation-duration: 1.5s; }
    .nr-skeleton, .nr-skeleton-line, .nr-progress-indeterminate .nr-progress-bar { animation: none; }
    .nr-pop:popover-open, .nr-drawer[open], .nr-loading-veil { animation: none; }
    .nr-toaster .nr-toast { animation-name: none, nr-toast-out; }
    .nr-carousel-viewport { scroll-behavior: auto; }
    .nr-marquee { overflow-x: auto; }
    .nr-marquee-track { animation: none; }
    .nr-marquee-track[aria-hidden] { display: none; }
    .nr-meter-bar, .nr-rating-item { transition: none; }
  }
}

@keyframes nr-spin {
  to { transform: rotate(1turn); }
}

@keyframes nr-pop {
  from { opacity: 0; transform: scale(0.96) translateY(-4px); }
}

@keyframes nr-shimmer {
  from { background-position: 200% 0; }
  to { background-position: -200% 0; }
}

@keyframes nr-fade {
  from { opacity: 0; }
}

@keyframes nr-drawer-in {
  from { transform: var(--nr-drawer-from); }
}

@keyframes nr-toast-in {
  from { opacity: 0; transform: translateY(0.5rem) scale(0.96); }
}

@keyframes nr-toast-out {
  from { max-height: 12rem; }
  to { max-height: 0; margin-top: -0.5rem; padding-block: 0; border-width: 0; opacity: 0; visibility: hidden; }
}

@keyframes nr-marquee {
  to { transform: translateX(calc(-100% - var(--nr-marquee-gap, 3rem))); }
}

@keyframes nr-slide {
  from { transform: translateX(-250%); }
  to { transform: translateX(350%); }
}
"####;
