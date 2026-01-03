//! # Channel Models
//!
//! Request and response types for notification channel API endpoints.
//! Defines data structures for creating, updating, and retrieving channel configurations.
//!
//! ## Channel Types
//!
//! - **Email**: SMTP-based email notifications
//! - **Webhook**: HTTP POST to custom endpoints (planned)
//! - **Discord**: Discord webhook integration (planned)
//! - **Slack**: Slack webhook integration (planned)

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChannelRequest {
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChannelResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateChannelRequest {
    pub label: Option<String>,
    pub value: Option<String>,
}
