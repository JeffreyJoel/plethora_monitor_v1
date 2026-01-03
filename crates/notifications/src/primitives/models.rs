//! # Notification Primitives - Models
//!
//! Core data structures for the notification system.
//! Defines alert types, notification destinations, and channel configurations.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelType {
    Email,
    Discord,
    Slack,
    Webhook,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub source: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum NotificationDestination {
    Email(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: Option<Uuid>,
    #[serde(rename = "type")]
    pub type_: NotificationChannelType,
    pub label: String,
    pub value: String,
}
