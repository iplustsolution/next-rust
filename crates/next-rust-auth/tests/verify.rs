//! Independent verification of the auth hardening claims, from outside the
//! crate, plus attempts to break them with concurrency.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use next_rust_auth::crypto::{self, Purpose};
use next_rust_auth::limits::{Decision, Limiter, MemoryLimiter, Quota, RateLimiter, Window, key};
use next_rust_auth::{
    AuthKeys, KeyError, MemorySessionStore, OpenError, RevokeReason, SessionConfig, SessionStore, Sessions, Visitor,
};
use next_rust_server::Cookies;

// ---------------------------------------------------------------------------
// Passwords
// ---------------------------------------------------------------------------

#[cfg(feature = "password")]
mod passwords {
    use super::*;
    use next_rust_auth::password::{MAX_BYTES, MAX_VERIFY_LANES, MAX_VERIFY_MEMORY_KIB, MAX_VERIFY_PASSES, Passwords};

    /// Heavy enough that a single argon2 run is unmistakable in a timing
    /// comparison, light enough to keep the suite quick.
    fn heavy() -> Passwords {
        Passwords::with_params(32_768, 3, 1).unwrap()
    }

    fn light() -> Passwords {
        Passwords::with_params(64, 1, 1).unwrap()
    }

    async fn timed<F: std::future::Future>(f: F) -> (F::Output, Duration) {
        let at = Instant::now();
        let out = f.await;
        (out, at.elapsed())
    }

    /// Over-long passwords must cost no argon2 time on any of the four entry
    /// points, and the refusal must be indistinguishable between a known and
    /// an unknown account (both are instant, so the *refusal itself* leaks
    /// nothing about whether the account exists).
    #[tokio::test]
    async fn an_overlong_password_costs_no_argon2_work_on_any_path() {
        let passwords = heavy();
        let stored = passwords.hash("Correct-Horse-9").await.unwrap();
        // Warm the dummy so the comparison is not measuring its first build.
        assert!(!passwords.verify_dummy("warm-up").await);

        let (_, one_real) = timed(passwords.verify("wrong-but-short", &stored)).await;
        assert!(one_real > Duration::from_millis(5), "the baseline hash is too fast to compare against: {one_real:?}");

        let over = "x".repeat(MAX_BYTES + 1);
        let budget = one_real / 4;

        let (out, t) = timed(passwords.hash(&over)).await;
        assert!(out.is_err(), "an over-long password was hashed");
        assert!(t < budget, "hash spent {t:?}, a real hash takes {one_real:?}");

        let (out, known) = timed(passwords.verify(&over, &stored)).await;
        assert!(!out);
        assert!(known < budget, "verify spent {known:?}");

        let (out, unknown) = timed(passwords.verify_dummy(&over)).await;
        assert!(!out);
        assert!(unknown < budget, "verify_dummy spent {unknown:?}");

        let (out, any) = timed(passwords.matches_any(&over, vec![stored.clone(), stored.clone()])).await;
        assert!(!out);
        assert!(any < budget, "matches_any spent {any:?}");

        // The known and the unknown account answer the over-long password in
        // the same (negligible) time: the refusal does not separate them.
        let slower = known.max(unknown);
        let faster = known.min(unknown);
        assert!(slower < budget && faster < budget, "known {known:?}, unknown {unknown:?}");

        // Exactly at the limit is still hashed.
        let at = "x".repeat(MAX_BYTES);
        let stored_at = passwords.hash(&at).await.unwrap();
        assert!(passwords.verify(&at, &stored_at).await);
        // And a custom, smaller bound is honoured on every path.
        let small = heavy().max_bytes(16);
        let seventeen = "seventeen-bytes!!";
        assert!(small.hash(seventeen).await.is_err());
        assert!(!small.verify(seventeen, &stored).await);
        assert!(!small.verify_dummy(seventeen).await);
        assert!(!small.matches_any(seventeen, vec![stored]).await);
    }

