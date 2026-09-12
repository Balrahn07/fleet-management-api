use std::{sync::Arc, time::Duration};

use sqlx::PgPool;
use tokio::sync::watch;

use crate::{events::EventPublisher, models::OutboxEvent};

pub const VEHICLE_EVENTS_TOPIC: &str = "vehicle-events";

/// Locks one row until Kafka acknowledges delivery and the database commit succeeds.
/// A crash between those operations can cause a duplicate with the same event ID.
pub async fn publish_next(db: &PgPool, publisher: &dyn EventPublisher) -> Result<bool, String> {
    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    let event = sqlx::query_as::<_, OutboxEvent>(
        r#"
        SELECT id, event_type, aggregate_id, payload, created_at, published_at
        FROM outbox_events
        WHERE published_at IS NULL
        ORDER BY created_at, id
        LIMIT 1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let Some(event) = event else {
        tx.commit().await.map_err(|e| e.to_string())?;
        return Ok(false);
    };

    let payload = serde_json::json!({
        "id": event.id,
        "event_type": event.event_type,
        "aggregate_id": event.aggregate_id,
        "created_at": event.created_at,
        "payload": event.payload,
    })
    .to_string();

    if let Err(error) = publisher
        .publish(
            VEHICLE_EVENTS_TOPIC,
            &event.aggregate_id.to_string(),
            &payload,
        )
        .await
    {
        tx.rollback().await.map_err(|e| e.to_string())?;
        return Err(format!("Failed to publish event {}: {error}", event.id));
    }

    sqlx::query("UPDATE outbox_events SET published_at = NOW() WHERE id = $1")
        .bind(event.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;

    tracing::info!(event_id = %event.id, "Outbox event published");
    Ok(true)
}

/// Processes up to 100 events per tick. Failures remain pending for a later tick.
pub async fn run(
    db: PgPool,
    publisher: Arc<dyn EventPublisher>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            _ = interval.tick() => {}
        }

        for _ in 0..100 {
            if *shutdown.borrow() {
                return;
            }
            match publish_next(&db, publisher.as_ref()).await {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    tracing::warn!(%error, "Outbox delivery failed; retrying on the next tick");
                    break;
                }
            }
        }
    }
}
