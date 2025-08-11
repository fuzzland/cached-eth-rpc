use eyre::Result;
use serde_json::Value;

mod common;
mod debug_trace_block_by_hash;
mod debug_trace_block_by_number;
mod debug_trace_call;
mod debug_trace_transaction;
mod eth_call;
mod eth_chainid;
mod eth_estimate_gas;
mod eth_get_balance;
mod eth_get_block_by_hash;
mod eth_get_block_by_number;
mod eth_get_block_receipts;
mod eth_get_code;
mod eth_get_logs;
mod eth_get_storage_at;
mod eth_get_transaction_by_block_hash_and_index;
mod eth_get_transaction_by_block_number_and_index;
mod eth_get_transaction_by_hash;
mod eth_get_transaction_count;
mod eth_get_transaction_receipt;

pub trait RpcCacheHandler: Send + Sync {
    fn method_name(&self) -> &'static str;

    fn extract_cache_key(&self, params: &Value) -> Result<Option<String>>;

    fn extract_cache_value(&self, result: &Value) -> Result<(bool, String)> {
        Ok((!result.is_null(), serde_json::to_string(result)?))
    }
}

pub type RpcCacheHandlerFactory = fn() -> Box<dyn RpcCacheHandler>;

pub fn get_factory<T>() -> fn() -> Box<dyn RpcCacheHandler>
where
    T: Default + RpcCacheHandler + 'static,
{
    || Box::<T>::default()
}

pub fn factories() -> Vec<RpcCacheHandlerFactory> {
    vec![
        get_factory::<debug_trace_block_by_hash::Handler>(),
        get_factory::<debug_trace_block_by_number::Handler>(),
        get_factory::<debug_trace_call::Handler>(),
        get_factory::<debug_trace_transaction::Handler>(),
        get_factory::<eth_call::Handler>(),
        get_factory::<eth_chainid::Handler>(),
        get_factory::<eth_estimate_gas::Handler>(),
        get_factory::<eth_get_balance::Handler>(),
        get_factory::<eth_get_block_by_hash::Handler>(),
        get_factory::<eth_get_block_by_number::Handler>(),
        get_factory::<eth_get_block_receipts::Handler>(),
        get_factory::<eth_get_code::Handler>(),
        get_factory::<eth_get_logs::Handler>(),
        get_factory::<eth_get_storage_at::Handler>(),
        get_factory::<eth_get_transaction_by_block_hash_and_index::Handler>(),
        get_factory::<eth_get_transaction_by_block_number_and_index::Handler>(),
        get_factory::<eth_get_transaction_by_hash::Handler>(),
        get_factory::<eth_get_transaction_count::Handler>(),
        get_factory::<eth_get_transaction_receipt::Handler>(),
    ]
}
