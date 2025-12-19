//! Transaction monitoring module.

use crate::PollingMonitor;
use crate::primitives::models::MonitorRule;
use crate::primitives::utils::format_value;
use alloy::consensus::Transaction;
use alloy::dyn_abi::JsonAbiExt;
use alloy::hex;
use alloy::json_abi::JsonAbi;
use alloy::network::{AnyRpcTransaction, TransactionResponse};
use alloy::providers::Provider;
use alloy::rpc::types::BlockId;
use std::time::Duration;
use tokio::time::sleep;

#[allow(async_fn_in_trait)]
pub trait TransactionMonitor {
    async fn monitor_transactions_polling<F>(
        self,
        rules: Vec<MonitorRule>,
        handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(AnyRpcTransaction) + Send + 'static;
}

impl TransactionMonitor for PollingMonitor {
    async fn monitor_transactions_polling<F>(
        self,
        rules: Vec<MonitorRule>,
        mut handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(AnyRpcTransaction) + Send + 'static,
    {
        tracing::info!(
            "TxMonitor: Started monitoring transactions for contract {:?}",
            self.contract_address
        );

        let mut current_block = self.provider.get_block_number().await?;

        loop {
            let latest_block = match self.provider.get_block_number().await {
                Ok(num) => num,
                Err(e) => {
                    tracing::error!("Error fetching latest block number: {}", e);
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            while current_block < latest_block {
                let target_block = current_block + 1;

                match self
                    .provider
                    .get_block_receipts(BlockId::Number(target_block.into()))
                    .await
                {
                    Ok(Some(receipts)) => {
                        for receipt in receipts {
                            if receipt.to != Some(self.contract_address) {
                                continue;
                            }

                            match self
                                .provider
                                .get_transaction_by_hash(receipt.transaction_hash)
                                .await
                            {
                                Ok(Some(tx)) => {
                                    use crate::tx::TxMatcher;

                                    for rule in &rules {
                                        if rule.tx_match(&tx) {
                                            tracing::info!(
                                                "Match found: Rule '{}' triggered by tx {:?}",
                                                rule.name,
                                                tx.tx_hash()
                                            );
                                            handler(tx);
                                            break;
                                        }
                                    }
                                }
                                Ok(None) => {
                                    tracing::warn!(
                                        "Transaction {:?} not found",
                                        receipt.transaction_hash
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Error fetching transaction {:?}: {}",
                                        receipt.transaction_hash,
                                        e
                                    );
                                }
                            }
                        }
                        current_block = target_block;
                    }
                    Ok(None) => {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error fetching receipts for block {}: {}",
                            target_block,
                            e
                        );
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    }
}

/// Maps function monitor rules to their corresponding ABI function definitions.
pub fn map_rules_to_abi(mut rules: Vec<MonitorRule>, abi: &JsonAbi) -> Vec<MonitorRule> {
    for rule in &mut rules {
        if let Some(func) = abi.function(&rule.name).and_then(|f| f.first()) {
            rule.abi_function = Some(func.clone());
        } else {
            eprintln!("Warning: Rule '{}' not found in contract ABI", rule.name);
        }
    }
    rules
}

/// Decodes transaction input data using the ABI and returns a formatted string for notifications.
pub fn get_tx_details(tx: &AnyRpcTransaction, abi: &JsonAbi) -> String {
    let input = tx.input();

    if input.len() < 4 {
        return "Transaction has no function data".to_string();
    }

    let selector = &input[0..4];
    let data = &input[4..];

    let func = abi
        .functions()
        .find(|f| f.selector().as_slice() == selector);

    if let Some(f) = func {
        match f.abi_decode_input(data) {
            Ok(decoded_inputs) => {
                let mut output = format!("Function: {}\n", f.name);

                for (i, input_def) in f.inputs.iter().enumerate() {
                    let val = decoded_inputs.get(i).unwrap();
                    output.push_str(&format!("- {}: {}\n", input_def.name, format_value(val)));
                }
                output
            }
            Err(e) => format!("Function: {} (Decode Error: {})", f.name, e),
        }
    } else {
        format!("Unknown Function (Selector: {})", hex::encode(selector))
    }
}

/// Trait for matching transactions against rules
pub trait TxMatcher {
    fn tx_match(&self, tx: &AnyRpcTransaction) -> bool;
}

/// Implementation of TxMatcher for MonitorRule
impl TxMatcher for crate::primitives::models::MonitorRule {
    fn tx_match(&self, tx: &AnyRpcTransaction) -> bool {
        use crate::primitives::models::Condition;
        use crate::primitives::utils::check_argument_condition;

        let func = match &self.abi_function {
            Some(f) => f,
            None => return false,
        };

        let input = tx.input();
        if input.len() < 4 {
            return false;
        }

        if &input[0..4] != func.selector().as_slice() {
            return false;
        }

        let decoded = match func.abi_decode_input(&input[4..]) {
            Ok(d) => d,
            Err(_) => return false,
        };

        for condition in &self.conditions {
            match condition {
                Condition::From(addr) => {
                    if tx.from() != *addr {
                        return false;
                    }
                }
                Condition::To(addr) => {
                    if tx.to() != Some(*addr) {
                        return false;
                    }
                }
                Condition::Function(_) => continue,
                Condition::Argument {
                    name,
                    operator,
                    value,
                } => {
                    let arg_index = func.inputs.iter().position(|p| &p.name == name);

                    if let Some(idx) = arg_index {
                        if let Some(arg_value) = decoded.get(idx) {
                            if !check_argument_condition(arg_value, operator, value) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        true
    }
}
