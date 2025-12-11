pub mod events;
pub mod filter;
pub mod primitives;
pub mod tx;

pub use events::EventMonitor;
pub use tx::TransactionMonitor;

use crate::primitives::models::MonitorRule;
use crate::tx::get_tx_details;
use notifications::{Alert, NotificationDestination, send_notification};

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
        tx_rules: Vec<MonitorRule>,
        event_names: Vec<String>,
        email_recipient: Option<String>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut sub_tasks = vec![];

            // Transaction Sub-Task
            if !tx_rules.is_empty() {
                let monitor_tx = self.clone();
                let n = name.clone();
                let email_addr = email_recipient.clone();

                let abi = self.contract_abi.clone();

                sub_tasks.push(tokio::spawn(async move {
                    let _ = monitor_tx
                        .monitor_transactions_polling(tx_rules, move |tx| {
                            println!("[TX ALERT] {}: {:?}", n, tx.tx_hash());

                            let details = get_tx_details(&tx, &abi);

                            let msg = format!(
                                "Transaction Alert: {}\nHash: {:?}\nFrom: {:?}\n{}",
                                n,
                                tx.tx_hash(),
                                tx.from(),
                                details
                            );

                            if let Some(email) = &email_addr {
                                let destination = NotificationDestination::Email(email.clone());
                                let alert = Alert {
                                    source: n.clone(),
                                    subject: "TX ALERT".to_string(),
                                    message: msg,
                                };

                                tokio::spawn(async move {
                                    let _ = send_notification(&destination, &alert).await;
                                });
                            }
                        })
                        .await;
                }));
            }

            // Event Sub-Task
            if !event_names.is_empty() {
                let monitor_events = self.clone();
                let n = name.clone();
                let email_addr = email_recipient.clone();
                let events_ref: Vec<String> = event_names.clone();

                let abi = self.contract_abi.clone();

                sub_tasks.push(tokio::spawn(async move {
                    // convert String -> &str for the trait
                    let refs: Vec<&str> = events_ref.iter().map(|s| s.as_str()).collect();
                    let _ = monitor_events
                        .monitor_events_polling(&refs, move |log| {
                            println!("[EVENT ALERT] {}: Block {:?}", n, log.block_number);

                            let event_details = events::get_event_details(&log, &abi);

                            let msg = format!(
                                "Event Alert: {}\nBlock: {:?}\n{}",
                                n, log.block_number, event_details
                            );

                            if let Some(email) = &email_addr {
                                let destination = NotificationDestination::Email(email.clone());
                                let alert = Alert {
                                    source: n.clone(),
                                    subject: "Event ALERT".to_string(),
                                    message: msg,
                                };

                                tokio::spawn(async move {
                                    let _ = send_notification(&destination, &alert).await;
                                });
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
