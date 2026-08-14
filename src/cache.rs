use async_trait::async_trait;
use dashmap::DashMap;

use std::time::{Duration, Instant};

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;

    async fn set(&self, key: &str, value: String) -> Result<(), CacheError>;

    async fn remove(&self, key: &str) -> Result<(), CacheError>;
}

#[derive(Debug)]
pub enum CacheError {
    Backend(String),
    Serialization(String),
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
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let entry = match self.store.get(key) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        if Instant::now() >= entry.expires_at {
            drop(entry);
            self.store.remove(key);
            return Ok(None);
        }

        Ok(Some(entry.value.clone()))
    }

    async fn set(&self, key: &str, value: String) -> Result<(), CacheError> {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };

        self.store.insert(key.to_owned(), entry);

        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<(), CacheError> {
        self.store.remove(key);

        Ok(())
    }
}

pub struct RedisCache {
    client: redis::Client,
    ttl_seconds: u64,
}

impl RedisCache {
    pub fn new(redis_url: &str, ttl_seconds: u64) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;

        Ok(Self {
            client,
            ttl_seconds,
        })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| CacheError::Backend(error.to_string()))?;

        redis::cmd("GET")
            .arg(key)
            .query_async(&mut connection)
            .await
            .map_err(|error| CacheError::Backend(error.to_string()))
    }

    async fn set(&self, key: &str, value: String) -> Result<(), CacheError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| CacheError::Backend(error.to_string()))?;

        redis::cmd("SETEX")
            .arg(key)
            .arg(self.ttl_seconds)
            .arg(value)
            .query_async::<()>(&mut connection)
            .await
            .map_err(|error| CacheError::Backend(error.to_string()))
    }

    async fn remove(&self, key: &str) -> Result<(), CacheError> {
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| CacheError::Backend(error.to_string()))?;

        redis::cmd("DEL")
            .arg(key)
            .query_async::<()>(&mut connection)
            .await
            .map_err(|error| CacheError::Backend(error.to_string()))
    }
}
