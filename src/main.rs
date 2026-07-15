use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
use warp::Filter;
use num_bigint::BigUint;
use num_traits::Num;
use std::str::FromStr;

/// Strip an optional hex prefix from a hex string without panicking.
/// Slicing `s[2..]` blindly panics when the RPC returns a value shorter than
/// two bytes (e.g. an empty string or a single digit); this handles that safely.
fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s)
}

/// Validate an Ethereum address: `0x` followed by exactly 40 hex digits.
fn is_valid_eth_address(address: &str) -> bool {
    let hex = match address.strip_prefix("0x").or_else(|| address.strip_prefix("0X")) {
        Some(h) => h,
        None => return false,
    };
    hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
    id: u32,
}

#[derive(Deserialize)]
struct BlockNumberResponse {
    jsonrpc: String,
    id: u32,
    result: String,
}

async fn get_block_number(rpc_url: Option<String>) -> Result<i64, Box<dyn std::error::Error>> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let id = rand::thread_rng().gen_range(1..=100);

    let payload = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "eth_blockNumber".to_string(),
        params: vec![],
        id,
    };

    let rpc = rpc_url.unwrap_or_else(|| "http://127.0.0.1:8545".to_string());

    let client = Client::new();
    let response = client.post(&rpc).json(&payload).send().await?;

    let body = response.text().await?;
    let block_response: BlockNumberResponse = serde_json::from_str(&body)?;

    println!(
        "currentTime: {} ID {} Block Number (Hex): {}",
        current_time, id, block_response.result
    );

    let block_number = i64::from_str_radix(strip_hex_prefix(&block_response.result), 16)?;

    println!(
        "currentTime: {} ID {} Block Number (Decimal): {}",
        current_time, id, block_number
    );

    Ok(block_number)
}

async fn get_block_by_tag(
    rpc_url: Option<String>,
    tag: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let id = rand::thread_rng().gen_range(1..=100);

    let payload = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "eth_getBlockByNumber".to_string(),
        params: vec![
            serde_json::Value::String(tag.to_string()),
            serde_json::Value::Bool(false),
        ],
        id,
    };

    let rpc = rpc_url.unwrap_or_else(|| "http://127.0.0.1:8545".to_string());

    let client = Client::new();
    let response = client.post(&rpc).json(&payload).send().await?;

    let body = response.text().await?;
    let json_response: serde_json::Value = serde_json::from_str(&body)?;

    if let Some(result) = json_response.get("result") {
        if let Some(number_hex) = result.get("number").and_then(|n| n.as_str()) {
            println!(
                "currentTime: {} ID {} Block {} Number (Hex): {}",
                current_time, id, tag, number_hex
            );

            let block_number = i64::from_str_radix(strip_hex_prefix(number_hex), 16)?;

            println!(
                "currentTime: {} ID {} Block {} Number (Decimal): {}",
                current_time, id, tag, block_number
            );

            return Ok(block_number);
        }
    }

    Err("Block not found or invalid response".into())
}

#[derive(Serialize)]
struct BlockResponse {
    block_number_hex: String,
    block_number_decimal: i64,
    status: String,
}

#[derive(Serialize)]
struct BlockDiffResponse {
    finalized_block: i64,
    latest_block: i64,
    difference: i64,
    finalized_hex: String,
    latest_hex: String,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct BalanceResponse {
    jsonrpc: String,
    id: u32,
    result: Option<String>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Serialize)]
struct CheckBalanceResponse {
    address: String,
    balance: String,
    balance_decimal: String,
    alert_threshold: String,
    status: String,
}

