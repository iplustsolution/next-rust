//! # next-rust-auth
//!
//! Building blocks for authentication and abuse control in a Next Rust
//! application, independent of any database. Enable the `auth` feature of
//! `next-rust` and use it as `next_rust::auth` (add `auth-redis` for the
//! Redis backends).
//!
//! | Module | What it gives you |
//! | --- | --- |
//! | [`crypto`] | [`AuthKeys`]: typed sealed tokens (AES-256-GCM, a key per [`Purpose`], expiry, secret rotation); random tokens, numeric codes, hashing, constant-time comparison |
//! | [`cookies`] | [`SecureCookies`]: `__Host-` prefixed `HttpOnly` cookies with safe defaults, sealed cookies for pending steps |
//! | [`session`] | [`Sessions`]: access token + rotating refresh token with reuse detection and a grace window, over your [`SessionStore`]; the [`CurrentSession`] extractor and `require` middleware |
//! | `password` | `Passwords`: argon2id off the async threads, dummy verification, rehash detection; `PasswordPolicy` |
//! | [`limits`] | [`RateLimiter`]: fixed and sliding windows in memory or Redis, network and identifier keys, fail-open or fail-closed, a middleware adapter |
//! | [`codes`] | [`Codes`]: one-time codes bound to user and purpose, attempt counting, decoys |
//!
//! None of it knows what a user is: you bring the accounts table, the
//! [`SessionStore`] (a few SQL statements) and the screens.
//!
//! ## What this crate deliberately leaves to you
//!
//! * **Non-enumerable answers.** Sign-up, sign-in and recovery must answer
//!   the same for a known and an unknown address. The pieces are here
//!   (`Passwords::verify_dummy`, decoy tickets in
//!   [`SecureCookies::set_sealed`], [`Codes::decoy_miss`]); wiring them
//!   into your flows is yours.
//! * **Delivery** of codes and security mail, **audit logging** (hook into
//!   [`Sessions::on_theft`]), account lockout in your database.
//! * **CSRF**: the framework's `[security] csrf` covers server actions and
//!   forms; these cookies are `SameSite=Lax` on top of that.

#![forbid(unsafe_code)]

pub mod codes;
pub mod cookies;
pub mod crypto;
pub mod limits;
#[cfg(feature = "password")]
pub mod password;
#[cfg(feature = "redis")]
pub mod redis_backend;
pub mod session;

pub use codes::{CodeCheck, CodePolicy, CodeStore, Codes, MemoryCodeStore, StoredCode};
pub use cookies::SecureCookies;
pub use crypto::{AuthKeys, KeyError, OpenError, Purpose};
pub use limits::{Decision, Limiter, MemoryLimiter, Quota, RateLimiter, RequestLimit, Throttled};
#[cfg(feature = "password")]
pub use password::{PasswordPolicy, Passwords, PolicyViolation};
#[cfg(feature = "redis")]
pub use redis_backend::{RedisGrace, RedisLimiter};
pub use session::{
    CurrentSession, Established, GraceStore, MemorySessionStore, RevokeReason, SessionConfig, SessionError,
    SessionGate, SessionRecord, SessionStore, Sessions, TheftEvent, Visitor,
};

/// The `redis` crate this build uses, for `ConnectionManager` and friends.
#[cfg(feature = "redis")]
pub use redis;

/// A failure of your store ([`SessionStore`], [`CodeStore`]): wrap the
/// database error with [`StoreError::new`].
///
/// ```
/// use next_rust_auth::StoreError;
/// fn query() -> Result<(), std::io::Error> { Ok(()) }
/// fn in_a_store() -> Result<(), StoreError> {
///     query().map_err(StoreError::new)
/// }
/// # in_a_store().unwrap();
/// ```
#[derive(Debug)]
pub struct StoreError(Box<dyn std::error::Error + Send + Sync>);

impl StoreError {
    pub fn new(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        StoreError(error.into())
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}
