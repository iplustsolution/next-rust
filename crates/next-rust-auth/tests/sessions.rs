//! The session engine against the in-memory store: a small "browser" carries
//! cookies from one request to the next.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use next_rust_auth::limits::{Quota, RateLimiter};
use next_rust_auth::{
    AuthKeys, CurrentSession, MemorySessionStore, RevokeReason, SessionConfig, SessionError, SessionStore, Sessions,
    TheftEvent, Visitor,
};
use next_rust_server::middleware::Endpoint;
use next_rust_server::{AuthUser, Cookies, Middleware, Next, Request, Response};

#[derive(Clone, Debug, PartialEq)]
struct User {
    name: String,
}

type Store = MemorySessionStore<User>;

const UA: &str = "Mozilla/5.0 (Macintosh) Firefox/140.0";

/// Cookies a browser holds; `secure` is how the server sees the connection.
#[derive(Clone, Default)]
struct Browser {
    cookies: BTreeMap<String, String>,
    secure: bool,
    user_agent: &'static str,
}

impl Browser {
    fn new() -> Self {
        Browser { cookies: BTreeMap::new(), secure: true, user_agent: UA }
    }

    fn headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();
        if !self.cookies.is_empty() {
            let line = self.cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
            headers.insert("cookie", line.parse().unwrap());
        }
        headers.insert("user-agent", self.user_agent.parse().unwrap());
        headers
    }

    /// A request's view of this browser.
    fn visit(&self) -> Visitor {
        let headers = self.headers();
        let jar = Cookies::from_headers(&headers, self.secure);
        Visitor::new(&headers, &jar, "203.0.113.77".parse().ok())
    }

    /// Applies the response's `Set-Cookie` headers.
    fn receive(&mut self, jar: &Cookies) {
        for header in jar.set_cookie_headers() {
            let (pair, attributes) = header.split_once(';').unwrap_or((&header, ""));
            let (name, value) = pair.split_once('=').unwrap();
            if attributes.contains("Max-Age=0") {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.to_owned(), value.to_owned());
            }
        }
    }

    /// Drops the access cookie: what 15 idle minutes do.
    fn expire_access(&mut self) {
        self.cookies.retain(|name, _| !name.ends_with("at"));
    }

    fn has(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }
}

const SECRET: &[u8] = b"sessions-test secret: 0123456789 abcdefghijklmnop";

fn keys() -> AuthKeys {
    AuthKeys::from_secret(SECRET).unwrap()
}

fn engine() -> (Sessions<Store>, Store) {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    (Sessions::new(store.clone(), keys()), store)
}

/// One request: `current()` and the cookies it set go back to the browser.
async fn request(sessions: &Sessions<Store>, browser: &mut Browser) -> Option<CurrentSession<User>> {
    let visitor = browser.visit();
    let session = sessions.current(&visitor).await.expect("store works");
    browser.receive(&visitor.cookies);
    session
}

async fn sign_in(sessions: &Sessions<Store>, browser: &mut Browser) -> String {
    let visitor = browser.visit();
    let opened = sessions.establish(&visitor, "u1", true).await.unwrap();
    browser.receive(&visitor.cookies);
    opened.session_id
}

#[tokio::test]
async fn sign_in_sets_host_cookies_and_the_session_verifies() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    assert!(request(&sessions, &mut browser).await.is_none(), "a guest");

    let sid = sign_in(&sessions, &mut browser).await;
    assert!(browser.has("__Host-at") && browser.has("__Host-rt") && browser.has("__Host-did"), "{:?}", browser.cookies);

    let session = request(&sessions, &mut browser).await.expect("signed in");
    assert_eq!(session.user, User { name: "Ada".into() });
    assert_eq!(
        (session.session_id.as_str(), session.user_id.as_str(), session.mfa_verified),
        (sid.as_str(), "u1", true)
    );

    let record = store.session(&sid).unwrap();
    assert_eq!(record.refresh_hash.len(), 64);
    assert_ne!(record.refresh_hash, browser.cookies["__Host-rt"], "only the hash is stored");
    assert_eq!(record.refresh_hash, next_rust_auth::crypto::sha256_hex(&browser.cookies["__Host-rt"]));
    assert_eq!(record.device_id, browser.cookies["__Host-did"]);
    assert_eq!(record.user_agent, UA);
    assert_eq!(record.id.len(), 36, "a UUID");
}

