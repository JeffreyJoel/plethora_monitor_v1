use alloy::primitives::Address;
use alloy_chains::{Chain, NamedChain};
use anyhow::Context;
use config::AppConfig;
use dotenvy::dotenv;
use foundry_block_explorers::Client;
use futures::future::join_all;
use monitor::PollingMonitor;
use std::{env, str::FromStr};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();
    let settings = AppConfig::new().expect("Failed to load Settings.toml");

    let etherscan_api_key =
        env::var("ETHERSCAN_API_KEY").context("ETHERSCAN_API_KEY must be set in your .env file")?;

    let mut handles = vec![];

    for monitor_cfg in settings.monitors {
        println!("🚀 Launching monitor: {}", monitor_cfg.name);

        let named_chain = NamedChain::from_str(&monitor_cfg.chain)?;

        let chain: Chain = Chain::from(named_chain);

        let client = Client::new(chain, &etherscan_api_key)?;

        let addr: Address = monitor_cfg.address.parse()?;

        let abi = client.contract_abi(addr).await?;

        let state_file = format!(
            "state_{}.json",
            monitor_cfg.name.replace(" ", "_").to_lowercase()
        );

        let monitor = PollingMonitor::new(&monitor_cfg.rpc_url, addr, abi.clone(), &state_file)
            .expect("Failed to init monitor");

        // Clone data needed for the thread
        let events: Vec<String> = monitor_cfg.events.clone();
        let name = monitor_cfg.name.clone();

        let abi_for_decode = abi.clone();

        let handle = tokio::spawn(async move {
            // Convert Vec<String> to Vec<&str> for the monitor call
            let event_refs: Vec<&str> = events.iter().map(|s| s.as_str()).collect();

            if let Err(e) = monitor
                .monitor_events_polling(&event_refs, move |log| {
                    println!("--------------------------------");
                    if let Some(topic0) = log.topics().first() {
                        // Find event by selector
                        if let Some(event) =
                            abi_for_decode.events().find(|e| e.selector() == *topic0)
                        {
                            println!("🔥 {}  EVENT DETECTED! on Monitor {}", event.name, name);
                        }
                    }
                    println!("Block Number: {:?}", log.block_number);
                    println!("Log Data: {:?}", log.data());
                    println!("--------------------------------");
                })
                .await
            {
                eprintln!("Monitor {} crashed: {:?}", monitor_cfg.name, e);
            }
        });

        handles.push(handle);
    }

    println!("All monitors launched. Waiting...");

    // join_all waits for every handle in the vector to finish.
    join_all(handles).await;

    Ok(())
}