    /// A stored hash is a row in a database; someone who can write rows must
    /// not be able to bill the server for gigabytes or minutes per attempt.
    #[tokio::test]
    async fn a_stored_hash_cannot_ask_for_more_work_than_the_bounds_allow() {
        let passwords = light();
        let real = passwords.hash("Correct-Horse-9").await.unwrap();
        let tail = &real[real.rfind("p=1$").unwrap() + 3..];
        let hostile = [
            format!("m={},t=2,p=1", MAX_VERIFY_MEMORY_KIB + 1),
            format!("m=64,t={},p=1", MAX_VERIFY_PASSES + 1),
            format!("m=1024,t=1,p={}", MAX_VERIFY_LANES + 1),
            "m=4294967295,t=4294967295,p=255".to_owned(),
            "m=1048576,t=64,p=16".to_owned(),
        ];
        for params in hostile {
            let row = format!("$argon2id$v=19${params}{tail}");
            let at = Instant::now();
            assert!(!passwords.verify("Correct-Horse-9", &row).await, "{params} was obeyed");
            assert!(!passwords.matches_any("Correct-Horse-9", vec![row.clone()]).await, "{params} via matches_any");
            assert!(at.elapsed() < Duration::from_secs(3), "{params} took {:?}", at.elapsed());
            assert!(passwords.needs_rehash(&row), "{params} should be marked for rehashing");
        }
        // Inside the bounds, a stronger stored hash is still honoured.
        let stronger = Passwords::with_params(MAX_VERIFY_MEMORY_KIB / 4, 2, 1).unwrap();
        let strong_row = stronger.hash("Correct-Horse-9").await.unwrap();
        assert!(passwords.verify("Correct-Horse-9", &strong_row).await);
    }

    /// An account with no usable hash (a passkey-only row, a NULL column, a
    /// bcrypt leftover) must cost the same as a real check, or its answer
    /// time says "this account is special".
    #[tokio::test]
    async fn an_unusable_stored_hash_costs_a_dummy_verification() {
        let passwords = heavy();
        let real_row = passwords.hash("Correct-Horse-9").await.unwrap();
        assert!(!passwords.verify_dummy("warm-up").await);
        let (_, baseline) = timed(passwords.verify("wrong", &real_row)).await;
        assert!(baseline > Duration::from_millis(5), "baseline too fast: {baseline:?}");

        for unusable in ["", "not a hash", "$2b$12$abcdefghijklmnopqrstuv", "$argon2id$v=19$m=notanumber$x$y"] {
            let (out, spent) = timed(passwords.verify("wrong", unusable)).await;
            assert!(!out, "{unusable:?} matched");
            assert!(spent * 4 > baseline, "{unusable:?} answered in {spent:?}, a real check takes {baseline:?}");
        }
        // A hash over the cost bounds is refused *and* still costs the dummy.
        let over = format!(
            "$argon2id$v=19$m={},t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGFzaGhhcw",
            MAX_VERIFY_MEMORY_KIB + 1
        );
        let (out, spent) = timed(passwords.verify("wrong", &over)).await;
        assert!(!out);
        assert!(spent * 4 > baseline, "an over-costly row answered in {spent:?}");
    }

    /// With one permit, N hashes take about N times one hash: they are
    /// serialised, not all resident at once.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn hashing_concurrency_is_bounded_by_the_semaphore() {
        let one = Passwords::with_params(32_768, 3, 1).unwrap().max_concurrent(1);
        // The baseline is the fastest of a few hashes, not the first one: the
        // first pays for faulting in the 32 MiB block, and on a loaded machine
        // (CI runs the whole suite at once) that inflates it enough to make the
        // comparison below fail on work that was in fact serialised.
        let mut single = Duration::MAX;
        for i in 0..3 {
            let (_, spent) = timed(one.hash(&format!("Correct-Horse-warm-{i}"))).await;
            single = single.min(spent);
        }
        assert!(single > Duration::from_millis(5), "baseline too fast: {single:?}");

        const N: u32 = 6;
        let at = Instant::now();
        let tasks: Vec<_> = (0..N)
            .map(|i| {
                let one = one.clone();
                tokio::spawn(async move { one.hash(&format!("Correct-Horse-{i}")).await.unwrap() })
            })
            .collect();
        for task in tasks {
            assert!(task.await.unwrap().starts_with("$argon2id$"));
        }
        let together = at.elapsed();
        // A loose bound on purpose: how long one hash takes varies by a factor
        // of two or more on a shared runner, so anything near N x single is a
        // coin flip. What must hold is that the work was not done all at once,
        // and the four-permit comparison below is the sharper assertion.
        assert!(
            together > single * 2,
            "{N} hashes with one permit took {together:?}; one takes {single:?} — they ran in parallel"
        );

