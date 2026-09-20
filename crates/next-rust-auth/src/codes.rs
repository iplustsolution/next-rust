//! One-time codes: six digits, ten minutes, five tries (all configurable).
//!
//! A code is stored as `sha256("{user_id}.{purpose}.{code}")`, never as the
//! digits. Binding the user and the purpose into the hash means a code for
//! one account or one step is worthless for another, and a unique index on
//! the hash column does not collide when two people hold the same six digits.
//!
//! Issuing a code retires the account's earlier code for that purpose. Each
//! wrong guess costs one attempt; the last one retires the code, so six
//! digits are never worth more than `max_attempts` guesses. Put a
//! per-account [`Quota`] on checks and on sends as
//! well, or requesting new codes buys unlimited guesses.
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! use next_rust_auth::codes::{CodeCheck, Codes, MemoryCodeStore};
//!
//! let codes = Codes::new(MemoryCodeStore::default());
//! let code = codes.issue("user-1", "email_verification").await.unwrap();   // send it
//! assert_eq!(codes.verify("user-1", "email_verification", "000000x").await.unwrap(), CodeCheck::Wrong { remaining: 4 });
//! assert_eq!(codes.verify("user-1", "email_verification", &code).await.unwrap(), CodeCheck::Valid);
//! assert_eq!(codes.verify("user-1", "email_verification", &code).await.unwrap(), CodeCheck::Expired, "single use");
//! # });
//! ```
//!
//! # Implementing [`CodeStore`]
//!
//! Two operations. `put` replaces the code of `(user_id, purpose)`. `check`
//! must be atomic: load the row `FOR UPDATE`, call
//! [`StoredCode::evaluate`] (which holds all the rules), write the row back,
//! commit. When the code's effect (marking an address verified, changing a
//! password) lives in the same database, do it in that transaction too, so a
//! code is only spent when its effect is saved.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use crate::StoreError;
use crate::crypto;
use crate::limits::{Quota, RateLimiter};

/// Shape and lifetime of issued codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodePolicy {
    /// Decimal digits.
    pub length: usize,
    pub ttl: Duration,
    /// Wrong guesses a code survives, counting the one that retires it.
    pub max_attempts: u32,
}

impl Default for CodePolicy {
    fn default() -> Self {
        CodePolicy { length: 6, ttl: Duration::from_secs(10 * 60), max_attempts: 5 }
    }
}

/// What is hashed: `"{user_id}.{purpose}.{code}"`.
///
/// The parts are joined, not length-prefixed, so keep `.` out of purposes
/// (and ideally out of user ids). It is not a way in: a store finds the row
/// by `(user_id, purpose)` first and compares the hash second, so a code only
/// ever meets the row of its own account and step.
pub fn material(user_id: &str, purpose: &str, code: &str) -> String {
    format!("{user_id}.{purpose}.{code}")
}

/// Lowercase hex SHA-256 of [`material`]; in Postgres,
/// `encode(sha256(convert_to($1, 'UTF8')), 'hex')` of the same string.
pub fn code_hash(user_id: &str, purpose: &str, code: &str) -> String {
    crypto::sha256_hex(&material(user_id, purpose, code.trim()))
}

/// The answer to a presented code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeCheck {
    /// Correct; the code is now spent.
    Valid,
    /// Wrong; `remaining` more guesses before the code is retired.
    Wrong { remaining: u32 },
    /// Wrong, and that was the last guess: the code is retired.
    Locked,
    /// No live code: never issued, expired, already used or retired.
    Expired,
}

/// A code as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCode {
    pub user_id: String,
    pub purpose: String,
    /// [`code_hash`].
    pub hash: String,
    pub attempts: u32,
    pub max_attempts: u32,
    /// Seconds since the epoch.
    pub expires_at: u64,
    /// Spent by a correct guess or retired by the last wrong one.
    pub used: bool,
}

impl StoredCode {
    /// Applies one guess: compares in constant time, counts a miss, spends
    /// or retires the code. The store saves `self` afterwards.
    pub fn evaluate(&mut self, presented_hash: &str, now: u64) -> CodeCheck {
        if self.used || self.expires_at <= now || self.attempts >= self.max_attempts {
            return CodeCheck::Expired;
        }
        if crypto::constant_time_eq(self.hash.as_bytes(), presented_hash.as_bytes()) {
            self.used = true;
            return CodeCheck::Valid;
        }
        self.attempts += 1;
        if self.attempts >= self.max_attempts {
            self.used = true;
            CodeCheck::Locked
        } else {
            CodeCheck::Wrong { remaining: self.max_attempts - self.attempts }
        }
    }
}

/// Where codes live; see the [module docs](self#implementing-codestore).
pub trait CodeStore: Send + Sync + 'static {
    /// Stores `code`, replacing any earlier code of its `(user_id, purpose)`.
    fn put(&self, code: StoredCode) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Atomically loads the code of `(user_id, purpose)`, applies
    /// [`StoredCode::evaluate`] and saves it. No code: [`CodeCheck::Expired`].
    fn check(
        &self,
        user_id: &str,
        purpose: &str,
        presented_hash: &str,
        now: u64,
    ) -> impl Future<Output = Result<CodeCheck, StoreError>> + Send;
}

