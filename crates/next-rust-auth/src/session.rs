//! Sessions: opened by a successful sign-in, verified on every request,
//! rotated as they are used, ended by sign-out.
//!
//! A session is a record in your [`SessionStore`] plus two cookies:
//!
//! * an **access token**: a [sealed](crate::crypto) token naming the
//!   session, the user, the family and a binding to this browser, good for
//!   15 minutes;
//! * a **refresh token**: 256 random bits. The store only ever sees its
//!   SHA-256, so a leaked sessions table signs nobody in.
//!
//! Every request with a valid access token still asks the store whether the
//! session is live ([`SessionStore::is_live`]), so a sign-out, a password
//! change or a suspension takes effect on the next click, not when the token
//! runs out. When the access token has expired, the refresh token is
//! *rotated*: a new record in the same family replaces the old one and the
//! browser gets new cookies.
//!
//! Presenting a refresh token that was already rotated means someone holds a
//! copy of it (the thief or the owner, whoever comes second). The whole
//! family ends and [`Sessions::on_theft`] fires. The exception is a short
//! *grace window* (15 seconds): two tabs refreshing at once, or a response
//! lost on a bad connection, present the same token legitimately, and get the
//! same replacement instead of an alarm. Grace entries live in this process
//! and, with several instances, in a shared [`GraceStore`] such as
//! `RedisGrace`.
//!
//! The grace answer is the *same* replacement the first request got, never a
//! second one, and it is only given to a request that also carries the
//! family's device cookie: a refresh token that leaked on its own (a log
//! line, a proxy) is theft even inside the window. Someone who copied the
//! whole cookie jar and wins the race inside the window does get the
//! replacement; that copy is caught at the next rotation, when the loser of
//! the race presents a retired token. Set [`SessionConfig::grace`] to zero to
//! close the window entirely (at the price of signing out people whose two
//! tabs refresh at once).
//!
//! ```ignore
//! use next_rust::auth::{AuthKeys, CurrentSession, Sessions, Visitor};
//!
//! // src/lib.rs – one engine for the process
//! pub static SESSIONS: LazyLock<Sessions<PgSessions>> = LazyLock::new(|| {
//!     let keys = AuthKeys::from_env("AUTH_SECRET", Some("AUTH_SECRET_PREVIOUS")).expect("AUTH_SECRET");
//!     Sessions::new(PgSessions::new(pool()), keys)
//! });
//!
//! // app/(account)/middleware.rs – everything below needs a session
//! pub async fn middleware(req: Request, next: Next) -> Response {
//!     SESSIONS.require("/login").handle(req, next).await
//! }
//!
//! // app/(account)/settings/page.rs
//! pub fn Page(session: CurrentSession<User>) -> impl View {
//!     h1!["Hello, ", session.user.name.clone()]
//! }
//!
//! // the sign-in action, after the password (and second factor) checked out
//! #[server_action]
//! pub async fn sign_in(ctx: ActionContext, form: SignIn) -> Result<()> {
//!     let user = check_password(&form).await?;
//!     SESSIONS.establish(&Visitor::from_action(&ctx), &user.id.to_string(), false).await?;
//!     Err(redirect("/settings"))
//! }
//! ```
//!
//! # What a lockout must not do
//!
//! Too many wrong passwords should block *new sign-ins*, not end open
//! sessions, or anyone can sign a person out by typing wrong passwords at
//! their address. Nothing here revokes on failed attempts; keep it that way
//! in your `is_live`.

use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use next_rust_server::{
    ActionContext, AuthUser, BoxFuture, ClientIp, Cookies, Ctx, FromContext, Middleware, Next, Request, Response,
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::StoreError;
use crate::cookies::{MAX_COOKIE_LIFETIME, SecureCookies};
use crate::crypto::{self, AuthKeys, Purpose};
use crate::limits::{Quota, RateLimiter, Throttled, key};

// ---------------------------------------------------------------------------
// Records and the store
// ---------------------------------------------------------------------------

/// Why a session ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevokeReason {
    /// Replaced by its successor in the family. Presenting its refresh
    /// token again (outside the grace window) is treated as theft.
    Rotated,
    Logout,
    /// A rotated refresh token came back; the whole family ended.
    ReuseDetected,
    /// Yours: `"password_changed"`, `"admin"`, ...
    Other(String),
}

impl RevokeReason {
    /// The text to store: `rotated`, `logout`, `reuse_detected` or your own.
    pub fn as_str(&self) -> &str {
        match self {
            RevokeReason::Rotated => "rotated",
            RevokeReason::Logout => "logout",
            RevokeReason::ReuseDetected => "reuse_detected",
            RevokeReason::Other(reason) => reason,
        }
    }

