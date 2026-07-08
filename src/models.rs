use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Clone)]
pub struct Vehicle {
    pub id: Uuid,
    pub vin: String,
    pub model: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateVehicleRequest {
    pub vin: String,
    pub model: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateVehicleRequest {
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListVehiclesQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}
