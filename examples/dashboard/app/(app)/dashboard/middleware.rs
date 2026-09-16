use next_rust::prelude::*;

/// Every route under /dashboard requires a signed-in user.
pub async fn middleware(req: Request, next: Next) -> Response {
    if req.cookies().get("user").is_none() {
        return Response::redirect("/login");
    }
    next.run(req).await
}
