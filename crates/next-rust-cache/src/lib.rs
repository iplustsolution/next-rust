//! Cache abstraction used for incremental static regeneration (ISR), the
//! data cache and optimized assets.
//!
//! * [`CacheStore`] – the storage trait. Implement it to plug in Redis,
//!   Memcached, a CDN KV store, ... The framework never requires Redis.
//! * [`MemoryStore`] – bounded in-memory LRU store (default).
//! * [`FileStore`] – persistent store on local disk.
//! * [`Cache`] – typed convenience API over any store.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod file;
mod memory;

pub use file::FileStore;
pub use memory::MemoryStore;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Cache key. Namespaces used by the framework: `page:<path>`, `data:<key>`,
/// `image:<hash>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CacheKey(pub String);

impl CacheKey {
    pub fn new(key: impl Into<String>) -> Self {
        CacheKey(key.into())
    }

    pub fn page(path: &str) -> Self {
        CacheKey(format!("page:{path}"))
    }

    pub fn data(key: &str) -> Self {
        CacheKey(format!("data:{key}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for CacheKey {
    fn from(s: &str) -> Self {
        CacheKey(s.to_owned())
    }
}

/// A cached value with freshness information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub value: Arc<[u8]>,
    pub created: SystemTime,
    /// Entry becomes *stale* (not invalid) after this long. `None` = never.
    pub revalidate: Option<Duration>,
    pub tags: Vec<String>,
    /// Free-form metadata (e.g. response headers, status).
    pub meta: BTreeMap<String, String>,
}

impl CacheEntry {
    pub fn new(value: impl Into<Arc<[u8]>>) -> Self {
        CacheEntry {
            value: value.into(),
            created: SystemTime::now(),
            revalidate: None,
            tags: Vec::new(),
            meta: BTreeMap::new(),
        }
    }

    pub fn revalidate(mut self, after: Option<Duration>) -> Self {
        self.revalidate = after;
        self
    }

    pub fn tags<I: IntoIterator<Item = S>, S: Into<String>>(mut self, tags: I) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn meta(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.meta.insert(k.into(), v.into());
        self
    }

    pub fn age(&self) -> Duration {
        SystemTime::now().duration_since(self.created).unwrap_or_default()
    }

    pub fn is_stale(&self) -> bool {
        self.revalidate.is_some_and(|r| self.age() >= r)
    }
}

#[derive(Debug)]
pub enum CacheError {
    Io(std::io::Error),
    Serde(String),
    Backend(String),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::Io(e) => write!(f, "cache i/o error: {e}"),
            CacheError::Serde(e) => write!(f, "cache serialization error: {e}"),
            CacheError::Backend(e) => write!(f, "cache backend error: {e}"),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<std::io::Error> for CacheError {
    fn from(e: std::io::Error) -> Self {
        CacheError::Io(e)
    }
}

/// Storage backend.
///
/// Implementations must be safe to share between threads. Stale entries are
/// returned by `get` (stale-while-revalidate is decided by the caller).
pub trait CacheStore: Send + Sync + 'static {
    fn get<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>>;
    fn set(&self, key: CacheKey, entry: CacheEntry) -> BoxFuture<'_, Result<(), CacheError>>;
    fn delete<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<bool, CacheError>>;
    /// Delete every entry carrying `tag`; returns the number removed.
    fn delete_tag<'a>(&'a self, tag: &'a str) -> BoxFuture<'a, Result<usize, CacheError>>;
    fn clear(&self) -> BoxFuture<'_, Result<(), CacheError>>;
}

/// Options for [`Cache::get_or_insert_json`].
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    pub revalidate: Option<Duration>,
    pub tags: Vec<String>,
}

impl CacheOptions {
    pub fn revalidate(secs: u64) -> Self {
        CacheOptions { revalidate: Some(Duration::from_secs(secs)), tags: Vec::new() }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Typed cache API over a shared store.
#[derive(Clone)]
pub struct Cache {
    store: Arc<dyn CacheStore>,
}

impl fmt::Debug for Cache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cache")
    }
}

impl Default for Cache {
    fn default() -> Self {
        Cache::new(MemoryStore::new(10_000))
    }
}

impl Cache {
    pub fn new(store: impl CacheStore) -> Self {
        Cache { store: Arc::new(store) }
    }

    pub fn from_arc(store: Arc<dyn CacheStore>) -> Self {
        Cache { store }
    }

    pub fn store(&self) -> &Arc<dyn CacheStore> {
        &self.store
    }

