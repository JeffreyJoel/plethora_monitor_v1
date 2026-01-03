//! # User Models
//!
//! Request and response types for user API endpoints.
//! Defines the data structures for user registration, profile updates, and user details.
//!
//! ## User Data Structure
//!
//! - **Clerk ID**: Primary identifier from Clerk authentication service
//! - **Email**: User's email address (optional, from Clerk)
//! - **Profile**: Additional user metadata (stored in database)

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