/// Codes in this process: tests and single-instance development.
#[derive(Default)]
pub struct MemoryCodeStore {
    codes: Mutex<HashMap<(String, String), StoredCode>>,
}

impl CodeStore for MemoryCodeStore {
    async fn put(&self, code: StoredCode) -> Result<(), StoreError> {
        let mut codes = self.codes.lock().unwrap_or_else(|e| e.into_inner());
        let now = crypto::now();
        codes.retain(|_, c| !c.used && c.expires_at > now);
        codes.insert((code.user_id.clone(), code.purpose.clone()), code);
        Ok(())
    }

    async fn check(
        &self,
        user_id: &str,
        purpose: &str,
        presented_hash: &str,
        now: u64,
    ) -> Result<CodeCheck, StoreError> {
        let mut codes = self.codes.lock().unwrap_or_else(|e| e.into_inner());
        Ok(codes
            .get_mut(&(user_id.to_owned(), purpose.to_owned()))
            .map_or(CodeCheck::Expired, |code| code.evaluate(presented_hash, now)))
    }
}

/// Issues and checks codes against a [`CodeStore`].
pub struct Codes<S> {
    store: S,
    policy: CodePolicy,
}

impl<S: CodeStore> Codes<S> {
    pub fn new(store: S) -> Self {
        Codes { store, policy: CodePolicy::default() }
    }

