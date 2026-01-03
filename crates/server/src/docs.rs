//! # API Documentation Module
//!
//! Generates OpenAPI/Swagger documentation for the REST API.
//! Uses `utoipa` to automatically generate API documentation from
//! handler function signatures and request/response types.
//!
//! ## Documentation Features
//!
//! - **Interactive Swagger UI**: Available at `/swagger-ui`
//! - **OpenAPI Spec**: JSON schema at `/api-docs/openapi.json`
//! - **JWT Authentication**: Documents Bearer token authentication
//! - **Request/Response Schemas**: Auto-generated from Rust types
//!
//! ## Endpoints Documented
//!
//! All API endpoints are automatically included:
//! - Monitor management (create, read, update, delete, list)
//! - User management (register, profile operations)
//! - Channel management (CRUD operations)
//!
//! ## Security
//!
//! The documentation includes JWT Bearer token authentication,
//! allowing users to test authenticated endpoints directly from Swagger UI.

use utoipa::OpenApi;
use crate::{monitors, users, channels};
use crate::monitors::models::CreateMonitorResponse;
use crate::monitors::models::MonitorResponse;
use crate::users::models::{RegisterUserRequest, UpdateUserRequest, UserDetailsResponse, UserRegisteredResponse};
use crate::channels::models::{CreateChannelRequest, ChannelResponse, UpdateChannelRequest};
use monitor::primitives::models::{MonitorConfig, MonitorRule, Condition, Operator};

#[derive(OpenApi)]
#[openapi(
    paths(
        monitors::handlers::create_monitor,
        monitors::handlers::get_monitors,
        monitors::handlers::get_monitor,
        monitors::handlers::update_monitor,
        monitors::handlers::delete_monitor,
        users::handlers::register_user,
        users::handlers::update_profile,
        users::handlers::get_me,
        channels::handlers::create_channel,
        channels::handlers::get_channels,
        channels::handlers::get_channel,
        channels::handlers::update_channel,
        channels::handlers::delete_channel
    ),
    components(
        schemas(
            MonitorConfig,
            MonitorRule,
            Condition,
            Operator,
            CreateMonitorResponse,
            MonitorResponse,
            RegisterUserRequest,
            UpdateUserRequest,
            UserDetailsResponse,
            UserRegisteredResponse,
            CreateChannelRequest,
            ChannelResponse,
            UpdateChannelRequest
        )
    ),
    tags(
        (name = "monitors", description = "Monitor management endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "channels", description = "Notification channel management endpoints")
    ),
    modifiers(&SecurityAddon),
    security(
        ("jwt" = [])
    )
)]
pub struct ApiDoc;

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::Modify;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "jwt",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

