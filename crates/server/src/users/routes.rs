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
