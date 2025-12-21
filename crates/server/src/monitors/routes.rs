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
