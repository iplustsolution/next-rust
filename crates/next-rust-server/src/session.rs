//! Server-side sessions.
//!
//! Sessions are opt-in. The cookie holds only a random 256-bit identifier;
//! data lives in a [`SessionStore`], so nothing needs to be signed and no
//! data is exposed to the client.
//!
//! ```ignore
//! App::new(routes()).middleware(sessions(MemorySessionStore::default()))
//! // in a page:  pub fn Page(Extension(session): Extension<Session>) -> impl View
//! ```

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Cookie;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::request::Request;

pub const SESSION_COOKIE: &str = "nr_session";

/// Session storage backend.
pub trait SessionStore: Send + Sync + 'static {
    fn load(&self, id: &str) -> BoxFuture<Option<BTreeMap<String, Value>>>;
    fn save(&self, id: &str, data: BTreeMap<String, Value>, ttl: Duration) -> BoxFuture<()>;
    fn destroy(&self, id: &str) -> BoxFuture<()>;
}

type StoredSession = (BTreeMap<String, Value>, Instant);

/// In-memory store for development and single-instance deployments.
#[derive(Default, Clone)]
pub struct MemorySessionStore {
    inner: Arc<Mutex<HashMap<String, StoredSession>>>,
}

impl SessionStore for MemorySessionStore {
    fn load(&self, id: &str) -> BoxFuture<Option<BTreeMap<String, Value>>> {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let data = match map.get(id) {
            Some((d, exp)) if *exp > now => Some(d.clone()),
            Some(_) => {
                map.remove(id);
                None
            }
            None => None,
        };
        Box::pin(async move { data })
    }

    fn save(&self, id: &str, data: BTreeMap<String, Value>, ttl: Duration) -> BoxFuture<()> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).insert(id.to_owned(), (data, Instant::now() + ttl));
        Box::pin(async {})
    }

    fn destroy(&self, id: &str) -> BoxFuture<()> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
        Box::pin(async {})
    }
}

#[derive(Debug, Default)]
struct SessionState {
    data: BTreeMap<String, Value>,
    dirty: bool,
    destroyed: bool,
    regenerate: bool,
}

/// Request-scoped session handle (cheap to clone).
#[derive(Debug, Clone, Default)]
pub struct Session {
    state: Arc<Mutex<SessionState>>,
}

impl Session {
    fn lock(&self) -> std::sync::MutexGuard<'_, SessionState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.lock().data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn insert<T: Serialize>(&self, key: &str, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            let mut s = self.lock();
            s.data.insert(key.to_owned(), v);
            s.dirty = true;
        }
    }

    pub fn remove(&self, key: &str) {
        let mut s = self.lock();
        s.dirty |= s.data.remove(key).is_some();
    }

    /// Issue a new session id (call after login to prevent fixation).
    pub fn regenerate(&self) {
        let mut s = self.lock();
        s.regenerate = true;
        s.dirty = true;
    }

    /// Delete the session and its cookie (logout).
    pub fn destroy(&self) {
        self.lock().destroyed = true;
    }
}

/// Session middleware.
pub fn sessions(store: impl SessionStore) -> impl Middleware {
    sessions_with_ttl(store, Duration::from_secs(60 * 60 * 24 * 7))
}

pub fn sessions_with_ttl(store: impl SessionStore, ttl: Duration) -> impl Middleware {
    let store = Arc::new(store);
    move |mut req: Request, next: Next| {
        let store = store.clone();
        async move {
            let incoming = req
                .cookies()
                .get(SESSION_COOKIE)
                .filter(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()));
            let data = match &incoming {
                Some(id) => store.load(id).await,
                None => None,
            };
            let exists = data.is_some();
            let session = Session {
                state: Arc::new(Mutex::new(SessionState { data: data.unwrap_or_default(), ..Default::default() })),
            };
            req.insert_extension(session.clone());
            let cookies = req.cookies().clone();
            let res = next.run(req).await;
            let (data, dirty, destroyed, regenerate) = {
                let s = session.lock();
                (s.data.clone(), s.dirty, s.destroyed, s.regenerate)
            };
            if destroyed {
                if let Some(id) = &incoming {
                    store.destroy(id).await;
                }
                cookies.delete(SESSION_COOKIE);
            } else if dirty || (exists && incoming.is_some()) {
                let id = match (&incoming, regenerate, exists) {
                    (Some(id), false, true) => id.clone(),
                    (old, _, _) => {
                        if let Some(old) = old {
                            store.destroy(old).await;
                        }
                        crate::random_hex(32)
                    }
                };
                store.save(&id, data, ttl).await;
                if incoming.as_deref() != Some(id.as_str()) || dirty {
                    cookies.set(Cookie::new(SESSION_COOKIE, id).max_age(ttl));
                }
            }
            res
        }
    }
}
