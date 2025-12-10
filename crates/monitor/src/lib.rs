pub mod events;
pub mod filter;
pub mod tx;

pub use events::EventMonitor;
pub use tx::TransactionMonitor;

use alloy::json_abi::JsonAbi;
use alloy::network::AnyNetwork;
use alloy::primitives::Address;
use alloy::providers::{ProviderBuilder, RootProvider};
use serde::{Deserialize, Serialize};

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
    pub fn new(rpc_url: &str, contract_address: Address, contract_abi: JsonAbi) -> Result<Self, anyhow::Error> {
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
}
