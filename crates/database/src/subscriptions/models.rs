//! # Subscription Models
//!
//! Data models for plans and subscriptions.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Subscription plan (cached from Stripe)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub stripe_price_id: Option<String>,
    pub monitor_limit: i64,
    pub notification_limit: i64,
    pub overage_price_cents: i64,
}

impl Plan {
    /// Check if this plan allows unlimited monitors
    pub fn has_unlimited_monitors(&self) -> bool {
        self.monitor_limit < 0
    }

    /// Check if user can create more monitors
    pub fn can_create_monitor(&self, current_count: i64) -> bool {
        self.has_unlimited_monitors() || current_count < self.monitor_limit
    }

    /// Check if user is within notification limit
    pub fn is_within_notification_limit(&self, sent_count: i64) -> bool {
        sent_count < self.notification_limit
    }
}

/// Subscription status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    PastDue,
    Trialing,
}

impl From<String> for SubscriptionStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "active" => SubscriptionStatus::Active,
            "canceled" => SubscriptionStatus::Canceled,
            "past_due" => SubscriptionStatus::PastDue,
            "trialing" => SubscriptionStatus::Trialing,
            _ => SubscriptionStatus::Active,
        }
    }
}

impl ToString for SubscriptionStatus {
    fn to_string(&self) -> String {
        match self {
            SubscriptionStatus::Active => "active".to_string(),
            SubscriptionStatus::Canceled => "canceled".to_string(),
            SubscriptionStatus::PastDue => "past_due".to_string(),
            SubscriptionStatus::Trialing => "trialing".to_string(),
        }
    }
}

/// User subscription record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: String,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub status: SubscriptionStatus,
    pub current_period_start: Option<NaiveDateTime>,
    pub current_period_end: Option<NaiveDateTime>,
    pub created_at: Option<NaiveDateTime>,
}
