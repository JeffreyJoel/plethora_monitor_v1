use anyhow::{Result, bail};
use reqwest::Client;
use serde_json::json;

pub async fn send_discord_webhook(webhook_url: &str, message: &str) -> Result<()> {
    let client = Client::new();

    // we use create the JSON payload with a "content" field, expected by discord
    let payload = json!({
        "content": message
    });

    let response = client.post(webhook_url).json(&payload).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        bail!("Discord API Error: {}", error_text);
    }

    Ok(())
}
