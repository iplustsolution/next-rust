//! Incoming HTTP request.

use std::net::{IpAddr, SocketAddr};

use bytes::Bytes;
use http::request::Parts;
use http::{HeaderMap, Method, Uri};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Empty, Full};
use next_rust_router::Params;
use serde::de::DeserializeOwned;

use crate::cookies::Cookies;
use crate::error::{Error, Result};

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
/// Request body type (streaming).
pub type RequestBody = UnsyncBoxBody<Bytes, BoxError>;

/// An HTTP request as seen by middleware, API routes and server actions.
pub struct Request {
    parts: Parts,
    // Mutex makes `Request: Sync` (the body stream is only `Send`); all access
    // goes through `&mut self`, so it is never contended.
    body: std::sync::Mutex<Option<RequestBody>>,
    buffered: Option<Bytes>,
    params: Params,
    cookies: Cookies,
    remote_addr: Option<SocketAddr>,
    body_limit: usize,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request").field("method", &self.parts.method).field("uri", &self.parts.uri).finish()
    }
}

impl Request {
    pub fn from_parts(
        parts: Parts,
        body: RequestBody,
        remote_addr: Option<SocketAddr>,
        body_limit: usize,
        secure_cookies: bool,
    ) -> Self {
        let cookies = Cookies::from_headers(&parts.headers, secure_cookies);
        Request {
            parts,
            body: std::sync::Mutex::new(Some(body)),
            buffered: None,
            params: Params::new(),
            cookies,
            remote_addr,
            body_limit,
        }
    }

    /// Build a request from any `http::Request` with a buffered body
    /// (useful in tests and serverless adapters).
    pub fn from_http(req: http::Request<Bytes>) -> Self {
        let (parts, body) = req.into_parts();
        let body = Full::new(body).map_err(|never| match never {}).boxed_unsync();
        Request::from_parts(parts, body, None, usize::MAX, false)
    }

    /// A body-less GET request for `uri` (tests, static rendering).
    pub fn get(uri: &str) -> Self {
        let req = http::Request::get(uri).body(Bytes::new()).expect("valid uri");
        Request::from_http(req)
    }

    pub fn method(&self) -> &Method {
        &self.parts.method
    }

    pub fn uri(&self) -> &Uri {
        &self.parts.uri
    }

    pub fn path(&self) -> &str {
        self.parts.uri.path()
    }

    pub fn query_string(&self) -> &str {
        self.parts.uri.query().unwrap_or("")
    }

    /// Deserialize the query string into `T`.
    pub fn query<T: DeserializeOwned>(&self) -> Result<T> {
        serde_urlencoded::from_str(self.query_string())
            .map_err(|e| Error::http(400, format!("invalid query string: {e}")))
    }

    /// A single query parameter.
    pub fn query_param(&self, name: &str) -> Option<String> {
        serde_urlencoded::from_str::<Vec<(String, String)>>(self.query_string())
            .ok()?
            .into_iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.parts.headers
    }

