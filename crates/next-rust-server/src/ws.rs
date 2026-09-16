//! WebSockets (feature `websocket`).
//!
//! ```ignore
//! // app/api/chat/route.rs
//! pub async fn GET(req: Request) -> Response {
//!     upgrade(req, |mut socket| async move {
//!         while let Some(Ok(msg)) = socket.recv().await {
//!             if let Message::Text(t) = msg { let _ = socket.send(Message::Text(t)).await; }
//!         }
//!     })
//! }
//! ```

use std::future::Future;

use futures_util::{SinkExt, StreamExt};
use hyper_util::rt::TokioIo;
use tokio_tungstenite::WebSocketStream;
pub use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;

use crate::request::Request;
use crate::response::Response;

/// An upgraded WebSocket connection.
pub struct WebSocket {
    inner: WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>,
}

impl WebSocket {
    /// Next message; `None` when the connection closed.
    pub async fn recv(&mut self) -> Option<Result<Message, crate::Error>> {
        self.inner.next().await.map(|r| r.map_err(crate::Error::from))
    }

    pub async fn send(&mut self, msg: Message) -> Result<(), crate::Error> {
        self.inner.send(msg).await.map_err(crate::Error::from)
    }

    pub async fn close(mut self) -> Result<(), crate::Error> {
        self.inner.close(None).await.map_err(crate::Error::from)
    }
}

/// Upgrade the request and run `handler` on the connection.
pub fn upgrade<H, Fut>(mut req: Request, handler: H) -> Response
where
    H: FnOnce(WebSocket) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let valid = req.is_websocket_upgrade()
        && req.header("connection").is_some_and(|c| c.to_ascii_lowercase().contains("upgrade"))
        && req.header("sec-websocket-version") == Some("13");
    let Some(key) = req.header("sec-websocket-key").map(str::to_owned).filter(|_| valid) else {
        return Response::text("expected a WebSocket upgrade request")
            .with_status(426)
            .with_header("upgrade", "websocket");
    };
    let Some(on_upgrade) = req.parts_mut().extensions.remove::<hyper::upgrade::OnUpgrade>() else {
        return Response::text("connection cannot be upgraded").with_status(400);
    };
    tokio::spawn(async move {
        match on_upgrade.await {
            Ok(upgraded) => {
                let stream = WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;
                handler(WebSocket { inner: stream }).await;
            }
            Err(e) => crate::log::warn(&format!("websocket upgrade failed: {e}")),
        }
    });
    Response::status(101)
        .with_header("upgrade", "websocket")
        .with_header("connection", "upgrade")
        .with_header("sec-websocket-accept", &derive_accept_key(key.as_bytes()))
}
