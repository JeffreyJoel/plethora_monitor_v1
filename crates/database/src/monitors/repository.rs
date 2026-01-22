//! # Monitor Repository
//!
//! Data access layer for monitor configurations.
//! Provides CRUD operations for storing and retrieving monitor settings
//! from the database. Monitor configurations are stored as JSONB for flexibility.

use monitor::primitives::models::MonitorConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct MonitorRepository {
    pool: PgPool,
}

impl MonitorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // saves a new monitor configuration linked to a user
    pub async fn create(&self, user_id: Uuid, config: &MonitorConfig) -> Result<Uuid, sqlx::Error> {
        // Serialize the Rust struct into JSONB for storage
        let config_json = serde_json::to_value(config).unwrap();

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO monitors (user_id, name, configuration) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(user_id)
        .bind(&config.name)
        .bind(config_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    // gets all active monitors for a specific user
    pub async fn get_by_user(&self, user_id: Uuid) -> Result<Vec<(Uuid, MonitorConfig)>, sqlx::Error> {
        let recs = sqlx::query(
            "SELECT id, configuration FROM monitors WHERE user_id = $1 AND is_active = true",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut monitors = Vec::new();
        for row in recs {
            let id: Uuid = row.try_get("id")?;
            let cfg_json: serde_json::Value = row.try_get("configuration")?;
            if let Ok(cfg) = serde_json::from_value(cfg_json) {
                monitors.push((id, cfg));
            }
        }

        Ok(monitors)
    }

    // gets all active monitors across all users (for startup restoration)
    pub async fn get_all_active(&self) -> Result<Vec<(Uuid, Uuid, MonitorConfig)>, sqlx::Error> {
        let recs = sqlx::query(
            "SELECT id, user_id, configuration FROM monitors WHERE is_active = true",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut monitors = Vec::new();
        for row in recs {
            let id: Uuid = row.try_get("id")?;
            let user_id: Uuid = row.try_get("user_id")?;
            let cfg_json: serde_json::Value = row.try_get("configuration")?;
            if let Ok(cfg) = serde_json::from_value(cfg_json) {
                monitors.push((id, user_id, cfg));
            }
        }

        Ok(monitors)
    }


    // gets a single monitor (Security: Must match user_id)
    pub async fn get_by_id(
        &self,
        monitor_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MonitorConfig>, sqlx::Error> {
        let rec = sqlx::query("SELECT configuration FROM monitors WHERE id = $1 AND user_id = $2")
            .bind(monitor_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(rec.and_then(|r| {
            r.try_get::<serde_json::Value, _>("configuration")
                .ok()
                .and_then(|v| serde_json::from_value(v).ok())
        }))
    }

    // updates a monitor
    pub async fn update(
        &self,
        monitor_id: Uuid,
        user_id: Uuid,
        config: &MonitorConfig,
    ) -> Result<(), sqlx::Error> {
        let config_json = serde_json::to_value(config).unwrap();

        let result = sqlx::query(
            "UPDATE monitors SET configuration = $1, name = $2 WHERE id = $3 AND user_id = $4",
        )
        .bind(config_json)
        .bind(&config.name)
        .bind(monitor_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    // deletes a monitor
    pub async fn delete(&self, monitor_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        let result = sqlx::query("DELETE FROM monitors WHERE id = $1 AND user_id = $2")
            .bind(monitor_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    /// Count active monitors for a user (for limit enforcement)
    pub async fn count_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM monitors WHERE user_id = $1 AND is_active = true",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

