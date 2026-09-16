use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{BoxFuture, CacheEntry, CacheError, CacheKey, CacheStore};

/// Bounded in-memory LRU store.
///
/// Eviction is O(n) over the entries when the bound is exceeded, which is
/// fine for the few-thousand-entry caches typical of ISR; plug in a
/// dedicated store for larger working sets.
pub struct MemoryStore {
    max_entries: usize,
    clock: AtomicU64,
    map: RwLock<HashMap<CacheKey, (CacheEntry, AtomicU64)>>,
}

impl MemoryStore {
    pub fn new(max_entries: usize) -> Self {
        MemoryStore { max_entries: max_entries.max(1), clock: AtomicU64::new(0), map: RwLock::new(HashMap::new()) }
    }

    pub fn len(&self) -> usize {
        self.map.read().map(|m| m.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }
}

impl CacheStore for MemoryStore {
    fn get<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        let now = self.tick();
        let result = self.map.read().ok().and_then(|m| {
            m.get(key).map(|(e, used)| {
                used.store(now, Ordering::Relaxed);
                e.clone()
            })
        });
        Box::pin(async move { Ok(result) })
    }

    fn set(&self, key: CacheKey, entry: CacheEntry) -> BoxFuture<'_, Result<(), CacheError>> {
        let now = self.tick();
        if let Ok(mut m) = self.map.write() {
            m.insert(key, (entry, AtomicU64::new(now)));
            while m.len() > self.max_entries {
                let oldest = m.iter().min_by_key(|(_, (_, used))| used.load(Ordering::Relaxed)).map(|(k, _)| k.clone());
                match oldest {
                    Some(k) => {
                        m.remove(&k);
                    }
                    None => break,
                }
            }
        }
        Box::pin(async { Ok(()) })
    }

    fn delete<'a>(&'a self, key: &'a CacheKey) -> BoxFuture<'a, Result<bool, CacheError>> {
        let removed = self.map.write().map(|mut m| m.remove(key).is_some()).unwrap_or(false);
        Box::pin(async move { Ok(removed) })
    }

    fn delete_tag<'a>(&'a self, tag: &'a str) -> BoxFuture<'a, Result<usize, CacheError>> {
        let removed = self
            .map
            .write()
            .map(|mut m| {
                let before = m.len();
                m.retain(|_, (e, _)| !e.tags.iter().any(|t| t == tag));
                before - m.len()
            })
            .unwrap_or(0);
        Box::pin(async move { Ok(removed) })
    }

    fn clear(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        if let Ok(mut m) = self.map.write() {
            m.clear();
        }
        Box::pin(async { Ok(()) })
    }
}
