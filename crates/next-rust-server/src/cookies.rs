//! Cookies with secure defaults.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http::HeaderMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    fn as_str(self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

/// A cookie to set.
///
/// Unless configured explicitly, cookies set through [`Cookies::set`] get
/// `Path=/`, `HttpOnly`, `SameSite=Lax`, and `Secure` outside development.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub max_age: Option<Duration>,
    pub expires: Option<SystemTime>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    pub same_site: Option<SameSite>,
    pub partitioned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieError {
    InvalidName(String),
    InvalidValue(String),
    InvalidAttribute(String),
}

impl std::fmt::Display for CookieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookieError::InvalidName(n) => write!(f, "invalid cookie name {n:?}"),
            CookieError::InvalidValue(n) => {
                write!(f, "invalid value for cookie {n:?}; use Cookie::encoded for arbitrary text")
            }
            CookieError::InvalidAttribute(a) => write!(f, "invalid cookie attribute {a:?}"),
        }
    }
}

impl std::error::Error for CookieError {}

fn is_token(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={}".contains(&b))
}

fn is_cookie_value(s: &str) -> bool {
    s.bytes().all(|b| {
        b == 0x21
            || (0x23..=0x2B).contains(&b)
            || (0x2D..=0x3A).contains(&b)
            || (0x3C..=0x5B).contains(&b)
            || (0x5D..=0x7E).contains(&b)
    })
}

fn is_attr_value(s: &str) -> bool {
    s.bytes().all(|b| (0x20..0x7F).contains(&b) && b != b';')
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Cookie {
            name: name.into(),
            value: value.into(),
            path: None,
            domain: None,
            max_age: None,
            expires: None,
            http_only: None,
            secure: None,
            same_site: None,
            partitioned: false,
        }
    }

    /// Percent-encode `value` so it may contain any text. Read it back with
    /// [`Cookies::get_decoded`].
    pub fn encoded(name: impl Into<String>, value: &str) -> Self {
        Cookie::new(name, percent_encode(value))
    }

    pub fn path(mut self, v: impl Into<String>) -> Self {
        self.path = Some(v.into());
        self
    }
    pub fn domain(mut self, v: impl Into<String>) -> Self {
        self.domain = Some(v.into());
        self
    }
    pub fn max_age(mut self, v: Duration) -> Self {
        self.max_age = Some(v);
        self
    }
    pub fn expires(mut self, v: SystemTime) -> Self {
        self.expires = Some(v);
        self
    }
    pub fn http_only(mut self, v: bool) -> Self {
        self.http_only = Some(v);
        self
    }
    pub fn secure(mut self, v: bool) -> Self {
        self.secure = Some(v);
        self
    }
    pub fn same_site(mut self, v: SameSite) -> Self {
        self.same_site = Some(v);
        self
    }
    pub fn partitioned(mut self, v: bool) -> Self {
        self.partitioned = v;
        self
    }

    /// Serialize as a `Set-Cookie` header value.
    pub fn to_header_value(&self) -> Result<String, CookieError> {
        if !is_token(&self.name) {
            return Err(CookieError::InvalidName(self.name.clone()));
        }
        if !is_cookie_value(&self.value) {
            return Err(CookieError::InvalidValue(self.name.clone()));
        }
        let mut s = format!("{}={}", self.name, self.value);
        if let Some(p) = &self.path {
            if !is_attr_value(p) {
                return Err(CookieError::InvalidAttribute(p.clone()));
            }
            s.push_str("; Path=");
            s.push_str(p);
        }
        if let Some(d) = &self.domain {
            if !is_attr_value(d) {
                return Err(CookieError::InvalidAttribute(d.clone()));
            }
            s.push_str("; Domain=");
            s.push_str(d);
        }
        if let Some(m) = self.max_age {
            s.push_str(&format!("; Max-Age={}", m.as_secs()));
        }
        if let Some(e) = self.expires {
            s.push_str("; Expires=");
            s.push_str(&crate::http_date::format(e));
        }
        if self.http_only == Some(true) {
            s.push_str("; HttpOnly");
        }
        // SameSite=None requires Secure.
        if self.secure == Some(true) || self.same_site == Some(SameSite::None) {
            s.push_str("; Secure");
        }
        if let Some(ss) = self.same_site {
            s.push_str("; SameSite=");
            s.push_str(ss.as_str());
        }
        if self.partitioned {
            s.push_str("; Partitioned");
        }
        Ok(s)
    }
}

#[derive(Debug, Default)]
struct JarInner {
    incoming: Vec<(String, String)>,
    changes: Vec<Cookie>,
}

/// Request cookies plus pending changes, shared between the request, the
/// render context and the response.
#[derive(Debug, Clone, Default)]
pub struct Cookies {
    inner: Arc<Mutex<JarInner>>,
    secure_default: bool,
}

