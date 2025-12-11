//! # Transactions Monitor
//! This module provides tools for monitoring transactions.
//!
//! It defines the `TransactionMonitor` trait, which allows for the detection of specific
//! transaction calls to targeted functions on the blockchain. The monitoring process
//! involves polling the blockchain for new blocks, fetching full transaction data, and
//! analyzing the `input` field of each transaction to match function selectors.

use crate::PollingMonitor;
use crate::primitives::models::{Condition, MonitorRule};
use alloy::json_abi::JsonAbi;
use alloy::network::AnyRpcTransaction;
use alloy::providers::Provider;
use alloy::rpc::types::BlockTransactions;
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
        println!(
            "TxMonitor: Watching transactions for {:?}",
            self.contract_address
        );

        let mut current_block = self.provider.get_block_number().await?;

        loop {
            let latest_block = match self.provider.get_block_number().await {
                Ok(num) => num,
                Err(e) => {
                    eprintln!("Error fetching latest block number: {}", e);
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            while current_block < latest_block {
                let target_block = current_block + 1;

                // We request the block by number to get all the transactio details in that block
                match self
                    .provider
                    .get_block_by_number(target_block.into())
                    .full()
                    .await
                {
                    Ok(Some(block)) => {
                        // Alloy returns BlockTransactions enum: either Hashes(Vec<B256>) or Full(Vec<Transaction>)
                        if let BlockTransactions::Full(txs) = &block.transactions {
                            for tx in txs {
                                for rule in &rules {
                                    if rule.tx_match(tx) {
                                        println!("Match found for rule: {}", rule.name);
                                        handler(tx.clone());
                                        break;
                                    }
                                }
                            }
                        }
                        current_block = target_block;
                    }
                    Ok(None) => {
                        // The block number exists (latest_block) but the block data isn't available yet.
                        // This happens due to eventual consistency in nodes. Wait briefly.
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Error fetching block {}: {}", target_block, e);
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    }
}

/// Helper functions

// this takes in a list of rules, and for every condition in the rule, we get the function to listen for
// and then map it to the abi definition
pub fn map_rules_to_abi(mut rules: Vec<MonitorRule>, abi: &JsonAbi) -> Vec<MonitorRule> {
    for rule in &mut rules {
        // Find which function this rule targets
        let target_name = rule.conditions.iter().find_map(|c| {
            if let Condition::Function(name) = c {
                Some(name.clone())
            } else {
                None
            }
        });

        // map it to the first instance of function in the ABI
        // this is done incase there are multiple instances of the same function in the abi (function overloading)
        if let Some(name) = target_name {
            if let Some(func) = abi.function(&name).and_then(|f| f.first()) {
                rule.abi_function = Some(func.clone());
            }
        }
    }
    rules
}