    pub fn policy(mut self, policy: CodePolicy) -> Self {
        self.policy =
            CodePolicy { length: policy.length.clamp(4, 12), max_attempts: policy.max_attempts.max(1), ..policy };
        self
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    /// A fresh code for `(user_id, purpose)`, stored hashed; the earlier one
    /// stops working. Returns the digits, to send and then forget.
    pub async fn issue(&self, user_id: &str, purpose: &str) -> Result<String, StoreError> {
        let code = crypto::numeric_code(self.policy.length);
        self.store
            .put(StoredCode {
                user_id: user_id.to_owned(),
                purpose: purpose.to_owned(),
                hash: code_hash(user_id, purpose, &code),
                attempts: 0,
                max_attempts: self.policy.max_attempts,
                expires_at: crypto::now().saturating_add(self.policy.ttl.as_secs()),
                used: false,
            })
            .await?;
        Ok(code)
    }

    /// Checks a presented code (surrounding whitespace is ignored).
    pub async fn verify(&self, user_id: &str, purpose: &str, code: &str) -> Result<CodeCheck, StoreError> {
        self.store.check(user_id, purpose, &code_hash(user_id, purpose, code), crypto::now()).await
    }

    /// The answer to a guess at a *decoy* (a pending step for an address with
    /// no account, which has no code anywhere): the same countdown a real
    /// code gives, counted in `limiter`, so the screens never reveal whether
    /// an account exists.
    pub async fn decoy_miss(&self, limiter: &RateLimiter, decoy_user_id: &str, purpose: &str) -> CodeCheck {
        let quota = Quota::per(self.policy.max_attempts, self.policy.ttl);
        match limiter.check(&decoy_key(decoy_user_id, purpose), quota).await {
            Ok(d) if d.remaining > 0 => CodeCheck::Wrong { remaining: d.remaining },
            Ok(_) => CodeCheck::Locked,
            Err(_) => CodeCheck::Expired,
        }
    }

    /// A decoy "sends" a new code: its countdown starts over.
    pub async fn decoy_reissue(&self, limiter: &RateLimiter, decoy_user_id: &str, purpose: &str) {
        limiter.reset(&decoy_key(decoy_user_id, purpose)).await;
    }
}

fn decoy_key(user_id: &str, purpose: &str) -> String {
    format!("decoy:{purpose}:{user_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes() -> Codes<MemoryCodeStore> {
        Codes::new(MemoryCodeStore::default())
    }

    /// A six-digit code that is not `code`.
    fn wrong(code: &str) -> String {
        if code == "111111" { "222222".into() } else { "111111".into() }
    }

    #[test]
    fn the_hash_binds_user_purpose_and_code() {
        assert_eq!(material("u1", "reset", "123456"), "u1.reset.123456");
        let h = code_hash("u1", "reset", "123456");
        assert_eq!(h, crypto::sha256_hex("u1.reset.123456"));
        assert_eq!(h, code_hash("u1", "reset", " 123456 "), "whitespace from a paste is ignored");
        assert_ne!(h, code_hash("u2", "reset", "123456"), "same digits, another account");
        assert_ne!(h, code_hash("u1", "login", "123456"), "same digits, another step");
    }

    #[tokio::test]
    async fn a_code_works_once_for_its_account_and_purpose() {
        let codes = codes();
        let code = codes.issue("u1", "reset").await.unwrap();
        assert_eq!(code.len(), 6);
        assert_eq!(codes.verify("u2", "reset", &code).await.unwrap(), CodeCheck::Expired, "u2 has no code");
        assert_eq!(codes.verify("u1", "login", &code).await.unwrap(), CodeCheck::Expired);
        assert_eq!(codes.verify("u1", "reset", &code).await.unwrap(), CodeCheck::Valid);
        assert_eq!(codes.verify("u1", "reset", &code).await.unwrap(), CodeCheck::Expired, "spent");
    }

    #[tokio::test]
    async fn wrong_guesses_count_down_and_retire_the_code() {
        let codes = codes();
        let code = codes.issue("u1", "reset").await.unwrap();
        let miss = wrong(&code);
        for remaining in [4, 3, 2, 1] {
            assert_eq!(codes.verify("u1", "reset", &miss).await.unwrap(), CodeCheck::Wrong { remaining });
        }
        assert_eq!(codes.verify("u1", "reset", &miss).await.unwrap(), CodeCheck::Locked);
        assert_eq!(codes.verify("u1", "reset", &code).await.unwrap(), CodeCheck::Expired, "even the right code");
    }

    #[tokio::test]
    async fn a_new_code_retires_the_old_one() {
        let codes = codes();
        let first = codes.issue("u1", "reset").await.unwrap();
        let mut second = codes.issue("u1", "reset").await.unwrap();
        while second == first {
            second = codes.issue("u1", "reset").await.unwrap();
        }
        assert_eq!(codes.verify("u1", "reset", &first).await.unwrap(), CodeCheck::Wrong { remaining: 4 });
        assert_eq!(codes.verify("u1", "reset", &second).await.unwrap(), CodeCheck::Valid);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn parallel_guesses_never_get_more_than_the_allowed_attempts() {
        let codes = std::sync::Arc::new(codes());
        let code = codes.issue("u1", "reset").await.unwrap();
        let miss = wrong(&code);
        let guesses: Vec<_> = (0..64)
            .map(|_| {
                let (codes, miss) = (codes.clone(), miss.clone());
                tokio::spawn(async move { codes.verify("u1", "reset", &miss).await.unwrap() })
            })
            .collect();
        let mut counted = 0;
        for guess in guesses {
            match guess.await.unwrap() {
                CodeCheck::Wrong { .. } | CodeCheck::Locked => counted += 1,
                CodeCheck::Expired => {}
                CodeCheck::Valid => panic!("a wrong code was accepted"),
            }
        }
        assert_eq!(counted, 5, "exactly max_attempts guesses were evaluated");
        assert_eq!(codes.verify("u1", "reset", &code).await.unwrap(), CodeCheck::Expired);
    }

    #[tokio::test]
    async fn hostile_codes_and_lifetimes_are_harmless() {
        let codes = codes().policy(CodePolicy { ttl: Duration::MAX, ..CodePolicy::default() });
        let code = codes.issue("u1", "reset").await.expect("a huge lifetime saturates");
        for junk in ["", "\0", &"9".repeat(1 << 20), "u1.reset.123456", "１２３４５６"] {
            assert!(
                matches!(codes.verify("u1", "reset", junk).await.unwrap(), CodeCheck::Wrong { .. }) || junk == code
            );
            let _ = codes.issue("u1", "reset").await.unwrap();
        }
    }

    #[test]
    fn codes_expire() {
        let mut stored = StoredCode {
            user_id: "u1".into(),
            purpose: "reset".into(),
            hash: code_hash("u1", "reset", "123456"),
            attempts: 0,
            max_attempts: 5,
            expires_at: 1_000,
            used: false,
        };
        let right = stored.hash.clone();
        assert_eq!(stored.clone().evaluate(&right, 999), CodeCheck::Valid);
        assert_eq!(stored.evaluate(&right, 1_000), CodeCheck::Expired);
        assert_eq!(stored.attempts, 0, "an expired code counts nothing");
    }

    #[tokio::test]
    async fn a_decoy_counts_down_like_a_real_code() {
        let codes = codes().policy(CodePolicy { max_attempts: 3, ..CodePolicy::default() });
        let limiter = RateLimiter::memory();
        let real = {
            let code = codes.issue("real", "reset").await.unwrap();
            let miss = wrong(&code);
            let mut seen = Vec::new();
            for _ in 0..4 {
                seen.push(codes.verify("real", "reset", &miss).await.unwrap());
            }
            seen
        };
        let mut decoy = Vec::new();
        for _ in 0..4 {
            decoy.push(codes.decoy_miss(&limiter, "decoy", "reset").await);
        }
        assert_eq!(real, decoy);
        assert_eq!(decoy[3], CodeCheck::Expired);
        codes.decoy_reissue(&limiter, "decoy", "reset").await;
        assert_eq!(codes.decoy_miss(&limiter, "decoy", "reset").await, CodeCheck::Wrong { remaining: 2 });
    }
}
