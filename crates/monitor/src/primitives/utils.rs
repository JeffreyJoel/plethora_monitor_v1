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
