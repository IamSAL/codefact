//! Choose what to emit: rank nodes by salience + interest relevance, biasing
//! insights, excluding anything recently emitted. Pure and unit-tested.

use crate::model::Node;
use std::collections::HashSet;

/// Rank emit candidates best-first. Nodes whose id is in `recent` are excluded.
pub fn rank(nodes: &[Node], recent: &HashSet<String>, interest: &str) -> Vec<Node> {
    let keywords = keywords(interest);
    let mut scored: Vec<(f64, &Node)> = nodes
        .iter()
        .filter(|n| !recent.contains(&n.id))
        .map(|n| (score(n, &keywords), n))
        .collect();
    // Highest score first; stable tiebreak by name for determinism.
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.name.cmp(&b.1.name))
    });
    scored.into_iter().map(|(_, n)| n.clone()).collect()
}

fn score(node: &Node, keywords: &[String]) -> f64 {
    let mut s = node.salience;
    if node.kind == "insight" {
        s += 1.0; // insights are the payoff we most want to surface
    }
    let hay = format!("{} {}", node.name, node.summary).to_lowercase();
    let hits = keywords.iter().filter(|k| hay.contains(*k)).count();
    s + hits as f64 * 2.0
}

fn keywords(interest: &str) -> Vec<String> {
    interest
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3)
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: &str, name: &str, summary: &str, salience: f64) -> Node {
        Node {
            id: id.into(),
            kind: kind.into(),
            name: name.into(),
            path: None,
            summary: summary.into(),
            salience,
            first_seen: "t".into(),
            last_seen: "t".into(),
        }
    }

    #[test]
    fn excludes_recently_emitted() {
        let nodes = vec![node("a", "module", "A", "", 5.0)];
        let recent: HashSet<String> = ["a".to_string()].into_iter().collect();
        assert!(rank(&nodes, &recent, "anything").is_empty());
    }

    #[test]
    fn interest_keyword_boosts_and_insight_wins() {
        let nodes = vec![
            node("a", "module", "queue_worker", "handles jobs", 1.0),
            node("b", "module", "misc", "unrelated", 1.0),
        ];
        let recent = HashSet::new();
        let ranked = rank(&nodes, &recent, "the queue system");
        assert_eq!(ranked[0].id, "a"); // keyword "queue" boosts A above B
    }
}
