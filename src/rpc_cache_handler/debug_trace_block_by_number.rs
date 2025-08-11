use eyre::{Context, ContextCompat as _};
use serde_json::Value;

use crate::rpc_cache_handler::common::ParamsSpec;
use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "debug_traceBlockByNumber"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, ParamsSpec::AtLeast(1))?;

        let block_tag = common::extract_and_format_block_number(&params[0])
            .context("params[0] not a valid block number")?;
        let block_number = match block_tag {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        if params.len() > 1 {
            let tracer_config =
                serde_json::to_string(params[1].as_object().context("params[1] not an object")?)?;

            let tracer_config_hash = common::hash_string(tracer_config.as_str());

            Ok(Some(format!("{block_number}-{tracer_config_hash}")))
        } else {
            Ok(Some(block_number))
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
            "0x12341324",
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
            "0x12341324-6c52bf3f36c00c206d7775565066213cc6265c95"
        );
    }

    #[test]
    fn test_normal_case_without_tracer_config() {
        let params = json!(["0x12341324"]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "0x12341324");
    }

    #[test]
    fn test_invalid_block_number() {
        let params = json!(["0xgg"]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "params[0] not a valid block number");
    }
}
