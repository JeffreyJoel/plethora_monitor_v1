//! # Billing Models
//!
//! Request and response models for billing API endpoints.

use database::Plan;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Current subscription and usage details
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionResponse {
    /// Current plan details
    pub plan: PlanResponse,
    /// Subscription status
    pub status: String,
    /// Current period usage
    pub usage: UsageResponse,
    /// Stripe customer ID (if connected)
    pub stripe_customer_id: Option<String>,
}

/// Plan details
#[derive(Debug, Serialize, ToSchema)]
pub struct PlanResponse {
    pub id: String,
    pub monitor_limit: i64,
    pub notification_limit: i64,
    /// -1 means unlimited
    pub is_unlimited_monitors: bool,
}

impl From<Plan> for PlanResponse {
    fn from(plan: Plan) -> Self {
        Self {
            id: plan.id.clone(),
            monitor_limit: plan.monitor_limit,
            notification_limit: plan.notification_limit,
            is_unlimited_monitors: plan.has_unlimited_monitors(),
        }
    }
}

/// Current period usage
#[derive(Debug, Serialize, ToSchema)]
pub struct UsageResponse {
    pub monitors_count: i64,
    pub notifications_sent: i64,
    pub notifications_limit: i64,
    pub overage_notifications: i64,
}

/// Request to upgrade/downgrade subscription
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePlanRequest {
    /// Target plan ID (pro, business)
    pub plan_id: String,
}

/// All available plans
#[derive(Debug, Serialize, ToSchema)]
pub struct PlansResponse {
    pub plans: Vec<PlanDetails>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlanDetails {
    pub id: String,
    pub monitor_limit: i64,
    pub notification_limit: i64,
    pub overage_price_cents: i64,
    /// Whether this is the user's current plan
    pub is_current: bool,
}
