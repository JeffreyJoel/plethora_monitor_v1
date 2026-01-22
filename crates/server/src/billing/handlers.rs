//! # Billing Handlers
//!
//! HTTP request handlers for billing endpoints.

use crate::billing::models::{
    PlanDetails, PlanResponse, PlansResponse, SubscriptionResponse, UsageResponse,
};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use clerk_rs::validators::authorizer::ClerkJwt;
use database::UserRepository;
use std::sync::Arc;

/// GET /api/billing/subscription
/// Returns current subscription, plan, and usage details
#[utoipa::path(
    get,
    path = "/api/billing/subscription",
    responses(
        (status = 200, description = "Current subscription details", body = SubscriptionResponse),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "billing"
)]
pub async fn get_subscription(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo.get_or_create(&jwt.sub, None).await.map_err(|e| {
        tracing::error!("DB User Error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get or create free subscription
    let subscription = state
        .db
        .subscriptions
        .get_or_create_subscription(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get subscription: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get plan details
    let plan = state
        .db
        .subscriptions
        .get_plan(&subscription.plan_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get plan: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::error!("Plan not found: {}", subscription.plan_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get current period usage
    let usage = state
        .db
        .usage
        .get_or_create_current(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get usage: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get actual monitor count
    let monitor_count = state.db.monitors.count_by_user(user_id).await.unwrap_or(0) as i64;

    Ok(Json(SubscriptionResponse {
        plan: PlanResponse::from(plan.clone()),
        status: subscription.status.to_string(),
        usage: UsageResponse {
            monitors_count: monitor_count,
            notifications_sent: usage.notifications_sent,
            notifications_limit: plan.notification_limit,
            overage_notifications: usage.overage_notifications,
        },
        stripe_customer_id: subscription.stripe_customer_id,
    }))
}

/// GET /api/billing/plans
/// Returns all available plans
#[utoipa::path(
    get,
    path = "/api/billing/plans",
    responses(
        (status = 200, description = "Available plans", body = PlansResponse),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "billing"
)]
pub async fn get_plans(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<PlansResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo.get_or_create(&jwt.sub, None).await.map_err(|e| {
        tracing::error!("DB User Error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get user's current plan
    let subscription = state
        .db
        .subscriptions
        .get_or_create_subscription(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get all plans
    let plans = state.db.subscriptions.get_all_plans().await.map_err(|e| {
        tracing::error!("Failed to get plans: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let plan_details: Vec<PlanDetails> = plans
        .into_iter()
        .map(|p| PlanDetails {
            id: p.id.clone(),
            monitor_limit: p.monitor_limit,
            notification_limit: p.notification_limit,
            overage_price_cents: p.overage_price_cents,
            is_current: p.id == subscription.plan_id,
        })
        .collect();

    Ok(Json(PlansResponse {
        plans: plan_details,
    }))
}

/// GET /api/billing/usage
/// Returns current period usage details
#[utoipa::path(
    get,
    path = "/api/billing/usage",
    responses(
        (status = 200, description = "Current usage", body = UsageResponse),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "billing"
)]
pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<UsageResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let subscription = state
        .db
        .subscriptions
        .get_or_create_subscription(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let plan = state
        .db
        .subscriptions
        .get_plan(&subscription.plan_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let usage = state
        .db
        .usage
        .get_or_create_current(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let monitor_count = state.db.monitors.count_by_user(user_id).await.unwrap_or(0) as i64;

    Ok(Json(UsageResponse {
        monitors_count: monitor_count,
        notifications_sent: usage.notifications_sent,
        notifications_limit: plan.notification_limit,
        overage_notifications: usage.overage_notifications,
    }))
}
