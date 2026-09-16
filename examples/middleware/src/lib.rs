use next_rust::{
    App, AppBuilder, AuthUser, Cors, MemorySessionStore, Next, RateLimit, Request, cors, rate_limit, request_id,
    sessions,
};

next_rust::routes!();

#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
}

/// Global middleware stack; runs before app/middleware.rs.
pub fn app() -> AppBuilder {
    App::new(routes())
        .middleware(request_id())
        .middleware(cors(Cors::default().allow_origin("https://app.example.com").allow_credentials(true)))
        .middleware(rate_limit(
            RateLimit::per_minute(100).key(|req: &Request| req.header("x-api-key").unwrap_or("anonymous").to_owned()),
        ))
        .middleware(sessions(MemorySessionStore::default()))
        .middleware(authenticate)
}

/// Token authentication: attaches the user for pages (`Auth<User>`) and handlers.
async fn authenticate(mut req: Request, next: Next) -> next_rust::Response {
    if req.header("authorization") == Some("Bearer admin-token") {
        req.insert_extension(AuthUser(User { name: "admin".into() }));
    }
    next.run(req).await
}
