//! # User Routes
//!
//! HTTP route definitions for user endpoints.
//! Maps URL paths to handler functions for user registration and profile management.
//!
//! ## Endpoints
//!
//! - `POST /api/users/register` - Register a new user (syncs with Clerk)
//! - `GET /api/users/me` - Get current user's profile information
//! - `PATCH /api/users/profile` - Update user profile

use super::handlers;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, patch, post},
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(handlers::register_user))
        .route("/profile", patch(handlers::update_profile))
        .route("/me", get(handlers::get_me))
}
