//! Stable content hashing.
//!
//! FNV-1a (64 bit) is used because it is tiny, deterministic across
//! platforms and releases, and fast for the short inputs typical of assets.
//! It is **not** a cryptographic hash and must never be used for security
//! decisions (CSRF tokens, signatures, ...).

/// Incremental FNV-1a 64-bit hasher.
#[derive(Debug, Clone, Copy)]
pub struct Fnv64(u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

impl Fnv64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub const fn new() -> Self {
        Fnv64(Self::OFFSET)
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    pub const fn finish(&self) -> u64 {
        self.0
    }
}

/// Hash `bytes` and return a lowercase 16-character hexadecimal string.
pub fn content_hash(bytes: &[u8]) -> String {
    let mut h = Fnv64::new();
    h.write(bytes);
    format!("{:016x}", h.finish())
}

/// Insert a short content hash before the file extension:
/// `app.css` → `app.1a2b3c4d5e6f7a8b.css`.
pub fn fingerprint_name(file_name: &str, bytes: &[u8]) -> String {
    let hash = content_hash(bytes);
    match file_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{hash}.{ext}"),
        _ => format!("{file_name}.{hash}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        // Reference values for FNV-1a 64.
        let mut h = Fnv64::new();
        h.write(b"");
        assert_eq!(h.finish(), 0xcbf29ce484222325);
        let mut h = Fnv64::new();
        h.write(b"a");
        assert_eq!(h.finish(), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn fingerprint() {
        let name = fingerprint_name("app.css", b"body{}");
        assert!(name.starts_with("app.") && name.ends_with(".css"));
        assert_eq!(name.len(), "app..css".len() + 16);
        assert_eq!(fingerprint_name("LICENSE", b"x").len(), "LICENSE.".len() + 16);
    }
}