#[tokio::test]
async fn development_cookies_have_no_prefix() {
    let (sessions, _) = engine();
    let mut browser = Browser { secure: false, ..Browser::new() };
    sign_in(&sessions, &mut browser).await;
    assert!(browser.has("at") && browser.has("rt") && !browser.has("__Host-at"));
    assert!(request(&sessions, &mut browser).await.is_some());
}

#[tokio::test]
async fn every_request_asks_the_store() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    let sid = sign_in(&sessions, &mut browser).await;

    // Revoked elsewhere (an admin, "sign out everywhere"): the still-valid
    // access token stops working on the next request.
    store.revoke(&sid, RevokeReason::Other("admin".into())).await.unwrap();
    assert!(request(&sessions, &mut browser).await.is_none());
    assert!(!browser.has("__Host-at") && !browser.has("__Host-rt"), "cookies cleared");
    assert!(browser.has("__Host-did"), "the device id names the browser, not the person");

    // A suspended account, same thing.
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;
    store.remove_user("u1");
    assert!(request(&sessions, &mut browser).await.is_none());
    // ... and its refresh token does not bring it back.
    let mut browser = Browser::new();
    store.insert_user("u1", User { name: "Ada".into() });
    sign_in(&sessions, &mut browser).await;
    store.remove_user("u1");
    browser.expire_access();
    assert!(request(&sessions, &mut browser).await.is_none());
}

#[tokio::test]
async fn a_copied_access_cookie_does_not_verify_in_another_browser() {
    let (sessions, _) = engine();
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;

    // Another user agent, only the access cookie (and the device id) copied.
    let mut thief = Browser { user_agent: "curl/8", ..browser.clone() };
    thief.cookies.remove("__Host-rt");
    assert!(request(&sessions, &mut thief).await.is_none(), "the binding does not match");
    assert!(!thief.has("__Host-at"));
    assert!(request(&sessions, &mut browser).await.is_some(), "the owner is unaffected");

    // Forged and foreign-key tokens are just as useless.
    let mut forged = Browser::new();
    forged.cookies.insert("__Host-at".into(), "v1.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into());
    assert!(request(&sessions, &mut forged).await.is_none());
}

#[tokio::test]
async fn an_expired_access_token_rotates_the_refresh_token() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    let first = sign_in(&sessions, &mut browser).await;
    let first_refresh = browser.cookies["__Host-rt"].clone();

    browser.expire_access();
    let session = request(&sessions, &mut browser).await.expect("rotated");
    assert_ne!(session.session_id, first);
    assert_ne!(browser.cookies["__Host-rt"], first_refresh);
    assert!(browser.has("__Host-at"), "a new access token");
    assert!(session.mfa_verified, "carried over");

    let old = store.session(&first).unwrap();
    let new = store.session(&session.session_id).unwrap();
    assert_eq!(old.revoked, Some(RevokeReason::Rotated));
    assert_eq!((new.revoked.clone(), &new.family_id), (None, &old.family_id));
    assert_eq!(new.absolute_expires_at, old.absolute_expires_at, "the family's end does not move");
    assert_eq!(store.sessions_of("u1").len(), 2);

    // The new cookies keep working without another rotation.
    let again = request(&sessions, &mut browser).await.unwrap();
    assert_eq!(again.session_id, session.session_id);
    assert_eq!(store.sessions_of("u1").len(), 2);
}

