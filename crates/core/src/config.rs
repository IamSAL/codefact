//! Non-secret configuration (TOML). Secrets live separately (see `secrets`).

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub schedule: Schedule,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
    #[serde(default)]
    pub engine: EngineConfig,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_history_keep")]
    pub history_keep: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// "HH:MM" 24h times, host-local.
    pub times: Vec<String>,
    #[serde(default = "default_tz")]
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub path: String,
    /// Free-form natural-language interest; the engine interprets it.
    pub interest: String,
    #[serde(default)]
    pub storage: Storage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    #[default]
    Central,
    InRepo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default = "default_claude_bin")]
    pub bin: String,
    #[serde(default = "default_allowed_tools")]
    pub allowed_tools: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_slice")]
    pub slice_files: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            bin: default_claude_bin(),
            allowed_tools: default_allowed_tools(),
            timeout_secs: default_timeout(),
            slice_files: default_slice(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_history_keep() -> usize {
    200
}
fn default_tz() -> String {
    "Europe/Oslo".to_string()
}
fn default_claude_bin() -> String {
    "claude".to_string()
}
fn default_allowed_tools() -> String {
    "Read Glob Grep".to_string()
}
fn default_timeout() -> u64 {
    180
}
fn default_slice() -> usize {
    12
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.schedule.times.is_empty() {
            anyhow::bail!("schedule.times must not be empty");
        }
        for t in &self.schedule.times {
            parse_hhmm(t)?;
        }
        Ok(())
    }
}

/// Parse "HH:MM" into (hour, minute), validating ranges.
pub fn parse_hhmm(t: &str) -> anyhow::Result<(u32, u32)> {
    let (h, m) = t
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("bad time '{t}', want HH:MM"))?;
    let h: u32 = h.parse()?;
    let m: u32 = m.parse()?;
    if h > 23 || m > 59 {
        anyhow::bail!("time '{t}' out of range");
    }
    Ok((h, m))
}

/// Map "HH:MM" times to iii 6-field cron expressions (sec min hour dom mon dow).
pub fn times_to_cron(times: &[String]) -> anyhow::Result<Vec<String>> {
    times
        .iter()
        .map(|t| {
            let (h, m) = parse_hhmm(t)?;
            Ok(format!("0 {m} {h} * * *"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_map_to_cron() {
        let cron = times_to_cron(&["14:00".into(), "09:05".into()]).unwrap();
        assert_eq!(cron, vec!["0 0 14 * * *", "0 5 9 * * *"]);
    }

    #[test]
    fn rejects_bad_time() {
        assert!(parse_hhmm("25:00").is_err());
        assert!(parse_hhmm("noon").is_err());
    }
}
