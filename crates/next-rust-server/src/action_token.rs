//! Signed, per-visitor server-action URLs.
//!
//! A server action is never reachable at a fixed address. Views render a
//! *marker* (`action!(f)`), and every HTML response replaces each marker with
//! a freshly minted token, so each page view gets different action URLs:
//!
//! ```text
//! /_next-rust/action/<base64url(version | action key | expiry | nonce | tag)>
//! ```
//!
//! * **action key**: HMAC of the action id under the server secret. It can't
//!   be derived from the source code, so actions can't be enumerated or
//!   guessed.
//! * **expiry**: tokens stop working after `[security] action_token_ttl`.
//! * **nonce**: 64 random bits, so no two page views share a URL.
//! * **tag**: truncated HMAC-SHA256 over all of the above plus the visitor's
//!   binding cookie (`__Host-next_rust_bind`, `HttpOnly`, `SameSite=Strict`). A URL
//!   copied out of one browser is rejected when sent from anywhere else, and
//!   cross-site requests never carry the cookie.
//!
//! The secret comes from `NEXT_RUST_SECRET` (at least 32 bytes). Without it a
//! random per-process secret is used: secure, but links stop working after a
//! restart and are not shared between instances.
//!
//! Tokens prove that a request comes from a browser that loaded one of your
//! pages. They are not authentication: anyone can load a public page and get
//! their own token, so actions must still check who the user is and what
//! they may do.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use next_rust_core::config::CsrfMode;
use sha2::Sha256;

use crate::Cookie;
use crate::cookies::{Cookies, SameSite};
use crate::response::{Body, ByteStream, Response};

type HmacSha256 = Hmac<Sha256>;

/// Environment variable holding the server secret.
pub const SECRET_ENV: &str = "NEXT_RUST_SECRET";
/// Minimum secret length in bytes.
pub const MIN_SECRET_LEN: usize = 32;
/// Binding cookie over plain HTTP (development).
pub const BIND_COOKIE: &str = "next_rust_bind";
/// Binding cookie when cookies are `Secure`: the `__Host-` prefix forbids a
/// `Domain` attribute, so sibling subdomains can't plant a value.
pub const BIND_COOKIE_SECURE: &str = "__Host-next_rust_bind";

const VERSION: u8 = 1;
pub(crate) const KEY_LEN: usize = 12;
const NONCE_LEN: usize = 8;
const TAG_LEN: usize = 16;
const SIGNED_LEN: usize = 1 + KEY_LEN + 4 + NONCE_LEN;
const TOKEN_LEN: usize = SIGNED_LEN + TAG_LEN;
const BINDING_LEN: usize = 64;

/// Identifies an action without revealing its id.
pub(crate) type ActionKey = [u8; KEY_LEN];

pub(crate) struct Keys {
    id: [u8; 32],
    token: [u8; 32],
    /// `nr~<16 hex>~`: start of every marker. Never leaves the server, so
    /// content can't smuggle markers in to be minted.
    marker_prefix: String,
    pub(crate) from_env: bool,
}

fn derive(secret: &[u8], label: &str) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts keys of any length");
    mac.update(b"next-rust/v1/");
    mac.update(label.as_bytes());
    mac.finalize().into_bytes().into()
}

impl Keys {
    fn from_secret(secret: &[u8], from_env: bool) -> Self {
        let marker = derive(secret, "action-marker");
        Keys {
            id: derive(secret, "action-id"),
            token: derive(secret, "action-token"),
            marker_prefix: format!("nr~{}~", hex(&marker[..8])),
            from_env,
        }
    }

    fn random() -> Self {
        let mut secret = [0u8; 32];
        crate::fill_random(&mut secret);
        Keys::from_secret(&secret, false)
    }

    fn tag(&self, signed: &[u8], binding: &str) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.token).expect("HMAC accepts keys of any length");
        mac.update(signed);
        mac.update(binding.as_bytes());
        mac
    }

    fn marker_len(&self) -> usize {
        self.marker_prefix.len() + KEY_LEN * 2 + 1
    }
}

