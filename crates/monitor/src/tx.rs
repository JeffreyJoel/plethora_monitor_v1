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
use alloy::providers::Provider;
use alloy::rpc::types::{BlockTransactions, Transaction};
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
        F: FnMut(Transaction) + Send + 'static;
}

impl TransactionMonitor for PollingMonitor {
    async fn monitor_transactions_polling<F>(
        self,
        function_names: Vec<String>,
        mut handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(Transaction) + Send + 'static,
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

        // We check if we have a saved state, otherwise we start from the current block.
        let latest_on_chain = self.provider.get_block_number().await?;
        let mut current_block = if let Some(saved_block) = self.load_state().await {
            println!("Resuming Tx Monitor from block: {}", saved_block);
            saved_block
        } else {
            println!("Starting Tx Monitor from head: {}", latest_on_chain);
            latest_on_chain
        };

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
                match self.provider.get_block_by_number(target_block.into()).await {
                    Ok(Some(block)) => {
                        // Alloy returns BlockTransactions enum: either Hashes(Vec<B256>) or Full(Vec<Transaction>)
                        if let BlockTransactions::Full(txs) = block.transactions {
                            for tx in txs {
                                // Filter A: Is it sent TO our contract?
                                if tx.inner.to() == Some(self.contract_address) {
                                    // Filter B: Does input data match a function selector?
                                    for selector in &selectors {
                                        // `tx.input` is a `Bytes` object, which supports `starts_with`
                                        if tx.inner.input().starts_with(selector) {
                                            handler(tx.clone());
                                            break; // Found a match, stop checking other selectors
                                        }
                                    }
                                }
                            }
                        }

                        // Update state after processing the block
                        current_block = target_block;
                        if let Err(e) = self.save_state(current_block).await {
                            eprintln!("Error saving state: {}", e);
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
