//! Cookies that carry sign-in state.
//!
//! Every cookie set through [`SecureCookies`] is `HttpOnly`, `Path=/`,
//! `SameSite=Lax` and, outside development, `Secure` with the `__Host-`
//! prefix. A browser only accepts a `__Host-` cookie from the exact host,
//! over HTTPS, without a `Domain`: a sibling subdomain can neither plant nor
//! overwrite it.
//!
//! You name cookies by their *base* name (`"at"`); the helper adds the prefix
//! when the jar is in secure mode, so the same code runs on
//! `http://localhost` (where a `__Host-` cookie would be refused) and in
//! production.
//!
//! ```
//! use std::time::Duration;
//! use next_rust_auth::SecureCookies;
//! use next_rust_server::Cookies;
//!
//! let jar = Cookies::default();               // in a page: the `Cookies` extractor
//! let cookies = SecureCookies::new(&jar);
//! cookies.set("flow", "abc", Duration::from_secs(300));
//! assert_eq!(cookies.get("flow").as_deref(), Some("abc"));
//! cookies.clear("flow");
//! assert_eq!(cookies.get("flow"), None);
//! ```

use std::time::Duration;

use next_rust_server::{Cookie, Cookies, SameSite};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::crypto::{AuthKeys, Purpose};

/// Longest cookie value read back; browsers cap a cookie near 4096 bytes.
const MAX_VALUE_LEN: usize = 4096;

/// Browsers cap a cookie's lifetime at 400 days.
pub const MAX_COOKIE_LIFETIME: Duration = Duration::from_secs(400 * 24 * 60 * 60);

/// A view of the request's cookie jar with safe defaults (see the module docs).
#[derive(Debug, Clone)]
pub struct SecureCookies {
    jar: Cookies,
    same_site: SameSite,
    host_prefix: bool,
}

impl SecureCookies {
    /// Over the request's jar (`req.cookies()`, the `Cookies` extractor, or
    /// `ctx.cookies` in a server action). The jar is shared, not copied.
    pub fn new(jar: &Cookies) -> Self {
        SecureCookies { jar: jar.clone(), same_site: SameSite::Lax, host_prefix: jar.secure_default() }
    }

    /// `SameSite=Strict` (or `None`) instead of `Lax`. `Lax` is right for
    /// session cookies: `Strict` signs people out of every link they follow
    /// from another site.
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    /// Turn the `__Host-` prefix off (on is only possible in secure mode).
    /// Needed only when the cookie must be shared across subdomains, which
    /// gives up the prefix's protection.
    pub fn host_prefix(mut self, on: bool) -> Self {
        self.host_prefix = on && self.jar.secure_default();
        self
    }

    /// Whether cookies are `Secure` (production) or plain (development over http).
    pub fn is_secure(&self) -> bool {
        self.jar.secure_default()
    }

    /// The name as sent: `__Host-at` in secure mode, `at` in development.
    pub fn name(&self, base: &str) -> String {
        if self.host_prefix { format!("__Host-{base}") } else { base.to_owned() }
    }

    /// The cookie's value; empty and oversized values count as absent.
    pub fn get(&self, base: &str) -> Option<String> {
        self.jar.get(&self.name(base)).filter(|v| !v.is_empty() && v.len() <= MAX_VALUE_LEN)
    }

    /// Sets the cookie for `lifetime` (capped at [`MAX_COOKIE_LIFETIME`]).
    pub fn set(&self, base: &str, value: &str, lifetime: Duration) {
        self.jar.set(self.cookie(base, value).max_age(lifetime.min(MAX_COOKIE_LIFETIME)));
    }

    /// Sets a cookie that ends with the browser session (no `Max-Age`).
    pub fn set_for_browser_session(&self, base: &str, value: &str) {
        self.jar.set(self.cookie(base, value));
    }

    fn cookie(&self, base: &str, value: &str) -> Cookie {
        Cookie::new(self.name(base), value.to_owned())
            .path("/")
            .http_only(true)
            .secure(self.jar.secure_default())
            .same_site(self.same_site)
    }

    /// Deletes the cookie, but only when the browser has it: a response that
    /// deletes four absent cookies is noise on every page.
    pub fn clear(&self, base: &str) {
        let name = self.name(base);
        if self.jar.get(&name).is_some() {
            self.jar.delete(&name);
        }
    }

    pub fn clear_all(&self, bases: &[&str]) {
        for base in bases {
            self.clear(base);
        }
    }

