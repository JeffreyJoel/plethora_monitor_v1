//! # Monitor Models
//!
//! Request and response types for monitor API endpoints.
//! These types are used for serialization/deserialization and OpenAPI documentation.

use monitor::primitives::models::MonitorConfig;
use uuid::Uuid;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct CreateMonitorResponse {
    pub id: String,
    pub status: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct MonitorResponse {
    pub id: Uuid,
    #[serde(flatten)]
    pub config: MonitorConfig,
}
