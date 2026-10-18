use std::{sync::Arc, time::Duration};

use rdkafka::{
    ClientConfig, Message,
    consumer::{CommitMode, Consumer, StreamConsumer},
};
use sqlx::PgPool;

use crate::events::VehicleStatusEvent;

/// A single INSERT is atomic. A replay of an event already saved is a no-op.
/// No foreign key to vehicles: history must survive deletion and delayed delivery.
pub async fn record_event(db: &PgPool, event: &VehicleStatusEvent) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO vehicle_status_history (event_id, vehicle_id, status, occurred_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(event.id)
    .bind(event.aggregate_id)
    .bind(&event.payload.status)
    .bind(event.created_at)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub fn create_consumer(
    brokers: &str,
    group: &str,
) -> Result<StreamConsumer, rdkafka::error::KafkaError> {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group)
        .set("enable.auto.commit", "false")
        .set("enable.auto.offset.store", "false")
        .set("auto.offset.reset", "earliest")
        .set("max.poll.interval.ms", "300000")
        .set("socket.timeout.ms", "10000")
        .create()
}

fn is_transient(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut | sqlx::Error::WorkerCrashed => true,
        sqlx::Error::Database(error) => error.code().is_some_and(|code| {
            code.starts_with("08")
                || matches!(
                    code.as_ref(),
                    "40001" | "40P01" | "57P01" | "57P02" | "57P03"
                )
        }),
        _ => false,
    }
}

/// Retry only transient database failures. Never advance to the next message on failure.
pub async fn record_with_retry(db: &PgPool, event: &VehicleStatusEvent) -> Result<bool, String> {
    for attempt in 0..5 {
        // Bound processing time below max.poll.interval.ms even during a DB outage.
        let result = tokio::time::timeout(Duration::from_secs(10), record_event(db, event)).await;
        match result {
            Ok(Ok(inserted)) => return Ok(inserted),
            Ok(Err(error)) if !is_transient(&error) => return Err(error.to_string()),
            failure => {
                tracing::warn!(event_id = %event.id, attempt = attempt + 1, ?failure, "History write failed");
                if attempt == 4 {
                    return Err(
                        "History write failed after five attempts; offset was not committed".into(),
                    );
                }
                tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
            }
        }
    }
    unreachable!()
}

/// Process sequentially so a later offset cannot acknowledge an earlier failed record.
pub async fn consume_one(db: &PgPool, consumer: &Arc<StreamConsumer>) -> Result<(), String> {
    let message = consumer.recv().await.map_err(|e| e.to_string())?;
    let context = format!(
        "{}:{}:{}",
        message.topic(),
        message.partition(),
        message.offset()
    );
    let bytes = message
        .payload()
        .ok_or_else(|| format!("{context}: empty Kafka payload"))?;
    let event = VehicleStatusEvent::decode(bytes).map_err(|e| format!("{context}: {e}"))?;
    let inserted = record_with_retry(db, &event)
        .await
        .map_err(|e| format!("{context}: {e}"))?;

    // A synchronous Kafka commit blocks; run it outside Tokio's async worker threads.
    let owned_message = message.detach();
    let committing_consumer = Arc::clone(consumer);
    tokio::task::spawn_blocking(move || {
        use rdkafka::{Offset, TopicPartitionList};
        let mut offsets = TopicPartitionList::new();
        offsets.add_partition_offset(
            owned_message.topic(),
            owned_message.partition(),
            Offset::Offset(owned_message.offset() + 1),
        )?;
        committing_consumer.commit(&offsets, CommitMode::Sync)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{context}: history saved but offset commit failed: {e}"))?;

    tracing::info!(event_id = %event.id, inserted, position = %context, "History saved and offset committed");
    Ok(())
}
