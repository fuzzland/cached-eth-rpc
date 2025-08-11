use alloy_primitives::{Address, U256};
use eyre::{bail, Context};
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getStorageAt"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::AtLeast(2))?;

        let block_tag = common::extract_and_format_block_tag(&params[2])
            .context("params[2] is not a valid block tag")?;
        let block_tag = match block_tag {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        let account: Address =
            serde_json::from_value(params[0].clone()).context("params[0] not a valid address")?;
        let lowercase_address = account.to_string().to_lowercase();

        let slot = match params[1] {
            Value::String(ref s) => {
                let slot_value = U256::from_str_radix(s.trim_start_matches("0x"), 16)
                    .context("params[1] not a valid hex value")?;

                format!("{}", slot_value)
            }

            Value::Number(ref n) => n
                .as_u64()
                .ok_or(eyre::eyre!("params[1] not a valid slot value"))?
                .to_string(),

            _ => bail!("params[1] not a valid slot value"),
        };

        Ok(Some(format!("{block_tag}-{lowercase_address}-{slot}")))
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
            "expected at least 2 params, got 0"
        );

        let params = json!(["0xC310e760778ECBca4C65B6C559874757A4c4Ece0"]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "expected at least 2 params, got 1"
        );
    }

    #[test]
    fn test_normal_case() {
        let params = json!([
            "0xC310e760778ECBca4C65B6C559874757A4c4Ece0",
            "0x1234",
            "0x1234"
        ]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(
            cache_key,
            "0x1234-0xc310e760778ecbca4c65b6c559874757a4c4ece0-4660"
        );

        let params = json!(["0xC310e760778ECBca4C65B6C559874757A4c4Ece0", 1234, "0x1234"]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(
            cache_key,
            "0x1234-0xc310e760778ecbca4c65b6c559874757a4c4ece0-1234"
        );

        let params = json!(["0x12341324", "0x1234", "earliest"]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap();
        assert_eq!(cache_key, None);
    }

    #[test]
    fn test_invalid_address() {
        let params = json!(["0x12341324", "0x1234", "0x12345"]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "params[0] not a valid address"
        );
    }

    #[test]
    fn test_invalid_slot() {
        let params = json!([
            "0xC310e760778ECBca4C65B6C559874757A4c4Ece0",
            "0x1234gg",
            "0x1234"
        ]);
        assert_eq!(
            HANDLER.extract_cache_key(&params).unwrap_err().to_string(),
            "params[1] not a valid hex value"
        );
    }
}
