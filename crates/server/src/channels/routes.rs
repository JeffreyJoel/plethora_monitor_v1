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
