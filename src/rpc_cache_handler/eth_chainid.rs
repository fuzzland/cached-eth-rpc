use serde_json::Value;

use crate::rpc_cache_handler::RpcCacheHandler;

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_chainId"
    }

    fn extract_cache_key(&self, _: &Value) -> eyre::Result<Option<String>> {
        Ok(Some("static".to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    static HANDLER: Handler = Handler;

    #[test]
    fn test() {
        let params = json!([]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "static");
    }
}