    /// The inverse of [`as_str`](Self::as_str), for reading a row back.
    pub fn parse(text: &str) -> Self {
        match text {
            "rotated" => RevokeReason::Rotated,
            "logout" => RevokeReason::Logout,
            "reuse_detected" => RevokeReason::ReuseDetected,
            other => RevokeReason::Other(other.to_owned()),
        }
    }
}

/// One session as the store keeps it. Times are seconds since the epoch;
/// ids are UUIDs in text form, so they fit `uuid` and `text` columns alike.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: String,
    /// Shared by a session and all its rotations.
    pub family_id: String,
    /// Lowercase hex SHA-256 of the refresh token. Index it, uniquely.
    pub refresh_hash: String,
    /// The browser's device cookie: a stable name for "your devices", not a secret.
    pub device_id: String,
    /// A second factor was used when the family was opened.
    pub mfa_verified: bool,
    pub created_at: u64,
    /// When this record's refresh token stops working.
    pub expires_at: u64,
    /// When the family ends for good, however often it is refreshed.
    pub absolute_expires_at: u64,
    /// `None` while the session is open.
    pub revoked: Option<RevokeReason>,
    pub ip: Option<IpAddr>,
    /// Trimmed to 512 bytes.
    pub user_agent: String,
}

/// Where sessions live. Seven operations, each one statement or one small
/// transaction in SQL; the documentation site has a complete sqlx/Postgres
/// implementation, and [`MemorySessionStore`] is the reference for tests.
///
/// Methods return `impl Future + Send`; implement them with plain `async fn`.
pub trait SessionStore: Send + Sync + 'static {
    /// What pages get for a live session: your account type.
    type User: Clone + Send + Sync + 'static;

    /// Whatever keeps the record found by
    /// [`find_by_refresh_hash`](Self::find_by_refresh_hash) locked until
    /// [`rotate`](Self::rotate): a database transaction holding
    /// `SELECT ... FOR UPDATE`. Dropping it must release the lock without
    /// changing anything (a transaction rolls back on drop).
    type Lock: Send;

    /// Inserts a new session.
    fn create(&self, session: &SessionRecord) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The session whose refresh token hashes to `hash`, **revoked or not**,
    /// locked against concurrent rotation. The lock is what makes two
    /// requests with the same token take turns: the second one must see the
    /// first one's rotation.
    fn find_by_refresh_hash(
        &self,
        hash: &str,
    ) -> impl Future<Output = Result<Option<(SessionRecord, Self::Lock)>, StoreError>> + Send;

    /// Under `lock`: inserts `new`, marks `old` revoked as
    /// [`RevokeReason::Rotated`], commits.
    fn rotate(
        &self,
        lock: Self::Lock,
        old: &SessionRecord,
        new: &SessionRecord,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Ends one session if it is open; whether it was.
    fn revoke(&self, session_id: &str, reason: RevokeReason) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// Ends every open session of a family; how many.
    fn revoke_family(
        &self,
        family_id: &str,
        reason: RevokeReason,
    ) -> impl Future<Output = Result<u64, StoreError>> + Send;

    /// Ends every open session of a user except `keep`; how many. For
    /// "sign out everywhere" and after a password change.
    fn revoke_user(
        &self,
        user_id: &str,
        reason: RevokeReason,
        keep: Option<&str>,
    ) -> impl Future<Output = Result<u64, StoreError>> + Send;

    /// The user behind session `session_id`, when the session is open, not
    /// expired (`expires_at` and `absolute_expires_at` after `now`), belongs
    /// to `user_id`, and the account may still be used (not deleted, not
    /// suspended). Called on every authenticated request: one indexed join.
    fn is_live(
        &self,
        session_id: &str,
        user_id: &str,
        now: u64,
    ) -> impl Future<Output = Result<Option<Self::User>, StoreError>> + Send;
}

// ---------------------------------------------------------------------------
// In-memory store
// ---------------------------------------------------------------------------

/// Sessions in this process: tests, examples, single-instance prototypes.
/// Users are whatever you [`insert_user`](Self::insert_user); removing one
/// is a suspension.
pub struct MemorySessionStore<U> {
    state: Arc<Mutex<MemoryState<U>>>,
    /// Stands in for the row lock: one rotation at a time.
    gate: Arc<tokio::sync::Mutex<()>>,
}

struct MemoryState<U> {
    sessions: HashMap<String, SessionRecord>,
    users: HashMap<String, U>,
}

impl<U> Default for MemorySessionStore<U> {
    fn default() -> Self {
        MemorySessionStore {
            state: Arc::new(Mutex::new(MemoryState { sessions: HashMap::new(), users: HashMap::new() })),
            gate: Arc::default(),
        }
    }
}

impl<U> Clone for MemorySessionStore<U> {
    fn clone(&self) -> Self {
        MemorySessionStore { state: self.state.clone(), gate: self.gate.clone() }
    }
}

