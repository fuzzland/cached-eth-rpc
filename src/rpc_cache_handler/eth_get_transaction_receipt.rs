use alloy_primitives::B256;
use eyre::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionReceipt"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::Exact(1))?;
        let tx_hash: B256 = serde_json::from_value(params[0].clone())
            .context("params[0] is not a valid transaction hash")?;

        Ok(Some(format!("{tx_hash:#x}")))
    }

    fn extract_cache_value(&self, result: &Value) -> eyre::Result<(bool, String)> {
        common::extract_transaction_cache_value(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    static HANDLER: Handler = Handler;

    #[test]
    fn test_invalid_params_len() {
        let params = json!([]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "expected 1 params, got 0"
        );

        let params = json!([
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            123
        ]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "expected 1 params, got 2"
        );
    }

    #[test]
    fn test_normal_case() {
        let params = json!(["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(
            cache_key,
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }
}
