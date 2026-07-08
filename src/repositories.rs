use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Vehicle;

pub async fn list_vehicles(
    db: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Vehicle>, sqlx::Error> {
    let vehicles = sqlx::query_as!(
        Vehicle,
        r#"
        SELECT id, vin, model, status, created_at, updated_at
        FROM vehicles
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
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

pub async fn update_vehicle(db: &PgPool, id: Uuid, status: String) -> Result<Vehicle, sqlx::Error> {
    let vehicle = sqlx::query_as!(
        Vehicle,
        r#"
        UPDATE vehicles
        SET status = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING id, vin, model, status, created_at, updated_at
        "#,
        status,
        id
    )
    .fetch_one(db)
    .await?;

    Ok(vehicle)
}

pub async fn delete_vehicle(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM vehicles
        WHERE id = $1
        "#,
        id
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn count_vehicles(db: &PgPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM vehicles
        "#
    )
    .fetch_one(db)
    .await?;

    Ok(result.count.unwrap_or(0))
}
