use anyhow::{Result, anyhow};
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

/// Check balance via RPC for different chains
pub fn check_balance(rpc_url: &str, address: &str, chain_type: chains::ChainType) -> Result<Option<String>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    match chain_type {
        chains::ChainType::EVM |
        chains::ChainType::PiNetwork |
        chains::ChainType::Tron |
        chains::ChainType::Dogecoin => {
            check_evm_balance(&client, rpc_url, address)
        }
        chains::ChainType::Solana => {
            check_solana_balance(&client, rpc_url, address)
        }
        chains::ChainType::Sui |
        chains::ChainType::Aptos => {
            check_ed25519_balance(&client, rpc_url, address)
        }
    }
}

/// Check EVM balance (ETH, Base, Polygon, etc.)
fn check_evm_balance(client: &Client, rpc_url: &str, address: &str) -> Result<Option<String>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()?;

    let json: Value = response.json()?;

    // Parse balance
    if let Some(result) = json.get("result") {
        if let Some(hex_balance) = result.as_str() {
            if let Some(balance_str) = hex_balance.strip_prefix("0x") {
                let balance = u128::from_str_radix(balance_str, 16)
                    .map_err(|e| anyhow!("Failed to parse balance: {}", e))?;

                // Convert to wei/ether (assuming 18 decimals)
                let ethers = balance as f64 / 1e18;

                if ethers > 0.0 {
                    Ok(Some(format!("{} ETH", ethers)))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Check Solana balance
fn check_solana_balance(client: &Client, rpc_url: &str, address: &str) -> Result<Option<String>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getBalance",
        "params": [address],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()?;

    let json: Value = response.json()?;

    // Parse balance
    if let Some(result) = json.get("result") {
        if let Some(value_obj) = result.as_object() {
            if let Some(value) = value_obj.get("value") {
                if let Some(balance_str) = value.as_str() {
                    let balance = balance_str.parse::<u64>()
                        .map_err(|e| anyhow!("Failed to parse balance: {}", e))?;

                    // Convert lamports to SOL (1 SOL = 1,000,000,000 lamports)
                    let sols = balance as f64 / 1_000_000_000.0;

                    if sols > 0.0 {
                        Ok(Some(format!("{} SOL", sols)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Check Ed25519 balance (Sui, Aptos)
fn check_ed25519_balance(client: &Client, rpc_url: &str, address: &str) -> Result<Option<String>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "suix_getBalance",
        "params": [address],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()?;

    let json: Value = response.json()?;

    // Parse balance
    if let Some(result) = json.get("result") {
        if let Some(total_balance) = result.as_str() {
            let balance = total_balance.parse::<u64>()
                .map_err(|e| anyhow!("Failed to parse balance: {}", e))?;

            // Sui/Aptos use different units, just show raw value
            if balance > 0 {
                Ok(Some(format!("{} tokens", balance)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
