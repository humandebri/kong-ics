// どこで: Discord Webhook を叩く通知クライアント
// 何を: .env から取得した webhook URL に content を送る
// なぜ: 鍵をコードに埋め込まずに通知を飛ばすため

use reqwest::Client;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("HTTP エラー: {0}")]
    Http(String),
}

#[derive(Clone)]
pub struct DiscordNotifier {
    client: Client,
    webhook_url: String,
}

#[derive(Serialize)]
struct DiscordPayload<'a> {
    content: &'a str,
}

impl DiscordNotifier {
    pub fn new(webhook_url: String) -> Self {
        DiscordNotifier {
            client: Client::new(),
            webhook_url,
        }
    }

    pub async fn notify(&self, message: &str) -> Result<(), NotifyError> {
        let payload = DiscordPayload { content: message };
        let res = self
            .client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotifyError::Http(e.to_string()))?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(NotifyError::Http(format!("status {}", res.status())))
        }
    }
}
