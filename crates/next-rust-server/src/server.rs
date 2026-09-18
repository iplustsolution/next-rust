//! Production HTTP server built on hyper.
//!
//! * HTTP/1.1 with keep-alive and HTTP/2 (h2c, and via TLS-terminating
//!   proxies) using hyper's automatic protocol detection.
//! * Header read timeout (slowloris protection) and request timeouts.
//! * Graceful shutdown on SIGINT/SIGTERM.
//! * gzip compression for compressible responses, including streams.
//!
//! TLS and HTTP/3 are expected to be terminated by a reverse proxy or load
//! balancer (see the "Deployment" docs page).

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::body::{Frame, Incoming};
#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::{TokioIo, TokioTimer};
#[cfg(feature = "http2")]
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;

use crate::app::App;
use crate::request::{BoxError, Request};
use crate::response::{Body, Response};

pub type HyperBody = UnsyncBoxBody<Bytes, BoxError>;

impl App {
    /// Bind to the configured address and serve until shutdown.
    pub async fn serve(self) -> std::io::Result<()> {
        let addr = self.config().socket_addr_string();
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("could not bind {addr}: {e} (is another server running? set PORT to change the port)"),
            )
        })?;
        self.serve_listener(listener).await
    }

    /// Serve on an existing listener (tests, socket activation).
    pub async fn serve_listener(self, listener: TcpListener) -> std::io::Result<()> {
        let local = listener.local_addr()?;
        let config = self.inner.config.clone();
        let banner = crate::banner::enabled(self.environment());
        if banner {
            crate::banner::started(local, self.inner.routes.pages.len(), self.inner.routes.apis.len());
        } else {
            crate::log::info(&format!(
                "ready on http://{}:{} ({}, {} routes)",
                if local.ip().is_unspecified() { "localhost".to_string() } else { local.ip().to_string() },
                local.port(),
                self.environment().as_str(),
                self.inner.routes.pages.len() + self.inner.routes.apis.len()
            ));
        }

        let header_timeout = Duration::from_secs(config.server.header_timeout.max(1));
        #[cfg(feature = "http2")]
        let builder = {
            let mut builder = auto::Builder::new(TokioExecutor::new());
            builder.http1().timer(TokioTimer::new()).header_read_timeout(header_timeout).keep_alive(true);
            if config.server.http2 {
                builder.http2().timer(TokioTimer::new()).keep_alive_interval(Duration::from_secs(20));
            } else {
                builder = builder.http1_only();
            }
            builder
        };
        #[cfg(not(feature = "http2"))]
        let builder = {
            if config.server.http2 {
                crate::log::warn("[server] http2 = true needs the `http2` feature of next-rust; serving HTTP/1.1");
            }
            let mut builder = hyper::server::conn::http1::Builder::new();
            builder.timer(TokioTimer::new()).header_read_timeout(header_timeout).keep_alive(true);
            builder
        };
        // Every connection task holds a receiver: sending tells them to finish
        // gracefully, and `closed()` resolves once all of them are done.
        let (graceful, _) = tokio::sync::watch::channel(());
        let shutdown = shutdown_signal();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, remote) = match accepted {
                        Ok(v) => v,
                        Err(e) => {
                            crate::log::warn(&format!("accept failed: {e}"));
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            continue;
                        }
                    };
                    let _ = stream.set_nodelay(true);
                    let app = self.clone();
                    let service = hyper::service::service_fn(move |req: hyper::Request<Incoming>| {
                        let app = app.clone();
                        async move { Ok::<_, Infallible>(app.handle_hyper(req, Some(remote)).await) }
                    });
                    #[cfg(feature = "http2")]
                    let conn = builder.serve_connection_with_upgrades(TokioIo::new(stream), service).into_owned();
                    #[cfg(not(feature = "http2"))]
                    let conn = builder.serve_connection(TokioIo::new(stream), service).with_upgrades();
                    let mut stop = graceful.subscribe();
                    tokio::spawn(async move {
                        let mut conn = std::pin::pin!(conn);
                        let result = tokio::select! {
                            result = conn.as_mut() => result,
                            _ = stop.changed() => {
                                conn.as_mut().graceful_shutdown();
                                conn.await
                            }
                        };
                        if let Err(e) = result {
                            crate::log::debug(&format!("connection error: {e}"));
                        }
                        drop(stop);
                    });
                }
                _ = &mut shutdown => {
                    if banner {
                        crate::banner::stopping();
                    }
                    crate::log::info("shutting down gracefully");
                    break;
                }
            }
        }
        let wait = Duration::from_secs(config.server.shutdown_timeout);
        tokio::select! {
            _ = async {
                let _ = graceful.send(());
                graceful.closed().await;
            } => {}
            _ = tokio::time::sleep(wait) => crate::log::warn("shutdown timeout: closing remaining connections"),
        }
        if banner {
            crate::banner::stopped();
        }
        Ok(())
    }

    /// Handle a hyper request (used by the built-in server).
    pub async fn handle_hyper(
        &self,
        req: hyper::Request<Incoming>,
        remote: Option<SocketAddr>,
    ) -> hyper::Response<HyperBody> {
        let start = Instant::now();
        let config = &self.inner.config;
        let (parts, body) = req.into_parts();
        let method = parts.method.clone();
        let path = parts.uri.path().to_owned();
        let accept_encoding = parts.headers.get("accept-encoding").and_then(|v| v.to_str().ok()).map(str::to_owned);
        let body = body.map_err(|e| Box::new(e) as BoxError).boxed_unsync();
        let request = Request::from_parts(parts, body, remote, config.server.body_limit, !self.environment().is_dev());

        let timeout = config.server.request_timeout;
        let res = if timeout > 0 {
            match tokio::time::timeout(Duration::from_secs(timeout), self.handle(request)).await {
                Ok(r) => r,
                Err(_) => Response::text("Gateway Timeout").with_status(504),
            }
        } else {
            self.handle(request).await
        };

        if config.logging.requests(self.environment()) && !path.starts_with("/_nr/dev/") {
            let id = res.header("x-request-id").map(str::to_owned);
            // Action URLs carry per-visitor tokens: keep them out of logs.
            let logged = if path.starts_with("/_nr/action/") { "/_nr/action/…" } else { path.as_str() };
            crate::log::request(
                method.as_str(),
                logged,
                res.status.as_u16(),
                start.elapsed().as_secs_f64() * 1000.0,
                id.as_deref(),
            );
        }
        let res = if config.server.compression { compress(res, accept_encoding.as_deref()) } else { res };
        into_hyper(res)
    }
}

