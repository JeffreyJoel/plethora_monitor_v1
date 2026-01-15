//! # Email Notification Implementation
//!
//! Email delivery implementation using Resend API.
//! Handles sending alert notifications via SMTP/email service.
//! Provides HTML formatting for email content.

use reqwest::Client;
use serde::Serialize;
use std::env;

#[derive(Serialize)]
struct EmailPayload {
    sender: Sender,
    to: Vec<Recipient>,
    subject: String,
    #[serde(rename = "textContent")]
    text_content: String,
    #[serde(rename = "htmlContent")]
    html_content: String,
}

#[derive(Serialize)]
struct Sender {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct Recipient {
    email: String,
}

pub async fn send_email(recipient: &str, subject: &str, body: &str) -> Result<(), anyhow::Error> {
    let api_key = env::var("BREVO_API_KEY").expect("BREVO_API_KEY must be set");
    let client = Client::new();

    let html = generate_html_alert("Monitor Alert", subject, body, None);

    let payload = EmailPayload {
        sender: Sender {
            name: "Plethora Monitor".to_string(),
            email: "plethora.onchain@gmail.com".to_string(),
        },
        to: vec![Recipient {
            email: recipient.to_string(),
        }],
        subject: subject.to_string(),
        text_content: body.to_string(),
        html_content: html,
    };

    let response = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", api_key)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Email sent via API to {}", recipient);
    } else {
        let error_text = response.text().await?;
        eprintln!("❌ Brevo API Error: {}", error_text);
    }

    Ok(())
}

/// Helper functions to display emails in html format

