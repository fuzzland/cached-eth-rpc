use eyre::{Context, ContextCompat as _};
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getBlockByNumber"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::AtLeast(1))?;

        let block_number = common::extract_and_format_block_number(&params[0])
            .context("params[0] not a valid block number")?;
        let block_tag = match block_number {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        if params.len() > 1 {
            let transaction_detail = params[1].as_bool().context("params[1] not a bool")?;
            Ok(Some(format!("{block_tag}-{transaction_detail}")))
        } else {
            Ok(Some(block_tag))
        }
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
            "expected at least 1 params, got 0"
        );
    }

    #[test]
    fn test() {
        let params = json!(["0x12341324", false]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12341324-false");

        let params = json!(["0x12341324", true]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12341324-true");

        let params = json!(["0x12341324"]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12341324");
    }

    #[test]
    fn test_not_fixed_block() {
        let params = json!(["pending", false]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap();
        assert_eq!(cache_key, None);
    }

    #[test]
    fn test_invalid_transaction_detail() {
        let params = json!(["0x1234", 1]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "params[1] not a bool"
        );
    }
}
