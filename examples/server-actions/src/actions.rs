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

/// Works as a plain HTML form post and as a JSON call from the client runtime.
#[server_action]
pub async fn sign_guestbook(input: EntryInput) -> Result<Entry> {
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
