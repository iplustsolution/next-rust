use std::sync::atomic::{AtomicUsize, Ordering};

use next_rust::prelude::*;

pub static LIKES: AtomicUsize = AtomicUsize::new(0);

#[derive(serde::Deserialize)]
pub struct SignUp {
    name: String,
    email: String,
    password: String,
    #[serde(default)]
    plan: String,
}

/// Validation errors appear under the matching fields, with or without JavaScript.
#[server_action]
pub async fn sign_up(input: SignUp) -> Result<String> {
    let mut errors = Vec::new();
    if input.name.trim().is_empty() {
        errors.push(("name", "Tell us your name"));
    }
    if !input.email.contains('@') {
        errors.push(("email", "Enter a valid email address"));
    }
    if input.password.len() < 8 {
        errors.push(("password", "Use at least 8 characters"));
    }
    if input.plan.is_empty() {
        errors.push(("plan", "Choose a plan"));
    }
    if !errors.is_empty() {
        return Err(Error::validation(errors));
    }
    Ok(format!("Welcome, {}!", input.name.trim()))
}

#[server_action]
pub async fn like() -> Result<usize> {
    Ok(LIKES.fetch_add(1, Ordering::SeqCst) + 1)
}
