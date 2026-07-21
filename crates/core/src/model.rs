//! Serializable knowledge-graph records. These are what live in iii-state.

use serde::{Deserialize, Serialize};

/// A knowledge entity: a module, service, library, API, integration, data-flow,
/// or a synthesized `insight` (kind == "insight").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    /// Stable identity. Entities use their name; insights use `insight:<hash>`.
    pub id: String,
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub salience: f64,
    pub first_seen: String,
    pub last_seen: String,
}

/// A directed relationship between two nodes (by node id/name).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: String,
    pub src: String,
    pub dst: String,
    pub rel: String,
}

/// Records that a file has been analyzed at a given content hash, so the miner
/// can detect changes and avoid re-analyzing unchanged files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Coverage {
    pub path: String,
    pub hash: String,
    pub analyzed_at: String,
}

/// A fact that was pushed to the user. Used for novelty/dedup in the selector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Emission {
    pub id: String,
    pub text: String,
    pub node_ids: Vec<String>,
    pub ts: String,
}
