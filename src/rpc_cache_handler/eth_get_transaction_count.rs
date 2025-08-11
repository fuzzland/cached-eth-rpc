use serde_json::Value;

use crate::rpc_cache_handler::RpcCacheHandler;

#[derive(Default, Clone)]
pub struct Handler {
    inner: super::eth_get_balance::Handler,
}

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionCount"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        self.inner.extract_cache_key(params)
    }
}
