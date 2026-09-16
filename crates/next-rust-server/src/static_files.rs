//! Static file serving for `public/` (and framework client files).
//!
//! Security properties:
//!
//! * the URL path is percent-decoded segment by segment; `..`, `.`, empty
//!   segments, backslashes, NUL bytes and hidden files (`.env`, `.git`) are
//!   rejected before touching the filesystem;
//! * the resolved file is canonicalized and must remain inside the
//!   canonicalized root, so symbolic links cannot escape it;
//! * only regular files are served (no directory listings).

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use bytes::Bytes;
use http::StatusCode;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::app::AppInner;
use crate::request::Request;
use crate::response::{Body, Response};

/// Resolve a URL path against `root` safely. Returns `None` for anything
/// suspicious or non-existent.
pub async fn resolve_safe(root: &Path, url_path: &str) -> Option<(PathBuf, std::fs::Metadata)> {
    let mut rel = PathBuf::new();
    for raw in url_path.split('/').filter(|s| !s.is_empty()) {
        let seg = next_rust_router::decode_segment(raw)?;
        if seg == "." || seg == ".." || seg.starts_with('.') || seg.contains(['\\', '\0', '/']) || seg.contains(':') {
            return None;
        }
        rel.push(seg);
    }
    if rel.as_os_str().is_empty() {
        return None;
    }
    let root = tokio::fs::canonicalize(root).await.ok()?;
    let full = tokio::fs::canonicalize(root.join(&rel)).await.ok()?;
    if !full.starts_with(&root) {
        return None;
    }
    let meta = tokio::fs::metadata(&full).await.ok()?;
    meta.is_file().then_some((full, meta))
}

pub(crate) async fn serve_public(inner: &AppInner, req: &Request) -> Option<Response> {
    let (path, meta) = resolve_safe(&inner.public_dir, req.path()).await?;
    let max_age = inner.config.assets.public_max_age;
    Some(serve_file(req, &path, &meta, &format!("public, max-age={max_age}")).await)
}

/// Serve a resolved file with conditional and range request support.
pub async fn serve_file(req: &Request, path: &Path, meta: &std::fs::Metadata, cache_control: &str) -> Response {
    let len = meta.len();
    let modified = meta.modified().ok();
    let mtime = modified.and_then(|m| m.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
    let etag = format!("W/\"{len:x}-{mtime:x}\"");

    let mut res = Response::default();
    res.set_header("etag", &etag);
    res.set_header("cache-control", cache_control);
    res.set_header("accept-ranges", "bytes");
    res.set_header("content-type", next_rust_assets::mime::from_path(&path.to_string_lossy()));
    if let Some(m) = modified {
        res.set_header("last-modified", &crate::http_date::format(m));
    }

    let not_modified = match req.header("if-none-match") {
        Some(inm) => inm.split(',').any(|t| t.trim() == etag || t.trim() == "*"),
        None => match (req.header("if-modified-since").and_then(crate::http_date::parse), modified) {
            (Some(since), Some(m)) => {
                m.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
                    <= since.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
            }
            _ => false,
        },
    };
    if not_modified {
        res.status = StatusCode::NOT_MODIFIED;
        return res;
    }

    let (start, end) = match req.header("range").map(|r| parse_range(r, len)) {
        None => (0, len.saturating_sub(1)),
        Some(Some((s, e))) => {
            res.status = StatusCode::PARTIAL_CONTENT;
            res.set_header("content-range", &format!("bytes {s}-{e}/{len}"));
            (s, e)
        }
        Some(None) => {
            let mut r = Response::status(416);
            r.set_header("content-range", &format!("bytes */{len}"));
            return r;
        }
    };
    let count = if len == 0 { 0 } else { end - start + 1 };
    res.set_header("content-length", &count.to_string());
    if req.method() == http::Method::HEAD || count == 0 {
        return res;
    }

    let Ok(mut file) = tokio::fs::File::open(path).await else { return Response::status(404) };
    if start > 0 && file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
        return Response::status(500);
    }
    if count <= 64 * 1024 {
        let mut buf = vec![0; count as usize];
        if file.read_exact(&mut buf).await.is_err() {
            return Response::status(500);
        }
        res.body = Body::Bytes(Bytes::from(buf));
        return res;
    }
    let stream = futures_util::stream::unfold((file, count), |(mut file, remaining)| async move {
        if remaining == 0 {
            return None;
        }
        let mut buf = vec![0; remaining.min(64 * 1024) as usize];
        match file.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                Some((Ok::<Bytes, crate::request::BoxError>(Bytes::from(buf)), (file, remaining - n as u64)))
            }
            Err(e) => Some((Err(e.into()), (file, 0))),
        }
    });
    res.body = Body::Stream(Box::pin(stream));
    res
}

/// Parse a single `bytes=` range. `None` = unsatisfiable.
fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?;
    if spec.contains(',') || len == 0 {
        return None;
    }
    let (s, e) = spec.split_once('-')?;
    let (start, end) = if s.is_empty() {
        let suffix: u64 = e.parse().ok()?;
        (len.saturating_sub(suffix), len - 1)
    } else {
        let start: u64 = s.parse().ok()?;
        let end = if e.is_empty() { len - 1 } else { e.parse::<u64>().ok()?.min(len - 1) };
        (start, end)
    };
    (start <= end && start < len).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-9", 100), Some((0, 9)));
        assert_eq!(parse_range("bytes=90-", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=50-500", 100), Some((50, 99)));
        assert_eq!(parse_range("bytes=100-", 100), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 100), None);
        assert_eq!(parse_range("items=0-1", 100), None);
    }

    #[tokio::test]
    async fn traversal_is_blocked() {
        let base = std::env::temp_dir().join(format!("nr-static-{}", std::process::id()));
        let public = base.join("public");
        std::fs::create_dir_all(public.join("images")).unwrap();
        std::fs::write(public.join("images/logo.png"), b"png").unwrap();
        std::fs::write(public.join(".env"), b"SECRET=1").unwrap();
        std::fs::write(base.join("secret.txt"), b"secret").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(base.join("secret.txt"), public.join("link.txt")).unwrap();

        assert!(resolve_safe(&public, "/images/logo.png").await.is_some());
        assert!(resolve_safe(&public, "/images/%6Cogo.png").await.is_some());
        for bad in [
            "/../secret.txt",
            "/images/../../secret.txt",
            "/%2e%2e/secret.txt",
            "/images/..%2F..%2Fsecret.txt",
            "/.env",
            "/images",
            "/",
            "/images\\..\\..\\secret.txt",
            "/link.txt",
            "/images/logo.png%00.txt",
        ] {
            assert!(resolve_safe(&public, bad).await.is_none(), "{bad} must not resolve");
        }
        std::fs::remove_dir_all(base).unwrap();
    }
}
