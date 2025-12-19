use alloy::dyn_abi::DynSolValue;
use alloy::json_abi::JsonAbi;
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, hex},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
use alloy_chains::{Chain, NamedChain};
use foundry_block_explorers::Client;
use std::env;
use std::str::FromStr;

pub async fn fetch_abi(
    chain_name: &str,
    address: Address,
    rpc_url: &str,
) -> Result<JsonAbi, anyhow::Error> {
    let key = env::var("ETHERSCAN_API_KEY")?;
    let named_chain = NamedChain::from_str(chain_name)?;
    let chain = Chain::from(named_chain);
    let client = Client::new(chain, &key)?;
    let addr = address;

    // Check the contract address whether its a proxy contract and then return the impl contract address
    // and get the abi for that
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let tx = TransactionRequest::default()
        .with_to(addr)
        .with_input("0x5c60da1b".parse::<Bytes>()?); // implementation function selector

    let target_addr = match provider.call(tx).await {
        Ok(bytes) if bytes.len() >= 32 => Address::from_slice(&bytes[12..32]),
        _ => addr,
    };

    let abi = client.contract_abi(target_addr).await?;

    Ok(abi)
}

pub fn format_value(val: &DynSolValue) -> String {
    match val {
        DynSolValue::Address(addr) => addr.to_string(),
        DynSolValue::Uint(i, _) => i.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::String(s) => s.clone(),
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::Bytes(b) => hex::encode(b),
        DynSolValue::FixedBytes(b, _) => hex::encode(b),

        DynSolValue::Array(arr) | DynSolValue::FixedArray(arr) => {
            let elements: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", elements.join(", "))
        }
        DynSolValue::Tuple(tuple) => {
            let elements: Vec<String> = tuple.iter().map(format_value).collect();
            format!("({})", elements.join(", "))
        }

        _ => format!("{:?}", val),
    }
}

/// Check if an argument value satisfies a condition with an operator
pub fn check_argument_condition(
    arg_value: &alloy::dyn_abi::DynSolValue,
    operator: &crate::primitives::models::Operator,
    expected_value: &str,
) -> bool {
    use crate::primitives::models::Operator;
    use alloy::dyn_abi::DynSolValue;
    use alloy::primitives::Address;
    use std::str::FromStr;

    match arg_value {
        DynSolValue::Address(addr) => match operator {
            Operator::Eq => {
                if let Ok(expected_addr) = Address::from_str(expected_value) {
                    return addr == &expected_addr;
                }
                false
            }
            Operator::Contains => addr
                .to_string()
                .to_lowercase()
                .contains(&expected_value.to_lowercase()),
            _ => false,
        },
        DynSolValue::Uint(val, _) => {
            let val_str = val.to_string();
            match operator {
                Operator::Eq => val_str == expected_value,
                Operator::Gt => val_str
                    .parse::<u128>()
                    .ok()
                    .and_then(|l| expected_value.parse::<u128>().ok().map(|r| l > r))
                    .unwrap_or(false),
                Operator::Lt => val_str
                    .parse::<u128>()
                    .ok()
                    .and_then(|l| expected_value.parse::<u128>().ok().map(|r| l < r))
                    .unwrap_or(false),
                _ => false,
            }
        }
        DynSolValue::Int(val, _) => {
            let val_str = val.to_string();
            match operator {
                Operator::Eq => val_str == expected_value,
                Operator::Gt => val_str
                    .parse::<i128>()
                    .ok()
                    .and_then(|l| expected_value.parse::<i128>().ok().map(|r| l > r))
                    .unwrap_or(false),
                Operator::Lt => val_str
                    .parse::<i128>()
                    .ok()
                    .and_then(|l| expected_value.parse::<i128>().ok().map(|r| l < r))
                    .unwrap_or(false),
                _ => false,
            }
        }
        DynSolValue::String(s) => match operator {
            Operator::Eq => s == expected_value,
            Operator::Contains => s.contains(expected_value),
            Operator::Gt => s
                .parse::<u128>()
                .ok()
                .and_then(|l| expected_value.parse::<u128>().ok().map(|r| l > r))
                .unwrap_or(false),
            Operator::Lt => s
                .parse::<u128>()
                .ok()
                .and_then(|l| expected_value.parse::<u128>().ok().map(|r| l < r))
                .unwrap_or(false),
        },
        DynSolValue::Bool(b) => match expected_value.to_lowercase().as_str() {
            "true" => *b,
            "false" => !*b,
            _ => false,
        },
        _ => false,
    }
}
