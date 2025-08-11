use serde_json::Value;

use crate::rpc_cache_handler::{eth_call, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler {
    inner: eth_call::Handler,
}

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_estimateGas"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        self.inner.extract_cache_key(params)
    }
}
