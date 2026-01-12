//! Event monitoring module.

use crate::block_watcher::BlockUpdate;
use crate::PollingMonitor;
use crate::primitives::utils::format_value;
use alloy::dyn_abi::EventExt;
use alloy::json_abi::JsonAbi;
use alloy::primitives::B256;
use alloy::providers::Provider;
use alloy::rpc::types::{Filter, Log};
use tokio::sync::broadcast;

/// Trait for matching events against rules
pub trait EventMatcher {
    fn event_match(&self, log: &Log) -> bool;
}

impl PollingMonitor {
    /// this method subscribes to block updates from the BlockWatcherRegistry and only
    /// fetches logs when notified, reducing RPC calls when multiple
    /// monitors share the same chain.
    pub async fn monitor_events_with_watcher<F>(
        self,
        event_names: &[&str],
        mut block_receiver: broadcast::Receiver<BlockUpdate>,
        mut handler: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(Log) + Send + 'static,
    {
        println!(
            "EventsMonitor (watcher): Watching transactions for {:?}",
            self.contract_address
        );

        let mut topics: Vec<B256> = Vec::new();
        for event_name in event_names {
            // find the event by name (e.g. "Transfer")
            if let Some(events) = self.contract_abi.events.get(*event_name) {
                if let Some(event) = events.first() {
                    topics.push(event.selector());
                }
            } else {
                eprintln!("Warning: Event '{}' not found in ABI", event_name);
            }
        }

        if topics.is_empty() {
            eprintln!("EventsMonitor: No valid event topics found. Monitor will not catch anything.");
        }

        loop {
            // Wait for block updates from the shared watcher
            let update = match block_receiver.recv().await {
                Ok(update) => update,
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::warn!("EventsMonitor: Block watcher channel closed, stopping");
                    return Ok(());
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(
                        "EventsMonitor: Lagged behind by {} blocks, catching up",
                        skipped
                    );
                    continue;
                }
            };

            // Check if we should try recovering to primary
            self.try_recover_primary().await;

            let from_block = update.previous_block + 1;
            let to_block = update.block_number;

            let filter = Filter::new()
                .address(self.contract_address)
                .event_signature(topics.clone())
                .from_block(from_block)
                .to_block(to_block);

            match self.provider().get_logs(&filter).await {
                Ok(logs) => {
                    self.record_success();
                    for log in logs {
                        handler(log);
                    }
                }
                Err(e) => {
                    self.record_failure();
                    eprintln!("Error fetching logs: {}", e);
                }
            }
        }
    }
}

/// Decodes event logs using the ABI and returns a formatted string for notifications.
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

/// Maps event monitor rules to their corresponding ABI event definitions.
pub fn map_event_rules_to_abi(
    mut rules: Vec<crate::primitives::models::MonitorRule>,
    abi: &JsonAbi,
) -> Vec<crate::primitives::models::MonitorRule> {
    for rule in &mut rules {
        if let Some(event) = abi.event(&rule.name).and_then(|e| e.first()) {
            rule.abi_event = Some(event.clone());
        } else {
            eprintln!(
                "Warning: Event rule '{}' not found in contract ABI",
                rule.name
            );
        }
    }
    rules
}

/// Implementation of EventMatcher for MonitorRule
impl EventMatcher for crate::primitives::models::MonitorRule {
    fn event_match(&self, log: &Log) -> bool {
        use crate::primitives::models::Condition;
        use crate::primitives::utils::check_argument_condition;
        use alloy::dyn_abi::EventExt;

        let event_def = match &self.abi_event {
            Some(e) => e,
            None => return false,
        };

        let topics = log.topics();
        if topics.is_empty() {
            return false;
        }

        if topics[0] != event_def.selector() {
            return false;
        }

        if self.conditions.is_empty() {
            return true;
        }

        let decoded = match event_def.decode_log(&log.data()) {
            Ok(d) => d,
            Err(_) => return false,
        };

        let mut arg_map: std::collections::HashMap<String, &alloy::dyn_abi::DynSolValue> =
            std::collections::HashMap::new();

        let mut indexed_iter = decoded.indexed.iter();
        let mut body_iter = decoded.body.iter();

        for input in &event_def.inputs {
            let val = if input.indexed {
                indexed_iter.next()
            } else {
                body_iter.next()
            };

            if let Some(v) = val {
                arg_map.insert(input.name.clone(), v);
            }
        }

        for condition in &self.conditions {
            match condition {
                Condition::Argument {
                    name,
                    operator,
                    value,
                } => {
                    if let Some(arg_value) = arg_map.get(name) {
                        if !check_argument_condition(arg_value, operator, value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => continue,
            }
        }

        true
    }
}

