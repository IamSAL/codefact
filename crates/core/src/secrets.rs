//! Secrets live in their own file with 0600 perms and are NEVER written to
//! iii-state (the console State page is visible). Redaction helpers keep the
//! token out of logs/status output.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secrets {
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
}

impl Secrets {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        set_owner_only(path)?;
        Ok(())
    }

    /// Safe-to-log rendering of the token: `…` + last 4 chars.
    pub fn redacted_token(&self) -> String {
        redact(&self.telegram_bot_token)
    }
}

/// Mask a secret, keeping only the last 4 characters for recognizability.
pub fn redact(secret: &str) -> String {
    let n = secret.chars().count();
    if n <= 4 {
        return "*".repeat(n);
    }
    let tail: String = secret.chars().skip(n - 4).collect();
    format!("…{tail}")
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) -> anyhow::Result<()> {
    // ponytail: Windows ACL hardening deferred; NTFS default is user-scoped.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_never_leaks_token() {
        let s = Secrets {
            telegram_bot_token: "123456:AAbbCCddEEffGG".into(),
            telegram_chat_id: "42".into(),
        };
        let r = s.redacted_token();
        assert!(!r.contains("AAbbCC"));
        assert!(r.ends_with("ffGG"));
    }
}
