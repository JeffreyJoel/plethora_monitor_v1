use anyhow::{Result, bail};
use reqwest::Client;

pub async fn send_telegram_message(token: &str, chat_id: &str, message: &str) -> Result<()> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

    let client = Client::new();

    let params = [
        ("chat_id", chat_id),
        ("text", message),
    ];

    let response = client.post(&url).form(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        bail!("Telegram API Error, {}", error_text)
    }

    Ok(())
}
