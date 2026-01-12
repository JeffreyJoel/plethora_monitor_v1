//! Shared block polling service - reduces RPC calls by consolidating block number
//! queries across multiple monitors on the same chain.

use crate::HttpProvider;
use crate::primitives::chains::{get_default_rpc_url, get_infura_url, normalize_chain_name};
use alloy::network::AnyNetwork;
use alloy::providers::{Provider, ProviderBuilder};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

const CHANNEL_CAPACITY: usize = 64;
const POLL_INTERVAL_SECS: u64 = 2;
const FAILURE_THRESHOLD: u32 = 3;
const PRIMARY_RETRY_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

#[derive(Debug, Clone)]
pub struct BlockUpdate {
    pub block_number: u64,
    pub previous_block: u64,
}

pub struct BlockWatcher {
    chain_name: String,
    rpc_url: String,
    primary_provider: HttpProvider,
    fallback_provider: Option<HttpProvider>,
    sender: broadcast::Sender<BlockUpdate>,
    task_handle: Option<JoinHandle<()>>,
}

impl BlockWatcher {
    pub fn new(chain_name: &str, rpc_url: &str) -> Result<Self, anyhow::Error> {
        let url = rpc_url.parse()?;
        let primary_provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .network::<AnyNetwork>()
            .connect_http(url);

        // Try to create fallback provider from Infura
        let fallback_provider = get_infura_url(chain_name).and_then(|infura_url| {
            infura_url.parse().ok().map(|url| {
                info!(
                    "Created Infura fallback provider for BlockWatcher on chain '{}'",
                    chain_name
                );
                ProviderBuilder::new()
                    .disable_recommended_fillers()
                    .network::<AnyNetwork>()
                    .connect_http(url)
            })
        });

        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);

