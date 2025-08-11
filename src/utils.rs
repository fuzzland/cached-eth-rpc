use reqwest::Url;
use serde::Serialize;
use serde_json::{json, Value};

pub async fn get_chain_id(client: &reqwest::Client, rpc_url: &str) -> eyre::Result<u64> {
    let request_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_chainId",
        "params": [],
        "id": 1
    });

    let response = client.post(rpc_url).json(&request_payload).send().await?;

    let json: Value = response.json().await?;
    match json["result"].as_str() {
        Some(chain_id) => Ok(u64::from_str_radix(&chain_id[2..], 16)?),
        None => Err(eyre::eyre!("fail to get chain id: {json}")),
    }
}

pub async fn do_rpc_request<T: Serialize + ?Sized>(
    client: &reqwest::Client,
    rpc_url: Url,
    body: &T,
) -> eyre::Result<Value> {
    let result = client
        .post(rpc_url)
        .json(body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(result)
}
