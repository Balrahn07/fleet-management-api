use chrono::Utc;
use fleet_management_api::{
    events::{VehicleEventType, VehicleStatusChanged, VehicleStatusEvent},
    history,
};
use sqlx::PgPool;
use uuid::Uuid;

fn event() -> VehicleStatusEvent {
    let vehicle_id = Uuid::new_v4();
    VehicleStatusEvent {
        id: Uuid::new_v4(),
        event_type: VehicleEventType::VehicleStatusChanged,
        aggregate_id: vehicle_id,
        created_at: Utc::now(),
        payload: VehicleStatusChanged {
            vehicle_id,
            status: "online".into(),
        },
    }
}

#[test]
fn wire_format_is_compatible_and_invalid_events_are_rejected() {
    let event = event();
    let mut json = serde_json::json!({
        "id": event.id, "event_type": "VehicleStatusChanged",
        "aggregate_id": event.aggregate_id, "created_at": event.created_at,
        "payload": {"vehicle_id": event.aggregate_id, "status": "online"}
    });
    assert_eq!(
        VehicleStatusEvent::decode(json.to_string().as_bytes())
            .unwrap()
            .id,
        event.id
    );
    for invalid in [
        serde_json::json!({"payload": {"vehicle_id": event.aggregate_id, "status": "flying"}}),
        serde_json::json!({"payload": {"vehicle_id": Uuid::new_v4(), "status": "online"}}),
        serde_json::json!({"event_type": "Unknown"}),
        serde_json::json!({"id": "not-a-uuid"}),
    ] {
        let mut candidate = json.clone();
        for (key, value) in invalid.as_object().unwrap() {
            candidate[key] = value.clone();
        }
        assert!(VehicleStatusEvent::decode(candidate.to_string().as_bytes()).is_err());
    }
    json.as_object_mut().unwrap().remove("payload");
    assert!(VehicleStatusEvent::decode(json.to_string().as_bytes()).is_err());
    assert!(VehicleStatusEvent::decode(b"not json").is_err());
}

#[sqlx::test]
async fn duplicate_delivery_creates_one_history_row(pool: PgPool) {
    let event = event();
    let (a, b) = tokio::join!(
        history::record_event(&pool, &event),
        history::record_event(&pool, &event)
    );
    assert_ne!(a.unwrap(), b.unwrap());
    let row: (Uuid, String, chrono::DateTime<Utc>) = sqlx::query_as(
        "SELECT vehicle_id, status, occurred_at FROM vehicle_status_history WHERE event_id = $1",
    )
    .bind(event.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, event.aggregate_id);
    assert_eq!(row.1, "online");
    // PostgreSQL stores microseconds.
    assert_eq!(
        row.2.timestamp_micros(),
        event.created_at.timestamp_micros()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vehicle_status_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn distinct_events_for_same_vehicle_are_retained(pool: PgPool) {
    let mut event = event();
    history::record_event(&pool, &event).await.unwrap();
    event.id = Uuid::new_v4();
    event.payload.status = "maintenance".into();
    history::record_event(&pool, &event).await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM vehicle_status_history WHERE vehicle_id = $1")
            .bind(event.aggregate_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test]
async fn permanent_database_errors_stop_without_retries(pool: PgPool) {
    sqlx::query("DROP TABLE vehicle_status_history")
        .execute(&pool)
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        history::record_with_retry(&pool, &event()),
    )
    .await
    .expect("Schema errors must not be retried");
    assert!(result.is_err());
}

/// Uses an isolated SQLx database, a unique Kafka topic and group; requires Kafka.
#[sqlx::test]
#[ignore = "requires local Kafka; run with --ignored"]
async fn kafka_restart_replay_and_poison_message(pool: PgPool) {
    use fleet_management_api::events::{EventPublisher, KafkaPublisher};
    use rdkafka::{
        ClientConfig, Message, Offset, TopicPartitionList,
        admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
        client::DefaultClientContext,
        consumer::Consumer,
    };
    use std::{sync::Arc, time::Duration};
    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let topic = format!("history-test-{}", Uuid::new_v4());
    let group = format!("history-test-{}", Uuid::new_v4());
    let admin: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()
        .unwrap();
    let results = admin
        .create_topics(
            &[NewTopic::new(&topic, 1, TopicReplication::Fixed(1))],
            &AdminOptions::new(),
        )
        .await
        .unwrap();
    assert!(results[0].is_ok());
    let publisher = KafkaPublisher::new(&brokers).unwrap();
    let event = event();
    let payload = serde_json::to_string(&event).unwrap();
    publisher
        .publish(&topic, &event.aggregate_id.to_string(), &payload)
        .await
        .unwrap();

    let first = history::create_consumer(&brokers, &group).unwrap();
    first.subscribe(&[&topic]).unwrap();
    let message = tokio::time::timeout(Duration::from_secs(60), first.recv())
        .await
        .unwrap()
        .unwrap();
    let decoded = VehicleStatusEvent::decode(message.payload().unwrap()).unwrap();
    history::record_event(&pool, &decoded).await.unwrap();
    drop(message);
    // Simulate exiting after DB success but BEFORE committing the offset.
    first.unsubscribe();
    drop(first);

    let restarted = Arc::new(history::create_consumer(&brokers, &group).unwrap());
    restarted.subscribe(&[&topic]).unwrap();
    tokio::time::timeout(
        Duration::from_secs(60),
        history::consume_one(&pool, &restarted),
    )
    .await
    .unwrap()
    .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vehicle_status_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let mut partitions = TopicPartitionList::new();
    partitions.add_partition(&topic, 0);
    let committed = restarted
        .committed_offsets(partitions.clone(), Duration::from_secs(10))
        .unwrap();
    assert_eq!(
        committed.find_partition(&topic, 0).unwrap().offset(),
        Offset::Offset(1)
    );

    publisher.publish(&topic, "bad", "not-json").await.unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_secs(30),
            history::consume_one(&pool, &restarted)
        )
        .await
        .unwrap()
        .is_err()
    );
    let committed = restarted
        .committed_offsets(partitions, Duration::from_secs(10))
        .unwrap();
    assert_eq!(
        committed.find_partition(&topic, 0).unwrap().offset(),
        Offset::Offset(1)
    );
    restarted.unsubscribe();
    drop(restarted);
    let results = admin
        .delete_topics(&[&topic], &AdminOptions::new())
        .await
        .unwrap();
    assert!(results[0].is_ok());
}

#[sqlx::test]
async fn transient_database_failure_retries_and_succeeds(pool: PgPool) {
    sqlx::raw_sql(
        r#"
        CREATE SEQUENCE history_attempts;
        CREATE FUNCTION fail_first_history_write() RETURNS trigger AS $$
        BEGIN
            IF nextval('history_attempts') = 1 THEN
                RAISE EXCEPTION 'simulated serialization failure' USING ERRCODE = '40001';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        CREATE TRIGGER fail_first_write BEFORE INSERT ON vehicle_status_history
        FOR EACH ROW EXECUTE FUNCTION fail_first_history_write();
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(history::record_with_retry(&pool, &event()).await.unwrap());
    let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM history_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, 2);
}
