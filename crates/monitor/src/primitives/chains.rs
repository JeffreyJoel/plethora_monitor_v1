//! Chain configuration utilities - maps chain names to default public RPC URLs.

use alloy_chains::NamedChain;
use std::env;
use std::str::FromStr;

/// Normalizes chain name to snake_case (e.g., "base-sepolia" -> "base_sepolia")
pub fn normalize_chain_name(chain: &str) -> String {
    let mut result = String::with_capacity(chain.len() + 4);
    let mut prev_was_separator = true;

    for c in chain.chars() {
        if c == '-' || c == '_' || c == ' ' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            prev_was_separator = true;
        } else if c.is_uppercase() {
            if !result.is_empty() && !prev_was_separator && !result.ends_with('_') {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_was_separator = false;
        } else {
            result.push(c.to_ascii_lowercase());
            prev_was_separator = false;
        }
    }

    result
}

pub fn get_default_rpc_url(chain_name: &str) -> Option<&'static str> {
    let normalized = normalize_chain_name(chain_name);

    match normalized.as_str() {
        "mainnet" | "ethereum" | "ethereum_mainnet" => Some("https://ethereum-rpc.publicnode.com"),
        "sepolia" | "ethereum_sepolia" => Some("https://ethereum-sepolia-rpc.publicnode.com"),

        "base" | "base_mainnet" => Some("https://base-rpc.publicnode.com"),
        "base_sepolia" => Some("https://base-sepolia-rpc.publicnode.com"),

        "polygon" | "polygon_mainnet" | "matic" | "matic_mainnet" => {
            Some("https://polygon-rpc.com")
        }
        "polygon_amoy" | "amoy" | "matic_amoy" => Some("https://rpc-amoy.polygon.technology"),

        "arbitrum" | "arbitrum_one" | "arbitrum_mainnet" => Some("https://arb1.arbitrum.io/rpc"),
        "arbitrum_sepolia" => Some("https://sepolia-rollup.arbitrum.io/rpc"),

        "optimism" | "optimism_mainnet" | "op" | "op_mainnet" => {
            Some("https://mainnet.optimism.io")
        }
        "optimism_sepolia" | "op_sepolia" => Some("https://sepolia.optimism.io"),

        "avalanche" | "avalanche_c" | "avax" | "avalanche_mainnet" => {
            Some("https://api.avax.network/ext/bc/C/rpc")
        }
        "avalanche_fuji" | "fuji" | "avax_fuji" => {
            Some("https://api.avax-test.network/ext/bc/C/rpc")
        }

        "bsc" | "bnb" | "binance" | "bsc_mainnet" | "bnb_mainnet" => {
            Some("https://bsc-rpc.publicnode.com")
        }
        "bsc_testnet" | "bnb_testnet" | "binance_testnet" => {
            Some("https://bsc-testnet-rpc.publicnode.com")
        }

        "celo" | "celo_mainnet" => Some("https://forno.celo.org"),
        "celo_alfajores" | "alfajores" => Some("https://alfajores-forno.celo-testnet.org"),

        "zksync" | "zksync_era" | "zksync_mainnet" => Some("https://mainnet.era.zksync.io"),
        "zksync_sepolia" => Some("https://sepolia.era.zksync.dev"),

        "linea" | "linea_mainnet" => Some("https://rpc.linea.build"),
        "linea_sepolia" => Some("https://rpc.sepolia.linea.build"),

        "scroll" | "scroll_mainnet" => Some("https://rpc.scroll.io"),
        "scroll_sepolia" => Some("https://sepolia-rpc.scroll.io"),

        "mantle" | "mantle_mainnet" => Some("https://rpc.mantle.xyz"),
        "mantle_sepolia" => Some("https://rpc.sepolia.mantle.xyz"),

        "unichain" | "unichain_mainnet" => Some("https://mainnet.unichain.org"),
        "unichain_sepolia" => Some("https://sepolia.unichain.org"),

        _ => None,
    }
}

/// Returns the Infura RPC URL for a chain if INFURA_API_KEY is set and chain is supported.
pub fn get_infura_url(chain_name: &str) -> Option<String> {
    let api_key = env::var("INFURA_API_KEY").ok()?;
    let normalized = normalize_chain_name(chain_name);

    let subdomain = match normalized.as_str() {
        "mainnet" | "ethereum" | "ethereum_mainnet" => "mainnet",
        "sepolia" | "ethereum_sepolia" => "sepolia",
        "holesky" | "ethereum_holesky" => "holesky",

        "polygon" | "polygon_mainnet" | "matic" => "polygon-mainnet",
        "polygon_amoy" | "amoy" => "polygon-amoy",

        "arbitrum" | "arbitrum_one" | "arbitrum_mainnet" => "arbitrum-mainnet",
        "arbitrum_sepolia" => "arbitrum-sepolia",

        "optimism" | "optimism_mainnet" | "op" => "optimism-mainnet",
        "optimism_sepolia" | "op_sepolia" => "optimism-sepolia",

        "base" | "base_mainnet" => "base-mainnet",
        "base_sepolia" => "base-sepolia",

        "linea" | "linea_mainnet" => "linea-mainnet",
        "linea_sepolia" => "linea-sepolia",

        "avalanche" | "avalanche_c" | "avax" => "avalanche-mainnet",
        "avalanche_fuji" | "fuji" => "avalanche-fuji",

        "celo" | "celo_mainnet" => "celo-mainnet",
        "celo_alfajores" | "alfajores" => "celo-alfajores",

        "bsc" | "bnb" | "binance" | "bsc_mainnet" => "bsc-mainnet",
        "bsc_testnet" | "bnb_testnet" => "bsc-testnet",

        "zksync" | "zksync_era" | "zksync_mainnet" => "zksync-mainnet",
        "zksync_sepolia" => "zksync-sepolia",

        "scroll" | "scroll_mainnet" => "scroll-mainnet",
        "scroll_sepolia" => "scroll-sepolia",

        "mantle" | "mantle_mainnet" => "mantle-mainnet",
        "mantle_sepolia" => "mantle-sepolia",

        "unichain" | "unichain_mainnet" => "unichain-mainnet",
        "unichain_sepolia" => "unichain-sepolia",

        _ => return None,
    };

    Some(format!("https://{}.infura.io/v3/{}", subdomain, api_key))
}

pub fn get_chain_id(chain_name: &str) -> Option<u64> {
    NamedChain::from_str(chain_name)
        .ok()
        .map(|chain| chain as u64)
}

pub fn supported_chains() -> &'static [&'static str] {
    &[
        "ethereum",
        "sepolia",
        "holesky",
        "base",
        "base_sepolia",
        "polygon",
        "polygon_amoy",
        "arbitrum",
        "arbitrum_sepolia",
        "optimism",
        "optimism_sepolia",
        "avalanche",
        "avalanche_fuji",
        "bsc",
        "bsc_testnet",
        "celo",
        "celo_alfajores",
        "zksync",
        "zksync_sepolia",
        "linea",
        "linea_sepolia",
        "scroll",
        "scroll_sepolia",
        "mantle",
        "mantle_sepolia",
        "unichain",
        "unichain_sepolia",
    ]
}

pub fn is_supported_chain(chain_name: &str) -> bool {
    get_default_rpc_url(chain_name).is_some()
}