#[tokio::test]
async fn a_changed_secret_costs_one_refresh_and_a_previous_key_costs_nothing() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let before = Sessions::new(store.clone(), keys());
    let mut browser = Browser::new();
    let sid = sign_in(&before, &mut browser).await;

    let new_secret = b"sessions-test next secret: 9876543210 zyxwvutsrq";
    let with_previous =
        Sessions::new(store.clone(), AuthKeys::from_secret(new_secret).unwrap().with_previous(SECRET).unwrap());
    assert_eq!(request(&with_previous, &mut browser).await.unwrap().session_id, sid, "old access token still opens");

    let without = Sessions::new(store.clone(), AuthKeys::from_secret(new_secret).unwrap());
    let rotated = request(&without, &mut browser).await.expect("the refresh token is not sealed; it still works");
    assert_ne!(rotated.session_id, sid);
}

#[tokio::test]
async fn concurrent_refreshes_share_one_replacement() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;
    browser.expire_access();

    // Two tabs wake up together and present the same refresh token.
    let (tab_a, tab_b) = (browser.visit(), browser.visit());
    let (a, b) = tokio::join!(sessions.current(&tab_a), sessions.current(&tab_b));
    let (a, b) = (a.unwrap().expect("tab a"), b.unwrap().expect("tab b"));
    assert_eq!(a.session_id, b.session_id, "the same replacement, not a theft alarm");
    assert_eq!(store.sessions_of("u1").len(), 2, "one rotation");

    let (mut after_a, mut after_b) = (browser.clone(), browser.clone());
    after_a.receive(&tab_a.cookies);
    after_b.receive(&tab_b.cookies);
    assert_eq!(after_a.cookies["__Host-rt"], after_b.cookies["__Host-rt"]);
    assert!(request(&sessions, &mut after_b).await.is_some());
}

#[tokio::test(start_paused = true)]
async fn a_late_retry_inside_the_grace_window_is_forgiven() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;
    browser.expire_access();

    // The response to the rotation is lost: the browser keeps its old cookies.
    let lost = browser.visit();
    let rotated = sessions.current(&lost).await.unwrap().unwrap();
    tokio::time::advance(Duration::from_secs(10)).await;
    let retried = request(&sessions, &mut browser).await.expect("within 15 seconds");
    assert_eq!(retried.session_id, rotated.session_id);
    assert!(store.session(&rotated.session_id).unwrap().revoked.is_none());
}

#[tokio::test(start_paused = true)]
async fn a_rotated_token_coming_back_ends_the_family() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let alarms: Arc<Mutex<Vec<TheftEvent>>> = Arc::default();
    let seen = alarms.clone();
    let sessions = Sessions::new(store.clone(), keys()).on_theft(move |event| seen.lock().unwrap().push(event.clone()));

    let mut owner = Browser::new();
    let first = sign_in(&sessions, &mut owner).await;
    // Someone copies the cookies, then the owner carries on and rotates.
    let mut thief = Browser { user_agent: "curl/8", ..owner.clone() };
    owner.expire_access();
    let current = request(&sessions, &mut owner).await.unwrap();

    // A second family of the same user, to show it is left alone.
    let mut phone = Browser::new();
    let phone_sid = sign_in(&sessions, &mut phone).await;

    tokio::time::advance(Duration::from_secs(16)).await;
    thief.expire_access();
    assert!(request(&sessions, &mut thief).await.is_none(), "the copy is refused");
    assert!(!thief.has("__Host-rt"));

    assert_eq!(store.session(&current.session_id).unwrap().revoked, Some(RevokeReason::ReuseDetected));
    assert_eq!(store.session(&first).unwrap().revoked, Some(RevokeReason::Rotated), "already closed; reason kept");
    assert!(request(&sessions, &mut owner).await.is_none(), "the owner signs in again: nobody can tell who was first");
    assert!(store.session(&phone_sid).unwrap().revoked.is_none());
    assert!(request(&sessions, &mut phone).await.is_some());

    let alarms = alarms.lock().unwrap();
    assert_eq!(alarms.len(), 1);
    assert_eq!((alarms[0].user_id.as_str(), alarms[0].session_id.as_str()), ("u1", first.as_str()));
    assert_eq!(alarms[0].network, "203.0.113.0/24");
    assert_eq!(alarms[0].user_agent, "curl/8");
}

