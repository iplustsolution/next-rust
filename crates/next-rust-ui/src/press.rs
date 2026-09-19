//! What happens when a component is pressed.

use next_rust_view::{Attr, attrs::raw_attr};

/// Behavior for `on_press`: call a server action, navigate, dispatch an
/// event, or run a script.
///
/// ```ignore
/// Button![on_press = action!(archive), "Archive"]                  // server action
/// Button![on_press = Press::from(action!(like)).input(&id), "Like"]
/// Button![on_press = Press::navigate("/settings"), "Settings"]
/// Button![on_press = Press::emit("open-cart"), "Cart"]              // `nr:open-cart` event
/// Button![on_press = Press::open_modal("confirm"), "Delete…"]       // opens `Modal![id = "confirm", ..]`
/// ```
///
/// After a successful action the page refreshes (see [`Press::no_refresh`]);
/// the element receives `nr:success` / `nr:error` events either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Press {
    kind: Kind,
    input: Option<String>,
    refresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Action(String),
    Navigate(String),
    Emit(String),
    Script(String),
    OpenModal(String),
    CloseModal,
}

impl Press {
    /// Call the server action at `url`. From a reference, use
    /// `Press::from(action!(save))` (or simply `on_press = action!(save)`).
    pub fn action(url: impl Into<String>) -> Self {
        Press { kind: Kind::Action(url.into()), input: None, refresh: true }
    }

    /// Client-side navigation to `href`.
    pub fn navigate(href: impl Into<String>) -> Self {
        Press { kind: Kind::Navigate(href.into()), input: None, refresh: true }
    }

    /// Dispatch a bubbling `nr:<name>` event from the element, for your own
    /// scripts: `addEventListener("nr:open-cart", …)`.
    pub fn emit(name: impl Into<String>) -> Self {
        Press { kind: Kind::Emit(name.into()), input: None, refresh: true }
    }

    /// Open the [`Modal`](struct@crate::Modal) with that `id` (as a modal dialog:
    /// focus trapped, page behind inert).
    pub fn open_modal(id: impl Into<String>) -> Self {
        Press { kind: Kind::OpenModal(id.into()), input: None, refresh: true }
    }

    /// Close the modal the element is inside.
    pub fn close_modal() -> Self {
        Press { kind: Kind::CloseModal, input: None, refresh: true }
    }

    /// Run a script (an `onclick` attribute). Trusted code only: never pass
    /// user input. Blocked by a strict Content Security Policy.
    pub fn script(js: impl Into<String>) -> Self {
        Press { kind: Kind::Script(js.into()), input: None, refresh: true }
    }

    /// The action's input, serialized as JSON.
    pub fn input(mut self, value: &impl serde::Serialize) -> Self {
        self.input = serde_json::to_string(value).ok();
        self
    }

    /// Keep the page as it is after a successful action.
    pub fn no_refresh(mut self) -> Self {
        self.refresh = false;
        self
    }

    pub(crate) fn is_action(&self) -> bool {
        matches!(self.kind, Kind::Action(_))
    }

    pub(crate) fn attrs(&self) -> Vec<Attr> {
        let (kind, target) = match &self.kind {
            Kind::Script(js) => return vec![raw_attr("onclick", js.clone())],
            Kind::Action(url) => ("action", url),
            Kind::Navigate(href) => ("navigate", href),
            Kind::Emit(name) => ("emit", name),
            Kind::OpenModal(id) => ("modal", id),
            Kind::CloseModal => {
                return vec![Attr::new("data-nr-ui", "press"), Attr::new("data-nr-press", "close-modal")];
            }
        };
        let mut attrs = vec![
            Attr::new("data-nr-ui", "press"),
            Attr::new("data-nr-press", kind),
            Attr::new("data-nr-press-target", target.clone()),
        ];
        if let Some(input) = &self.input {
            attrs.push(Attr::new("data-nr-press-input", input.clone()));
        }
        if !self.refresh {
            attrs.push(Attr::new("data-nr-press-refresh", "false"));
        }
        attrs
    }
}
