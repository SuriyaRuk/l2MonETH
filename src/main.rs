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
    let response = client
        .post(&rpc)
        .json(&payload)
        .send()
        .await?;

    let body = response.text().await?;
    let block_response: BlockNumberResponse = serde_json::from_str(&body)?;

    println!("currentTime: {} ID {} Block Number (Hex): {}", 
             current_time, id, block_response.result);

    let block_number = i64::from_str_radix(&block_response.result[2..], 16)?;
    
    println!("currentTime: {} ID {} Block Number (Decimal): {}", 
             current_time, id, block_number);

    Ok(block_number)
}

#[derive(Serialize)]
struct BlockResponse {
    block_number_hex: String,
    block_number_decimal: i64,
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
            return Ok(warp::reply::with_status(warp::reply::json(&error_response), warp::http::StatusCode::INTERNAL_SERVER_ERROR));
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
            return Ok(warp::reply::with_status(warp::reply::json(&error_response), warp::http::StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    let block_diff = block_number_second - block_number_first;
    
    // Return both hex and decimal formats
    let response = BlockResponse {
        block_number_hex: format!("0x{:x}", block_number_second),
        block_number_decimal: block_number_second,
        status: if block_diff != 0 { "synced".to_string() } else { "not_synced".to_string() },
    };
    
    if block_diff != 0 {
        Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
    } else {
        Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::SERVICE_UNAVAILABLE))
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "9999".to_string())
        .parse::<u16>()
        .unwrap_or(9999);

    let routes = warp::path::end()
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(|query_params: std::collections::HashMap<String, String>| {
            let rpc_url = query_params.get("rpc").cloned();
            check_sync(rpc_url)
        });

    println!("Starting server on port {}", port);

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}