use std::{env, time::Duration};

use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        let url = required_env("DATABASE_URL");

        let max_connections = parse_env_or_default("DATABASE_MAX_CONNECTIONS", 20);

        let min_connections = parse_env_or_default("DATABASE_MIN_CONNECTIONS", 2);

        let acquire_timeout_seconds = parse_env_or_default("DATABASE_ACQUIRE_TIMEOUT_SECONDS", 3);

        let idle_timeout_seconds = parse_env_or_default("DATABASE_IDLE_TIMEOUT_SECONDS", 600);

        let max_lifetime_seconds = parse_env_or_default("DATABASE_MAX_LIFETIME_SECONDS", 1800);

        if min_connections > max_connections {
            panic!(
                "DATABASE_MIN_CONNECTIONS ({min_connections}) cannot be greater \
                 than DATABASE_MAX_CONNECTIONS ({max_connections})"
            );
        }

        Self {
            url,
            max_connections,
            min_connections,
            acquire_timeout: Duration::from_secs(acquire_timeout_seconds),
            idle_timeout: Duration::from_secs(idle_timeout_seconds),
            max_lifetime: Duration::from_secs(max_lifetime_seconds),
        }
    }

    pub async fn create_pool(&self) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(self.acquire_timeout)
            .idle_timeout(Some(self.idle_timeout))
            .max_lifetime(Some(self.max_lifetime))
            .connect(&self.url)
            .await
    }
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

fn parse_env_or_default<T>(name: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .unwrap_or_else(|error| panic!("{name} has an invalid value: {error}")),
        Err(env::VarError::NotPresent) => default,
        Err(error) => panic!("Failed to read {name}: {error}"),
    }
}
