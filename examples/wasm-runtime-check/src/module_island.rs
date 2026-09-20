use next_rust::prelude::*;

/// An island whose behaviour comes from an ES module: the runtime has to
/// resolve `data-module` against this origin, import it and call its
/// `hydrate(element, props)` export, and fall back to the declarative
/// bindings when there is none.
#[client(script = "badge.js")]
pub fn Badge(label: String) -> impl View {
    div![id("badge"), span![id("badge-label"), label]]
}

// The module's source lives in Rust, not in a `.js` file: the build extracts
// it and serves it at /_next-rust/client/badge.js.
next_rust::client_script!(
    "badge.js",
    r##"
export function hydrate(element, props) {
  const label = element.querySelector("#badge-label");
  if (label) label.textContent = "hydrated:" + (props && props.label);
  element.setAttribute("data-hydrated", "module");
}
"##
);
