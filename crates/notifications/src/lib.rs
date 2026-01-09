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
//! - **Discord** - Discord webhook integration
//!
//! Planned:
//! - **Webhook** - HTTP POST to custom endpoints
//! - **Slack** - Slack webhook integration
//!

use crate::primitives::models::{
    Alert, NotificationChannel, NotificationChannelType, NotificationDestination, TelegramConfig,
};
use serde_json;
use std::fmt;
use utils::crypto;

pub mod discord;
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
                // for email, the 'value' field in the DB is the email string
                Some(NotificationDestination::Email(self.value.clone()))
            }
            NotificationChannelType::Telegram => {
                let json_string = crypto::decrypt(&self.value).ok()?;
                let config: TelegramConfig = serde_json::from_str(&json_string).ok()?;
                // for telegram, the 'value' field in the DB is a json string of the config struct
                //like this: "{\"token\": \"12345:AbCdEf\", \"chat_id\": \"987654321\"}"
                Some(NotificationDestination::Telegram(config))
            }
            NotificationChannelType::Discord => {
                let webhook_url = crypto::decrypt(&self.value).ok()?;
                // for discord, the 'value' field in the DB is the webhook url string
                Some(NotificationDestination::Discord(webhook_url))
            }

            _ => None,
        }
    }
}

impl fmt::Display for NotificationChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationChannelType::Email => write!(f, "email"),
            NotificationChannelType::Telegram => write!(f, "telegram"),
            NotificationChannelType::Discord => write!(f, "discord"),
            NotificationChannelType::Slack => write!(f, "slack"),
            NotificationChannelType::Webhook => write!(f, "webhook"),
        }
    }
}

impl TryFrom<String> for NotificationChannelType {
    type Error = String;
    
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "email" => Ok(NotificationChannelType::Email),
            "telegram" => Ok(NotificationChannelType::Telegram),
            "discord" => Ok(NotificationChannelType::Discord),
            "slack" => Ok(NotificationChannelType::Slack),
            "webhook" => Ok(NotificationChannelType::Webhook),
            _ => Err(format!("Invalid notification channel type: {}", s))
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
        NotificationDestination::Discord(webhook_url) => {
            let full_msg = format!("**{}**\n\n{}", alert.subject, alert.message);
            discord::send_discord_webhook(webhook_url, &full_msg).await
        }
    }
}
