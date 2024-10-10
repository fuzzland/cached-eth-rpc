use anyhow::{bail, Context};
use serde_json::Value;

use crate::rpc_cache_handler::common::require_array_params;
use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getLogs"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = &require_array_params(params, common::ParamsSpec::Exact(1))?;

        println!("params: {:?}", params);

        let filter = &params[0];

        if !filter.is_object() {
            bail!("params[0] not a filter object");
        }

        let mut block_tag = if !filter["blockHash"].is_null() {
            Some(
                common::extract_and_format_block_hash(&filter["blockHash"])
                    .context("expect a valid block hash")?,
            )
        } else {
            None
        };

        if block_tag.is_none() {
            let from_block = if !filter["fromBlock"].is_null() {
                common::extract_and_format_block_number(&filter["fromBlock"])
                    .context("`fromBlock` is not a valid block number")?
            } else {
                None
            };

            let to_block = if !filter["toBlock"].is_null() {
                common::extract_and_format_block_number(&filter["toBlock"])
                    .context("`toBlock` is not a valid block number")?
            } else {
                None
            };

            if let (Some(from_block), Some(to_block)) = (from_block, to_block) {
                block_tag = Some(format!("{}-{}", from_block, to_block));
            }
        }

        let cache_key = block_tag.map(|block_tag| {
            format!(
                "{}-{}",
                block_tag,
                common::hash_string(&serde_json::to_string(filter).unwrap())
            )
        });

        Ok(cache_key)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    static HANDLER: Handler = Handler;

    #[test]
    fn test_block_range() {
        let params = json!([
          {
            "address": [
              "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
            ],
            "fromBlock": "0x429d3b",
            "toBlock": "0x429d3c",
            "topics": [
              "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
              "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
              "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
            ]
          },
        ]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap();
        assert_eq!(
            cache_key,
            Some("0x429d3b-0x429d3c-bc57b716eb2996bd7f98537dd51516bb541ca882".to_string())
        );
    }

    #[test]
    fn test_block_hash() {
        let params = json!([
          {
            "address": [
              "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
            ],
            "blockHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "topics": [
              "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
              "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
              "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
            ]
          },
        ]);

        let cache_key = HANDLER.extract_cache_key(&params).unwrap();
        assert_eq!(
            cache_key,
            Some(
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef-\
                 b27f759a6574516ddaa99bc9534f4cfcae86d386"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_invalid_block_number() {
        let params = json!([
          {
            "address": [
              "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
            ],
            "fromBlock": "0x12345ggggggg",
            "toBlock": "0x12345",
            "topics": [
              "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
              "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
              "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
            ]
          },
        ]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "`fromBlock` is not a valid block number");

        let params = json!([
          {
            "address": [
              "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
            ],
            "fromBlock": "0x12345",
            "toBlock": "0x12345ggggggg",
            "topics": [
              "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
              "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
              "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
            ]
          },
        ]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "`toBlock` is not a valid block number");
    }

    #[test]
    fn test_invalid_block_hash() {
        let params = json!([
          {
            "address": [
              "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
            ],
            "blockHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef123456789ggggggg",
            "topics": [
              "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
              "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
              "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
            ]
          },
        ]);

        let err = HANDLER.extract_cache_key(&params).unwrap_err();
        assert_eq!(err.to_string(), "expect a valid block hash");
    }
}
