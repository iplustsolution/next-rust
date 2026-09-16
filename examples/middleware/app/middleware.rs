use next_rust::prelude::*;

/// Runs for every request before routing: rewrites and response headers.
pub async fn middleware(mut req: Request, next: Next) -> Response {
    if req.path() == "/home" {
        let _ = req.set_path("/");
    }
    next.run(req).await.with_header("x-powered-by", "next-rust")
}
