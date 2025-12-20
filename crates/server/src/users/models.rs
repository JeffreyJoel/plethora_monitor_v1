use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct RegisterUserRequest {
    pub email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub profile_picture: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct UserRegisteredResponse {
    pub id: String,
    pub status: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct UserDetailsResponse {
    pub id: String,
    pub clerk_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}
