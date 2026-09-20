//! `AppBuilder::state`: one shared value reaches pages and programmatic
//! routes as `Extension<T>`.

use std::sync::Arc;

use bytes::Bytes;
use next_rust_core::{Config, Environment};
use next_rust_server::*;
use next_rust_view::*;

struct Shared {
    name: &'static str,
}

fn greeting(ctx: Ctx) -> BoxFuture<Result<Node>> {
    Box::pin(async move {
        let Extension(shared) = Extension::<Arc<Shared>>::from_context(&ctx)?;
        Ok(p![format!("hello {}", shared.name)].into_node())
    })
}

fn app(with_state: bool) -> App {
    let routes = Routes {
        pages: vec![PageDef {
            pattern: "/",
            source: "test",
            body: PageBody::Rust(greeting),
            segments: vec![SegmentDef::default()],
            metadata: None,
            rendering: Rendering::Dynamic,
            revalidate: None,
            generate_params: None,
            dynamic_params: true,
            tags: &[],
            intercept_from: None,
        }],
        ..Default::default()
    };
    let config =
        Config { root: std::env::temp_dir().join(format!("nr-state-{}", std::process::id())), ..Config::default() };
    let mut builder = App::new(routes).config(config).environment(Environment::Development);
    if with_state {
        builder = builder.state(Arc::new(Shared { name: "state" }));
    }
    builder
        .get("/api", |req: Request| async move {
            req.extensions().get::<Arc<Shared>>().map(|s| s.name).unwrap_or("none").to_owned()
        })
        .build()
        .unwrap()
}

async fn get(app: &App, uri: &str) -> (u16, String) {
    let res = app.handle(Request::from_http(http::Request::get(uri).body(Bytes::new()).unwrap())).await;
    (res.status.as_u16(), res.into_text().await)
}

#[tokio::test]
async fn state_reaches_pages_and_routes_as_an_extension() {
    let app = app(true);
    let (status, body) = get(&app, "/").await;
    assert_eq!(status, 200);
    assert!(body.contains("hello state"), "{body}");
    assert_eq!(get(&app, "/api").await, (200, "state".to_owned()));
}

#[tokio::test]
async fn a_page_asking_for_missing_state_is_an_error_not_a_panic() {
    let app = app(false);
    assert_eq!(get(&app, "/").await.0, 500);
    assert_eq!(get(&app, "/api").await, (200, "none".to_owned()));
}
