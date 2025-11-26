use alloy::primitives::address;
use alloy_chains::{Chain, NamedChain};
use anyhow::Context;
use dotenvy::dotenv;
use foundry_block_explorers::Client;
use monitor::PollingMonitor;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();
    let rpc_url = "https://sepolia.base.org";

    let chain = Chain::from(NamedChain::BaseSepolia);

    let etherscan_api_key =
        env::var("ETHERSCAN_API_KEY").context("ETHERSCAN_API_KEY must be set in your .env file")?;

    let client = Client::new(chain, &etherscan_api_key)?;

    let contract_addr = address!("0x036CbD53842c5426634e7929541eC2318f3dCF7e"); //USDC address on Base Sepolia

    let abi = client
        .contract_abi("0x036CbD53842c5426634e7929541eC2318f3dCF7e".parse()?)
        .await?;

    let monitor = PollingMonitor::new(rpc_url, contract_addr, abi)?;

    println!("Start your engine (Polling Mode)...");

    monitor
        .monitor_events_polling(&["Transfer"], |log| {
            println!("--------------------------------");
            println!("🔥 EVENT DETECTED!");
            println!("Block Number: {:?}", log.block_number);
            println!("Log Data: {:?}", log.data());
            println!("--------------------------------");
        })
        .await?;

    Ok(())
}