    /// Header value as text (`None` if missing or not visible ASCII).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.parts.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name)
    }

    pub(crate) fn set_params(&mut self, params: Params) {
        self.params = params;
    }

    pub fn cookies(&self) -> &Cookies {
        &self.cookies
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// The client's IP address. With `[server] trust_proxy`, the first
    /// `X-Forwarded-For` entry (the address your proxy saw); otherwise, or
    /// when that entry is not an address, the peer's. Never trust the header
    /// without a proxy that overwrites it: clients can send anything.
    pub fn client_ip(&self) -> Option<IpAddr> {
        client_ip_from(self.headers(), self.remote_addr, crate::trust_proxy())
    }

    pub fn extensions(&self) -> &http::Extensions {
        &self.parts.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut http::Extensions {
        &mut self.parts.extensions
    }

    /// Typed request-scoped value inserted by middleware (e.g. the current user).
    pub fn extension<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.parts.extensions.get::<T>()
    }

    pub fn insert_extension<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
        self.parts.extensions.insert(value);
    }

    /// Rewrite the request path (middleware only; routing happens afterwards).
    pub fn set_path(&mut self, path_and_query: &str) -> Result<()> {
        let uri: Uri = path_and_query.parse().map_err(|_| Error::http(400, "invalid rewrite target"))?;
        self.parts.uri = uri;
        Ok(())
    }

    /// Lower the body limit to `limit` if it is currently higher.
    pub(crate) fn limit_body(&mut self, limit: usize) {
        self.body_limit = self.body_limit.min(limit);
    }

    /// Override the maximum body size for this request.
    pub fn set_body_limit(&mut self, limit: usize) {
        self.body_limit = limit;
    }

    pub fn is_websocket_upgrade(&self) -> bool {
        self.header("upgrade").is_some_and(|u| u.eq_ignore_ascii_case("websocket"))
    }

    /// `true` unless the browser says the request came from another site.
    ///
    /// Checks `Sec-Fetch-Site`, then `Origin` against the request host
    /// (`X-Forwarded-Host` behind a trusted proxy), allowing `[security]
    /// allowed_origins`. Requests without either header (curl, server to
    /// server) pass: they carry no ambient cookies across sites. Server
    /// actions run this check already; call it yourself in API routes and
    /// WebSocket handlers that change state or return private data:
    ///
    /// ```ignore
    /// pub async fn POST(mut req: Request) -> Result<Json<Receipt>> {
    ///     if !req.same_origin() {
    ///         return Err(Error::http(403, "cross-site request refused"));
    ///     }
    ///     // …
    /// }
    /// ```
    pub fn same_origin(&self) -> bool {
        let allowed = self
            .extension::<std::sync::Arc<next_rust_core::Config>>()
            .map(|c| c.security.allowed_origins.as_slice())
            .unwrap_or(&[]);
        crate::actions::same_origin(self, allowed)
    }

    /// Whether the connection (or the trusted proxy) used HTTPS.
    pub fn is_secure(&self) -> bool {
        self.parts.uri.scheme_str() == Some("https") || self.header("x-forwarded-proto") == Some("https")
    }

    /// Read the whole body (enforcing the body limit: 413 when exceeded).
    /// Can be called repeatedly; the body is buffered on first read.
    pub async fn bytes(&mut self) -> Result<Bytes> {
        if let Some(b) = &self.buffered {
            return Ok(b.clone());
        }
        if let Some(len) = self.header("content-length").and_then(|l| l.parse::<usize>().ok())
            && len > self.body_limit
        {
            return Err(Error::http(413, "request body too large"));
        }
        let limit = self.body_limit;
        let mut body = self
            .body
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_else(|| Empty::new().map_err(|never| match never {}).boxed_unsync());
        let mut buf: Vec<u8> = Vec::new();
        while let Some(frame) = body.frame().await {
            let frame = match frame {
                Ok(f) => f,
                Err(e) => return Err(read_error(&*e)),
            };
            if let Ok(data) = frame.into_data() {
                if buf.len() + data.len() > limit {
                    return Err(Error::http(413, "request body too large"));
                }
                buf.extend_from_slice(&data);
            }
        }
        let bytes = Bytes::from(buf);
        self.buffered = Some(bytes.clone());
        Ok(bytes)
    }

    /// Raw body — use for webhook signature verification.
    pub async fn raw_body(&mut self) -> Result<Bytes> {
        self.bytes().await
    }

    pub async fn text(&mut self) -> Result<String> {
        let b = self.bytes().await?;
        String::from_utf8(b.to_vec()).map_err(|_| Error::http(400, "request body is not valid UTF-8"))
    }

    pub async fn json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let b = self.bytes().await?;
        serde_json::from_slice(&b).map_err(|e| Error::http(400, format!("invalid JSON body: {e}")))
    }

    /// `application/x-www-form-urlencoded` body.
    pub async fn form<T: DeserializeOwned>(&mut self) -> Result<T> {
        let b = self.bytes().await?;
        serde_urlencoded::from_bytes(&b).map_err(|e| Error::http(400, format!("invalid form body: {e}")))
    }

    /// Take the streaming body (for large uploads). Returns `None` if the
    /// body was already consumed.
    pub fn take_body(&mut self) -> Option<RequestBody> {
        if let Some(b) = self.buffered.clone() {
            return Some(Full::new(b).map_err(|never| match never {}).boxed_unsync());
        }
        self.body.get_mut().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    #[cfg_attr(not(feature = "websocket"), allow(dead_code))]
    pub(crate) fn parts_mut(&mut self) -> &mut Parts {
        &mut self.parts
    }
}

