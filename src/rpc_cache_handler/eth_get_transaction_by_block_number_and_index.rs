use alloy_primitives::U64;
use eyre::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionByBlockNumberAndIndex"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::Exact(2))?;

        let block_number = common::extract_and_format_block_number(&params[0])
            .context("params[0] is not a valid block number")?;
        let block_number = match block_number {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        let tx_index: U64 =
            serde_json::from_value(params[1].clone()).context("params[1] is not a valid index")?;

        Ok(Some(format!("{block_number}-{tx_index}")))
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
            "expected 2 params, got 0"
        );

        let params = json!(["0x12345"]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "expected 2 params, got 1"
        );

        let params = json!(["0x12345", 123, 456]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "expected 2 params, got 3"
        );
    }

    #[test]
    fn test_normal_case() {
        let params = json!(["0x12345", 0]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12345-0");

        let params = json!(["0x12345", 1234]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12345-1234");

        let params = json!(["0x12345", "0x1234"]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12345-4660");
    }

    #[test]
    fn test_not_fixed_block() {
        let params = json!(["earliest", 1234]);
        assert_eq!(HANDLER.extract_cache_key(&params).unwrap(), None);
    }

    #[test]
    fn test_invalid_tx_index() {
        let params = json!(["0x12345", "gg"]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "params[1] is not a valid index"
        );
    }
}
