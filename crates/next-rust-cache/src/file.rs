use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{BoxFuture, CacheEntry, CacheError, CacheKey, CacheStore};

/// Persistent store: one `<hash>.bin` (value) and `<hash>.json` (metadata)
/// pair per key. Writes are atomic (write to a temp file, then rename).
///
/// Tag invalidation scans the metadata files, so it is O(entries); this is
/// adequate for single-node deployments. Use a networked store for fleets.
pub struct FileStore {
    dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Meta {
    key: String,
    created_ms: u64,
    revalidate_ms: Option<u64>,
    tags: Vec<String>,
    meta: BTreeMap<String, String>,
}

impl FileStore {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        FileStore { dir: dir.as_ref().to_path_buf() }
    }

    fn paths(&self, key: &CacheKey) -> (PathBuf, PathBuf) {
        let h = next_rust_assets::content_hash(key.0.as_bytes());
        (self.dir.join(format!("{h}.bin")), self.dir.join(format!("{h}.json")))
    }
}

async fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await
}

fn not_found(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::NotFound
}

impl CacheStore for FileStore {
    fn get<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let (bin, json) = self.paths(key);
            let meta = match tokio::fs::read(&json).await {
                Ok(m) => m,
                Err(e) if not_found(&e) => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            let meta: Meta = serde_json::from_slice(&meta).map_err(|e| CacheError::Serde(e.to_string()))?;
            if meta.key != key.0 {
                return Ok(None); // hash collision
            }
            let value = match tokio::fs::read(&bin).await {
                Ok(v) => v,
                Err(e) if not_found(&e) => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            Ok(Some(CacheEntry {
                value: value.into(),
                created: UNIX_EPOCH + Duration::from_millis(meta.created_ms),
                revalidate: meta.revalidate_ms.map(Duration::from_millis),
                tags: meta.tags,
                meta: meta.meta,
            }))
        })
    }

    fn set(&self, key: CacheKey, entry: CacheEntry) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move {
            tokio::fs::create_dir_all(&self.dir).await?;
            let (bin, json) = self.paths(&key);
            let meta = Meta {
                key: key.0,
                created_ms: entry.created.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
                revalidate_ms: entry.revalidate.map(|d| d.as_millis() as u64),
                tags: entry.tags,
                meta: entry.meta,
            };
            write_atomic(&bin, &entry.value).await?;
            let json_bytes = serde_json::to_vec(&meta).map_err(|e| CacheError::Serde(e.to_string()))?;
            write_atomic(&json, &json_bytes).await?;
            Ok(())
        })
    }

    fn delete<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let (bin, json) = self.paths(key);
            let existed = match tokio::fs::remove_file(&json).await {
                Ok(()) => true,
                Err(e) if not_found(&e) => false,
                Err(e) => return Err(e.into()),
            };
            let _ = tokio::fs::remove_file(&bin).await;
            Ok(existed)
        })
    }

    fn delete_tag<'a>(&'a self, tag: &'a str) -> BoxFuture<'a, Result<usize, CacheError>> {
        Box::pin(async move {
            let mut rd = match tokio::fs::read_dir(&self.dir).await {
                Ok(rd) => rd,
                Err(e) if not_found(&e) => return Ok(0),
                Err(e) => return Err(e.into()),
            };
            let mut removed = 0;
            while let Some(entry) = rd.next_entry().await? {
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "json") {
                    continue;
                }
                let Ok(bytes) = tokio::fs::read(&path).await else { continue };
                let Ok(meta) = serde_json::from_slice::<Meta>(&bytes) else { continue };
                if meta.tags.iter().any(|t| t == tag) && self.delete(&CacheKey(meta.key)).await? {
                    removed += 1;
                }
            }
            Ok(removed)
        })
    }

    fn clear(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move {
            match tokio::fs::remove_dir_all(&self.dir).await {
                Ok(()) => Ok(()),
                Err(e) if not_found(&e) => Ok(()),
                Err(e) => Err(e.into()),
            }
        })
    }
}

#[allow(dead_code)]
fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
