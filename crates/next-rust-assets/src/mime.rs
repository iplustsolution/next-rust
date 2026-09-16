//! Extension based MIME type detection.

/// Return the MIME type for a path based on its extension.
/// Unknown extensions map to `application/octet-stream`.
pub fn from_path(path: &str) -> &'static str {
    let ext = path.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    from_extension(ext)
}

/// Return the MIME type for an extension (without the leading dot).
pub fn from_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "xml" => "application/xml",
        "txt" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        _ => "application/octet-stream",
    }
}

/// Whether responses of this MIME type benefit from compression.
pub fn is_compressible(mime: &str) -> bool {
    let base = mime.split(';').next().unwrap_or("").trim();
    base.starts_with("text/")
        || matches!(
            base,
            "application/json"
                | "application/javascript"
                | "application/xml"
                | "application/manifest+json"
                | "application/wasm"
                | "image/svg+xml"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert_eq!(from_path("/a/b/logo.PNG"), "image/png");
        assert_eq!(from_path("robots.txt"), "text/plain; charset=utf-8");
        assert_eq!(from_path("noext"), "application/octet-stream");
        assert!(is_compressible("text/html; charset=utf-8"));
        assert!(!is_compressible("image/png"));
    }
}
