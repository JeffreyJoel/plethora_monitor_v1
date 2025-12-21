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