    /// Memoize a serializable value. Stale values are recomputed inline;
    /// errors from `f` are returned and nothing is cached.
    pub async fn get_or_insert_json<T, E, F, Fut>(&self, key: &str, opts: CacheOptions, f: F) -> Result<T, E>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let key = CacheKey::data(key);
        if let Ok(Some(entry)) = self.store.get(&key).await
            && !entry.is_stale()
            && let Ok(v) = serde_json::from_slice(&entry.value)
        {
            return Ok(v);
        }
        let value = f().await?;
        if let Ok(bytes) = serde_json::to_vec(&value) {
            let entry = CacheEntry::new(bytes).revalidate(opts.revalidate).tags(opts.tags);
            let _ = self.store.set(key, entry).await;
        }
        Ok(value)
    }

    /// Invalidate a cached page (ISR) so the next request re-renders it.
    pub async fn revalidate_path(&self, path: &str) -> Result<bool, CacheError> {
        let path = if path.len() > 1 { path.trim_end_matches('/') } else { path };
        self.store.delete(&CacheKey::page(path)).await
    }

    /// Invalidate every page and data entry tagged with `tag`.
    pub async fn revalidate_tag(&self, tag: &str) -> Result<usize, CacheError> {
        self.store.delete_tag(tag).await
    }

    pub async fn invalidate(&self, key: &str) -> Result<bool, CacheError> {
        self.store.delete(&CacheKey::data(key)).await
    }
}

static GLOBAL: OnceLock<Cache> = OnceLock::new();

/// Install the process-wide cache (done by the server at startup).
/// Returns `false` if one was already installed.
pub fn install_global(cache: Cache) -> bool {
    GLOBAL.set(cache).is_ok()
}

/// The process-wide cache (an in-memory default if none was installed).
pub fn global() -> &'static Cache {
    GLOBAL.get_or_init(Cache::default)
}

/// Memoize `f` in the global data cache.
pub async fn cache<T, E, F, Fut>(key: &str, opts: CacheOptions, f: F) -> Result<T, E>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    global().get_or_insert_json(key, opts, f).await
}

/// Re-render `path` on its next request.
pub async fn revalidate_path(path: &str) {
    let _ = global().revalidate_path(path).await;
}

/// Invalidate all entries tagged `tag`.
pub async fn revalidate_tag(tag: &str) {
    let _ = global().revalidate_tag(tag).await;
}

/// Remove a data-cache entry.
pub async fn invalidate(key: &str) {
    let _ = global().invalidate(key).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn exercise(store: impl CacheStore) {
        let cache = Cache::new(store);
        let mut calls = 0;
        for _ in 0..3 {
            let v: Result<u32, ()> = cache
                .get_or_insert_json("answer", CacheOptions::revalidate(60).tag("math"), || {
                    calls += 1;
                    async { Ok(42) }
                })
                .await;
            assert_eq!(v, Ok(42));
        }
        assert_eq!(calls, 1);
        assert_eq!(cache.revalidate_tag("math").await.unwrap(), 1);
        let v: Result<u32, ()> = cache.get_or_insert_json("answer", CacheOptions::default(), || async { Ok(7) }).await;
        assert_eq!(v, Ok(7));

        let store = cache.store();
        let entry = CacheEntry::new(b"<html>".to_vec()).tags(["posts"]).meta("status", "200");
        store.set(CacheKey::page("/blog"), entry.clone()).await.unwrap();
        let got = store.get(&CacheKey::page("/blog")).await.unwrap().unwrap();
        assert_eq!(&*got.value, b"<html>");
        assert_eq!(got.meta["status"], "200");
        assert!(cache.revalidate_path("/blog/").await.unwrap());
        assert!(store.get(&CacheKey::page("/blog")).await.unwrap().is_none());

        let stale =
            CacheEntry { created: SystemTime::now() - Duration::from_secs(120), ..CacheEntry::new(b"x".to_vec()) }
                .revalidate(Some(Duration::from_secs(60)));
        assert!(stale.is_stale());
        store.set(CacheKey::new("s"), stale).await.unwrap();
        assert!(store.get(&CacheKey::new("s")).await.unwrap().unwrap().is_stale());
        store.clear().await.unwrap();
        assert!(store.get(&CacheKey::new("s")).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn memory_store() {
        exercise(MemoryStore::new(100)).await;
    }

    #[tokio::test]
    async fn file_store() {
        let dir = std::env::temp_dir().join(format!("nr-cache-{}", std::process::id()));
        exercise(FileStore::new(&dir)).await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn memory_store_evicts_least_recently_used() {
        let store = MemoryStore::new(2);
        store.set("a".into(), CacheEntry::new(b"1".to_vec())).await.unwrap();
        store.set("b".into(), CacheEntry::new(b"2".to_vec())).await.unwrap();
        store.get(&"a".into()).await.unwrap();
        store.set("c".into(), CacheEntry::new(b"3".to_vec())).await.unwrap();
        assert!(store.get(&"a".into()).await.unwrap().is_some());
        assert!(store.get(&"b".into()).await.unwrap().is_none());
    }
}
