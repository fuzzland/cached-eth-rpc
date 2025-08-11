use std::sync::Arc;

use eyre::Context;
use dashmap::DashMap;
use serde_json::{from_str, Value};

use super::{CacheBackend, CacheBackendFactory, CacheStatus};

pub struct MemoryBackendFactory {
    data: Arc<DashMap<String, String>>,
}

impl MemoryBackendFactory {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }
}

impl CacheBackendFactory for MemoryBackendFactory {
    fn get_instance(&self) -> eyre::Result<Box<dyn CacheBackend>> {
        Ok(Box::new(MemoryBackend {
            data: self.data.clone(),
        }))
    }
}

pub struct MemoryBackend {
    data: Arc<DashMap<String, String>>,
}

impl CacheBackend for MemoryBackend {
    fn read(&mut self, method: &str, params_key: &str) -> eyre::Result<CacheStatus> {
        let key = format!("{method}:{params_key}");

        let v = match self.data.get(&key) {
            Some(value) => {
                let value = from_str::<Value>(&value).context("fail to deserialize cache value")?;

                CacheStatus::Cached { key, value }
            }

            None => CacheStatus::Missed { key },
        };

        Ok(v)
    }

    fn write(&mut self, key: &str, value: &str) -> eyre::Result<()> {
        let _ = self.data.insert(key.to_string(), value.to_string());
        Ok(())
    }
}
