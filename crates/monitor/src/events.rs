//! # Transactions Monitor
//! This module provides tools for monitoring events.
//!
//! It defines the `EventMonitor` trait, which enables the detection and handling of specific
//! events emitted by smart contracts on the blockchain. The monitoring process involves polling
//! the blockchain for new blocks, filtering logs based on event signatures, and invoking user-defined
//! handlers for each detected event.

use crate::PollingMonitor;
use crate::primitives::utils::format_value;
use alloy::dyn_abi::EventExt;
use alloy::json_abi::JsonAbi;
use alloy::primitives::B256;
use alloy::providers::Provider;
use alloy::rpc::types::{Filter, Log};
use std::time::Duration;
use tokio::time::sleep;

#[allow(async_fn_in_trait)]
pub trait EventMonitor {
    async fn monitor_events_polling<F>(
        self,
        event_names: &[&str],
        handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(Log) + Send + 'static;
}

impl EventMonitor for PollingMonitor {
    async fn monitor_events_polling<F>(
        self,
        event_names: &[&str],
        mut handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(Log) + Send + 'static,
    {
        println!(
            "EventsMonitor: Watching transactions for {:?}",
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
        let current_block = self.provider.get_block_number().await?;

        loop {
            let latest_block = match self.provider.get_block_number().await {
                Ok(num) => num,
                Err(e) => {
                    eprintln!("Error fetching block number: {}", e);
                    // wait before retrying to avoid spamming the node on failure.
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            if latest_block > current_block {
                let from_block = current_block + 1;
                let to_block = latest_block;

                // this builds a filter to query for logs in the specified block range.
                let filter = Filter::new()
                    .address(self.contract_address)
                    .event_signature(topics.clone())
                    .from_block(from_block)
                    .to_block(to_block);

                // this fetches the logs from the provider.
                match self.provider.get_logs(&filter).await {
                    Ok(logs) => {
                        for log in logs {
                            handler(log);
                        }
                    }
                    Err(e) => eprintln!("Error fetching logs: {}", e),
                }
            }

            // wait for a short duration before the next poll.
            sleep(Duration::from_secs(2)).await;
        }
    }
}

pub fn get_event_details(log: &Log, abi: &JsonAbi) -> String {
    let topics = log.topics();
    if topics.is_empty() {
        return "Event: Anonymous/Malformed (No topics)".to_string();
    }

    let selector = topics[0];

    let event = abi.events().find(|e| e.selector() == selector);

    if let Some(e) = event {
        match e.decode_log(&log.data()) {
            Ok(decoded) => {
                let mut output = format!("Event: {}\n", e.name);

                let mut indexed_iter = decoded.indexed.into_iter();
                let mut body_iter = decoded.body.into_iter();

                for input in &e.inputs {
                    let val = if input.indexed {
                        indexed_iter.next()
                    } else {
                        body_iter.next()
                    };

                    if let Some(v) = val {
                        output.push_str(&format!("- {}: {}\n", input.name, format_value(&v)));
                    }
                }
                output
            }
            Err(err) => format!("Event: {} (Decode Error: {})", e.name, err),
        }
    } else {
        format!("Unknown Event (Signature: {})", selector)
    }
}
