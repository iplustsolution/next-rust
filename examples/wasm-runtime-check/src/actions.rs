use std::sync::{LazyLock, Mutex};

use next_rust::prelude::*;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Entry {
    pub name: String,
    pub message: String,
}

pub static ENTRIES: LazyLock<Mutex<Vec<Entry>>> = LazyLock::new(Default::default);

#[derive(serde::Deserialize)]
pub struct EntryInput {
    name: String,
    message: String,
}

/// Works as a plain HTML form post and as a JSON call from the browser runtime.
#[server_action]
pub async fn sign(input: EntryInput) -> Result<Entry> {
    let mut errors = Vec::new();
    if input.name.trim().is_empty() {
        errors.push(("name", "Please tell us your name"));
    }
    if input.message.trim().len() < 3 {
        errors.push(("message", "Messages need at least 3 characters"));
    }
    if !errors.is_empty() {
        return Err(Error::validation(errors));
    }
    let entry = Entry { name: input.name.trim().to_owned(), message: input.message.trim().to_owned() };
    ENTRIES.lock().unwrap().push(entry.clone());
    Ok(entry)
}

#[server_action]
pub async fn count_entries() -> Result<usize> {
    Ok(ENTRIES.lock().unwrap().len())
}

/// An action that always redirects: `body.redirect` has to become a client
/// navigation, not a full page load.
#[server_action]
pub async fn go_about() -> Result<()> {
    Err(redirect("/about"))
}

#[derive(serde::Deserialize)]
pub struct Echo {
    text: String,
}

/// What the declarative `action:` op calls: the island's whole state arrives
/// as the input, and whatever comes back is written into the bound key.
#[server_action]
pub async fn echo_text(input: serde_json::Value) -> Result<String> {
    Ok(input.get("n").map(|n| n.to_string()).unwrap_or_else(|| "?".into()))
}

/// Plain success with data, for `window.nextRust.action(url, input)`.
#[server_action]
pub async fn echo(input: Echo) -> Result<String> {
    Ok(input.text.to_uppercase())
}
