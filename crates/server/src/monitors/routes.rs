//! # Monitor Routes
//!
//! HTTP route definitions for monitor endpoints.
//! Maps URL paths to handler functions and HTTP methods.
//!
//! ## Endpoints
//!
//! - `POST /api/monitors` - Create a new monitor
//! - `GET /api/monitors` - List all monitors for the authenticated user
//! - `GET /api/monitors/:id` - Get a specific monitor configuration
//! - `PUT /api/monitors/:id` - Update an existing monitor
//! - `DELETE /api/monitors/:id` - Delete a monitor

use super::handlers;
use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(handlers::create_monitor))
        .route("/", get(handlers::get_monitors))
        .route("/{id}", get(handlers::get_monitor))
        .route("/{id}", put(handlers::update_monitor))
        .route("/{id}", delete(handlers::delete_monitor))
}
