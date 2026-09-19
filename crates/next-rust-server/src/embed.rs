//! Files compiled into the production binary.
//!
//! Release builds embed `next-rust.toml`, `public/`, `assets/` and `client/`
//! (see `next-rust-build`), so the server binary is the only file a
//! deployment needs. Development builds leave this empty and read from disk.

/// One embedded file. `path` is relative to its directory and uses `/`.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedFile {
    pub path: &'static str,
    pub bytes: &'static [u8],
    /// Content hash (`next_rust_assets::content_hash`), used for ETags and
    /// to validate hashed asset URLs.
    pub hash: &'static str,
    /// The file compressed with Brotli at build time, when that is smaller.
    pub br: Option<&'static [u8]>,
    /// The file compressed with gzip at build time, when that is smaller.
    pub gz: Option<&'static [u8]>,
}

/// Everything a release binary carries with it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Embedded {
    /// Contents of the configuration file and whether it is JSON.
    pub config: Option<(&'static str, bool)>,
    /// Build time in seconds since the Unix epoch (`Last-Modified`).
    pub built_at: u64,
    /// Sorted by path.
    pub public: &'static [EmbeddedFile],
    /// Sorted by path.
    pub assets: &'static [EmbeddedFile],
    /// Sorted by path.
    pub client: &'static [EmbeddedFile],
}

impl Embedded {
    pub fn is_empty(&self) -> bool {
        self.config.is_none() && self.public.is_empty() && self.assets.is_empty() && self.client.is_empty()
    }
}

/// Find a file by URL path using the same rules as disk lookups: segments are
/// percent-decoded and `.`/`..`, hidden files and separators are rejected.
pub fn find(files: &'static [EmbeddedFile], url_path: &str) -> Option<&'static EmbeddedFile> {
    let mut rel = String::new();
    for raw in url_path.split('/').filter(|s| !s.is_empty()) {
        let seg = next_rust_router::decode_segment(raw)?;
        if seg.starts_with('.') || seg.contains(['\\', '\0', '/', ':']) {
            return None;
        }
        if !rel.is_empty() {
            rel.push('/');
        }
        rel.push_str(&seg);
    }
    if rel.is_empty() {
        return None;
    }
    files.binary_search_by(|f| f.path.cmp(rel.as_str())).ok().map(|i| &files[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: &[EmbeddedFile] = &[
        EmbeddedFile { path: "favicon.svg", bytes: b"<svg/>", hash: "a", br: None, gz: None },
        EmbeddedFile { path: "images/logo space.png", bytes: b"png", hash: "b", br: None, gz: None },
        EmbeddedFile { path: "robots.txt", bytes: b"ok", hash: "c", br: None, gz: None },
    ];

    #[test]
    fn lookups_follow_disk_rules() {
        assert_eq!(find(FILES, "/favicon.svg").map(|f| f.hash), Some("a"));
        assert_eq!(find(FILES, "/images/logo%20space.png").map(|f| f.hash), Some("b"));
        assert_eq!(find(FILES, "//robots.txt").map(|f| f.hash), Some("c"));
        for bad in ["/", "/images", "/../robots.txt", "/.env", "/images%2Flogo%20space.png", "/nope.txt"] {
            assert!(find(FILES, bad).is_none(), "{bad}");
        }
    }
}
