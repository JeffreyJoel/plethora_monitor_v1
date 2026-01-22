//! # Billing Routes
//!
//! Route definitions for billing API endpoints.

use super::handlers;
use crate::state::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/subscription", get(handlers::get_subscription))
        .route("/plans", get(handlers::get_plans))
        .route("/usage", get(handlers::get_usage))
}

