use async_trait::async_trait;
use dashmap::DashMap;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;

    async fn set(&self, key: &str, value: String);

    async fn remove(&self, key: &str);
}

pub struct InMemoryCache {
    store: DashMap<String, String>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Option<String> {
        self.store.get(key).map(|value| value.clone())
    }

    async fn set(&self, key: &str, value: String) {
        self.store.insert(key.to_owned(), value);
    }

    async fn remove(&self, key: &str) {
        self.store.remove(key);
    }
}
