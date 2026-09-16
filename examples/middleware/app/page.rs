use next_rust::prelude::*;

pub fn Page(Extension(session): Extension<Session>) -> impl View {
    let views: u32 = session.get("views").unwrap_or(0) + 1;
    session.insert("views", views);
    div![h1!["Home"], p![format!("Session views: {views}")]]
}
