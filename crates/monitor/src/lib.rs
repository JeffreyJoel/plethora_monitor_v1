//! # Monitor Crate
//!
//! The core monitoring engine for EVM blockchain monitoring.
//! This crate provides the low-level functionality for tracking smart contract
//! transactions and events in real-time.
//!
//! ## Core Components
//!
//! - **`PollingMonitor`** - The main monitoring engine that maintains RPC connections
//!   and orchestrates transaction and event monitoring
//! - **`TransactionMonitor`** - Scans blocks for transactions matching user-defined rules
//! - **`EventMonitor`** - Monitors contract events/logs based on ABI signatures
//! - **`primitives`** - Shared data structures and utilities (models, ABI fetching, etc.)
//!
//! ## Features
//!
//! - **Multi-chain Support**: Works with any EVM-compatible blockchain via RPC endpoints
//! - **ABI-based Decoding**: Automatically decodes transaction inputs and event logs
//! - **Rule Matching**: Flexible rule system for filtering transactions and events
//! - **Background Monitoring**: Runs monitoring tasks asynchronously without blocking
//! - **Notification Integration**: Sends alerts when matches are found
//!
//! ## Architecture
//!
//! The monitor uses a polling-based approach:
//!
//! 1. Connects to an RPC endpoint via HTTP
//! 2. Polls for new blocks at regular intervals
//! 3. Filters transactions/events based on configured rules
//! 4. Decodes matched items using the contract ABI
//! 5. Triggers notifications for matches
//!
//! ## Usage Example
//!
//! ```no_run
//! use monitor::{PollingMonitor, primitives::models::MonitorRule};
//! use alloy::primitives::Address;
//! use alloy::json_abi::JsonAbi;
//!
//! // Create a monitor instance
//! let monitor = PollingMonitor::new(
//!     "https://sepolia.base.org",
//!     contract_address,
//!     contract_abi
//! )?;
//!
//! // Start monitoring in the background
//! let handle = monitor.start_background_monitoring(
//!     "My Monitor".to_string(),
//!     function_rules,
//!     event_rules,
//!     notification_destination
//! );
//! ```

pub mod events;
pub mod primitives;
pub mod tx;

pub use events::{EventMatcher, EventMonitor};
use notifications::primitives::models::{Alert, NotificationDestination};
pub use tx::{TransactionMonitor, TxMatcher};

use crate::primitives::models::MonitorRule;
use crate::tx::get_tx_details;
use notifications::send_notification;

use alloy::json_abi::JsonAbi;
use alloy::network::{AnyNetwork, TransactionResponse};
use alloy::primitives::Address;
use alloy::providers::{ProviderBuilder, RootProvider};
use serde::{Deserialize, Serialize};

use futures::future::{BoxFuture, FutureExt, join_all};
use tokio::task::JoinHandle;
use tracing::error;

pub type HttpProvider = RootProvider<AnyNetwork>;

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorState {
    pub last_processed_block: u64,
}

#[derive(Clone)]
pub struct PollingMonitor {
    pub provider: HttpProvider,
    pub contract_address: Address,
    pub contract_abi: JsonAbi,
}

impl PollingMonitor {
    pub fn new(
        rpc_url: &str,
        contract_address: Address,
        contract_abi: JsonAbi,
    ) -> Result<Self, anyhow::Error> {
        let url = rpc_url.parse()?;
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .network::<AnyNetwork>()
            .connect_http(url);

        Ok(Self {
            provider,
            contract_address,
            contract_abi,
        })
    }

    pub fn start_background_monitoring(
        self,
        name: String,
        function_rules: Vec<MonitorRule>,
        event_rules: Vec<MonitorRule>,
        destination: Option<NotificationDestination>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut sub_tasks: Vec<BoxFuture<'static, ()>> = vec![];

            // Transaction Sub-Task
            if !function_rules.is_empty() {
                let monitor_tx = self.clone();
                let n = name.clone();
                let abi = self.contract_abi.clone();
                let dest_for_tx = destination.clone();
                let tx_future = async move {
                    let _ = monitor_tx
                        .monitor_transactions_polling(function_rules, move |tx| {
                            println!("[TX ALERT] {}: {:?}", n, tx.tx_hash());

                            let details = get_tx_details(&tx, &abi);

                            let msg = format!(
                                "Transaction Alert: {}\nHash: {:?}\nFrom: {:?}\n{}",
                                n,
                                tx.tx_hash(),
                                tx.from(),
                                details
                            );

                            if let Some(dest) = &dest_for_tx {
                                let alert = Alert {
                                    source: n.clone(),
                                    subject: "TX ALERT".to_string(),
                                    message: msg,
                                };

                                let dest_clone = dest.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = send_notification(&dest_clone, &alert).await {
                                        error!(
                                            destination = ?dest_clone,
                                            source = %alert.source,
                                            subject = %alert.subject,
                                            "Failed to send notification: {e}"
                                        );
                                    }
                                });
                            }
                        })
                        .await;
                }
                .boxed();
                sub_tasks.push(tx_future);
            };

            // Event Sub-Task (supports conditional event rules)
            if !event_rules.is_empty() {
                let monitor_events = self.clone();
                let n = name.clone();
                let abi = self.contract_abi.clone();
                let dest_for_events = destination.clone();
                let event_rules_ref: Vec<MonitorRule> = event_rules.clone();

                // Build event name list for the poller
                let event_names: Vec<String> =
                    event_rules_ref.iter().map(|r| r.name.clone()).collect();

                let events_future = async move {
                    // convert String -> &str for the trait
                    let refs: Vec<&str> = event_names.iter().map(|s| s.as_str()).collect();

                    let _ = monitor_events
                        .monitor_events_polling(&refs, move |log| {
                            // Check each rule to see if it matches
                            for rule in &event_rules_ref {
                                if rule.event_match(&log) {
                                    println!("[EVENT ALERT] {}: Block {:?}", n, log.block_number);
                                    let event_details = events::get_event_details(&log, &abi);
                                    let block = log
                                        .block_number
                                        .map(|b| b.to_string())
                                        .unwrap_or_else(|| "unknown".into());
                                    let msg = format!(
                                        "Event Alert: {}\nBlock: {}\n{}",
                                        n, block, event_details
                                    );

                                    if let Some(dest) = &dest_for_events {
                                        let alert = Alert {
                                            source: n.clone(),
                                            subject: "Event ALERT".to_string(),
                                            message: msg,
                                        };

                                        let dest_clone = dest.clone();

                                        tokio::spawn(async move {
                                            if let Err(e) =
                                                send_notification(&dest_clone, &alert).await
                                            {
                                                error!(
                                                    destination = ?dest_clone,
                                                    source = %alert.source,
                                                    subject = %alert.subject,
                                                    "Failed to send notification: {e}"
                                                );
                                            }
                                        });
                                    }
                                    break; // Only notify once per log
                                }
                            }
                        })
                        .await;
                }
                .boxed();

                sub_tasks.push(events_future);
            }

            // Keep alive
            if !sub_tasks.is_empty() {
                join_all(sub_tasks).await;
            }
        })
    }
}
