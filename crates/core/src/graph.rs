//! The memory graph, expressed over a [`Store`]. Scopes namespace each repo's
//! nodes/edges/coverage/emissions.

use crate::model::{Coverage, Edge, Emission, Node};
use crate::store::Store;
use std::collections::HashSet;

pub struct Graph<'a> {
    store: &'a dyn Store,
    repo: String,
}

impl<'a> Graph<'a> {
    pub fn new(store: &'a dyn Store, repo: impl Into<String>) -> Self {
        Self {
            store,
            repo: repo.into(),
        }
    }

    fn scope(&self, kind: &str) -> String {
        format!("cf:{}:{}", self.repo, kind)
    }

    /// Insert or merge a node by id. Preserves `first_seen` and keeps the max
    /// salience seen so recurring entities accrue importance.
    pub async fn upsert_node(&self, mut n: Node) -> anyhow::Result<()> {
        let scope = self.scope("nodes");
        if let Some(existing) = self.store.get(&scope, &n.id).await? {
            if let Ok(old) = serde_json::from_value::<Node>(existing) {
                n.first_seen = old.first_seen;
                n.salience = n.salience.max(old.salience);
            }
        }
        self.store
            .set(&scope, &n.id.clone(), serde_json::to_value(&n)?)
            .await
    }

    pub async fn add_edge(&self, e: Edge) -> anyhow::Result<()> {
        self.store
            .set(&self.scope("edges"), &e.id.clone(), serde_json::to_value(&e)?)
            .await
    }

    pub async fn record_coverage(&self, c: Coverage) -> anyhow::Result<()> {
        self.store
            .set(
                &self.scope("coverage"),
                &c.path.clone(),
                serde_json::to_value(&c)?,
            )
            .await
    }

    pub async fn record_emission(&self, e: Emission) -> anyhow::Result<()> {
        self.store
            .set(
                &self.scope("emissions"),
                &e.id.clone(),
                serde_json::to_value(&e)?,
            )
            .await
    }

    pub async fn nodes(&self) -> anyhow::Result<Vec<Node>> {
        Ok(self
            .store
            .list(&self.scope("nodes"))
            .await?
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect())
    }

    pub async fn emissions(&self) -> anyhow::Result<Vec<Emission>> {
        Ok(self
            .store
            .list(&self.scope("emissions"))
            .await?
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect())
    }

    pub async fn covered_paths(&self) -> anyhow::Result<HashSet<String>> {
        Ok(self
            .store
            .list(&self.scope("coverage"))
            .await?
            .into_iter()
            .filter_map(|v| serde_json::from_value::<Coverage>(v).ok())
            .map(|c| c.path)
            .collect())
    }

    /// True if a file has never been analyzed or its content hash changed.
    pub async fn needs_analysis(&self, path: &str, hash: &str) -> anyhow::Result<bool> {
        match self.store.get(&self.scope("coverage"), path).await? {
            Some(v) => match serde_json::from_value::<Coverage>(v) {
                Ok(c) => Ok(c.hash != hash),
                Err(_) => Ok(true),
            },
            None => Ok(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;

    fn node(id: &str, salience: f64, first_seen: &str) -> Node {
        Node {
            id: id.into(),
            kind: "module".into(),
            name: id.into(),
            path: None,
            summary: "s".into(),
            salience,
            first_seen: first_seen.into(),
            last_seen: "later".into(),
        }
    }

    #[tokio::test]
    async fn upsert_dedups_and_preserves_first_seen_and_max_salience() {
        let store = InMemoryStore::new();
        let g = Graph::new(&store, "repo1");
        g.upsert_node(node("a", 1.0, "t0")).await.unwrap();
        g.upsert_node(node("a", 5.0, "t9")).await.unwrap();
        let nodes = g.nodes().await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].first_seen, "t0"); // preserved
        assert_eq!(nodes[0].salience, 5.0); // max kept
    }

    #[tokio::test]
    async fn coverage_detects_hash_change() {
        let store = InMemoryStore::new();
        let g = Graph::new(&store, "repo1");
        assert!(g.needs_analysis("f.rs", "h1").await.unwrap());
        g.record_coverage(Coverage {
            path: "f.rs".into(),
            hash: "h1".into(),
            analyzed_at: "t".into(),
        })
        .await
        .unwrap();
        assert!(!g.needs_analysis("f.rs", "h1").await.unwrap());
        assert!(g.needs_analysis("f.rs", "h2").await.unwrap());
    }
}