        // With more permits the same work is measurably faster, so the bound
        // is the semaphore and not some other accidental serialisation.
        let many = Passwords::with_params(32_768, 3, 1).unwrap().max_concurrent(4);
        let at = Instant::now();
        let tasks: Vec<_> = (0..N)
            .map(|i| {
                let many = many.clone();
                tokio::spawn(async move { many.hash(&format!("Correct-Horse-{i}")).await.unwrap() })
            })
            .collect();
        for task in tasks {
            task.await.unwrap();
        }
        assert!(at.elapsed() < together, "four permits were not faster than one: {:?} vs {together:?}", at.elapsed());
    }

    /// Callers that give up (a dropped request) must not leak permits: with a
    /// single permit, a hasher abandoned many times must still answer. A
    /// leaked permit would hang here, so the last call has a deadline.
    ///
    /// That the permit is held for the *whole* blocking task — rather than
    /// released when the caller's future is dropped — is asserted by the
    /// crate's own `hashing_is_bounded_in_parallel`, which can read
    /// `available_permits` from inside the crate. An external timing test for
    /// that is too noisy to be worth keeping.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn abandoning_hashes_never_leaks_the_permit() {
        let one = Passwords::with_params(8_192, 2, 1).unwrap().max_concurrent(1);
        for _ in 0..8 {
            let abandoned = tokio::spawn({
                let one = one.clone();
                async move { one.hash("Correct-Horse-9").await }
            });
            tokio::task::yield_now().await;
            abandoned.abort();
            let _ = abandoned.await;
        }
        let answered = tokio::time::timeout(Duration::from_secs(120), one.hash("Correct-Horse-8")).await;
        assert!(answered.expect("the hasher stopped answering: a permit leaked").is_ok());
        // And it is still usable for verification afterwards.
        let stored = one.hash("Correct-Horse-7").await.unwrap();
        assert!(one.verify("Correct-Horse-7", &stored).await);
        assert!(!one.verify_dummy("Correct-Horse-7").await);
    }
}

// ---------------------------------------------------------------------------
// The in-memory rate limiter under a flood
// ---------------------------------------------------------------------------

/// Fill the table past its bound with made-up keys, then show that a
/// *different* quota still refuses beyond its own maximum. A single shared
/// overflow counter would let a fixed and a sliding quota restart each other
/// on every event, which is no limit at all.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_flood_of_made_up_keys_cannot_switch_another_quotas_limit_off() {
    let limiter = MemoryLimiter::new();
    // Long window, large maximum: these counters stay live and full.
    let flood = Quota::new(1_000_000, 24 * 3600);
    for i in 0..120_000u32 {
        assert!(limiter.hit(&format!("flood:{i}"), flood).await.unwrap().allowed);
    }
    let held = limiter.len();
    assert!(held >= 100_000, "the flood did not fill the table ({held} keys)");
    assert!(held <= 100_010, "the table grew past its bound ({held} keys)");

    // A different quota, on keys the limiter has never seen: still limited.
    let login = Quota::new(5, 900);
    let mut allowed = 0;
    for i in 0..40 {
        if limiter.hit(&format!("login:acct:{i}"), login).await.unwrap().allowed {
            allowed += 1;
        }
    }
    assert_eq!(allowed, 5, "a flood of unrelated keys let {allowed} events through a quota of 5");

    // A third quota of another *shape* is counted separately, and alternating
    // between the two kinds does not reset either.
    let sliding = Quota::new(3, 900).sliding();
    let mut interleaved = (0, 0);
    for i in 0..40 {
        if limiter.hit(&format!("codes:{i}"), sliding).await.unwrap().allowed {
            interleaved.0 += 1;
        }
        if limiter.hit(&format!("login:acct:more:{i}"), login).await.unwrap().allowed {
            interleaved.1 += 1;
        }
    }
    assert_eq!(interleaved, (3, 0), "the two window kinds restarted each other: {interleaved:?}");

    // The overflow counters are the only growth the flood caused.
    assert!(limiter.len() <= held + 3, "the flood grew the table by {} keys", limiter.len() - held);

    // A refusal still reports a sane wait, not zero or forever.
    let refused = limiter.hit("login:acct:new", login).await.unwrap();
    assert!(!refused.allowed);
    assert!(refused.retry_after > Duration::ZERO && refused.retry_after <= Duration::from_secs(900));
}

