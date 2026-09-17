//! Outgoing HTTP response.

use std::pin::Pin;

use bytes::Bytes;
use futures_util::stream::{Stream, StreamExt};
use http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderName, HeaderValue, LOCATION};
use http::{HeaderMap, StatusCode};
use serde::Serialize;

use crate::error::{Error, ErrorKind};
use crate::request::BoxError;

pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, BoxError>> + Send + 'static>>;

/// Response body.
pub enum Body {
    Empty,
    Bytes(Bytes),
    Stream(ByteStream),
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Body::Empty => f.write_str("Empty"),
            Body::Bytes(b) => write!(f, "Bytes({})", b.len()),
            Body::Stream(_) => f.write_str("Stream"),
        }
    }
}

/// An HTTP response.
#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Default for Response {
    fn default() -> Self {
        Response { status: StatusCode::OK, headers: HeaderMap::new(), body: Body::Empty }
    }
}

impl Response {
    pub fn new(status: StatusCode, body: Body) -> Self {
        Response { status, headers: HeaderMap::new(), body }
    }

    /// Empty response with a status code.
    pub fn status(code: u16) -> Self {
        Response::new(StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Body::Empty)
    }

    pub fn text(body: impl Into<String>) -> Self {
        Response::new(StatusCode::OK, Body::Bytes(Bytes::from(body.into())))
            .with_content_type("text/plain; charset=utf-8")
    }

    pub fn html(body: impl Into<String>) -> Self {
        Response::new(StatusCode::OK, Body::Bytes(Bytes::from(body.into())))
            .with_content_type("text/html; charset=utf-8")
    }

    /// XML response (feeds, sitemaps).
    pub fn xml(body: impl Into<String>) -> Self {
        Response::new(StatusCode::OK, Body::Bytes(Bytes::from(body.into())))
            .with_content_type("application/xml; charset=utf-8")
    }

    /// SVG response, for images generated on the fly.
    pub fn svg(body: impl Into<String>) -> Self {
        Response::new(StatusCode::OK, Body::Bytes(Bytes::from(body.into())))
            .with_content_type("image/svg+xml; charset=utf-8")
    }

    /// JSON response. Serialization failures produce a 500.
    pub fn json<T: Serialize + ?Sized>(data: &T) -> Self {
        match serde_json::to_vec(data) {
            Ok(v) => Response::new(StatusCode::OK, Body::Bytes(Bytes::from(v))).with_content_type("application/json"),
            Err(e) => Error::from(e).into_response(),
        }
    }

    pub fn bytes(body: impl Into<Bytes>) -> Self {
        Response::new(StatusCode::OK, Body::Bytes(body.into())).with_content_type("application/octet-stream")
    }

    /// Stream arbitrary byte chunks.
    pub fn stream<S, B, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<B, E>> + Send + 'static,
        B: Into<Bytes>,
        E: Into<BoxError>,
    {
        let s = stream.map(|r| r.map(Into::into).map_err(Into::into));
        Response::new(StatusCode::OK, Body::Stream(Box::pin(s)))
    }

    /// Temporary redirect (307).
    pub fn redirect(location: &str) -> Self {
        Response::redirect_with(location, StatusCode::TEMPORARY_REDIRECT)
    }

    /// Permanent redirect (308).
    pub fn permanent_redirect(location: &str) -> Self {
        Response::redirect_with(location, StatusCode::PERMANENT_REDIRECT)
    }

    /// Redirect after a form POST (303 See Other).
    pub fn see_other(location: &str) -> Self {
        Response::redirect_with(location, StatusCode::SEE_OTHER)
    }

    pub fn redirect_with(location: &str, status: StatusCode) -> Self {
        let mut r = Response::new(status, Body::Empty);
        match HeaderValue::from_str(location) {
            Ok(v) => {
                r.headers.insert(LOCATION, v);
            }
            Err(_) => return Response::status(500),
        }
        r
    }

    pub fn not_found() -> Self {
        Response::text("Not Found").with_status(404)
    }

    pub fn with_status(mut self, code: u16) -> Self {
        self.status = StatusCode::from_u16(code).unwrap_or(self.status);
        self
    }

    /// Set a header. Invalid names or values (e.g. containing CR/LF) are
    /// rejected, which prevents header injection; they are logged and dropped.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.set_header(name, value);
        self
    }

    pub fn set_header(&mut self, name: &str, value: &str) -> bool {
        match (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            (Ok(n), Ok(v)) => {
                self.headers.insert(n, v);
                true
            }
            _ => {
                crate::log::warn(&format!("invalid response header {name:?} dropped"));
                false
            }
        }
    }

    pub fn append_header(&mut self, name: &str, value: &str) -> bool {
        match (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            (Ok(n), Ok(v)) => {
                self.headers.append(n, v);
                true
            }
            _ => false,
        }
    }

    pub fn with_content_type(mut self, ct: &'static str) -> Self {
        self.headers.insert(CONTENT_TYPE, HeaderValue::from_static(ct));
        self
    }

    pub fn with_cache_control(mut self, value: &str) -> Self {
        if let Ok(v) = HeaderValue::from_str(value) {
            self.headers.insert(CACHE_CONTROL, v);
        }
        self
    }

    /// Add a cookie to the response.
    pub fn with_cookie(mut self, cookie: crate::Cookie) -> Self {
        match cookie.to_header_value() {
            Ok(v) => {
                self.append_header("set-cookie", &v);
            }
            Err(e) => crate::log::warn(&format!("cookie not sent: {e}")),
        }
        self
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Collect the body into bytes (tests and adapters).
    pub async fn into_bytes(self) -> Result<Bytes, BoxError> {
        match self.body {
            Body::Empty => Ok(Bytes::new()),
            Body::Bytes(b) => Ok(b),
            Body::Stream(mut s) => {
                let mut buf = Vec::new();
                while let Some(chunk) = s.next().await {
                    buf.extend_from_slice(&chunk?);
                }
                Ok(Bytes::from(buf))
            }
        }
    }

    /// Collect the body as UTF-8 text (lossy).
    pub async fn into_text(self) -> String {
        match self.into_bytes().await {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => format!("<body error: {e}>"),
        }
    }
}

/// Conversion into a [`Response`].
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Response {
        Response::bytes(self)
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        Response::bytes(self)
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::status(204)
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        Response::new(self, Body::Empty)
    }
}

impl<T: IntoResponse> IntoResponse for (StatusCode, T) {
    fn into_response(self) -> Response {
        let mut r = self.1.into_response();
        r.status = self.0;
        r
    }
}

impl<T: IntoResponse, E: Into<Error>> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(v) => v.into_response(),
            Err(e) => e.into().into_response(),
        }
    }
}

/// JSON body wrapper: `Json(data)`.
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        Response::json(&self.0)
    }
}

/// HTML body wrapper: `Html(markup)`.
pub struct Html<T>(pub T);

impl<T: Into<String>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        Response::html(self.0)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = self.status();
        match self.into_kind() {
            ErrorKind::Redirect { location, status } => Response::redirect_with(&location, status),
            ErrorKind::Validation(fields) => {
                let mut r = Response::json(&serde_json::json!({ "error": "validation failed", "fields": fields }));
                r.status = status;
                r
            }
            kind => {
                let err = Error::new(kind);
                if status.is_server_error() {
                    crate::log::error(&format!("request failed: {}", err.detailed_message()));
                }
                let msg = if crate::is_dev() { err.detailed_message() } else { err.public_message() };
                let mut r = Response::text(msg);
                r.status = status;
                r
            }
        }
    }
}