static KEYS: OnceLock<Keys> = OnceLock::new();

pub(crate) fn keys() -> &'static Keys {
    KEYS.get_or_init(|| match std::env::var(SECRET_ENV) {
        Ok(s) if s.len() >= MIN_SECRET_LEN => Keys::from_secret(s.as_bytes(), true),
        // Rejected by `check_secret` when the app is built.
        _ => Keys::random(),
    })
}

/// Validate `NEXT_RUST_SECRET` (called when an app is built).
pub(crate) fn check_secret() -> Result<(), String> {
    match std::env::var(SECRET_ENV) {
        Ok(s) if s.len() < MIN_SECRET_LEN => Err(format!(
            "{SECRET_ENV} must be at least {MIN_SECRET_LEN} bytes (it is {}); generate one with `openssl rand -hex 32`",
            s.len()
        )),
        _ => Ok(()),
    }
}

/// Keyed identifier of an action id.
pub(crate) fn action_key(id: &str) -> ActionKey {
    let mut mac = HmacSha256::new_from_slice(&keys().id).expect("HMAC accepts keys of any length");
    mac.update(id.as_bytes());
    let full = mac.finalize().into_bytes();
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&full[..KEY_LEN]);
    key
}

/// Placeholder URL rendered into HTML; replaced per response by [`seal`].
pub(crate) fn marker_url(id: &str) -> String {
    format!("/_next-rust/action/{}{}~", keys().marker_prefix, hex(&action_key(id)))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Mint a token for `key`, bound to `binding`, valid for `ttl` seconds.
pub(crate) fn mint(key: &ActionKey, binding: &str, ttl: u64) -> String {
    mint_at(keys(), key, binding, now().saturating_add(ttl))
}

fn mint_at(keys: &Keys, key: &ActionKey, binding: &str, expires: u64) -> String {
    let mut buf = [0u8; TOKEN_LEN];
    buf[0] = VERSION;
    buf[1..1 + KEY_LEN].copy_from_slice(key);
    buf[1 + KEY_LEN..SIGNED_LEN - NONCE_LEN].copy_from_slice(&(expires.min(u64::from(u32::MAX)) as u32).to_be_bytes());
    crate::fill_random(&mut buf[SIGNED_LEN - NONCE_LEN..SIGNED_LEN]);
    let tag = keys.tag(&buf[..SIGNED_LEN], binding).finalize().into_bytes();
    buf[SIGNED_LEN..].copy_from_slice(&tag[..TAG_LEN]);
    base64url_encode(&buf)
}

/// Why a token was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rejection {
    Malformed,
    /// The request carries no binding cookie (cross-site, or cookies off).
    Unbound,
    /// Wrong signature: another browser's token, another secret, or tampering.
    Forged,
    Expired,
}

impl Rejection {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Rejection::Malformed => "malformed token",
            Rejection::Unbound => "no binding cookie",
            Rejection::Forged => "signature mismatch",
            Rejection::Expired => "expired token",
        }
    }
}

/// Check a token and return the action it grants.
pub(crate) fn verify(token: &str, binding: Option<&str>) -> Result<ActionKey, Rejection> {
    verify_at(keys(), token, binding, now())
}

fn verify_at(keys: &Keys, token: &str, binding: Option<&str>, now: u64) -> Result<ActionKey, Rejection> {
    let buf =
        base64url_decode(token).filter(|b| b.len() == TOKEN_LEN && b[0] == VERSION).ok_or(Rejection::Malformed)?;
    let binding = binding.ok_or(Rejection::Unbound)?;
    // Constant-time comparison of the truncated tag.
    keys.tag(&buf[..SIGNED_LEN], binding).verify_truncated_left(&buf[SIGNED_LEN..]).map_err(|_| Rejection::Forged)?;
    let mut exp = [0u8; 4];
    exp.copy_from_slice(&buf[1 + KEY_LEN..SIGNED_LEN - NONCE_LEN]);
    if u64::from(u32::from_be_bytes(exp)) < now {
        return Err(Rejection::Expired);
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&buf[1..1 + KEY_LEN]);
    Ok(key)
}

