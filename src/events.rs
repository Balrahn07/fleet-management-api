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
