use eyre::Context;
use redis::Commands;
use serde_json::{from_str, Value};

use super::{CacheBackend, CacheBackendFactory, CacheStatus};

pub struct RedisBackendFactory {
    chain_id: u64,
    client: r2d2::Pool<redis::Client>,
}

impl RedisBackendFactory {
    pub fn new(chain_id: u64, client: r2d2::Pool<redis::Client>) -> Self {
        Self { chain_id, client }
    }
}

impl CacheBackendFactory for RedisBackendFactory {
    fn get_instance(&self) -> eyre::Result<Box<dyn CacheBackend>> {
        Ok(Box::new(RedisBackend {
            chain_id: self.chain_id,
            conn: self.client.get()?,
        }))
    }
}

pub struct RedisBackend {
    chain_id: u64,
    conn: r2d2::PooledConnection<redis::Client>,
}

impl CacheBackend for RedisBackend {
    fn read(&mut self, method: &str, params_key: &str) -> eyre::Result<CacheStatus> {
        let cache_key = format!("{}:{method}:{params_key}", self.chain_id);
        let value: Option<String> = self.conn.get(&cache_key)?;

        let v = match value {
            Some(value) => {
                let value = from_str::<Value>(&value).context("fail to deserialize cache value")?;
                CacheStatus::Cached {
                    key: cache_key,
                    value,
                }
            }
            None => CacheStatus::Missed { key: cache_key },
        };

        Ok(v)
    }

    fn write(&mut self, key: &str, value: &str) -> eyre::Result<()> {
        let _ = self.conn.set::<_, _, String>(key, value);
        Ok(())
    }
}
