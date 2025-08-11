use eyre::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getBlockReceipts"
    }

    fn extract_cache_key(&self, params: &Value) -> eyre::Result<Option<String>> {
        let params = common::require_array_params(params, common::ParamsSpec::AtLeast(1))?;

        let block_tag = common::extract_and_format_block_tag(&params[0])
            .context("params[0] is not a valid block tag")?;
        let block_tag = match block_tag {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        Ok(Some(block_tag))
    }
}
