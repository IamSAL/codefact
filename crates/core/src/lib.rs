//! codefact-core — the engine-agnostic knowledge logic. Pure and testable;
//! the iii worker supplies real `Store`/`Engine`/`Sender` implementations.

pub mod config;
pub mod engine;
pub mod graph;
pub mod miner;
pub mod model;
pub mod paths;
pub mod secrets;
pub mod selector;
pub mod sender;
pub mod store;

use crate::config::RepoConfig;
use crate::engine::{Engine, ExtractRequest};
use crate::graph::Graph;
use crate::model::{Coverage, Edge, Emission, Node};
use crate::sender::Sender;
use crate::store::Store;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// RFC3339 timestamp helper.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Short content hash used for insight/emission ids and file coverage.
pub fn short_hash(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Mine a slice: analyze `files`, merge entities/relations/insights into the
/// graph, and record coverage. Returns the number of nodes written.
pub async fn run_mine(
    store: &dyn Store,
    engine: &dyn Engine,
    repo: &RepoConfig,
    files: Vec<String>,
) -> anyhow::Result<usize> {
    let g = Graph::new(store, paths::repo_id(&repo.path));
    let summary = graph_summary(&g).await?;
    let req = ExtractRequest {
        repo_path: repo.path.clone(),
        interest: repo.interest.clone(),
        files: files.clone(),
        graph_summary: summary,
    };
    let ex = engine.extract(&req).await?;
    let now = now_rfc3339();
    let mut count = 0;

    for e in ex.entities {
        g.upsert_node(Node {
            id: e.name.clone(),
            kind: e.kind,
            name: e.name,
            path: e.path,
            summary: e.summary,
            salience: e.salience.unwrap_or(0.0),
            first_seen: now.clone(),
            last_seen: now.clone(),
        })
        .await?;
        count += 1;
    }
    for r in ex.relations {
        g.add_edge(Edge {
            id: format!("{}|{}|{}", r.from, r.rel, r.to),
            src: r.from,
            dst: r.to,
            rel: r.rel,
        })
        .await?;
    }
    for i in ex.insights {
        let id = format!("insight:{}", short_hash(&i.text));
        g.upsert_node(Node {
            id: id.clone(),
            kind: "insight".into(),
            name: id,
            path: None,
            summary: i.text,
            salience: 1.0 + i.novelty.unwrap_or(0.5),
            first_seen: now.clone(),
            last_seen: now.clone(),
        })
        .await?;
        count += 1;
    }
    for f in &files {
        let hash = file_hash(&repo.path, f).unwrap_or_default();
        g.record_coverage(Coverage {
            path: f.clone(),
            hash,
            analyzed_at: now.clone(),
        })
        .await?;
    }
    Ok(count)
}

/// Select and send the best insight not recently emitted. Returns the text sent.
pub async fn run_emit(
    store: &dyn Store,
    sender: &dyn Sender,
    repo: &RepoConfig,
) -> anyhow::Result<Option<String>> {
    let g = Graph::new(store, paths::repo_id(&repo.path));
    let nodes = g.nodes().await?;
    if nodes.is_empty() {
        return Ok(None);
    }
    let emissions = g.emissions().await?;
    let recent: HashSet<String> = emissions
        .iter()
        .rev()
        .take(20)
        .flat_map(|e| e.node_ids.clone())
        .collect();

    let ranked = selector::rank(&nodes, &recent, &repo.interest);
    let Some(top) = ranked.into_iter().next() else {
        return Ok(None);
    };
    let text = top.summary.clone();
    sender.send(&text).await?;
    g.record_emission(Emission {
        id: short_hash(&text),
        text: text.clone(),
        node_ids: vec![top.id],
        ts: now_rfc3339(),
    })
    .await?;
    Ok(Some(text))
}

async fn graph_summary(g: &Graph<'_>) -> anyhow::Result<String> {
    let nodes = g.nodes().await?;
    let names: Vec<String> = nodes
        .iter()
        .filter(|n| n.kind != "insight")
        .take(40)
        .map(|n| format!("{}({})", n.name, n.kind))
        .collect();
    Ok(names.join(", "))
}

fn file_hash(repo_path: &str, rel: &str) -> Option<String> {
    let full = std::path::Path::new(repo_path).join(rel);
    let bytes = std::fs::read(full).ok()?;
    Some(
        Sha256::digest(&bytes)
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Extraction, InsightOut};
    use crate::store::InMemoryStore;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockEngine(Extraction);
    #[async_trait]
    impl Engine for MockEngine {
        async fn extract(&self, _req: &ExtractRequest) -> anyhow::Result<Extraction> {
            Ok(self.0.clone())
        }
    }

    #[derive(Default)]
    struct MockSender(Mutex<Vec<String>>);
    #[async_trait]
    impl Sender for MockSender {
        async fn send(&self, text: &str) -> anyhow::Result<()> {
            self.0.lock().unwrap().push(text.to_string());
            Ok(())
        }
    }

    fn repo() -> RepoConfig {
        RepoConfig {
            path: ".".into(),
            interest: "queues and apis".into(),
            storage: crate::config::Storage::Central,
        }
    }

    #[tokio::test]
    async fn mine_then_emit_sends_insight_and_dedups_next_time() {
        let store = InMemoryStore::new();
        let engine = MockEngine(Extraction {
            entities: vec![],
            relations: vec![],
            insights: vec![InsightOut {
                text: "The queue worker retries via a dead-letter topic.".into(),
                node_refs: vec![],
                novelty: Some(0.9),
            }],
        });
        let sender = MockSender::default();

        let n = run_mine(&store, &engine, &repo(), vec![]).await.unwrap();
        assert_eq!(n, 1);

        let sent = run_emit(&store, &sender, &repo()).await.unwrap();
        assert!(sent.is_some());
        assert_eq!(sender.0.lock().unwrap().len(), 1);

        // Second emit has nothing new → no send.
        let again = run_emit(&store, &sender, &repo()).await.unwrap();
        assert!(again.is_none());
        assert_eq!(sender.0.lock().unwrap().len(), 1);
    }
}
