use next_rust::prelude::*;

use crate::User;

pub async fn middleware(req: Request, next: Next) -> Response {
    if req.extension::<AuthUser<User>>().is_none() {
        return Response::text("Unauthorized").with_status(401).with_header("www-authenticate", "Bearer");
    }
    next.run(req).await
}
