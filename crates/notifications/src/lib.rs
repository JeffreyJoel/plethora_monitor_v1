use crate::primitives::models::{
    Alert, NotificationChannel, NotificationChannelType, NotificationDestination,
};
use std::fmt;

pub mod email;
pub mod primitives;

pub async fn send_notification(
    dest: &NotificationDestination,
    alert: &Alert,
) -> Result<(), anyhow::Error> {
    match dest {
        NotificationDestination::Email(recipient) => {
            email::send_email(recipient, &alert.subject, &alert.message).await
        }
    }
}

impl fmt::Display for NotificationChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationChannelType::Email => write!(f, "Email"),
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

pub trait ToDestination {
    fn to_destination(&self) -> Option<NotificationDestination>;
}

impl ToDestination for NotificationChannel {
    fn to_destination(&self) -> Option<NotificationDestination> {
        match self.type_ {
            NotificationChannelType::Email => {
                Some(NotificationDestination::Email(self.value.clone()))
            }
            _ => None,
        }
    }
}