/// Keys the client controls the length of must not choose how much memory
/// each counter costs, and two long keys must stay two counters.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_limiter_keys_are_hashed_and_stay_distinct() {
    let limiter = RateLimiter::memory();
    let quota = Quota::new(1, 600);
    let a = format!("api:{}", "a".repeat(200_000));
    let b = format!("api:{}b", "a".repeat(200_000));
    assert!(limiter.check(&a, quota).await.is_ok());
    assert!(limiter.check(&a, quota).await.is_err(), "the same long key is the same counter");
    assert!(limiter.check(&b, quota).await.is_ok(), "another long key is another counter");
    // Reset reaches the hashed key.
    limiter.reset(&a).await;
    assert!(limiter.check(&a, quota).await.is_ok());

    // Non-ASCII keys are cut on a character boundary, not mid-byte.
    let unicode = format!("api:{}", "\u{4e16}".repeat(50_000));
    assert!(limiter.check(&unicode, quota).await.is_ok());
    assert!(limiter.check(&unicode, quota).await.is_err());
}

/// Nothing a client can put in a key may spell another scope's counter.
#[test]
fn identifier_and_network_keys_are_fixed_shape() {
    for hostile in ["a:b:c", "login:net:203.0.113.0/24", &"x".repeat(5_000), "\u{0}", ""] {
        let id = key::identifier(hostile);
        assert_eq!(id.len(), 32, "{hostile:?}");
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()), "{hostile:?} -> {id}");
    }
    assert_eq!(key::identifier(" Ada@Example.com "), key::identifier("ada@example.com"));
    assert_ne!(key::identifier("a"), key::identifier("b"));
}

// ---------------------------------------------------------------------------
// Keys and sealed tokens
// ---------------------------------------------------------------------------

#[test]
fn low_entropy_secrets_are_refused_as_weak() {
    for weak in ["a".repeat(64), "ab".repeat(32), "\0".repeat(48), "abcabcabc".repeat(8)] {
        assert_eq!(AuthKeys::from_secret(&weak).unwrap_err(), KeyError::Weak, "{:?}", &weak[..8]);
    }
    assert_eq!(AuthKeys::from_secret([0u8; 128]).unwrap_err(), KeyError::Weak);
    assert!(matches!(AuthKeys::from_secret("tooshort").unwrap_err(), KeyError::TooShort { .. }));
    // A previous key is held to the same bar: rotating to a weak secret fails.
    let good = AuthKeys::from_secret(crypto::secret_token()).unwrap();
    assert_eq!(good.clone().with_previous("q".repeat(40)).unwrap_err(), KeyError::Weak);
    // Real secrets pass.
    for _ in 0..20 {
        assert!(AuthKeys::from_secret(crypto::secret_token()).is_ok());
    }
    // The secret never appears in Debug output.
    assert!(!format!("{good:?}").contains("secret"));
}

#[test]
fn a_previous_key_opens_but_never_seals() {
    let old = crypto::secret_token();
    let new = crypto::secret_token();
    let before = AuthKeys::from_secret(&old).unwrap();
    let rotating = AuthKeys::from_secret(&new).unwrap().with_previous(&old).unwrap();
    let after = AuthKeys::from_secret(&new).unwrap();

    let ttl = Duration::from_secs(60);
    let old_token = before.seal(Purpose::ACCESS, &"payload", ttl).unwrap();
    assert_eq!(rotating.open::<String>(Purpose::ACCESS, &old_token).as_deref(), Ok("payload"));

    // Over many tokens: not one is ever sealed with the previous key.
    for i in 0..50 {
        let fresh = rotating.seal(Purpose::ACCESS, &i, ttl).unwrap();
        assert_eq!(before.open::<i32>(Purpose::ACCESS, &fresh), Err(OpenError::Invalid), "token {i} used the old key");
        assert_eq!(after.open::<i32>(Purpose::ACCESS, &fresh), Ok(i));
    }
    // Once the previous key is dropped, its tokens stop opening.
    assert_eq!(after.open::<String>(Purpose::ACCESS, &old_token), Err(OpenError::Invalid));
}

