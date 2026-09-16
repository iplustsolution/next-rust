//! In-process testing without a network socket.
//!
//! ```ignore
//! #[tokio::test]
//! async fn home_page() {
//!     let client = TestClient::new(my_app::routes());
//!     let res = client.get("/").await;
//!     assert_eq!(res.status, 200);
//!     assert!(res.text.contains("Hello"));
//! }
//! ```

use bytes::Bytes;
use futures_util::StreamExt;
use http::HeaderMap;
use next_rust_core::{Config, Environment};

use crate::app::{App, AppBuilder, Routes};
use crate::request::Request;
use crate::response::Body;

/// A collected response.
#[derive(Debug, Clone)]
pub struct TestResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub text: String,
    /// Body chunks as sent (useful for streaming assertions).
    pub chunks: Vec<String>,
}

impl TestResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> T {
        serde_json::from_str(&self.text).unwrap_or_else(|e| panic!("response is not valid JSON ({e}): {}", self.text))
    }

    /// All `Set-Cookie` values.
    pub fn cookies(&self) -> Vec<&str> {
        self.headers.get_all("set-cookie").iter().filter_map(|v| v.to_str().ok()).collect()
    }
}

/// Sends requests to an [`App`] directly.
#[derive(Clone)]
pub struct TestClient {
    app: App,
    cookies: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
}

impl TestClient {
    /// Build the app with the project's configuration (discovered from
    /// `CARGO_MANIFEST_DIR`) in the `test` environment.
    pub fn new(routes: Routes) -> Self {
        Self::with(App::new(routes), Environment::Test)
    }

    /// Like [`TestClient::new`] but in production mode (static caching,
    /// generic error messages).
    pub fn production(routes: Routes) -> Self {
        Self::with(App::new(routes), Environment::Production)
    }

    pub fn with(builder: AppBuilder, env: Environment) -> Self {
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
        let config = Config::discover(&root).expect("valid next-rust configuration");
        let app = builder
            .config(config)
            .environment(env)
            .page_store(next_rust_cache::MemoryStore::new(1000))
            .build()
            .expect("app builds");
        TestClient { app, cookies: Default::default() }
    }

    pub fn from_app(app: App) -> Self {
        TestClient { app, cookies: Default::default() }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    /// Send an arbitrary request. Cookies set by earlier responses are sent
    /// along (a minimal cookie jar), unless the request already has a
    /// `Cookie` header.
    pub async fn send(&self, mut req: http::Request<Bytes>) -> TestResponse {
        if !req.headers().contains_key("cookie") {
            let jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner()).clone();
            if !jar.is_empty() {
                let v = jar.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
                if let Ok(v) = http::HeaderValue::from_str(&v) {
                    req.headers_mut().insert("cookie", v);
                }
            }
        }
        if !req.headers().contains_key("host") {
            req.headers_mut().insert("host", http::HeaderValue::from_static("localhost"));
        }
        let res = self.app.handle(Request::from_http(req)).await;
        let status = res.status.as_u16();
        let headers = res.headers.clone();
        {
            let mut jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
            for v in headers.get_all("set-cookie").iter().filter_map(|v| v.to_str().ok()) {
                let Some((pair, _)) = v.split_once(';').or(Some((v, ""))) else { continue };
                let Some((k, val)) = pair.split_once('=') else { continue };
                jar.retain(|(name, _)| name != k);
                if !v.contains("Max-Age=0") {
                    jar.push((k.to_owned(), val.to_owned()));
                }
            }
        }
        let mut chunks = Vec::new();
        match res.body {
            Body::Empty => {}
            Body::Bytes(b) => chunks.push(String::from_utf8_lossy(&b).into_owned()),
            Body::Stream(mut s) => {
                while let Some(chunk) = s.next().await {
                    match chunk {
                        Ok(b) => chunks.push(String::from_utf8_lossy(&b).into_owned()),
                        Err(e) => chunks.push(format!("<stream error: {e}>")),
                    }
                }
            }
        }
        TestResponse { status, headers, text: chunks.concat(), chunks }
    }

    pub async fn get(&self, uri: &str) -> TestResponse {
        self.send(http::Request::get(uri).body(Bytes::new()).expect("valid request")).await
    }

    pub async fn post_json<T: serde::Serialize>(&self, uri: &str, body: &T) -> TestResponse {
        let bytes = serde_json::to_vec(body).expect("serializable body");
        self.send(
            http::Request::post(uri)
                .header("content-type", "application/json")
                .body(Bytes::from(bytes))
                .expect("valid request"),
        )
        .await
    }

    /// `application/x-www-form-urlencoded` POST, e.g. `"name=Ada&email=a%40b.c"`.
    pub async fn post_form(&self, uri: &str, body: &str) -> TestResponse {
        self.send(
            http::Request::post(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Bytes::from(body.to_owned()))
                .expect("valid request"),
        )
        .await
    }
}
