//! # Notification Service
//!
//! Provides limit-checked notification sending by combining the notification
//! sending functionality with the usage limiter.

use database::{NotificationResult, UsageLimiter};
use notifications::primitives::models::{Alert, NotificationDestination};
use notifications::{NotificationSender, send_notification};
use std::sync::Arc;
use uuid::Uuid;

/// Notification sender with limit enforcement.
/// Implements NotificationSender trait so it can be used polymorphically with monitors.
pub struct LimitedNotificationSender {
    /// The destination to send notifications to
    destination: NotificationDestination,
    /// User ID for tracking usage
    user_id: Uuid,
    /// Limiter for checking and recording usage
    limiter: Arc<UsageLimiter>,
}

impl LimitedNotificationSender {
    /// Create a new limit-checked notification sender
    pub fn new(
        destination: NotificationDestination,
        user_id: Uuid,
        limiter: Arc<UsageLimiter>,
    ) -> Self {
        Self {
            destination,
            user_id,
            limiter,
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for LimitedNotificationSender {
    /// Send a notification with limit checking and usage tracking.
    /// - Blocks notifications for plans with no overage pricing (hard limit)
    /// - Allows and tracks overage for paid plans (soft limit)
    async fn send(&self, alert: &Alert) -> Result<(), anyhow::Error> {
        // Record the notification (this checks limits and may block)
        let result = self.limiter.record_notification(self.user_id).await?;

        match result {
            NotificationResult::Blocked => {
                tracing::warn!(
                    user_id = %self.user_id,
                    source = %alert.source,
                    "Notification blocked - user at limit with no overage allowance"
                );
                // Don't send - hard limit reached
                return Ok(());
            }
            NotificationResult::Allowed => {
                tracing::debug!(
                    user_id = %self.user_id,
                    "Notification sent within limits"
                );
            }
            NotificationResult::Overage => {
                tracing::warn!(
                    user_id = %self.user_id,
                    "Notification sent as overage - will be billed"
                );
            }
        }

        // Send the notification
        send_notification(&self.destination, alert).await
    }
}
