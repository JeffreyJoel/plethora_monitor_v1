use alloy::{
    network::{TransactionResponse},
    primitives::Address,
};
use config::AppConfig;
use dotenvy::dotenv;
use futures::future::join_all;
use monitor::primitives::utils;
use monitor::{EventMonitor, PollingMonitor, TransactionMonitor};
use std::{str::FromStr};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    // read from the Settings.toml
    let settings = AppConfig::new().expect("Failed to load Settings.toml");

    let mut handles = vec![];

    for monitor_cfg in settings.monitors {
        println!("Launching monitor: {}", monitor_cfg.name);

        let addr = Address::from_str(&monitor_cfg.address)?;

        let abi = utils::fetch_abi(
            &monitor_cfg.chain,
            &monitor_cfg.address,
            &monitor_cfg.rpc_url,
        ).await?;


        let functions_config = monitor_cfg.functions.clone();
        let name_clone = monitor_cfg.name.clone();

        let handle = tokio::spawn(async move {
            let monitor = PollingMonitor::new(&monitor_cfg.rpc_url, addr, abi.clone())
                .expect("Failed to init monitor");

            let mut tx_handle = None;

            if let Some(funcs) = functions_config {
                if !funcs.is_empty() {
                    let monitor_tx = monitor.clone();

                    // Spawn for Transactions
                    tx_handle = Some(tokio::spawn(async move {
                        if let Err(e) = monitor_tx
                            .monitor_transactions_polling(funcs, move |tx| {
                                println!("--------------------------------");
                                println!("[{}] FUNCTION CALL DETECTED", name_clone);
                                println!("Tx Hash: {:?}", tx.tx_hash());
                                println!("From: {:?}", tx.from());
                                println!("--------------------------------");
                            })
                            .await
                        {
                            eprintln!("Tx Monitor crashed: {:?}", e);
                        }
                    }));
                }
            }

            // Run monitor for events
            if let Some(events) = &monitor_cfg.events {
                if !events.is_empty() {
                    let event_refs: Vec<&str> = events.iter().map(|s| s.as_str()).collect();
                    let name = monitor_cfg.name.clone();

                    if let Err(e) = monitor
                        .monitor_events_polling(&event_refs, move |log| {
                            println!("--------------------------------");
                            if let Some(topic0) = log.topics().first() {
                                if let Some(event) =
                                    abi.clone().events().find(|e| e.selector() == *topic0)
                                {
                                    println!("{}  EVENT DETECTED! on Monitor {}", event.name, name);
                                }
                            }
                            println!("Block Number: {:?}", log.block_number);
                            println!("Transaction Hash: {:?}", log.transaction_hash);
                            println!("Transaction data: {:?}", log.data());
                            println!("--------------------------------");
                        })
                        .await
                    {
                        eprintln!("Event Monitor {} crashed: {:?}", monitor_cfg.name, e);
                    }
                }
            }

            // Wait for the transaction monitor if it was spawned
            if let Some(h) = tx_handle {
                let _ = h.await;
            }
        });

        handles.push(handle);
    }

    println!("All monitors launched. Waiting...");

    // join_all waits for every handle in the vector to finish.
    join_all(handles).await;

    Ok(())
}
