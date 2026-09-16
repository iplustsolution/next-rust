//! The error type used by pages, layouts, loaders, API routes and actions.

use std::collections::BTreeMap;
use std::fmt;

use http::StatusCode;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Framework error.
///
/// Any `std::error::Error` converts into it with `?`. Control-flow variants
/// ([`not_found`], [`redirect`]) are created with helper functions and
/// returned like errors: `return Err(redirect("/login"))`.
pub struct Error {
    kind: ErrorKind,
}

pub enum ErrorKind {
    /// Render the nearest `not-found.rs` (404).
    NotFound,
    /// Redirect to `location`.
    Redirect { location: String, status: StatusCode },
    /// An HTTP error with a public message.
    Http { status: StatusCode, message: String },
    /// Field validation failed (form handling, server actions).
    Validation(BTreeMap<String, String>),
    /// A request-bound API was used while rendering a static page.
    DynamicUsage(&'static str),
    /// Unexpected failure. The message is only shown in development.
    Internal(BoxError),
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn into_kind(self) -> ErrorKind {
        self.kind
    }

    pub fn new(kind: ErrorKind) -> Self {
        Error { kind }
    }

    /// Internal error from a message.
    pub fn msg(message: impl Into<String>) -> Self {
        Error { kind: ErrorKind::Internal(message.into().into()) }
    }

    /// HTTP error with a message that is safe to show to users.
    pub fn http(status: u16, message: impl Into<String>) -> Self {
        Error {
            kind: ErrorKind::Http {
                status: StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                message: message.into(),
            },
        }
    }

    /// Validation errors keyed by field name.
    pub fn validation<I, K, V>(fields: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Error { kind: ErrorKind::Validation(fields.into_iter().map(|(k, v)| (k.into(), v.into())).collect()) }
    }

    pub fn status(&self) -> StatusCode {
        match &self.kind {
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Redirect { status, .. } => *status,
            ErrorKind::Http { status, .. } => *status,
            ErrorKind::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorKind::DynamicUsage(_) | ErrorKind::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self.kind, ErrorKind::NotFound)
    }

    pub fn is_redirect(&self) -> bool {
        matches!(self.kind, ErrorKind::Redirect { .. })
    }

    /// Message safe for end users in production.
    pub fn public_message(&self) -> String {
        match &self.kind {
            ErrorKind::NotFound => "Not Found".into(),
            ErrorKind::Redirect { location, .. } => format!("Redirecting to {location}"),
            ErrorKind::Http { message, .. } => message.clone(),
            ErrorKind::Validation(_) => "Validation failed".into(),
            ErrorKind::DynamicUsage(_) | ErrorKind::Internal(_) => "Internal Server Error".into(),
        }
    }

    /// Full message including internal details (development only).
    pub fn detailed_message(&self) -> String {
        match &self.kind {
            ErrorKind::Internal(e) => {
                let mut s = e.to_string();
                let mut source = e.source();
                while let Some(inner) = source {
                    s.push_str(&format!("\n  caused by: {inner}"));
                    source = inner.source();
                }
                s
            }
            ErrorKind::DynamicUsage(what) => format!(
                "`{what}` reads request data, but this page is rendered statically.\n\
                 Mark the route dynamic with `pub const RENDERING: Rendering = Rendering::Dynamic;`\n\
                 or remove the request-bound argument."
            ),
            ErrorKind::Validation(fields) => {
                fields.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("\n")
            }
            _ => self.public_message(),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error({}: {})", self.status(), self.detailed_message())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detailed_message())
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(e: E) -> Self {
        Error { kind: ErrorKind::Internal(Box::new(e)) }
    }
}

/// `Result` alias with [`Error`] as the default error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Render the nearest `not-found.rs` with status 404.
pub fn not_found() -> Error {
    Error { kind: ErrorKind::NotFound }
}

/// Temporary redirect (307, method preserved).
pub fn redirect(location: impl Into<String>) -> Error {
    Error { kind: ErrorKind::Redirect { location: location.into(), status: StatusCode::TEMPORARY_REDIRECT } }
}

/// Permanent redirect (308, method preserved).
pub fn permanent_redirect(location: impl Into<String>) -> Error {
    Error { kind: ErrorKind::Redirect { location: location.into(), status: StatusCode::PERMANENT_REDIRECT } }
}

/// Convert `None` into a 404.
pub trait OrNotFound<T> {
    fn or_not_found(self) -> Result<T>;
}

impl<T> OrNotFound<T> for Option<T> {
    fn or_not_found(self) -> Result<T> {
        self.ok_or_else(not_found)
    }
}

/// Accept both `T` and `Result<T, E>` from user functions (generated code).
pub trait IntoResult<T> {
    fn into_result(self) -> Result<T>;
}

impl<T, E: Into<Error>> IntoResult<T> for std::result::Result<T, E> {
    fn into_result(self) -> Result<T> {
        self.map_err(Into::into)
    }
}

macro_rules! plain_into_result {
    ($($t:ty),*) => {$(
        impl IntoResult<$t> for $t {
            fn into_result(self) -> Result<$t> {
                Ok(self)
            }
        }
    )*};
}

plain_into_result!(next_rust_view::Metadata, Vec<next_rust_router::Params>, crate::seo::Sitemap, crate::seo::Robots);
