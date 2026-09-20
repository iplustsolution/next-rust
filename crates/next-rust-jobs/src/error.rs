//! Error types.

use std::fmt;
use std::time::Duration;

/// An infrastructure error: enqueueing, the backend, configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The backend failed (connection lost, script error, corrupt record).
    Backend(String),
    /// The job input could not be serialised.
    Serialize(String),
    /// [`enqueue`](crate::enqueue) was called before a runner or client was
    /// installed with [`JobClient::install_global`](crate::JobClient::install_global).
    NoGlobalClient,
    /// Invalid runner configuration: a bad cron expression, a duplicate name.
    Config(String),
    /// The job was refused before it reached the backend: a payload over the
    /// size limit, an empty or oversized queue name, job name or unique key,
    /// or one with control characters.
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Backend(e) => write!(f, "job backend error: {e}"),
            Error::Serialize(e) => write!(f, "could not serialise the job input: {e}"),
            Error::NoGlobalClient => f.write_str(
                "no job client is installed: build a JobRunner (or call JobClient::install_global) before enqueue()",
            ),
            Error::Config(e) => write!(f, "invalid job configuration: {e}"),
            Error::Invalid(e) => write!(f, "the job was refused: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// Why a job attempt failed, returned from the job function.
///
/// Any `std::error::Error` converts with `?` into a retryable failure. Use
/// [`JobError::fatal`] when retrying cannot help (the job goes straight to
/// the dead-letter list) and [`JobError::retry_in`] to override the backoff.
///
/// Like `anyhow::Error`, this type deliberately does not implement
/// `std::error::Error` itself, which is what makes the blanket `From` possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobError {
    pub(crate) message: String,
    pub(crate) retryable: bool,
    pub(crate) retry_after: Option<Duration>,
}

impl JobError {
    /// A retryable failure.
    ///
    /// Messages are stored with the job and shown in the dead-letter list;
    /// the runner keeps the first [`MAX_ERROR_LEN`](crate::MAX_ERROR_LEN) bytes.
    pub fn msg(message: impl fmt::Display) -> Self {
        JobError { message: message.to_string(), retryable: true, retry_after: None }
    }

    /// A failure that must not be retried.
    pub fn fatal(message: impl fmt::Display) -> Self {
        JobError { message: message.to_string(), retryable: false, retry_after: None }
    }

    /// A retryable failure with an explicit delay instead of the backoff
    /// (for example the `Retry-After` of a rate-limited API).
    pub fn retry_in(delay: Duration, message: impl fmt::Display) -> Self {
        JobError { message: message.to_string(), retryable: true, retry_after: Some(delay) }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl<E: std::error::Error> From<E> for JobError {
    fn from(e: E) -> Self {
        JobError::msg(e)
    }
}

/// What a job function may return: `()`, or `Result<(), E>` for any `E`
/// that converts into [`JobError`].
pub trait IntoJobResult {
    fn into_job_result(self) -> Result<(), JobError>;
}

impl IntoJobResult for () {
    fn into_job_result(self) -> Result<(), JobError> {
        Ok(())
    }
}

impl<E: Into<JobError>> IntoJobResult for Result<(), E> {
    fn into_job_result(self) -> Result<(), JobError> {
        self.map_err(Into::into)
    }
}