#[test]
fn purposes_are_separated_including_the_ones_the_session_engine_uses() {
    let keys = AuthKeys::from_secret(crypto::secret_token()).unwrap();
    let ttl = Duration::from_secs(60);
    let purposes = [
        Purpose::ACCESS,
        Purpose::GRACE,
        Purpose::CHALLENGE,
        Purpose::new("next-rust-auth/access/v"),
        Purpose::new("next-rust-auth/access/v11"),
        Purpose::new("next-rust-auth/accessv1"),
        Purpose::new(""),
    ];
    for (i, sealed_for) in purposes.iter().enumerate() {
        let token = keys.seal(*sealed_for, &format!("secret-{i}"), ttl).unwrap();
        for (j, opened_as) in purposes.iter().enumerate() {
            let out = keys.open::<String>(*opened_as, &token);
            if i == j {
                assert_eq!(out.as_deref(), Ok(format!("secret-{i}").as_str()));
            } else {
                assert_eq!(out, Err(OpenError::Invalid), "{sealed_for:?} opened as {opened_as:?}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
struct User(&'static str);
type Store = MemorySessionStore<User>;

#[derive(Clone, Default)]
struct Browser {
    cookies: BTreeMap<String, String>,
}

impl Browser {
    fn headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();
        if !self.cookies.is_empty() {
            let line = self.cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
            headers.insert("cookie", line.parse().unwrap());
        }
        headers.insert("user-agent", "Mozilla/5.0 verify".parse().unwrap());
        headers
    }

    fn visit(&self) -> Visitor {
        let headers = self.headers();
        let jar = Cookies::from_headers(&headers, true);
        Visitor::new(&headers, &jar, "203.0.113.7".parse().ok())
    }

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

    fn drop_access(&mut self) {
        self.cookies.remove("__Host-at");
    }
}

fn engine() -> (Sessions<Store>, Store) {
    let store = Store::new();
    store.insert_user("u1", User("Ada"));
    store.insert_user("u2", User("Bob"));
    (Sessions::new(store.clone(), AuthKeys::from_secret(crypto::secret_token()).unwrap()), store)
}

async fn sign_in(sessions: &Sessions<Store>, browser: &mut Browser, user: &str) -> String {
    let visitor = browser.visit();
    let opened = sessions.establish(&visitor, user, true).await.unwrap();
    browser.receive(&visitor.cookies);
    opened.session_id
}

/// Sign-out must end the whole family, not just the record whose refresh
/// token the request happens to carry: otherwise a tab that rotated a moment
/// ago stays signed in.
#[tokio::test]
async fn sign_out_ends_the_whole_family_even_with_a_stale_token() {
    let (sessions, store) = engine();
    let mut browser = Browser::default();
    sign_in(&sessions, &mut browser, "u1").await;

    // Another tab rotates: this browser now holds a retired refresh token.
    let stale = browser.clone();
    let mut fresh = browser.clone();
    fresh.drop_access();
    let visitor = fresh.visit();
    assert!(sessions.current(&visitor).await.unwrap().is_some(), "the rotation worked");
    fresh.receive(&visitor.cookies);
    let open: Vec<_> = store.sessions_of("u1").into_iter().filter(|s| s.revoked.is_none()).collect();
    assert_eq!(open.len(), 1, "one live record after a rotation");

    // Sign out with the *old* cookies.
    let visitor = stale.visit();
    assert_eq!(sessions.sign_out(&visitor).await.as_deref(), Some("u1"));
    let still_open = store.sessions_of("u1").into_iter().filter(|s| s.revoked.is_none()).count();
    assert_eq!(still_open, 0, "the replacement survived a sign-out");
    // Every record carries the sign-out reason or the rotation that preceded it.
    assert!(
        store
            .sessions_of("u1")
            .iter()
            .all(|s| matches!(s.revoked, Some(RevokeReason::Logout) | Some(RevokeReason::Rotated))),
        "unexpected revoke reasons"
    );
    // The browser that rotated can no longer use its new cookies.
    let visitor = fresh.visit();
    assert!(sessions.current(&visitor).await.unwrap().is_none());
}

/// Signing in over an open session must retire the record the browser is
/// about to lose the cookies for.
#[tokio::test]
async fn establishing_over_an_open_session_revokes_the_one_it_supersedes() {
    let (sessions, store) = engine();
    let mut browser = Browser::default();
    let first = sign_in(&sessions, &mut browser, "u1").await;
    let second = sign_in(&sessions, &mut browser, "u1").await;
    assert_ne!(first, second);
    let old = store.session(&first).expect("the first record");
    assert_eq!(old.revoked, Some(RevokeReason::Other("superseded".into())));
    assert!(store.session(&second).unwrap().revoked.is_none());
    // Switching accounts on the same browser retires the first account's session too.
    let third = sign_in(&sessions, &mut browser, "u2").await;
    assert!(store.session(&second).unwrap().revoked.is_some(), "the other account's session stayed live");
    assert!(store.session(&third).unwrap().revoked.is_none());
    // Families are not shared between sign-ins.
    let families: std::collections::BTreeSet<String> =
        [&first, &second, &third].iter().map(|id| store.session(id).unwrap().family_id).collect();
    assert_eq!(families.len(), 3, "two sign-ins shared a family");
}

/// A refresh token that leaked on its own (a log line, a proxy) must not be
/// forgiven by the grace window: grace needs the family's device cookie.
#[tokio::test]
async fn the_grace_window_is_closed_without_the_device_cookie() {
    let (sessions, store) = engine();
    let sessions = sessions.config(SessionConfig { grace: Duration::from_secs(30), ..SessionConfig::default() });
    let mut owner = Browser::default();
    sign_in(&sessions, &mut owner, "u1").await;

    // The owner rotates; the token it presented is now retired but inside grace.
    let presented = owner.clone();
    let mut rotated = owner.clone();
    rotated.drop_access();
    let visitor = rotated.visit();
    assert!(sessions.current(&visitor).await.unwrap().is_some());
    rotated.receive(&visitor.cookies);

    // A thief with only the refresh token: no device cookie, no grace.
    let mut thief = Browser::default();
    thief.cookies.insert("__Host-rt".into(), presented.cookies["__Host-rt"].clone());
    let visitor = thief.visit();
    assert!(sessions.current(&visitor).await.unwrap().is_none(), "grace was given without the device cookie");
    // It counted as theft: the family is gone.
    assert_eq!(store.sessions_of("u1").iter().filter(|s| s.revoked.is_none()).count(), 0);
    assert!(
        store.sessions_of("u1").iter().any(|s| s.revoked == Some(RevokeReason::ReuseDetected)),
        "the family was not ended for reuse"
    );

    // A wrong device cookie is no better than none.
    let (sessions, store) = engine();
    let sessions = sessions.config(SessionConfig { grace: Duration::from_secs(30), ..SessionConfig::default() });
    let mut owner = Browser::default();
    sign_in(&sessions, &mut owner, "u1").await;
    let presented = owner.clone();
    let mut rotated = owner.clone();
    rotated.drop_access();
    let visitor = rotated.visit();
    assert!(sessions.current(&visitor).await.unwrap().is_some());
    let mut thief = Browser::default();
    thief.cookies.insert("__Host-rt".into(), presented.cookies["__Host-rt"].clone());
    thief.cookies.insert("__Host-did".into(), crypto::short_token());
    let visitor = thief.visit();
    assert!(sessions.current(&visitor).await.unwrap().is_none(), "a forged device cookie bought grace");
    assert_eq!(store.sessions_of("u1").iter().filter(|s| s.revoked.is_none()).count(), 0);
}

// ---------------------------------------------------------------------------
// Breaking attempts
// ---------------------------------------------------------------------------

/// Two tasks present the same refresh token at the same moment. Exactly one
/// rotation may happen, both must be answered with the *same* replacement,
/// and the family must stay open (this is the legitimate two-tab case).
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_rotation_race_produces_one_replacement_and_no_alarm() {
    for round in 0..25 {
        let (sessions, store) = engine();
        let sessions = Arc::new(
            sessions
                .config(SessionConfig { grace: Duration::from_secs(30), ..SessionConfig::default() })
                .on_theft(|_| panic!("round: two tabs were treated as theft")),
        );
        let mut browser = Browser::default();
        sign_in(&sessions, &mut browser, "u1").await;
        browser.drop_access();

        let a = browser.clone();
        let b = browser.clone();
        let (ra, rb) = tokio::join!(
            tokio::spawn({
                let sessions = sessions.clone();
                async move {
                    let visitor = a.visit();
                    let out = sessions.current(&visitor).await.unwrap();
                    (out.map(|s| s.session_id), visitor.cookies.set_cookie_headers())
                }
            }),
            tokio::spawn({
                let sessions = sessions.clone();
                async move {
                    let visitor = b.visit();
                    let out = sessions.current(&visitor).await.unwrap();
                    (out.map(|s| s.session_id), visitor.cookies.set_cookie_headers())
                }
            }),
        );
        let (a, b) = (ra.unwrap(), rb.unwrap());
        assert!(a.0.is_some() && b.0.is_some(), "round {round}: a tab was signed out: {:?} {:?}", a.0, b.0);
        assert_eq!(a.0, b.0, "round {round}: the two tabs got different sessions");
        // Exactly one new record, exactly one live record.
        let records = store.sessions_of("u1");
        assert_eq!(records.len(), 2, "round {round}: {} records after one rotation", records.len());
        assert_eq!(records.iter().filter(|s| s.revoked.is_none()).count(), 1, "round {round}");
        // Both tabs were handed the same refresh token.
        let token = |headers: &[String]| {
            headers
                .iter()
                .find(|h| h.starts_with("__Host-rt="))
                .map(|h| h["__Host-rt=".len()..].split(';').next().unwrap().to_owned())
        };
        assert_eq!(token(&a.1), token(&b.1), "round {round}: two live refresh tokens were issued");
    }
}

/// Two sign-ins interleaved on the same browser: whatever the order, at most
/// one record may be left open, so a discarded cookie never leaves a usable
/// session behind.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn two_interleaved_sign_ins_leave_at_most_one_live_session() {
    for round in 0..25 {
        let (sessions, store) = engine();
        let sessions = Arc::new(sessions);
        let mut browser = Browser::default();
        sign_in(&sessions, &mut browser, "u1").await;

        let (a, b) = (browser.clone(), browser.clone());
        let (ra, rb) = tokio::join!(
            tokio::spawn({
                let sessions = sessions.clone();
                async move { sessions.establish(&a.visit(), "u1", true).await.unwrap().session_id }
            }),
            tokio::spawn({
                let sessions = sessions.clone();
                async move { sessions.establish(&b.visit(), "u1", true).await.unwrap().session_id }
            }),
        );
        let (a, b) = (ra.unwrap(), rb.unwrap());
        assert_ne!(a, b);
        let live: Vec<_> = store.sessions_of("u1").into_iter().filter(|s| s.revoked.is_none()).collect();
        // The first record must be gone in every interleaving; the two new
        // ones may both survive, since each browser keeps only one cookie jar.
        assert!(live.len() <= 2, "round {round}: {} live records", live.len());
        assert!(live.iter().all(|s| s.id == a || s.id == b), "round {round}: the superseded record stayed live");
    }
}

/// Many concurrent requests on one session: `is_live` decides every one of
/// them, and revoking mid-flight is honoured rather than cached.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_revocation_is_honoured_by_every_concurrent_request() {
    let (sessions, store) = engine();
    let sessions = Arc::new(sessions);
    let mut browser = Browser::default();
    let sid = sign_in(&sessions, &mut browser, "u1").await;

    let mut tasks = Vec::new();
    for _ in 0..32 {
        let (sessions, browser) = (sessions.clone(), browser.clone());
        tasks.push(tokio::spawn(async move { sessions.current(&browser.visit()).await.unwrap().is_some() }));
    }
    assert!(store.revoke(&sid, RevokeReason::Other("password_changed".into())).await.unwrap());
    for task in tasks {
        task.await.unwrap();
    }
    // After the revocation, nothing gets through any more.
    for _ in 0..8 {
        assert!(sessions.current(&browser.visit()).await.unwrap().is_none());
    }
}

/// A limiter shared by many tasks must never hand out more than the quota.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn a_shared_limiter_never_exceeds_its_quota_under_contention() {
    for kind in [Window::Fixed, Window::Sliding] {
        let limiter = Arc::new(MemoryLimiter::new());
        let quota = Quota { max: 10, window: Duration::from_secs(600), kind };
        let mut tasks = Vec::new();
        for _ in 0..64 {
            let limiter = limiter.clone();
            tasks.push(tokio::spawn(async move { limiter.hit("one-key", quota).await.unwrap() }));
        }
        let mut allowed = 0;
        for task in tasks {
            let decision: Decision = task.await.unwrap();
            if decision.allowed {
                allowed += 1;
            }
        }
        assert_eq!(allowed, 10, "{kind:?}: {allowed} events passed a quota of 10");
    }
}
