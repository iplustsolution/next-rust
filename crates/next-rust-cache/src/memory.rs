use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{BoxFuture, CacheEntry, CacheError, CacheKey, CacheStore};

/// Bounded in-memory LRU store.
///
/// Both bounds are honoured: at most `max_entries` entries, holding at most
/// `max_bytes` of values. The byte bound matters because what a cache holds
/// is often decided by a visitor — a static route with `dynamic_params` and
/// no `generate_params`, or a `cached(&format!("price:{id}"), ..)` fragment
/// key built from a path parameter — so a count alone says nothing about how
/// much memory the process ends up using. One entry is always kept, however
/// large it is.
///
/// Eviction is O(n) over the entries when a bound is exceeded, which is fine
/// for the few-thousand-entry caches typical of ISR; plug in a dedicated
/// store for larger working sets.
pub struct MemoryStore {
    max_entries: usize,
    max_bytes: usize,
    clock: AtomicU64,
    map: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<CacheKey, (CacheEntry, AtomicU64)>,
    bytes: usize,
}

/// What one entry costs, near enough to bound memory by: the value, the key
/// and the tags.
fn weight(key: &CacheKey, entry: &CacheEntry) -> usize {
    entry.value.len() + key.0.len() + entry.tags.iter().map(String::len).sum::<usize>()
}

impl MemoryStore {
    /// At most `max_entries` entries; bytes bounded at a generous 256 MiB so
    /// that a store nobody sized cannot grow without limit.
    pub fn new(max_entries: usize) -> Self {
        Self::with_max_bytes(max_entries, 256 * 1024 * 1024)
    }

    /// At most `max_entries` entries holding at most `max_bytes` of values.
    pub fn with_max_bytes(max_entries: usize, max_bytes: usize) -> Self {
        MemoryStore {
            max_entries: max_entries.max(1),
            max_bytes: max_bytes.max(1),
            clock: AtomicU64::new(0),
            map: RwLock::new(Inner::default()),
        }
    }

    pub fn len(&self) -> usize {
        self.map.read().map(|m| m.entries.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Bytes of values, keys and tags currently held.
    pub fn bytes(&self) -> usize {
        self.map.read().map(|m| m.bytes).unwrap_or(0)
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }
}

impl Inner {
    fn remove(&mut self, key: &CacheKey) -> bool {
        match self.entries.remove(key) {
            Some((entry, _)) => {
                self.bytes = self.bytes.saturating_sub(weight(key, &entry));
                true
            }
            None => false,
        }
    }

    /// Drop least-recently-used entries until both bounds hold. The entry
    /// just written is the most recently used, so it is never the one to go.
    fn evict(&mut self, max_entries: usize, max_bytes: usize) {
        while self.entries.len() > max_entries || (self.bytes > max_bytes && self.entries.len() > 1) {
            let oldest =
                self.entries.iter().min_by_key(|(_, (_, used))| used.load(Ordering::Relaxed)).map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    self.remove(&k);
                }
                None => break,
            }
        }
    }
}

impl CacheStore for MemoryStore {
    fn get<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        let now = self.tick();
        let result = self.map.read().ok().and_then(|m| {
            m.entries.get(key).map(|(e, used)| {
                used.store(now, Ordering::Relaxed);
                e.clone()
            })
        });
        Box::pin(async move { Ok(result) })
    }

    fn set(&self, key: CacheKey, entry: CacheEntry) -> BoxFuture<'_, Result<(), CacheError>> {
        let now = self.tick();
        if let Ok(mut m) = self.map.write() {
            m.remove(&key);
            m.bytes += weight(&key, &entry);
            m.entries.insert(key, (entry, AtomicU64::new(now)));
            m.evict(self.max_entries, self.max_bytes);
        }
        Box::pin(async { Ok(()) })
    }

    fn delete<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<bool, CacheError>> {
        let removed = self.map.write().map(|mut m| m.remove(key)).unwrap_or(false);
        Box::pin(async move { Ok(removed) })
    }

    fn delete_tag<'a>(&'a self, tag: &'a str) -> BoxFuture<'a, Result<usize, CacheError>> {
        let removed = self
            .map
            .write()
            .map(|mut m| {
                let tagged: Vec<CacheKey> = m
                    .entries
                    .iter()
                    .filter(|(_, (e, _))| e.tags.iter().any(|t| t == tag))
                    .map(|(k, _)| k.clone())
                    .collect();
                tagged.iter().filter(|k| m.remove(k)).count()
            })
            .unwrap_or(0);
        Box::pin(async move { Ok(removed) })
    }

    fn clear(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        if let Ok(mut m) = self.map.write() {
            m.entries.clear();
            m.bytes = 0;
        }
        Box::pin(async { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(bytes: usize) -> CacheEntry {
        CacheEntry::new(vec![b'x'; bytes])
    }

    /// What a cache holds is often chosen by a visitor (one page-cache entry
    /// per URL asked for, one fragment per crafted key), so a bound on the
    /// number of entries alone leaves memory open.
    #[tokio::test]
    async fn the_store_is_bounded_in_bytes_as_well_as_in_entries() {
        let store = MemoryStore::with_max_bytes(1_000, 4_096);
        for i in 0..100 {
            store.set(CacheKey::new(format!("k{i}")), entry(1_000)).await.unwrap();
        }
        assert!(store.len() < 100, "entries were evicted: {}", store.len());
        assert!(store.bytes() <= 4_096, "{} bytes held", store.bytes());
        // The newest is the one that is kept.
        assert!(store.get(&CacheKey::new("k99")).await.unwrap().is_some());
        assert!(store.get(&CacheKey::new("k0")).await.unwrap().is_none());

        // An entry larger than the bound is still served: a cache that holds
        // nothing is worse than one over its budget by one entry.
        store.set(CacheKey::new("huge"), entry(100_000)).await.unwrap();
        assert!(store.get(&CacheKey::new("huge")).await.unwrap().is_some());
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn deleting_gives_the_bytes_back() {
        let store = MemoryStore::with_max_bytes(10, 1_000_000);
        store.set(CacheKey::new("a"), entry(100).tags(["t"])).await.unwrap();
        store.set(CacheKey::new("b"), entry(100).tags(["t"])).await.unwrap();
        let full = store.bytes();
        assert!(full >= 200);
        // Writing the same key again replaces, it does not add up.
        store.set(CacheKey::new("a"), entry(100).tags(["t"])).await.unwrap();
        assert_eq!(store.bytes(), full);
        assert_eq!(store.delete_tag("t").await.unwrap(), 2);
        assert_eq!(store.bytes(), 0);
        assert_eq!(store.len(), 0);

        store.set(CacheKey::new("a"), entry(100)).await.unwrap();
        assert!(store.delete(&CacheKey::new("a")).await.unwrap());
        assert_eq!(store.bytes(), 0);
        store.set(CacheKey::new("a"), entry(100)).await.unwrap();
        store.clear().await.unwrap();
        assert_eq!(store.bytes(), 0);
    }
}
