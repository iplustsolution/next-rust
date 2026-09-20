//! The "auth & rate limits" documentation page.

use next_rust::prelude::*;

fn heading(anchor: &'static str, text: &'static str) -> Node {
    h2![id(anchor), a![class("anchor"), href(format!("#{anchor}")), text]].into_node()
}

fn sub(anchor: &'static str, text: &'static str) -> Node {
    h3![id(anchor), a![class("anchor"), href(format!("#{anchor}")), text]].into_node()
}

fn rust(source: &'static str) -> Node {
    pre![code![class("language-rust"), source]].into_node()
}

pub fn content() -> Node {
    fragment![
        p![
            "The framework's ",
            a![href("/docs/middleware"), "middleware page"],
            " covers the primitives: cookies, ",
            code!["Auth<T>"],
            ", ",
            code!["protected"],
            ", ",
            code!["rate_limit"],
            ". This page is about ",
            code!["next_rust::auth"],
            ", an optional kit of the parts a real sign-in system is made of, distilled from a production implementation. \
             It knows nothing about your database or what a user is: you bring the accounts table, a few SQL statements \
             and the screens.",
        ],
        div![
            class("callout"),
            p![
                strong!["In plain words. "],
                "Signing in is a front desk. Someone shows a password (proof of who they are), gets a wristband (a session cookie) so they are not asked again on every page, and the desk keeps a guest list it can strike a name from at any moment (sign-out, a stolen wristband). A guard counts how often anyone tries the door (rate limits). This kit is the desk, the wristbands and the guard; the guest book itself, your users table, stays yours.",
            ],
        ],
        p!["The words this page uses:"],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["term"], th!["meaning"]]],
                tbody![
                    tr![
                        td![strong!["Cookie"]],
                        td!["a small named value the browser stores for your site and sends back with every request"],
                    ],
                    tr![
                        td![strong!["Session"]],
                        td![
                            "the server's record that this browser is signed in as this user; the browser only holds tokens that point at it"
                        ],
                    ],
                    tr![
                        td![strong!["Token"]],
                        td![
                            "a string that proves something (\"this browser holds session 42\"). Sealed means encrypted and tamper-proof: the browser can carry it but not read or change it"
                        ],
                    ],
                    tr![
                        td![strong!["Hash"]],
                        td![
                            "a one-way scramble. Passwords are stored only as hashes, so a stolen database does not reveal them; argon2id is a hash designed to be slow and memory-hungry, which makes guessing expensive"
                        ],
                    ],
                    tr![
                        td![strong!["Rate limit"]],
                        td![
                            "a cap on how often something may happen in a time window, such as 10 sign-in attempts per account in 15 minutes; past it the answer is 429 Too Many Requests"
                        ],
                    ],
                    tr![
                        td![strong!["Secret"]],
                        td![
                            "a long random value only the server knows, here the environment variable ",
                            code!["AUTH_SECRET"],
                            ". All sealed tokens depend on it. It lives in the server's environment (",
                            code![".env.local"],
                            " in development), never in Git, ",
                            code!["next-rust.toml"],
                            " or the page",
                        ],
                    ],
                ],
            ],
        ],
        p![
            "The kit is optional and switched on as a Cargo feature. This line goes in the project's ",
            code!["Cargo.toml"],
            "; without it ",
            code!["next_rust::auth"],
            " does not exist and nothing of it is compiled into the binary:",
        ],
        pre![code![
            class("language-toml"),
            r#"[dependencies]
next-rust = { version = "0.1", features = ["auth"] }          # or "auth-redis" for the Redis backends"#,
        ]],
        div![
            class("table-wrap"),
            table![
                thead![tr![th!["piece"], th!["what it does"]]],
                tbody![
                    tr![
                        td![code!["AuthKeys"]],
                        td!["typed sealed tokens: AES-256-GCM, one key per purpose, expiry, secret rotation"]
                    ],
                    tr![
                        td![code!["SecureCookies"]],
                        td![
                            code!["__Host-"],
                            " prefixed, ",
                            code!["HttpOnly"],
                            " cookies with safe defaults; sealed cookies for pending steps"
                        ]
                    ],
                    tr![
                        td![code!["Sessions"]],
                        td![
                            "access token + rotating refresh token, reuse detection, a grace window, per-request verification, middleware and the ",
                            code!["CurrentSession<U>"],
                            " extractor"
                        ]
                    ],
                    tr![
                        td![code!["Passwords"], ", ", code!["PasswordPolicy"]],
                        td![
                            "argon2id off the async threads, dummy verification, rehash detection, a configurable policy"
                        ]
                    ],
                    tr![
                        td![code!["RateLimiter"]],
                        td![
                            "fixed and sliding windows, in memory or in Redis, network and identifier keys, fail-open or fail-closed, a middleware adapter"
                        ]
                    ],
                    tr![td![code!["Codes"]], td!["one-time codes bound to user and purpose, attempt counting, decoys"]],
                ],
            ],
        ],
        heading("sealed-tokens", "Sealed tokens"),
        p![
            "Use a sealed token whenever you must hand the browser a piece of information it should carry but not read or alter: an invitation link, a password-reset link, a \"step 2 of sign-in\" marker. It is a sealed envelope only your server can open.",
        ],
        p![
            "A sealed token is JSON encrypted with AES-256-GCM. The key is derived (HKDF-SHA256) from your secret and the token's ",
            em!["purpose"],
            ", and the purpose is also the associated data. A token sealed for one use cannot be opened as another, the browser \
             can neither read nor forge it, and every token carries its own expiry, checked on open.",
        ],
        p![
            "This example creates the keys from the ",
            code!["AUTH_SECRET"],
            " environment variable once (put it where your app builds its shared state, such as ",
            code!["src/lib.rs"],
            "), then seals and opens an invitation:",
        ],
        rust(
            r#"use next_rust::auth::{AuthKeys, Purpose};

const INVITE: Purpose = Purpose::new("invite/v1");      // a new kind of token gets a new purpose

let keys = AuthKeys::from_env("AUTH_SECRET", Some("AUTH_SECRET_PREVIOUS"))?;   // >= 32 bytes: openssl rand -base64 48
let token = keys.seal(INVITE, &Invite { team, role }, Duration::from_secs(7 * 86400))?;
let invite: Invite = keys.open(INVITE, &token)?;         // Err: Malformed | Invalid | Expired | Shape"#,
        ),
        p![
            strong!["Rotating the secret. "],
            "Deploy the new secret with the old one as a previous key (",
            code!["AuthKeys::from_secret(new)?.with_previous(old)?"],
            "). New tokens use the current key; old ones still open until they expire. Drop the previous key once the \
             longest token lifetime has passed. Even without it, a changed secret costs signed-in people one silent \
             refresh, not their session: refresh tokens are random, not sealed.",
        ],
        p![
            "The module also has the small things every flow needs: ",
            code!["secret_token()"],
            " (256 bits), ",
            code!["short_token()"],
            ", ",
            code!["numeric_code(6)"],
            " (no modulo bias), ",
            code!["sha256_hex"],
            " and ",
            code!["constant_time_eq"],
            ".",
        ],
        heading("cookies", "Cookies"),
        p![
            code!["SecureCookies"],
            " names cookies by a base name and sends them as ",
            code!["__Host-<name>"],
            " whenever the jar is in secure mode (production), always ",
            code!["HttpOnly"],
            ", ",
            code!["Path=/"],
            ", ",
            code!["SameSite=Lax"],
            ". A browser accepts a ",
            code!["__Host-"],
            " cookie only from the exact host over HTTPS, so a sibling subdomain can neither plant nor overwrite it; and \
             because only the prefixed name is read, a planted unprefixed cookie is ignored. On ",
            code!["http://localhost"],
            " the bare name is used, so the same code runs in development.",
        ],
        p![
            "Inside a page, action or middleware, wrap the request's cookie jar and set, read or clear cookies by base name:",
        ],
        rust(
            r#"let cookies = SecureCookies::new(&jar);                 // jar: the Cookies extractor, req.cookies(), ctx.cookies
cookies.set("flow", &id, Duration::from_secs(300));
cookies.get("flow");    cookies.clear("flow");           // clear() only deletes what the browser has

// A pending step, handed from one screen to the next; the browser cannot read or forge it.
cookies.set_sealed(&keys, Purpose::CHALLENGE, "ch", &Pending { user_id, step, next }, Duration::from_secs(900));
let pending: Option<Pending> = cookies.get_sealed(&keys, Purpose::CHALLENGE, "ch");"#,
        ),
        heading("sessions", "Sessions"),
        p![
            "A session is what keeps someone signed in between pages. The design below answers two questions a simple \"user id in a cookie\" cannot: how do you sign someone out ",
            em!["right now"],
            ", and how do you notice that a cookie was stolen?",
        ],
        p![
            "A session is a record in your store plus two cookies: a sealed ",
            strong!["access token"],
            " (15 minutes) naming the session, the user and a binding to this browser, and a random ",
            strong!["refresh token"],
            " of which the store only ever sees the SHA-256.",
        ],
        ul![
            li![
                strong!["Every request asks the store. "],
                "A valid access token still goes through ",
                code!["SessionStore::is_live"],
                ", so a sign-out, a password change or a suspension takes effect on the next click, not when the token runs out.",
            ],
            li![
                strong!["Rotation. "],
                "When the access token has expired, the refresh token is exchanged: a new record in the same family replaces \
                 the old one, and the browser gets new cookies.",
            ],
            li![
                strong!["Reuse detection. "],
                "A refresh token that was already rotated and comes back means someone holds a copy. The whole family ends \
                 (thief and owner alike: nobody can tell who came second) and ",
                code!["on_theft"],
                " fires.",
            ],
            li![
                strong!["The grace window. "],
                "For 15 seconds a rotated token answers with its replacement instead: two tabs waking up together, or a \
                 response lost on a bad connection, are not theft. The request must carry the family's device cookie \
                 too: a refresh token that leaked on its own is treated as theft even inside the window. Grace entries \
                 live in the process and, with several instances, sealed in Redis. Signing out ends the whole family, \
                 and signing in again revokes the session it replaces.",
            ],
            li![
                strong!["Browser binding. "],
                "The access token is tied to the device cookie and user agent. A copied access cookie replayed elsewhere does \
                 not verify; the refresh path, with its reuse detection, decides.",
            ],
        ],
        sub("sessions-wiring", "Wiring it up"),
        p![
            "Three pieces in three files: one ",
            code!["Sessions"],
            " engine for the whole process in ",
            code!["src/lib.rs"],
            ", a ",
            code!["middleware.rs"],
            " in the folder whose pages need a signed-in visitor (here the route group ",
            code!["app/(account)/"],
            "; every page below it is protected), and pages that take the session as an argument:",
        ],
        rust(
            r#"// src/lib.rs: one engine for the process
use next_rust::auth::{limits::Quota, AuthKeys, RateLimiter, Sessions};

pub static SESSIONS: LazyLock<Sessions<PgSessions>> = LazyLock::new(|| {
    let keys = AuthKeys::from_env("AUTH_SECRET", Some("AUTH_SECRET_PREVIOUS")).expect("AUTH_SECRET");
    Sessions::new(PgSessions { pool: pool() }, keys)
        .limit_refresh(RateLimiter::memory(), Quota::new(600, 900))
        .on_theft(|event| audit::token_reuse(event.clone()))      // spawn anything slow
});

// app/(account)/middleware.rs: everything below needs a session
pub async fn middleware(req: Request, next: Next) -> Response {
    SESSIONS.require("/login").handle(req, next).await
}

// app/(account)/settings/page.rs
pub fn Page(session: CurrentSession<User>) -> impl View {
    h1!["Hello, ", session.user.name.clone()]
}"#,
        ),
        p![
            code!["require(login)"],
            " redirects guests to ",
            code!["login?next=<the local path they wanted>"],
            " (401 for other methods), answers 429 when the refresh limit is spent and 503 when the store is down (an outage \
             signs nobody out), and marks responses ",
            code!["Cache-Control: private, no-store"],
            ". It also inserts ",
            code!["AuthUser<U>"],
            ", so the framework's ",
            code!["Auth<U>"],
            " extractor works too. ",
            code!["SESSIONS.load()"],
            " is the optional variant for pages guests may see. A page that takes ",
            code!["CurrentSession<U>"],
            " is always rendered per request.",
        ],
        p![
            "Signing in and out are server actions (functions a form calls), for example in ",
            code!["app/login/page.rs"],
            " or ",
            code!["src/actions.rs"],
            ". ",
            code!["establish"],
            " creates the session and sets the cookies; ",
            code!["revoke_all"],
            " ends every other session of a user:",
        ],
        rust(
            r#"#[server_action]
pub async fn sign_in(ctx: ActionContext, form: SignIn) -> Result<()> {
    let user = check_password(&ctx, &form).await?;            // see "Passwords" and "Rate limits"
    SESSIONS.establish(&Visitor::from_action(&ctx), &user.id.to_string(), false).await?;
    Err(redirect("/settings"))
}

#[server_action]
pub async fn sign_out(ctx: ActionContext) -> Result<()> {
    SESSIONS.sign_out(&Visitor::from_action(&ctx)).await;
    Err(redirect("/"))
}

// after a password change: everywhere else, out
SESSIONS.revoke_all(&session.user_id, "password_changed", Some(&session.session_id)).await?;"#,
        ),
        sub("session-store", "The store: sqlx and Postgres"),
        p![
            code!["SessionStore"],
            " is seven small operations. ",
            code!["find_by_refresh_hash"],
            " returns the record together with a ",
            em!["lock"],
            " (here: the transaction holding ",
            code!["SELECT ... FOR UPDATE"],
            ") which ",
            code!["rotate"],
            " commits. That lock is what makes two requests with the same token take turns. ",
            code!["MemorySessionStore"],
            " is the reference implementation for tests.",
        ],
        p![
            "The store is where session records live: your database. The kit does not talk to a database itself; you implement a small trait for yours. First the table, to run once against your Postgres database (as a migration):",
        ],
        pre![code![
            class("language-sql"),
            r#"CREATE TABLE sessions (
    id                  uuid PRIMARY KEY,
    user_id             uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id           uuid NOT NULL,
    refresh_hash        text NOT NULL UNIQUE,
    device_id           text NOT NULL,
    mfa_verified        boolean NOT NULL,
    created_at          timestamptz NOT NULL,
    expires_at          timestamptz NOT NULL,
    absolute_expires_at timestamptz NOT NULL,
    revoked_at          timestamptz,
    revoked_reason      text,
    ip                  inet,
    user_agent          text NOT NULL,
    CHECK ((revoked_at IS NULL) = (revoked_reason IS NULL))
);
CREATE INDEX ON sessions (family_id) WHERE revoked_at IS NULL;
CREATE INDEX ON sessions (user_id)   WHERE revoked_at IS NULL;"#,
        ]],
        p![
            "Then the implementation, a file of your own such as ",
            code!["src/sessions.rs"],
            ". It is long because it is complete; copy it and adjust the ",
            code!["users"],
            " columns in ",
            code!["is_live"],
            " to your table:",
        ],
        rust(
            r#"use next_rust::auth::{RevokeReason, SessionRecord, SessionStore, StoreError};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

pub struct PgSessions { pub pool: PgPool }

const COLUMNS: &str = "id::text, user_id::text, family_id::text, refresh_hash, device_id, mfa_verified, \
    extract(epoch from created_at)::bigint, extract(epoch from expires_at)::bigint, \
    extract(epoch from absolute_expires_at)::bigint, revoked_reason, host(ip), user_agent";

type Row = (String, String, String, String, String, bool, i64, i64, i64, Option<String>, Option<String>, String);

fn record(r: Row) -> SessionRecord {
    SessionRecord {
        id: r.0, user_id: r.1, family_id: r.2, refresh_hash: r.3, device_id: r.4, mfa_verified: r.5,
        created_at: r.6 as u64, expires_at: r.7 as u64, absolute_expires_at: r.8 as u64,
        revoked: r.9.as_deref().map(RevokeReason::parse),
        ip: r.10.and_then(|ip| ip.parse().ok()),
        user_agent: r.11,
    }
}

async fn insert(conn: &mut PgConnection, s: &SessionRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sessions (id, user_id, family_id, refresh_hash, device_id, mfa_verified,
                               created_at, expires_at, absolute_expires_at, ip, user_agent)
         VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6,
                 to_timestamp($7::bigint), to_timestamp($8::bigint), to_timestamp($9::bigint), $10::inet, $11)",
    )
    .bind(&s.id).bind(&s.user_id).bind(&s.family_id).bind(&s.refresh_hash).bind(&s.device_id).bind(s.mfa_verified)
    .bind(s.created_at as i64).bind(s.expires_at as i64).bind(s.absolute_expires_at as i64)
    .bind(s.ip.map(|ip| ip.to_string())).bind(&s.user_agent)
    .execute(conn).await.map(|_| ())
}

