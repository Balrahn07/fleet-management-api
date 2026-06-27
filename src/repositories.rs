use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Vehicle;

pub async fn list_vehicles(db: &PgPool) -> Result<Vec<Vehicle>, sqlx::Error> {
    let vehicles = sqlx::query_as!(
        Vehicle,
        r#"
        SELECT id, vin, model, status, created_at, updated_at
        FROM vehicles
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(db)
    .await?;

    Ok(vehicles)
}

pub async fn get_vehicle(db: &PgPool, id: Uuid) -> Result<Option<Vehicle>, sqlx::Error> {
    let vehicle = sqlx::query_as!(
        Vehicle,
        r#"
        SELECT id, vin, model, status, created_at, updated_at
        FROM vehicles
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db)
    .await?;

    Ok(vehicle)
}

pub async fn create_vehicle(
    db: &PgPool,
    id: Uuid,
    vin: String,
    model: String,
    status: String,
) -> Result<Vehicle, sqlx::Error> {
    let vehicle = sqlx::query_as!(
        Vehicle,
        r#"
        INSERT INTO vehicles (id, vin, model, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id, vin, model, status, created_at, updated_at
        "#,
        id,
        vin,
        model,
        status
    )
    .fetch_one(db)
    .await?;

    Ok(vehicle)
}
