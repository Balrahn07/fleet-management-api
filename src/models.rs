use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
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

#[derive(Deserialize)]
pub struct UpdateVehicleRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ListVehiclesQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub page: i64,
    pub limit: i64,
    pub total_items: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_previous: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug)]
pub struct VehicleFilter {
    pub status: Option<String>,
    pub sort_field: VehicleSortField,
    pub sort_order: SortOrder,
}

#[derive(Debug)]
pub enum VehicleSortField {
    CreatedAt,
    Model,
    Status,
}

#[derive(Debug)]
pub enum SortOrder {
    Asc,
    Desc,
}