impl<U: Clone + Send + Sync + 'static> MemorySessionStore<U> {
    pub fn new() -> Self {
        MemorySessionStore::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, MemoryState<U>> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert_user(&self, user_id: &str, user: U) {
        self.lock().users.insert(user_id.to_owned(), user);
    }

    /// The account can no longer be used: its sessions stop being live.
    pub fn remove_user(&self, user_id: &str) {
        self.lock().users.remove(user_id);
    }

    pub fn session(&self, session_id: &str) -> Option<SessionRecord> {
        self.lock().sessions.get(session_id).cloned()
    }

    /// Every record of a user, open or not, oldest first.
    pub fn sessions_of(&self, user_id: &str) -> Vec<SessionRecord> {
        let mut all: Vec<_> = self.lock().sessions.values().filter(|s| s.user_id == user_id).cloned().collect();
        all.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
        all
    }

    fn revoke_where(&self, reason: &RevokeReason, matches: impl Fn(&SessionRecord) -> bool) -> u64 {
        let mut count = 0;
        for session in self.lock().sessions.values_mut().filter(|s| s.revoked.is_none() && matches(s)) {
            session.revoked = Some(reason.clone());
            count += 1;
        }
        count
    }
}

impl<U: Clone + Send + Sync + 'static> SessionStore for MemorySessionStore<U> {
    type User = U;
    type Lock = tokio::sync::OwnedMutexGuard<()>;

    async fn create(&self, session: &SessionRecord) -> Result<(), StoreError> {
        self.lock().sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    async fn find_by_refresh_hash(&self, hash: &str) -> Result<Option<(SessionRecord, Self::Lock)>, StoreError> {
        let guard = self.gate.clone().lock_owned().await;
        Ok(self.lock().sessions.values().find(|s| s.refresh_hash == hash).cloned().map(|s| (s, guard)))
    }

    async fn rotate(&self, lock: Self::Lock, old: &SessionRecord, new: &SessionRecord) -> Result<(), StoreError> {
        {
            let mut state = self.lock();
            state.sessions.insert(new.id.clone(), new.clone());
            if let Some(old) = state.sessions.get_mut(&old.id) {
                old.revoked = Some(RevokeReason::Rotated);
            }
        }
        drop(lock);
        Ok(())
    }

    async fn revoke(&self, session_id: &str, reason: RevokeReason) -> Result<bool, StoreError> {
        Ok(self.revoke_where(&reason, |s| s.id == session_id) > 0)
    }

    async fn revoke_family(&self, family_id: &str, reason: RevokeReason) -> Result<u64, StoreError> {
        Ok(self.revoke_where(&reason, |s| s.family_id == family_id))
    }

    async fn revoke_user(&self, user_id: &str, reason: RevokeReason, keep: Option<&str>) -> Result<u64, StoreError> {
        Ok(self.revoke_where(&reason, |s| s.user_id == user_id && Some(s.id.as_str()) != keep))
    }

    async fn is_live(&self, session_id: &str, user_id: &str, now: u64) -> Result<Option<U>, StoreError> {
        let state = self.lock();
        let live = state.sessions.get(session_id).is_some_and(|s| {
            s.user_id == user_id && s.revoked.is_none() && s.expires_at > now && s.absolute_expires_at > now
        });
        Ok(if live { state.users.get(user_id).cloned() } else { None })
    }
}

// ---------------------------------------------------------------------------
// Grace
// ---------------------------------------------------------------------------

pub type GraceFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A shared place for grace entries when several instances serve one site.
/// Values are already sealed (they hold a live refresh token); the store
/// only keeps a string for a few seconds. Failures are the store's to log:
/// the in-process copy still covers requests that reach the same instance.
pub trait GraceStore: Send + Sync + 'static {
    fn park<'a>(&'a self, key: &'a str, sealed: &'a str, ttl: Duration) -> GraceFuture<'a, ()>;
    fn find<'a>(&'a self, key: &'a str) -> GraceFuture<'a, Option<String>>;
}

/// The replacement parked for concurrent requests.
#[derive(Serialize, Deserialize, Clone)]
struct Grace {
    sid: String,
    uid: String,
    fam: String,
    refresh: String,
    expires_at: u64,
    mfa: bool,
}

// ---------------------------------------------------------------------------
// The visitor
// ---------------------------------------------------------------------------

/// What the engine needs to know about the request: its cookie jar, user
/// agent and address. Build one wherever you are.
#[derive(Clone, Debug)]
pub struct Visitor {
    pub cookies: Cookies,
    pub user_agent: String,
    pub ip: Option<IpAddr>,
}