#[tokio::test(start_paused = true)]
async fn a_signed_out_token_coming_back_is_not_theft() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let alarms = Arc::new(Mutex::new(0));
    let seen = alarms.clone();
    let sessions = Sessions::new(store.clone(), keys()).on_theft(move |_| *seen.lock().unwrap() += 1);

    let mut browser = Browser::new();
    let sid = sign_in(&sessions, &mut browser).await;
    let stale = browser.clone();

    let visitor = browser.visit();
    assert_eq!(sessions.sign_out(&visitor).await.as_deref(), Some("u1"));
    browser.receive(&visitor.cookies);
    assert!(!browser.has("__Host-at") && !browser.has("__Host-rt"));
    assert_eq!(store.session(&sid).unwrap().revoked, Some(RevokeReason::Logout));

    let mut stale = stale;
    assert!(request(&sessions, &mut stale.clone()).await.is_none(), "the access token died with the session");
    stale.expire_access();
    assert!(request(&sessions, &mut stale).await.is_none());
    assert_eq!(*alarms.lock().unwrap(), 0);

    // Signing out twice, or as a guest, is harmless.
    assert_eq!(sessions.sign_out(&Browser::new().visit()).await, None);
}

#[tokio::test]
async fn sign_out_works_with_only_the_access_cookie() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    let sid = sign_in(&sessions, &mut browser).await;
    browser.cookies.remove("__Host-rt");
    assert_eq!(sessions.sign_out(&browser.visit()).await.as_deref(), Some("u1"));
    assert_eq!(store.session(&sid).unwrap().revoked, Some(RevokeReason::Logout));
}

#[tokio::test]
async fn refresh_tokens_expire_and_unknown_ones_are_ignored() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let config = SessionConfig { refresh_ttl: Duration::ZERO, ..SessionConfig::default() };
    let sessions = Sessions::new(store.clone(), keys()).config(config);
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;
    assert!(
        request(&sessions, &mut browser.clone()).await.is_none(),
        "the record has expired, whatever the token says"
    );
    browser.expire_access();
    assert!(request(&sessions, &mut browser).await.is_none());
    assert_eq!(store.sessions_of("u1").len(), 1, "nothing was rotated");

    let mut made_up = Browser::new();
    made_up.cookies.insert("__Host-rt".into(), "x".repeat(43));
    assert!(request(&sessions, &mut made_up).await.is_none());
    assert!(!made_up.has("__Host-rt"));
}

#[tokio::test]
async fn a_rotation_never_outlives_the_family() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let config = SessionConfig {
        refresh_ttl: Duration::from_secs(3600),
        absolute_ttl: Duration::from_secs(600),
        ..SessionConfig::default()
    };
    let sessions = Sessions::new(store.clone(), keys()).config(config);
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;
    browser.expire_access();
    let rotated = request(&sessions, &mut browser).await.unwrap();
    let record = store.session(&rotated.session_id).unwrap();
    assert_eq!(record.expires_at, record.absolute_expires_at);
}

#[tokio::test]
async fn revoke_all_keeps_the_session_that_asked() {
    let (sessions, _) = engine();
    let (mut laptop, mut phone) = (Browser::new(), Browser::new());
    let keep = sign_in(&sessions, &mut laptop).await;
    sign_in(&sessions, &mut phone).await;
    assert_eq!(sessions.revoke_all("u1", "password_changed", Some(&keep)).await.unwrap(), 1);
    assert!(request(&sessions, &mut laptop).await.is_some());
    assert!(request(&sessions, &mut phone).await.is_none());
    assert_eq!(sessions.revoke_all("u1", "logout", None).await.unwrap(), 1);
}

#[tokio::test]
async fn rotations_are_limited_per_network() {
    let store = Store::new();
    let sessions = Sessions::new(store, keys()).limit_refresh(RateLimiter::memory(), Quota::new(2, 900));
    let mut flood = Browser::new();
    flood.cookies.insert("__Host-rt".into(), "y".repeat(43));
    for _ in 0..2 {
        assert!(sessions.current(&flood.visit()).await.unwrap().is_none());
    }
    match sessions.current(&flood.visit()).await {
        Err(SessionError::Throttled(t)) => assert!(t.retry_after_secs() > 0),
        other => panic!("expected a throttle, got {:?}", other.map(|s| s.map(|s| s.session_id))),
    }
    assert!(sessions.current(&Browser::new().visit()).await.unwrap().is_none(), "guests never reach the limiter");
}

