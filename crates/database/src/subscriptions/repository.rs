//! # Subscription Repository
//!
//! Data access layer for subscriptions and plans.

use super::models::{Plan, Subscription, SubscriptionStatus};
use chrono::NaiveDateTime;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct SubscriptionRepository {
    pool: PgPool,
}

impl SubscriptionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a plan by ID
    pub async fn get_plan(&self, plan_id: &str) -> Result<Option<Plan>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, stripe_price_id, monitor_limit, notification_limit, overage_price_cents
            FROM plans WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Plan {
            id: r.get("id"),
            stripe_price_id: r.get("stripe_price_id"),
            monitor_limit: r.get("monitor_limit"),
            notification_limit: r.get("notification_limit"),
            overage_price_cents: r.get("overage_price_cents"),
        }))
    }

    /// Get all available plans
    pub async fn get_all_plans(&self) -> Result<Vec<Plan>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, stripe_price_id, monitor_limit, notification_limit, overage_price_cents
            FROM plans ORDER BY monitor_limit ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Plan {
                id: r.get("id"),
                stripe_price_id: r.get("stripe_price_id"),
                monitor_limit: r.get("monitor_limit"),
                notification_limit: r.get("notification_limit"),
                overage_price_cents: r.get("overage_price_cents"),
            })
            .collect())
    }

    // ========== Subscription Operations ==========

    /// Get subscription for a user, creating a free one if none exists
    pub async fn get_or_create_subscription(
        &self,
        user_id: Uuid,
    ) -> Result<Subscription, sqlx::Error> {
        // Try to get existing subscription
        if let Some(sub) = self.get_subscription_by_user(user_id).await? {
            return Ok(sub);
        }

        // Create free subscription
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO subscriptions (id, user_id, plan_id, status)
            VALUES ($1, $2, 'free', 'active')
            ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // Fetch the created/existing subscription
        self.get_subscription_by_user(user_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Get subscription by user ID
    pub async fn get_subscription_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Subscription>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, plan_id, stripe_customer_id, stripe_subscription_id,
                   status, current_period_start, current_period_end, created_at
            FROM subscriptions WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let status_str: String = r.get("status");
            Subscription {
                id: r.get("id"),
                user_id: r.get("user_id"),
                plan_id: r.get("plan_id"),
                stripe_customer_id: r.get("stripe_customer_id"),
                stripe_subscription_id: r.get("stripe_subscription_id"),
                status: SubscriptionStatus::from(status_str),
                current_period_start: r.get("current_period_start"),
                current_period_end: r.get("current_period_end"),
                created_at: r.get("created_at"),
            }
        }))
    }

    /// Update subscription after Stripe webhook
    pub async fn update_subscription(
        &self,
        user_id: Uuid,
        plan_id: &str,
        stripe_customer_id: Option<&str>,
        stripe_subscription_id: Option<&str>,
        status: SubscriptionStatus,
        period_start: Option<NaiveDateTime>,
        period_end: Option<NaiveDateTime>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE subscriptions SET
                plan_id = $1,
                stripe_customer_id = COALESCE($2, stripe_customer_id),
                stripe_subscription_id = COALESCE($3, stripe_subscription_id),
                status = $4,
                current_period_start = COALESCE($5, current_period_start),
                current_period_end = COALESCE($6, current_period_end)
            WHERE user_id = $7
            "#,
        )
        .bind(plan_id)
        .bind(stripe_customer_id)
        .bind(stripe_subscription_id)
        .bind(status.to_string())
        .bind(period_start)
        .bind(period_end)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get subscription by Stripe customer ID (for webhook handling)
    pub async fn get_subscription_by_stripe_customer(
        &self,
        stripe_customer_id: &str,
    ) -> Result<Option<Subscription>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, plan_id, stripe_customer_id, stripe_subscription_id,
                   status, current_period_start, current_period_end, created_at
            FROM subscriptions WHERE stripe_customer_id = $1
            "#,
        )
        .bind(stripe_customer_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let status_str: String = r.get("status");
            Subscription {
                id: r.get("id"),
                user_id: r.get("user_id"),
                plan_id: r.get("plan_id"),
                stripe_customer_id: r.get("stripe_customer_id"),
                stripe_subscription_id: r.get("stripe_subscription_id"),
                status: SubscriptionStatus::from(status_str),
                current_period_start: r.get("current_period_start"),
                current_period_end: r.get("current_period_end"),
                created_at: r.get("created_at"),
            }
        }))
    }
}
