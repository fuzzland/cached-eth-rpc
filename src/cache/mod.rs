pub mod lru_backend;
pub mod memory_backend;
pub mod redis_backend;

use serde_json::Value;

pub enum CacheStatus {
    Cached { key: String, value: Value },
    Missed { key: String },
}

pub trait CacheBackendFactory: Send + Sync {
    fn get_instance(&self) -> anyhow::Result<Box<dyn CacheBackend>>;
}

pub trait CacheBackend {
    fn read(&mut self, method: &str, params_key: &str) -> anyhow::Result<CacheStatus>;
    fn write(&mut self, key: &str, value: &str) -> anyhow::Result<()>;
}