// --- middleware -------------------------------------------------------------

fn http_request(browser: &Browser, method: &str, uri: &str) -> Request {
    let mut builder = http::Request::builder().method(method).uri(uri);
    for (name, value) in &browser.headers() {
        builder = builder.header(name, value);
    }
    Request::from_http(builder.body(Default::default()).unwrap())
}

fn endpoint() -> Arc<dyn Endpoint> {
    Arc::new(|req: Request| async move {
        let session = req.extension::<CurrentSession<User>>().map(|s| s.user.name.clone());
        let auth_user = req.extension::<AuthUser<User>>().map(|u| u.0.name.clone());
        Response::text(format!("{session:?} {auth_user:?}"))
    })
}

async fn through(gate: &impl Middleware, req: Request) -> Response {
    gate.handle(req, Next::new(Arc::from(Vec::new()), endpoint())).await
}

#[tokio::test]
async fn the_gate_redirects_guests_and_passes_sessions() {
    let (sessions, _) = engine();
    let gate = sessions.require("/login");
    // `Request::from_http` builds a development (non-secure) jar.
    let mut browser = Browser { secure: false, ..Browser::new() };

    let res = through(&gate, http_request(&browser, "GET", "/account/settings?tab=security")).await;
    assert_eq!(res.status.as_u16(), 307);
    assert_eq!(res.header("location"), Some("/login?next=%2Faccount%2Fsettings%3Ftab%3Dsecurity"));
    let res = through(&gate, http_request(&browser, "GET", "/a?x=1&next=//evil.example")).await;
    assert_eq!(res.header("location"), Some("/login?next=%2Fa%3Fx%3D1%26next%3D%2F%2Fevil.example"));
    let res = through(&gate, http_request(&browser, "POST", "/account/settings")).await;
    assert_eq!(res.status.as_u16(), 401);

    sign_in(&sessions, &mut browser).await;
    let res = through(&gate, http_request(&browser, "GET", "/account/settings")).await;
    assert_eq!(res.status.as_u16(), 200);
    assert_eq!(res.header("cache-control"), Some("private, no-store"));
    assert_eq!(res.into_text().await, r#"Some("Ada") Some("Ada")"#, "CurrentSession<U> and Auth<U> both work");
}

#[tokio::test]
async fn the_optional_gate_lets_guests_through() {
    let (sessions, _) = engine();
    let gate = sessions.load();
    let mut browser = Browser { secure: false, ..Browser::new() };
    let res = through(&gate, http_request(&browser, "GET", "/")).await;
    assert_eq!(res.status.as_u16(), 200);
    assert_eq!(res.header("cache-control"), None);
    assert_eq!(res.into_text().await, "None None");
    sign_in(&sessions, &mut browser).await;
    assert_eq!(
        through(&gate, http_request(&browser, "GET", "/")).await.into_text().await,
        r#"Some("Ada") Some("Ada")"#
    );
}

/// A store that is down.
struct Down;

impl SessionStore for Down {
    type User = User;
    type Lock = ();
    async fn create(&self, _: &next_rust_auth::SessionRecord) -> Result<(), next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
    async fn find_by_refresh_hash(
        &self,
        _: &str,
    ) -> Result<Option<(next_rust_auth::SessionRecord, ())>, next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
    async fn rotate(
        &self,
        _: (),
        _: &next_rust_auth::SessionRecord,
        _: &next_rust_auth::SessionRecord,
    ) -> Result<(), next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
    async fn revoke(&self, _: &str, _: RevokeReason) -> Result<bool, next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
    async fn revoke_family(&self, _: &str, _: RevokeReason) -> Result<u64, next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
    async fn revoke_user(&self, _: &str, _: RevokeReason, _: Option<&str>) -> Result<u64, next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
    async fn is_live(&self, _: &str, _: &str, _: u64) -> Result<Option<User>, next_rust_auth::StoreError> {
        Err(next_rust_auth::StoreError::new("connection refused"))
    }
}

#[tokio::test]
async fn a_store_outage_is_a_503_and_keeps_the_cookies() {
    let sessions = Sessions::new(Down, keys());
    let mut browser = Browser { secure: false, ..Browser::new() };
    browser.cookies.insert("rt".into(), "z".repeat(43));
    let req = http_request(&browser, "GET", "/account");
    let jar = req.cookies().clone();
    let res = through(&sessions.require("/login"), req).await;
    assert_eq!(res.status.as_u16(), 503);
    assert_eq!(res.header("retry-after"), Some("30"));
    assert!(!jar.has_changes(), "an outage signs nobody out");
    assert!(sessions.establish(&browser.visit(), "u1", false).await.is_err());
}

// ---------------------------------------------------------------------------
// Regressions from the security review
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn a_refresh_token_without_its_device_cookie_gets_no_grace() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let alarms = Arc::new(Mutex::new(0));
    let seen = alarms.clone();
    let sessions = Sessions::new(store.clone(), keys()).on_theft(move |_| *seen.lock().unwrap() += 1);

    let mut owner = Browser::new();
    sign_in(&sessions, &mut owner).await;
    // Only the refresh token leaked (a log line, a proxy), not the whole jar.
    let mut thief = Browser::new();
    thief.cookies.insert("__Host-rt".into(), owner.cookies["__Host-rt"].clone());
    let mut thief_with_wrong_device = thief.clone();
    thief_with_wrong_device.cookies.insert("__Host-did".into(), "A".repeat(22));

    owner.expire_access();
    let current = request(&sessions, &mut owner).await.unwrap();

    // Well inside the grace window: still no replacement for the copy.
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(request(&sessions, &mut thief_with_wrong_device).await.is_none());
    assert!(!thief_with_wrong_device.has("__Host-rt") && !thief_with_wrong_device.has("__Host-at"));
    assert_eq!(*alarms.lock().unwrap(), 1, "reuse is detected inside the window too");
    assert_eq!(store.session(&current.session_id).unwrap().revoked, Some(RevokeReason::ReuseDetected));
    assert!(request(&sessions, &mut thief).await.is_none());
}