pub fn generate_html_alert(source: &str, title: &str, details: &str, link: Option<&str>) -> String {
    format!(
        r##"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: 'Space Mono', 'SF Mono', 'Consolas', monospace; line-height: 1.6; color: #fafafa; background-color: #0a0a0a; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .card {{ background: #121212; border-radius: 12px; border: 1px solid #242424; overflow: hidden; }}
    .header {{ background: linear-gradient(135deg, #121212 0%, #1a1a1a 100%); padding: 24px; text-align: center; border-bottom: 1px solid #242424; }}
    .header h1 {{ margin: 0; font-size: 20px; color: #D2E823; text-transform: uppercase; letter-spacing: 3px; }}
    .content {{ padding: 30px; }}
    .alert-badge {{ display: inline-block; background: #D2E823; color: #0a0a0a; padding: 6px 14px; border-radius: 99px; font-size: 11px; font-weight: bold; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 16px; }}
    .source-title {{ margin: 0 0 8px 0; font-size: 18px; color: #fafafa; }}
    .subtitle {{ color: #8c8c8c; margin: 0 0 20px 0; font-size: 14px; }}
    .data-table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
    .data-table td {{ padding: 12px 0; border-bottom: 1px solid #242424; vertical-align: top; }}
    .label {{ color: #8c8c8c; font-weight: 600; width: 30%; font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px; }}
    .value {{ font-family: 'Space Mono', monospace; color: #fafafa; word-break: break-all; font-size: 13px; }}
    .btn {{ display: block; width: 100%; text-align: center; background: #D2E823; color: #0a0a0a; padding: 14px; border-radius: 8px; text-decoration: none; font-weight: bold; margin-top: 20px; text-transform: uppercase; letter-spacing: 1px; font-size: 12px; }}
    .footer {{ text-align: center; padding: 20px; color: #525252; font-size: 11px; }}
    .footer a {{ color: #525252; text-decoration: underline; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="card">
      <div class="header">
        <h1>⚡ Plethora Monitor</h1>
      </div>
      <div class="content">
        <span class="alert-badge">Alert Triggered</span>
        <h2 class="source-title">{source}</h2>
        <p class="subtitle">{title}</p>

        <table class="data-table">
          {details_rows}
        </table>

        {button_html}
      </div>
    </div>
    <div class="footer">
      Powered by Plethora Monitor • <a href="#">Unsubscribe</a>
    </div>
  </div>
</body>
</html>
"##,
        source = source,
        title = title,
        details_rows = format_details_to_rows(details),
        button_html = if let Some(url) = link {
            format!(r#"<a href="{}" class="btn">View on Explorer</a>"#, url)
        } else {
            String::new()
        }
    )
}

// Helper to display emails in html table format
fn format_details_to_rows(raw_text: &str) -> String {
    raw_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if let Some((key, val)) = line.split_once(':') {
                format!(
                    r#"<tr><td class="label">{}</td><td class="value">{}</td></tr>"#,
                    key.trim(),
                    val.trim()
                )
            } else {
                format!(r#"<tr><td colspan="2" class="value">{}</td></tr>"#, line)
            }
        })
        .collect()
}

/// Send a verification email to confirm email channel ownership
pub async fn send_verification_email(
    recipient: &str,
    token: &str,
    base_url: &str,
) -> Result<(), anyhow::Error> {
    let api_key = env::var("BREVO_API_KEY").expect("BREVO_API_KEY must be set");
    let client = Client::new();

    let verification_link = format!("{}/verify-channel?token={}", base_url, token);
    let html = generate_verification_html(&verification_link);
    let subject = "Verify your email for Plethora Monitor";
    let body = format!(
        "Click this link to verify your email: {}",
        verification_link
    );

    let payload = EmailPayload {
        sender: Sender {
            name: "Plethora Monitor".to_string(),
            email: "plethora.onchain@gmail.com".to_string(),
        },
        to: vec![Recipient {
            email: recipient.to_string(),
        }],
        subject: subject.to_string(),
        text_content: body,
        html_content: html,
    };

    let response = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", api_key)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Verification email sent to {}", recipient);
    } else {
        let error_text = response.text().await?;
        eprintln!("❌ Brevo API Error: {}", error_text);
    }

    Ok(())
}

fn generate_verification_html(verification_link: &str) -> String {
    format!(
        r##"
<!DOCTYPE html>
<html>
<head>
  <style>
    body {{ font-family: 'Space Mono', 'SF Mono', 'Consolas', monospace; line-height: 1.6; color: #fafafa; background-color: #0a0a0a; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
    .card {{ background: #121212; border-radius: 12px; border: 1px solid #242424; overflow: hidden; }}
    .header {{ background: linear-gradient(135deg, #121212 0%, #1a1a1a 100%); padding: 24px; text-align: center; border-bottom: 1px solid #242424; }}
    .header h1 {{ margin: 0; font-size: 20px; color: #D2E823; text-transform: uppercase; letter-spacing: 3px; }}
    .content {{ padding: 40px 30px; text-align: center; }}
    .icon {{ font-size: 56px; margin-bottom: 24px; }}
    .title {{ margin: 0 0 16px 0; font-size: 22px; color: #fafafa; }}
    .description {{ color: #8c8c8c; font-size: 14px; margin: 0 0 28px 0; line-height: 1.7; }}
    .btn {{ display: inline-block; background: #D2E823; color: #0a0a0a; padding: 16px 32px; border-radius: 8px; text-decoration: none; font-weight: bold; text-transform: uppercase; letter-spacing: 1px; font-size: 12px; }}
    .note {{ color: #525252; font-size: 12px; margin-top: 28px; line-height: 1.6; }}
    .footer {{ text-align: center; padding: 20px; color: #525252; font-size: 11px; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="card">
      <div class="header">
        <h1>⚡ Plethora Monitor</h1>
      </div>
      <div class="content">
        <h2 class="title">Verify Your Email</h2>
        <p class="description">Thanks for adding this email to your Plethora Monitor notification channels. Click the button below to verify your email and start receiving alerts.</p>
        <a href="{}" class="btn">Verify Email Address</a>
        <p class="note">This link expires in 24 hours. If you didn't request this verification, you can safely ignore this email.</p>
      </div>
    </div>
    <div class="footer">
      Plethora Monitor • Email Verification
    </div>
  </div>
</body>
</html>
"##,
        verification_link
    )
}