    /// Seals `data` for `purpose` and stores it for `ttl`: the way to hand a
    /// pending step (which account, which code, where to go next) from one
    /// screen to the next without the browser being able to read or forge it.
    ///
    /// To keep the screens from revealing whether an account exists, store a
    /// *decoy* of the same shape for an unknown address (a random user id and
    /// a `decoy: true` field your code checks) and treat its guesses exactly
    /// like wrong codes (see [`crate::codes::Codes::decoy_miss`]).
    pub fn set_sealed<T: Serialize>(&self, keys: &AuthKeys, purpose: Purpose, base: &str, data: &T, ttl: Duration) {
        if let Ok(sealed) = keys.seal(purpose, data, ttl) {
            self.set(base, &sealed, ttl);
        }
    }

    /// The data stored by [`set_sealed`](Self::set_sealed); `None` when it is
    /// absent, forged, expired or was sealed for another purpose.
    pub fn get_sealed<T: DeserializeOwned>(&self, keys: &AuthKeys, purpose: Purpose, base: &str) -> Option<T> {
        keys.open(purpose, &self.get(base)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secure_jar(cookie_header: Option<&str>) -> Cookies {
        let mut headers = http::HeaderMap::new();
        if let Some(c) = cookie_header {
            headers.insert("cookie", c.parse().unwrap());
        }
        Cookies::from_headers(&headers, true)
    }

    #[test]
    fn secure_mode_uses_the_host_prefix_and_safe_attributes() {
        let jar = secure_jar(None);
        let cookies = SecureCookies::new(&jar);
        assert_eq!(cookies.name("at"), "__Host-at");
        cookies.set("at", "value", Duration::from_secs(900));
        let sent = jar.set_cookie_headers();
        assert_eq!(sent.len(), 1);
        let header = sent[0].to_ascii_lowercase();
        for part in ["__host-at=value", "path=/", "max-age=900", "httponly", "secure", "samesite=lax"] {
            assert!(header.contains(part), "{part} in {header}");
        }
        assert!(!header.contains("domain"));
        assert_eq!(cookies.get("at").as_deref(), Some("value"));
    }

    #[test]
    fn development_uses_the_bare_name() {
        let jar = Cookies::default();
        let cookies = SecureCookies::new(&jar);
        assert_eq!(cookies.name("at"), "at");
        cookies.set("at", "value", Duration::from_secs(60));
        let header = jar.set_cookie_headers()[0].to_ascii_lowercase();
        assert!(header.starts_with("at=value"));
        assert!(!header.contains("secure"));
        assert!(header.contains("httponly"));
        assert_eq!(SecureCookies::new(&jar).host_prefix(true).name("at"), "at", "never prefixed over http");
    }

    #[test]
    fn an_unprefixed_cookie_is_not_read_in_secure_mode() {
        // A sibling subdomain can set `at` for the parent domain; only the
        // `__Host-` cookie counts.
        let jar = secure_jar(Some("at=planted; __Host-rt=real"));
        let cookies = SecureCookies::new(&jar);
        assert_eq!(cookies.get("at"), None);
        assert_eq!(cookies.get("rt").as_deref(), Some("real"));
    }

    #[test]
    fn clear_only_deletes_what_the_browser_has() {
        let jar = secure_jar(Some("__Host-rt=x"));
        let cookies = SecureCookies::new(&jar);
        cookies.clear_all(&["at", "rt"]);
        let sent = jar.set_cookie_headers();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].starts_with("__Host-rt=;"), "{}", sent[0]);
        assert_eq!(cookies.get("rt"), None);
    }

    #[test]
    fn lifetimes_are_capped_and_oversized_values_ignored() {
        let jar = Cookies::default();
        let cookies = SecureCookies::new(&jar);
        cookies.set("did", "x", Duration::from_secs(10_000 * 24 * 3600));
        assert!(jar.set_cookie_headers()[0].contains(&format!("Max-Age={}", MAX_COOKIE_LIFETIME.as_secs())));
        cookies.set_for_browser_session("big", &"x".repeat(5000));
        assert_eq!(cookies.get("big"), None);
    }

    #[test]
    fn sealed_cookies_round_trip_and_ignore_forgeries() {
        let keys = AuthKeys::from_secret(*b"cookie-test secret: 0123456789abcdef").unwrap();
        let jar = Cookies::default();
        let cookies = SecureCookies::new(&jar);
        cookies.set_sealed(&keys, Purpose::CHALLENGE, "ch", &("user-1", "reset"), Duration::from_secs(900));
        let back: Option<(String, String)> = cookies.get_sealed(&keys, Purpose::CHALLENGE, "ch");
        assert_eq!(back, Some(("user-1".into(), "reset".into())));
        assert_eq!(cookies.get_sealed::<(String, String)>(&keys, Purpose::ACCESS, "ch"), None);
        cookies.set("ch", "v1.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", Duration::from_secs(60));
        assert_eq!(cookies.get_sealed::<(String, String)>(&keys, Purpose::CHALLENGE, "ch"), None);
    }
}
