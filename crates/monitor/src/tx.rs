//! # Transactions Monitor
//! This module provides tools for monitoring transactions.
//!
//! It defines the `TransactionMonitor` trait, which allows for the detection of specific
//! transaction calls to targeted functions on the blockchain. The monitoring process
//! involves polling the blockchain for new blocks, fetching full transaction data, and
//! analyzing the `input` field of each transaction to match function selectors.

use crate::PollingMonitor;
use alloy::consensus::Transaction as TransactionTrait;
use alloy::hex;
use alloy::network::{AnyRpcTransaction, TransactionResponse};
use alloy::primitives::address;
use alloy::providers::Provider;
use alloy::rpc::types::BlockTransactions;
use std::time::Duration;
use tokio::time::sleep;

#[allow(async_fn_in_trait)]
pub trait TransactionMonitor {
    async fn monitor_transactions_polling<F>(
        self,
        function_names: Vec<String>,
        handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(AnyRpcTransaction) + Send + 'static;
}

impl TransactionMonitor for PollingMonitor {
    async fn monitor_transactions_polling<F>(
        self,
        function_names: Vec<String>,
        mut handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(AnyRpcTransaction) + Send + 'static,
    {
        println!(
            "TxMonitor: Watching transactions for {:?}",
            self.contract_address
        );

        // This converts the functiion names into their corresponding selectors using the contract ABI.
        let mut selectors: Vec<Vec<u8>> = Vec::new();

        for func_name in function_names {
            if let Some(funcs) = self.contract_abi.functions.get(&func_name) {
                if let Some(func) = funcs.first() {
                    let selector = func.selector().to_vec();
                    println!(
                        "  Watching Function: '{}' -> 0x{}",
                        func_name,
                        hex::encode(&selector)
                    );
                    selectors.push(selector);
                }
            } else {
                eprintln!("Warning: Function '{}' not found in ABI.", func_name);
            }
        }

        let current_block = self.provider.get_block_number().await?;

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
                                // temporary tx filter, TODO: create a separate file for filters
                                if tx.from()
                                    == address!("0xddb342ecc94236c29a5307d3757d0724d759453c")
                                {
                                    for selector in &selectors {
                                        if tx.input().starts_with(selector) {
                                            handler(tx.clone());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
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
