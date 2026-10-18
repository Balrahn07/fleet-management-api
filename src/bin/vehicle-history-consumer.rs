use std::sync::Arc;

use fleet_management_api::{config::DatabaseConfig, history, outbox::VEHICLE_EVENTS_TOPIC};
use rdkafka::consumer::Consumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let db = DatabaseConfig::from_env().create_pool().await?;
    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let group =
        std::env::var("KAFKA_HISTORY_GROUP").unwrap_or_else(|_| "vehicle-status-history".into());
    let consumer = Arc::new(history::create_consumer(&brokers, &group)?);
    consumer.subscribe(&[VEHICLE_EVENTS_TOPIC])?;
    tracing::info!(%group, topic = VEHICLE_EVENTS_TOPIC, "Vehicle history consumer started");

    let processing = async {
        loop {
            if let Err(error) = history::consume_one(&db, &consumer).await {
                tracing::error!(%error, "Consumer stopped; failed message was not acknowledged");
                return Err::<(), String>(error);
            }
        }
    };
    tokio::select! {
        result = processing => result.map_err(std::io::Error::other)?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            // Cancellation may happen after an INSERT but before a commit: replay is safe.
            tracing::info!("Stopping consumer; unfinished messages may be replayed");
        }
    }
    Ok(())
}
