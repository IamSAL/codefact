//! Filesystem locations: per-OS config/data dirs (central) or `.codefacts/` in
//! the repo (opt-in), plus a stable per-repo id.

use crate::config::Storage;
use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn dirs() -> anyhow::Result<ProjectDirs> {
    ProjectDirs::from("dev", "codefacts", "codefacts")
        .ok_or_else(|| anyhow::anyhow!("could not resolve OS project directories"))
}

pub fn config_path() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.config_dir().join("config.toml"))
}

pub fn secrets_path() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.config_dir().join("secrets.toml"))
}

pub fn central_data_dir() -> anyhow::Result<PathBuf> {
    Ok(dirs()?.data_dir().to_path_buf())
}

/// Stable short id for a repo, derived from its canonical path.
pub fn repo_id(repo_path: &str) -> String {
    let canon = std::fs::canonicalize(repo_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| repo_path.to_string());
    let digest = Sha256::digest(canon.as_bytes());
    hex12(&digest)
}

/// Where a repo's local artifacts (e.g. a file-based state store) would live.
pub fn data_dir_for(repo_path: &str, storage: Storage) -> anyhow::Result<PathBuf> {
    match storage {
        Storage::Central => Ok(central_data_dir()?.join(repo_id(repo_path))),
        Storage::InRepo => Ok(PathBuf::from(repo_path).join(".codefacts")),
    }
}

fn hex12(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_id_is_stable_and_short() {
        let a = repo_id("/tmp/some/repo");
        let b = repo_id("/tmp/some/repo");
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
    }
}
