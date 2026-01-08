//! # Notifications Crate
//!
//! Handles delivery of alerts and notifications to various channels.
//! Provides a unified interface for sending notifications regardless of the
//! delivery mechanism (email, webhook, Discord, Slack, etc.).
//!
//! ## Architecture
//!
//! The notification system uses a trait-based design:
//!
//! - **`ToDestination`** - Trait for converting channel configurations to destinations
//! - **`NotificationDestination`** - Enum representing different delivery methods
//! - **`Alert`** - Structured alert data (source, subject, message)
//!
//! ## Supported Channels
//!
//! Currently implemented:
//! - **Email** - SMTP-based email delivery
//! - **Telegram**
//!
//! Planned:
//! - **Webhook** - HTTP POST to custom endpoints
//! - **Discord** - Discord webhook integration
//! - **Slack** - Slack webhook integration
//!
//! ## Usage
//!
//! ```no_run
//! use notifications::{send_notification, primitives::models::{Alert, NotificationDestination}};
//!
//! let alert = Alert {
//!     source: "Monitor Name".to_string(),
//!     subject: "Transaction Alert".to_string(),
//!     message: "A matching transaction was detected".to_string(),
//! };
//!
//! let destination = NotificationDestination::Email("user@example.com".to_string());
//! send_notification(&destination, &alert).await?;
//! ```

use crate::primitives::models::{
    Alert, NotificationChannel, NotificationChannelType, NotificationDestination, TelegramConfig,
};
use serde_json;
use std::fmt;

pub mod email;
pub mod primitives;
pub mod telegram;

pub trait ToDestination {
    fn to_destination(&self) -> Option<NotificationDestination>;
}

impl ToDestination for NotificationChannel {
    fn to_destination(&self) -> Option<NotificationDestination> {
        match self.type_ {
            NotificationChannelType::Email => {
                Some(NotificationDestination::Email(self.value.clone()))
            }
            NotificationChannelType::Telegram => {
                let config: TelegramConfig = serde_json::from_str(&self.value).ok()?;
                Some(NotificationDestination::Telegram(config))
            }
            _ => None,
        }
    }
}

impl fmt::Display for NotificationChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationChannelType::Email => write!(f, "Email"),
            NotificationChannelType::Telegram => write!(f, "Telegram"),
            NotificationChannelType::Discord => write!(f, "Discord"),
            NotificationChannelType::Slack => write!(f, "Slack"),
            NotificationChannelType::Webhook => write!(f, "Webhook"),
        }
    }
}

impl From<String> for NotificationChannelType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Email" => NotificationChannelType::Email,
            "Discord" => NotificationChannelType::Discord,
            "Slack" => NotificationChannelType::Slack,
            "Webhook" => NotificationChannelType::Webhook,
            _ => NotificationChannelType::Email,
        }
    }
}

pub async fn send_notification(
    dest: &NotificationDestination,
    alert: &Alert,
) -> Result<(), anyhow::Error> {
    match dest {
        NotificationDestination::Email(recipient) => {
            email::send_email(recipient, &alert.subject, &alert.message).await
        }
        NotificationDestination::Telegram(config) => {
            let full_msg = format!("*{}*\n\n{}", alert.subject, alert.message);
            telegram::send_telegram_message(&config.token, &config.chat_id, &full_msg).await
        }
    }
}
