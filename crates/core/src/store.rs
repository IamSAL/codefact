//! Storage abstraction. The worker backs this with iii-state; tests use the
//! in-memory impl. Keeping the graph logic behind this trait is what lets the
//! knowledge logic be unit-tested without a live iii engine.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// A scoped key-value store. `scope` groups keys (e.g. one repo's nodes).
#[async_trait]
pub trait Store: Send + Sync {
    async fn set(&self, scope: &str, key: &str, value: Value) -> anyhow::Result<()>;
    async fn get(&self, scope: &str, key: &str) -> anyhow::Result<Option<Value>>;
    async fn list(&self, scope: &str) -> anyhow::Result<Vec<Value>>;
    async fn delete(&self, scope: &str, key: &str) -> anyhow::Result<()>;
}

/// In-memory [`Store`] for tests. Not used in production.
#[derive(Default)]
pub struct InMemoryStore {
    inner: Mutex<BTreeMap<(String, String), Value>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Store for InMemoryStore {
    async fn set(&self, scope: &str, key: &str, value: Value) -> anyhow::Result<()> {
        self.inner
            .lock()
            .unwrap()
            .insert((scope.to_string(), key.to_string()), value);
        Ok(())
    }

    async fn get(&self, scope: &str, key: &str) -> anyhow::Result<Option<Value>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(&(scope.to_string(), key.to_string()))
            .cloned())
    }

    async fn list(&self, scope: &str) -> anyhow::Result<Vec<Value>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .iter()
            .filter(|((s, _), _)| s == scope)
            .map(|(_, v)| v.clone())
            .collect())
    }

    async fn delete(&self, scope: &str, key: &str) -> anyhow::Result<()> {
        self.inner
            .lock()
            .unwrap()
            .remove(&(scope.to_string(), key.to_string()));
        Ok(())
    }
}
