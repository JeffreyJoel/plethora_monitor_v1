use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterUserRequest {
    pub email: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub profile_picture: Option<String>,
}

#[derive(Serialize)]
pub struct UserRegisteredResponse {
    pub id: String,
    pub status: String,
}

#[derive(serde::Serialize)]
pub struct UserDetailsResponse {
    pub id: String,
    pub clerk_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}
