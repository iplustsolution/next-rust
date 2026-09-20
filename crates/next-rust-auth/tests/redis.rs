//! The Redis backends against a real server. Runs only with the `redis`
//! feature and `REDIS_URL` set:
//!
//! ```sh
//! REDIS_URL=redis://localhost:6379 cargo test -p next-rust-auth --features redis --test redis
//! ```
//!
//! Every run uses its own key prefix and deletes what it wrote.
#![cfg(feature = "redis")]

use std::time::Duration;

use next_rust_auth::limits::{Limiter, Quota, RateLimiter};
use next_rust_auth::redis::aio::ConnectionManager;
use next_rust_auth::{AuthKeys, MemorySessionStore, RedisGrace, RedisLimiter, Sessions, Visitor, crypto};
use next_rust_server::Cookies;

/// A connection and a prefix of this test's own; `None` without `REDIS_URL`.
async fn redis(test: &str) -> Option<(ConnectionManager, String)> {
    let Ok(url) = std::env::var("REDIS_URL") else {
        eprintln!("REDIS_URL is not set; skipping {test}");
        return None;
    };
    let client = next_rust_auth::redis::Client::open(url).expect("REDIS_URL is a redis:// url");
    let conn = ConnectionManager::new(client).await.expect("redis is reachable at REDIS_URL");
    Some((conn, format!("nra-test:{test}:{}:", crypto::short_token())))
}

async fn clean_up(conn: &mut ConnectionManager, prefix: &str) {
    let keys: Vec<String> =
        next_rust_auth::redis::cmd("KEYS").arg(format!("{prefix}*")).query_async(conn).await.unwrap();
    if !keys.is_empty() {
        let _: () = next_rust_auth::redis::cmd("DEL").arg(&keys).query_async(conn).await.unwrap();
    }
    let left: Vec<String> =
        next_rust_auth::redis::cmd("KEYS").arg(format!("{prefix}*")).query_async(conn).await.unwrap();
    assert!(left.is_empty(), "left behind: {left:?}");
}

#[tokio::test]
async fn fixed_windows_count_expire_and_reset() {
    let Some((mut conn, prefix)) = redis("fixed").await else { return };
    let limiter = RedisLimiter::new(conn.clone(), &prefix).timeout(Duration::from_secs(2));
    let quota = Quota::per(3, Duration::from_millis(600));

    for left in [2, 1, 0] {
        let d = limiter.hit("k", quota).await.unwrap();
        assert_eq!((d.allowed, d.remaining), (true, left));
    }
    let refused = limiter.hit("k", quota).await.unwrap();
    assert!(!refused.allowed);
    assert!(refused.retry_after > Duration::ZERO && refused.retry_after <= Duration::from_millis(600));
    assert!(limiter.hit("other", quota).await.unwrap().allowed, "keys are independent");

    let ttl: i64 = next_rust_auth::redis::cmd("PTTL").arg(format!("{prefix}k")).query_async(&mut conn).await.unwrap();
    assert!(ttl > 0 && ttl <= 600, "the counter expires by itself: {ttl}");

    limiter.reset("k").await.unwrap();
    assert_eq!(limiter.hit("k", quota).await.unwrap().remaining, 2, "reset");

    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(limiter.hit("k", quota).await.unwrap().remaining, 2, "a new window");
    clean_up(&mut conn, &prefix).await;
}

