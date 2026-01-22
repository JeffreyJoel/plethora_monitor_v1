//! # Usage Repository
//!
//! Data access layer for usage tracking and limits enforcement.

use super::models::Usage;
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct UsageRepository {
    pool: PgPool,
}

impl UsageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the first day of the current month
    fn current_period_start() -> NaiveDate {
        let now = Utc::now();
        NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap()
    }

    /// Get or create usage record for current period
    pub async fn get_or_create_current(&self, user_id: Uuid) -> Result<Usage, sqlx::Error> {
        let period_start = Self::current_period_start();

        // Upsert to ensure record exists
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO usage (id, user_id, period_start, monitors_count, notifications_sent, overage_notifications)
            VALUES ($1, $2, $3, 0, 0, 0)
            ON CONFLICT (user_id, period_start) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(period_start)
        .execute(&self.pool)
        .await?;

        // Fetch the record
        self.get_usage(user_id, period_start)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Get usage for a specific period
    pub async fn get_usage(
        &self,
        user_id: Uuid,
        period_start: NaiveDate,
    ) -> Result<Option<Usage>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, period_start, monitors_count, notifications_sent, overage_notifications
            FROM usage WHERE user_id = $1 AND period_start = $2
            "#,
        )
        .bind(user_id)
        .bind(period_start)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Usage {
            id: r.get("id"),
            user_id: r.get("user_id"),
            period_start: r.get("period_start"),
            monitors_count: r.get("monitors_count"),
            notifications_sent: r.get("notifications_sent"),
            overage_notifications: r.get("overage_notifications"),
        }))
    }

    /// Increment notification count for current period
    /// Returns (new_count, is_overage) - is_overage is true if over the limit
    pub async fn increment_notifications(
        &self,
        user_id: Uuid,
        notification_limit: i64,
    ) -> Result<(i64, bool), sqlx::Error> {
        let period_start = Self::current_period_start();

        // Ensure record exists first
        self.get_or_create_current(user_id).await?;

        // Increment and check if overage
        let row = sqlx::query(
            r#"
            UPDATE usage 
            SET 
                notifications_sent = notifications_sent + 1,
                overage_notifications = CASE 
                    WHEN notifications_sent >= $3 THEN overage_notifications + 1 
                    ELSE overage_notifications 
                END
            WHERE user_id = $1 AND period_start = $2
            RETURNING notifications_sent, overage_notifications
            "#,
        )
        .bind(user_id)
        .bind(period_start)
        .bind(notification_limit)
        .fetch_one(&self.pool)
        .await?;

        let new_count: i64 = row.get("notifications_sent");
        let overage: i64 = row.get("overage_notifications");

        Ok((new_count, overage > 0))
    }

    /// Update monitor count for current period
    pub async fn update_monitor_count(&self, user_id: Uuid, count: i64) -> Result<(), sqlx::Error> {
        let period_start = Self::current_period_start();

        // Ensure record exists first
        self.get_or_create_current(user_id).await?;

        sqlx::query(
            r#"
            UPDATE usage SET monitors_count = $1
            WHERE user_id = $2 AND period_start = $3
            "#,
        )
        .bind(count)
        .bind(user_id)
        .bind(period_start)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get total notifications sent in current period
    pub async fn get_current_notification_count(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let usage = self.get_or_create_current(user_id).await?;
        Ok(usage.notifications_sent)
    }

    /// Get usage history for a user (last N months)
    pub async fn get_usage_history(
        &self,
        user_id: Uuid,
        months: i32,
    ) -> Result<Vec<Usage>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, period_start, monitors_count, notifications_sent, overage_notifications
            FROM usage 
            WHERE user_id = $1 
            ORDER BY period_start DESC 
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(months)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Usage {
                id: r.get("id"),
                user_id: r.get("user_id"),
                period_start: r.get("period_start"),
                monitors_count: r.get("monitors_count"),
                notifications_sent: r.get("notifications_sent"),
                overage_notifications: r.get("overage_notifications"),
            })
            .collect())
    }
}
