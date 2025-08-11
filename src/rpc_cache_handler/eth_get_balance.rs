use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getBalance"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        common::extract_address_cache_key(params)
    }
}
