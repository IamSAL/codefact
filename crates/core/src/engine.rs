//! Analysis engine: turns a slice of the repo into structured knowledge by
//! calling headless `claude`. The argv/prompt/parse steps are pure functions so
//! they can be unit-tested without spawning anything.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub repo_path: String,
    pub interest: String,
    pub files: Vec<String>,
    /// Compact summary of what's already in the graph (to avoid repetition).
    pub graph_summary: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Extraction {
    #[serde(default)]
    pub entities: Vec<EntityOut>,
    #[serde(default)]
    pub relations: Vec<RelationOut>,
    #[serde(default)]
    pub insights: Vec<InsightOut>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EntityOut {
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub salience: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RelationOut {
    pub from: String,
    pub to: String,
    pub rel: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct InsightOut {
    pub text: String,
    #[serde(default)]
    pub node_refs: Vec<String>,
    #[serde(default)]
    pub novelty: Option<f64>,
}

#[async_trait]
pub trait Engine: Send + Sync {
    async fn extract(&self, req: &ExtractRequest) -> anyhow::Result<Extraction>;
}

/// Real engine: spawns `claude -p` with read-only tools inside the repo.
pub struct ClaudeEngine {
    pub bin: String,
    pub allowed_tools: String,
    pub timeout: Duration,
    /// Canonicalized repo paths the engine is permitted to run inside.
    pub allowed_repo_paths: Vec<String>,
}

#[async_trait]
impl Engine for ClaudeEngine {
    async fn extract(&self, req: &ExtractRequest) -> anyhow::Result<Extraction> {
        // Security: only ever run inside a configured repo (no arbitrary cwd).
        let canon = std::fs::canonicalize(&req.repo_path)?
            .to_string_lossy()
            .to_string();
        if !self.allowed_repo_paths.iter().any(|p| p == &canon) {
            anyhow::bail!("repo path '{}' is not in the allowed list", req.repo_path);
        }

        let prompt = build_prompt(req);
        let args = claude_argv(&self.allowed_tools, &prompt);

        // One retry: models occasionally wrap JSON in prose despite instructions.
        let mut last_err = None;
        for _ in 0..2 {
            let out = tokio::time::timeout(
                self.timeout,
                Command::new(&self.bin)
                    .args(&args)
                    .current_dir(&canon)
                    .output(),
            )
            .await
            .map_err(|_| anyhow::anyhow!("claude timed out"))??;

            if !out.status.success() {
                last_err = Some(anyhow::anyhow!(
                    "claude exited {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                ));
                continue;
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            match parse_extraction(&stdout) {
                Ok(ex) => return Ok(ex),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("claude produced no parseable output")))
    }
}

/// Build the argv passed to the claude binary (no shell involved).
pub fn claude_argv(allowed_tools: &str, prompt: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        prompt.to_string(),
        "--allowedTools".to_string(),
        allowed_tools.to_string(),
    ]
}

/// Compose the mining prompt. Instructs strict-JSON output.
pub fn build_prompt(req: &ExtractRequest) -> String {
    format!(
        "You are mapping the codebase in the current directory for a developer.\n\
         Their interest: {interest}\n\n\
         Inspect these files (read them with your tools): {files}\n\n\
         Already known (avoid repeating): {summary}\n\n\
         Extract structured knowledge and reply with ONLY a JSON object, no prose, \
         no markdown fences, matching:\n\
         {{\"entities\":[{{\"kind\":\"module|service|lib|api|integration|dataflow\",\
         \"name\":\"..\",\"path\":\"..\",\"summary\":\"..\",\"salience\":0.0}}],\
         \"relations\":[{{\"from\":\"..\",\"to\":\"..\",\"rel\":\"calls|imports|depends_on|flows_to|exposes\"}}],\
         \"insights\":[{{\"text\":\"one genuinely useful, specific fact\",\"node_refs\":[\"..\"],\"novelty\":0.0}}]}}\n\
         Prefer concrete, grounded facts (name real files/symbols). 1-4 insights.",
        interest = req.interest,
        files = req.files.join(", "),
        summary = if req.graph_summary.is_empty() { "(nothing yet)" } else { &req.graph_summary },
    )
}

/// Parse claude stdout into an [`Extraction`], tolerating code fences / prose
/// around the JSON object.
pub fn parse_extraction(stdout: &str) -> anyhow::Result<Extraction> {
    let json = extract_json_object(stdout)
        .ok_or_else(|| anyhow::anyhow!("no JSON object found in output"))?;
    Ok(serde_json::from_str(json)?)
}

/// Slice out the first balanced-looking `{...}` region.
fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_has_print_flag_prompt_and_readonly_tools() {
        let args = claude_argv("Read Glob Grep", "hello");
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "hello");
        assert!(args.contains(&"--allowedTools".to_string()));
        assert!(args.contains(&"Read Glob Grep".to_string()));
    }

    #[test]
    fn parses_json_wrapped_in_fences() {
        let out = "here you go:\n```json\n{\"entities\":[],\"relations\":[],\"insights\":[{\"text\":\"x\"}]}\n```";
        let ex = parse_extraction(out).unwrap();
        assert_eq!(ex.insights.len(), 1);
        assert_eq!(ex.insights[0].text, "x");
    }

    #[test]
    fn malformed_output_errors() {
        assert!(parse_extraction("no json here").is_err());
    }
}
