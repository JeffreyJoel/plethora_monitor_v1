pub mod events;
pub mod filter;
pub mod primitives;
pub mod tx;

pub use events::EventMonitor;
use futures::future::join_all;
use tokio::task::JoinHandle;
pub use tx::TransactionMonitor;

use alloy::json_abi::JsonAbi;
use alloy::network::{AnyNetwork, TransactionResponse};
use alloy::primitives::Address;
use alloy::providers::{ProviderBuilder, RootProvider};
use serde::{Deserialize, Serialize};

use crate::primitives::models::MonitorRule;

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
        tx_rules: Vec<MonitorRule>,
        event_names: Vec<String>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut sub_tasks = vec![];

            // Transaction Sub-Task
            if !tx_rules.is_empty() {
                let monitor_tx = self.clone();
                let n = name.clone();
                sub_tasks.push(tokio::spawn(async move {
                    let _ = monitor_tx
                        .monitor_transactions_polling(tx_rules, move |tx| {
                            println!("[TX ALERT] {}: {:?}", n, tx.tx_hash());
                        })
                        .await;
                }));
            }

            // Event Sub-Task
            if !event_names.is_empty() {
                let monitor_events = self.clone();
                let n = name.clone();
                let events_ref: Vec<String> = event_names.clone();

                sub_tasks.push(tokio::spawn(async move {
                    // convert String -> &str for the trait
                    let refs: Vec<&str> = events_ref.iter().map(|s| s.as_str()).collect();
                    let _ = monitor_events
                        .monitor_events_polling(&refs, move |log| {
                            println!("[EVENT ALERT] {}: Block {:?}", n, log.block_number);
                        })
                        .await;
                }));
            }

            // Keep alive
            if !sub_tasks.is_empty() {
                join_all(sub_tasks).await;
            }
        })
    }
}
