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
//! - **`BlockWatcher`** - Shared block polling service that reduces RPC calls by
//!   consolidating block number queries across multiple monitors on the same chain
//! - **`BlockWatcherRegistry`** - Registry for managing BlockWatcher instances per chain
//! - **`primitives`** - Shared data structures and utilities (models, ABI fetching, etc.)
//!
//! ## Features
//!
//! - **Multi-chain Support**: Works with any EVM-compatible blockchain via RPC endpoints
//! - **Shared Block Polling**: Multiple monitors on the same chain share a single block watcher
//! - **ABI-based Decoding**: Automatically decodes transaction inputs and event logs
//! - **Rule Matching**: Flexible rule system for filtering transactions and events
//! - **Background Monitoring**: Runs monitoring tasks asynchronously without blocking
//! - **Notification Integration**: Sends alerts when matches are found
//!
//! ## Architecture
//!
//! The monitor uses a shared block watcher approach:
//!
//! 1. BlockWatcherRegistry manages one BlockWatcher per chain (keyed by normalized chain name)
//! 2. All monitors on the same chain share a single BlockWatcher regardless of RPC URL
//! 3. BlockWatcher polls for new blocks and broadcasts to all subscribers
//! 4. Monitors subscribe to block updates instead of polling independently
//! 5. Filters transactions/events based on configured rules
//! 6. Decodes matched items using the contract ABI
//! 7. Triggers notifications for matches
//!
//! ## Usage Example
//!
//! ```no_run
//! use monitor::{PollingMonitor, primitives::models::MonitorRule, block_watcher::BlockWatcherRegistry};
//! use alloy::primitives::Address;
//! use alloy::json_abi::JsonAbi;
//! use std::sync::Arc;
//!
//! // Create a shared block watcher registry
//! let registry = Arc::new(BlockWatcherRegistry::new());
//!
//! // Create a monitor instance
//! let monitor = PollingMonitor::new(
//!     "https://sepolia.base.org",
//!     contract_address,
//!     contract_abi
//! )?;
//!
//! // Start monitoring in the background with shared block watching
//! let handle = monitor.start_background_monitoring_with_watcher(
//!     "My Monitor".to_string(),
//!     function_rules,
//!     event_rules,
//!     notification_destination,
//!     registry
//! ).await;
//! ```

pub mod block_watcher;
pub mod events;
pub mod primitives;
pub mod tx;

pub use block_watcher::{BlockUpdate, BlockWatcher, BlockWatcherRegistry};
pub use primitives::chains::normalize_chain_name;
pub use events::EventMatcher;
use notifications::primitives::models::{Alert, NotificationDestination};
pub use tx::TxMatcher;

use crate::primitives::models::MonitorRule;
use crate::tx::get_tx_details;
use notifications::send_notification;

use alloy::json_abi::JsonAbi;
use alloy::network::{AnyNetwork, TransactionResponse};
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use serde::{Deserialize, Serialize};

use crate::primitives::chains::get_infura_url;
use futures::future::{BoxFuture, FutureExt, join_all};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub type HttpProvider = RootProvider<AnyNetwork>;

const FAILURE_THRESHOLD: u32 = 3;
const PRIMARY_RETRY_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorState {
    pub last_processed_block: u64,
}

/// Shared state for tracking provider failures across cloned monitors.
pub struct ProviderState {
    consecutive_failures: AtomicU32,
    use_fallback: std::sync::atomic::AtomicBool,
    last_primary_check: RwLock<Instant>,
}

impl ProviderState {
    fn new() -> Self {
        Self {
            consecutive_failures: AtomicU32::new(0),
            use_fallback: std::sync::atomic::AtomicBool::new(false),
            last_primary_check: RwLock::new(Instant::now()),
        }
    }
}

#[derive(Clone)]
pub struct PollingMonitor {
    primary_provider: HttpProvider,
    fallback_provider: Option<HttpProvider>,
    provider_state: Arc<ProviderState>,
    pub contract_address: Address,
    pub contract_abi: JsonAbi,
    pub rpc_url: String,
    pub chain: String,
}

impl PollingMonitor {
    pub fn new(
        rpc_url: &str,
        contract_address: Address,
        contract_abi: JsonAbi,
        chain: &str,
    ) -> Result<Self, anyhow::Error> {
        let url = rpc_url.parse()?;
        let primary_provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .network::<AnyNetwork>()
            .connect_http(url);

        // Try to create fallback provider from Infura
        let fallback_provider = get_infura_url(chain).and_then(|infura_url| {
            infura_url.parse().ok().map(|url| {
                info!("Created Infura fallback provider for chain '{}'", chain);
                ProviderBuilder::new()
                    .disable_recommended_fillers()
                    .network::<AnyNetwork>()
                    .connect_http(url)
            })
        });

        Ok(Self {
            primary_provider,
            fallback_provider,
            provider_state: Arc::new(ProviderState::new()),
            contract_address,
            contract_abi,
            rpc_url: rpc_url.to_string(),
            chain: chain.to_string(),
        })
    }

