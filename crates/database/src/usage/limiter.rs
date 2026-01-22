//! # Usage Limiter
//!
//! Provides a unified interface for checking and recording notification usage
//! against subscription limits.

use super::UsageRepository;
use crate::subscriptions::SubscriptionRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Result of attempting to send a notification
#[derive(Debug, Clone)]
pub enum NotificationResult {
    /// Notification sent within plan limits
    Allowed,
    /// Notification sent as overage (over limit, but allowed for paid plans)
    Overage,
    /// Notification blocked - plan has no overage allowance (hard limit)
    Blocked,
}

/// Combines subscription and usage checking for limit enforcement
#[derive(Clone)]
pub struct UsageLimiter {
    subscriptions: SubscriptionRepository,
    usage: UsageRepository,
}

impl UsageLimiter {
    pub fn new(pool: PgPool) -> Self {
        Self {
            subscriptions: SubscriptionRepository::new(pool.clone()),
            usage: UsageRepository::new(pool),
        }
    }

    /// Check limits and record a notification for a user.
    ///
    /// Returns:
    /// - `Allowed` if within plan limits
    /// - `Overage` if over limit but plan allows overage (overage_price_cents > 0)
    /// - `Blocked` if over limit and plan has no overage pricing (hard limit)
    ///
    /// TODO(billing): Review this hard limit logic before production.
    ///   - Should free tier users be notified when approaching limit?
    ///   - Should we send an email when they hit the limit?
    ///   - Consider adding a grace period or warning threshold.
    pub async fn record_notification(
        &self,
        user_id: Uuid,
    ) -> Result<NotificationResult, anyhow::Error> {
        // Get user's subscription and plan
        let subscription = self
            .subscriptions
            .get_or_create_subscription(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get subscription: {}", e))?;

        let plan = self
            .subscriptions
            .get_plan(&subscription.plan_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get plan: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Plan not found: {}", subscription.plan_id))?;

        // Check current usage BEFORE incrementing to enforce hard limits
        let current_count = self
            .usage
            .get_current_notification_count(user_id)
            .await
            .unwrap_or(0);
        let would_be_overage = current_count >= plan.notification_limit;

        // Hard limit: we are using this currently, since we have not setup actual billing
        if would_be_overage && plan.overage_price_cents == 0 {
            tracing::warn!(
                user_id = %user_id,
                plan = %subscription.plan_id,
                current = current_count,
                limit = plan.notification_limit,
                "Notification BLOCKED - plan has no overage allowance"
            );
            return Ok(NotificationResult::Blocked);
        }

        // Record notification and check if overage
        let (_count, is_overage) = self
            .usage
            .increment_notifications(user_id, plan.notification_limit)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to record notification: {}", e))?;

        if is_overage {
            tracing::info!(
                user_id = %user_id,
                plan = %subscription.plan_id,
                "Notification sent as overage - will be billed"
            );
            Ok(NotificationResult::Overage)
        } else {
            Ok(NotificationResult::Allowed)
        }
    }

    /// Get current notification count for a user in the current period
    pub async fn get_current_count(&self, user_id: Uuid) -> Result<i64, anyhow::Error> {
        self.usage
            .get_current_notification_count(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get notification count: {}", e))
    }
}