async fn revoke_where(pool: &PgPool, filter: &str, id: &str, reason: RevokeReason, keep: Option<&str>) -> Result<u64, StoreError> {
    sqlx::query(&format!(
        "UPDATE sessions SET revoked_at = now(), revoked_reason = $2
          WHERE {filter} = $1::uuid AND revoked_at IS NULL AND ($3::uuid IS NULL OR id <> $3::uuid)"
    ))
    .bind(id).bind(reason.as_str()).bind(keep)
    .execute(pool).await.map(|done| done.rows_affected()).map_err(StoreError::new)
}

impl SessionStore for PgSessions {
    type User = User;
    type Lock = Transaction<'static, Postgres>;       // dropped without commit: rolled back, lock released

    async fn create(&self, s: &SessionRecord) -> Result<(), StoreError> {
        let mut conn = self.pool.acquire().await.map_err(StoreError::new)?;
        insert(&mut conn, s).await.map_err(StoreError::new)
    }

    async fn find_by_refresh_hash(&self, hash: &str) -> Result<Option<(SessionRecord, Self::Lock)>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(StoreError::new)?;
        let row: Option<Row> = sqlx::query_as(&format!("SELECT {COLUMNS} FROM sessions WHERE refresh_hash = $1 FOR UPDATE"))
            .bind(hash).fetch_optional(&mut *tx).await.map_err(StoreError::new)?;
        Ok(row.map(|r| (record(r), tx)))              // revoked rows too: that is how reuse is noticed
    }

    async fn rotate(&self, mut tx: Self::Lock, old: &SessionRecord, new: &SessionRecord) -> Result<(), StoreError> {
        insert(&mut tx, new).await.map_err(StoreError::new)?;
        sqlx::query("UPDATE sessions SET revoked_at = now(), revoked_reason = 'rotated' WHERE id = $1::uuid")
            .bind(&old.id).execute(&mut *tx).await.map_err(StoreError::new)?;
        tx.commit().await.map_err(StoreError::new)
    }

    async fn revoke(&self, session_id: &str, reason: RevokeReason) -> Result<bool, StoreError> {
        Ok(revoke_where(&self.pool, "id", session_id, reason, None).await? > 0)
    }
    async fn revoke_family(&self, family_id: &str, reason: RevokeReason) -> Result<u64, StoreError> {
        revoke_where(&self.pool, "family_id", family_id, reason, None).await
    }
    async fn revoke_user(&self, user_id: &str, reason: RevokeReason, keep: Option<&str>) -> Result<u64, StoreError> {
        revoke_where(&self.pool, "user_id", user_id, reason, keep).await
    }

    async fn is_live(&self, session_id: &str, user_id: &str, now: u64) -> Result<Option<User>, StoreError> {
        sqlx::query_as(
            "SELECT u.id, u.email, u.name
               FROM sessions s JOIN users u ON u.id = s.user_id
              WHERE s.id = $1::uuid AND s.user_id = $2::uuid AND s.revoked_at IS NULL
                AND s.expires_at > to_timestamp($3::bigint) AND s.absolute_expires_at > to_timestamp($3::bigint)
                AND u.status = 'active'",                    // suspended or deleted: signed out on the next click
        )
        .bind(session_id).bind(user_id).bind(now as i64)
        .fetch_optional(&self.pool).await.map_err(StoreError::new)
    }
}"#,
        ),
        p![
            "Rotated and expired rows can be deleted once ",
            code!["absolute_expires_at"],
            " has passed; keep rotated rows until then, because finding one is how reuse is noticed. Too many wrong \
             passwords should block ",
            em!["new sign-ins"],
            ", never end open sessions, or anyone can sign a person out by typing wrong passwords at their address.",
        ],
        sub("sessions-instances", "Several instances"),
        p![
            "When the app runs as several copies behind a load balancer, two requests of one visitor can land on different copies. Share the grace window and the refresh limit through Redis (the ",
            code!["auth-redis"],
            " feature) so every copy gives the same answer. This replaces the engine set-up in ",
            code!["src/lib.rs"],
            ":",
        ],
        rust(
            r#"let redis = RedisLimiter::connect(&redis_url, "myapp:rl:").await?;
let sessions = Sessions::new(store, keys)
    .shared_grace(RedisGrace::new(redis.connection(), "myapp:grace:"))   // a retry may land on another instance
    .limit_refresh(RateLimiter::new(redis), Quota::new(600, 900));"#,
        ),
        heading("passwords", "Passwords"),
        p![
            "A password is never stored. What is stored is its hash, a one-way scramble: checking a password means scrambling what was typed and comparing. The snippets below belong in your sign-up and sign-in actions.",
        ],
        p![
            "Hashes are argon2id PHC strings (19 MiB, 2 passes, 1 lane by default), so the parameters travel with each hash. \
             Hashing and verifying run on tokio's blocking pool, never on a request thread.",
        ],
        rust(
            r#"use next_rust::auth::{PasswordPolicy, Passwords};

static PASSWORDS: LazyLock<Passwords> = LazyLock::new(Passwords::new);

// setting a password
PasswordPolicy::default().min_length(12).check(&form.password, &[&user.email, &user.phone])
    .map_err(|why| Error::validation([("password", why.to_string())]))?;
let hash = PASSWORDS.hash(&form.password).await?;

// checking one: an unknown account costs the same time as a wrong password
let ok = match &account {
    Some(a) => PASSWORDS.verify(&form.password, &a.password_hash).await,
    None => PASSWORDS.verify_dummy(&form.password).await,       // always false
};
if ok && PASSWORDS.needs_rehash(&a.password_hash) { /* hash again with today's parameters and save */ }"#,
        ),
        p![
            "The default policy: 12 to 256 characters, upper and lower case, a digit, not on the common list (extend it with ",
            code!["also_refuse"],
            "), no run of six neighbouring characters (",
            code!["abcdef"],
            ", ",
            code!["987654"],
            "), and none of the account's identifiers: an email's local part or a user name of four or more characters, a \
             phone's full number or its last ten digits. ",
            code!["PasswordPolicy::length_only()"],
            " drops the composition rules. The minimum never goes below 8, whatever a setting says. ",
            code!["matches_any"],
            " checks a password history in one blocking task.",
        ],
        p![
            "Argon2 is expensive on purpose, which makes sign-in a way to spend your memory. So a password over 1024 bytes is \
             refused before anything is hashed (",
            code!["Passwords::max_bytes"],
            "; raise it together with the policy's maximum), at most one hash per CPU runs at a time (",
            code!["max_concurrent"],
            "), a stored hash that asks for more than 256 MiB or 16 passes is refused, and an account whose stored hash is \
             missing or malformed costs a dummy verification, so it answers as slowly as any other. Passwords are not \
             Unicode-normalised, and the length rules count characters.",
        ],
        heading("rate-limits", "Rate limits"),
        p![
            "Without a limit, a program can try thousands of passwords a minute against your sign-in form. A rate limit caps the attempts, per network and per account. This example sits at the top of a sign-in action, before the password is checked:",
        ],
        p![
            "The built-in ",
            code!["rate_limit"],
            " middleware is a per-process token bucket over whole requests. ",
            code!["RateLimiter"],
            " is for the limits a sign-in flow needs on top: per action and per subject, optionally shared between instances.",
        ],
        rust(
            r#"use next_rust::auth::limits::{key, Quota, RateLimiter};

const LOGIN_PER_NETWORK: Quota = Quota::new(200, 15 * 60);
const LOGIN_PER_ACCOUNT: Quota = Quota::new(10, 15 * 60).sliding();
const RESEND: Quota = Quota::cooldown(45);

let limiter = RateLimiter::memory();                          // or RateLimiter::new(RedisLimiter::connect(..).await?)
limiter.check(&key::scoped("login:net", &key::network(ctx.client_ip())), LOGIN_PER_NETWORK).await?;   // Err -> 429
let account = key::scoped("login:acct", &key::identifier(&form.email));
limiter.check(&account, LOGIN_PER_ACCOUNT).await?;
// ... after a correct password:
limiter.reset(&account).await;"#,
        ),
        ul![
            li![
                code!["key::network"],
                " masks the address to its /24 (IPv4 and IPv4-mapped) or /64 (IPv6): attackers rotate through neighbours, \
                 and one IPv6 customer owns a whole /64. ",
                code!["key::identifier"],
                " trims, lowercases and hashes, so the limiter's store never holds an email address.",
            ],
            li![
                strong!["Fixed window"],
                " (default): one counter that expires ",
                code!["window"],
                " after its first event; up to twice the quota can fall across a boundary. ",
                strong!["Sliding"],
                " (",
                code![".sliding()"],
                ") remembers each allowed event, so no window-long span ever holds more than the quota; it costs ",
                code!["max"],
                " timestamps per key, so use it for small quotas. Refused events never extend the wait.",
            ],
            li![
                "In Redis each event is one Lua script, atomic however many instances count at once, with a 250 ms deadline \
                 per command. The sliding window reads Redis's clock, so instances need not agree on the time (Redis 5+).",
            ],
            li![
                strong!["Failure policy. "],
                "When the backend cannot answer, the limiter ",
                em!["fails open"],
                " and logs it: a cache outage must not lock everyone out. Use ",
                code![".fail_closed()"],
                " for limits that are the only guard of something, and keep a second guard that does not depend on the cache \
                 (an account lockout in the database) where it matters.",
            ],
        ],
        p![
            "As middleware, counted in the limiter's backend instead of per process, with the same 429 and ",
            code!["Retry-After"],
            ":"
        ],
        rust(
            r#"// app/api/middleware.rs
static LIMIT: LazyLock<RequestLimit> = LazyLock::new(|| limiter().requests(Quota::per_minute(120)).scope("api"));
//   .key(|req| req.header("x-api-key").unwrap_or("anonymous").to_owned())   // default: the client's network

pub async fn middleware(req: Request, next: Next) -> Response {
    LIMIT.handle(req, next).await
}"#,
        ),
        heading("codes", "One-time codes"),
        p![
            "The six digits sent by email or SMS to confirm an address or a sign-in. Issue one, deliver it yourself, verify what the visitor types:",
        ],
        p![
            "A code is stored as ",
            code![r#"sha256("{user_id}.{purpose}.{code}")"#],
            ", never as digits: a code for one account or one step is worthless for another, and a unique index on the hash \
             does not collide when two people hold the same six digits. A new code retires the earlier one; each wrong guess \
             costs an attempt and the fifth retires the code.",
        ],
        rust(
            r#"use next_rust::auth::{CodeCheck, Codes, MemoryCodeStore};

let codes = Codes::new(store);                                 // your CodeStore; MemoryCodeStore for tests
let code = codes.issue(&user_id, "email_verification").await?; // send it, then forget it
match codes.verify(&user_id, "email_verification", &form.code).await? {
    CodeCheck::Valid => { /* the step is done */ }
    CodeCheck::Wrong { remaining } => { /* "{remaining} attempts remaining" */ }
    CodeCheck::Locked | CodeCheck::Expired => { /* "request a new code" */ }
}"#,
        ),
        p![
            code!["CodeStore"],
            " has two operations: ",
            code!["put"],
            " replaces the code of ",
            code!["(user_id, purpose)"],
            "; ",
            code!["check"],
            " loads the row ",
            code!["FOR UPDATE"],
            ", calls ",
            code!["StoredCode::evaluate"],
            " (all the rules live there) and writes it back, ideally in the transaction that also saves the code's effect. \
             Put per-account quotas on checks and on sends as well, or requesting new codes buys unlimited guesses.",
        ],
        p![
            strong!["Decoys. "],
            "So that sign-up and recovery never reveal whether an account exists, store a pending step of the same shape for an \
             unknown address (a random user id, ",
            code!["decoy: true"],
            ") with ",
            code!["set_sealed"],
            ", send nothing, and answer guesses with ",
            code!["codes.decoy_miss(&limiter, ..)"],
            ": the same countdown a real code gives.",
        ],
        heading("common-mistakes", "Common mistakes"),
        ul![
            li![
                strong!["No AUTH_SECRET in production, or a short one. "],
                code!["AuthKeys::from_env"],
                " wants at least 32 bytes (",
                code!["openssl rand -base64 48"],
                "). Set it in the server's environment, the same on every instance, and keep it out of Git.",
            ],
            li![
                strong!["Changing the secret without with_previous. "],
                "Every outstanding invitation and reset link stops opening at once. Deploy the old secret as the previous key first.",
            ],
            li![
                strong!["Forgetting the feature flag. "],
                code!["next_rust::auth"],
                " not found means ",
                code!["features = [\"auth\"]"],
                " is missing in ",
                code!["Cargo.toml"],
                ".",
            ],
            li![
                strong!["Protecting pages but not actions. "],
                "A ",
                code!["middleware.rs"],
                " guards the pages in its folder. A server action or API route must check the session itself before it changes anything.",
            ],
            li![
                strong!["Deleting rotated session rows early. "],
                "Finding a rotated row is how a stolen token is noticed. Keep them until ",
                code!["absolute_expires_at"],
                ".",
            ],
            li![
                strong!["Locking accounts by ending sessions. "],
                "Wrong passwords should block new sign-ins, never sign the real owner out.",
            ],
            li![
                strong!["Different answers for known and unknown accounts. "],
                "\"No such user\" tells an attacker which addresses are registered. Use ",
                code!["verify_dummy"],
                " and decoys so both cases look and take the same.",
            ],
            li![
                strong!["Expecting secure cookies over plain HTTP in production. "],
                code!["__Host-"],
                " cookies are only accepted over HTTPS. Serve production behind TLS; ",
                code!["http://localhost"],
                " works in development because the bare name is used there.",
            ],
        ],
        sub("check-it-worked", "How to check it worked"),
        ul![
            li!["Open a protected page in a private window: you should land on ", code!["/login?next=…"], ".",],
            li![
                "Sign in and look at the cookies in the browser's developer tools: ",
                code!["HttpOnly"],
                ", and in production the ",
                code!["__Host-"],
                " prefix with ",
                code!["Secure"],
                ".",
            ],
            li!["Sign out in one tab; the next click in another tab should ask you to sign in again."],
            li![
                "Type a wrong password more often than the quota allows: the answer becomes 429 with a ",
                code!["Retry-After"],
                " header when the limiter runs as middleware.",
            ],
            li![
                "In tests, use ",
                code!["MemorySessionStore"],
                " and ",
                code!["MemoryCodeStore"],
                " with ",
                a![href("/docs/testing"), code!["TestClient"]],
                ", which keeps cookies between requests.",
            ],
        ],
        heading("not-included", "What the kit leaves to you"),
        ul![
            li![
                "The accounts table, the screens, delivery of codes and security mail, audit logging (hook into ",
                code!["on_theft"],
                ")."
            ],
            li![
                "Non-enumerable answers end to end: the pieces are here (dummy verification, decoys, identical countdowns); using them in every branch of your flows is yours."
            ],
            li![
                "Passkeys, social sign-in and TOTP. A second factor plugs in before ",
                code!["establish(.., mfa_verified: true)"],
                "."
            ],
            li![
                "A cache in front of ",
                code!["is_live"],
                ": wrap your store if one indexed query per request is too much, and remember to drop the cached entry on revocation."
            ],
            li![
                "CSRF is the framework's job (",
                code!["[security] csrf"],
                "); these cookies are ",
                code!["SameSite=Lax"],
                " on top of it."
            ],
        ],
    ]
}
