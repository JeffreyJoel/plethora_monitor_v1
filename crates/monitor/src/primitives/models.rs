use alloy::json_abi::Function;
use alloy::primitives::Address;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub enum Operator {
    Eq,
    Gt,
    Lt,
    Contains,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Condition {
    From(Address),
    To(String),
    Function(String),
    Argument {
        name: String,
        operator: Operator,
        value: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonitorRule {
    pub name: String,
    pub conditions: Vec<Condition>,

    #[serde(skip)] //skip this, because we are not fetching the abi function from the toml
    pub abi_function: Option<Function>,
}
