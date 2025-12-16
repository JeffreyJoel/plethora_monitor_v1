use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Email,
    Discord,
    Slack,
    Webhook,
}

// implement Display for easy DB storage
impl ToString for ChannelType {
    fn to_string(&self) -> String {
        match self {
            ChannelType::Email => "Email".to_string(),
            ChannelType::Discord => "Discord".to_string(),
            ChannelType::Slack => "Slack".to_string(),
            ChannelType::Webhook => "Webhook".to_string(),
        }
    }
}

// convert DB string back to Enum
impl From<String> for ChannelType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Email" => ChannelType::Email,
            "Discord" => ChannelType::Discord,
            "Slack" => ChannelType::Slack,
            "Webhook" => ChannelType::Webhook,
            _ => ChannelType::Email,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: Option<Uuid>,
    #[serde(rename = "type")]
    pub type_: ChannelType,
    pub label: String,
    pub value: String,
}
