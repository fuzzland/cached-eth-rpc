use alloy_primitives::B256;
use eyre::{Context, ContextCompat as _};
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "debug_traceTransaction"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::AtLeast(1))?;

        let tx_hash: B256 = serde_json::from_value(params[0].clone())
            .context("params[0] is not a valid transaction hash")?;

        if params.len() > 1 {
            let tracer_config =
                serde_json::to_string(params[1].as_object().context("params[1] not an object")?)
                    .unwrap();

            let tracer_config_hash = common::hash_string(tracer_config.as_str());
            Ok(Some(format!("{tx_hash:#x}-{tracer_config_hash}")))
        } else {
            Ok(Some(format!("{tx_hash:#x}")))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    static HANDLER: Handler = Handler;

    #[test]
    fn test_normal_case_with_tracer_config() {
        let params = json!([
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            {
                "tracer": "callTracer",
                "traceConfig": {
                    "disableMemory": true,
                    "disableStack": false,
                    "disableStorage": false
                }
            }
        ]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(
            cache_key,
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef-6c52bf3f36c00c206d7775565066213cc6265c95"
        );
    }

    #[test]
    fn test_invalid_tx() {
        let params = json!(["0xgg"]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "params[0] is not a valid transaction hash");
    }
}
