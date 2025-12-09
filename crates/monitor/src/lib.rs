pub mod events;
pub mod tx;

pub use events::EventMonitor;
pub use tx::TransactionMonitor;

use alloy::json_abi::JsonAbi;
use alloy::network::Ethereum;
use alloy::primitives::Address;
use alloy::providers::{ProviderBuilder, RootProvider};
use serde::{Deserialize, Serialize};
use tokio::fs;

pub type HttpProvider = RootProvider<Ethereum>;

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorState {
    pub last_processed_block: u64,
}

#[derive(Clone)]
pub struct PollingMonitor {
    pub provider: HttpProvider,
    pub contract_address: Address,
    pub contract_abi: JsonAbi,
    pub state_file_path: String,
}

impl PollingMonitor {
    pub fn new(
        rpc_url: &str,
        contract_address: Address,
        contract_abi: JsonAbi,
        state_file_path: &str,
    ) -> Result<Self, anyhow::Error> {
        let url = rpc_url.parse()?;
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .connect_http(url);

        Ok(Self {
            provider,
            contract_address,
            contract_abi,
            state_file_path: state_file_path.to_string(),
        })
    }

    pub async fn load_state(&self) -> Option<u64> {
        let content = fs::read_to_string(&self.state_file_path).await.ok()?;
        let state: MonitorState = serde_json::from_str(&content).ok()?;
        Some(state.last_processed_block)
    }

    pub async fn save_state(&self, block_number: u64) -> Result<(), anyhow::Error> {
        let state = MonitorState {
            last_processed_block: block_number,
        };
        let content = serde_json::to_string_pretty(&state)?;
        fs::write(&self.state_file_path, content).await?;
        Ok(())
    }
}