#[tokio::test(start_paused = true)]
async fn the_grace_answer_is_the_same_replacement_never_a_new_one() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    sign_in(&sessions, &mut browser).await;
    browser.expire_access();
    let stale = browser.clone();
    let rotated = request(&sessions, &mut browser).await.unwrap();

    for _ in 0..5 {
        tokio::time::advance(Duration::from_secs(2)).await;
        let mut retry = stale.clone();
        let again = request(&sessions, &mut retry).await.expect("inside the window");
        assert_eq!(again.session_id, rotated.session_id);
        assert_eq!(retry.cookies["__Host-rt"], browser.cookies["__Host-rt"], "the same refresh token");
    }
    assert_eq!(store.sessions_of("u1").len(), 2, "replays inside the window mint nothing");
}

#[tokio::test]
async fn sign_out_with_a_just_rotated_token_ends_the_replacement_too() {
    let (sessions, store) = engine();
    let mut tab_a = Browser::new();
    sign_in(&sessions, &mut tab_a).await;
    tab_a.expire_access();
    // Tab B's sign-out request was built before tab A's rotation came back.
    let tab_b = tab_a.clone();
    let replacement = request(&sessions, &mut tab_a).await.unwrap();

    assert_eq!(sessions.sign_out(&tab_b.visit()).await.as_deref(), Some("u1"));
    assert_eq!(store.session(&replacement.session_id).unwrap().revoked, Some(RevokeReason::Logout));
    assert!(request(&sessions, &mut tab_a).await.is_none(), "signed out means signed out, in every tab");
}