impl Visitor {
    pub fn new(headers: &http::HeaderMap, cookies: &Cookies, ip: Option<IpAddr>) -> Self {
        let mut user_agent =
            headers.get(http::header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned();
        if user_agent.len() > 512 {
            let mut end = 512;
            while !user_agent.is_char_boundary(end) {
                end -= 1;
            }
            user_agent.truncate(end);
        }
        Visitor { cookies: cookies.clone(), user_agent, ip }
    }

    /// In middleware and API routes.
    pub fn from_request(req: &Request) -> Self {
        Visitor::new(req.headers(), req.cookies(), req.client_ip())
    }

    /// In loaders and pages (take the `Ctx` extractor).
    pub fn from_context(ctx: &Ctx) -> Self {
        let ip = ClientIp::from_context(ctx).ok().and_then(|c| c.0);
        Visitor::new(&ctx.headers, &ctx.cookies, ip)
    }

    /// In server actions.
    pub fn from_action(ctx: &ActionContext) -> Self {
        Visitor::new(&ctx.headers, &ctx.cookies, ctx.client_ip())
    }

    /// The masked address ([`key::network`]): for limits, and for "a sign-in
    /// from 203.0.113.0/24" in security mail.
    pub fn network(&self) -> String {
        key::network(self.ip)
    }
}

// ---------------------------------------------------------------------------
// The engine
// ---------------------------------------------------------------------------

/// Lifetimes and cookie names.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// How long an access token is accepted before the refresh token is
    /// needed. Default 15 minutes.
    pub access_ttl: Duration,
    /// How long an unused refresh token lasts. Default 30 days.
    pub refresh_ttl: Duration,
    /// How long a family lasts however often it is refreshed. Default 365 days.
    pub absolute_ttl: Duration,
    /// How long a rotated refresh token keeps answering with its
    /// replacement. Default 15 seconds.
    pub grace: Duration,
    /// Base cookie names (see [`SecureCookies`]). Defaults `at`, `rt`, `did`.
    pub access_cookie: String,
    pub refresh_cookie: String,
    pub device_cookie: String,
    /// More base names to clear whenever the sign-in state is cleared
    /// (a pending challenge, a passkey ceremony).
    pub also_clear: Vec<String>,
    /// Include the user agent in the access token's browser binding.
    /// Default true; a browser update then costs one refresh, nothing more.
    pub bind_user_agent: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        const DAY: u64 = 24 * 60 * 60;
        SessionConfig {
            access_ttl: Duration::from_secs(15 * 60),
            refresh_ttl: Duration::from_secs(30 * DAY),
            absolute_ttl: Duration::from_secs(365 * DAY),
            grace: Duration::from_secs(15),
            access_cookie: "at".into(),
            refresh_cookie: "rt".into(),
            device_cookie: "did".into(),
            also_clear: Vec::new(),
            bind_user_agent: true,
        }
    }
}

/// A verified session: what [`Sessions::current`] returns and what pages
/// below [`Sessions::require`] take as an argument.
#[derive(Clone, Debug)]
pub struct CurrentSession<U> {
    pub user: U,
    pub user_id: String,
    pub session_id: String,
    pub family_id: String,
    /// A second factor was used when this family was opened.
    pub mfa_verified: bool,
}

impl<U: Clone + Send + Sync + 'static> FromContext for CurrentSession<U> {
    /// The session verified by the middleware above this page; without one
    /// (no middleware, or [`Sessions::load`] and a guest) the page is a 401.
    fn from_context(ctx: &Ctx) -> next_rust_server::Result<Self> {
        ctx.extensions
            .get::<CurrentSession<U>>()
            .cloned()
            .ok_or_else(|| next_rust_server::Error::http(401, "Sign in to continue."))
    }
}

/// A rotated refresh token came back after its grace window.
#[derive(Clone, Debug)]
pub struct TheftEvent {
    pub user_id: String,
    pub family_id: String,
    /// The retired session whose token was presented.
    pub session_id: String,
    /// Masked address of the request that presented it.
    pub network: String,
    pub user_agent: String,
}

/// Why the engine could not answer.
#[derive(Debug)]
pub enum SessionError {
    /// The refresh limit for this network is spent.
    Throttled(Throttled),
    Store(StoreError),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::Throttled(t) => t.fmt(f),
            SessionError::Store(e) => write!(f, "session store: {e}"),
        }
    }
}

impl From<StoreError> for SessionError {
    fn from(e: StoreError) -> Self {
        SessionError::Store(e)
    }
}

impl From<SessionError> for next_rust_server::Error {
    /// 429 when throttled; otherwise a 500 whose detail stays in the log.
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::Throttled(t) => t.into(),
            SessionError::Store(e) => next_rust_server::Error::from(e),
        }
    }
}

/// A session opened by [`Sessions::establish`].
#[derive(Clone, Debug)]
pub struct Established {
    pub session_id: String,
    pub family_id: String,
    pub device_id: String,
}

#[derive(Serialize, Deserialize)]
struct AccessClaims {
    sid: String,
    uid: String,
    fam: String,
    /// Hash of this browser's device id (and user agent).
    bind: String,
    mfa: bool,
}

type TheftHook = Arc<dyn Fn(&TheftEvent) + Send + Sync>;

