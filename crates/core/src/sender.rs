//! Delivery. No iii registry worker exists for Telegram/messaging, so this is
//! a genuinely-needed small custom sender (outbound HTTPS via ureq).

use async_trait::async_trait;

#[async_trait]
pub trait Sender: Send + Sync {
    async fn send(&self, text: &str) -> anyhow::Result<()>;
}

pub struct TelegramSender {
    pub bot_token: String,
    pub chat_id: String,
}

#[async_trait]
impl Sender for TelegramSender {
    async fn send(&self, text: &str) -> anyhow::Result<()> {
        // ponytail: ureq is blocking; at 3 sends/day the brief block on the async
        // runtime is irrelevant. Move to spawn_blocking if volume ever grows.
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );
        let resp = ureq::post(&url)
            .send_form(&[("chat_id", &self.chat_id), ("text", text)])
            .map_err(|e| anyhow::anyhow!("telegram request failed: {e}"))?;
        if resp.status() != 200 {
            anyhow::bail!("telegram returned status {}", resp.status());
        }
        Ok(())
    }
}
