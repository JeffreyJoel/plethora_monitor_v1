//! # Channel Routes
//!
//! HTTP route definitions for notification channel endpoints.
//! Maps URL paths to handler functions for channel management operations.
//!
//! ## Endpoints
//!
//! - `POST /api/channels` - Create a new notification channel
//! - `GET /api/channels` - List all channels for the authenticated user
//! - `GET /api/channels/:id` - Get a specific channel configuration
//! - `PUT /api/channels/:id` - Update a channel configuration
//! - `DELETE /api/channels/:id` - Delete a channel

use crate::channels::handlers;
use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::create_channel))
        .route("/", get(handlers::get_channels))
        .route("/{id}", get(handlers::get_channel))
        .route("/{id}", put(handlers::update_channel))
        .route("/{id}", delete(handlers::delete_channel))
}
