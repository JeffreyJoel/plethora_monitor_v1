use alloy::{
    network::{TransactionBuilder, TransactionResponse},
    primitives::{Address, Bytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
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

        // Check the contract address whether its a proxy contract and then return the impl contract address
        // and get the abi for that
        let addr = Address::from_str(&monitor_cfg.address)?;
        let provider = ProviderBuilder::new().connect_http(monitor_cfg.rpc_url.parse()?);
        let tx = TransactionRequest::default()
            .with_to(addr)
            .with_input("0x5c60da1b".parse::<Bytes>()?); // implementation()

        let target_addr = match provider.call(tx).await {
            Ok(bytes) if bytes.len() >= 32 => Address::from_slice(&bytes[12..32]),
            _ => addr,
        };

        let abi = client.contract_abi(target_addr).await?;

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
                                println!("Tx Hash: {:?}", tx.tx_hash());
                                println!("From: {:?}", tx.from());
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
                            if let Some(topic0) = log.topics().first() {
                                // Find event by selector
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
        });

        handles.push(handle);
    }

    println!("All monitors launched. Waiting...");

    // join_all waits for every handle in the vector to finish.
    join_all(handles).await;

    Ok(())
}
