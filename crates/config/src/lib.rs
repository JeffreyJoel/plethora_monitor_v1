//! # Config Structure
//!
//! This module defines the configuration structures for monitoring smart contract events.
//! It uses the `config` crate to load settings from a configuration file named `Settings.toml`.
//! The configuration is deserialized into typed structures (`AppConfig`, `MonitorConfig`) that
//! specify which contracts to monitor, their addresses, RPC endpoints, and the specific events of interest.

use monitor::filter::MonitorRule;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub monitors: Vec<MonitorConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub rpc_url: String,
    pub chain: String,
    pub address: String, // We make use of the String type for config, we'll conver to Address during implementation
    pub events: Option<Vec<String>>,
    pub functions: Option<Vec<MonitorRule>>,
}

impl AppConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder().add_source(config::File::with_name("Settings"));

        builder.build()?.try_deserialize()
    }
}
