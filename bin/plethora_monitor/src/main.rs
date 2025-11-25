use alloy::primitives::address;
use monitor::PollingMonitor;
use std::fs::File;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let rpc_url = "https://sepolia.base.org";

    let contract_addr = address!("0x036CbD53842c5426634e7929541eC2318f3dCF7e"); //USDC address on Base Sepolia

    let abi_file = File::open("data/erc_20.json")?;
    let abi: serde_json::Value = serde_json::from_reader(abi_file)?;

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
