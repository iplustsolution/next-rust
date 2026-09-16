use next_rust::prelude::*;

/// Rendered on every request (it reads cookies, headers and the query).
pub const RENDERING: Rendering = Rendering::Dynamic;

pub fn Page(Query(query): Query, cookies: Cookies, headers: Headers, response: ResponseHeaders) -> impl View {
    let visits: u32 = cookies.get("visits").and_then(|v| v.parse().ok()).unwrap_or(0) + 1;
    cookies.set(Cookie::new("visits", visits.to_string()));
    response.set("x-visits", &visits.to_string());
    let agent = headers.get("user-agent").unwrap_or("unknown").to_owned();
    div![
        h1![format!("Hello, {}", query.get("name").unwrap_or("stranger"))],
        p![format!("Visit #{visits}")],
        p![class("agent"), agent],
    ]
}
