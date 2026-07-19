use std::sync::Arc;

use sqlx::PgPool;

use crate::cache::Cache;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub cache: Arc<dyn Cache>,
}
