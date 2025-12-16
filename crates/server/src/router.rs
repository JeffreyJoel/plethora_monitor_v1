use crate::state::AppState;
use crate::{middleware, monitors, users};
use axum::Router;
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    // group domain routes
    let api_routes = Router::new()
        .nest("/monitors", monitors::routes::routes())
        .nest("/users", users::routes::routes());
    // add clerk middleware

    Router::new()
        .nest("/api", api_routes)
        .layer(middleware::auth_layer(state.clerk.clone()))
        .with_state(state)
}
