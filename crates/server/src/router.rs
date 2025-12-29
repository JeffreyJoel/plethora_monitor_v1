use crate::state::AppState;
use crate::{channels, middleware, monitors, users};
use crate::docs::ApiDoc;
use axum::http::{Method, header};
use axum::{Router, http};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use tower_http::cors::{AllowOrigin, CorsLayer};

pub fn create_router(state: Arc<AppState>) -> Router {
    // group domain routes
    let api_routes = Router::new()
        .nest("/monitors", monitors::routes::routes())
        .nest("/users", users::routes::routes())
        .nest("/channels", channels::routes::routes());

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _request_head| {
            origin.as_bytes().ends_with(b".vercel.app") 
                || origin.as_bytes().starts_with(b"http://localhost")
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true);

    let openapi = ApiDoc::openapi();

    let protected_routes = Router::new()
    .nest("/api", api_routes)
    .layer(middleware::auth_layer(state.clerk.clone()));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .merge(protected_routes)
        .layer(cors)
        .with_state(state)
}