/// Convert a framework response into a hyper response.
pub fn into_hyper(res: Response) -> hyper::Response<HyperBody> {
    let body: HyperBody = match res.body {
        Body::Empty => Empty::new().map_err(|never| match never {}).boxed_unsync(),
        Body::Bytes(b) => Full::new(b).map_err(|never| match never {}).boxed_unsync(),
        Body::Stream(s) => StreamBody::new(s.map(|r| r.map(Frame::data))).boxed_unsync(),
    };
    let mut out = hyper::Response::new(body);
    *out.status_mut() = res.status;
    *out.headers_mut() = res.headers;
    out
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[cfg(feature = "compression")]
fn accepts_gzip(header: Option<&str>) -> bool {
    header.is_some_and(|h| {
        h.split(',').any(|part| {
            let mut it = part.trim().split(';');
            let coding = it.next().unwrap_or("").trim();
            let q = it.find_map(|p| p.trim().strip_prefix("q=")).and_then(|q| q.parse::<f32>().ok()).unwrap_or(1.0);
            (coding.eq_ignore_ascii_case("gzip") || coding == "*") && q > 0.0
        })
    })
}

#[cfg(feature = "compression")]
pub(crate) fn compress(mut res: Response, accept_encoding: Option<&str>) -> Response {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    let ct = res.header("content-type").unwrap_or("");
    let eligible = accepts_gzip(accept_encoding)
        && !res.headers.contains_key("content-encoding")
        && res.status != http::StatusCode::PARTIAL_CONTENT
        && res.status != http::StatusCode::NOT_MODIFIED
        && !ct.starts_with("text/event-stream")
        && next_rust_assets::mime::is_compressible(ct);
    if !eligible {
        return res;
    }
    match std::mem::replace(&mut res.body, Body::Empty) {
        Body::Bytes(b) if b.len() >= 1024 => {
            let mut enc = GzEncoder::new(Vec::with_capacity(b.len() / 3), Compression::fast());
            if enc.write_all(&b).is_err() {
                res.body = Body::Bytes(b);
                return res;
            }
            match enc.finish() {
                Ok(out) => {
                    res.body = Body::Bytes(Bytes::from(out));
                    res.headers.remove("content-length");
                    res.set_header("content-encoding", "gzip");
                    res.append_header("vary", "Accept-Encoding");
                }
                Err(_) => res.body = Body::Bytes(b),
            }
        }
        Body::Stream(stream) => {
            // Each chunk is compressed and sync-flushed so streamed HTML still
            // reaches the browser progressively.
            let state = (stream, Some(GzEncoder::new(Vec::new(), Compression::fast())));
            let compressed = futures_util::stream::unfold(state, |(mut stream, enc)| async move {
                let mut enc = enc?;
                match stream.next().await {
                    Some(Ok(chunk)) => {
                        if enc.write_all(&chunk).is_err() || enc.flush().is_err() {
                            return Some((Err::<Bytes, BoxError>("gzip failure".into()), (stream, None)));
                        }
                        let out = std::mem::take(enc.get_mut());
                        Some((Ok(Bytes::from(out)), (stream, Some(enc))))
                    }
                    Some(Err(e)) => Some((Err(e), (stream, None))),
                    None => match enc.finish() {
                        Ok(tail) => Some((Ok(Bytes::from(tail)), (stream, None))),
                        Err(e) => Some((Err(e.into()), (stream, None))),
                    },
                }
            });
            res.body = Body::Stream(Box::pin(compressed));
            res.headers.remove("content-length");
            res.set_header("content-encoding", "gzip");
            res.append_header("vary", "Accept-Encoding");
        }
        other => res.body = other,
    }
    res
}

#[cfg(not(feature = "compression"))]
pub(crate) fn compress(res: Response, _accept_encoding: Option<&str>) -> Response {
    res
}
