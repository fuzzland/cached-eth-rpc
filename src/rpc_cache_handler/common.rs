use std::str::FromStr;

use alloy_primitives::{Address, B256, U64};
use eyre::{bail, Context, ContextCompat as _};
use serde_json::Value;
use sha1::Digest;

pub enum ParamsSpec {
    Exact(usize),
    AtLeast(usize),
}

pub fn require_array_params(params: &Value, len: ParamsSpec) -> eyre::Result<&Vec<Value>> {
    let array = params.as_array().context("expect params to be an array")?;

    match len {
        ParamsSpec::Exact(expected_len) if array.len() != expected_len => {
            eyre::bail!("expected {} params, got {}", expected_len, array.len());
        }
        ParamsSpec::AtLeast(expected_len) if array.len() < expected_len => {
            eyre::bail!(
                "expected at least {} params, got {}",
                expected_len,
                array.len()
            );
        }
        _ => {}
    };

    Ok(array)
}

pub fn extract_address_cache_key(params: &Value) -> eyre::Result<Option<String>> {
    let params = require_array_params(params, ParamsSpec::AtLeast(1))?;

    let account: Address =
        serde_json::from_value(params[0].clone()).context("params[0] not a valid address")?;

    let block_tag = match extract_and_format_block_tag(&params[1])
        .context("params[1] not a valid block tag")?
    {
        Some(block_tag) => block_tag,
        None => return Ok(None),
    };

    let lowercase_address = account.to_string().to_lowercase();

    Ok(Some(format!("{block_tag}-{lowercase_address}")))
}

pub fn extract_transaction_cache_value(result: &Value) -> eyre::Result<(bool, String)> {
    let can_cache = result.is_object() && !result["blockHash"].is_null();

    Ok((can_cache, serde_json::to_string(result)?))
}

pub fn extract_and_format_block_number(value: &Value) -> eyre::Result<Option<String>> {
    let value = value.as_str().context("block tag not a string")?;

    let block_tag = match value {
        "earliest" | "latest" | "pending" | "finalized" | "safe" => None,
        _ => {
            let v = U64::from_str(value)
                .context("block tag not a valid block number")?
                .as_limbs()[0];
            Some(format!("0x{:x}", v))
        }
    };

    Ok(block_tag)
}

pub fn extract_and_format_block_hash(value: &Value) -> eyre::Result<String> {
    let value_str = value.as_str().context("block tag not a string")?;

    if value_str.len() != 66 {
        bail!("expect a valid block hash");
    }

    let block_hash = B256::from_str(&value_str[2..]).context("expect a valid block hash")?;
    Ok(format!("{block_hash:#x}"))
}

pub fn extract_and_format_block_tag(value: &Value) -> eyre::Result<Option<String>> {
    let value_str = value.as_str().context("block tag not a string")?;

    if value_str.len() == 66 {
        extract_and_format_block_hash(value).map(Some)
    } else {
        let block_tag = extract_and_format_block_number(value)?;
        Ok(block_tag)
    }
}

pub fn hash_string(s: &str) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();

    hex::encode(result.as_slice())
}

#[cfg(test)]
mod test {
    mod test_extract_and_format_block_tag {
        use super::super::*;
        use serde_json::json;

        #[test]
        fn test_earliest() {
            let block_tag = extract_and_format_block_tag(&json!("earliest")).unwrap();
            assert_eq!(block_tag, None);
        }

        #[test]
        fn test_latest() {
            let block_tag = extract_and_format_block_tag(&json!("latest")).unwrap();
            assert_eq!(block_tag, None);
        }

        #[test]
        fn test_pending() {
            let block_tag = extract_and_format_block_tag(&json!("pending")).unwrap();
            assert_eq!(block_tag, None);
        }

        #[test]
        fn test_finalized() {
            let block_tag = extract_and_format_block_tag(&json!("finalized")).unwrap();
            assert_eq!(block_tag, None);
        }

        #[test]
        fn test_safe() {
            let block_tag = extract_and_format_block_tag(&json!("safe")).unwrap();
            assert_eq!(block_tag, None);
        }

        #[test]
        fn test_block_hash() {
            let block_tag = extract_and_format_block_tag(&json!(
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ))
            .unwrap();
            assert_eq!(
                block_tag,
                Some(
                    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                        .to_string()
                )
            );
        }

        #[test]
        fn test_invalid_block_hash() {
            let block_tag = extract_and_format_block_tag(&json!(
                "0x1234567890abcdef1234567890abcdef1234567890abcdef123456789ggggggg"
            ))
            .unwrap_err();

            assert_eq!(block_tag.to_string(), "expect a valid block hash");
        }

        #[test]
        fn test_block_number() {
            let block_tag = extract_and_format_block_tag(&json!("0x12345")).unwrap();
            assert_eq!(block_tag, Some("0x12345".to_string()));
        }

        #[test]
        fn test_invalid_block_number() {
            let block_tag = extract_and_format_block_tag(&json!("0x12345ggggggg")).unwrap_err();

            assert_eq!(block_tag.to_string(), "block tag not a valid block number");
        }
    }

    mod test_extract_address_cache_key {
        use super::super::*;
        use serde_json::json;

        #[test]
        fn test_fixed_block() {
            let params = json!(["0x1234567890abcdef1234567890abcdef12345678", "0x12345"]);

            let cache_key = extract_address_cache_key(&params).unwrap().unwrap();
            assert_eq!(
                cache_key,
                "0x12345-0x1234567890abcdef1234567890abcdef12345678"
            );
        }

        #[test]
        fn test_with_block_tag() {
            let params = json!(["0x1234567890abcdef1234567890abcdef12345678", "earliest"]);

            let cache_key = extract_address_cache_key(&params).unwrap();
            assert_eq!(cache_key, None);
        }

        #[test]
        fn test_invalid_address() {
            let params = json!(["0x1234567890abcdef1234567890abcdef1234gggg", "latest"]);

            let err = extract_address_cache_key(&params).unwrap_err();
            assert_eq!(err.to_string(), "params[0] not a valid address");
        }

        #[test]
        fn test_invalid_block_tag() {
            let params = json!(["0x1234567890abcdef1234567890abcdef12345678", "ggg tag"]);

            let err = extract_address_cache_key(&params).unwrap_err();
            assert_eq!(err.to_string(), "params[1] not a valid block tag");
        }
    }
}