        Ok(Self {
            chain_name: normalize_chain_name(chain_name),
            rpc_url: rpc_url.to_string(),
            primary_provider,
            fallback_provider,
            sender,
            task_handle: None,
        })
    }

    pub fn chain_name(&self) -> &str {
        &self.chain_name
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BlockUpdate> {
        self.sender.subscribe()
    }

    pub fn start(&mut self) {
        if self.task_handle.is_some() {
            warn!(
                "BlockWatcher for chain '{}' is already running",
                self.chain_name
            );
            return;
        }

        let primary_provider = self.primary_provider.clone();
        let fallback_provider = self.fallback_provider.clone();
        let sender = self.sender.clone();
        let chain_name = self.chain_name.clone();
        let rpc_url = self.rpc_url.clone();

        let handle = tokio::spawn(async move {
            info!(
                "BlockWatcher started for chain '{}' (RPC: {})",
                chain_name, rpc_url
            );

            // Shared state for fallback tracking
            let use_fallback = Arc::new(AtomicBool::new(false));
            let consecutive_failures = Arc::new(AtomicU32::new(0));
            let last_primary_check = Arc::new(RwLock::new(Instant::now()));

            // Helper to get current provider
            let get_provider = |use_fb: bool| -> &HttpProvider {
                if use_fb {
                    fallback_provider.as_ref().unwrap_or(&primary_provider)
                } else {
                    &primary_provider
                }
            };

            let provider = get_provider(false);
            let mut current_block = match provider.get_block_number().await {
                Ok(num) => {
                    debug!("BlockWatcher [{}]: Initial block {}", chain_name, num);
                    num
                }
                Err(e) => {
                    error!(
                        "BlockWatcher [{}]: Failed to get initial block number: {}",
                        chain_name, e
                    );
                    0
                }
            };

            loop {
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

                let using_fallback = use_fallback.load(Ordering::Relaxed);

                // Periodically try to recover to primary when on fallback
                if using_fallback && fallback_provider.is_some() {
                    let should_check = {
                        let last_check = last_primary_check.read().await;
                        last_check.elapsed() >= PRIMARY_RETRY_INTERVAL
                    };

                    if should_check {
                        if primary_provider.get_block_number().await.is_ok() {
                            use_fallback.store(false, Ordering::Relaxed);
                            consecutive_failures.store(0, Ordering::Relaxed);
                            info!(
                                "BlockWatcher [{}]: Recovered to primary provider",
                                chain_name
                            );
                        }
                        *last_primary_check.write().await = Instant::now();
                    }
                }

                let provider = if use_fallback.load(Ordering::Relaxed) {
                    fallback_provider.as_ref().unwrap_or(&primary_provider)
                } else {
                    &primary_provider
                };

                match provider.get_block_number().await {
                    Ok(latest_block) => {
                        consecutive_failures.store(0, Ordering::Relaxed);

                        if latest_block > current_block {
                            let update = BlockUpdate {
                                block_number: latest_block,
                                previous_block: current_block,
                            };

                            let _ = sender.send(update);

                            debug!(
                                "BlockWatcher [{}]: New block {} (previous: {})",
                                chain_name, latest_block, current_block
                            );

                            current_block = latest_block;
                        }
                    }
                    Err(e) => {
                        let failures = consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                        warn!(
                            "BlockWatcher [{}]: Error fetching block number (attempt {}): {}",
                            chain_name, failures, e
                        );

                        // Switch to fallback after threshold
                        if failures >= FAILURE_THRESHOLD
                            && fallback_provider.is_some()
                            && !use_fallback.load(Ordering::Relaxed)
                        {
                            use_fallback.store(true, Ordering::Relaxed);
                            warn!(
                                "BlockWatcher [{}]: Switched to Infura fallback after {} failures",
                                chain_name, failures
                            );
                        }
                    }
                }
            }
        });

        self.task_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            info!("BlockWatcher stopped for chain '{}'", self.chain_name);
        }
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Drop for BlockWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Registry that manages one BlockWatcher per chain.
pub struct BlockWatcherRegistry {
    watchers: RwLock<HashMap<String, Arc<RwLock<BlockWatcher>>>>,
}

impl BlockWatcherRegistry {
    pub fn new() -> Self {
        Self {
            watchers: RwLock::new(HashMap::new()),
        }
    }

    /// Gets or creates a BlockWatcher for the chain. Uses default public RPC if available.
    pub async fn subscribe(
        &self,
        chain_name: &str,
        fallback_rpc_url: &str,
    ) -> Result<broadcast::Receiver<BlockUpdate>, anyhow::Error> {
        let normalized_chain = normalize_chain_name(chain_name);

        // Try to get existing watcher
        {
            let watchers = self.watchers.read().await;
            if let Some(watcher) = watchers.get(&normalized_chain) {
                let watcher_guard = watcher.read().await;
                debug!(
                    "Reusing existing BlockWatcher for chain '{}' ({} subscribers)",
                    normalized_chain,
                    watcher_guard.receiver_count()
                );
                return Ok(watcher_guard.subscribe());
            }
        }

        // Create new watcher
        let mut watchers = self.watchers.write().await;

        // Double-check after acquiring write lock
        if let Some(watcher) = watchers.get(&normalized_chain) {
            let watcher_guard = watcher.read().await;
            return Ok(watcher_guard.subscribe());
        }

        let rpc_url = get_default_rpc_url(chain_name).unwrap_or(fallback_rpc_url);

        let mut watcher = BlockWatcher::new(chain_name, rpc_url)?;
        watcher.start();

        let receiver = watcher.subscribe();
        let watcher_arc = Arc::new(RwLock::new(watcher));

        watchers.insert(normalized_chain.clone(), watcher_arc);

        let rpc_source = if get_default_rpc_url(chain_name).is_some() {
            "default"
        } else {
            "fallback"
        };
        info!(
            "Created new BlockWatcher for chain '{}' (RPC: {} [{}])",
            normalized_chain, rpc_url, rpc_source
        );

        Ok(receiver)
    }

    pub async fn watcher_count(&self) -> usize {
        self.watchers.read().await.len()
    }

    pub async fn remove(&self, chain_name: &str) {
        let normalized_chain = normalize_chain_name(chain_name);
        let mut watchers = self.watchers.write().await;
        if let Some(watcher) = watchers.remove(&normalized_chain) {
            let mut watcher_guard = watcher.write().await;
            watcher_guard.stop();
            info!("Removed BlockWatcher for chain '{}'", normalized_chain);
        }
    }

    pub async fn shutdown(&self) {
        let mut watchers = self.watchers.write().await;
        for (chain, watcher) in watchers.drain() {
            let mut watcher_guard = watcher.write().await;
            watcher_guard.stop();
            info!("Shutdown BlockWatcher for chain '{}'", chain);
        }
    }
}
