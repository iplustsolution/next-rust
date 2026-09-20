//! Randomness, hashing and sealed tokens.
//!
//! A sealed token is JSON encrypted with AES-256-GCM. Its key is derived
//! (HKDF-SHA256) from your secret and the token's [`Purpose`], and the
//! purpose is also the associated data. So a ticket sealed for one use cannot
//! be opened as another, the browser can neither read nor forge one, and
//! every token carries its own expiry, checked on open.
//!
//! ```
//! use std::time::Duration;
//! use next_rust_auth::{AuthKeys, Purpose};
//!
//! const INVITE: Purpose = Purpose::new("invite/v1");
//!
//! let keys = AuthKeys::from_secret(b"an application secret of at least 32 bytes").unwrap();
//! let token = keys.seal(INVITE, &("team-7", "editor"), Duration::from_secs(3600)).unwrap();
//! let (team, role): (String, String) = keys.open(INVITE, &token).unwrap();
//! assert_eq!((team.as_str(), role.as_str()), ("team-7", "editor"));
//! assert!(keys.open::<(String, String)>(Purpose::CHALLENGE, &token).is_err());
//! ```
//!
//! # Rotating the secret
//!
//! Deploy the new secret with the old one as a *previous* key. New tokens are
//! sealed with the current key; tokens sealed before the change still open
//! until they expire. Drop the previous key once the longest token lifetime
//! has passed.
//!
//! ```
//! # use next_rust_auth::AuthKeys;
//! let keys = AuthKeys::from_secret(b"the new secret, thirty-two bytes or more")
//!     .unwrap()
//!     .with_previous(b"the old secret, thirty-two bytes or more")
//!     .unwrap();
//! ```

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64ct::{Base64UrlUnpadded, Encoding};
use hkdf::Hkdf;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

/// Shortest secret accepted by [`AuthKeys::from_secret`].
pub const MIN_SECRET_LEN: usize = 32;
/// A secret must hold at least this many different byte values. Length alone
/// says nothing: thirty-two zeros, or one repeated character, are as good as
/// published. Random secrets pass by a wide margin (32 random hex characters
/// hold about 14 different ones).
pub const MIN_SECRET_DISTINCT_BYTES: usize = 8;

const NONCE_LEN: usize = 12;
const VERSION: &str = "v1.";
/// Tokens are small; anything longer is refused before it is decoded.
const MAX_TOKEN_LEN: usize = 8192;

/// What a sealed token is for. A new kind of token gets a new purpose; a
/// purpose is never reused for different data. Put a version in the label
/// (`"invite/v1"`) so a change of shape can retire the old tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Purpose(&'static str);

impl Purpose {
    /// The access token of [`crate::session`].
    pub const ACCESS: Purpose = Purpose("next-rust-auth/access/v1");
    /// A just-rotated refresh token, parked for concurrent requests.
    pub const GRACE: Purpose = Purpose("next-rust-auth/grace/v1");
    /// A pending verification step (see [`crate::SecureCookies::set_sealed`]).
    pub const CHALLENGE: Purpose = Purpose("next-rust-auth/challenge/v1");

    pub const fn new(label: &'static str) -> Self {
        Purpose(label)
    }

    pub const fn label(self) -> &'static str {
        self.0
    }
}

/// Why [`AuthKeys`] could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// The secret is shorter than [`MIN_SECRET_LEN`] bytes.
    TooShort { len: usize },
    /// The secret is long enough but obviously not random: fewer than
    /// [`MIN_SECRET_DISTINCT_BYTES`] different byte values (a repeated
    /// character, a zeroed buffer, a placeholder).
    Weak,
    /// The environment variable is not set (or not unicode).
    Missing(String),
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::TooShort { len } => {
                write!(f, "auth secret is {len} bytes; at least {MIN_SECRET_LEN} random bytes are required")
            }
            KeyError::Weak => write!(
                f,
                "auth secret is not random (fewer than {MIN_SECRET_DISTINCT_BYTES} different byte values); generate one with `openssl rand -base64 48`"
            ),
            KeyError::Missing(var) => write!(f, "environment variable {var} is not set"),
        }
    }
}

impl std::error::Error for KeyError {}

