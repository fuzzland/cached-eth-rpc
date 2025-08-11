use eyre::{bail, Context, ContextCompat as _};
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_call"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::AtLeast(1))?;

        let tx = serde_json::to_string(
            params[0]
                .as_object()
                .context("params[0] not a transaction call object")?,
        )
        .unwrap();
        let block_tag = common::extract_and_format_block_tag(&params[1])
            .context("params[1] not a valid block tag")?;
        let block_tag = match block_tag {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        let tx_hash = common::hash_string(tx.as_str());

        if params.len() > 2 {
            if !params[2].is_object() {
                bail!("params[2] not a state override setting object")
            }

            let state_override = common::hash_string(&serde_json::to_string(&params[2]).unwrap());

            return Ok(Some(format!("{block_tag}-{tx_hash}-{state_override}",)));
        }

        Ok(Some(format!("{block_tag}-{tx_hash}")))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    static HANDLER: Handler = Handler;

    #[test]
    fn test() {
        let params = json!([
            {
                "from": null,
                "to": "0x6b175474e89094c44da98b954eedeac495271d0f",
                "data": "0x70a082310000000000000000000000006E0d01A76C3Cf4288372a29124A26D4353EE51BE"
            },
            "0x12341324",
        ]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(
            cache_key,
            "0x12341324-aa734bab822de3d5f3191359094abe1eb49e3563"
        );
    }

    #[test]
    fn test_invalid_tx() {
        let params = json!(["0xgg"]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "params[0] not a transaction call object");
    }

    #[test]
    fn test_invalid_block_tag() {
        let params = json!([
            {
                "from": null,
                "to": "0x6b175474e89094c44da98b954eedeac495271d0f",
                "data": "0x70a082310000000000000000000000006E0d01A76C3Cf4288372a29124A26D4353EE51BE"
            },
            "ggg tag"
        ]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "params[1] not a valid block tag");
    }

    #[test]
    fn test_with_state_override() {
        let params = json!([
            {
                "from": null,
                "to": "0x6b175474e89094c44da98b954eedeac495271d0f",
                "data": "0x70a082310000000000000000000000006E0d01A76C3Cf4288372a29124A26D4353EE51BE"
            },
            "0x12341324",
            {
                "0x6b175474e89094c44da98b954eedeac495271d0f": {
                    "code": "0x12341234"
                }
            }
        ]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(
            cache_key,
            "0x12341324-aa734bab822de3d5f3191359094abe1eb49e3563-22884c3a09357b73375ee790393367081571afb7"
        );
    }

    #[test]
    fn test_invalid_state_override() {
        let params = json!([
            {
                "from": null,
                "to": "0x6b175474e89094c44da98b954eedeac495271d0f",
                "data": "0x70a082310000000000000000000000006E0d01A76C3Cf4288372a29124A26D4353EE51BE"
            },
            "0x12341324",
            "ggg"
        ]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(
            err.to_string(),
            "params[2] not a state override setting object"
        );
    }
}
