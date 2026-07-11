use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::models::{SortOrder, Vehicle, VehicleFilter, VehicleSortField};

pub async fn list_vehicles(
    db: &PgPool,
    limit: i64,
    offset: i64,
    filter: &VehicleFilter,
) -> Result<Vec<Vehicle>, sqlx::Error> {
    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        SELECT id, vin, model, status, driver_id, created_at, updated_at
        FROM vehicles
        "#,
    );

    if let Some(status) = filter.status.as_deref() {
        query.push(" WHERE status = ");
        query.push_bind(status);
    }

    query.push(" ORDER BY ");

    match filter.sort_field {
        VehicleSortField::CreatedAt => query.push("created_at"),
        VehicleSortField::Model => query.push("model"),
        VehicleSortField::Status => query.push("status"),
    };

    match filter.sort_order {
        SortOrder::Asc => query.push(" ASC, id ASC"),
        SortOrder::Desc => query.push(" DESC, id DESC"),
    };

    query.push(" LIMIT ");
    query.push_bind(limit);

    query.push(" OFFSET ");
    query.push_bind(offset);

    let vehicles = query.build_query_as::<Vehicle>().fetch_all(db).await?;

    Ok(vehicles)
}

pub async fn get_vehicle(db: &PgPool, id: Uuid) -> Result<Option<Vehicle>, sqlx::Error> {
    let vehicle = sqlx::query_as!(
        Vehicle,
        r#"
        SELECT id, vin, model, status, driver_id, created_at, updated_at
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
        RETURNING id, vin, model, status, driver_id, created_at, updated_at
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
        RETURNING id, vin, model, status, driver_id, created_at, updated_at
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

pub async fn count_vehicles(db: &PgPool, filter: &VehicleFilter) -> Result<i64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM vehicles
        WHERE ($1::text IS NULL OR status = $1)
        "#,
        filter.status.as_deref()
    )
    .fetch_one(db)
    .await?;

    Ok(result.count.unwrap_or(0))
}

pub async fn find_vehicle_driver_for_update(
    tx: &mut Transaction<'_, Postgres>,
    vehicle_id: Uuid,
) -> Result<Option<Option<Uuid>>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT driver_id
        FROM vehicles
        WHERE id = $1
        FOR UPDATE
        "#,
        vehicle_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    Ok(result)
}

pub async fn driver_exists(
    tx: &mut Transaction<'_, Postgres>,
    driver_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM drivers
            WHERE id = $1
        )
        "#,
        driver_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);

    Ok(exists)
}

pub async fn is_driver_assigned(
    tx: &mut Transaction<'_, Postgres>,
    driver_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM vehicles
            WHERE driver_id = $1
        )
        "#,
        driver_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);

    Ok(exists)
}

pub async fn assign_driver_to_vehicle(
    tx: &mut Transaction<'_, Postgres>,
    vehicle_id: Uuid,
    driver_id: Uuid,
) -> Result<Vehicle, sqlx::Error> {
    let vehicle = sqlx::query_as!(
        Vehicle,
        r#"
        UPDATE vehicles
        SET driver_id = $1,
            updated_at = NOW()
        WHERE id = $2
        RETURNING
            id,
            vin,
            model,
            status,
            driver_id,
            created_at,
            updated_at
        "#,
        driver_id,
        vehicle_id
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(vehicle)
}
