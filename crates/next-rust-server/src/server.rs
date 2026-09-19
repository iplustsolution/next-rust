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

        if config.logging.requests(self.environment()) && !path.starts_with("/_next-rust/dev/") {
            let id = res.header("x-request-id").map(str::to_owned);
            // Action URLs carry per-visitor tokens: keep them out of logs.
            let logged = if path.starts_with("/_next-rust/action/") { "/_next-rust/action/…" } else { path.as_str() };
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
/// Whether `Accept-Encoding` allows `coding`.
pub(crate) fn accepts(header: Option<&str>, coding: &str) -> bool {
    header.is_some_and(|h| {
        h.split(',').any(|part| {
            let mut it = part.trim().split(';');
            let name = it.next().unwrap_or("").trim();
            let q = it.find_map(|p| p.trim().strip_prefix("q=")).and_then(|q| q.parse::<f32>().ok()).unwrap_or(1.0);
            (name.eq_ignore_ascii_case(coding) || name == "*") && q > 0.0
        })
    })
}

/// A streaming encoder: Brotli when the client accepts it, gzip otherwise.
#[cfg(feature = "compression")]
enum Encoder {
    Br(Box<brotli::CompressorWriter<Vec<u8>>>),
    Gz(flate2::write::GzEncoder<Vec<u8>>),
}

#[cfg(feature = "compression")]
impl Encoder {
    /// Fast settings: pages are compressed per request.
    fn new(br: bool) -> Self {
        if br {
            Encoder::Br(Box::new(brotli::CompressorWriter::new(Vec::with_capacity(8192), 4096, 4, 20)))
        } else {
            Encoder::Gz(flate2::write::GzEncoder::new(Vec::with_capacity(8192), flate2::Compression::fast()))
        }
    }

    fn coding(&self) -> &'static str {
        match self {
            Encoder::Br(_) => "br",
            Encoder::Gz(_) => "gzip",
        }
    }

    fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        match self {
            Encoder::Br(w) => w.write_all(data),
            Encoder::Gz(w) => w.write_all(data),
        }
    }

    /// Everything encoded so far, flushed so the browser can use it.
    fn take(&mut self) -> std::io::Result<Vec<u8>> {
        use std::io::Write;
        match self {
            Encoder::Br(w) => {
                w.flush()?;
                Ok(std::mem::take(w.get_mut()))
            }
            Encoder::Gz(w) => {
                w.flush()?;
                Ok(std::mem::take(w.get_mut()))
            }
        }
    }

    fn finish(self) -> std::io::Result<Vec<u8>> {
        match self {
            Encoder::Br(w) => Ok(w.into_inner()),
            Encoder::Gz(w) => w.finish(),
        }
    }
}

#[cfg(feature = "compression")]
pub(crate) fn compress(mut res: Response, accept_encoding: Option<&str>) -> Response {
    let ct = res.header("content-type").unwrap_or("");
    let br = accepts(accept_encoding, "br");
    let eligible = (br || accepts(accept_encoding, "gzip"))
        && !res.headers.contains_key("content-encoding")
        && res.status != http::StatusCode::PARTIAL_CONTENT
        && res.status != http::StatusCode::NOT_MODIFIED
        && !ct.starts_with("text/event-stream")
        && next_rust_assets::mime::is_compressible(ct);
    if !eligible {
        return res;
    }
    let coding = Encoder::new(br).coding();
    match std::mem::replace(&mut res.body, Body::Empty) {
        Body::Bytes(b) if b.len() >= 1024 => {
            let mut enc = Encoder::new(br);
            match enc.write(&b).and_then(|()| enc.finish()) {
                Ok(out) => {
                    res.body = Body::Bytes(Bytes::from(out));
                    res.headers.remove("content-length");
                    res.set_header("content-encoding", coding);
                    res.append_header("vary", "Accept-Encoding");
                }
                Err(_) => res.body = Body::Bytes(b),
            }
        }
        Body::Stream(stream) => {
            // Each chunk is compressed and flushed so streamed HTML still
            // reaches the browser progressively.
            let state = (stream, Some(Encoder::new(br)));
            let compressed = futures_util::stream::unfold(state, |(mut stream, enc)| async move {
                let mut enc = enc?;
                match stream.next().await {
                    Some(Ok(chunk)) => match enc.write(&chunk).and_then(|()| enc.take()) {
                        Ok(out) => Some((Ok(Bytes::from(out)), (stream, Some(enc)))),
                        Err(e) => Some((Err::<Bytes, BoxError>(e.into()), (stream, None))),
                    },
                    Some(Err(e)) => Some((Err(e), (stream, None))),
                    None => match enc.finish() {
                        Ok(tail) => Some((Ok(Bytes::from(tail)), (stream, None))),
                        Err(e) => Some((Err(e.into()), (stream, None))),
                    },
                }
            });
            res.body = Body::Stream(Box::pin(compressed));
            res.headers.remove("content-length");
            res.set_header("content-encoding", coding);
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

#[cfg(all(test, feature = "compression"))]
mod compression_tests {
    use std::io::Read;

    use bytes::Bytes;
    use futures_util::StreamExt;

    use super::compress;
    use crate::response::{Body, Response};

    fn page() -> String {
        "<p>Hello, compressed world.</p>".repeat(100)
    }

    fn inflate(coding: &str, data: &[u8]) -> String {
        let mut out = String::new();
        match coding {
            "br" => brotli::Decompressor::new(data, 4096).read_to_string(&mut out).unwrap(),
            _ => flate2::read::GzDecoder::new(data).read_to_string(&mut out).unwrap(),
        };
        out
    }

    #[tokio::test]
    async fn brotli_is_preferred_and_gzip_still_works() {
        for (accept, coding) in [("gzip, deflate, br", "br"), ("gzip", "gzip"), ("br;q=0, gzip", "gzip")] {
            let res = compress(Response::html(page()), Some(accept));
            assert_eq!(res.header("content-encoding"), Some(coding), "{accept}");
            assert_eq!(res.header("vary"), Some("Accept-Encoding"));
            let body = res.into_bytes().await.unwrap();
            assert_eq!(inflate(coding, &body), page());
        }
        let plain = compress(Response::html(page()), Some("identity"));
        assert_eq!(plain.header("content-encoding"), None);
        assert_eq!(compress(Response::html("tiny"), Some("br")).header("content-encoding"), None, "small bodies stay");
    }

    #[tokio::test]
    async fn streamed_bodies_are_flushed_per_chunk() {
        let chunks = vec![Ok::<_, crate::request::BoxError>(Bytes::from(page())), Ok(Bytes::from("<!-- end -->"))];
        let stream: crate::response::ByteStream = Box::pin(futures_util::stream::iter(chunks));
        let res = compress(
            Response::new(http::StatusCode::OK, Body::Stream(stream)).with_content_type("text/html"),
            Some("br"),
        );
        assert_eq!(res.header("content-encoding"), Some("br"));
        let Body::Stream(mut s) = res.body else { panic!("stream") };
        let mut parts = Vec::new();
        while let Some(chunk) = s.next().await {
            parts.push(chunk.unwrap());
        }
        assert!(parts.len() >= 2, "one compressed chunk per source chunk: {}", parts.len());
        assert_eq!(inflate("br", &parts.concat()), page() + "<!-- end -->");
    }
}