struct Inner<S> {
    store: S,
    keys: AuthKeys,
    config: SessionConfig,
    local_grace: Mutex<HashMap<String, (Instant, Grace)>>,
    shared_grace: Option<Arc<dyn GraceStore>>,
    refresh_limit: Option<(RateLimiter, Quota)>,
    on_theft: Option<TheftHook>,
}

/// The session engine. Make one at startup and share it (it is an `Arc`
/// inside; cloning is cheap).
pub struct Sessions<S: SessionStore> {
    inner: Arc<Inner<S>>,
}

impl<S: SessionStore> Clone for Sessions<S> {
    fn clone(&self) -> Self {
        Sessions { inner: self.inner.clone() }
    }
}

impl<S: SessionStore> Sessions<S> {
    pub fn new(store: S, keys: AuthKeys) -> Self {
        Sessions {
            inner: Arc::new(Inner {
                store,
                keys,
                config: SessionConfig::default(),
                local_grace: Mutex::new(HashMap::new()),
                shared_grace: None,
                refresh_limit: None,
                on_theft: None,
            }),
        }
    }

    fn configure(mut self, f: impl FnOnce(&mut Inner<S>)) -> Self {
        f(Arc::get_mut(&mut self.inner).expect("configure Sessions before sharing it"));
        self
    }

    /// Builder methods are for startup: they panic once the engine has been cloned.
    pub fn config(self, config: SessionConfig) -> Self {
        self.configure(|inner| inner.config = config)
    }

    /// Share grace entries between instances.
    pub fn shared_grace(self, store: impl GraceStore) -> Self {
        self.configure(|inner| inner.shared_grace = Some(Arc::new(store)))
    }

    /// Limit rotations per network (suggested: `Quota::new(600, 900)`). A
    /// rotation costs a transaction; nothing else bounds a flood of made-up
    /// refresh tokens.
    pub fn limit_refresh(self, limiter: RateLimiter, quota: Quota) -> Self {
        self.configure(|inner| inner.refresh_limit = Some((limiter, quota)))
    }

    /// Called after a family was ended for token reuse: write the audit
    /// event, mail the owner (spawn anything slow).
    pub fn on_theft(self, hook: impl Fn(&TheftEvent) + Send + Sync + 'static) -> Self {
        self.configure(|inner| inner.on_theft = Some(Arc::new(hook)))
    }

    pub fn store(&self) -> &S {
        &self.inner.store
    }

    pub fn keys(&self) -> &AuthKeys {
        &self.inner.keys
    }

    fn cookies(&self, visitor: &Visitor) -> SecureCookies {
        SecureCookies::new(&visitor.cookies)
    }

