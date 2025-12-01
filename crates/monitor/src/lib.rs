//! # Polling Monitor
//!
//! This module monitors a smart contract for events by periodically polling an Ethereum node.
//! It is simple and works with standard HTTP RPC endpoints. It keeps track of the
//! latest block it has processed and queries for new logs in the range from that block
//! to the current chain head

use alloy::json_abi::JsonAbi;
use alloy::network::Ethereum;
use alloy::primitives::{Address, B256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{Filter, Log};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::fs;
use tokio::time::sleep;

/// A type alias for the Ethereum HTTP provider.
type HttpProvider = RootProvider<Ethereum>;

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorState {
    last_processed_block: u64,
}

pub struct PollingMonitor {
    provider: HttpProvider,
    contract_address: Address,
    contract_abi: JsonAbi,
    state_file_path: String,
}

impl PollingMonitor {
    /// Creates a new `PollingMonitor`.
    pub fn new(
        rpc_url: &str,
        contract_address: Address,
        contract_abi: JsonAbi,
        state_file_path: &str,
    ) -> Result<Self, anyhow::Error> {
        // Set up the HTTP provider using the provided RPC URL.
        let url = rpc_url.parse()?;
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .connect_http(url);

        Ok(Self {
            provider,
            contract_address,
            contract_abi,
            state_file_path: state_file_path.to_string(),
        })
    }

    ///  helper function to load the state_file
    async fn load_state(&self) -> Option<u64> {
        let content = fs::read_to_string(&self.state_file_path).await.ok()?;

        let state: MonitorState = serde_json::from_str(&content).ok()?;

        Some(state.last_processed_block)
    }

    ///  helper function to write to the state_file
    async fn save_state(&self, block_number: u64) -> Result<(), anyhow::Error> {
        let state = MonitorState {
            last_processed_block: block_number,
        };
        let content = serde_json::to_string_pretty(&state)?;

        // We use tokio::fs write to file asynchronously
        fs::write(&self.state_file_path, content).await?;

        Ok(())
    }

    /// Starts the event monitoring loop.
    pub async fn monitor_events_polling<F>(
        self,
        event_names: &[&str],
        mut handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(Log) + Send + 'static,
    {
        println!(
            "PollingMonitor: Starting poll loop for {:?}",
            self.contract_address
        );

        // this prepares the event topics from the ABI for efficient filtering.
        let mut topics: Vec<B256> = Vec::new();
        for event_name in event_names {
            if let Some(events) = self.contract_abi.events.get(*event_name) {
                if let Some(event) = events.first() {
                    // The event selector is a hash of the event signature.
                    topics.push(event.selector());
                }
            }
        }

        // this initialises the starting block for polling to the current block number.
        let mut current_block = if let Some(saved_block) = self.load_state().await {
            println!(" returning from saved_block: {}", saved_block);
            saved_block
        } else {
            let head = self.provider.get_block_number().await?;
            println!("  Starting from block: {}", head);
            head
        };

        loop {
            // Get the latest block number from the chain.
            let latest_block = match self.provider.get_block_number().await {
                Ok(num) => num,
                Err(e) => {
                    eprintln!("Error fetching block number: {}", e);
                    // Wait before retrying to avoid spamming the node on failure.
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            // Check if there are new blocks to process.
            if latest_block > current_block {
                let from_block = current_block + 1;
                let to_block = latest_block;

                // Build a filter to query for logs in the specified block range.
                let filter = Filter::new()
                    .address(self.contract_address)
                    .event_signature(topics.clone())
                    .from_block(from_block)
                    .to_block(to_block);

                // Fetch the logs from the provider.
                match self.provider.get_logs(&filter).await {
                    Ok(logs) => {
                        for log in logs {
                            // Pass each log to the provided handler function.
                            handler(log);
                        }
                        // Update the current block number to the latest block processed.
                        current_block = latest_block;

                        if let Err(e) = self.save_state(current_block).await {
                            eprintln!("Error: failed to save state {}", e)
                        }
                    }
                    Err(e) => eprintln!("Error fetching logs: {}", e),
                }
            }

            // Wait for a short duration before the next poll.
            sleep(Duration::from_secs(2)).await;
        }
    }
}
