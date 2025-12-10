use alloy::consensus::Transaction;
use alloy::dyn_abi::{DynSolValue, JsonAbiExt};
use alloy::json_abi::Function;
use alloy::network::{AnyRpcTransaction, TransactionResponse};
use alloy::primitives::{Address, U256};
use serde::Deserialize;
use std::str::FromStr;

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

impl MonitorRule {
    pub fn tx_match(&self, tx: &AnyRpcTransaction) -> bool {
        for condition in &self.conditions {
            match condition {
                Condition::From(expected) => {
                    if tx.from() != *expected {
                        return false;
                    }
                }
                Condition::To(expected) => {
                    if let Ok(target_address) = Address::from_str(expected) {
                        if tx.to() != Some(target_address) {
                            return false;
                        }
                    } else {
                        eprintln!("Invalid address in rule: {}", expected);
                        return false;
                    }
                }

                Condition::Function(expected) => {
                    if let Some(func_abi) = &self.abi_function {
                        if func_abi.name != *expected {
                            return false;
                        }

                        let selector = func_abi.selector();
                        if !tx.input().starts_with(selector.as_slice()) {
                            return false;
                        }
                    }
                }
                Condition::Argument {
                    name,
                    operator,
                    value,
                } => {
                    let func_abi = match &self.abi_function {
                        Some(f) => f,
                        None => return false,
                    };

                    //we ensure that we are filtering non-function interactions such as eth-transfer
                    if tx.input().len() < 4 {
                        return false;
                    }

                    // here we are slicing away the function selector(first 4bytes)
                    // to ensure we only get the actual tx parameters
                    let input_data = &tx.input()[4..];

                    if let Ok(decoded_input) = func_abi.abi_decode_input(input_data) {
                        //this gets the index of an argument based on the name of the argument provided
                        // e.g. if we need to filter based on the amount arg, it tells us what
                        // index in the tx amount is at
                        let arg_index = func_abi.inputs.iter().position(|i| i.name == *name);

                        match arg_index {
                            Some(idx) => {
                                let actual_value = &decoded_input[idx];
                                if !check_value(actual_value, operator, value) {
                                    return false;
                                }
                            }
                            None => return false,
                        }
                    } else {
                        return false;
                    }
                }
            }
        }
        return true; // this means all the conditions matched
    }
}

pub fn check_value(actual_value: &DynSolValue, operator: &Operator, value: &str) -> bool {
    match actual_value {
        //numeric check
        DynSolValue::Uint(n, _) => {
            if let Ok(target_num) = U256::from_str(value) {
                match operator {
                    Operator::Gt => *n > target_num,
                    Operator::Lt => *n < target_num,
                    Operator::Eq => *n == target_num,

                    _ => false,
                }
            } else {
                false
            }
        }

        // address check
        DynSolValue::Address(addr) => {
            if let Ok(target_addr) = Address::from_str(value) {
                match operator {
                    Operator::Contains => *addr == target_addr,

                    _ => false,
                }
            } else {
                false
            }
        }

        _ => false,
    }
}
