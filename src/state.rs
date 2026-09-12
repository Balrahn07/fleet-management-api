use std::sync::Arc;

use sqlx::PgPool;

use crate::cache::Cache;
use crate::events::EventPublisher;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub cache: Arc<dyn Cache>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