fn read_error(e: &(dyn std::error::Error + Send + Sync)) -> Error {
    Error::http(400, format!("failed to read request body: {e}"))
}

/// See [`Request::client_ip`]; `trust` is `[server] trust_proxy`.
pub(crate) fn client_ip_from(headers: &HeaderMap, peer: Option<SocketAddr>, trust: bool) -> Option<IpAddr> {
    if trust
        && let Some(first) =
            headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()).and_then(|v| v.split(',').next())
        && let Some(ip) = parse_ip(first.trim())
    {
        return Some(ip);
    }
    peer.map(|a| a.ip())
}

/// `1.2.3.4`, `2001:db8::1`, and the forms some proxies add a port to:
/// `1.2.3.4:5678`, `[2001:db8::1]:5678`, `[2001:db8::1]`.
fn parse_ip(s: &str) -> Option<IpAddr> {
    s.parse::<IpAddr>()
        .ok()
        .or_else(|| s.parse::<SocketAddr>().ok().map(|a| a.ip()))
        .or_else(|| s.strip_prefix('[')?.strip_suffix(']')?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_headers(headers: &[(&str, &str)]) -> Request {
        let mut b = http::Request::post("/api/x");
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        Request::from_http(b.body(Bytes::new()).unwrap())
    }

    #[test]
    fn same_origin_reads_fetch_metadata_then_origin() {
        assert!(with_headers(&[("host", "app.example"), ("sec-fetch-site", "same-origin")]).same_origin());
        assert!(with_headers(&[("host", "app.example"), ("sec-fetch-site", "none")]).same_origin());
        assert!(!with_headers(&[("host", "app.example"), ("sec-fetch-site", "cross-site")]).same_origin());
        assert!(with_headers(&[("host", "app.example"), ("origin", "https://app.example")]).same_origin());
        assert!(!with_headers(&[("host", "app.example"), ("origin", "https://evil.example")]).same_origin());
        // No browser headers at all: a server-to-server or curl request.
        assert!(with_headers(&[("host", "app.example")]).same_origin());
        // Configured extra origins pass.
        let mut req = with_headers(&[("host", "app.example"), ("origin", "https://admin.example")]);
        assert!(!req.same_origin());
        let mut config = next_rust_core::Config::default();
        config.security.allowed_origins = vec!["https://admin.example".into()];
        req.insert_extension(std::sync::Arc::new(config));
        assert!(req.same_origin());
    }

    #[test]
    fn client_ip_honours_trust_proxy() {
        use std::net::{Ipv4Addr, Ipv6Addr};
        let peer: SocketAddr = "10.0.0.1:443".parse().unwrap();
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "203.0.113.7, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip_from(&h, Some(peer), false), Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))), "untrusted");
        assert_eq!(client_ip_from(&h, Some(peer), true), Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))));
        for (value, want) in [
            ("198.51.100.2:5678", IpAddr::V4(Ipv4Addr::new(198, 51, 100, 2))),
            ("[2001:db8::1]:5678", IpAddr::V6("2001:db8::1".parse::<Ipv6Addr>().unwrap())),
            ("[2001:db8::1]", IpAddr::V6("2001:db8::1".parse::<Ipv6Addr>().unwrap())),
        ] {
            h.insert("x-forwarded-for", value.parse().unwrap());
            assert_eq!(client_ip_from(&h, Some(peer), true), Some(want), "{value}");
        }
        // Garbage falls back to the peer rather than becoming its own client.
        h.insert("x-forwarded-for", "not-an-ip".parse().unwrap());
        assert_eq!(client_ip_from(&h, Some(peer), true), Some(peer.ip()));
        assert_eq!(client_ip_from(&HeaderMap::new(), None, true), None);
    }
}
