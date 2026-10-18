use async_trait::async_trait;
use fleet_management_api::{events::EventPublisher, outbox, repositories};
use sqlx::PgPool;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

#[derive(Default)]
struct Publisher {
    fail: AtomicBool,
    messages: Mutex<Vec<(String, String, serde_json::Value)>>,
}

#[async_trait]
impl EventPublisher for Publisher {
    async fn publish(&self, topic: &str, key: &str, payload: &str) -> Result<(), String> {
        if self.fail.load(Ordering::SeqCst) {
            return Err("broker unavailable".into());
        }
        self.messages.lock().unwrap().push((
            topic.into(),
            key.into(),
            serde_json::from_str(payload).unwrap(),
        ));
        Ok(())
    }
}

#[sqlx::test]
async fn status_update_is_delivered_and_not_republished(pool: PgPool) {
    let vehicle = repositories::create_vehicle(
        &pool,
        Uuid::new_v4(),
        "5YJ3E1EA7KF317123".into(),
        "Model 3".into(),
        "offline".into(),
    )
    .await
    .unwrap();
    repositories::update_vehicle(&pool, vehicle.id, "online".into())
        .await
        .unwrap();
    let publisher = Publisher::default();

    assert!(outbox::publish_next(&pool, &publisher).await.unwrap());
    assert!(!outbox::publish_next(&pool, &publisher).await.unwrap());
    let messages = publisher.messages.lock().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].0, "vehicle-events");
    assert_eq!(messages[0].1, vehicle.id.to_string());
    assert_eq!(messages[0].2["event_type"], "VehicleStatusChanged");
    assert_eq!(messages[0].2["payload"]["status"], "online");
    assert!(messages[0].2["id"].as_str().is_some());
}

#[sqlx::test]
async fn failed_delivery_remains_pending_and_can_retry(pool: PgPool) {
    let event_id = Uuid::new_v4();
    let vehicle_id = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    repositories::insert_outbox_event(
        &mut tx,
        event_id,
        "VehicleStatusChanged",
        vehicle_id,
        serde_json::json!({"vehicle_id": vehicle_id, "status": "online"}),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let publisher = Publisher::default();
    publisher.fail.store(true, Ordering::SeqCst);
    assert!(outbox::publish_next(&pool, &publisher).await.is_err());
    let pending: bool =
        sqlx::query_scalar("SELECT published_at IS NULL FROM outbox_events WHERE id = $1")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(pending);

    publisher.fail.store(false, Ordering::SeqCst);
    assert!(outbox::publish_next(&pool, &publisher).await.unwrap());
    assert_eq!(
        publisher.messages.lock().unwrap()[0].2["id"],
        event_id.to_string()
    );
}

#[sqlx::test]
async fn outbox_failure_rolls_back_vehicle_update(pool: PgPool) {
    let vehicle = repositories::create_vehicle(
        &pool,
        Uuid::new_v4(),
        "5YJ3E1EA7KF317123".into(),
        "Model 3".into(),
        "offline".into(),
    )
    .await
    .unwrap();
    // This constraint affects only the isolated database created for this test.
    sqlx::query("ALTER TABLE outbox_events ADD CONSTRAINT reject_event CHECK (false)")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        repositories::update_vehicle(&pool, vehicle.id, "online".into())
            .await
            .is_err()
    );
    assert_eq!(
        repositories::get_vehicle(&pool, vehicle.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        "offline"
    );
}