    /// Returns the currently active provider (primary or fallback).
    pub fn provider(&self) -> &HttpProvider {
        if self.provider_state.use_fallback.load(Ordering::Relaxed) {
            self.fallback_provider
                .as_ref()
                .unwrap_or(&self.primary_provider)
        } else {
            &self.primary_provider
        }
    }

    /// Records a successful RPC call, resetting the failure counter.
    pub fn record_success(&self) {
        let prev_failures = self.provider_state.consecutive_failures.swap(0, Ordering::Relaxed);
        if prev_failures > 0 {
            info!(
                "Provider for chain '{}' recovered after {} failures",
                self.chain, prev_failures
            );
        }
    }

    /// Records a failed RPC call. Switches to fallback after FAILURE_THRESHOLD consecutive failures.
    pub fn record_failure(&self) {
        let failures = self
            .provider_state
            .consecutive_failures
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        if failures >= FAILURE_THRESHOLD
            && self.fallback_provider.is_some()
            && !self.provider_state.use_fallback.load(Ordering::Relaxed)
        {
            self.provider_state
                .use_fallback
                .store(true, Ordering::Relaxed);
            warn!(
                "Switched to Infura fallback for chain '{}' after {} consecutive failures",
                self.chain, failures
            );
        }
    }

    /// Checks if we should try the primary provider again (called periodically when on fallback).
    pub async fn try_recover_primary(&self) {
        if !self.provider_state.use_fallback.load(Ordering::Relaxed) {
            return;
        }

        let should_check = {
            let last_check = self.provider_state.last_primary_check.read().await;
            last_check.elapsed() >= PRIMARY_RETRY_INTERVAL
        };

        if should_check {
            // Try a simple call to primary to see if it's back
            if self
                .primary_provider
                .get_block_number()
                .await
                .is_ok()
            {
                self.provider_state
                    .use_fallback
                    .store(false, Ordering::Relaxed);
                self.provider_state
                    .consecutive_failures
                    .store(0, Ordering::Relaxed);
                info!(
                    "Recovered to primary provider for chain '{}'",
                    self.chain
                );
            }
            // Update last check time
            *self.provider_state.last_primary_check.write().await = Instant::now();
        }
    }

    /// Returns true if currently using the fallback provider.
    pub fn is_using_fallback(&self) -> bool {
        self.provider_state.use_fallback.load(Ordering::Relaxed)
    }


    /// Starts background monitoring using a shared BlockWatcherRegistry.
    /// Monitors on the same chain share a single BlockWatcher to reduce RPC calls.
    pub async fn start_background_monitoring_with_watcher(
        self,
        name: String,
        function_rules: Vec<MonitorRule>,
        event_rules: Vec<MonitorRule>,
        destination: Option<NotificationDestination>,
        registry: Arc<BlockWatcherRegistry>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        // Subscribe to block updates from the shared watcher (keyed by chain name)
        let block_receiver = registry.subscribe(&self.chain, &self.rpc_url).await?;

        Ok(self.start_monitoring_with_receiver(
            name,
            function_rules,
            event_rules,
            destination,
            block_receiver,
        ))
    }

    /// Internal method that starts monitoring with a block receiver.
    fn start_monitoring_with_receiver(
        self,
        name: String,
        function_rules: Vec<MonitorRule>,
        event_rules: Vec<MonitorRule>,
        destination: Option<NotificationDestination>,
        block_receiver: broadcast::Receiver<BlockUpdate>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut sub_tasks: Vec<BoxFuture<'static, ()>> = vec![];

            // Transaction Sub-Task with shared block watcher
            if !function_rules.is_empty() {
                let monitor_tx = self.clone();
                let n = name.clone();
                let abi = self.contract_abi.clone();
                let dest_for_tx = destination.clone();
                let block_rx = block_receiver.resubscribe();

                let tx_future = async move {
                    let _ = monitor_tx
                        .monitor_transactions_with_watcher(function_rules, block_rx, move |tx| {
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

            // Event Sub-Task with shared block watcher
            if !event_rules.is_empty() {
                let monitor_events = self.clone();
                let n = name.clone();
                let abi = self.contract_abi.clone();
                let dest_for_events = destination.clone();
                let event_rules_ref: Vec<MonitorRule> = event_rules.clone();
                let block_rx = block_receiver.resubscribe();

                // Build event name list for the poller
                let event_names: Vec<String> =
                    event_rules_ref.iter().map(|r| r.name.clone()).collect();

                let events_future = async move {
                    // convert String -> &str for the trait
                    let refs: Vec<&str> = event_names.iter().map(|s| s.as_str()).collect();

                    let _ = monitor_events
                        .monitor_events_with_watcher(&refs, block_rx, move |log| {
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
