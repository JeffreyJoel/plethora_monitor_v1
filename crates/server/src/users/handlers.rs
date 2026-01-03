//! # User Handlers
//!
//! HTTP request handlers for user management endpoints.
//! Handles user registration, profile retrieval, and profile updates.
//! Integrates with Clerk for authentication and maintains user records in the database.
//!
//! ## Authentication Flow
//!
//! 1. User authenticates via Clerk (external service)
//! 2. Client sends JWT token in `Authorization` header
//! 3. Server validates token using Clerk middleware
//! 4. User record is created/retrieved from database
//! 5. Request proceeds with authenticated user context

use super::models::{
    RegisterUserRequest, UpdateUserRequest, UserDetailsResponse, UserRegisteredResponse,
};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use clerk_rs::validators::authorizer::ClerkJwt;
use database::UserRepository;
use std::sync::Arc;

// POST /api/users/register
#[utoipa::path(
    post,
    path = "/api/users/register",
    request_body = RegisterUserRequest,
    responses(
        (status = 200, description = "User registered successfully", body = UserRegisteredResponse),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "users"
)]
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Json(payload): Json<RegisterUserRequest>,
) -> Result<Json<UserRegisteredResponse>, StatusCode> {
    let clerk_id = jwt.sub;

    // This creates the user if they don't exist, or returns their ID if they do.
    let user_id = state
        .db
        .users
        .get_or_create(&clerk_id, payload.email.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to sync user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(UserRegisteredResponse {
        id: user_id.to_string(),
        status: "synced".to_string(),
    }))
}

// PATCH /api/users/profile
#[utoipa::path(
    patch,
    path = "/api/users/profile",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Profile updated successfully", body = UserRegisteredResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "users"
)]
pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserRegisteredResponse>, StatusCode> {
    // We use get_or_create here just to get the ID safely,
    // to ensure that they exist before updating.
    let user_id = state
        .db
        .users
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // handle the update
    state
        .db
        .users
        .update_profile(
            user_id,
            payload.name.as_deref(),
            payload.profile_picture.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update profile: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(UserRegisteredResponse {
        id: user_id.to_string(),
        status: "updated".to_string(),
    }))
}

// GET /api/users/me
#[utoipa::path(
    get,
    path = "/api/users/me",
    responses(
        (status = 200, description = "User details", body = UserDetailsResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "users"
)]
pub async fn get_me(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<UserDetailsResponse>, StatusCode> {
    let repo = UserRepository::new(state.db.pool.clone());

    // get user id from clerk id
    let user_id = repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // fetch user details
    let details = repo
        .get_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some((clerk_id, email, name)) = details {
        Ok(Json(UserDetailsResponse {
            id: user_id.to_string(),
            clerk_id,
            email,
            name,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
