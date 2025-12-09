use alloy::{consensus::Transaction, primitives::Address};
use alloy_chains::{Chain, NamedChain};
use anyhow::Context;
use config::AppConfig;
use dotenvy::dotenv;
use foundry_block_explorers::Client;
use futures::future::join_all;
use monitor::{EventMonitor, PollingMonitor, TransactionMonitor};
use std::{env, str::FromStr};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    // read from the Settings.toml
    let settings = AppConfig::new().expect("Failed to load Settings.toml");

    let etherscan_api_key =
        env::var("ETHERSCAN_API_KEY").context("ETHERSCAN_API_KEY must be set in your .env file")?;

    let mut handles = vec![];

    for monitor_cfg in settings.monitors {
        println!("Launching monitor: {}", monitor_cfg.name);

        let named_chain = NamedChain::from_str(&monitor_cfg.chain)?;

        let chain: Chain = Chain::from(named_chain);

        let client = Client::new(chain, &etherscan_api_key)?;

        let addr: Address = monitor_cfg.address.parse()?;

        let abi = client.contract_abi(addr).await?;

        let state_file = format!(
            "state_{}.json",
            monitor_cfg.name.replace(" ", "_").to_lowercase()
        );

        let functions_config = monitor_cfg.functions.clone();
        let name_clone = monitor_cfg.name.clone();

        let handle = tokio::spawn(async move {
            let monitor = PollingMonitor::new(&monitor_cfg.rpc_url, addr, abi.clone(), &state_file)
                .expect("Failed to init monitor");

            if let Some(funcs) = functions_config {
                if !funcs.is_empty() {
                    let monitor_tx = monitor.clone();

                    // Spawn for Transactions
                    tokio::spawn(async move {
                        if let Err(e) = monitor_tx
                            .monitor_transactions_polling(funcs, move |tx| {
                                println!("--------------------------------");
                                println!("[{}] FUNCTION CALL DETECTED", name_clone);
                                println!("Tx Hash: {:?}", tx.inner.hash());
                                println!("To: {:?}", tx.inner.to());
                                println!("--------------------------------");
                            })
                            .await
                        {
                            eprintln!("Tx Monitor crashed: {:?}", e);
                        }
                    });
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
                            println!("[{}] EVENT DETECTED!", name);
                            println!("Block Number: {:?}", log.block_number);
                            println!("--------------------------------");
                        })
                        .await
                    {
                        eprintln!("Event Monitor {} crashed: {:?}", monitor_cfg.name, e);
                    }
                }
            }
        });

        handles.push(handle);
    }

    println!("All monitors launched. Waiting...");

    // join_all waits for every handle in the vector to finish.
    join_all(handles).await;

    Ok(())
}
