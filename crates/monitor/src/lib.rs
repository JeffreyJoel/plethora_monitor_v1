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
use std::time::Duration;
use tokio::time::sleep;

/// A type alias for the Ethereum HTTP provider.
type HttpProvider = RootProvider<Ethereum>;

pub struct PollingMonitor {
    provider: HttpProvider,
    contract_address: Address,
    contract_abi: JsonAbi,
}

impl PollingMonitor {
    /// Creates a new `PollingMonitor`.
    pub fn new(
        rpc_url: &str,
        contract_address: Address,
        contract_abi: JsonAbi,
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
        })
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
        let mut current_block = self.provider.get_block_number().await?;
        println!("  Starting from block: {}", current_block);

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

                println!("  Fetching logs from {} to {}", from_block, to_block);

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
                    }
                    Err(e) => eprintln!("Error fetching logs: {}", e),
                }
            }

            // Wait for a short duration before the next poll.
            sleep(Duration::from_secs(2)).await;
        }
    }
}
