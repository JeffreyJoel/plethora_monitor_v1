//! # Router Module
//!
//! Configures the HTTP router and middleware for the API server.
//! Sets up routing, CORS, authentication, and API documentation.
//!
//! ## Route Structure
//!
//! - `/api/monitors/*` - Monitor management endpoints
//! - `/api/users/*` - User management endpoints
//! - `/api/channels/*` - Notification channel endpoints
//! - `/swagger-ui` - Interactive API documentation
//! - `/api-docs/openapi.json` - OpenAPI specification
//!
//! ## Middleware Stack
//!
//! 1. **CORS Layer**: Allows requests from Vercel deployments and localhost
//! 2. **Authentication Layer**: Validates JWT tokens via Clerk
//! 3. **State Layer**: Provides application state to handlers
//!
//! ## Security
//!
//! All `/api/*` routes are protected by authentication middleware.
//! Only the Swagger UI and OpenAPI spec endpoints are publicly accessible.

use crate::state::AppState;
use crate::{channels, middleware, monitors, users};
use crate::docs::ApiDoc;
use axum::http::{Method, header};
use axum::{Router};
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

    // Public routes
    let public_routes = Router::new()
        .route("/channels/verify", axum::routing::get(channels::handlers::verify_email_channel));

    let protected_routes = Router::new()
    .nest("/api", api_routes)
    .layer(middleware::auth_layer(state.clerk.clone()));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors)
        .with_state(state)
}
