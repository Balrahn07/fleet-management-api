use chrono::{DateTime, Utc};
use rdkafka::{Message, message::Headers};
use serde::{Deserialize, Serialize};

pub const VEHICLE_EVENTS_DLQ: &str = "vehicle-events-dlq";

#[derive(Debug, Serialize, Deserialize)]
pub struct DeadLetterHeader {
    pub key: String,
    pub value_bytes: Option<Vec<u8>>,
}

/// Byte arrays preserve even invalid UTF-8 and distinguish missing from empty payloads.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadLetter {
    pub id: String,
    pub source_topic: String,
    pub source_partition: i32,
    pub source_offset: i64,
    pub source_timestamp_ms: Option<i64>,
    pub key_bytes: Option<Vec<u8>>,
    pub payload_bytes: Option<Vec<u8>>,
    pub headers: Vec<DeadLetterHeader>,
    pub error: String,
    pub rejected_at: DateTime<Utc>,
}

impl DeadLetter {
    pub fn from_message(message: &impl Message, error: String) -> Self {
        Self {
            // Stable for redelivery of this record within the same Kafka cluster/topic lifetime.
            id: format!(
                "{}:{}:{}",
                message.topic(),
                message.partition(),
                message.offset()
            ),
            source_topic: message.topic().into(),
            source_partition: message.partition(),
            source_offset: message.offset(),
            source_timestamp_ms: message.timestamp().to_millis(),
            key_bytes: message.key().map(<[u8]>::to_vec),
            payload_bytes: message.payload().map(<[u8]>::to_vec),
            headers: message
                .headers()
                .map(|headers| {
                    headers
                        .iter()
                        .map(|header| DeadLetterHeader {
                            key: header.key.into(),
                            value_bytes: header.value.map(<[u8]>::to_vec),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            error,
            rejected_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::message::{Header, OwnedHeaders, OwnedMessage, Timestamp};

    #[test]
    fn envelope_preserves_metadata_and_has_stable_identity() {
        let message = OwnedMessage::new(
            Some(vec![255, 0, 254]),
            Some(vec![0, 255]),
            "vehicle-events".into(),
            Timestamp::CreateTime(123),
            2,
            42,
            Some(OwnedHeaders::new().insert(Header {
                key: "trace",
                value: Some(&[0u8, 255][..]),
            })),
        );
        let a = DeadLetter::from_message(&message, "invalid JSON".into());
        let b = DeadLetter::from_message(&message, "another attempt".into());
        assert_eq!(a.id, "vehicle-events:2:42");
        assert_eq!(a.id, b.id);
        let decoded: DeadLetter =
            serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(decoded.payload_bytes, Some(vec![255, 0, 254]));
        assert_eq!(decoded.key_bytes, Some(vec![0, 255]));
        assert_eq!(decoded.source_timestamp_ms, Some(123));
        assert_eq!(decoded.headers[0].value_bytes, Some(vec![0, 255]));
    }

    #[test]
    fn missing_and_empty_payloads_remain_distinct() {
        for payload in [None, Some(vec![])] {
            let message = OwnedMessage::new(
                payload.clone(),
                None,
                "topic".into(),
                Timestamp::NotAvailable,
                0,
                0,
                None,
            );
            let rejected = DeadLetter::from_message(&message, "invalid".into());
            assert_eq!(rejected.payload_bytes, payload);
            assert_eq!(rejected.key_bytes, None);
        }
    }
}