#[tokio::test]
async fn signing_in_again_retires_the_session_it_replaces() {
    let (sessions, store) = engine();
    let mut browser = Browser::new();
    let first = sign_in(&sessions, &mut browser).await;
    let copy_of_first = browser.clone();
    let device = browser.cookies["__Host-did"].clone();
    let second = sign_in(&sessions, &mut browser).await;

    assert_ne!(first, second, "a fresh session id at every sign-in");
    assert_eq!(browser.cookies["__Host-did"], device, "the device keeps its name");
    assert_ne!(browser.cookies["__Host-rt"], copy_of_first.cookies["__Host-rt"]);
    assert_eq!(store.session(&first).unwrap().revoked, Some(RevokeReason::Other("superseded".into())));
    assert!(request(&sessions, &mut copy_of_first.clone()).await.is_none());
    assert!(request(&sessions, &mut browser).await.is_some());
}

#[tokio::test]
async fn malformed_refresh_cookies_never_reach_the_store() {
    struct Counting(Store, Arc<Mutex<u32>>);
    impl SessionStore for Counting {
        type User = User;
        type Lock = <Store as SessionStore>::Lock;
        async fn create(&self, s: &next_rust_auth::SessionRecord) -> Result<(), next_rust_auth::StoreError> {
            self.0.create(s).await
        }
        async fn find_by_refresh_hash(
            &self,
            hash: &str,
        ) -> Result<Option<(next_rust_auth::SessionRecord, Self::Lock)>, next_rust_auth::StoreError> {
            *self.1.lock().unwrap() += 1;
            self.0.find_by_refresh_hash(hash).await
        }
        async fn rotate(
            &self,
            lock: Self::Lock,
            old: &next_rust_auth::SessionRecord,
            new: &next_rust_auth::SessionRecord,
        ) -> Result<(), next_rust_auth::StoreError> {
            self.0.rotate(lock, old, new).await
        }
        async fn revoke(&self, id: &str, reason: RevokeReason) -> Result<bool, next_rust_auth::StoreError> {
            self.0.revoke(id, reason).await
        }
        async fn revoke_family(&self, id: &str, reason: RevokeReason) -> Result<u64, next_rust_auth::StoreError> {
            self.0.revoke_family(id, reason).await
        }
        async fn revoke_user(
            &self,
            id: &str,
            reason: RevokeReason,
            keep: Option<&str>,
        ) -> Result<u64, next_rust_auth::StoreError> {
            self.0.revoke_user(id, reason, keep).await
        }
        async fn is_live(&self, sid: &str, uid: &str, now: u64) -> Result<Option<User>, next_rust_auth::StoreError> {
            self.0.is_live(sid, uid, now).await
        }
    }
    let lookups = Arc::new(Mutex::new(0));
    let sessions = Sessions::new(Counting(Store::new(), lookups.clone()), keys());
    for junk in ["x", &"x".repeat(42), &"x".repeat(44), &"x".repeat(4000), &format!("{}!", "x".repeat(42)), "v1.AAAA"] {
        let mut browser = Browser::new();
        browser.cookies.insert("__Host-rt".into(), junk.to_owned());
        let visitor = browser.visit();
        assert!(sessions.current(&visitor).await.unwrap().is_none());
        assert_eq!(sessions.sign_out(&visitor).await, None);
        browser.receive(&visitor.cookies);
        assert!(!browser.has("__Host-rt"), "{junk:?} is cleared");
    }
    assert_eq!(*lookups.lock().unwrap(), 0);
}

#[tokio::test]
async fn enormous_lifetimes_saturate_instead_of_overflowing() {
    let store = Store::new();
    store.insert_user("u1", User { name: "Ada".into() });
    let config = SessionConfig { refresh_ttl: Duration::MAX, absolute_ttl: Duration::MAX, ..SessionConfig::default() };
    let sessions = Sessions::new(store.clone(), keys()).config(config);
    let mut browser = Browser::new();
    let sid = sign_in(&sessions, &mut browser).await;
    assert_eq!(store.session(&sid).unwrap().absolute_expires_at, u64::MAX);
    browser.expire_access();
    assert!(request(&sessions, &mut browser).await.is_some(), "and rotation works");
}
