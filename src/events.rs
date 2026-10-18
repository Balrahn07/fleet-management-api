use async_trait::async_trait;
use rdkafka::{
    ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use std::time::Duration;

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, topic: &str, key: &str, payload: &str) -> Result<(), String>;
}

pub struct KafkaPublisher {
    producer: FutureProducer,
}

impl KafkaPublisher {
    pub fn new(brokers: &str) -> Result<Self, String> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "10000")
            .create::<FutureProducer>()
            .map_err(|e| e.to_string())?;

        Ok(Self { producer })
    }
}

#[async_trait]
impl EventPublisher for KafkaPublisher {
    async fn publish(&self, topic: &str, key: &str, payload: &str) -> Result<(), String> {
        self.producer
            .send(
                FutureRecord::to(topic).key(key).payload(payload),
                Duration::from_secs(3),
            )
            .await
            .map_err(|(error, _message)| error.to_string())?;

        Ok(())
    }
}

/// The wire format is unchanged from the original JSON outbox messages.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VehicleStatusEvent {
    pub id: uuid::Uuid,
    pub event_type: VehicleEventType,
    pub aggregate_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub payload: VehicleStatusChanged,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum VehicleEventType {
    VehicleStatusChanged,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VehicleStatusChanged {
    pub vehicle_id: uuid::Uuid,
    pub status: String,
}

impl VehicleStatusEvent {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let event: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if event.aggregate_id != event.payload.vehicle_id {
            return Err("aggregate_id differs from payload.vehicle_id".into());
        }
        if !matches!(
            event.payload.status.as_str(),
            "online" | "offline" | "maintenance"
        ) {
            return Err("Invalid vehicle status".into());
        }
        Ok(event)
    }
}