fn bind_cookie_name(cookies: &Cookies) -> &'static str {
    if cookies.secure_default() { BIND_COOKIE_SECURE } else { BIND_COOKIE }
}

/// The visitor's binding value, if the request carries a well-formed one.
pub(crate) fn binding(cookies: &Cookies) -> Option<String> {
    cookies
        .get(bind_cookie_name(cookies))
        .filter(|v| v.len() == BINDING_LEN && v.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn ensure_binding(cookies: &Cookies) -> String {
    if let Some(b) = binding(cookies) {
        return b;
    }
    let value = crate::random_hex(BINDING_LEN / 2);
    // Session cookie: cleared when the browser closes. Strict, so no
    // cross-site request carries it.
    cookies.set(
        Cookie::new(bind_cookie_name(cookies), value.clone()).path("/").http_only(true).same_site(SameSite::Strict),
    );
    value
}

/// Replace action markers in an HTML response with tokens bound to this
/// visitor. Responses that receive tokens become `private, no-store`: they
/// are personal and must not be kept by shared caches.
pub(crate) fn seal(res: &mut Response, cookies: &Cookies, ttl: u64, csrf: CsrfMode) {
    let is_html = res.header("content-type").is_some_and(|c| c.starts_with("text/html"));
    if !is_html {
        return;
    }
    let keys = keys();
    match std::mem::replace(&mut res.body, Body::Empty) {
        Body::Bytes(bytes) if find(&bytes, keys.marker_prefix.as_bytes()).is_some() => {
            let sealer = Sealer { keys, binding: ensure_binding(cookies), ttl };
            res.body = Body::Bytes(Bytes::from(sealer.replace(&bytes)));
            res.set_header("cache-control", "private, no-store");
            ensure_csrf_cookie(cookies, csrf);
        }
        Body::Stream(stream) => {
            let sealer = Sealer { keys, binding: ensure_binding(cookies), ttl };
            res.body = Body::Stream(seal_stream(stream, sealer));
            ensure_csrf_cookie(cookies, csrf);
        }
        body => res.body = body,
    }
}

/// With `csrf = "token"` the client runtime echoes `next_rust_csrf` in a header.
fn ensure_csrf_cookie(cookies: &Cookies, csrf: CsrfMode) {
    if csrf == CsrfMode::Token && cookies.get(crate::CSRF_COOKIE).is_none() {
        cookies.set(Cookie::new(crate::CSRF_COOKIE, crate::random_hex(32)).http_only(false));
    }
}

struct Sealer {
    keys: &'static Keys,
    binding: String,
    ttl: u64,
}

impl Sealer {
    fn replace(&self, input: &[u8]) -> Vec<u8> {
        let prefix = self.keys.marker_prefix.as_bytes();
        let expires = now().saturating_add(self.ttl);
        let mut out = Vec::with_capacity(input.len() + 64);
        let mut rest = input;
        while let Some(at) = find(rest, prefix) {
            out.extend_from_slice(&rest[..at]);
            let tail = &rest[at + prefix.len()..];
            match parse_key(tail) {
                Some(key) => {
                    out.extend_from_slice(mint_at(self.keys, &key, &self.binding, expires).as_bytes());
                    rest = &tail[KEY_LEN * 2 + 1..];
                }
                None => {
                    out.extend_from_slice(prefix);
                    rest = tail;
                }
            }
        }
        out.extend_from_slice(rest);
        out
    }

    /// Start of a trailing partial marker that the next chunk may complete.
    fn hold_back(&self, buf: &[u8]) -> usize {
        let window = self.keys.marker_len() - 1;
        let start = buf.len().saturating_sub(window);
        (start..buf.len()).find(|&i| self.is_marker_prefix(&buf[i..])).unwrap_or(buf.len())
    }

    fn is_marker_prefix(&self, s: &[u8]) -> bool {
        let prefix = self.keys.marker_prefix.as_bytes();
        s.iter().enumerate().all(|(i, &b)| match i.checked_sub(prefix.len()) {
            None => b == prefix[i],
            Some(j) if j < KEY_LEN * 2 => matches!(b, b'0'..=b'9' | b'a'..=b'f'),
            Some(_) => b == b'~',
        })
    }
}

fn seal_stream(inner: ByteStream, sealer: Sealer) -> ByteStream {
    struct State {
        inner: ByteStream,
        sealer: Sealer,
        carry: Vec<u8>,
        done: bool,
    }
    let state = State { inner, sealer, carry: Vec::new(), done: false };
    Box::pin(futures_util::stream::unfold(state, |mut s| async move {
        if s.done {
            return None;
        }
        loop {
            match s.inner.next().await {
                Some(Ok(chunk)) => {
                    s.carry.extend_from_slice(&chunk);
                    let cut = s.sealer.hold_back(&s.carry);
                    if cut == 0 {
                        continue;
                    }
                    let rest = s.carry.split_off(cut);
                    let out = s.sealer.replace(&s.carry);
                    s.carry = rest;
                    return Some((Ok(Bytes::from(out)), s));
                }
                Some(Err(e)) => {
                    s.done = true;
                    return Some((Err(e), s));
                }
                None => {
                    s.done = true;
                    if s.carry.is_empty() {
                        return None;
                    }
                    let out = s.sealer.replace(&std::mem::take(&mut s.carry));
                    return Some((Ok(Bytes::from(out)), s));
                }
            }
        }
    }))
}

fn parse_key(tail: &[u8]) -> Option<ActionKey> {
    let hex_part = tail.get(..KEY_LEN * 2)?;
    if tail.get(KEY_LEN * 2) != Some(&b'~') {
        return None;
    }
    let mut key = [0u8; KEY_LEN];
    for (i, pair) in hex_part.chunks(2).enumerate() {
        let hi = hex_value(pair[0])?;
        let lo = hex_value(pair[1])?;
        key[i] = (hi << 4) | lo;
    }
    Some(key)
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
        for i in 0..=chunk.len() {
            out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    if input.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    for chunk in input.as_bytes().chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            let v = B64.iter().position(|&x| x == c)? as u32;
            n |= v << (18 - 6 * i);
        }
        for i in 0..chunk.len() - 1 {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    // Reject non-canonical encodings (unused low bits set).
    (base64url_encode(&out) == input).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keys() -> Keys {
        Keys::from_secret(b"0123456789abcdef0123456789abcdef", true)
    }

    const BIND: &str = "aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11aa11";

    #[test]
    fn base64url_roundtrip() {
        for len in 0..50 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let enc = base64url_encode(&data);
            assert!(enc.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
            assert_eq!(base64url_decode(&enc), Some(data));
        }
        assert_eq!(base64url_decode("A"), None);
        assert_eq!(base64url_decode("AB"), None, "non-canonical trailing bits");
        assert_eq!(base64url_decode("A+=="), None);
    }

    #[test]
    fn tokens_verify_only_for_their_binding_and_lifetime() {
        let keys = test_keys();
        let key = [7u8; KEY_LEN];
        let token = mint_at(&keys, &key, BIND, 1_000);
        assert_eq!(token.len(), 55);
        assert_eq!(verify_at(&keys, &token, Some(BIND), 999), Ok(key));
        assert_eq!(verify_at(&keys, &token, Some(BIND), 1_000), Ok(key));
        assert_eq!(verify_at(&keys, &token, Some(BIND), 1_001), Err(Rejection::Expired));
        assert_eq!(verify_at(&keys, &token, None, 0), Err(Rejection::Unbound));
        let other = BIND.replace("aa", "bb");
        assert_eq!(verify_at(&keys, &token, Some(&other), 0), Err(Rejection::Forged));
        let other_secret = Keys::from_secret(b"another secret, 32 bytes or more!", true);
        assert_eq!(verify_at(&other_secret, &token, Some(BIND), 0), Err(Rejection::Forged));
        assert_eq!(verify_at(&keys, "", Some(BIND), 0), Err(Rejection::Malformed));
        assert_eq!(verify_at(&keys, "661d1685c535653e", Some(BIND), 0), Err(Rejection::Malformed));
    }

    #[test]
    fn every_token_is_unique_and_tampering_is_detected() {
        let keys = test_keys();
        let key = [1u8; KEY_LEN];
        let a = mint_at(&keys, &key, BIND, 5_000);
        let b = mint_at(&keys, &key, BIND, 5_000);
        assert_ne!(a, b, "a fresh nonce per token");
        let mut raw = base64url_decode(&a).unwrap();
        for i in 0..raw.len() {
            raw[i] ^= 1;
            let forged = base64url_encode(&raw);
            assert!(verify_at(&keys, &forged, Some(BIND), 0).is_err(), "byte {i} is authenticated");
            raw[i] ^= 1;
        }
        // Extending the lifetime or switching action breaks the signature.
        raw[1] ^= 0xff;
        assert_eq!(verify_at(&keys, &base64url_encode(&raw), Some(BIND), 0), Err(Rejection::Forged));
    }

    fn sealer() -> Sealer {
        Sealer { keys: Box::leak(Box::new(test_keys())), binding: BIND.into(), ttl: 60 }
    }

    fn marker(s: &Sealer, key: u8) -> String {
        format!("{}{}~", s.keys.marker_prefix, hex(&[key; KEY_LEN]))
    }

    #[test]
    fn markers_become_tokens() {
        let s = sealer();
        let html = format!(
            r#"<form action="/_next-rust/action/{m1}"></form><a data-x="/_next-rust/action/{m2}">{p}zz</a>"#,
            m1 = marker(&s, 1),
            m2 = marker(&s, 2),
            p = s.keys.marker_prefix
        );
        let out = String::from_utf8(s.replace(html.as_bytes())).unwrap();
        assert!(!out.contains(&marker(&s, 1)) && !out.contains(&marker(&s, 2)));
        assert!(out.contains(&format!("{}zz", s.keys.marker_prefix)), "incomplete markers are left alone");
        let tokens: Vec<&str> =
            out.split("/_next-rust/action/").skip(1).map(|t| t.split('"').next().unwrap()).collect();
        assert_eq!(verify_at(s.keys, tokens[0], Some(BIND), 0), Ok([1; KEY_LEN]));
        assert_eq!(verify_at(s.keys, tokens[1], Some(BIND), 0), Ok([2; KEY_LEN]));
    }

    #[tokio::test]
    async fn markers_split_across_stream_chunks() {
        let s = sealer();
        let keys = s.keys;
        let html = format!("<p>hi</p><form action=\"/_next-rust/action/{}\"></form>n", marker(&s, 3));
        for split in 0..html.len() {
            let chunks: Vec<Result<Bytes, crate::request::BoxError>> = vec![
                Ok(Bytes::copy_from_slice(&html.as_bytes()[..split])),
                Ok(Bytes::copy_from_slice(&html.as_bytes()[split..])),
            ];
            let stream: ByteStream = Box::pin(futures_util::stream::iter(chunks));
            let sealer = Sealer { keys, binding: BIND.into(), ttl: 60 };
            let parts: Vec<Bytes> = seal_stream(stream, sealer).map(|c| c.unwrap()).collect().await;
            let out = String::from_utf8(parts.concat()).unwrap();
            let token = out.split("/_next-rust/action/").nth(1).unwrap().split('"').next().unwrap();
            assert_eq!(verify_at(keys, token, Some(BIND), 0), Ok([3; KEY_LEN]), "split at {split}");
            assert!(out.starts_with("<p>hi</p>") && out.ends_with("</form>n"));
        }
    }
}