/// Why a token did not open. Show none of this to the client: every variant
/// means "no".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    /// Not a token of this format (wrong prefix, bad base64, too short or long).
    Malformed,
    /// Sealed for another purpose or with another key, or tampered with.
    Invalid,
    /// Authentic, but past its expiry.
    Expired,
    /// Authentic, but the data is not the type asked for.
    Shape,
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OpenError::Malformed => "malformed token",
            OpenError::Invalid => "token is not authentic for this purpose",
            OpenError::Expired => "token has expired",
            OpenError::Shape => "token data has an unexpected shape",
        })
    }
}

impl std::error::Error for OpenError {}

/// The data could not be serialized to JSON.
#[derive(Debug)]
pub struct SealError(serde_json::Error);

impl std::fmt::Display for SealError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "token data could not be serialized: {}", self.0)
    }
}

impl std::error::Error for SealError {}

/// The secrets tokens are sealed with: one current, any number of previous
/// ones that still open old tokens. Cheap to clone.
#[derive(Clone)]
pub struct AuthKeys {
    /// Current first.
    secrets: std::sync::Arc<Vec<Vec<u8>>>,
    context: std::sync::Arc<str>,
}

impl std::fmt::Debug for AuthKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never the secrets.
        f.debug_struct("AuthKeys").field("keys", &self.secrets.len()).field("context", &self.context).finish()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope<T> {
    /// Expiry, seconds since the epoch.
    exp: u64,
    data: T,
}

impl AuthKeys {
    /// Keys from a secret of at least [`MIN_SECRET_LEN`] bytes. Generate one
    /// with `openssl rand -base64 48` and keep it out of the repository.
    pub fn from_secret(secret: impl AsRef<[u8]>) -> Result<Self, KeyError> {
        let secret = secret.as_ref();
        check_secret(secret)?;
        Ok(AuthKeys { secrets: std::sync::Arc::new(vec![secret.to_vec()]), context: "".into() })
    }

    /// Keys from an environment variable; a second variable, when given and
    /// set, holds the previous secret.
    ///
    /// ```no_run
    /// # use next_rust_auth::AuthKeys;
    /// let keys = AuthKeys::from_env("AUTH_SECRET", Some("AUTH_SECRET_PREVIOUS")).expect("AUTH_SECRET");
    /// ```
    pub fn from_env(var: &str, previous_var: Option<&str>) -> Result<Self, KeyError> {
        let secret = std::env::var(var).map_err(|_| KeyError::Missing(var.to_owned()))?;
        let keys = AuthKeys::from_secret(secret.trim())?;
        match previous_var.and_then(|v| std::env::var(v).ok()).filter(|v| !v.trim().is_empty()) {
            Some(previous) => keys.with_previous(previous.trim()),
            None => Ok(keys),
        }
    }

    /// Also open tokens sealed with `secret` (an earlier current secret).
    /// Call it again for each older secret still in circulation.
    pub fn with_previous(mut self, secret: impl AsRef<[u8]>) -> Result<Self, KeyError> {
        let secret = secret.as_ref();
        check_secret(secret)?;
        std::sync::Arc::make_mut(&mut self.secrets).push(secret.to_vec());
        Ok(self)
    }

    /// Mix an application name into every derived key, so two applications
    /// that (wrongly) share a secret still cannot open each other's tokens.
    pub fn with_context(mut self, context: &str) -> Self {
        self.context = context.into();
        self
    }

