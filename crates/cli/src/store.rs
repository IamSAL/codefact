//! `Store` backed by the iii-state worker, via `state::*` function calls.
//! Response shapes verified live: get → raw value, list → array of values.

use async_trait::async_trait;
use codefacts_core::store::Store;
use iii_sdk::IIIClient;
use iii_sdk::protocol::TriggerRequest;
use serde_json::{Value, json};

pub struct IiiStore {
    c: IIIClient,
}

impl IiiStore {
    pub fn new(c: IIIClient) -> Self {
        Self { c }
    }

    async fn call(&self, function_id: &str, payload: Value) -> anyhow::Result<Value> {
        self.c
            .trigger(TriggerRequest {
                function_id: function_id.to_string(),
                payload,
                action: None,
                timeout_ms: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!("{function_id} failed: {e}"))
    }
}

#[async_trait]
impl Store for IiiStore {
    async fn set(&self, scope: &str, key: &str, value: Value) -> anyhow::Result<()> {
        self.call("state::set", json!({ "scope": scope, "key": key, "value": value }))
            .await?;
        Ok(())
    }

    async fn get(&self, scope: &str, key: &str) -> anyhow::Result<Option<Value>> {
        match self.call("state::get", json!({ "scope": scope, "key": key })).await {
            Ok(Value::Null) => Ok(None),
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }

    async fn list(&self, scope: &str) -> anyhow::Result<Vec<Value>> {
        match self.call("state::list", json!({ "scope": scope })).await? {
            Value::Array(a) => Ok(a),
            _ => Ok(Vec::new()),
        }
    }

    async fn delete(&self, scope: &str, key: &str) -> anyhow::Result<()> {
        self.call("state::delete", json!({ "scope": scope, "key": key }))
            .await?;
        Ok(())
    }
}
