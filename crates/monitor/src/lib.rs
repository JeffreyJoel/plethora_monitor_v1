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

use futures::future::join_all;
use tokio::task::JoinHandle;

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
            let mut sub_tasks = vec![];

            // Transaction Sub-Task
            if !function_rules.is_empty() {
                let monitor_tx = self.clone();
                let n = name.clone();
                let abi = self.contract_abi.clone();
                let dest_for_tx = destination.clone();
                sub_tasks.push(tokio::spawn(async move {
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
                                    let _ = send_notification(&dest_clone, &alert).await;
                                });
                            }
                        })
                        .await;
                }));
            }

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

                sub_tasks.push(tokio::spawn(async move {
                    // convert String -> &str for the trait
                    let refs: Vec<&str> = event_names.iter().map(|s| s.as_str()).collect();

                    let _ = monitor_events
                        .monitor_events_polling(&refs, move |log| {
                            // Check each rule to see if it matches
                            for rule in &event_rules_ref {
                                if rule.event_match(&log) {
                                    println!("[EVENT ALERT] {}: Block {:?}", n, log.block_number);
                                    let event_details = events::get_event_details(&log, &abi);
                                    let msg = format!(
                                        "Event Alert: {}\nBlock: {:?}\n{}",
                                        n, log.block_number, event_details
                                    );

                                    if let Some(dest) = &dest_for_events {
                                        let alert = Alert {
                                            source: n.clone(),
                                            subject: "Event ALERT".to_string(),
                                            message: msg,
                                        };

                                        let dest_clone = dest.clone();

                                        tokio::spawn(async move {
                                            let _ = send_notification(&dest_clone, &alert).await;
                                        });
                                    }
                                    break; // Only notify once per log
                                }
                            }
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