    fn cipher(&self, secret: &[u8], purpose: Purpose) -> Aes256Gcm {
        let salt = format!("next-rust-auth/v1|{}", self.context);
        let mut key = [0u8; 32];
        Hkdf::<Sha256>::new(Some(salt.as_bytes()), secret)
            .expand(purpose.label().as_bytes(), &mut key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        Aes256Gcm::new(&key.into())
    }

    /// Seals `data` for `purpose`, valid for `ttl`.
    pub fn seal<T: Serialize>(&self, purpose: Purpose, data: &T, ttl: Duration) -> Result<String, SealError> {
        self.seal_at(purpose, data, ttl, now())
    }

    /// [`seal`](Self::seal) with the clock given (tests, backdated tokens).
    pub fn seal_at<T: Serialize>(
        &self,
        purpose: Purpose,
        data: &T,
        ttl: Duration,
        now: u64,
    ) -> Result<String, SealError> {
        let envelope = Envelope { exp: now.saturating_add(ttl.as_secs()), data };
        let plain = serde_json::to_vec(&envelope).map_err(SealError)?;
        let nonce = random_bytes(NONCE_LEN);
        let sealed = self
            .cipher(&self.secrets[0], purpose)
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: &plain, aad: purpose.label().as_bytes() })
            .expect("AES-GCM encryption of a small buffer cannot fail");
        let mut out = nonce;
        out.extend_from_slice(&sealed);
        Ok(format!("{VERSION}{}", Base64UrlUnpadded::encode_string(&out)))
    }

    /// Opens a token sealed for `purpose` with the current or a previous key.
    pub fn open<T: DeserializeOwned>(&self, purpose: Purpose, token: &str) -> Result<T, OpenError> {
        self.open_at(purpose, token, now())
    }

    /// [`open`](Self::open) with the clock given.
    pub fn open_at<T: DeserializeOwned>(&self, purpose: Purpose, token: &str, now: u64) -> Result<T, OpenError> {
        if token.len() > MAX_TOKEN_LEN {
            return Err(OpenError::Malformed);
        }
        let body = token.strip_prefix(VERSION).ok_or(OpenError::Malformed)?;
        let raw = Base64UrlUnpadded::decode_vec(body).map_err(|_| OpenError::Malformed)?;
        if raw.len() <= NONCE_LEN {
            return Err(OpenError::Malformed);
        }
        let (nonce, sealed) = raw.split_at(NONCE_LEN);
        let plain = self
            .secrets
            .iter()
            .find_map(|secret| {
                self.cipher(secret, purpose)
                    .decrypt(Nonce::from_slice(nonce), Payload { msg: sealed, aad: purpose.label().as_bytes() })
                    .ok()
            })
            .ok_or(OpenError::Invalid)?;
        // The expiry first, on its own: an expired token of another shape is
        // still "expired".
        let exp = serde_json::from_slice::<Envelope<serde::de::IgnoredAny>>(&plain).map_err(|_| OpenError::Shape)?.exp;
        if exp <= now {
            return Err(OpenError::Expired);
        }
        serde_json::from_slice::<Envelope<T>>(&plain).map(|e| e.data).map_err(|_| OpenError::Shape)
    }
}

fn check_secret(secret: &[u8]) -> Result<(), KeyError> {
    if secret.len() < MIN_SECRET_LEN {
        return Err(KeyError::TooShort { len: secret.len() });
    }
    let mut seen = [false; 256];
    for b in secret {
        seen[usize::from(*b)] = true;
    }
    if seen.iter().filter(|s| **s).count() < MIN_SECRET_DISTINCT_BYTES {
        return Err(KeyError::Weak);
    }
    Ok(())
}

/// Seconds since the epoch.
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `n` bytes from the operating system's CSPRNG.
///
/// # Panics
///
/// When the system generator fails: an all-zero secret must never be handed out.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n];
    getrandom::fill(&mut out).expect("operating system random number generator unavailable");
    out
}

/// A 256-bit secret, base64url (43 characters): refresh tokens, reset links.
pub fn secret_token() -> String {
    b64url(&random_bytes(32))
}

/// A 128-bit id, base64url (22 characters): ceremony ids, device ids.
pub fn short_token() -> String {
    b64url(&random_bytes(16))
}

/// A random (version 4) UUID in its canonical text form. Session and family
/// ids use it, so they fit a `uuid` column as well as a text one.
pub fn uuid_v4() -> String {
    let mut b = random_bytes(16);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = hex(&b);
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// A uniformly random integer in `0..upper` (rejection sampling, no modulo bias).
///
/// # Panics
///
/// When `upper` is zero.
pub fn below(upper: u32) -> u32 {
    assert!(upper > 0, "below(0) has no values to choose from");
    let zone = u32::MAX - (u32::MAX % upper);
    loop {
        let bytes = random_bytes(4);
        let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if n < zone {
            return n % upper;
        }
    }
}

/// A one-time code of `len` decimal digits.
pub fn numeric_code(len: usize) -> String {
    (0..len).map(|_| char::from(b'0' + below(10) as u8)).collect()
}

pub fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

/// Lowercase hex SHA-256 (what `encode(sha256(..), 'hex')` gives in SQL).
pub fn sha256_hex(input: &str) -> String {
    hex(&Sha256::digest(input.as_bytes()))
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Equal without leaking where they differ.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    // `black_box` keeps the optimizer from turning the fold into an early exit.
    std::hint::black_box(a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | std::hint::black_box(x ^ y))) == 0
}