    /// This browser's device id, minted on first use.
    pub fn device_id(&self, visitor: &Visitor) -> String {
        let cookies = self.cookies(visitor);
        let name = &self.inner.config.device_cookie;
        if let Some(id) = cookies
            .get(name)
            .filter(|v| v.len() == 22 && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
        {
            return id;
        }
        let fresh = crypto::short_token();
        cookies.set(name, &fresh, MAX_COOKIE_LIFETIME);
        fresh
    }

    /// Ties an access token to the browser it was issued to: a copied access
    /// cookie replayed elsewhere does not verify (the refresh path decides).
    fn binding(&self, visitor: &Visitor) -> String {
        let device = self.cookies(visitor).get(&self.inner.config.device_cookie).unwrap_or_default();
        let agent = if self.inner.config.bind_user_agent { visitor.user_agent.as_str() } else { "" };
        crypto::sha256_hex(&format!("{device}|{agent}"))[..32].to_owned()
    }

    fn set_session_cookies(&self, visitor: &Visitor, claims: &AccessClaims, refresh: &str, expires_at: u64) {
        let config = &self.inner.config;
        let cookies = self.cookies(visitor);
        if let Ok(access) = self.inner.keys.seal(Purpose::ACCESS, claims, config.access_ttl) {
            cookies.set(&config.access_cookie, &access, config.access_ttl.max(Duration::from_secs(1)));
        }
        let lifetime = expires_at.saturating_sub(crypto::now()).max(60);
        cookies.set(&config.refresh_cookie, refresh, Duration::from_secs(lifetime));
    }

    /// Forgets the sign-in on this browser (the device id stays: it names
    /// the browser, not the person).
    pub fn clear_cookies(&self, visitor: &Visitor) {
        let config = &self.inner.config;
        let cookies = self.cookies(visitor);
        cookies.clear(&config.access_cookie);
        cookies.clear(&config.refresh_cookie);
        for base in &config.also_clear {
            cookies.clear(base);
        }
    }

    /// Opens a session for `user_id` on this browser and sets its cookies.
    /// Call it only after every factor you require has been checked.
    pub async fn establish(
        &self,
        visitor: &Visitor,
        user_id: &str,
        mfa_verified: bool,
    ) -> Result<Established, SessionError> {
        let config = &self.inner.config;
        let known_device = self.cookies(visitor).get(&config.device_cookie);
        let device_id = self.device_id(visitor);
        // Renewed at every sign-in, so it always outlives the session it names.
        if known_device.as_deref() == Some(device_id.as_str()) {
            self.cookies(visitor).set(&config.device_cookie, &device_id, MAX_COOKIE_LIFETIME);
        }
        // Signing in over an open session replaces it: the old record must
        // not stay usable behind a cookie this browser is about to overwrite.
        if let Some(raw) = self.cookies(visitor).get(&config.refresh_cookie).filter(|raw| is_refresh_token(raw))
            && let Some((previous, lock)) = self.inner.store.find_by_refresh_hash(&crypto::sha256_hex(&raw)).await?
        {
            drop(lock);
            if previous.revoked.is_none() {
                self.inner.store.revoke(&previous.id, RevokeReason::Other("superseded".into())).await?;
            }
        }
        let refresh = crypto::secret_token();
        let now = crypto::now();
        let record = SessionRecord {
            id: crypto::uuid_v4(),
            user_id: user_id.to_owned(),
            family_id: crypto::uuid_v4(),
            refresh_hash: crypto::sha256_hex(&refresh),
            device_id: device_id.clone(),
            mfa_verified,
            created_at: now,
            expires_at: now.saturating_add(config.refresh_ttl.as_secs().min(config.absolute_ttl.as_secs())),
            absolute_expires_at: now.saturating_add(config.absolute_ttl.as_secs()),
            revoked: None,
            ip: visitor.ip,
            user_agent: visitor.user_agent.clone(),
        };
        self.inner.store.create(&record).await?;
        let claims = AccessClaims {
            sid: record.id.clone(),
            uid: record.user_id.clone(),
            fam: record.family_id.clone(),
            bind: self.binding(visitor),
            mfa: mfa_verified,
        };
        self.set_session_cookies(visitor, &claims, &refresh, record.expires_at);
        for base in &config.also_clear {
            self.cookies(visitor).clear(base);
        }
        Ok(Established { session_id: record.id, family_id: record.family_id, device_id })
    }

    /// The verified session of this request, rotating an expired access
    /// token on the way. Ended and invalid sessions have their cookies
    /// cleared. Safe to call more than once per request: cookies set by the
    /// first call are what the second one reads.
    pub async fn current(&self, visitor: &Visitor) -> Result<Option<CurrentSession<S::User>>, SessionError> {
        let config = &self.inner.config;
        let cookies = self.cookies(visitor);
        if let Some(raw) = cookies.get(&config.access_cookie)
            && let Ok(claims) = self.inner.keys.open::<AccessClaims>(Purpose::ACCESS, &raw)
            && crypto::constant_time_eq(claims.bind.as_bytes(), self.binding(visitor).as_bytes())
        {
            return match self.inner.store.is_live(&claims.sid, &claims.uid, crypto::now()).await? {
                Some(user) => Ok(Some(CurrentSession {
                    user,
                    user_id: claims.uid,
                    session_id: claims.sid,
                    family_id: claims.fam,
                    mfa_verified: claims.mfa,
                })),
                None => {
                    self.clear_cookies(visitor);
                    Ok(None)
                }
            };
        }
        if cookies.get(&config.refresh_cookie).is_some() {
            return self.rotate(visitor).await;
        }
        // A stray access cookie without a refresh cookie is useless now.
        cookies.clear(&config.access_cookie);
        Ok(None)
    }

    async fn resume(&self, visitor: &Visitor, grace: Grace) -> Result<Option<CurrentSession<S::User>>, SessionError> {
        let claims = AccessClaims {
            sid: grace.sid,
            uid: grace.uid,
            fam: grace.fam,
            bind: self.binding(visitor),
            mfa: grace.mfa,
        };
        self.set_session_cookies(visitor, &claims, &grace.refresh, grace.expires_at);
        // Keep the device cookie alive as long as the session is used.
        let cookies = self.cookies(visitor);
        if let Some(device) = cookies.get(&self.inner.config.device_cookie) {
            cookies.set(&self.inner.config.device_cookie, &device, MAX_COOKIE_LIFETIME);
        }
        match self.inner.store.is_live(&claims.sid, &claims.uid, crypto::now()).await? {
            Some(user) => Ok(Some(CurrentSession {
                user,
                user_id: claims.uid,
                session_id: claims.sid,
                family_id: claims.fam,
                mfa_verified: claims.mfa,
            })),
            None => {
                self.clear_cookies(visitor);
                Ok(None)
            }
        }
    }

    /// Rotates the refresh token: the old record is retired, a new one in
    /// the same family takes over, and new cookies are set.
    async fn rotate(&self, visitor: &Visitor) -> Result<Option<CurrentSession<S::User>>, SessionError> {
        let inner = &self.inner;
        if let Some((limiter, quota)) = &inner.refresh_limit {
            limiter
                .check(&key::scoped("refresh:net", &visitor.network()), *quota)
                .await
                .map_err(SessionError::Throttled)?;
        }
        let Some(raw) = self.cookies(visitor).get(&inner.config.refresh_cookie) else { return Ok(None) };
        // Anything that is not the shape of a refresh token never reaches the store.
        if !is_refresh_token(&raw) {
            self.clear_cookies(visitor);
            return Ok(None);
        }
        let presented = crypto::sha256_hex(&raw);

        let Some((old, lock)) = inner.store.find_by_refresh_hash(&presented).await? else {
            self.clear_cookies(visitor);
            return Ok(None);
        };

        if let Some(reason) = &old.revoked {
            drop(lock);
            // Grace is for this family's own browser: the device cookie must
            // come along. A refresh token on its own is a copy.
            let device = self.cookies(visitor).get(&inner.config.device_cookie).unwrap_or_default();
            let same_device =
                !old.device_id.is_empty() && crypto::constant_time_eq(device.as_bytes(), old.device_id.as_bytes());
            if same_device && let Some(grace) = self.find_grace(&presented).await {
                return self.resume(visitor, grace).await;
            }
            if *reason == RevokeReason::Rotated {
                self.theft(visitor, &old).await;
            }
            self.clear_cookies(visitor);
            return Ok(None);
        }

        let now = crypto::now();
        let usable = old.expires_at > now
            && old.absolute_expires_at > now
            && inner.store.is_live(&old.id, &old.user_id, now).await?.is_some();
        if !usable {
            drop(lock);
            self.clear_cookies(visitor);
            return Ok(None);
        }

        let refresh = crypto::secret_token();
        let new = SessionRecord {
            id: crypto::uuid_v4(),
            refresh_hash: crypto::sha256_hex(&refresh),
            created_at: now,
            expires_at: now.saturating_add(inner.config.refresh_ttl.as_secs()).min(old.absolute_expires_at),
            revoked: None,
            ip: visitor.ip,
            user_agent: visitor.user_agent.clone(),
            ..old.clone()
        };
        let grace = Grace {
            sid: new.id.clone(),
            uid: new.user_id.clone(),
            fam: new.family_id.clone(),
            refresh,
            expires_at: new.expires_at,
            mfa: new.mfa_verified,
        };
        // Parked before the commit: a concurrent request waiting on the lock
        // must find the replacement the moment it sees the old record retired.
        self.park_grace(&presented, &grace).await;
        if let Err(e) = inner.store.rotate(lock, &old, &new).await {
            inner.local_grace.lock().unwrap_or_else(|e| e.into_inner()).remove(&presented);
            return Err(e.into());
        }
        self.resume(visitor, grace).await
    }

    async fn park_grace(&self, presented_hash: &str, grace: &Grace) {
        let ttl = self.inner.config.grace;
        if ttl.is_zero() {
            return;
        }
        {
            let mut map = self.inner.local_grace.lock().unwrap_or_else(|e| e.into_inner());
            map.retain(|_, (at, _)| at.elapsed() < ttl);
            map.insert(presented_hash.to_owned(), (Instant::now(), grace.clone()));
        }
        if let Some(shared) = &self.inner.shared_grace
            && let Ok(sealed) = self.inner.keys.seal(Purpose::GRACE, grace, ttl.max(Duration::from_secs(1)))
        {
            shared.park(presented_hash, &sealed, ttl).await;
        }
    }

    async fn find_grace(&self, presented_hash: &str) -> Option<Grace> {
        let ttl = self.inner.config.grace;
        let local = {
            let mut map = self.inner.local_grace.lock().unwrap_or_else(|e| e.into_inner());
            // Entries hold live refresh tokens: none stays past its window.
            map.retain(|_, (at, _)| at.elapsed() < ttl);
            map.get(presented_hash).filter(|(at, _)| at.elapsed() < ttl).map(|(_, g)| g.clone())
        };
        if local.is_some() {
            return local;
        }
        let sealed = self.inner.shared_grace.as_ref()?.find(presented_hash).await?;
        self.inner.keys.open::<Grace>(Purpose::GRACE, &sealed).ok()
    }

    /// A retired refresh token came back: someone has a copy. Every session
    /// in its family ends and the hook is told.
    async fn theft(&self, visitor: &Visitor, old: &SessionRecord) {
        if let Err(e) = self.inner.store.revoke_family(&old.family_id, RevokeReason::ReuseDetected).await {
            next_rust_server::log::error(&format!(
                "auth: could not end family {} after token reuse: {e}",
                old.family_id
            ));
        }
        if let Some(hook) = &self.inner.on_theft {
            hook(&TheftEvent {
                user_id: old.user_id.clone(),
                family_id: old.family_id.clone(),
                session_id: old.id.clone(),
                network: visitor.network(),
                user_agent: visitor.user_agent.clone(),
            });
        }
    }

    /// Ends this browser's session family (found by its refresh token, or
    /// by the access token when only that is left) and clears its cookies.
    /// Returns the user who was signed out, for your audit log.
    ///
    /// The *family* ends, not just the presented record: when another tab
    /// rotated the token a moment ago, this request still carries the retired
    /// one, and ending only that record would leave the replacement signed in.
    pub async fn sign_out(&self, visitor: &Visitor) -> Option<String> {
        let config = &self.inner.config;
        let cookies = self.cookies(visitor);
        let mut target: Option<(String, String)> = None;
        if let Some(raw) = cookies.get(&config.refresh_cookie).filter(|raw| is_refresh_token(raw))
            && let Ok(Some((record, lock))) = self.inner.store.find_by_refresh_hash(&crypto::sha256_hex(&raw)).await
        {
            drop(lock);
            target = Some((record.family_id, record.user_id));
        }
        if target.is_none()
            && let Some(raw) = cookies.get(&config.access_cookie)
            && let Ok(claims) = self.inner.keys.open::<AccessClaims>(Purpose::ACCESS, &raw)
        {
            target = Some((claims.fam, claims.uid));
        }
        self.clear_cookies(visitor);
        let (family_id, user_id) = target?;
        match self.inner.store.revoke_family(&family_id, RevokeReason::Logout).await {
            Ok(0) => None,
            Ok(_) => Some(user_id),
            Err(e) => {
                next_rust_server::log::error(&format!("auth: sign-out could not end family {family_id}: {e}"));
                None
            }
        }
    }

    /// Ends every session of `user_id` except `keep` (pass the current
    /// session's id after a password change); how many ended.
    pub async fn revoke_all(&self, user_id: &str, reason: &str, keep: Option<&str>) -> Result<u64, SessionError> {
        Ok(self.inner.store.revoke_user(user_id, RevokeReason::parse(reason), keep).await?)
    }

    /// Middleware: requests without a live session are sent to `login`
    /// (`GET`/`HEAD`, with `?next=` set to the local path they wanted) or
    /// answered `401`. With one, the page below can take
    /// [`CurrentSession<U>`] (or the framework's `Auth<U>`), and the response
    /// is marked `Cache-Control: private, no-store`.
    pub fn require(&self, login: &str) -> SessionGate<S> {
        SessionGate { sessions: self.clone(), login: Some(login.to_owned()) }
    }

    /// Middleware: verifies a session when there is one and lets guests
    /// through. Pages read `Auth<U>`.
    pub fn load(&self) -> SessionGate<S> {
        SessionGate { sessions: self.clone(), login: None }
    }
}

/// The middleware made by [`Sessions::require`] and [`Sessions::load`].
pub struct SessionGate<S: SessionStore> {
    sessions: Sessions<S>,
    login: Option<String>,
}

impl<S: SessionStore> Middleware for SessionGate<S> {
    fn handle(&self, mut req: Request, next: Next) -> BoxFuture<Response> {
        let sessions = self.sessions.clone();
        let login = self.login.clone();
        Box::pin(async move {
            match sessions.current(&Visitor::from_request(&req)).await {
                Ok(Some(session)) => {
                    req.insert_extension(AuthUser(session.user.clone()));
                    req.insert_extension(session);
                    // Pages that know who is looking must never be stored by
                    // a shared cache or the back-forward cache.
                    next.run(req).await.with_header("cache-control", "private, no-store")
                }
                Ok(None) => match login {
                    None => next.run(req).await,
                    Some(login) if matches!(*req.method(), http::Method::GET | http::Method::HEAD) => {
                        let here = req.uri().path_and_query().map_or("/", |p| p.as_str());
                        let here = if next_rust_server::is_local_path(here) { here } else { "/" };
                        let join = if login.contains('?') { '&' } else { '?' };
                        Response::redirect(&format!("{login}{join}next={}", encode_query_value(here)))
                    }
                    Some(_) => Response::text("Unauthorized").with_status(401),
                },
                Err(SessionError::Throttled(throttled)) => throttled.response(),
                Err(SessionError::Store(e)) => {
                    next_rust_server::log::error(&format!("auth: session check failed: {e}"));
                    Response::text("Sign-in is temporarily unavailable. Please try again in a few minutes.")
                        .with_status(503)
                        .with_header("retry-after", "30")
                }
            }
        })
    }
}

/// The shape of [`crypto::secret_token`]: 43 base64url characters.
fn is_refresh_token(raw: &str) -> bool {
    raw.len() == 43 && raw.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Percent-encodes everything but unreserved characters, so a `next` value
/// can never add parameters to the login URL.
fn encode_query_value(s: &str) -> String {
    use std::fmt::Write;
    s.bytes().fold(String::with_capacity(s.len()), |mut out, b| {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
        out
    })
}
