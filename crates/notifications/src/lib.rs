pub mod email;

/// A generic payload for any alert
#[derive(Debug, Clone)]
pub struct Alert {
    pub source: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum NotificationDestination {
    Email(String),
}

pub async fn send_notification(
    dest: &NotificationDestination,
    alert: &Alert,
) -> Result<(), anyhow::Error> {
    match dest {
        NotificationDestination::Email(recipient) => {
            email::send_email(recipient, &alert.subject, &alert.message).await
        }
    }
}
