use next_rust::prelude::*;

use crate::User;

pub fn Page(auth: Auth<User>) -> impl View {
    h1![format!("Admin: {}", auth.user().map(|u| u.name.as_str()).unwrap_or("?"))]
}
