use alloy::json_abi::Function;
use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    Eq,
    Gt,
    Lt,
    Contains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    From(Address),
    To(Address),
    Function(String),
    Argument {
        name: String,
        operator: Operator,
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorRule {
    pub name: String,
    pub conditions: Vec<Condition>,

    #[serde(skip)] //skip this, because we are not fetching the abi function from the toml
    pub abi_function: Option<Function>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub rpc_url: String,
    pub chain: String,
    pub address: Address,
    pub events: Option<Vec<String>>,
    pub functions: Option<Vec<MonitorRule>>,
    pub notification_channel_id: Option<Uuid>,
    // pub email_recipient: Option<String>,
}