#[tokio::test]
async fn sliding_windows_never_hold_more_than_max() {
    let Some((mut conn, prefix)) = redis("sliding").await else { return };
    let limiter = RedisLimiter::new(conn.clone(), &prefix).timeout(Duration::from_secs(2));
    let quota = Quota::per(2, Duration::from_millis(900)).sliding();

    assert!(limiter.hit("k", quota).await.unwrap().allowed);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(limiter.hit("k", quota).await.unwrap().remaining, 0);
    let refused = limiter.hit("k", quota).await.unwrap();
    assert!(!refused.allowed);
    assert!(refused.retry_after <= Duration::from_millis(450), "until the first event leaves: {refused:?}");

    // The first event leaves the window; the second (500 ms later) is still in it.
    tokio::time::sleep(refused.retry_after + Duration::from_millis(50)).await;
    assert!(limiter.hit("k", quota).await.unwrap().allowed);
    assert!(!limiter.hit("k", quota).await.unwrap().allowed, "a fixed window would have started over");

    limiter.reset("k").await.unwrap();
    assert_eq!(limiter.hit("k", quota).await.unwrap().remaining, 1);
    clean_up(&mut conn, &prefix).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counting_is_atomic_across_instances() {
    let Some((mut conn, prefix)) = redis("atomic").await else { return };
    for (name, quota) in [("fixed", Quota::new(20, 30)), ("sliding", Quota::new(20, 30).sliding())] {
        let mut tasks = Vec::new();
        for _ in 0..4 {
            // Four "instances", each with its own limiter, 25 events each.
            let limiter = RedisLimiter::new(conn.clone(), &prefix).timeout(Duration::from_secs(5));
            tasks.push(tokio::spawn(async move {
                let mut allowed = 0;
                for _ in 0..25 {
                    if limiter.hit(name, quota).await.unwrap().allowed {
                        allowed += 1;
                    }
                }
                allowed
            }));
        }
        let mut allowed = 0;
        for task in tasks {
            allowed += task.await.unwrap();
        }
        assert_eq!(allowed, 20, "{name}: exactly the quota, however the events interleave");
    }
    clean_up(&mut conn, &prefix).await;
}

#[tokio::test]
async fn a_redis_error_fails_open_or_closed_as_chosen() {
    let Some((mut conn, prefix)) = redis("errors").await else { return };
    // A list where the counter should be: every INCR answers WRONGTYPE.
    let _: () = next_rust_auth::redis::cmd("RPUSH")
        .arg(format!("{prefix}k"))
        .arg("not a counter")
        .query_async(&mut conn)
        .await
        .unwrap();
    let broken = RedisLimiter::new(conn.clone(), &prefix).timeout(Duration::from_secs(2));
    assert!(broken.hit("k", Quota::new(1, 60)).await.is_err());
    let open = RateLimiter::new(broken.clone());
    for _ in 0..3 {
        assert!(open.check("k", Quota::new(1, 60)).await.is_ok(), "an outage locks nobody out");
    }
    let closed = RateLimiter::new(broken).fail_closed();
    assert!(closed.check("k", Quota::new(1, 60)).await.is_err());
    clean_up(&mut conn, &prefix).await;
}

#[tokio::test]
async fn a_rotation_on_one_instance_is_forgiven_on_another() {
    let Some((mut conn, prefix)) = redis("grace").await else { return };
    let store = MemorySessionStore::<String>::new();
    store.insert_user("u1", "Ada".to_owned());
    let keys = AuthKeys::from_secret(*b"redis-test secret: 0123456789abcdef").unwrap();
    let instance = || Sessions::new(store.clone(), keys.clone()).shared_grace(RedisGrace::new(conn.clone(), &prefix));
    let (a, b, alone) = (instance(), instance(), Sessions::new(store.clone(), keys.clone()));

    let visit = |cookie: Option<&str>| {
        let mut headers = http::HeaderMap::new();
        if let Some(cookie) = cookie {
            headers.insert("cookie", cookie.parse().unwrap());
        }
        Visitor::new(&headers, &Cookies::from_headers(&headers, true), None)
    };
    let refresh_cookie = |visitor: &Visitor| {
        let line = visitor.cookies.set_cookie_headers().into_iter().find(|h| h.starts_with("__Host-rt=")).unwrap();
        line.split(';').next().unwrap().to_owned()
    };
    // What the browser sends back: the refresh token and the device cookie next
    // to it. Grace is only given to the device the family belongs to.
    let browser_cookies = |visitor: &Visitor| {
        let device = visitor.cookies.set_cookie_headers().into_iter().find(|h| h.starts_with("__Host-did=")).unwrap();
        format!("{}; {}", refresh_cookie(visitor), device.split(';').next().unwrap())
    };

    // Sign in on A; the access token "expires"; A rotates, but the response is lost.
    let signed_in = visit(None);
    a.establish(&signed_in, "u1", false).await.unwrap();
    let old = browser_cookies(&signed_in);
    let rotated = a.current(&visit(Some(&old))).await.unwrap().expect("rotated on A");

    // The retry lands on B, which has no local grace entry for it.
    let retry = visit(Some(&old));
    let forgiven = b.current(&retry).await.unwrap().expect("B finds A's grace entry in Redis");
    assert_eq!(forgiven.session_id, rotated.session_id);
    assert_eq!(forgiven.user, "Ada");

    // What Redis holds is sealed: no refresh token in the clear.
    let parked: Vec<String> =
        next_rust_auth::redis::cmd("KEYS").arg(format!("{prefix}*")).query_async(&mut conn).await.unwrap();
    assert_eq!(parked.len(), 1);
    let value: String = next_rust_auth::redis::cmd("GET").arg(&parked[0]).query_async(&mut conn).await.unwrap();
    assert!(value.starts_with("v1."));
    assert!(!value.contains(refresh_cookie(&retry).trim_start_matches("__Host-rt=")));
    let ttl: i64 = next_rust_auth::redis::cmd("PTTL").arg(&parked[0]).query_async(&mut conn).await.unwrap();
    assert!(ttl > 0 && ttl <= 15_000, "{ttl}");

    // An instance without the shared store sees a rotated token coming back: theft.
    assert!(alone.current(&visit(Some(&old))).await.unwrap().is_none());
    assert!(b.current(&visit(Some(&browser_cookies(&retry)))).await.unwrap().is_none(), "the family has ended");
    clean_up(&mut conn, &prefix).await;
}
