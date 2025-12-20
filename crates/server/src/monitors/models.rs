#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct CreateMonitorResponse {
    pub id: String,
    pub status: String,
}