pub fn b64url(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

/// Base64url with or without padding.
pub fn from_b64url(text: &str) -> Option<Vec<u8>> {
    Base64UrlUnpadded::decode_vec(text.trim_end_matches('=')).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &[u8] = b"secret-a: 0123456789 abcdefghijklmnop";
    const B: &[u8] = b"secret-b: 9876543210 ponmlkjihgfedcba";
    const MINUTE: Duration = Duration::from_secs(60);

    fn keys() -> AuthKeys {
        AuthKeys::from_secret(A).unwrap()
    }

    #[test]
    fn short_secrets_are_refused() {
        assert_eq!(AuthKeys::from_secret("short").unwrap_err(), KeyError::TooShort { len: 5 });
        assert!(keys().with_previous("short").is_err());
        assert!(!format!("{:?}", keys()).contains("secret-a"), "Debug never prints the secret");
    }

    #[test]
    fn long_but_obviously_weak_secrets_are_refused() {
        assert_eq!(AuthKeys::from_secret("").unwrap_err(), KeyError::TooShort { len: 0 });
        assert_eq!(AuthKeys::from_secret([0u8; 64]).unwrap_err(), KeyError::Weak);
        assert_eq!(AuthKeys::from_secret("a".repeat(48)).unwrap_err(), KeyError::Weak);
        assert_eq!(AuthKeys::from_secret("abababababababababababababababababab").unwrap_err(), KeyError::Weak);
        assert_eq!(keys().with_previous([7u8; 32]).unwrap_err(), KeyError::Weak);
        assert!(KeyError::Weak.to_string().contains("not random"));
        // Random hex and base64 secrets pass.
        assert!(AuthKeys::from_secret("9f86d081884c7d659a2feaa0c55ad015").is_ok());
        assert!(AuthKeys::from_secret(b64url(&random_bytes(32))).is_ok());
    }

    #[test]
    fn expiry_edges_never_overflow() {
        let k = keys();
        // A huge lifetime saturates instead of wrapping to the past.
        let forever = k.seal_at(Purpose::ACCESS, &1u8, Duration::MAX, u64::MAX - 5).unwrap();
        assert_eq!(k.open_at::<u8>(Purpose::ACCESS, &forever, u64::MAX - 1), Ok(1));
        assert_eq!(k.open_at::<u8>(Purpose::ACCESS, &forever, u64::MAX), Err(OpenError::Expired));
        // Sealed at the epoch with no lifetime: exp = 0 is expired at every time.
        let never = k.seal_at(Purpose::ACCESS, &1u8, Duration::ZERO, 0).unwrap();
        assert_eq!(k.open_at::<u8>(Purpose::ACCESS, &never, 0), Err(OpenError::Expired));
        // A clock before the epoch reads as 0; live tokens still open, none is revived.
        let live = k.seal_at(Purpose::ACCESS, &1u8, MINUTE, 1_000).unwrap();
        assert_eq!(k.open_at::<u8>(Purpose::ACCESS, &live, 0), Ok(1));
    }

    #[test]
    fn sealed_tokens_open_only_for_their_purpose() {
        let token = keys().seal(Purpose::CHALLENGE, &"alice", MINUTE).unwrap();
        assert_eq!(keys().open::<String>(Purpose::CHALLENGE, &token).as_deref(), Ok("alice"));
        assert_eq!(keys().open::<String>(Purpose::ACCESS, &token), Err(OpenError::Invalid));
        assert_eq!(keys().open::<String>(Purpose::new("custom/v1"), &token), Err(OpenError::Invalid));
    }

    #[test]
    fn tampered_truncated_and_malformed_tokens_are_refused() {
        let token = keys().seal(Purpose::ACCESS, &42u32, MINUTE).unwrap();
        // Every single-character change is caught.
        for i in VERSION.len()..token.len() {
            let mut bytes = token.clone().into_bytes();
            bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
            let tampered = String::from_utf8(bytes).unwrap();
            if tampered != token {
                assert!(keys().open::<u32>(Purpose::ACCESS, &tampered).is_err(), "byte {i}");
            }
        }
        // Cut short. Dropping characters may leave a non-canonical tail (refused
        // as malformed); dropping whole bytes leaves a broken tag (invalid).
        assert!(keys().open::<u32>(Purpose::ACCESS, &token[..token.len() - 4]).is_err());
        let raw = from_b64url(&token[VERSION.len()..]).unwrap();
        let cut = format!("{VERSION}{}", b64url(&raw[..raw.len() - 3]));
        assert_eq!(keys().open::<u32>(Purpose::ACCESS, &cut), Err(OpenError::Invalid));
        for junk in ["", "v1.", "v2.AAAA", "garbage", "v1.AAAA", "v1.!!!!"] {
            assert_eq!(keys().open::<u32>(Purpose::ACCESS, junk), Err(OpenError::Malformed), "{junk:?}");
        }
        let long = format!("v1.{}", "A".repeat(MAX_TOKEN_LEN));
        assert_eq!(keys().open::<u32>(Purpose::ACCESS, &long), Err(OpenError::Malformed));
    }

    #[test]
    fn tokens_expire() {
        let k = keys();
        let token = k.seal_at(Purpose::ACCESS, &7u8, MINUTE, 1_000).unwrap();
        assert_eq!(k.open_at::<u8>(Purpose::ACCESS, &token, 1_059), Ok(7));
        assert_eq!(k.open_at::<u8>(Purpose::ACCESS, &token, 1_060), Err(OpenError::Expired));
        assert_eq!(k.open_at::<String>(Purpose::ACCESS, &token, 1_060), Err(OpenError::Expired));
        assert_eq!(k.open_at::<String>(Purpose::ACCESS, &token, 1_000), Err(OpenError::Shape));
        let spent = k.seal(Purpose::ACCESS, &7u8, Duration::ZERO).unwrap();
        assert_eq!(k.open::<u8>(Purpose::ACCESS, &spent), Err(OpenError::Expired));
    }

    #[test]
    fn previous_keys_open_old_tokens_and_new_tokens_use_the_current_key() {
        let old = AuthKeys::from_secret(A).unwrap();
        let rotated = AuthKeys::from_secret(B).unwrap().with_previous(A).unwrap();
        let before = old.seal(Purpose::ACCESS, &1u8, MINUTE).unwrap();
        assert_eq!(rotated.open::<u8>(Purpose::ACCESS, &before), Ok(1));
        let after = rotated.seal(Purpose::ACCESS, &2u8, MINUTE).unwrap();
        assert_eq!(old.open::<u8>(Purpose::ACCESS, &after), Err(OpenError::Invalid), "sealed with the new key");
        let retired = AuthKeys::from_secret(B).unwrap();
        assert_eq!(retired.open::<u8>(Purpose::ACCESS, &before), Err(OpenError::Invalid));
        assert_eq!(retired.open::<u8>(Purpose::ACCESS, &after), Ok(2));
    }

    #[test]
    fn a_context_separates_applications_sharing_a_secret() {
        let shop = keys().with_context("shop");
        let admin = keys().with_context("admin");
        let token = shop.seal(Purpose::ACCESS, &1u8, MINUTE).unwrap();
        assert_eq!(shop.open::<u8>(Purpose::ACCESS, &token), Ok(1));
        assert_eq!(admin.open::<u8>(Purpose::ACCESS, &token), Err(OpenError::Invalid));
    }

    #[test]
    fn two_seals_of_the_same_data_differ() {
        let a = keys().seal(Purpose::ACCESS, &1u8, MINUTE).unwrap();
        let b = keys().seal(Purpose::ACCESS, &1u8, MINUTE).unwrap();
        assert_ne!(a, b, "a fresh nonce every time");
    }

    #[test]
    fn codes_are_digits_of_the_right_length() {
        for _ in 0..200 {
            let code = numeric_code(6);
            assert_eq!(code.len(), 6);
            assert!(code.bytes().all(|b| b.is_ascii_digit()));
        }
        let mut seen = [0u32; 10];
        for _ in 0..5000 {
            seen[below(10) as usize] += 1;
        }
        assert!(seen.iter().all(|&n| n > 350), "roughly uniform: {seen:?}");
    }

    #[test]
    fn hashes_tokens_and_ids() {
        assert_eq!(sha256_hex("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"diff"));
        assert!(!constant_time_eq(b"short", b"longer"));
        assert_eq!(secret_token().len(), 43);
        assert_eq!(short_token().len(), 22);
        assert_eq!(from_b64url(&b64url(b"hello")).as_deref(), Some(&b"hello"[..]));
        assert_eq!(from_b64url("aGk=").as_deref(), Some(&b"hi"[..]), "padding is tolerated");
        let id = uuid_v4();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert!(matches!(&id[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(id, uuid_v4());
    }

    // -----------------------------------------------------------------------
    // Deterministic fuzzing: a seeded xorshift, so a failure reproduces.
    // -----------------------------------------------------------------------

    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn bytes(&mut self, n: usize) -> Vec<u8> {
            (0..n).map(|_| self.next() as u8).collect()
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Claims {
        sid: String,
        n: u64,
    }

    #[test]
    fn fuzz_random_input_never_opens_and_never_panics() {
        let k = keys().with_previous(B).unwrap();
        let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.=+/ \0";
        for round in 0..4000 {
            let len = rng.below(200);
            let token = match round % 4 {
                // Arbitrary bytes, lossily made text (multi-byte characters included).
                0 => String::from_utf8_lossy(&rng.bytes(len)).into_owned(),
                // The right prefix and alphabet, random content.
                1 => format!("v1.{}", b64url(&rng.bytes(len))),
                // Token-like characters, with or without the prefix.
                2 => (0..len).map(|_| char::from(ALPHABET[rng.below(ALPHABET.len())])).collect(),
                // The prefix and then anything.
                _ => format!("v1.{}", String::from_utf8_lossy(&rng.bytes(len))),
            };
            assert!(k.open::<Claims>(Purpose::ACCESS, &token).is_err(), "round {round}: {token:?}");
            assert!(k.open::<serde::de::IgnoredAny>(Purpose::CHALLENGE, &token).is_err(), "round {round}");
        }
        // At and around the length limit.
        for len in [MAX_TOKEN_LEN - 4, MAX_TOKEN_LEN - 3, MAX_TOKEN_LEN, MAX_TOKEN_LEN + 1, 1 << 20] {
            let token = format!("v1.{}", "A".repeat(len));
            assert!(k.open::<Claims>(Purpose::ACCESS, &token).is_err());
        }
    }

    #[test]
    fn fuzz_mutated_tokens_never_open_and_never_panic() {
        let k = keys();
        let mut rng = XorShift(0xD1B5_4A32_D192_ED03);
        for round in 0..300 {
            let sid_len = rng.below(40);
            let claims = Claims { sid: b64url(&rng.bytes(sid_len)), n: rng.next() };
            let token = k.seal(Purpose::ACCESS, &claims, MINUTE).unwrap();
            assert_eq!(k.open::<Claims>(Purpose::ACCESS, &token).as_ref(), Ok(&claims));
            let raw = from_b64url(&token[VERSION.len()..]).unwrap();
            let reencode = |bytes: &[u8]| format!("{VERSION}{}", b64url(bytes));

            // Every single bit of nonce, ciphertext and tag matters.
            for _ in 0..24 {
                let mut flipped = raw.clone();
                let bit = rng.below(flipped.len() * 8);
                flipped[bit / 8] ^= 1 << (bit % 8);
                assert_eq!(
                    k.open::<Claims>(Purpose::ACCESS, &reencode(&flipped)),
                    Err(OpenError::Invalid),
                    "round {round}, bit {bit}"
                );
            }
            // Truncated at every byte length, and at a random character.
            for cut in 0..raw.len() {
                assert!(k.open::<Claims>(Purpose::ACCESS, &reencode(&raw[..cut])).is_err(), "cut {cut}");
            }
            let cut = rng.below(token.len());
            assert!(k.open::<Claims>(Purpose::ACCESS, &token[..cut]).is_err(), "round {round}, char cut {cut}");
            // Extended: random bytes after, before, and text glued on.
            let mut longer = raw.clone();
            let extra = 1 + rng.below(40);
            longer.extend(rng.bytes(extra));
            assert!(k.open::<Claims>(Purpose::ACCESS, &reencode(&longer)).is_err());
            let extra = 1 + rng.below(16);
            let mut shifted = rng.bytes(extra);
            shifted.extend_from_slice(&raw);
            assert!(k.open::<Claims>(Purpose::ACCESS, &reencode(&shifted)).is_err());
            for glued in [format!("{token}A"), format!("{token}="), format!("{token}{token}"), format!(" {token}")] {
                assert!(k.open::<Claims>(Purpose::ACCESS, &glued).is_err(), "{glued:?}");
            }
            // Two tokens spliced: one's nonce with the other's body.
            let other = from_b64url(&k.seal(Purpose::ACCESS, &claims, MINUTE).unwrap()[VERSION.len()..]).unwrap();
            let mut spliced = raw[..NONCE_LEN].to_vec();
            spliced.extend_from_slice(&other[NONCE_LEN..]);
            assert_eq!(k.open::<Claims>(Purpose::ACCESS, &reencode(&spliced)), Err(OpenError::Invalid));
        }
    }

    #[test]
    fn fuzz_a_token_for_one_purpose_never_opens_under_another() {
        // Labels that are prefixes, extensions, concatenations and case
        // variants of each other, next to the built-in ones.
        const PURPOSES: &[Purpose] = &[
            Purpose::ACCESS,
            Purpose::GRACE,
            Purpose::CHALLENGE,
            Purpose::new(""),
            Purpose::new("a"),
            Purpose::new("ab"),
            Purpose::new("a/b"),
            Purpose::new("a|b"),
            Purpose::new("invite"),
            Purpose::new("invite/v1"),
            Purpose::new("invite/v10"),
            Purpose::new("invite/v1invite/v1"),
            Purpose::new("Invite/v1"),
            Purpose::new("invite/v1 "),
            Purpose::new("invite/v1\0"),
            Purpose::new("next-rust-auth/access/v1next-rust-auth/grace/v1"),
            Purpose::new("next-rust-auth/access/v"),
        ];
        let k = keys();
        let mut rng = XorShift(0x2545_F491_4F6C_DD1D);
        for (i, sealed_for) in PURPOSES.iter().enumerate() {
            let claims = Claims { sid: b64url(&rng.bytes(16)), n: rng.next() };
            let token = k.seal(*sealed_for, &claims, MINUTE).unwrap();
            for (j, opened_as) in PURPOSES.iter().enumerate() {
                let result = k.open::<Claims>(*opened_as, &token);
                if i == j {
                    assert_eq!(result.as_ref(), Ok(&claims));
                } else {
                    assert_eq!(result, Err(OpenError::Invalid), "{sealed_for:?} opened as {opened_as:?}");
                }
            }
        }
        // The context is not interchangeable with the purpose either: moving
        // text from one to the other gives another key.
        let token = keys().with_context("shop").seal(Purpose::new("cart"), &1u8, MINUTE).unwrap();
        for (context, purpose) in
            [("", "shopcart"), ("shopcart", ""), ("sho", "pcart"), ("shop|", "cart"), ("cart", "shop")]
        {
            let other = keys().with_context(context);
            assert_eq!(
                other.open::<u8>(Purpose::new(purpose), &token),
                Err(OpenError::Invalid),
                "{context:?} {purpose:?}"
            );
        }
    }

    #[test]
    fn fuzz_tokens_under_a_removed_key_fail() {
        let mut rng = XorShift(0x0123_4567_89AB_CDEF);
        for round in 0..50 {
            let (old, new, unrelated) = (rng.bytes(32), rng.bytes(40), rng.bytes(32));
            let before = AuthKeys::from_secret(&old).unwrap();
            let during = AuthKeys::from_secret(&new).unwrap().with_previous(&old).unwrap();
            let after = AuthKeys::from_secret(&new).unwrap();
            let elsewhere = AuthKeys::from_secret(&new).unwrap().with_previous(&unrelated).unwrap();
            let claims = Claims { sid: format!("s{round}"), n: rng.next() };
            let token = before.seal(Purpose::ACCESS, &claims, MINUTE).unwrap();
            assert_eq!(during.open::<Claims>(Purpose::ACCESS, &token).as_ref(), Ok(&claims), "still rotating");
            assert_eq!(after.open::<Claims>(Purpose::ACCESS, &token), Err(OpenError::Invalid), "key removed");
            assert_eq!(elsewhere.open::<Claims>(Purpose::ACCESS, &token), Err(OpenError::Invalid));
            // While rotating, new tokens never use the old key.
            let fresh = during.seal(Purpose::ACCESS, &claims, MINUTE).unwrap();
            assert_eq!(before.open::<Claims>(Purpose::ACCESS, &fresh), Err(OpenError::Invalid));
            assert_eq!(after.open::<Claims>(Purpose::ACCESS, &fresh).as_ref(), Ok(&claims));
        }
    }
}
