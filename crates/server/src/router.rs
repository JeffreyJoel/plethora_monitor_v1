use crate::state::AppState;
use crate::{middleware, monitors, users};
use axum::http::{Method, header};
use axum::{Router, http};
use std::sync::Arc;

use tower_http::cors::CorsLayer;

pub fn create_router(state: Arc<AppState>) -> Router {
    // group domain routes
    let api_routes = Router::new()
        .nest("/monitors", monitors::routes::routes())
        .nest("/users", users::routes::routes());

    let cors = CorsLayer::new()
        .allow_origin(
            "http://localhost:3000"
                .parse::<http::HeaderValue>()
                .unwrap(),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .nest("/api", api_routes)
        .layer(middleware::auth_layer(state.clerk.clone()))
        .layer(cors)
        .with_state(state)
}
