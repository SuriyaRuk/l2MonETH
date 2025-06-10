use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
use warp::Filter;

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

    let block_number = i64::from_str_radix(&block_response.result[2..], 16)?;

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

            let block_number = i64::from_str_radix(&number_hex[2..], 16)?;

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
struct BalanceResponse {
    jsonrpc: String,
    id: u32,
    result: String,
}

#[derive(Serialize)]
struct CheckBalanceResponse {
    address: String,
    balance: String,
    balance_decimal: u128,
    alert_threshold: u128,
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
) -> Result<u128, Box<dyn std::error::Error>> {
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

    let balance = u128::from_str_radix(&balance_response.result[2..], 16)?;

    Ok(balance)
}

async fn check_balance(
    rpc_url: Option<String>,
    address: String,
    alert: Option<u128>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let balance = match get_balance(rpc_url, address.clone()).await {
        Ok(bal) => bal,
        Err(_) => {
            let error_response = CheckBalanceResponse {
                address: address.clone(),
                balance: "0x0".to_string(),
                balance_decimal: 0,
                alert_threshold: alert.unwrap_or(0),
                status: "error".to_string(),
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let alert_threshold = alert.unwrap_or(0);

    let response = CheckBalanceResponse {
        address: address.clone(),
        balance: format!("0x{:x}", balance),
        balance_decimal: balance,
        alert_threshold,
        status: if alert_threshold < balance {
            "balance_sufficient".to_string()
        } else {
            "balance_low".to_string()
        },
    };

    if alert_threshold < balance {
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
            let alert = query_params
                .get("alert")
                .and_then(|a| a.parse::<u128>().ok());
            check_balance(rpc_url, address, alert)
        });

    let routes = sync_route.or(diff_route).or(balance_route);

    println!("Starting server on port {}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
