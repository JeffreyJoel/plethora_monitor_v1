use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub label: Option<String>,
    pub value: Option<String>,
}
