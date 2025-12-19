use alloy::json_abi::{Event, Function};
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
    #[serde(skip)]
    pub abi_function: Option<Function>,
    #[serde(skip)]
    pub abi_event: Option<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub rpc_url: String,
    pub chain: String,
    pub address: Address,
    pub event_rules: Option<Vec<MonitorRule>>,
    pub function_rules: Option<Vec<MonitorRule>>,
    pub notification_channel_id: Option<Uuid>,
}