impl Cookies {
    pub fn from_headers(headers: &HeaderMap, secure_default: bool) -> Self {
        let mut incoming = Vec::new();
        for value in headers.get_all(http::header::COOKIE) {
            let Ok(s) = value.to_str() else { continue };
            for pair in s.split(';') {
                if let Some((k, v)) = pair.split_once('=') {
                    let v = v.trim();
                    let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
                    incoming.push((k.trim().to_owned(), v.to_owned()));
                }
            }
        }
        Cookies { inner: Arc::new(Mutex::new(JarInner { incoming, changes: Vec::new() })), secure_default }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, JarInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Current value, reflecting changes made during this request.
    pub fn get(&self, name: &str) -> Option<String> {
        let inner = self.lock();
        if let Some(c) = inner.changes.iter().rev().find(|c| c.name == name) {
            let deleted = c.max_age == Some(Duration::ZERO);
            return if deleted { None } else { Some(c.value.clone()) };
        }
        inner.incoming.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    /// Value decoded with percent-decoding (see [`Cookie::encoded`]).
    pub fn get_decoded(&self, name: &str) -> Option<String> {
        self.get(name).map(|v| percent_decode(&v))
    }

    pub fn all(&self) -> Vec<(String, String)> {
        self.lock().incoming.clone()
    }

    /// Set a cookie (secure defaults are applied to unset attributes).
    pub fn set(&self, mut cookie: Cookie) {
        cookie.path.get_or_insert_with(|| "/".into());
        cookie.http_only.get_or_insert(true);
        cookie.same_site.get_or_insert(SameSite::Lax);
        cookie.secure.get_or_insert(self.secure_default);
        self.lock().changes.push(cookie);
    }

    /// Delete a cookie (on path `/`).
    pub fn delete(&self, name: &str) {
        self.delete_with_path(name, "/");
    }

    pub fn delete_with_path(&self, name: &str, path: &str) {
        let cookie = Cookie::new(name, "")
            .path(path)
            .max_age(Duration::ZERO)
            .expires(UNIX_EPOCH)
            .http_only(true)
            .secure(self.secure_default)
            .same_site(SameSite::Lax);
        self.lock().changes.push(cookie);
    }

    /// `Set-Cookie` header values for all changes (invalid cookies are
    /// skipped and reported).
    pub fn set_cookie_headers(&self) -> Vec<String> {
        let inner = self.lock();
        let mut out = Vec::new();
        for c in &inner.changes {
            match c.to_header_value() {
                Ok(v) => out.push(v),
                Err(e) => crate::log::warn(&format!("cookie not sent: {e}")),
            }
        }
        out
    }

    pub fn has_changes(&self) -> bool {
        !self.lock().changes.is_empty()
    }
}

pub(crate) fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~!$&'()*+:@/?".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(crate) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_attributes() {
        let c = Cookie::new("sid", "abc")
            .path("/")
            .domain("example.com")
            .max_age(Duration::from_secs(60))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict);
        assert_eq!(
            c.to_header_value().unwrap(),
            "sid=abc; Path=/; Domain=example.com; Max-Age=60; HttpOnly; Secure; SameSite=Strict"
        );
        assert!(Cookie::new("a b", "x").to_header_value().is_err());
        assert!(Cookie::new("a", "x;y").to_header_value().is_err());
        assert!(Cookie::new("a", "x\r\nSet-Cookie: evil=1").to_header_value().is_err());
        assert!(Cookie::new("a", "x").path("/;x").to_header_value().is_err());
        assert_eq!(
            Cookie::new("a", "").same_site(SameSite::None).to_header_value().unwrap(),
            "a=; Secure; SameSite=None"
        );
    }

    #[test]
    fn jar_defaults_and_changes() {
        let mut headers = HeaderMap::new();
        headers.insert(http::header::COOKIE, "theme=dark; sid=\"123\"".parse().unwrap());
        let jar = Cookies::from_headers(&headers, true);
        assert_eq!(jar.get("theme").as_deref(), Some("dark"));
        assert_eq!(jar.get("sid").as_deref(), Some("123"));
        jar.set(Cookie::new("theme", "light"));
        assert_eq!(jar.get("theme").as_deref(), Some("light"));
        jar.delete("sid");
        assert_eq!(jar.get("sid"), None);
        let h = jar.set_cookie_headers();
        assert_eq!(h[0], "theme=light; Path=/; HttpOnly; Secure; SameSite=Lax");
        assert!(h[1].starts_with("sid=; Path=/; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly"));
    }

    #[test]
    fn encoded_values_roundtrip() {
        let c = Cookie::encoded("data", "{\"a\": \"b c;\"}");
        assert!(c.to_header_value().is_ok());
        let mut headers = HeaderMap::new();
        headers.insert(http::header::COOKIE, format!("data={}", c.value).parse().unwrap());
        let jar = Cookies::from_headers(&headers, false);
        assert_eq!(jar.get_decoded("data").as_deref(), Some("{\"a\": \"b c;\"}"));
    }
}
