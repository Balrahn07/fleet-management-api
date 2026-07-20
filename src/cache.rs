use async_trait::async_trait;
use dashmap::DashMap;

use std::time::{Duration, Instant};

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;

    async fn set(&self, key: &str, value: String);

    async fn remove(&self, key: &str);
}

struct CacheEntry {
    value: String,
    expires_at: Instant,
}

pub struct InMemoryCache {
    store: DashMap<String, CacheEntry>,
    ttl: Duration,
}

impl InMemoryCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: DashMap::new(),
            ttl,
        }
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Option<String> {
        let entry = self.store.get(key)?;

        if Instant::now() >= entry.expires_at {
            drop(entry);
            self.store.remove(key);
            return None;
        }

        Some(entry.value.clone())
    }

    async fn set(&self, key: &str, value: String) {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };

        self.store.insert(key.to_owned(), entry);
    }

    async fn remove(&self, key: &str) {
        self.store.remove(key);
    }
}