async fn check_sync(rpc_url: Option<String>) -> Result<impl warp::Reply, warp::Rejection> {
    let block_number_first = match get_block_number(rpc_url.clone()).await {
        Ok(num) => num,
        Err(_) => {
            let error_response = BlockResponse {
                block_number_hex: "".to_string(),
                block_number_decimal: 0,
                status: "error".to_string(),
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    sleep(Duration::from_secs(30)).await;

    let block_number_second = match get_block_number(rpc_url).await {
        Ok(num) => num,
        Err(_) => {
            let error_response = BlockResponse {
                block_number_hex: "".to_string(),
                block_number_decimal: 0,
                status: "error".to_string(),
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let block_diff = block_number_second - block_number_first;

    // Return both hex and decimal formats
    let response = BlockResponse {
        block_number_hex: format!("0x{:x}", block_number_second),
        block_number_decimal: block_number_second,
        status: if block_diff != 0 {
            "synced".to_string()
        } else {
            "not_synced".to_string()
        },
    };

    if block_diff != 0 {
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

async fn finalized_latest_diff(
    rpc_url: Option<String>,
    diff: Option<i64>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let finalized_block = match get_block_by_tag(rpc_url.clone(), "finalized").await {
        Ok(num) => num,
        Err(_) => {
            let error_response = BlockDiffResponse {
                finalized_block: 0,
                latest_block: 0,
                difference: 0,
                finalized_hex: "".to_string(),
                latest_hex: "".to_string(),
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let latest_block = match get_block_by_tag(rpc_url, "latest").await {
        Ok(num) => num,
        Err(_) => {
            let error_response = BlockDiffResponse {
                finalized_block,
                latest_block: 0,
                difference: 0,
                finalized_hex: format!("0x{:x}", finalized_block),
                latest_hex: "".to_string(),
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let difference = latest_block - finalized_block;

    let response = BlockDiffResponse {
        finalized_block,
        latest_block,
        difference,
        finalized_hex: format!("0x{:x}", finalized_block),
        latest_hex: format!("0x{:x}", latest_block),
    };

    if difference < diff.unwrap_or(0) {
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

async fn get_balance(
    rpc_url: Option<String>,
    address: String,
) -> Result<BigUint, Box<dyn std::error::Error>> {
    let id = rand::thread_rng().gen_range(1..=100);

    let payload = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "eth_getBalance".to_string(),
        params: vec![
            serde_json::Value::String(address),
            serde_json::Value::String("latest".to_string()),
        ],
        id,
    };

    let rpc = rpc_url.unwrap_or_else(|| "http://127.0.0.1:8545".to_string());

    let client = Client::new();
    let response = client.post(&rpc).json(&payload).send().await?;

    let body = response.text().await?;
    let balance_response: BalanceResponse = serde_json::from_str(&body)?;

    // Surface the node's JSON-RPC error rather than a generic "missing field" serde error.
    if let Some(err) = balance_response.error {
        return Err(format!("RPC error {}: {}", err.code, err.message).into());
    }

    let result = balance_response
        .result
        .ok_or("RPC response contained neither result nor error")?;

    let balance = BigUint::from_str_radix(strip_hex_prefix(&result), 16)
        .map_err(|e| format!("Failed to parse balance hex: {}", e))?;

    Ok(balance)
}

async fn check_balance(
    rpc_url: Option<String>,
    address: String,
    alert: Option<String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Reject a missing or malformed address up front rather than forwarding it to
    // the RPC node and surfacing an opaque deserialization error.
    if !is_valid_eth_address(&address) {
        let error_response = CheckBalanceResponse {
            address: address.clone(),
            balance: "0x0".to_string(),
            balance_decimal: "0".to_string(),
            alert_threshold: alert.clone().unwrap_or_else(|| "0".to_string()),
            status: "error: invalid or missing address (expected 0x + 40 hex digits)"
                .to_string(),
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&error_response),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    // Parse the alert threshold before doing any work. A malformed value must be
    // rejected loudly — silently defaulting to 0 would permanently disable the alert.
    let alert_threshold = match &alert {
        Some(a) => match BigUint::from_str(a) {
            Ok(t) => t,
            Err(_) => {
                let error_response = CheckBalanceResponse {
                    address: address.clone(),
                    balance: "0x0".to_string(),
                    balance_decimal: "0".to_string(),
                    alert_threshold: a.clone(),
                    status: format!("error: invalid alert threshold '{}'", a),
                };
                return Ok(warp::reply::with_status(
                    warp::reply::json(&error_response),
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }
        },
        None => BigUint::from(0u32),
    };

    let balance = match get_balance(rpc_url, address.clone()).await {
        Ok(bal) => bal,
        Err(e) => {
            let error_response = CheckBalanceResponse {
                address: address.clone(),
                balance: "0x0".to_string(),
                balance_decimal: "0".to_string(),
                alert_threshold: alert_threshold.to_string(),
                status: format!("error: {}", e),
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let response = CheckBalanceResponse {
        address: address.clone(),
        balance: format!("0x{:x}", balance),
        balance_decimal: balance.to_string(),
        alert_threshold: alert_threshold.to_string(),
        status: if alert_threshold < balance {
            "balance_sufficient".to_string()
        } else {
            "balance_low".to_string()
        },
    };

    // Must mirror the `status` field above: "balance_low" (alert_threshold >= balance)
    // maps to an error status so monitors keying off the HTTP code alert at the threshold.
    if alert_threshold >= balance {
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&response),
            warp::http::StatusCode::OK,
        ))
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "9999".to_string())
        .parse::<u16>()
        .unwrap_or(9999);

    let sync_route = warp::path::end()
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(|query_params: std::collections::HashMap<String, String>| {
            let rpc_url = query_params.get("rpc").cloned();
            check_sync(rpc_url)
        });

    let diff_route = warp::path("finalized_latest_diff")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(|query_params: std::collections::HashMap<String, String>| {
            let rpc_url = query_params.get("rpc").cloned();
            let diff = query_params.get("diff").and_then(|d| d.parse::<i64>().ok());
            finalized_latest_diff(rpc_url, diff)
        });

    let balance_route = warp::path("check_balance")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(|query_params: std::collections::HashMap<String, String>| {
            let rpc_url = query_params.get("rpc").cloned();
            let address = query_params.get("address").cloned().unwrap_or_default();
            let alert = query_params.get("alert").cloned();
            check_balance(rpc_url, address, alert)
        });

    let routes = sync_route.or(diff_route).or(balance_route);

    println!("Starting server on port {}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_get_block_number_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":"0x1a2b3c"}"#)
            .create_async()
            .await;

        let result = get_block_number(Some(server.url())).await;
        mock.assert_async().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x1a2b3c);
    }

    #[tokio::test]
    async fn test_get_block_number_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .create_async()
            .await;

        let result = get_block_number(Some(server.url())).await;
        mock.assert_async().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_block_by_tag_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"number":"0xabcdef"}}"#)
            .create_async()
            .await;

        let result = get_block_by_tag(Some(server.url()), "latest").await;
        mock.assert_async().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0xabcdef);
    }

    #[tokio::test]
    async fn test_get_block_by_tag_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .create_async()
            .await;

        let result = get_block_by_tag(Some(server.url()), "latest").await;
        mock.assert_async().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_balance_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":"0xde0b6b3a7640000"}"#)
            .create_async()
            .await;

        let result = get_balance(Some(server.url()), "0x123456789".to_string()).await;
        mock.assert_async().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BigUint::from(1000000000000000000u128));
    }

    #[tokio::test]
    async fn test_get_balance_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .create_async()
            .await;

        let result = get_balance(Some(server.url()), "0x123456789".to_string()).await;
        mock.assert_async().await;

        assert!(result.is_err());
    }

    #[test]
    fn test_rpc_request_serialization() {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: vec![],
            id: 1,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("eth_blockNumber"));
        assert!(json.contains("2.0"));
    }

    #[test]
    fn test_block_number_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":"0x123"}"#;
        let response: BlockNumberResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert_eq!(response.result, "0x123");
    }

    #[test]
    fn test_balance_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":"0xde0b6b3a7640000"}"#;
        let response: BalanceResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert_eq!(response.result.as_deref(), Some("0xde0b6b3a7640000"));
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_get_balance_rpc_error_surfaced() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"invalid address"}}"#,
            )
            .create_async()
            .await;

        let result = get_balance(Some(server.url()), "0x123456789".to_string()).await;
        mock.assert_async().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid address"));
    }

    #[test]
    fn test_block_response_serialization() {
        let response = BlockResponse {
            block_number_hex: "0x123".to_string(),
            block_number_decimal: 291,
            status: "synced".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("0x123"));
        assert!(json.contains("291"));
        assert!(json.contains("synced"));
    }

    #[test]
    fn test_large_balance_parsing() {
        let large_hex = "204fce5e3e25026110000000";
        let result = BigUint::from_str_radix(large_hex, 16);
        assert!(result.is_ok());
        let balance = result.unwrap();
        let balance_str = balance.to_string();
        assert!(balance_str.len() > 20);
    }

    #[test]
    fn test_check_balance_response_with_large_values() {
        let response = CheckBalanceResponse {
            address: "0x123456789".to_string(),
            balance: "0x204fce5e3e25026110000000".to_string(),
            balance_decimal: "99999999999999999999999999999999999999".to_string(),
            alert_threshold: "1000000000000000000".to_string(),
            status: "balance_sufficient".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("99999999999999999999999999999999999999"));
    }
}
