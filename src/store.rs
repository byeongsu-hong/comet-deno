use std::collections::HashMap;

use async_trait::async_trait;
use deno_core::error::AnyError;

#[async_trait]
pub trait Store: Sync + Send {
    async fn set(&mut self, key: String, value: String) -> Result<Option<String>, AnyError>;
    async fn get(&self, key: String) -> Result<Option<String>, AnyError>;
    async fn len(&self) -> Result<usize, AnyError>;
}

pub struct MemoryStore(HashMap<String, String>);

impl MemoryStore {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn set(&mut self, key: String, value: String) -> Result<Option<String>, AnyError> {
        Ok(self.0.insert(key, value))
    }

    async fn get(&self, key: String) -> Result<Option<String>, AnyError> {
        Ok(self.0.get(&key).cloned())
    }

    async fn len(&self) -> Result<usize, AnyError> {
        Ok(self.0.len())
    }
}
